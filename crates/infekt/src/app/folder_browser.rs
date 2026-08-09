use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, Stream, StreamExt};
use iced::{Subscription, stream};
use notify::{RecursiveMode, Watcher};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowseDirection {
    Previous,
    Next,
}

#[derive(Debug, Clone)]
pub(crate) enum WatchEvent {
    Changed(PathBuf),
    Failed(PathBuf, String),
}

#[derive(Debug, Clone)]
pub(crate) struct ScanRequest {
    directory: PathBuf,
    generation: u64,
}

impl ScanRequest {
    pub(crate) fn run(self) -> ScanResult {
        let files = scan_directory(&self.directory);

        ScanResult {
            directory: self.directory,
            generation: self.generation,
            files,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ScanResult {
    directory: PathBuf,
    generation: u64,
    files: Result<Vec<PathBuf>, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScanUpdate {
    Ignored,
    Updated,
    LoadNearest(PathBuf),
    Failed(String),
}

#[derive(Debug, Default)]
pub(crate) struct FolderBrowser {
    directory: Option<PathBuf>,
    files: Vec<PathBuf>,
    current_index: Option<usize>,
    anchor_index: usize,
    scan_generation: u64,
    watch_generation: u64,
    watch_failed: bool,
}

impl FolderBrowser {
    pub(crate) fn begin_for_file(&mut self, path: &Path) -> Option<ScanRequest> {
        if !is_nfo_path(path) {
            self.clear();
            return None;
        }

        let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        else {
            self.clear();
            return None;
        };
        let directory = parent.to_path_buf();

        if self.directory.as_ref() != Some(&directory) {
            self.files.clear();
            self.current_index = None;
            self.anchor_index = 0;
        }

        self.directory = Some(directory);
        self.watch_failed = false;
        self.watch_generation = self.watch_generation.wrapping_add(1);
        self.set_current_path(path);
        self.request_scan()
    }

    pub(crate) fn request_scan(&mut self) -> Option<ScanRequest> {
        let directory = self.directory.clone()?;
        self.scan_generation = self.scan_generation.wrapping_add(1);

        Some(ScanRequest {
            directory,
            generation: self.scan_generation,
        })
    }

    pub(crate) fn request_scan_for(&mut self, directory: &Path) -> Option<ScanRequest> {
        (self.directory.as_deref() == Some(directory))
            .then(|| self.request_scan())
            .flatten()
    }

    pub(crate) fn apply_scan(
        &mut self,
        result: ScanResult,
        current_path: Option<&Path>,
    ) -> ScanUpdate {
        if self.directory.as_ref() != Some(&result.directory)
            || self.scan_generation != result.generation
        {
            return ScanUpdate::Ignored;
        }

        let files = match result.files {
            Ok(files) => files,
            Err(error) => {
                self.files.clear();
                self.current_index = None;
                self.watch_failed = true;
                return ScanUpdate::Failed(error);
            }
        };

        self.files = files;

        if let Some(current_path) = current_path
            && let Some(index) = self.files.iter().position(|path| path == current_path)
        {
            self.current_index = Some(index);
            self.anchor_index = index;
            return ScanUpdate::Updated;
        }

        self.current_index = None;

        if self.files.is_empty() {
            return ScanUpdate::Updated;
        }

        let index = self.anchor_index.min(self.files.len() - 1);
        ScanUpdate::LoadNearest(self.files[index].clone())
    }

    pub(crate) fn set_current_path(&mut self, path: &Path) {
        self.current_index = self.files.iter().position(|candidate| candidate == path);

        if let Some(index) = self.current_index {
            self.anchor_index = index;
        }
    }

    pub(crate) fn paths_in_direction(&self, direction: BrowseDirection) -> Vec<PathBuf> {
        if !self.is_active() {
            return Vec::new();
        }

        let current = self.current_index.expect("active browser has an index");
        (1..self.files.len())
            .map(|offset| {
                let index = match direction {
                    BrowseDirection::Previous => current
                        .checked_sub(offset)
                        .unwrap_or_else(|| self.files.len() - (offset - current)),
                    BrowseDirection::Next => (current + offset) % self.files.len(),
                };

                self.files[index].clone()
            })
            .collect()
    }

    pub(crate) fn replacement_paths(&self, first: &Path) -> Vec<PathBuf> {
        let Some(first_index) = self.files.iter().position(|path| path == first) else {
            return Vec::new();
        };

        (0..self.files.len())
            .map(|offset| self.files[(first_index + offset) % self.files.len()].clone())
            .collect()
    }

    pub(crate) fn position(&self) -> Option<(usize, usize)> {
        self.is_active().then(|| {
            (
                self.current_index.expect("active browser has an index") + 1,
                self.files.len(),
            )
        })
    }

    pub(crate) fn is_active(&self) -> bool {
        self.files.len() > 1 && self.current_index.is_some()
    }

    pub(crate) fn mark_watch_failed(&mut self, directory: &Path) -> bool {
        if self.directory.as_deref() != Some(directory) || self.watch_failed {
            return false;
        }

        self.watch_failed = true;
        true
    }

    pub(crate) fn subscription(&self) -> Subscription<WatchEvent> {
        if self.watch_failed {
            return Subscription::none();
        }

        self.directory
            .as_ref()
            .map_or_else(Subscription::none, |directory| {
                Subscription::run_with((directory.clone(), self.watch_generation), watch_folder)
            })
    }

    fn clear(&mut self) {
        self.directory = None;
        self.files.clear();
        self.current_index = None;
        self.anchor_index = 0;
        self.scan_generation = self.scan_generation.wrapping_add(1);
        self.watch_generation = self.watch_generation.wrapping_add(1);
        self.watch_failed = false;
    }
}

fn watch_folder(data: &(PathBuf, u64)) -> impl Stream<Item = WatchEvent> + use<> {
    let directory = data.0.clone();

    stream::channel(8, async move |mut output| {
        let (mut sender, mut receiver) = mpsc::channel(1);
        let callback_directory = directory.clone();
        let mut watcher = match notify::recommended_watcher(move |event| {
            let _ = sender.try_send(event);
        }) {
            Ok(watcher) => watcher,
            Err(error) => {
                let _ = output
                    .send(WatchEvent::Failed(callback_directory, error.to_string()))
                    .await;
                return;
            }
        };

        if let Err(error) = watcher.watch(&directory, RecursiveMode::NonRecursive) {
            let _ = output
                .send(WatchEvent::Failed(directory, error.to_string()))
                .await;
            return;
        }

        if output
            .send(WatchEvent::Changed(directory.clone()))
            .await
            .is_err()
        {
            return;
        }

        while let Some(event) = receiver.next().await {
            let message = match event {
                Ok(_) => WatchEvent::Changed(directory.clone()),
                Err(error) => WatchEvent::Failed(directory.clone(), error.to_string()),
            };
            let failed = matches!(message, WatchEvent::Failed(_, _));

            if output.send(message).await.is_err() || failed {
                return;
            }
        }
    })
}

fn scan_directory(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(directory).map_err(|error| {
        format!(
            "Unable to browse NFO folder '{}': {error}",
            directory.to_string_lossy()
        )
    })?;
    let mut files = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_nfo_path(path))
        .collect::<Vec<_>>();

    files.sort_by(|left, right| {
        natural_path_cmp(left, right).then_with(|| left.as_os_str().cmp(right.as_os_str()))
    });
    Ok(files)
}

fn is_nfo_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("nfo"))
}

fn natural_path_cmp(left: &Path, right: &Path) -> Ordering {
    let left = left
        .file_name()
        .unwrap_or(left.as_os_str())
        .to_string_lossy()
        .to_lowercase();
    let right = right
        .file_name()
        .unwrap_or(right.as_os_str())
        .to_string_lossy()
        .to_lowercase();

    natural_cmp(left.as_bytes(), right.as_bytes())
}

fn natural_cmp(left: &[u8], right: &[u8]) -> Ordering {
    let (mut left_index, mut right_index) = (0, 0);

    while left_index < left.len() && right_index < right.len() {
        if left[left_index].is_ascii_digit() && right[right_index].is_ascii_digit() {
            let left_end = digit_run_end(left, left_index);
            let right_end = digit_run_end(right, right_index);
            let ordering =
                numeric_run_cmp(&left[left_index..left_end], &right[right_index..right_end]);

            if ordering != Ordering::Equal {
                return ordering;
            }

            left_index = left_end;
            right_index = right_end;
        } else {
            let ordering = left[left_index].cmp(&right[right_index]);

            if ordering != Ordering::Equal {
                return ordering;
            }

            left_index += 1;
            right_index += 1;
        }
    }

    left.len().cmp(&right.len())
}

fn digit_run_end(value: &[u8], start: usize) -> usize {
    value[start..]
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .map_or(value.len(), |offset| start + offset)
}

fn numeric_run_cmp(left: &[u8], right: &[u8]) -> Ordering {
    let left_significant = trim_leading_zeroes(left);
    let right_significant = trim_leading_zeroes(right);

    left_significant
        .len()
        .cmp(&right_significant.len())
        .then_with(|| left_significant.cmp(right_significant))
        .then_with(|| left.len().cmp(&right.len()))
}

fn trim_leading_zeroes(value: &[u8]) -> &[u8] {
    let first_non_zero = value
        .iter()
        .position(|byte| *byte != b'0')
        .unwrap_or(value.len().saturating_sub(1));

    &value[first_non_zero..]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan_result(browser: &FolderBrowser, directory: &Path, files: &[&str]) -> ScanResult {
        let mut files = files
            .iter()
            .map(|name| directory.join(name))
            .collect::<Vec<_>>();
        files.sort_by(|left, right| {
            natural_path_cmp(left, right).then_with(|| left.as_os_str().cmp(right.as_os_str()))
        });

        ScanResult {
            directory: directory.to_path_buf(),
            generation: browser.scan_generation,
            files: Ok(files),
        }
    }

    #[test]
    fn nfo_filter_is_case_insensitive_and_excludes_other_art_files() {
        assert!(is_nfo_path(Path::new("release.nfo")));
        assert!(is_nfo_path(Path::new("release.NFO")));
        assert!(!is_nfo_path(Path::new("release.diz")));
        assert!(!is_nfo_path(Path::new("release.txt")));
    }

    #[test]
    fn natural_order_compares_numeric_runs_and_uses_paths_as_ties() {
        let mut paths = [
            PathBuf::from("release10.nfo"),
            PathBuf::from("release02.nfo"),
            PathBuf::from("Release2.nfo"),
            PathBuf::from("release1.nfo"),
        ];
        paths.sort_by(|left, right| {
            natural_path_cmp(left, right).then_with(|| left.as_os_str().cmp(right.as_os_str()))
        });

        assert_eq!(paths[0], PathBuf::from("release1.nfo"));
        assert_eq!(paths[1], PathBuf::from("Release2.nfo"));
        assert_eq!(paths[2], PathBuf::from("release02.nfo"));
        assert_eq!(paths[3], PathBuf::from("release10.nfo"));

        let mut ties = [PathBuf::from("release2.nfo"), PathBuf::from("Release2.NFO")];
        let mut expected = ties.clone();
        expected.sort_by(|left, right| left.as_os_str().cmp(right.as_os_str()));

        assert_eq!(natural_path_cmp(&ties[0], &ties[1]), Ordering::Equal);
        ties.sort_by(|left, right| {
            natural_path_cmp(left, right).then_with(|| left.as_os_str().cmp(right.as_os_str()))
        });
        assert_eq!(ties, expected);
    }

    #[test]
    fn directory_scan_is_non_recursive_and_only_returns_nfo_files() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("release10.nfo"), b"ten").unwrap();
        std::fs::write(directory.path().join("release2.NFO"), b"two").unwrap();
        std::fs::write(directory.path().join("release1.txt"), b"text").unwrap();
        let nested = directory.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(nested.join("hidden.nfo"), b"hidden").unwrap();

