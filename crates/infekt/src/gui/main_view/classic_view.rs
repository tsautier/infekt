use super::{InfektMainView, Message};

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use iced::Element;
use iced::Length::{Fill, Shrink};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{self, Text, container, scrollable, text};

use crate::core::nfo_data::NfoData;

impl InfektMainView {
    pub(super) fn classic_tab<'a>(
        &self,
        current_nfo: &'a NfoData,
        stripped: bool,
    ) -> Element<'a, Message> {
        let scrollable_id = widget::Id::new(if stripped {
            "main view stripped"
        } else {
            "main view classic"
        });
        let _has_blocks = !stripped && current_nfo.has_blocks();

        scrollable(
            container(
                (if stripped {
                    self.stripped_content(current_nfo)
                } else {
                    self.classic_content(current_nfo)
                })
                .font(font_with_name(&self.active_render_settings.font_name))
                .size(self.active_render_settings.classic_font_size)
                .line_height(text::LineHeight::Relative(1.0))
                .shaping(text::Shaping::Advanced)
                .wrapping(text::Wrapping::None),
            )
            .center_x(Shrink)
            .padding(25),
        )
        .id(scrollable_id)
        .direction(Direction::Both {
            vertical: Scrollbar::default(),
            horizontal: Scrollbar::default(),
        })
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn classic_content<'a>(&self, current_nfo: &'a NfoData) -> Text<'a> {
        if !current_nfo.has_blocks() {
            return self.stripped_content(current_nfo);
        }

        text(current_nfo.get_classic_text())
    }

    fn stripped_content<'a>(&self, current_nfo: &'a NfoData) -> Text<'a> {
        text(current_nfo.get_stripped_text())
    }
}

fn font_with_name(name: &str) -> iced::Font {
    static FONT_NAMES: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();

    // Iced stores font-family names for the lifetime of the application. Intern each
    // selected system font once instead of leaking a new copy on every view rebuild.
    let mut font_names = FONT_NAMES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let name = *font_names
        .entry(name.to_owned())
        .or_insert_with_key(|name| Box::leak(name.clone().into_boxed_str()));

    iced::Font::with_name(name)
}
