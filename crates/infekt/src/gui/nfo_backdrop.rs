use iced::widget::image::Handle;
use image::{Rgba as ImageRgba, RgbaImage, imageops};
use palette::rgb::{Rgb, Rgba};

use crate::{
    core::nfo_renderer_grid::{NfoRendererBlockShape, NfoRendererGrid},
    settings::NfoRenderSettings,
};

pub(crate) const BACKDROP_WIDTH: u32 = 640;
pub(crate) const BACKDROP_HEIGHT: u32 = 400;

const ALGORITHM_VERSION: u8 = 1;
const CROP_MARGIN_CELLS: usize = 2;
const CANVAS_PADDING: f64 = 24.0;
const BLUR_SIGMA: f32 = 28.0;
const FALLBACK_CHARACTER_RATIO_MILLI: u16 = 583;
const MAX_CHARACTER_RATIO_MILLI: u16 = 4_000;
const TEXT_MARK_OPACITY: f32 = 0.38;

/// Everything that can change the pixels of an ambient NFO backdrop.
///
/// Renderer settings that do not participate here (zoom, font, Glow, links,
/// and ANSI presentation) deliberately do not invalidate the backdrop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct BackdropKey {
    algorithm_version: u8,
    grid_id: u64,
    background_color: [u8; 3],
    text_color: [u8; 4],
    art_color: [u8; 4],
    character_ratio_milli: u16,
}

