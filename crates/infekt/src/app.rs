mod file_operations;
mod theme;
mod utils;
mod view;

use std::path::PathBuf;
use std::sync::Arc;

use iced::{Task, Theme};

use crate::core::nfo_data::NfoData;
use crate::gui::about_screen::{self, InfektAboutScreen};
use crate::gui::main_view::{self, InfektMainView};
use crate::gui::nfo_backdrop::{BackdropImage, BackdropKey, NfoBackdrop};
use crate::gui::presentation_inspector::{self, PresentationInspector};
use crate::presentation::PresentationState;
use crate::settings::NfoRenderSettings;

#[derive(Debug, Clone)]
pub(crate) enum Message {
    NoOp,
    MainWindowCreated(Option<iced::window::Id>),
    MainView(main_view::Message),
    Inspector(presentation_inspector::Message),
    About(about_screen::Message),
    OpenFileDialog,
    OpenFile(Option<PathBuf>),
    ZoomIn,
    ZoomOut,
    ToggleInspector,
    ToggleOverflow,
    ShowAbout,
    CloseAbout,
    BackdropReady(BackdropKey, u64, Option<BackdropImage>),
}

#[derive(Debug, Clone)]
pub(crate) enum Action {
    None,
    SelectFileForOpening,
    ShowErrorMessage(String),
}

#[derive(Default)]
pub(crate) struct InfektApp {
    main_window_id: Option<iced::window::Id>,
    main_view: InfektMainView,
    presentation_inspector: PresentationInspector,
    about_screen: InfektAboutScreen,
    presentation: PresentationState,
    backdrop: NfoBackdrop,
    theme: Option<Theme>,
    active_render_settings: Arc<NfoRenderSettings>,
    current_nfo: NfoData,
}

impl InfektApp {
    pub fn new() -> (Self, Task<Message>) {
        let presentation = PresentationState::new();
        let mut settings = NfoRenderSettings::default();
        presentation
            .theme_values
            .apply_to_render_settings(&mut settings);
        presentation.apply_zoom(&mut settings);
        let active_render_settings = Arc::new(settings);

        let mut main_view = InfektMainView::default();
        main_view.update(main_view::Message::RenderSettingsChanged(Arc::clone(
            &active_render_settings,
        )));

        let app = Self {
            main_view,
            theme: Some(theme::create_theme(Arc::clone(&active_render_settings))),
            active_render_settings,
            presentation,
            ..Self::default()
        };

        let task = Task::batch([
            iced::window::oldest().map(Message::MainWindowCreated),
            PresentationInspector::load_font_names().map(Message::Inspector),
        ]);

        (app, task)
    }

