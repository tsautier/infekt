use iced::advanced::layout;
use iced::advanced::renderer::Style;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, mouse, overlay};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector};

/// Places a leading widget directly before trailing metadata.
///
/// The trailing widget is measured first so its intrinsic width remains
/// available. The leading widget receives the remaining width and is clipped
/// when its contents are wider than that allocation.
pub(crate) struct AdjacentPair<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    children: [Element<'a, Message, Theme, Renderer>; 2],
    spacing: f32,
}

impl<'a, Message, Theme, Renderer> AdjacentPair<'a, Message, Theme, Renderer> {
    pub(crate) fn new(
        leading: impl Into<Element<'a, Message, Theme, Renderer>>,
        trailing: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self {
            children: [leading.into(), trailing.into()],
            spacing: 0.0,
        }
    }

    pub(crate) fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.max(0.0);
        self
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for AdjacentPair<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
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
        let (leading, trailing) = self.children.split_at_mut(1);
        let (leading_tree, trailing_tree) = tree.children.split_at_mut(1);

        let trailing_limits = intrinsic_limits(limits.max());
        let trailing_node =
            trailing[0]
                .as_widget_mut()
                .layout(&mut trailing_tree[0], renderer, &trailing_limits);
        let trailing_size = trailing_node.size();

        let leading_max_width = (limits.max().width - trailing_size.width - self.spacing).max(0.0);
        let leading_limits = intrinsic_limits(Size::new(leading_max_width, limits.max().height));
        let leading_node =
            leading[0]
                .as_widget_mut()
                .layout(&mut leading_tree[0], renderer, &leading_limits);
        let leading_size = leading_node.size();

        let spacing = if leading_size.width > 0.0 && trailing_size.width > 0.0 {
            self.spacing
        } else {
            0.0
        };
        let intrinsic_size = Size::new(
            leading_size.width + spacing + trailing_size.width,
            leading_size.height.max(trailing_size.height),
        );
        let size = limits.resolve(Length::Shrink, Length::Shrink, intrinsic_size);

        layout::Node::with_children(
            size,
            vec![
                leading_node.move_to(Point::new(0.0, (size.height - leading_size.height) / 2.0)),
                trailing_node.move_to(Point::new(
                    leading_size.width + spacing,
                    (size.height - trailing_size.height) / 2.0,
                )),
            ],
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
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, tree), layout)| {
                    child
                        .as_widget_mut()
                        .operate(tree, layout, renderer, operation);
                });
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
        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, tree), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
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
        let mut layouts = layout.children();
        let leading_layout = layouts.next().expect("leading layout");
        let trailing_layout = layouts.next().expect("trailing layout");

        if let Some(leading_viewport) = leading_layout.bounds().intersection(viewport) {
            self.children[0].as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                leading_layout,
                cursor,
                &leading_viewport,
            );
        }

        if trailing_layout.bounds().intersects(viewport) {
            self.children[1].as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                style,
                trailing_layout,
                cursor,
                viewport,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<AdjacentPair<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(pair: AdjacentPair<'a, Message, Theme, Renderer>) -> Self {
        Element::new(pair)
    }
}

fn intrinsic_limits(max: Size) -> layout::Limits {
    layout::Limits::new(Size::ZERO, max)
        .width(Length::Shrink)
        .height(Length::Shrink)
}
