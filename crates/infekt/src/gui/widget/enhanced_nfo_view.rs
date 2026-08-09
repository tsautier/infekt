use std::sync::Arc;

use iced::Point;
use iced::advanced::graphics::geometry::{Cache, Frame, Text};
use iced::advanced::layout;
use iced::advanced::renderer::Style;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{self, Clipboard, Layout, Shell, Widget};
use iced::alignment;
use iced::mouse;
use iced::{Color, Element, Event, Length, Rectangle, Renderer, Size, Vector};

use crate::core::nfo_data::NfoData;
use crate::core::nfo_renderer_grid::{NfoRendererBlockShape, NfoRendererGrid, NfoRendererLine};
use crate::gui::utils::to_iced_color;
use crate::settings::NfoRenderSettings;

pub struct EnhancedNfoView<'a> {
    render_settings: Arc<NfoRenderSettings>,
    block_width_float: f32,
    block_height_float: f32,
    renderer_grid: Option<&'a NfoRendererGrid>,
}

impl<'a> EnhancedNfoView<'a> {
    pub fn new(render_settings: Arc<NfoRenderSettings>, current_nfo: &'a NfoData) -> Self {
        Self {
            renderer_grid: current_nfo.get_renderer_grid(),
            block_width_float: render_settings.enhanced_view_block_width as f32,
            block_height_float: render_settings.enhanced_view_block_height as f32,
            render_settings,
        }
    }
}

#[derive(Default)]
struct State {
    cache_key: Option<(u64, u64)>,
    cache: Vec<Cache<Renderer>>,
}

// Number of lines in one geometry cache entry:
const CACHE_STRIDE_LINES: usize = 100;

