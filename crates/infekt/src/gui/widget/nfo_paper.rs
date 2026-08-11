use iced::advanced::layout;
use iced::advanced::renderer::{self, Style};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, mouse, overlay};
use iced::{Color, Element, Event, Length, Point, Rectangle, Shadow, Size, Vector};

const PAPER_PADDING: f32 = 24.0;
const PAPER_FADE_RADIUS: f32 = 24.0;
const PAPER_GLOW_INNER_RADIUS: f32 = 36.0;
const PAPER_GLOW_OUTER_RADIUS: f32 = 64.0;
const PAPER_GLOW_INNER_DARK_OPACITY: f32 = 0.22;
const PAPER_GLOW_INNER_LIGHT_OPACITY: f32 = 0.14;
const PAPER_GLOW_OUTER_DARK_OPACITY: f32 = 0.10;
const PAPER_GLOW_OUTER_LIGHT_OPACITY: f32 = 0.06;
const PAPER_GLOW_BLOCK_ART_OPACITY_SCALE: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NfoPaperStyle {
    background: Color,
    art: Color,
    is_dark: bool,
}

impl NfoPaperStyle {
    pub(crate) fn new(background: Color, art: Color, is_dark: bool) -> Self {
        Self {
            background,
            art,
            is_dark,
        }
    }
}

/// Draws an opaque paper around intrinsic NFO content.
///
/// A bidirectional scrollable compresses ordinary `Fill` containers to their
/// intrinsic width. This widget keeps those intrinsic scroll bounds, then
/// visually centers the paper whenever it fits inside the current viewport
/// and extends short papers to the bottom of that viewport.
pub(crate) struct NfoPaper<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    style: NfoPaperStyle,
    has_blocks: bool,
}

impl<'a, Message, Theme, Renderer> NfoPaper<'a, Message, Theme, Renderer> {
    pub(crate) fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        style: NfoPaperStyle,
        has_blocks: bool,
    ) -> Self {
        Self {
            content: content.into(),
            style,
            has_blocks,
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for NfoPaper<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.content.as_widget()]);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child_limits = limits
            .loose()
            .shrink(Size::new(PAPER_PADDING * 2.0, PAPER_PADDING * 2.0));
        let child =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &child_limits);
        let size = paper_size(child.size());

        layout::Node::with_children(
            size,
            vec![child.move_to(Point::new(PAPER_PADDING, PAPER_PADDING))],
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout.children().next().expect("NFO paper content layout"),
                renderer,
                operation,
            );
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let translation = paper_translation(layout.bounds(), *viewport);

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().expect("NFO paper content layout"),
            cursor - translation,
            renderer,
            clipboard,
            shell,
            &(*viewport - translation),
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let translation = paper_translation(layout.bounds(), *viewport);

        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().expect("NFO paper content layout"),
            cursor - translation,
            &(*viewport - translation),
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let translation = paper_translation(bounds, *viewport);
        let presented_bounds = presented_paper_bounds(bounds, *viewport);

        if !presented_bounds
            .expand(PAPER_GLOW_OUTER_RADIUS.max(PAPER_FADE_RADIUS))
            .intersects(viewport)
        {
            return;
        }

        renderer.with_layer(*viewport, |renderer| {
            for shadow in paper_glows(self.style.art, self.style.is_dark, self.has_blocks) {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: presented_bounds,
                        shadow,
                        ..renderer::Quad::default()
                    },
                    Color::TRANSPARENT,
                );
            }
            renderer.fill_quad(
                renderer::Quad {
                    bounds: presented_bounds,
                    shadow: paper_fade(self.style.background),
                    ..renderer::Quad::default()
                },
                self.style.background,
            );
        });

        renderer.with_translation(translation, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout.children().next().expect("NFO paper content layout"),
                cursor - translation,
                &(*viewport - translation),
            );
        });
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        let paper_translation = paper_translation(layout.bounds(), *viewport);

        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().expect("NFO paper content layout"),
            renderer,
            &(*viewport - paper_translation),
            translation + paper_translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<NfoPaper<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(paper: NfoPaper<'a, Message, Theme, Renderer>) -> Self {
        Element::new(paper)
    }
}

