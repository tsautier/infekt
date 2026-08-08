use iced::advanced::layout;
use iced::advanced::renderer::Style;
use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget, mouse, overlay};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector};

/// Displays interactive content in an overlay anchored to another widget.
///
/// The anchor alone participates in normal layout. The overlay aligns its
/// right edge with the anchor and opens below it, while staying inside the
/// current viewport.
pub(crate) struct AnchoredOverlay<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    anchor: Element<'a, Message, Theme, Renderer>,
    overlay: Element<'a, Message, Theme, Renderer>,
    open: bool,
    gap: f32,
}

impl<'a, Message, Theme, Renderer> AnchoredOverlay<'a, Message, Theme, Renderer> {
    pub(crate) fn new(
        anchor: impl Into<Element<'a, Message, Theme, Renderer>>,
        overlay: impl Into<Element<'a, Message, Theme, Renderer>>,
        open: bool,
    ) -> Self {
        Self {
            anchor: anchor.into(),
            overlay: overlay.into(),
            open,
            gap: 0.0,
        }
    }

    pub(crate) fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for AnchoredOverlay<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.anchor), Tree::new(&self.overlay)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.anchor.as_widget(), self.overlay.as_widget()]);
    }

    fn size(&self) -> Size<Length> {
        self.anchor.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.anchor.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.anchor
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
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
        self.anchor.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
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
        self.anchor.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
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
        self.anchor.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.anchor
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        let mut children = tree.children.iter_mut();
        let anchor_overlay = self.anchor.as_widget_mut().overlay(
            children.next().expect("anchor tree"),
            layout,
            renderer,
            viewport,
            translation,
        );

        let menu_overlay = self.open.then(|| {
            overlay::Element::new(Box::new(Overlay {
                anchor_bounds: layout.bounds() + translation,
                content: &mut self.overlay,
                tree: children.next().expect("overlay tree"),
                viewport: *viewport,
                gap: self.gap,
            }))
        });

        if anchor_overlay.is_some() || menu_overlay.is_some() {
            Some(
                overlay::Group::with_children(
                    anchor_overlay.into_iter().chain(menu_overlay).collect(),
                )
                .overlay(),
            )
        } else {
            None
        }
    }
}

impl<'a, Message, Theme, Renderer> From<AnchoredOverlay<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(anchored: AnchoredOverlay<'a, Message, Theme, Renderer>) -> Self {
        Element::new(anchored)
    }
}

struct Overlay<'a, 'b, Message, Theme, Renderer> {
    anchor_bounds: Rectangle,
    content: &'b mut Element<'a, Message, Theme, Renderer>,
    tree: &'b mut Tree,
    viewport: Rectangle,
    gap: f32,
}

impl<Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for Overlay<'_, '_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, _bounds: Size) -> layout::Node {
        let content = self.content.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, self.viewport.size()),
        );
        let position =
            overlay_position(self.anchor_bounds, content.size(), self.viewport, self.gap);

        layout::Node::with_children(content.size(), vec![content]).move_to(position)
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        self.content.as_widget_mut().update(
            self.tree,
            event,
            layout.children().next().expect("overlay content layout"),
            cursor,
            renderer,
            clipboard,
            shell,
            &self.viewport,
        );
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout.children().next().expect("overlay content layout"),
            cursor,
            &self.viewport,
        );
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            self.tree,
            layout.children().next().expect("overlay content layout"),
            cursor,
            &self.viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.content.as_widget_mut().operate(
            self.tree,
            layout.children().next().expect("overlay content layout"),
            renderer,
            operation,
        );
    }
}

fn overlay_position(anchor: Rectangle, overlay: Size, viewport: Rectangle, gap: f32) -> Point {
    let viewport_right = viewport.x + viewport.width;
    let viewport_bottom = viewport.y + viewport.height;
    let max_x = (viewport_right - overlay.width).max(viewport.x);
    let max_y = (viewport_bottom - overlay.height).max(viewport.y);

    Point::new(
        (anchor.x + anchor.width - overlay.width).clamp(viewport.x, max_x),
        (anchor.y + anchor.height + gap).clamp(viewport.y, max_y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligns_the_overlay_to_the_anchor_instead_of_the_window_edge() {
        let position = overlay_position(
            Rectangle::new(Point::new(620.0, 8.0), Size::new(38.0, 38.0)),
            Size::new(170.0, 42.0),
            Rectangle::new(Point::ORIGIN, Size::new(1024.0, 800.0)),
            6.0,
        );

        assert_eq!(position, Point::new(488.0, 52.0));
    }

    #[test]
    fn keeps_the_overlay_inside_the_viewport() {
        let position = overlay_position(
            Rectangle::new(Point::new(8.0, 760.0), Size::new(38.0, 38.0)),
            Size::new(170.0, 80.0),
            Rectangle::new(Point::ORIGIN, Size::new(1024.0, 800.0)),
            6.0,
        );

        assert_eq!(position, Point::new(0.0, 720.0));
    }
}
