use iced::widget::image::Handle;
use image::{Rgba as ImageRgba, RgbaImage, imageops};
use palette::rgb::{Rgb, Rgba};

use crate::{
    core::nfo_renderer_grid::{NfoRendererBlockShape, NfoRendererGrid},
    settings::NfoRenderSettings,
};

pub(crate) const BACKDROP_WIDTH: u32 = 640;
pub(crate) const BACKDROP_HEIGHT: u32 = 400;

const BACKDROP_TILE_WIDTH: u32 = BACKDROP_WIDTH / 2;
const ALGORITHM_VERSION: u8 = 8;
const CROP_MARGIN_CELLS: usize = 2;
const LEADING_ART_BLOCK_THRESHOLD: usize = 20;
const CANVAS_PADDING: f64 = 24.0;
const BLUR_SIGMA: f32 = 8.0;
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
    content_top: usize,
}

impl ContentBounds {
    fn width(self) -> usize {
        self.right.saturating_sub(self.left)
    }

    fn height(self) -> usize {
        self.bottom.saturating_sub(self.top)
    }

    fn contains(self, col: usize, row: usize) -> bool {
        col >= self.left
            && col < self.right
            && row >= self.top
            && row < self.bottom
            && row >= self.content_top
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
    fn new(
        bounds: ContentBounds,
        character_ratio_milli: u16,
        horizontal_focal: f64,
        vertical_focal: f64,
        canvas_width: u32,
        canvas_height: u32,
    ) -> Option<Self> {
        let crop_width = bounds.width();
        let crop_height = bounds.height();

        if crop_width == 0 || crop_height == 0 {
            return None;
        }

        let available_width = f64::from(canvas_width) - CANVAS_PADDING * 2.0;
        let available_height = f64::from(canvas_height) - CANVAS_PADDING * 2.0;

        if available_width <= 0.0 || available_height <= 0.0 {
            return None;
        }

        let character_ratio = f64::from(character_ratio_milli) / 1_000.0;
        let cell_height = (available_width / (crop_width as f64 * character_ratio))
            .max(available_height / crop_height as f64);

        if !cell_height.is_finite() || cell_height <= 0.0 {
            return None;
        }

        let cell_width = cell_height * character_ratio;
        let rendered_width = cell_width * crop_width as f64;
        let rendered_height = cell_height * crop_height as f64;
        let focal_offset_x = (horizontal_focal - bounds.left as f64) * cell_width;
        let focal_offset_y = (vertical_focal - bounds.top as f64) * cell_height;
        let origin_x = cover_origin(f64::from(canvas_width), rendered_width, focal_offset_x);
        let origin_y = cover_origin(f64::from(canvas_height), rendered_height, focal_offset_y);

        Some(Self {
            bounds,
            origin_x,
            origin_y,
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

fn cover_origin(canvas_extent: f64, rendered_extent: f64, focal_offset: f64) -> f64 {
    let available_extent = canvas_extent - CANVAS_PADDING * 2.0;
    let minimum_origin = (CANVAS_PADDING + available_extent - rendered_extent).min(CANVAS_PADDING);
    let maximum_origin = CANVAS_PADDING;

    (canvas_extent * 0.5 - focal_offset).clamp(minimum_origin, maximum_origin)
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
    let (horizontal_focal, vertical_focal) = ink_focal(grid, bounds, key);
    let transform = RasterTransform::new(
        bounds,
        key.character_ratio_milli,
        horizontal_focal,
        vertical_focal,
        BACKDROP_TILE_WIDTH,
        BACKDROP_HEIGHT,
    )?;
    let background = ImageRgba([
        key.background_color[0],
        key.background_color[1],
        key.background_color[2],
        255,
    ]);
    let mut tile = RgbaImage::from_pixel(BACKDROP_TILE_WIDTH, BACKDROP_HEIGHT, background);

    paint_grid(&mut tile, grid, bounds, transform, key);

    // Mirror one cover-rendered tile instead of stretching the NFO.
    // The matching center-edge pixels let the later full-image blur erase the
    // change in direction without introducing a value seam.
    let mirrored_tile = imageops::flip_horizontal(&tile);
    let mut image = RgbaImage::from_pixel(BACKDROP_WIDTH, BACKDROP_HEIGHT, background);
    imageops::replace(&mut image, &tile, 0, 0);
    imageops::replace(
        &mut image,
        &mirrored_tile,
        i64::from(BACKDROP_TILE_WIDTH),
        0,
    );

    Some(image)
}

fn paint_grid(
    image: &mut RgbaImage,
    grid: &NfoRendererGrid,
    bounds: ContentBounds,
    transform: RasterTransform,
    key: BackdropKey,
) {
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
                    image,
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
                blend_rect(image, mark, key.text_color, TEXT_MARK_OPACITY);
            }
        }
    }
}

/// Returns the center of mass of the ink that will actually be painted.
///
/// Block area and shade opacity match `block_geometry`; ordinary text uses
/// the same subdued mark dimensions and opacity as `rasterize`. The selected
/// content bounds also exclude any sparse prelude before a dense art anchor.
fn ink_focal(grid: &NfoRendererGrid, bounds: ContentBounds, key: BackdropKey) -> (f64, f64) {
    let mut weighted_col = 0.0;
    let mut weighted_row = 0.0;
    let mut total_weight = 0.0;
    let art_alpha = f64::from(key.art_color[3]) / 255.0;
    let text_alpha = f64::from(key.text_color[3]) / 255.0;
    let text_mark_area = (0.84 - 0.16) * (0.66 - 0.34);
    let text_weight = text_mark_area * f64::from(TEXT_MARK_OPACITY) * text_alpha;

    for line in &grid.lines {
        if line.row >= grid.height || line.row < bounds.content_top || line.row >= bounds.bottom {
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

                let Some((rect, shade_opacity)) = block_geometry(shape) else {
                    continue;
                };
                let area = (rect.right - rect.left) * (rect.bottom - rect.top);
                let weight = area * f64::from(shade_opacity) * art_alpha;
                let center_col = col as f64 + (rect.left + rect.right) * 0.5;
                let center_row = line.row as f64 + (rect.top + rect.bottom) * 0.5;

                weighted_col += center_col * weight;
                weighted_row += center_row * weight;
                total_weight += weight;
            }
        }

        if text_weight <= 0.0 {
            continue;
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

                weighted_col += (col as f64 + 0.5) * text_weight;
                weighted_row += (line.row as f64 + 0.5) * text_weight;
                total_weight += text_weight;
            }
        }
    }