impl BackdropKey {
    pub(crate) fn new(
        grid: &NfoRendererGrid,
        settings: &NfoRenderSettings,
        character_ratio: f32,
    ) -> Self {
        Self {
            algorithm_version: ALGORITHM_VERSION,
            grid_id: grid.id,
            background_color: rgb8(settings.background_color),
            text_color: rgba8(settings.text_color),
            art_color: rgba8(settings.art_color),
            character_ratio_milli: character_ratio_milli(character_ratio),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BackdropImage {
    key: BackdropKey,
    generation: u64,
    handle: Handle,
}

impl BackdropImage {
    pub(crate) fn handle(&self) -> &Handle {
        &self.handle
    }
}

/// An owned, `'static` generation job suitable for `Task::perform`.
#[derive(Clone)]
pub(crate) struct BackdropRequest {
    key: BackdropKey,
    generation: u64,
    grid: NfoRendererGrid,
}

impl BackdropRequest {
    pub(crate) fn key(&self) -> BackdropKey {
        self.key
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// Produces a fixed-size blurred image, or `None` when the grid has no
    /// visible blocks or ordinary text. Links are intentionally ignored.
    pub(crate) fn generate(self) -> Option<BackdropImage> {
        let pixels = rasterize(&self.grid, self.key)?;
        let pixels = imageops::fast_blur(&pixels, BLUR_SIGMA);
        let handle = Handle::from_rgba(BACKDROP_WIDTH, BACKDROP_HEIGHT, pixels.into_raw());

        Some(BackdropImage {
            key: self.key,
            generation: self.generation,
            handle,
        })
    }
}

/// Tracks the requested and currently displayed ambient backdrop.
#[derive(Debug, Default)]
pub(crate) struct NfoBackdrop {
    requested_key: Option<BackdropKey>,
    requested_generation: u64,
    image: Option<BackdropImage>,
}

impl NfoBackdrop {
    /// Invalidates stale pixels and returns a new generation request only when
    /// an input that affects backdrop pixels changed.
    pub(crate) fn request(
        &mut self,
        grid: Option<&NfoRendererGrid>,
        settings: &NfoRenderSettings,
        character_ratio: f32,
    ) -> Option<BackdropRequest> {
        let Some(grid) = grid else {
            self.clear();
            return None;
        };

        let key = BackdropKey::new(grid, settings, character_ratio);

        if self.requested_key == Some(key) {
            return None;
        }

        self.advance_generation();
        self.requested_key = Some(key);
        self.image = None;

        Some(BackdropRequest {
            key,
            generation: self.requested_generation,
            grid: grid.clone(),
        })
    }

    /// Accepts a generated image only if it still belongs to the latest
    /// request. A current `None` result leaves the theme gradient visible.
    pub(crate) fn accept_result(
        &mut self,
        key: BackdropKey,
        generation: u64,
        image: Option<BackdropImage>,
    ) -> bool {
        if self.requested_key != Some(key) || self.requested_generation != generation {
            return false;
        }

        self.image = image.filter(|image| image.key == key && image.generation == generation);
        true
    }

    pub(crate) fn current_handle(&self) -> Option<&Handle> {
        self.image.as_ref().map(BackdropImage::handle)
    }

    /// Invalidates the current result before loading a new file. The separate
    /// generation prevents an in-flight result from a layout with a colliding
    /// renderer-grid ID from being accepted for the new source.
    pub(crate) fn invalidate_source(&mut self) {
        self.advance_generation();
        self.requested_key = None;
        self.image = None;
    }

    pub(crate) fn clear(&mut self) {
        self.advance_generation();
        self.requested_key = None;
        self.image = None;
    }

    fn advance_generation(&mut self) {
        self.requested_generation = self.requested_generation.wrapping_add(1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContentBounds {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

impl ContentBounds {
    fn width(self) -> usize {
        self.right.saturating_sub(self.left)
    }

    fn height(self) -> usize {
        self.bottom.saturating_sub(self.top)
    }

    fn contains(self, col: usize, row: usize) -> bool {
        col >= self.left && col < self.right && row >= self.top && row < self.bottom
    }
}

#[derive(Debug, Clone, Copy)]
struct RasterTransform {
    bounds: ContentBounds,
    origin_x: f64,
    origin_y: f64,
    cell_width: f64,
    cell_height: f64,
}

impl RasterTransform {
    fn new(bounds: ContentBounds, character_ratio_milli: u16) -> Option<Self> {
        let crop_width = bounds.width();
        let crop_height = bounds.height();

        if crop_width == 0 || crop_height == 0 {
            return None;
        }

        let available_width = f64::from(BACKDROP_WIDTH) - CANVAS_PADDING * 2.0;
        let available_height = f64::from(BACKDROP_HEIGHT) - CANVAS_PADDING * 2.0;
        let character_ratio = f64::from(character_ratio_milli) / 1_000.0;
        let cell_height = (available_width / (crop_width as f64 * character_ratio))
            .min(available_height / crop_height as f64);
        let cell_width = cell_height * character_ratio;
        let rendered_width = cell_width * crop_width as f64;
        let rendered_height = cell_height * crop_height as f64;

        Some(Self {
            bounds,
            origin_x: (f64::from(BACKDROP_WIDTH) - rendered_width) * 0.5,
            origin_y: (f64::from(BACKDROP_HEIGHT) - rendered_height) * 0.5,
            cell_width,
            cell_height,
        })
    }

    fn cell_rect(self, col: usize, row: usize) -> PixelRect {
        let relative_col = col.saturating_sub(self.bounds.left) as f64;
        let relative_row = row.saturating_sub(self.bounds.top) as f64;
        let left = self.origin_x + relative_col * self.cell_width;
        let top = self.origin_y + relative_row * self.cell_height;

        PixelRect {
            left,
            top,
            right: left + self.cell_width,
            bottom: top + self.cell_height,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PixelRect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl PixelRect {
    fn subrect(self, left: f64, top: f64, right: f64, bottom: f64) -> Self {
        let width = self.right - self.left;
        let height = self.bottom - self.top;

        Self {
            left: self.left + width * left,
            top: self.top + height * top,
            right: self.left + width * right,
            bottom: self.top + height * bottom,
        }
    }
}

fn rasterize(grid: &NfoRendererGrid, key: BackdropKey) -> Option<RgbaImage> {
    let bounds = visible_bounds(grid)?;
    let transform = RasterTransform::new(bounds, key.character_ratio_milli)?;
    let background = ImageRgba([
        key.background_color[0],
        key.background_color[1],
        key.background_color[2],
        255,
    ]);
    let mut image = RgbaImage::from_pixel(BACKDROP_WIDTH, BACKDROP_HEIGHT, background);

    for line in &grid.lines {
        if line.row >= grid.height || line.row < bounds.top || line.row >= bounds.bottom {
            continue;
        }

        for group in &line.block_groups {
            for (offset, shape) in group.blocks.iter().enumerate() {
                let Some(col) = group.col.checked_add(offset) else {
                    break;
                };

                if col >= grid.width || !bounds.contains(col, line.row) {
                    continue;
                }

                let Some((relative_rect, shade_opacity)) = block_geometry(shape) else {
                    continue;
                };
                let cell = transform.cell_rect(col, line.row);
                blend_rect(
                    &mut image,
                    cell.subrect(
                        relative_rect.left,
                        relative_rect.top,
                        relative_rect.right,
                        relative_rect.bottom,
                    ),
                    key.art_color,
                    shade_opacity,
                );
            }
        }

        for flight in &line.text_flights {
            for (offset, character) in flight.text.chars().enumerate() {
                if character.is_whitespace() {
                    continue;
                }

                let Some(col) = flight.col.checked_add(offset) else {
                    break;
                };

                if col >= grid.width || !bounds.contains(col, line.row) {
                    continue;
                }

                let mark = transform
                    .cell_rect(col, line.row)
                    .subrect(0.16, 0.34, 0.84, 0.66);
                blend_rect(&mut image, mark, key.text_color, TEXT_MARK_OPACITY);
            }
        }
    }

    Some(image)
}

fn visible_bounds(grid: &NfoRendererGrid) -> Option<ContentBounds> {
    if grid.width == 0 || grid.height == 0 {
        return None;
    }

    let mut minimum_col = usize::MAX;
    let mut minimum_row = usize::MAX;
    let mut maximum_col = 0;
    let mut maximum_row = 0;
    let mut has_content = false;

    let mut include = |col: usize, row: usize| {
        if col >= grid.width || row >= grid.height {
            return;
        }

        has_content = true;
        minimum_col = minimum_col.min(col);
        minimum_row = minimum_row.min(row);
        maximum_col = maximum_col.max(col);
        maximum_row = maximum_row.max(row);
    };

    for line in &grid.lines {
        for group in &line.block_groups {
            for (offset, shape) in group.blocks.iter().enumerate() {
                if !is_visible_block(shape) {
                    continue;
                }

                let Some(col) = group.col.checked_add(offset) else {
                    break;
                };
                include(col, line.row);
            }
        }

        for flight in &line.text_flights {
            for (offset, character) in flight.text.chars().enumerate() {
                if character.is_whitespace() {
                    continue;
                }

                let Some(col) = flight.col.checked_add(offset) else {
                    break;
                };
                include(col, line.row);
            }
        }
    }

    has_content.then(|| ContentBounds {
        left: minimum_col.saturating_sub(CROP_MARGIN_CELLS),
        top: minimum_row.saturating_sub(CROP_MARGIN_CELLS),
        right: maximum_col
            .saturating_add(CROP_MARGIN_CELLS + 1)
            .min(grid.width),
        bottom: maximum_row
            .saturating_add(CROP_MARGIN_CELLS + 1)
            .min(grid.height),
    })
}

fn is_visible_block(shape: &NfoRendererBlockShape) -> bool {
    !matches!(
        shape,
        NfoRendererBlockShape::NoBlock
            | NfoRendererBlockShape::Whitespace
            | NfoRendererBlockShape::WhitespaceInText
    )
}

fn block_geometry(shape: &NfoRendererBlockShape) -> Option<(PixelRect, f32)> {
    let full = PixelRect {
        left: 0.0,
        top: 0.0,
        right: 1.0,
        bottom: 1.0,
    };

    match shape {
        NfoRendererBlockShape::FullBlock => Some((full, 1.0)),
        NfoRendererBlockShape::FullBlockLightShade => Some((full, 90.0 / 255.0)),
        NfoRendererBlockShape::FullBlockMediumShade => Some((full, 140.0 / 255.0)),
        NfoRendererBlockShape::FullBlockDarkShade => Some((full, 190.0 / 255.0)),
        NfoRendererBlockShape::LowerHalf => Some((full.subrect(0.0, 0.5, 1.0, 1.0), 1.0)),
        NfoRendererBlockShape::UpperHalf => Some((full.subrect(0.0, 0.0, 1.0, 0.5), 1.0)),
        NfoRendererBlockShape::RightHalf => Some((full.subrect(0.5, 0.0, 1.0, 1.0), 1.0)),
        NfoRendererBlockShape::LeftHalf => Some((full.subrect(0.0, 0.0, 0.5, 1.0), 1.0)),
        NfoRendererBlockShape::BlackSquare => Some((full.subrect(0.125, 0.125, 0.875, 0.875), 1.0)),
        NfoRendererBlockShape::BlackSquareSmall => {
            Some((full.subrect(0.25, 0.25, 0.75, 0.75), 1.0))
        }
        NfoRendererBlockShape::NoBlock
        | NfoRendererBlockShape::Whitespace
        | NfoRendererBlockShape::WhitespaceInText => None,
    }
}

fn blend_rect(image: &mut RgbaImage, rect: PixelRect, color: [u8; 4], opacity: f32) {
    let left = rect.left.floor().clamp(0.0, f64::from(BACKDROP_WIDTH)) as u32;
    let top = rect.top.floor().clamp(0.0, f64::from(BACKDROP_HEIGHT)) as u32;
    let right = rect.right.ceil().clamp(0.0, f64::from(BACKDROP_WIDTH)) as u32;
    let bottom = rect.bottom.ceil().clamp(0.0, f64::from(BACKDROP_HEIGHT)) as u32;
    let source_alpha = (f32::from(color[3]) / 255.0 * opacity).clamp(0.0, 1.0);

    if left >= right || top >= bottom || source_alpha <= 0.0 {
        return;
    }

    for y in top..bottom {
        for x in left..right {
            let destination = image.get_pixel_mut(x, y);

            for channel in 0..3 {
                destination[channel] = (f32::from(color[channel]) * source_alpha
                    + f32::from(destination[channel]) * (1.0 - source_alpha))
                    .round() as u8;
            }
            destination[3] = 255;
        }
    }
}

fn character_ratio_milli(character_ratio: f32) -> u16 {
    if !character_ratio.is_finite() || character_ratio <= 0.0 {
        return FALLBACK_CHARACTER_RATIO_MILLI;
    }

    (character_ratio * 1_000.0)
        .round()
        .clamp(1.0, f32::from(MAX_CHARACTER_RATIO_MILLI)) as u16
}

fn rgb8(color: Rgb) -> [u8; 3] {
    [
        channel8(color.red),
        channel8(color.green),
        channel8(color.blue),
    ]
}

fn rgba8(color: Rgba) -> [u8; 4] {
    [
        channel8(color.red),
        channel8(color.green),
        channel8(color.blue),
        channel8(color.alpha),
    ]
}

fn channel8(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::nfo_renderer_grid::{
        NfoRendererBlockGroup, NfoRendererLine, NfoRendererLink, NfoRendererTextFlight,
    };

    fn grid_with_line(
        width: usize,
        height: usize,
        id: u64,
        line: NfoRendererLine,
    ) -> NfoRendererGrid {
        NfoRendererGrid {
            width,
            height,
            lines: vec![line],
            has_blocks: true,
            id,
        }
    }

    fn line(row: usize) -> NfoRendererLine {
        NfoRendererLine {
            row,
            block_groups: Vec::new(),
            text_flights: Vec::new(),
            links: Vec::new(),
        }
    }

    fn single_block_grid(shape: NfoRendererBlockShape) -> NfoRendererGrid {
        let mut content = line(5);
        content.block_groups.push(NfoRendererBlockGroup {
            col: 5,
            blocks: vec![shape],
        });
        grid_with_line(11, 11, 7, content)
    }

    #[test]
    fn key_tracks_only_inputs_that_change_backdrop_pixels() {
        let grid = single_block_grid(NfoRendererBlockShape::FullBlock);
        let settings = NfoRenderSettings::default();
        let original = BackdropKey::new(&grid, &settings, 0.583);

        let mut changed_grid = grid.clone();
        changed_grid.id += 1;
        assert_ne!(original, BackdropKey::new(&changed_grid, &settings, 0.583));

        let mut changed = settings.clone();
        changed.background_color = Rgb::new(0.2, 0.3, 0.4);
        assert_ne!(original, BackdropKey::new(&grid, &changed, 0.583));
        changed = settings.clone();
        changed.text_color = Rgba::new(0.2, 0.3, 0.4, 0.5);
        assert_ne!(original, BackdropKey::new(&grid, &changed, 0.583));
        changed = settings.clone();
        changed.art_color = Rgba::new(0.2, 0.3, 0.4, 0.5);
        assert_ne!(original, BackdropKey::new(&grid, &changed, 0.583));
        assert_ne!(original, BackdropKey::new(&grid, &settings, 0.600));

        changed = settings.clone();
        changed.enhanced_view_block_width = 99;
        changed.enhanced_view_block_height = 99;
        changed.classic_font_size = 32.0;
        changed.font_name = "Different Font".into();
        changed.blur_enabled = !changed.blur_enabled;
        changed.blur_color = Rgba::new(1.0, 0.0, 0.0, 1.0);
        changed.blur_radius = 48;
        changed.blur_enabled_for_ansi_art = !changed.blur_enabled_for_ansi_art;
        changed.hyperlink_color = Rgba::new(1.0, 0.0, 0.0, 1.0);
        changed.hyperlink_underline = !changed.hyperlink_underline;
        assert_eq!(original, BackdropKey::new(&grid, &changed, 0.583));
    }

    #[test]
    fn bounds_crop_to_visible_content_with_two_cell_margin() {
        let mut content = line(10);
        content.block_groups.push(NfoRendererBlockGroup {
            col: 20,
            blocks: vec![NfoRendererBlockShape::FullBlock],
        });
        content.text_flights.push(NfoRendererTextFlight {
            col: 30,
            text: "A".into(),
        });
        let grid = grid_with_line(100, 50, 1, content);

        assert_eq!(
            visible_bounds(&grid),
            Some(ContentBounds {
                left: 18,
                top: 8,
                right: 33,
                bottom: 13,
            })
        );
    }

    #[test]
    fn block_geometry_matches_foreground_renderer_shapes_and_shades() {
        let (full, full_opacity) = block_geometry(&NfoRendererBlockShape::FullBlock).unwrap();
        assert_eq!(
            (full.left, full.top, full.right, full.bottom),
            (0.0, 0.0, 1.0, 1.0)
        );
        assert_eq!(full_opacity, 1.0);

        let (lower, _) = block_geometry(&NfoRendererBlockShape::LowerHalf).unwrap();
        assert_eq!(
            (lower.left, lower.top, lower.right, lower.bottom),
            (0.0, 0.5, 1.0, 1.0)
        );
        let (upper, _) = block_geometry(&NfoRendererBlockShape::UpperHalf).unwrap();
        assert_eq!(
            (upper.left, upper.top, upper.right, upper.bottom),
            (0.0, 0.0, 1.0, 0.5)
        );
        let (right, _) = block_geometry(&NfoRendererBlockShape::RightHalf).unwrap();
        assert_eq!(
            (right.left, right.top, right.right, right.bottom),
            (0.5, 0.0, 1.0, 1.0)
        );
        let (left, _) = block_geometry(&NfoRendererBlockShape::LeftHalf).unwrap();
        assert_eq!(
            (left.left, left.top, left.right, left.bottom),
            (0.0, 0.0, 0.5, 1.0)
        );
        let (square, _) = block_geometry(&NfoRendererBlockShape::BlackSquare).unwrap();
        assert_eq!(
            (square.left, square.top, square.right, square.bottom),
            (0.125, 0.125, 0.875, 0.875)
        );
        let (small, _) = block_geometry(&NfoRendererBlockShape::BlackSquareSmall).unwrap();
        assert_eq!(
            (small.left, small.top, small.right, small.bottom),
            (0.25, 0.25, 0.75, 0.75)
        );

        assert_eq!(
            block_geometry(&NfoRendererBlockShape::FullBlockLightShade)
                .unwrap()
                .1,
            90.0 / 255.0
        );
        assert_eq!(
            block_geometry(&NfoRendererBlockShape::FullBlockMediumShade)
                .unwrap()
                .1,
            140.0 / 255.0
        );
        assert_eq!(
            block_geometry(&NfoRendererBlockShape::FullBlockDarkShade)
                .unwrap()
                .1,
            190.0 / 255.0
        );
    }

    #[test]
    fn blur_spreads_art_beyond_the_unblurred_shape() {
        let grid = single_block_grid(NfoRendererBlockShape::FullBlock);
        let settings = NfoRenderSettings {
            background_color: Rgb::new(0.0, 0.0, 0.0),
            art_color: Rgba::new(1.0, 1.0, 1.0, 1.0),
            ..NfoRenderSettings::default()
        };
        let key = BackdropKey::new(&grid, &settings, 0.583);
        let raw = rasterize(&grid, key).unwrap();
        let blurred = imageops::fast_blur(&raw, BLUR_SIGMA);
        let background = ImageRgba([0, 0, 0, 255]);
        let raw_non_background = raw.pixels().filter(|pixel| **pixel != background).count();
        let blurred_non_background = blurred
            .pixels()
            .filter(|pixel| **pixel != background)
            .count();

        assert!(blurred_non_background > raw_non_background);
        assert!(blurred.pixels().all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn empty_whitespace_and_links_only_grids_generate_no_backdrop() {
        let empty = NfoRendererGrid {
            width: 80,
            height: 25,
            lines: Vec::new(),
            has_blocks: false,
            id: 1,
        };
        assert!(visible_bounds(&empty).is_none());

        let mut links_only = line(4);
        links_only.links.push(NfoRendererLink {
            col: 4,
            text: "https://infekt.ws".into(),
            url: "https://infekt.ws".into(),
        });
        let links_only = grid_with_line(80, 25, 2, links_only);
        assert!(visible_bounds(&links_only).is_none());

        let mut whitespace = line(4);
        whitespace.text_flights.push(NfoRendererTextFlight {
            col: 4,
            text: "   \t".into(),
        });
        let whitespace = grid_with_line(80, 25, 3, whitespace);
        assert!(visible_bounds(&whitespace).is_none());
    }

    #[test]
    fn extreme_declared_grid_still_allocates_only_the_fixed_backdrop() {
        let mut content = line(usize::MAX - 2);
        content.block_groups.push(NfoRendererBlockGroup {
            col: usize::MAX - 2,
            blocks: vec![NfoRendererBlockShape::FullBlock],
        });
        let grid = grid_with_line(usize::MAX, usize::MAX, 99, content);
        let settings = NfoRenderSettings::default();
        let key = BackdropKey::new(&grid, &settings, 0.583);
        let pixels = rasterize(&grid, key).unwrap();

        assert_eq!(pixels.dimensions(), (BACKDROP_WIDTH, BACKDROP_HEIGHT));
        assert_eq!(
            pixels.len(),
            (BACKDROP_WIDTH * BACKDROP_HEIGHT * 4) as usize
        );
    }

    #[test]
    fn stale_results_are_rejected_without_replacing_the_current_image() {
        let first_grid = single_block_grid(NfoRendererBlockShape::FullBlock);
        let mut second_grid = first_grid.clone();
        second_grid.id += 1;
        let settings = NfoRenderSettings::default();
        let mut state = NfoBackdrop::default();

        let first = state.request(Some(&first_grid), &settings, 0.583).unwrap();
        let first_key = first.key();
        let first_generation = first.generation();
        let first_image = first.generate().unwrap();
        let second = state.request(Some(&second_grid), &settings, 0.583).unwrap();
        let second_key = second.key();
        let second_generation = second.generation();

        assert!(!state.accept_result(first_key, first_generation, Some(first_image)));
        assert!(state.current_handle().is_none());
        assert!(state.accept_result(second_key, second_generation, second.generate()));
        assert!(state.current_handle().is_some());
        assert!(
            state
                .request(Some(&second_grid), &settings, 0.583)
                .is_none()
        );
    }

    #[test]
    fn source_invalidation_forces_same_key_regeneration_and_rejects_old_work() {
        let grid = single_block_grid(NfoRendererBlockShape::FullBlock);
        let settings = NfoRenderSettings::default();
        let mut state = NfoBackdrop::default();
        let first = state.request(Some(&grid), &settings, 0.583).unwrap();
        let first_key = first.key();
        let first_generation = first.generation();
        let first_image = first.generate().unwrap();

        state.invalidate_source();
        let second = state.request(Some(&grid), &settings, 0.583).unwrap();

        assert_eq!(second.key(), first_key);
        assert_ne!(second.generation(), first_generation);
        assert!(!state.accept_result(first_key, first_generation, Some(first_image)));
        assert!(state.current_handle().is_none());
    }
}