fn paper_size(content: Size) -> Size {
    Size::new(
        content.width + PAPER_PADDING * 2.0,
        content.height + PAPER_PADDING * 2.0,
    )
}

fn paper_translation(bounds: Rectangle, viewport: Rectangle) -> Vector {
    Vector::new(horizontal_offset(bounds.width, viewport.width), 0.0)
}

fn presented_paper_bounds(bounds: Rectangle, viewport: Rectangle) -> Rectangle {
    let mut presented = bounds + paper_translation(bounds, viewport);
    let height_to_viewport_bottom = (viewport.y + viewport.height - presented.y).max(0.0);
    presented.height = presented.height.max(height_to_viewport_bottom);
    presented
}

fn horizontal_offset(paper_width: f32, viewport_width: f32) -> f32 {
    ((viewport_width - paper_width) * 0.5).max(0.0)
}

fn paper_fade(background: Color) -> Shadow {
    Shadow {
        color: background,
        blur_radius: PAPER_FADE_RADIUS,
        ..Shadow::default()
    }
}

fn paper_glows(art_color: Color, is_dark: bool, has_blocks: bool) -> [Shadow; 2] {
    let (outer_opacity, inner_opacity) = if is_dark {
        (PAPER_GLOW_OUTER_DARK_OPACITY, PAPER_GLOW_INNER_DARK_OPACITY)
    } else {
        (
            PAPER_GLOW_OUTER_LIGHT_OPACITY,
            PAPER_GLOW_INNER_LIGHT_OPACITY,
        )
    };
    let content_opacity = if has_blocks {
        PAPER_GLOW_BLOCK_ART_OPACITY_SCALE
    } else {
        1.0
    };

    [
        Shadow {
            color: Color {
                a: art_color.a * outer_opacity * content_opacity,
                ..art_color
            },
            blur_radius: PAPER_GLOW_OUTER_RADIUS,
            ..Shadow::default()
        },
        Shadow {
            color: Color {
                a: art_color.a * inner_opacity * content_opacity,
                ..art_color
            },
            blur_radius: PAPER_GLOW_INNER_RADIUS,
            ..Shadow::default()
        },
    ]
}

#[cfg(test)]
mod tests {
    use iced::{Color, Rectangle, Shadow, Size, Vector};

    use super::{
        PAPER_FADE_RADIUS, PAPER_GLOW_BLOCK_ART_OPACITY_SCALE, PAPER_GLOW_INNER_DARK_OPACITY,
        PAPER_GLOW_INNER_LIGHT_OPACITY, PAPER_GLOW_INNER_RADIUS, PAPER_GLOW_OUTER_DARK_OPACITY,
        PAPER_GLOW_OUTER_LIGHT_OPACITY, PAPER_GLOW_OUTER_RADIUS, PAPER_PADDING, horizontal_offset,
        paper_fade, paper_glows, paper_size, paper_translation, presented_paper_bounds,
    };

    #[test]
    fn paper_size_adds_equal_padding_to_every_edge() {
        assert_eq!(PAPER_PADDING, 24.0);
        assert_eq!(paper_size(Size::new(560.0, 400.0)), Size::new(608.0, 448.0));
    }

    #[test]
    fn narrow_paper_is_centered_in_the_viewport() {
        assert_eq!(horizontal_offset(608.0, 1000.0), 196.0);

        let translation = paper_translation(
            Rectangle::new([0.0, 12.0].into(), Size::new(608.0, 448.0)),
            Rectangle::new([0.0, 0.0].into(), Size::new(1000.0, 700.0)),
        );

        assert_eq!(translation, [196.0, 0.0].into());
    }

    #[test]
    fn wide_paper_keeps_its_scroll_origin() {
        assert_eq!(horizontal_offset(1200.0, 900.0), 0.0);
        assert_eq!(horizontal_offset(900.0, 900.0), 0.0);
    }

    #[test]
    fn short_paper_extends_to_the_bottom_of_the_viewport() {
        let bounds = Rectangle::new([0.0, 12.0].into(), Size::new(608.0, 448.0));
        let viewport = Rectangle::new([0.0, 0.0].into(), Size::new(1_000.0, 700.0));

        assert_eq!(
            presented_paper_bounds(bounds, viewport),
            Rectangle::new([196.0, 12.0].into(), Size::new(608.0, 688.0))
        );
    }

