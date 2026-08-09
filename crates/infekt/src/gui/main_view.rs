mod classic_view;

use std::sync::Arc;

use iced::Length::Fill;
use iced::widget::operation;
use iced::widget::scrollable::{Direction, RelativeOffset, Scrollbar};
use iced::widget::{self, scrollable};
use iced::{Color, Element, Task};

use crate::core::nfo_data::NfoData;
use crate::settings::NfoRenderSettings;

use super::widget::enhanced_nfo_view::EnhancedNfoView;
use super::widget::nfo_paper::NfoPaper;

const ENHANCED_SCROLL_ID: &str = "enhanced view";
pub(super) const CLASSIC_SCROLL_ID: &str = "main view classic";
pub(super) const TEXT_ONLY_SCROLL_ID: &str = "main view stripped";

#[derive(Default)]
pub struct InfektMainView {
    active_tab: TabId,
    active_render_settings: Arc<NfoRenderSettings>,
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
}

impl InfektMainView {
    pub fn active_tab(&self) -> TabId {
        self.active_tab
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::TabSelected(selected) => self.active_tab = selected,
            Message::RenderSettingsChanged(settings) => self.active_render_settings = settings,
        }
    }

    pub fn reset_scroll(&self) -> Task<Message> {
        Task::batch([
            operation::snap_to(widget::Id::new(ENHANCED_SCROLL_ID), RelativeOffset::START),
            operation::snap_to(widget::Id::new(CLASSIC_SCROLL_ID), RelativeOffset::START),
            operation::snap_to(widget::Id::new(TEXT_ONLY_SCROLL_ID), RelativeOffset::START),
        ])
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
            NfoPaper::new(nfo, nfo_paper_color(self.active_render_settings.as_ref())).into()
        } else {
            nfo.into()
        };

        scrollable(content)
            .id(widget::Id::new(ENHANCED_SCROLL_ID))
            .direction(Direction::Both {
                vertical: Scrollbar::default(),
                horizontal: Scrollbar::default(),
            })
            .width(Fill)
            .height(Fill)
            .into()
    }
}

pub(super) fn nfo_paper_color(settings: &NfoRenderSettings) -> Color {
    let background = settings.background_color;

    Color::from_rgb(background.red, background.green, background.blue)
}

#[cfg(test)]
mod tests {
    use iced::Color;
    use palette::rgb::Rgb;

    use super::nfo_paper_color;
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
}