impl<Message, Theme> Widget<Message, Theme, Renderer> for EnhancedNfoView<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn diff(&self, tree: &mut Tree) {
        self.sync_cache(tree.state.downcast_mut::<State>());
    }

    fn size(&self) -> Size<Length> {
        let rows = self.renderer_grid.map_or(0.0, |g| g.height as f32);
        let columns = self.renderer_grid.map_or(0.0, |g| g.width as f32);

        Size {
            width: Length::Fixed(columns * self.block_width_float),
            height: Length::Fixed(rows * self.block_height_float),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let rows = self.renderer_grid.map_or(0.0, |g| g.height as f32);
        let columns = self.renderer_grid.map_or(0.0, |g| g.width as f32);

        layout::atomic(
            limits,
            Length::Fixed(columns * self.block_width_float),
            rows * self.block_height_float,
        )
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        _event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();

        if self.sync_cache(state) {
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;

        let Some(grid) = self.renderer_grid else {
            return;
        };

        /*println!(
            "NfoViewEnhanced::draw() - viewport: {:?} - bounds: {:?}",
            viewport,
            layout.bounds()
        );*/

        // XXX: heavily WIP !!!

        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();

        if state.cache.is_empty() {
            eprintln!("NfoViewEnhanced::draw() - cache is uninitialized");

            return;
        }

        let Some((first_visible_line, last_visible_line)) = visible_line_range(
            bounds.y,
            self.block_height_float,
            grid.height,
            viewport.y,
            viewport.height,
        ) else {
            return;
        };

        let first_cache_index = first_visible_line / CACHE_STRIDE_LINES;
        let last_cache_index = (last_visible_line / CACHE_STRIDE_LINES).min(state.cache.len() - 1);

        renderer.start_layer(*viewport);

        let cache_bounds = Size {
            width: self.block_width_float * grid.width as f32,
            height: CACHE_STRIDE_LINES as f32 * self.block_height_float,
        };

        (first_cache_index..=last_cache_index).for_each(|cache_index| {
            let first_line = cache_index * CACHE_STRIDE_LINES;
            let last_line = (cache_index + 1) * CACHE_STRIDE_LINES - 1;

            let y_offset = first_line as f32 * self.block_height_float;

            let geometry =
                state
                    .cache
                    .get(cache_index)
                    .unwrap()
                    .draw(renderer, cache_bounds, |frame| {
                        for line in &grid.lines {
                            if line.row < first_line || line.row > last_line {
                                continue;
                            }

                            line.text_flights.iter().for_each(|flight| {
                                let x = flight.col as f32 * self.block_width_float;
                                let y = line.row as f32 * self.block_height_float - y_offset;

                                // XXX: this is bullshit
                                frame.fill_text(Text {
                                    content: flight.text.clone(),
                                    position: Point { x, y },
                                    size: iced::Pixels(self.block_height_float),
                                    color: to_iced_color(self.render_settings.text_color),
                                    align_x: alignment::Horizontal::Left.into(),
                                    align_y: alignment::Vertical::Top,
                                    // horizontal_alignment: alignment::Horizontal::Left,
                                    // vertical_alignment: alignment::Vertical::Top,
                                    line_height: advanced::text::LineHeight::Absolute(
                                        iced::Pixels(self.block_height_float),
                                    ),
                                    font: iced::Font::with_name("Cascadia Mono"), // XXX: how to take from settings?
                                    shaping: advanced::text::Shaping::Basic,
                                    max_width: f32::INFINITY,
                                });
                            });
                        }

                        // XXX: pass for blur
                        // XXX: layers?

                        render_blocks(
                            &mut grid
                                .lines
                                .iter()
                                .filter(|l| l.row >= first_line && l.row <= last_line),
                            y_offset,
                            self.render_settings.enhanced_view_block_width,
                            self.render_settings.enhanced_view_block_height,
                            to_iced_color(self.render_settings.art_color),
                            frame,
                        );
                    });

            let bounds_translation = Vector::new(bounds.x, bounds.y + y_offset);

            renderer.with_translation(bounds_translation, |renderer| {
                use iced::advanced::graphics::geometry::Renderer as _;

                renderer.draw_geometry(geometry);
            });
        });

        renderer.end_layer();
    }
}

fn visible_line_range(
    content_top: f32,
    line_height: f32,
    line_count: usize,
    viewport_top: f32,
    viewport_height: f32,
) -> Option<(usize, usize)> {
    if line_count == 0 || line_height <= 0.0 || viewport_height <= 0.0 {
        return None;
    }

    let content_height = line_count as f32 * line_height;
    let visible_top = (viewport_top - content_top).clamp(0.0, content_height);
    let visible_bottom = (viewport_top + viewport_height - content_top).clamp(0.0, content_height);

    if visible_top >= visible_bottom {
        return None;
    }

    let first = (visible_top / line_height).floor() as usize;
    let last = ((visible_bottom / line_height).ceil() as usize)
        .saturating_sub(1)
        .min(line_count - 1);

    Some((first, last))
}

impl EnhancedNfoView<'_> {
    fn sync_cache(&self, state: &mut State) -> bool {
        let cache_key = self
            .renderer_grid
            .map(|grid| (grid.id, self.render_settings.hash()));

        if state.cache_key == cache_key {
            return false;
        }

        state.cache_key = cache_key;
        state.cache.clear();

        if let Some(grid) = self.renderer_grid {
            state
                .cache
                .resize_with(grid.height / CACHE_STRIDE_LINES + 1, Default::default);
        }

        true
    }
}

fn render_blocks(
    lines: &mut dyn Iterator<Item = &NfoRendererLine>,
    y_offset: f32,
    block_width: u16,
    block_height: u16,
    block_color: Color,
    frame: &mut Frame<Renderer>,
) {
    let block_size = Size::new(block_width as f32, block_height as f32);

    for line in lines {
        let row = line.row;
        let y = row as f32 * block_size.height - y_offset;

        for block_group in &line.block_groups {
            for (block_index, block_shape) in block_group.blocks.iter().enumerate() {
                let x = (block_group.col + block_index) as f32 * block_size.width;

                let opacity: f32 = match block_shape {
                    NfoRendererBlockShape::FullBlockLightShade => 90.0 / 255.0,
                    NfoRendererBlockShape::FullBlockMediumShade => 140.0 / 255.0,
                    NfoRendererBlockShape::FullBlockDarkShade => 190.0 / 255.0,
                    _ => 1.0,
                };

                draw_block(
                    Point::new(x, y),
                    block_size,
                    block_shape,
                    block_color.scale_alpha(opacity),
                    frame,
                );
            }
        }
    }
}

