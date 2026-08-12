use iced::advanced::image;
use iced::advanced::layout;
use iced::advanced::widget::Tree;
use iced::advanced::{Layout, Widget};
use iced::widget::image::Handle;
use iced::{ContentFit, Element, Length, Point, Rectangle, Size, Vector};

const BASE_SCALE: f32 = 1.08;
const EDGE_SAFETY: f32 = 2.0;

/// Draws the fixed backdrop with enough real overscan to absorb parallax.
///
/// The stock image widget clips to its unshifted layout bounds, which exposes
/// an edge after translation. Drawing the handle directly lets the viewport be
/// the clip while preserving a fixed overscan margin on every side.
pub(crate) struct BackdropParallax {
    handle: Handle,
    translation: Vector,
    opacity: f32,
    minimum_overscan: f32,
}

impl BackdropParallax {
    pub(crate) fn new(
        handle: Handle,
        translation: Vector,
        opacity: f32,
        minimum_overscan: f32,
    ) -> Self {
        Self {
            handle,
            translation,
            opacity,
            minimum_overscan,
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for BackdropParallax
where
    Renderer: iced::advanced::Renderer + image::Renderer<Handle = Handle>,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(Length::Fill, Length::Fill, Size::ZERO))
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &iced::advanced::renderer::Style,
        layout: Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let source = renderer.measure_image(&self.handle).unwrap_or_default();
        if source.width == 0 || source.height == 0 || bounds.width <= 0.0 || bounds.height <= 0.0 {
            return;
        }

        let fitted = ContentFit::Cover.fit(
            Size::new(source.width as f32, source.height as f32),
            bounds.size(),
        );
        let scale = backdrop_scale(fitted, self.minimum_overscan);
        let final_size = fitted * scale;
        let drawing_bounds = Rectangle::new(
            Point::new(
                bounds.center_x() - final_size.width * 0.5 + self.translation.x,
                bounds.center_y() - final_size.height * 0.5 + self.translation.y,
            ),
            final_size,
        );
        let Some(clip_bounds) = bounds.intersection(viewport) else {
            return;
        };

        renderer.draw_image(
            image::Image::new(self.handle.clone())
                .filter_method(image::FilterMethod::Linear)
                .opacity(self.opacity)
                .snap(true),
            drawing_bounds,
            clip_bounds,
        );
    }
}

impl<'a, Message, Theme, Renderer> From<BackdropParallax> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + image::Renderer<Handle = Handle> + 'a,
{
    fn from(parallax: BackdropParallax) -> Self {
        Element::new(parallax)
    }
}

fn backdrop_scale(fitted: Size, minimum_overscan: f32) -> f32 {
    let overscan = minimum_overscan.max(0.0) + EDGE_SAFETY;
    let horizontal = 1.0 + overscan * 2.0 / fitted.width.max(1.0);
    let vertical = 1.0 + overscan * 2.0 / fitted.height.max(1.0);

    BASE_SCALE.max(horizontal).max(vertical)
}

#[cfg(test)]
mod tests {
    use iced::Size;

    use super::{BASE_SCALE, EDGE_SAFETY, backdrop_scale};

    #[test]
    fn scale_guarantees_the_requested_overscan_on_every_edge() {
        let fitted = Size::new(960.0, 600.0);
        let scale = backdrop_scale(fitted, 48.0);

        let vertical_overscan = fitted.height * (scale - 1.0) * 0.5;

        assert!((vertical_overscan - (48.0 + EDGE_SAFETY)).abs() < 1.0e-4);
        assert!(fitted.width * (scale - 1.0) * 0.5 >= 48.0 + EDGE_SAFETY);
    }

    #[test]
    fn large_viewports_keep_the_existing_visual_scale() {
        assert_eq!(
            backdrop_scale(Size::new(2_048.0, 1_280.0), 48.0),
            BASE_SCALE
        );
    }
}