    if total_weight > f64::EPSILON {
        (weighted_col / total_weight, weighted_row / total_weight)
    } else {
        (
            (bounds.left as f64 + bounds.right as f64) * 0.5,
            (bounds.top as f64 + bounds.bottom as f64) * 0.5,
        )
    }
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
    let content_top = first_dense_block_row(grid).unwrap_or(0);

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
        if line.row < content_top {
            continue;
        }

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
        content_top,
    })
}

fn first_dense_block_row(grid: &NfoRendererGrid) -> Option<usize> {
    grid.lines
        .iter()
        .filter(|line| line.row < grid.height)
        .filter_map(|line| {
            let mut visible_cols = [usize::MAX; LEADING_ART_BLOCK_THRESHOLD];
            let mut visible_blocks = 0;

            for group in &line.block_groups {
                for (offset, shape) in group.blocks.iter().enumerate() {
                    let Some(col) = group.col.checked_add(offset) else {
                        break;
                    };

                    if col < grid.width
                        && is_visible_block(shape)
                        && !visible_cols[..visible_blocks].contains(&col)
                    {
                        visible_cols[visible_blocks] = col;
                        visible_blocks += 1;

                        if visible_blocks >= LEADING_ART_BLOCK_THRESHOLD {
                            return Some(line.row);
                        }
                    }
                }
            }

            None
        })
        .min()
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
    let width = f64::from(image.width());
    let height = f64::from(image.height());
    let left = rect.left.floor().clamp(0.0, width) as u32;
    let top = rect.top.floor().clamp(0.0, height) as u32;
    let right = rect.right.ceil().clamp(0.0, width) as u32;
    let bottom = rect.bottom.ceil().clamp(0.0, height) as u32;
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
        grid_with_lines(width, height, id, vec![line])
    }

    fn grid_with_lines(
        width: usize,
        height: usize,
        id: u64,
        lines: Vec<NfoRendererLine>,
    ) -> NfoRendererGrid {
        NfoRendererGrid {
            width,
            height,
            lines,
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

    fn repeat_energy_grid() -> NfoRendererGrid {
        let mut anchor = line(100);
        anchor.block_groups.push(NfoRendererBlockGroup {
            col: 5,
            blocks: vec![NfoRendererBlockShape::FullBlock; 70],
        });
        let mut trailing = line(180);
        trailing.block_groups.push(NfoRendererBlockGroup {
            col: 74,
            blocks: vec![NfoRendererBlockShape::FullBlock],
        });

        grid_with_lines(80, 220, 20, vec![anchor, trailing])
    }

    fn high_contrast_settings() -> NfoRenderSettings {
        NfoRenderSettings {
            background_color: Rgb::new(0.0, 0.0, 0.0),
            art_color: Rgba::new(1.0, 1.0, 1.0, 1.0),
            ..NfoRenderSettings::default()
        }
    }

    fn assert_cover(transform: RasterTransform, canvas_width: u32, canvas_height: u32) {
        let epsilon = 1.0e-7;
        let rendered_width = transform.cell_width * transform.bounds.width() as f64;
        let rendered_height = transform.cell_height * transform.bounds.height() as f64;

        assert!(transform.origin_x <= CANVAS_PADDING + epsilon);
        assert!(
            transform.origin_x + rendered_width
                >= f64::from(canvas_width) - CANVAS_PADDING - epsilon
        );
        assert!(transform.origin_y <= CANVAS_PADDING + epsilon);
        assert!(
            transform.origin_y + rendered_height
                >= f64::from(canvas_height) - CANVAS_PADDING - epsilon
        );
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
                content_top: 0,
            })
        );
    }

    #[test]
    fn leading_sparse_rows_are_excluded_after_dense_art_anchor() {
        let mut early = line(3);
        early.block_groups.push(NfoRendererBlockGroup {
            col: 0,
            blocks: vec![NfoRendererBlockShape::FullBlock; 19],
        });
        early.text_flights.push(NfoRendererTextFlight {
            col: 70,
            text: "ordinary text does not establish the art anchor".into(),
        });

        let mut inside_margin = line(8);
        inside_margin.block_groups.push(NfoRendererBlockGroup {
            col: 35,
            blocks: vec![NfoRendererBlockShape::FullBlock],
        });

        let mut anchor = line(10);
        anchor.block_groups.push(NfoRendererBlockGroup {
            col: 30,
            blocks: vec![NfoRendererBlockShape::FullBlock; 20],
        });

        let mut trailing = line(15);
        trailing.block_groups.push(NfoRendererBlockGroup {
            col: 60,
            blocks: vec![NfoRendererBlockShape::FullBlock],
        });
        trailing.text_flights.push(NfoRendererTextFlight {
            col: 70,
            text: "A".into(),
        });

        let grid = grid_with_lines(100, 30, 11, vec![early, inside_margin, anchor, trailing]);
        let bounds = visible_bounds(&grid).unwrap();

        assert_eq!(
            bounds,
            ContentBounds {
                left: 28,
                top: 8,
                right: 73,
                bottom: 18,
                content_top: 10,
            }
        );
        assert!(!bounds.contains(35, 8));
        assert!(bounds.contains(30, 10));
        assert!(bounds.contains(70, 15));
    }

    #[test]
    fn sparse_art_and_text_only_grids_keep_the_existing_fallback() {
        let mut sparse = line(7);
        sparse.block_groups.push(NfoRendererBlockGroup {
            col: 12,
            blocks: vec![NfoRendererBlockShape::FullBlock; 19],
        });
        let sparse_grid = grid_with_line(80, 25, 12, sparse);
        let sparse_bounds = visible_bounds(&sparse_grid).unwrap();

        assert_eq!(first_dense_block_row(&sparse_grid), None);
        assert_eq!(sparse_bounds.content_top, 0);
        assert!(sparse_bounds.contains(12, 7));

        let mut text = line(4);
        text.text_flights.push(NfoRendererTextFlight {
            col: 6,
            text: "text-only NFO".into(),
        });
        let text_grid = grid_with_line(80, 25, 13, text);
        let text_bounds = visible_bounds(&text_grid).unwrap();

        assert_eq!(first_dense_block_row(&text_grid), None);
        assert_eq!(text_bounds.content_top, 0);
        assert!(text_bounds.contains(6, 4));
    }

    #[test]
    fn dense_anchor_uses_lowest_row_with_twenty_distinct_valid_blocks() {
        let mut out_of_bounds = line(1);
        out_of_bounds.block_groups.push(NfoRendererBlockGroup {
            col: 80,
            blocks: vec![NfoRendererBlockShape::FullBlock; 20],
        });

        let mut duplicates = line(2);
        duplicates.block_groups.push(NfoRendererBlockGroup {
            col: 0,
            blocks: vec![NfoRendererBlockShape::FullBlock; 10],
        });
        duplicates.block_groups.push(NfoRendererBlockGroup {
            col: 0,
            blocks: vec![NfoRendererBlockShape::FullBlock; 10],
        });

        let mut whitespace = line(3);
        whitespace.block_groups.push(NfoRendererBlockGroup {
            col: 0,
            blocks: vec![NfoRendererBlockShape::Whitespace; 20],
        });

        let mut later = line(12);
        later.block_groups.push(NfoRendererBlockGroup {
            col: 30,
            blocks: vec![NfoRendererBlockShape::FullBlock; 20],
        });

        let mut earlier = line(5);
        earlier.block_groups.push(NfoRendererBlockGroup {
            col: 20,
            blocks: vec![NfoRendererBlockShape::FullBlock; 20],
        });

        let grid = grid_with_lines(
            80,
            25,
            14,
            vec![later, whitespace, duplicates, earlier, out_of_bounds],
        );

        assert_eq!(first_dense_block_row(&grid), Some(5));
    }

    #[test]
    fn asymmetric_eligible_ink_is_centered_on_the_overflowing_axis() {
        let mut prelude = line(8);
        prelude.block_groups.push(NfoRendererBlockGroup {
            col: 0,
            blocks: vec![NfoRendererBlockShape::FullBlock; 19],
        });

        let mut anchor = line(10);
        anchor.block_groups.push(NfoRendererBlockGroup {
            col: 30,
            blocks: vec![NfoRendererBlockShape::FullBlock; 20],
        });
        anchor.block_groups.push(NfoRendererBlockGroup {
            col: 70,
            blocks: vec![NfoRendererBlockShape::FullBlock],
        });

        let grid = grid_with_lines(100, 40, 15, vec![prelude, anchor]);
        let settings = NfoRenderSettings::default();
        let key = BackdropKey::new(&grid, &settings, 0.583);
        let bounds = visible_bounds(&grid).unwrap();
        let (focal_x, focal_y) = ink_focal(&grid, bounds, key);
        let expected_focal_x = (20.0 * 40.0 + 70.5) / 21.0;
        let transform = RasterTransform::new(
            bounds,
            key.character_ratio_milli,
            focal_x,
            focal_y,
            BACKDROP_TILE_WIDTH,
            BACKDROP_HEIGHT,
        )
        .unwrap();
        let focal_pixel_x =
            transform.origin_x + (focal_x - bounds.left as f64) * transform.cell_width;

        assert_eq!(bounds.content_top, 10);
        assert!((focal_x - expected_focal_x).abs() < 1.0e-9);
        assert_eq!(focal_y, 10.5);
        assert!((focal_pixel_x - f64::from(BACKDROP_TILE_WIDTH) * 0.5).abs() < 1.0e-9);
        assert!(transform.origin_x < CANVAS_PADDING);
        assert_cover(transform, BACKDROP_TILE_WIDTH, BACKDROP_HEIGHT);
    }

    #[test]
    fn focal_weight_matches_block_shade_and_geometry_ink() {
        let mut content = line(5);
        content.block_groups.push(NfoRendererBlockGroup {
            col: 10,
            blocks: vec![NfoRendererBlockShape::FullBlock],
        });
        content.block_groups.push(NfoRendererBlockGroup {
            col: 30,
            blocks: vec![NfoRendererBlockShape::FullBlockLightShade],
        });
        content.block_groups.push(NfoRendererBlockGroup {
            col: 50,
            blocks: vec![NfoRendererBlockShape::RightHalf],
        });
        let grid = grid_with_line(80, 20, 16, content);
        let settings = NfoRenderSettings::default();
        let key = BackdropKey::new(&grid, &settings, 0.583);
        let bounds = visible_bounds(&grid).unwrap();
        let (focal, focal_y) = ink_focal(&grid, bounds, key);
        let shade_weight = f64::from(90.0_f32 / 255.0);
        let half_weight = 0.5;
        let expected =
            (10.5 + 30.5 * shade_weight + 50.75 * half_weight) / (1.0 + shade_weight + half_weight);

        assert!((focal - expected).abs() < 1.0e-9);
        assert!(focal < (10.5 + 30.5 + 50.75) / 3.0);
        assert_eq!(focal_y, 5.5);
    }

    #[test]
    fn vertical_focal_uses_the_painted_geometry_center_and_weight() {
        let mut upper = line(10);
        upper.block_groups.push(NfoRendererBlockGroup {
            col: 20,
            blocks: vec![NfoRendererBlockShape::FullBlock],
        });
        let mut lower = line(30);
        lower.block_groups.push(NfoRendererBlockGroup {
            col: 20,
            blocks: vec![NfoRendererBlockShape::LowerHalf],
        });
        let grid = grid_with_lines(50, 40, 19, vec![upper, lower]);
        let settings = NfoRenderSettings::default();
        let key = BackdropKey::new(&grid, &settings, 0.583);
        let bounds = visible_bounds(&grid).unwrap();
        let (focal_x, focal_y) = ink_focal(&grid, bounds, key);
        let expected_y = (10.5 + 30.75 * 0.5) / 1.5;

        assert_eq!(focal_x, 20.5);
        assert!((focal_y - expected_y).abs() < 1.0e-9);
    }

    #[test]
    fn cover_transform_clamps_extreme_focals_without_exposing_the_inset() {
        let wide = ContentBounds {
            left: 0,
            top: 0,
            right: 100,
            bottom: 4,
            content_top: 0,
        };
        let wide_left =
            RasterTransform::new(wide, 583, 0.5, 2.0, BACKDROP_TILE_WIDTH, BACKDROP_HEIGHT)
                .unwrap();
        let wide_right =
            RasterTransform::new(wide, 583, 99.5, 2.0, BACKDROP_TILE_WIDTH, BACKDROP_HEIGHT)
                .unwrap();
        let wide_rendered_width = wide_left.cell_width * wide.width() as f64;

        assert_eq!(wide_left.origin_x, CANVAS_PADDING);
        assert!(
            (wide_right.origin_x
                - (f64::from(BACKDROP_TILE_WIDTH) - CANVAS_PADDING - wide_rendered_width))
                .abs()
                < 1.0e-7
        );
        assert_cover(wide_left, BACKDROP_TILE_WIDTH, BACKDROP_HEIGHT);
        assert_cover(wide_right, BACKDROP_TILE_WIDTH, BACKDROP_HEIGHT);

        let tall = ContentBounds {
            left: 0,
            top: 0,
            right: 4,
            bottom: 100,
            content_top: 0,
        };
        let tall_top =
            RasterTransform::new(tall, 583, 2.0, 0.5, BACKDROP_TILE_WIDTH, BACKDROP_HEIGHT)
                .unwrap();
        let tall_bottom =
            RasterTransform::new(tall, 583, 2.0, 99.5, BACKDROP_TILE_WIDTH, BACKDROP_HEIGHT)
                .unwrap();
        let tall_rendered_height = tall_top.cell_height * tall.height() as f64;

        assert_eq!(tall_top.origin_y, CANVAS_PADDING);
        assert!(
            (tall_bottom.origin_y
                - (f64::from(BACKDROP_HEIGHT) - CANVAS_PADDING - tall_rendered_height))
                .abs()
                < 1.0e-7
        );
        assert_cover(tall_top, BACKDROP_TILE_WIDTH, BACKDROP_HEIGHT);
        assert_cover(tall_bottom, BACKDROP_TILE_WIDTH, BACKDROP_HEIGHT);
    }

    #[test]
    fn cover_transform_remains_finite_for_extreme_aspect_ratios() {
        let tall = ContentBounds {
            left: 0,
            top: 0,
            right: 3,
            bottom: 10_000,
            content_top: 0,
        };
        let wide = ContentBounds {
            left: 0,
            top: 0,
            right: 10_000,
            bottom: 3,
            content_top: 0,
        };

        for transform in [
            RasterTransform::new(
                tall,
                583,
                1.5,
                5_000.0,
                BACKDROP_TILE_WIDTH,
                BACKDROP_HEIGHT,
            )
            .unwrap(),
            RasterTransform::new(
                wide,
                583,
                5_000.0,
                1.5,
                BACKDROP_TILE_WIDTH,
                BACKDROP_HEIGHT,
            )
            .unwrap(),
        ] {
            assert!(transform.origin_x.is_finite());
            assert!(transform.origin_y.is_finite());
            assert!(transform.cell_width.is_finite());
            assert!(transform.cell_height.is_finite());
            assert_cover(transform, BACKDROP_TILE_WIDTH, BACKDROP_HEIGHT);
        }
    }

    #[test]
    fn text_marks_contribute_to_the_focal_without_counting_whitespace() {
        let mut content = line(4);
        content.text_flights.push(NfoRendererTextFlight {
            col: 4,
            text: "A  B".into(),
        });
        let grid = grid_with_line(20, 10, 17, content);
        let settings = NfoRenderSettings::default();
        let key = BackdropKey::new(&grid, &settings, 0.583);
        let bounds = visible_bounds(&grid).unwrap();

        assert_eq!(ink_focal(&grid, bounds, key), (6.0, 4.5));
    }

    #[test]
    fn zero_ink_uses_the_sparse_fallback_bounds_midpoint() {
        let grid = single_block_grid(NfoRendererBlockShape::FullBlock);
        let settings = NfoRenderSettings {
            text_color: Rgba::new(1.0, 1.0, 1.0, 0.0),
            art_color: Rgba::new(1.0, 1.0, 1.0, 0.0),
            ..NfoRenderSettings::default()
        };
        let key = BackdropKey::new(&grid, &settings, 0.583);
        let bounds = visible_bounds(&grid).unwrap();
        let expected = (
            (bounds.left as f64 + bounds.right as f64) * 0.5,
            (bounds.top as f64 + bounds.bottom as f64) * 0.5,
        );

        assert_eq!(bounds.content_top, 0);
        assert_eq!(ink_focal(&grid, bounds, key), expected);

        let empty = NfoRendererGrid {
            width: 80,
            height: 25,
            lines: Vec::new(),
            has_blocks: false,
            id: 18,
        };
        assert!(visible_bounds(&empty).is_none());
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
    fn mirror_repeat_distributes_blurred_energy_across_both_outer_quarters() {
        let grid = repeat_energy_grid();
        let settings = high_contrast_settings();
        let key = BackdropKey::new(&grid, &settings, 0.583);
        let raw = rasterize(&grid, key).unwrap();
        let blurred = imageops::fast_blur(&raw, BLUR_SIGMA);
        let quarter_width = BACKDROP_WIDTH / 4;
        let energetic_pixels = [0, 3].map(|quarter| {
            let left = quarter * quarter_width;
            let right = left + quarter_width;
            (0..BACKDROP_HEIGHT)
                .flat_map(|y| (left..right).map(move |x| (x, y)))
                .filter(|&(x, y)| blurred.get_pixel(x, y)[0] >= 12)
                .count()
        });

        assert!(
            energetic_pixels.iter().all(|&count| count > 0),
            "an outer quarter contained no ambient energy: {energetic_pixels:?}"
        );
        assert_eq!(
            energetic_pixels[0], energetic_pixels[1],
            "mirrored outer quarters contained different amounts of ambient energy"
        );
    }

    #[test]
    fn mirror_repeat_has_no_abrupt_center_seam_after_blur() {
        let grid = repeat_energy_grid();
        let settings = high_contrast_settings();
        let key = BackdropKey::new(&grid, &settings, 0.583);
        let raw = rasterize(&grid, key).unwrap();
        let blurred = imageops::fast_blur(&raw, BLUR_SIGMA);
        let seam_left = BACKDROP_TILE_WIDTH - 1;
        let seam_right = BACKDROP_TILE_WIDTH;
        let mut maximum_jump = 0;

        for y in 0..BACKDROP_HEIGHT {
            for channel in 0..3 {
                maximum_jump = maximum_jump.max(
                    blurred.get_pixel(seam_left, y)[channel]
                        .abs_diff(blurred.get_pixel(seam_right, y)[channel]),
                );
            }
        }

        assert!(
            maximum_jump <= 1,
            "center seam changed by {maximum_jump} levels after blur"
        );
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