#[inline]
fn draw_block(
    top_left: Point,
    block_size: Size,
    block_shape: &NfoRendererBlockShape,
    color: Color,
    frame: &mut Frame<Renderer>,
) {
    let half_block_size = Size::new(block_size.width * 0.5, block_size.height * 0.5);
    let half_vertical_block_size = Size::new(half_block_size.width, block_size.height);
    let half_horizontal_block_size = Size::new(block_size.width, half_block_size.height);
    let three_quarters_block_size = Size::new(block_size.width * 0.75, block_size.height * 0.75);

    match block_shape {
        NfoRendererBlockShape::FullBlock
        | NfoRendererBlockShape::FullBlockLightShade
        | NfoRendererBlockShape::FullBlockMediumShade
        | NfoRendererBlockShape::FullBlockDarkShade => {
            frame.fill_rectangle(top_left, block_size, color);
        }
        NfoRendererBlockShape::LowerHalf => {
            frame.fill_rectangle(
                Point::new(top_left.x, top_left.y + half_horizontal_block_size.height),
                half_horizontal_block_size,
                color,
            );
        }
        NfoRendererBlockShape::UpperHalf => {
            frame.fill_rectangle(top_left, half_horizontal_block_size, color);
        }
        NfoRendererBlockShape::RightHalf => {
            frame.fill_rectangle(
                Point::new(top_left.x + half_vertical_block_size.width, top_left.y),
                half_vertical_block_size,
                color,
            );
        }
        NfoRendererBlockShape::LeftHalf => {
            frame.fill_rectangle(top_left, half_vertical_block_size, color);
        }
        NfoRendererBlockShape::BlackSquare => {
            frame.fill_rectangle(
                Point::new(
                    top_left.x + (block_size.width - three_quarters_block_size.width) * 0.5,
                    top_left.y + (block_size.height - three_quarters_block_size.height) * 0.5,
                ),
                three_quarters_block_size,
                color,
            );
        }
        NfoRendererBlockShape::BlackSquareSmall => {
            frame.fill_rectangle(
                Point::new(
                    top_left.x + block_size.width * 0.25,
                    top_left.y + block_size.height * 0.25,
                ),
                half_block_size,
                color,
            );
        }
        _ => {}
    }
}

impl<'a, Message, Theme> From<EnhancedNfoView<'a>> for Element<'a, Message, Theme, Renderer> {
    fn from(w: EnhancedNfoView<'a>) -> Self {
        Self::new(w)
    }
}

#[cfg(test)]
mod tests {
    use super::{CACHE_STRIDE_LINES, visible_line_range};

    #[test]
    fn visible_lines_ignore_padding_outside_the_rendered_grid() {
        assert_eq!(visible_line_range(24.0, 12.0, 10, 0.0, 48.0), Some((0, 1)));
        assert_eq!(visible_line_range(24.0, 12.0, 10, 144.0, 24.0), None);
    }

    #[test]
    fn visible_lines_clamp_at_the_last_rendered_row() {
        for line_count in [99, 100, 101] {
            let content_top = 24.0;
            let content_bottom = content_top + line_count as f32 * 12.0;
            let range =
                visible_line_range(content_top, 12.0, line_count, content_bottom - 12.0, 36.0);

            assert_eq!(range, Some((line_count - 1, line_count - 1)));

            let last_cache = range.unwrap().1 / CACHE_STRIDE_LINES;
            let cache_count = line_count / CACHE_STRIDE_LINES + 1;
            assert!(last_cache < cache_count);
        }
    }

    #[test]
    fn visible_lines_handle_empty_and_exact_boundaries() {
        assert_eq!(visible_line_range(24.0, 12.0, 0, 24.0, 120.0), None);
        assert_eq!(visible_line_range(24.0, 12.0, 4, 36.0, 24.0), Some((1, 2)));
        assert_eq!(visible_line_range(24.0, 0.0, 4, 24.0, 48.0), None);
    }
}