    #[test]
    fn long_paper_keeps_its_intrinsic_scroll_height() {
        let bounds = Rectangle::new([0.0, 0.0].into(), Size::new(608.0, 900.0));
        let viewport = Rectangle::new([0.0, 0.0].into(), Size::new(1_000.0, 700.0));

        assert_eq!(
            presented_paper_bounds(bounds, viewport),
            Rectangle::new([196.0, 0.0].into(), Size::new(608.0, 900.0))
        );
    }

    #[test]
    fn scrolling_a_long_paper_does_not_extend_its_document_height() {
        let bounds = Rectangle::new([0.0, 62.0].into(), Size::new(608.0, 1_200.0));
        let viewport = Rectangle::new([0.0, 562.0].into(), Size::new(1_000.0, 700.0));

        assert_eq!(
            presented_paper_bounds(bounds, viewport),
            Rectangle::new([196.0, 62.0].into(), Size::new(608.0, 1_200.0))
        );
    }

    #[test]
    fn paper_fade_is_theme_colored_and_does_not_change_layout() {
        let background = Color::from_rgb(0.125, 0.25, 0.75);

        assert_eq!(PAPER_FADE_RADIUS, PAPER_PADDING);
        assert_eq!(
            paper_fade(background),
            Shadow {
                color: background,
                offset: Vector::ZERO,
                blur_radius: 24.0,
            }
        );
        assert_eq!(paper_size(Size::new(560.0, 400.0)), Size::new(608.0, 448.0));
    }

    #[test]
    fn paper_glow_uses_the_art_color_without_changing_layout() {
        let art_color = Color::from_rgba(0.125, 0.75, 0.5, 0.5);

        assert_eq!(
            paper_glows(art_color, true, false),
            [
                Shadow {
                    color: Color::from_rgba(0.125, 0.75, 0.5, 0.5 * PAPER_GLOW_OUTER_DARK_OPACITY,),
                    offset: Vector::ZERO,
                    blur_radius: PAPER_GLOW_OUTER_RADIUS,
                },
                Shadow {
                    color: Color::from_rgba(0.125, 0.75, 0.5, 0.5 * PAPER_GLOW_INNER_DARK_OPACITY,),
                    offset: Vector::ZERO,
                    blur_radius: PAPER_GLOW_INNER_RADIUS,
                },
            ]
        );
        assert_eq!(paper_size(Size::new(560.0, 400.0)), Size::new(608.0, 448.0));
    }

    #[test]
    fn bright_paper_glow_is_more_subtle() {
        let art_color = Color::from_rgb(0.125, 0.75, 0.5);
        let [outer, inner] = paper_glows(art_color, false, false);

        assert_eq!(outer.color.a, PAPER_GLOW_OUTER_LIGHT_OPACITY);
        assert_eq!(inner.color.a, PAPER_GLOW_INNER_LIGHT_OPACITY);
        assert!(outer.color.a < PAPER_GLOW_OUTER_DARK_OPACITY);
        assert!(inner.color.a < PAPER_GLOW_INNER_DARK_OPACITY);
    }

    #[test]
    fn block_art_halves_both_paper_glow_layers() {
        let art_color = Color::from_rgba(0.125, 0.75, 0.5, 0.8);

        for is_dark in [false, true] {
            let regular = paper_glows(art_color, is_dark, false);
            let with_blocks = paper_glows(art_color, is_dark, true);

            for (regular, with_blocks) in regular.into_iter().zip(with_blocks) {
                assert_eq!(with_blocks.color.r, regular.color.r);
                assert_eq!(with_blocks.color.g, regular.color.g);
                assert_eq!(with_blocks.color.b, regular.color.b);
                assert_eq!(with_blocks.blur_radius, regular.blur_radius);
                assert_eq!(with_blocks.offset, regular.offset);
                assert_eq!(
                    with_blocks.color.a,
                    regular.color.a * PAPER_GLOW_BLOCK_ART_OPACITY_SCALE
                );
            }
        }
    }
}