        let files = scan_directory(directory.path()).unwrap();

        assert_eq!(
            files,
            vec![
                directory.path().join("release2.NFO"),
                directory.path().join("release10.nfo"),
            ]
        );
    }

    #[test]
    fn browser_activates_at_two_files_and_wraps_both_directions() {
        let directory = Path::new("/tmp/releases");
        let current = directory.join("one.nfo");
        let mut browser = FolderBrowser::default();
        browser.begin_for_file(&current);

        let one = scan_result(&browser, directory, &["one.nfo"]);
        assert_eq!(browser.apply_scan(one, Some(&current)), ScanUpdate::Updated);
        assert!(!browser.is_active());

        browser.request_scan();
        let two = scan_result(&browser, directory, &["one.nfo", "two.nfo"]);
        assert_eq!(browser.apply_scan(two, Some(&current)), ScanUpdate::Updated);
        assert_eq!(browser.position(), Some((1, 2)));
        assert_eq!(
            browser.paths_in_direction(BrowseDirection::Previous),
            vec![directory.join("two.nfo")]
        );
        assert_eq!(
            browser.paths_in_direction(BrowseDirection::Next),
            vec![directory.join("two.nfo")]
        );

        browser.set_current_path(&directory.join("two.nfo"));
        assert_eq!(
            browser.paths_in_direction(BrowseDirection::Next),
            vec![directory.join("one.nfo")]
        );
    }

    #[test]
    fn browse_candidates_visit_every_other_file_once_in_the_requested_direction() {
        let directory = Path::new("/tmp/releases");
        let current = directory.join("release2.nfo");
        let mut browser = FolderBrowser::default();
        browser.begin_for_file(&current);
        let initial = scan_result(
            &browser,
            directory,
            &[
                "release1.nfo",
                "release2.nfo",
                "release3.nfo",
                "release4.nfo",
            ],
        );
        browser.apply_scan(initial, Some(&current));

        assert_eq!(
            browser.paths_in_direction(BrowseDirection::Next),
            vec![
                directory.join("release3.nfo"),
                directory.join("release4.nfo"),
                directory.join("release1.nfo"),
            ]
        );
        assert_eq!(
            browser.paths_in_direction(BrowseDirection::Previous),
            vec![
                directory.join("release1.nfo"),
                directory.join("release4.nfo"),
                directory.join("release3.nfo"),
            ]
        );
    }

    #[test]
    fn browsing_previous_from_later_index_does_not_underflow() {
        let directory = Path::new("/tmp/releases");
        let current = directory.join("release3.nfo");
        let mut browser = FolderBrowser::default();
        browser.begin_for_file(&current);
        let initial = scan_result(
            &browser,
            directory,
            &[
                "release1.nfo",
                "release2.nfo",
                "release3.nfo",
                "release4.nfo",
            ],
        );
        browser.apply_scan(initial, Some(&current));

        assert_eq!(
            browser.paths_in_direction(BrowseDirection::Previous),
            vec![
                directory.join("release2.nfo"),
                directory.join("release1.nfo"),
                directory.join("release4.nfo"),
            ]
        );
    }

    #[test]
    fn replacement_candidates_start_at_nearest_and_are_bounded_to_one_cycle() {
        let directory = Path::new("/tmp/releases");
        let current = directory.join("release2.nfo");
        let mut browser = FolderBrowser::default();
        browser.begin_for_file(&current);
        let initial = scan_result(
            &browser,
            directory,
            &["release1.nfo", "release2.nfo", "release3.nfo"],
        );
        browser.apply_scan(initial, Some(&current));

        browser.request_scan();
        let removed = scan_result(
            &browser,
            directory,
            &["release1.nfo", "release3.nfo", "release4.nfo"],
        );
        let ScanUpdate::LoadNearest(nearest) = browser.apply_scan(removed, Some(&current)) else {
            panic!("removing the current file should select its nearest replacement");
        };

        assert_eq!(
            browser.replacement_paths(&nearest),
            vec![
                directory.join("release3.nfo"),
                directory.join("release4.nfo"),
                directory.join("release1.nfo"),
            ]
        );
    }

    #[test]
    fn removing_current_loads_the_same_position_or_new_last() {
        let directory = Path::new("/tmp/releases");
        let current = directory.join("two.nfo");
        let mut browser = FolderBrowser::default();
        browser.begin_for_file(&current);
        let initial = scan_result(&browser, directory, &["one.nfo", "two.nfo", "three.nfo"]);
        browser.apply_scan(initial, Some(&current));

        browser.request_scan();
        let removed = scan_result(&browser, directory, &["one.nfo", "three.nfo"]);
        assert_eq!(
            browser.apply_scan(removed, Some(&current)),
            ScanUpdate::LoadNearest(directory.join("three.nfo"))
        );

        browser.set_current_path(&directory.join("three.nfo"));
        browser.request_scan();
        let removed_last = scan_result(&browser, directory, &["one.nfo"]);
        assert_eq!(
            browser.apply_scan(removed_last, Some(&directory.join("three.nfo"))),
            ScanUpdate::LoadNearest(directory.join("one.nfo"))
        );
    }

    #[test]
    fn rescans_recompute_position_when_files_are_inserted_or_removed_before_current() {
        let directory = Path::new("/tmp/releases");
        let current = directory.join("release10.nfo");
        let mut browser = FolderBrowser::default();
        browser.begin_for_file(&current);
        let initial = scan_result(&browser, directory, &["release2.nfo", "release10.nfo"]);
        assert_eq!(
            browser.apply_scan(initial, Some(&current)),
            ScanUpdate::Updated
        );
        assert_eq!(browser.position(), Some((2, 2)));

        browser.request_scan();
        let inserted = scan_result(
            &browser,
            directory,
            &["release1.nfo", "release2.nfo", "release10.nfo"],
        );
        assert_eq!(
            browser.apply_scan(inserted, Some(&current)),
            ScanUpdate::Updated
        );
        assert_eq!(browser.position(), Some((3, 3)));

        browser.request_scan();
        let removed = scan_result(&browser, directory, &["release1.nfo", "release10.nfo"]);
        assert_eq!(
            browser.apply_scan(removed, Some(&current)),
            ScanUpdate::Updated
        );
        assert_eq!(browser.position(), Some((2, 2)));
    }

    #[test]
    fn empty_scan_hides_navigation_but_keeps_the_anchor_for_reactivation() {
        let directory = Path::new("/tmp/releases");
        let current = directory.join("two.nfo");
        let mut browser = FolderBrowser::default();
        browser.begin_for_file(&current);
        let initial = scan_result(&browser, directory, &["one.nfo", "two.nfo"]);
        browser.apply_scan(initial, Some(&current));

        browser.request_scan();
        let empty = scan_result(&browser, directory, &[]);
        assert_eq!(
            browser.apply_scan(empty, Some(&current)),
            ScanUpdate::Updated
        );
        assert!(!browser.is_active());

        browser.request_scan();
        let recreated = scan_result(&browser, directory, &["new.nfo"]);
        assert_eq!(
            browser.apply_scan(recreated, Some(&current)),
            ScanUpdate::LoadNearest(directory.join("new.nfo"))
        );
    }

    #[test]
    fn stale_scan_results_are_ignored() {
        let directory = Path::new("/tmp/releases");
        let current = directory.join("one.nfo");
        let mut browser = FolderBrowser::default();
        let stale_request = browser.begin_for_file(&current).unwrap();
        browser.request_scan();
        let stale = ScanResult {
            directory: stale_request.directory,
            generation: stale_request.generation,
            files: Ok(vec![current.clone()]),
        };

        assert_eq!(
            browser.apply_scan(stale, Some(&current)),
            ScanUpdate::Ignored
        );
    }
}
