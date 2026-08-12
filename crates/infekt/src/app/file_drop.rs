use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) enum FileDropEvent {
    Hovered(PathBuf),
    Dropped(PathBuf),
    Left,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FileDropOutcome {
    None,
    Open(PathBuf),
    RejectMultiple,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileDropHover<'a> {
    Single(&'a Path),
    Multiple(usize),
}

#[derive(Debug, Default)]
pub(crate) struct FileDropState {
    phase: Phase,
}

#[derive(Debug, Default)]
enum Phase {
    #[default]
    Idle,
    Hovering(Vec<PathBuf>),
    // Successful drops have no trailing `FilesHoveredLeft` event, so suppress
    // the remaining paths using the count collected during the hover gesture.
    Rejecting {
        remaining: usize,
    },
}

impl FileDropState {
    pub(crate) fn handle(&mut self, event: FileDropEvent) -> FileDropOutcome {
        match event {
            FileDropEvent::Hovered(path) => {
                match &mut self.phase {
                    Phase::Hovering(paths) => paths.push(path),
                    Phase::Idle | Phase::Rejecting { .. } => {
                        self.phase = Phase::Hovering(vec![path]);
                    }
                }

                FileDropOutcome::None
            }
            FileDropEvent::Dropped(path) => self.handle_drop(path),
            FileDropEvent::Left => {
                self.phase = Phase::Idle;
                FileDropOutcome::None
            }
        }
    }

    pub(crate) fn hover(&self) -> Option<FileDropHover<'_>> {
        match &self.phase {
            Phase::Hovering(paths) if paths.len() == 1 => {
                Some(FileDropHover::Single(paths[0].as_path()))
            }
            Phase::Hovering(paths) if paths.len() > 1 => Some(FileDropHover::Multiple(paths.len())),
            Phase::Idle | Phase::Hovering(_) | Phase::Rejecting { .. } => None,
        }
    }

    fn handle_drop(&mut self, path: PathBuf) -> FileDropOutcome {
        match std::mem::take(&mut self.phase) {
            // Supported backends normally announce every path with `Hovered`
            // first. Keep a standalone `Dropped` fallback for robustness.
            Phase::Idle => FileDropOutcome::Open(path),
            Phase::Hovering(paths) if paths.len() <= 1 => FileDropOutcome::Open(path),
            Phase::Hovering(paths) => {
                self.phase = Phase::Rejecting {
                    remaining: paths.len() - 1,
                };
                FileDropOutcome::RejectMultiple
            }
            Phase::Rejecting { remaining } => {
                if remaining > 1 {
                    self.phase = Phase::Rejecting {
                        remaining: remaining - 1,
                    };
                }

                FileDropOutcome::None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_file_drop_opens_once_and_clears_hover() {
        let path = PathBuf::from("release.nfo");
        let mut state = FileDropState::default();

        assert_eq!(
            state.handle(FileDropEvent::Hovered(path.clone())),
            FileDropOutcome::None
        );
        assert_eq!(state.hover(), Some(FileDropHover::Single(path.as_path())));
        assert_eq!(
            state.handle(FileDropEvent::Dropped(path.clone())),
            FileDropOutcome::Open(path)
        );
        assert_eq!(state.hover(), None);
    }

    #[test]
    fn cancelled_hover_clears_state() {
        let mut state = FileDropState::default();

        let _ = state.handle(FileDropEvent::Hovered(PathBuf::from("release.nfo")));
        assert_ne!(state.hover(), None);
        assert_eq!(state.handle(FileDropEvent::Left), FileDropOutcome::None);
        assert_eq!(state.hover(), None);
    }

    #[test]
    fn multiple_files_are_rejected_once_and_every_path_is_suppressed() {
        let paths = ["one.nfo", "two.nfo", "three.nfo"].map(PathBuf::from);
        let mut state = FileDropState::default();

        for path in &paths {
            assert_eq!(
                state.handle(FileDropEvent::Hovered(path.clone())),
                FileDropOutcome::None
            );
        }
        assert_eq!(state.hover(), Some(FileDropHover::Multiple(paths.len())));

        assert_eq!(
            state.handle(FileDropEvent::Dropped(paths[0].clone())),
            FileDropOutcome::RejectMultiple
        );
        assert_eq!(state.hover(), None);
        assert_eq!(
            state.handle(FileDropEvent::Dropped(paths[1].clone())),
            FileDropOutcome::None
        );
        assert_eq!(
            state.handle(FileDropEvent::Dropped(paths[2].clone())),
            FileDropOutcome::None
        );
        assert_eq!(state.hover(), None);
    }

    #[test]
    fn unannounced_drop_is_treated_as_a_single_file() {
        let path = PathBuf::from("release.nfo");
        let mut state = FileDropState::default();

        assert_eq!(
            state.handle(FileDropEvent::Dropped(path.clone())),
            FileDropOutcome::Open(path)
        );
    }

    #[test]
    fn new_hover_recovers_from_an_incomplete_rejected_batch() {
        let mut state = FileDropState::default();

        for path in ["one.nfo", "two.nfo", "three.nfo"] {
            let _ = state.handle(FileDropEvent::Hovered(PathBuf::from(path)));
        }
        assert_eq!(
            state.handle(FileDropEvent::Dropped(PathBuf::from("one.nfo"))),
            FileDropOutcome::RejectMultiple
        );

        let next = PathBuf::from("next.nfo");
        assert_eq!(
            state.handle(FileDropEvent::Hovered(next.clone())),
            FileDropOutcome::None
        );
        assert_eq!(state.hover(), Some(FileDropHover::Single(next.as_path())));
        assert_eq!(
            state.handle(FileDropEvent::Dropped(next.clone())),
            FileDropOutcome::Open(next)
        );
    }

    #[test]
    fn identical_hover_paths_still_count_as_multiple_entries() {
        let path = PathBuf::from("release.nfo");
        let mut state = FileDropState::default();

        let _ = state.handle(FileDropEvent::Hovered(path.clone()));
        let _ = state.handle(FileDropEvent::Hovered(path.clone()));

        assert_eq!(state.hover(), Some(FileDropHover::Multiple(2)));
        assert_eq!(
            state.handle(FileDropEvent::Dropped(path)),
            FileDropOutcome::RejectMultiple
        );
    }
}
