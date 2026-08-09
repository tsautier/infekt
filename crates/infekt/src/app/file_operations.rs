use iced::Task;
use std::path::PathBuf;

use crate::core::nfo_data::NfoData;

use super::{InfektApp, Message};

impl InfektApp {
    pub(super) fn task_open_nfo_file_dialog(&mut self) -> Task<Message> {
        iced::window::run(self.main_window_id.unwrap(), move |w| {
            let path = rfd::FileDialog::new()
                .set_parent(&w)
                .add_filter("Block Art Files", &["nfo", "diz", "asc", "txt"])
                .pick_file();

            Message::OpenFile(path)
        })
    }

    pub(super) fn load_new_nfo(
        &mut self,
        file_path: Option<PathBuf>,
    ) -> Result<Task<Message>, String> {
        let Some(file_path) = file_path else {
            return Ok(Task::none());
        };

        self.load_nfo_path(file_path, true)
    }

    pub(super) fn load_browsed_nfo(&mut self, file_path: PathBuf) -> Result<Task<Message>, String> {
        self.load_nfo_path(file_path, false)
    }

    fn load_nfo_path(
        &mut self,
        file_path: PathBuf,
        restart_browser: bool,
    ) -> Result<Task<Message>, String> {
        let mut candidate = NfoData::new();
        candidate
            .load_from_file(&file_path)
            .map_err(|error| format!("Failed to load file: {error}"))?;

        self.current_nfo = candidate;
        self.backdrop.invalidate_source();

        let scan = if restart_browser {
            let request = self.folder_browser.begin_for_file(&file_path);
            Self::scan_task(request)
        } else {
            self.folder_browser.set_current_path(&file_path);
            Task::none()
        };

        Ok(Task::batch([
            self.ensure_backdrop(),
            self.main_view.reset_scroll().map(Message::MainView),
            scan,
        ]))
    }
}
