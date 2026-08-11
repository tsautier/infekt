mod file_drop;
mod file_operations;
mod folder_browser;
mod theme;
mod utils;
mod view;

use std::path::PathBuf;
use std::sync::Arc;

use iced::keyboard::{self, Key, Modifiers};
use iced::{Subscription, Task, Theme};

use crate::core::nfo_data::NfoData;
use crate::gui::about_screen::{self, InfektAboutScreen};
use crate::gui::main_view::{self, InfektMainView};
use crate::gui::nfo_backdrop::{BackdropImage, BackdropKey, NfoBackdrop};
use crate::gui::presentation_inspector::{self, PresentationInspector};
use crate::presentation::PresentationState;
use crate::settings::NfoRenderSettings;

use self::file_drop::{FileDropEvent, FileDropOutcome, FileDropState};
use self::folder_browser::{
    BrowseDirection, FolderBrowser, ScanRequest, ScanResult, ScanUpdate, WatchEvent,
};

#[derive(Debug, Clone)]
pub(crate) enum Message {
    NoOp,
    MainWindowCreated(Option<iced::window::Id>),
    MainView(main_view::Message),
    Inspector(presentation_inspector::Message),
    About(about_screen::Message),
    OpenFileDialog,
    OpenFile(Option<PathBuf>),
    FileDrop(iced::window::Id, FileDropEvent),
    Browse(BrowseDirection),
    FolderWatch(WatchEvent),
    FolderScanReady(ScanResult),
    ZoomIn,
    ZoomOut,
    ToggleInspector,
    ToggleOverflow,
    ShowAbout,
    CloseAbout,
    BackdropReady(BackdropKey, u64, Option<BackdropImage>),
}

#[derive(Default)]
pub(crate) struct InfektApp {
    main_window_id: Option<iced::window::Id>,
    main_view: InfektMainView,
    presentation_inspector: PresentationInspector,
    about_screen: InfektAboutScreen,
    presentation: PresentationState,
    backdrop: NfoBackdrop,
    folder_browser: FolderBrowser,
    file_drop: FileDropState,
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

        let task = match message {
            Message::NoOp => Task::none(),
            Message::MainWindowCreated(window_id) => {
                self.main_window_id = window_id;
                Task::none()
            }
            Message::MainView(message) => {
                self.main_view.update(message);
                Task::none()
            }
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

                Task::none()
            }
            Message::About(message) => {
                self.about_screen.update(message);
                Task::none()
            }
            Message::OpenFileDialog => self.task_open_nfo_file_dialog(),
            Message::OpenFile(file) => self.open_file(file),
            Message::FileDrop(window_id, event) => {
                if self.main_window_id != Some(window_id) {
                    Task::none()
                } else {
                    if matches!(&event, FileDropEvent::Hovered(_)) {
                        self.presentation.overflow_open = false;
                    }

                    match self.file_drop.handle(event) {
                        FileDropOutcome::None => Task::none(),
                        FileDropOutcome::Open(path) => self.open_file(Some(path)),
                        FileDropOutcome::RejectMultiple => self.show_error_message_popup(
                            "Please drop exactly one file at a time.".to_owned(),
                        ),
                    }
                }
            }
            Message::Browse(direction) => self.browse(direction),
            Message::FolderWatch(event) => self.handle_folder_watch(event),
            Message::FolderScanReady(result) => self.handle_folder_scan(result),
            Message::ZoomIn => {
                self.presentation.zoom_in();
                self.apply_current_zoom();
                follow_up = self.ensure_backdrop();
                Task::none()
            }
            Message::ZoomOut => {
                self.presentation.zoom_out();
                self.apply_current_zoom();
                follow_up = self.ensure_backdrop();
                Task::none()
            }
            Message::ToggleInspector => {
                self.presentation.inspector_open = !self.presentation.inspector_open;
                self.presentation.overflow_open = false;
                Task::none()
            }
            Message::ToggleOverflow => {
                self.presentation.overflow_open = !self.presentation.overflow_open;
                Task::none()
            }
            Message::ShowAbout => {
                self.presentation.about_open = true;
                self.presentation.overflow_open = false;

                if let Some(task) = self.about_screen.on_before_shown() {
                    task.map(Message::About)
                } else {
                    Task::none()
                }
            }
            Message::CloseAbout => {
                self.presentation.about_open = false;
                Task::none()
            }
            Message::BackdropReady(key, generation, image) => {
                self.backdrop.accept_result(key, generation, image);
                Task::none()
            }
        };

        Task::batch([task, follow_up])
    }

    pub fn theme(&self) -> Option<Theme> {
        self.theme.clone()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let watcher = self.folder_browser.subscription().map(Message::FolderWatch);
        let keyboard = if shortcuts_enabled(
            self.folder_browser.is_active(),
            self.presentation.about_open,
            self.presentation.overflow_open,
        ) {
            keyboard::listen()
                .filter_map(|event| browse_direction_for_event(event).map(Message::Browse))
        } else {
            Subscription::none()
        };
        let file_drop = iced::window::events().filter_map(|(window_id, event)| {
            let event = match event {
                iced::window::Event::FileHovered(path) => FileDropEvent::Hovered(path),
                iced::window::Event::FileDropped(path) => FileDropEvent::Dropped(path),
                iced::window::Event::FilesHoveredLeft => FileDropEvent::Left,
                _ => return None,
            };

            Some(Message::FileDrop(window_id, event))
        });

        Subscription::batch([watcher, keyboard, file_drop])
    }

    fn open_file(&mut self, file: Option<PathBuf>) -> Task<Message> {
        match self.load_new_nfo(file) {
            Ok(task) => task,
            Err(message) => self.show_error_message_popup(message),
        }
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

    fn browse(&mut self, direction: BrowseDirection) -> Task<Message> {
        let paths = self.folder_browser.paths_in_direction(direction);
        self.load_first_browsable(paths)
    }

    fn load_first_browsable(&mut self, paths: Vec<PathBuf>) -> Task<Message> {
        for path in paths {
            if let Ok(task) = self.load_browsed_nfo(path) {
                return task;
            }
        }

        Task::none()
    }

    fn handle_folder_watch(&mut self, event: WatchEvent) -> Task<Message> {
        match event {
            WatchEvent::Changed(directory) => {
                let request = self.folder_browser.request_scan_for(&directory);
                Self::scan_task(request)
            }
            WatchEvent::Failed(directory, error) => {
                if self.folder_browser.mark_watch_failed(&directory) {
                    self.show_error_message_popup(format!(
                        "Folder monitoring stopped for '{}': {error}",
                        directory.to_string_lossy()
                    ))
                } else {
                    Task::none()
                }
            }
        }
    }

    fn handle_folder_scan(&mut self, result: ScanResult) -> Task<Message> {
        let current_path = self.current_nfo.get_file_path().map(PathBuf::from);

        match self
            .folder_browser
            .apply_scan(result, current_path.as_deref())
        {
            ScanUpdate::Ignored | ScanUpdate::Updated => Task::none(),
            ScanUpdate::LoadNearest(path) => {
                let paths = self.folder_browser.replacement_paths(&path);
                self.load_first_browsable(paths)
            }
            ScanUpdate::Failed(message) => self.show_error_message_popup(message),
        }
    }

    fn scan_task(request: Option<ScanRequest>) -> Task<Message> {
        request.map_or_else(Task::none, |request| {
            Task::perform(async move { request.run() }, Message::FolderScanReady)
        })
    }
}

