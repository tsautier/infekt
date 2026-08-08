mod classic_view;

use std::sync::Arc;

use iced::Element;
use iced::Length::Fill;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{self, scrollable};

use crate::app::Action;
use crate::core::nfo_data::NfoData;
use crate::settings::NfoRenderSettings;

use super::widget::enhanced_nfo_view::EnhancedNfoView;

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

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::TabSelected(selected) => self.active_tab = selected,
            Message::RenderSettingsChanged(settings) => self.active_render_settings = settings,
        }

        Action::None
    }

    pub fn view<'a>(&self, current_nfo: &'a NfoData) -> Element<'a, Message> {
        match self.active_tab {
            TabId::Enhanced => self.enhanced_tab(current_nfo),
            TabId::Classic => self.classic_tab(current_nfo, false),
            TabId::TextOnly => self.classic_tab(current_nfo, true),
        }
    }

    fn enhanced_tab<'a>(&self, current_nfo: &'a NfoData) -> Element<'a, Message> {
        scrollable(EnhancedNfoView::new(
            self.active_render_settings.clone(),
            current_nfo,
        ))
        .id(widget::Id::new("enhanced view"))
        .direction(Direction::Both {
            vertical: Scrollbar::default(),
            horizontal: Scrollbar::default(),
        })
        .width(Fill)
        .height(Fill)
        .into()
    }
}
