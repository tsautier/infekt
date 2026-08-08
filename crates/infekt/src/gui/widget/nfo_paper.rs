use iced::advanced::layout;
use iced::advanced::renderer::{self, Style};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, mouse, overlay};
use iced::{Color, Element, Event, Length, Point, Rectangle, Size, Vector};

const PAPER_PADDING: f32 = 24.0;

/// Draws an opaque paper around intrinsic NFO content.
///
/// A bidirectional scrollable compresses ordinary `Fill` containers to their
/// intrinsic width. This widget keeps those intrinsic scroll bounds, then
/// visually centers the paper whenever it fits inside the current viewport.
pub(crate) struct NfoPaper<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    background: Color,
}

impl<'a, Message, Theme, Renderer> NfoPaper<'a, Message, Theme, Renderer> {
    pub(crate) fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        background: Color,
    ) -> Self {
        Self {
            content: content.into(),
            background,
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
        let presented_bounds = bounds + translation;

        if !presented_bounds.intersects(viewport) {
            return;
        }

        renderer.fill_quad(
            renderer::Quad {
                bounds: presented_bounds,
                ..renderer::Quad::default()
            },
            self.background,
        );

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

fn horizontal_offset(paper_width: f32, viewport_width: f32) -> f32 {
    ((viewport_width - paper_width) * 0.5).max(0.0)
}

#[cfg(test)]
mod tests {
    use iced::{Rectangle, Size};

    use super::{PAPER_PADDING, horizontal_offset, paper_size, paper_translation};

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
}