    pub fn title(&self) -> String {
        if self.current_nfo.is_loaded() {
            format!(
                "iNFekt NFO Viewer - {}",
                self.current_nfo.get_file_name().unwrap_or_default()
            )
        } else {
            "iNFekt NFO Viewer".to_owned()
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let mut follow_up = Task::none();

        let action = match message {
            Message::NoOp => Action::None,
            Message::MainWindowCreated(window_id) => {
                self.main_window_id = window_id;
                Action::None
            }
            Message::MainView(message) => self.main_view.update(message),
            Message::Inspector(message) => {
                let mut settings = (*self.active_render_settings).clone();
                let result = self.presentation_inspector.update(
                    message,
                    &mut self.presentation,
                    &mut settings,
                );

                if result == presentation_inspector::Update::RenderSettingsChanged {
                    self.apply_render_settings(settings);
                    follow_up = self.ensure_backdrop();
                }

                Action::None
            }
            Message::About(message) => self.about_screen.update(message),
            Message::OpenFileDialog => Action::SelectFileForOpening,
            Message::OpenFile(file) => {
                let should_refresh_backdrop = file.is_some();

                if should_refresh_backdrop {
                    self.backdrop.invalidate_source();
                }

                let action = self.action_load_new_nfo(file);

                if should_refresh_backdrop {
                    follow_up = self.ensure_backdrop();
                }

                action
            }
            Message::ZoomIn => {
                self.presentation.zoom_in();
                self.apply_current_zoom();
                follow_up = self.ensure_backdrop();
                Action::None
            }
            Message::ZoomOut => {
                self.presentation.zoom_out();
                self.apply_current_zoom();
                follow_up = self.ensure_backdrop();
                Action::None
            }
            Message::ToggleInspector => {
                self.presentation.inspector_open = !self.presentation.inspector_open;
                self.presentation.overflow_open = false;
                Action::None
            }
            Message::ToggleOverflow => {
                self.presentation.overflow_open = !self.presentation.overflow_open;
                Action::None
            }
            Message::ShowAbout => {
                self.presentation.about_open = true;
                self.presentation.overflow_open = false;

                if let Some(task) = self.about_screen.on_before_shown() {
                    return task.map(Message::About);
                }

                Action::None
            }
            Message::CloseAbout => {
                self.presentation.about_open = false;
                Action::None
            }
            Message::BackdropReady(key, generation, image) => {
                self.backdrop.accept_result(key, generation, image);
                Action::None
            }
        };

        let action_task = match action {
            Action::None => Task::none(),
            Action::SelectFileForOpening => self.task_open_nfo_file_dialog(),
            Action::ShowErrorMessage(message) => self.show_error_message_popup(message),
        };

        Task::batch([action_task, follow_up])
    }

    pub fn theme(&self) -> Option<Theme> {
        self.theme.clone()
    }

    fn apply_current_zoom(&mut self) {
        let mut settings = (*self.active_render_settings).clone();
        self.presentation.apply_zoom(&mut settings);
        self.apply_render_settings(settings);
    }

    fn apply_render_settings(&mut self, settings: NfoRenderSettings) {
        self.active_render_settings = Arc::new(settings);
        self.theme = Some(theme::create_theme(Arc::clone(
            &self.active_render_settings,
        )));
        self.main_view
            .update(main_view::Message::RenderSettingsChanged(Arc::clone(
                &self.active_render_settings,
            )));
    }

    fn ensure_backdrop(&mut self) -> Task<Message> {
        let Some(request) = self.backdrop.request(
            self.current_nfo.get_renderer_grid(),
            self.active_render_settings.as_ref(),
            self.presentation.character_ratio,
        ) else {
            return Task::none();
        };
        let key = request.key();
        let generation = request.generation();

        Task::perform(async move { request.generate() }, move |image| {
            Message::BackdropReady(key, generation, image)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::main_view::TabId;
    use crate::gui::presentation_inspector;
    use crate::presentation::NfoThemePreset;

    #[test]
    fn shell_controls_update_session_state() {
        let (mut app, _startup) = InfektApp::new();

        let _ = app.update(Message::ToggleInspector);
        assert!(!app.presentation.inspector_open);

        let _ = app.update(Message::ToggleOverflow);
        assert!(app.presentation.overflow_open);

        let _ = app.update(Message::ShowAbout);
        assert!(app.presentation.about_open);
        assert!(!app.presentation.overflow_open);
        let _ = app.update(Message::CloseAbout);
        assert!(!app.presentation.about_open);

        let _ = app.update(Message::MainView(main_view::Message::TabSelected(
            TabId::Classic,
        )));
        assert_eq!(app.main_view.active_tab(), TabId::Classic);

        let _ = app.update(Message::ZoomIn);
        assert_eq!(app.presentation.zoom_percent, 125);

        let _ = app.update(Message::Inspector(
            presentation_inspector::Message::ThemeSelected(NfoThemePreset::CobaltPaper),
        ));
        assert_eq!(app.presentation.selected_theme, NfoThemePreset::CobaltPaper);
    }

    #[test]
    fn presentation_only_controls_do_not_change_render_settings() {
        let (mut app, _startup) = InfektApp::new();
        let settings_hash = app.active_render_settings.hash();

        let _ = app.update(Message::Inspector(
            presentation_inspector::Message::UseAnsiColorsChanged(false),
        ));
        let _ = app.update(Message::Inspector(
            presentation_inspector::Message::LineWrappingChanged(true),
        ));
        let _ = app.update(Message::Inspector(
            presentation_inspector::Message::AntialiasingChanged(false),
        ));

        assert_eq!(app.active_render_settings.hash(), settings_hash);
        assert!(!app.presentation.use_ansi_colors);
        assert!(app.presentation.line_wrapping);
        assert!(!app.presentation.antialiasing);
    }
}
