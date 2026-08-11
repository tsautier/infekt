mod classic_view;

use std::sync::Arc;

use iced::Length::Fill;
use iced::widget::operation;
use iced::widget::scrollable::{AbsoluteOffset, Direction, RelativeOffset, Scrollbar};
use iced::widget::{self, scrollable};
use iced::{Color, Element, Task, Vector};

use crate::core::nfo_data::NfoData;
use crate::settings::NfoRenderSettings;

use super::widget::enhanced_nfo_view::EnhancedNfoView;
use super::widget::nfo_paper::{NfoPaper, NfoPaperStyle};
use super::{shell_style::ShellTokens, utils::to_iced_color};

const ENHANCED_SCROLL_ID: &str = "enhanced view";
pub(super) const CLASSIC_SCROLL_ID: &str = "main view classic";
pub(super) const TEXT_ONLY_SCROLL_ID: &str = "main view stripped";
const BACKDROP_PARALLAX_FACTOR: f32 = 0.10;
pub(crate) const BACKDROP_PARALLAX_LIMIT: f32 = 48.0;

#[derive(Default)]
pub struct InfektMainView {
    active_tab: TabId,
    active_render_settings: Arc<NfoRenderSettings>,
    scroll_offsets: [AbsoluteOffset; 3],
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Copy)]
pub enum TabId {
    #[default]
    Enhanced,
    Classic,
    TextOnly,
}

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(TabId),
    RenderSettingsChanged(Arc<NfoRenderSettings>),
    Scrolled(TabId, AbsoluteOffset),
}

impl InfektMainView {
    pub fn active_tab(&self) -> TabId {
        self.active_tab
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::TabSelected(selected) => self.active_tab = selected,
            Message::RenderSettingsChanged(settings) => self.active_render_settings = settings,
            Message::Scrolled(tab, offset) => self.scroll_offsets[tab.index()] = offset,
        }
    }

    pub fn reset_scroll(&mut self) -> Task<Message> {
        self.scroll_offsets = [AbsoluteOffset::default(); 3];

        Task::batch([
            operation::snap_to(widget::Id::new(ENHANCED_SCROLL_ID), RelativeOffset::START),
            operation::snap_to(widget::Id::new(CLASSIC_SCROLL_ID), RelativeOffset::START),
            operation::snap_to(widget::Id::new(TEXT_ONLY_SCROLL_ID), RelativeOffset::START),
        ])
    }

    pub(crate) fn backdrop_translation(&self) -> Vector {
        let offset = self.scroll_offsets[self.active_tab.index()];

        Vector::new(
            parallax_translation(offset.x),
            parallax_translation(offset.y),
        )
    }

    pub fn view<'a>(&self, current_nfo: &'a NfoData) -> Element<'a, Message> {
        match self.active_tab {
            TabId::Enhanced => self.enhanced_tab(current_nfo),
            TabId::Classic => self.classic_tab(current_nfo, false),
            TabId::TextOnly => self.classic_tab(current_nfo, true),
        }
    }

    fn enhanced_tab<'a>(&self, current_nfo: &'a NfoData) -> Element<'a, Message> {
        let nfo = EnhancedNfoView::new(self.active_render_settings.clone(), current_nfo);
        let content: Element<'a, Message> = if current_nfo.is_loaded() {
            NfoPaper::new(
                nfo,
                nfo_paper_style(self.active_render_settings.as_ref()),
                current_nfo.has_blocks(),
            )
            .into()
        } else {
            nfo.into()
        };

        scrollable(content)
            .id(widget::Id::new(ENHANCED_SCROLL_ID))
            .on_scroll(|viewport| Message::Scrolled(TabId::Enhanced, viewport.absolute_offset()))
            .direction(Direction::Both {
                vertical: Scrollbar::default(),
                horizontal: Scrollbar::default(),
            })
            .width(Fill)
            .height(Fill)
            .into()
    }
}

impl TabId {
    fn index(self) -> usize {
        match self {
            Self::Enhanced => 0,
            Self::Classic => 1,
            Self::TextOnly => 2,
        }
    }
}

fn parallax_translation(scroll_offset: f32) -> f32 {
    -(scroll_offset.max(0.0) * BACKDROP_PARALLAX_FACTOR).min(BACKDROP_PARALLAX_LIMIT)
}

pub(super) fn nfo_paper_color(settings: &NfoRenderSettings) -> Color {
    let background = settings.background_color;

    Color::from_rgb(background.red, background.green, background.blue)
}

pub(super) fn nfo_paper_style(settings: &NfoRenderSettings) -> NfoPaperStyle {
    NfoPaperStyle::new(
        nfo_paper_color(settings),
        to_iced_color(settings.art_color),
        ShellTokens::from_settings(settings).is_dark,
    )
}

#[cfg(test)]
mod tests {
    use iced::Color;
    use iced::Vector;
    use iced::widget::scrollable::AbsoluteOffset;
    use palette::rgb::Rgb;

    use super::{InfektMainView, Message, TabId, nfo_paper_color, nfo_paper_style};
    use crate::settings::NfoRenderSettings;

    #[test]
    fn nfo_paper_uses_the_exact_opaque_theme_background() {
        let settings = NfoRenderSettings {
            background_color: Rgb::new(0.125, 0.25, 0.75),
            ..NfoRenderSettings::default()
        };

        let background = nfo_paper_color(&settings);

        assert_eq!(background, Color::from_rgb(0.125, 0.25, 0.75));
        assert_eq!(background.a, 1.0);
    }

    #[test]
    fn nfo_paper_style_tracks_the_art_color_and_shell_brightness() {
        let settings = NfoRenderSettings {
            background_color: Rgb::new(0.01, 0.02, 0.03),
            art_color: palette::rgb::Rgba::new(0.125, 0.75, 0.5, 0.4),
            ..NfoRenderSettings::default()
        };

        let style = nfo_paper_style(&settings);

        assert_eq!(
            style,
            super::NfoPaperStyle::new(
                Color::from_rgb(0.01, 0.02, 0.03),
                Color::from_rgba(0.125, 0.75, 0.5, 0.4),
                true,
            )
        );
    }

    #[test]
    fn backdrop_parallax_tracks_each_view_and_resets_with_the_paper() {
        let mut view = InfektMainView::default();

        view.update(Message::Scrolled(
            TabId::Enhanced,
            AbsoluteOffset {
                x: 100.0,
                y: 1_000.0,
            },
        ));
        assert_eq!(view.backdrop_translation(), Vector::new(-10.0, -48.0));

        view.update(Message::TabSelected(TabId::Classic));
        assert_eq!(view.backdrop_translation(), Vector::ZERO);
        view.update(Message::Scrolled(
            TabId::Classic,
            AbsoluteOffset { x: 500.0, y: 250.0 },
        ));
        assert_eq!(view.backdrop_translation(), Vector::new(-48.0, -25.0));

        let _ = view.reset_scroll();
        assert_eq!(view.backdrop_translation(), Vector::ZERO);
        view.update(Message::TabSelected(TabId::Enhanced));
        assert_eq!(view.backdrop_translation(), Vector::ZERO);
    }
}