fn browse_direction_for_event(event: keyboard::Event) -> Option<BrowseDirection> {
    let keyboard::Event::KeyPressed {
        modified_key,
        modifiers,
        repeat,
        ..
    } = event
    else {
        return None;
    };

    browse_direction_for_key(modified_key.as_ref(), modifiers, repeat)
}

fn browse_direction_for_key(
    key: Key<&str>,
    modifiers: Modifiers,
    repeat: bool,
) -> Option<BrowseDirection> {
    if repeat || !modifiers.is_empty() {
        return None;
    }

    match key {
        Key::Character("j") => Some(BrowseDirection::Next),
        Key::Character("k") => Some(BrowseDirection::Previous),
        _ => None,
    }
}

fn shortcuts_enabled(browser_active: bool, about_open: bool, overflow_open: bool) -> bool {
    browser_active && !about_open && !overflow_open
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

    #[test]
    fn folder_shortcuts_use_unmodified_non_repeated_lowercase_keys() {
        assert_eq!(
            browse_direction_for_key(Key::Character("j"), Modifiers::empty(), false),
            Some(BrowseDirection::Next)
        );
        assert_eq!(
            browse_direction_for_key(Key::Character("k"), Modifiers::empty(), false),
            Some(BrowseDirection::Previous)
        );
        assert_eq!(
            browse_direction_for_key(Key::Character("J"), Modifiers::empty(), false),
            None
        );
        assert_eq!(
            browse_direction_for_key(Key::Character("j"), Modifiers::SHIFT, false),
            None
        );
        assert_eq!(
            browse_direction_for_key(Key::Character("j"), Modifiers::empty(), true),
            None
        );
    }

    #[test]
    fn folder_shortcuts_are_disabled_behind_shell_overlays() {
        assert!(shortcuts_enabled(true, false, false));
        assert!(!shortcuts_enabled(false, false, false));
        assert!(!shortcuts_enabled(true, true, false));
        assert!(!shortcuts_enabled(true, false, true));
    }

    #[test]
    fn failed_transactional_load_preserves_the_current_nfo() {
        let directory = tempfile::tempdir().unwrap();
        let valid = directory.path().join("valid.nfo");
        std::fs::write(&valid, b"valid NFO").unwrap();
        let missing = directory.path().join("missing.nfo");
        let (mut app, _startup) = InfektApp::new();

        let _ = app.load_new_nfo(Some(valid.clone())).unwrap();
        assert!(app.load_new_nfo(Some(missing)).is_err());

        assert_eq!(app.current_nfo.get_file_path(), Some(valid.as_path()));
        assert!(app.current_nfo.is_loaded());
    }

    #[test]
    fn file_drop_events_only_apply_to_the_main_window() {
        let (mut app, _startup) = InfektApp::new();
        let main_window = iced::window::Id::unique();
        let other_window = iced::window::Id::unique();
        app.main_window_id = Some(main_window);

        let _ = app.update(Message::FileDrop(
            other_window,
            FileDropEvent::Hovered(PathBuf::from("ignored.nfo")),
        ));
        assert_eq!(app.file_drop.hover(), None);

        let _ = app.update(Message::FileDrop(
            main_window,
            FileDropEvent::Hovered(PathBuf::from("accepted.nfo")),
        ));
        assert!(app.file_drop.hover().is_some());

        let _ = app.update(Message::FileDrop(other_window, FileDropEvent::Left));
        let _ = app.update(Message::FileDrop(
            other_window,
            FileDropEvent::Dropped(PathBuf::from("ignored.nfo")),
        ));
        assert!(app.file_drop.hover().is_some());
    }

    #[test]
    fn accepted_drop_uses_the_regular_loader_without_an_extension_filter() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("release.bin");
        std::fs::write(&path, b"valid NFO").unwrap();
        let (mut app, _startup) = InfektApp::new();
        let main_window = iced::window::Id::unique();
        app.main_window_id = Some(main_window);

        let _ = app.update(Message::FileDrop(
            main_window,
            FileDropEvent::Hovered(path.clone()),
        ));
        let _ = app.update(Message::FileDrop(
            main_window,
            FileDropEvent::Dropped(path.clone()),
        ));

        assert_eq!(app.current_nfo.get_file_path(), Some(path.as_path()));
        assert!(app.current_nfo.is_loaded());
        assert_eq!(app.file_drop.hover(), None);
    }

    #[test]
    fn invalid_drops_preserve_the_current_nfo() {
        let directory = tempfile::tempdir().unwrap();
        let valid = directory.path().join("valid.nfo");
        std::fs::write(&valid, b"valid NFO").unwrap();
        let missing = directory.path().join("missing.nfo");
        let malformed = directory.path().join("malformed.nfo");
        std::fs::write(&malformed, "x".repeat(2_001)).unwrap();
        let oversized = directory.path().join("oversized.nfo");
        std::fs::write(&oversized, vec![b'x'; 3 * 1024 * 1024 + 1]).unwrap();
        let (mut app, _startup) = InfektApp::new();
        let main_window = iced::window::Id::unique();
        app.main_window_id = Some(main_window);
        let _ = app.load_new_nfo(Some(valid.clone())).unwrap();

        for path in [
            directory.path().to_path_buf(),
            missing,
            malformed,
            oversized,
        ] {
            let _ = app.update(Message::FileDrop(
                main_window,
                FileDropEvent::Hovered(path.clone()),
            ));
            let _ = app.update(Message::FileDrop(main_window, FileDropEvent::Dropped(path)));

            assert_eq!(app.current_nfo.get_file_path(), Some(valid.as_path()));
            assert!(app.current_nfo.is_loaded());
            assert_eq!(app.file_drop.hover(), None);
        }
    }

    #[test]
    fn browsing_skips_invalid_files_and_stops_after_one_cycle() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("release1.nfo");
        let invalid = directory.path().join("release2.nfo");
        let third = directory.path().join("release3.nfo");
        std::fs::write(&first, b"first").unwrap();
        std::fs::write(&invalid, "x".repeat(2_001)).unwrap();
        std::fs::write(&third, b"third").unwrap();
        let (mut app, _startup) = InfektApp::new();

        let _ = app.load_new_nfo(Some(first.clone())).unwrap();
        let scan = app.folder_browser.request_scan().unwrap().run();
        assert_eq!(
            app.folder_browser.apply_scan(scan, Some(&first)),
            ScanUpdate::Updated
        );

        let _ = app.browse(BrowseDirection::Next);
        assert_eq!(app.current_nfo.get_file_path(), Some(third.as_path()));

        std::fs::write(&first, "x".repeat(2_001)).unwrap();
        let _ = app.browse(BrowseDirection::Next);
        assert_eq!(app.current_nfo.get_file_path(), Some(third.as_path()));
    }

    #[test]
    fn deleting_current_skips_an_invalid_nearest_replacement() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("release1.nfo");
        let current = directory.path().join("release2.nfo");
        let invalid = directory.path().join("release3.nfo");
        let fourth = directory.path().join("release4.nfo");
        std::fs::write(&first, b"first").unwrap();
        std::fs::write(&current, b"current").unwrap();
        std::fs::write(&invalid, "x".repeat(2_001)).unwrap();
        std::fs::write(&fourth, b"fourth").unwrap();
        let (mut app, _startup) = InfektApp::new();

        let _ = app.load_new_nfo(Some(current.clone())).unwrap();
        let initial = app.folder_browser.request_scan().unwrap().run();
        assert_eq!(
            app.folder_browser.apply_scan(initial, Some(&current)),
            ScanUpdate::Updated
        );

        std::fs::remove_file(&current).unwrap();
        let rescan = app.folder_browser.request_scan().unwrap().run();
        let _ = app.handle_folder_scan(rescan);

        assert_eq!(app.current_nfo.get_file_path(), Some(fourth.as_path()));
    }
}
