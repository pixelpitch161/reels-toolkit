//! The Library's view models: the six sample videos, an output folder with no videos in it, and one that is missing.

use std::collections::BTreeMap;

use super::base;
use crate::model::View;
use crate::model::library::FolderState;
use crate::seam::sample;
use crate::state::{AppState, LibraryView, ViewModel};

/// What the store says when the output folder `vm.settings.output_folder` is missing (`store::library::scan`).
pub fn missing_text(vm: &ViewModel) -> String {
    format!("The output folder {} is missing.", vm.settings.output_folder.display())
}

/// The base with the Library open: the six videos and covers of `sample::library()`, newest first (each dated at
/// 12:00 UTC, so the dates read the same in every time zone), the second (Tuba vs Bass drum) selected; the fourth
/// (Alto sax vs Tenor sax) carries the problem "Made by a newer version of the app.".
pub fn vm() -> ViewModel {
    let (entries, covers): (Vec<_>, BTreeMap<_, _>) = sample::library()
        .into_iter()
        .map(|(entry, cover)| {
            let folder = entry.folder.clone();
            (entry, (folder, cover))
        })
        .unzip();
    let selected = entries.get(1).map(|e| e.folder.clone());
    on_library(LibraryView {
        entries,
        covers,
        selected,
        scanning: false,
        problem: None,
    })
}

/// The Library open on an output folder that holds no videos yet.
pub fn empty() -> ViewModel {
    on_library(LibraryView::default())
}

/// The Library open on an output folder that is missing: no videos, and the store's problem in words.
pub fn missing() -> ViewModel {
    let mut vm = on_library(LibraryView::default());
    vm.library.problem = Some(missing_text(&vm));
    vm.folder = FolderState::Missing;
    vm.status = AppState::showing(vm.clone()).status();
    vm
}

/// The base with the Library showing `library`.
fn on_library(library: LibraryView) -> ViewModel {
    let mut vm = base::vm();
    vm.view = View::Library;
    vm.library = library;
    vm.status = AppState::showing(vm.clone()).status();
    vm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_fixture_is_the_sample() {
        let vm = vm();
        assert_eq!(vm.view, View::Library);
        let lib = &vm.library;
        assert_eq!(lib.entries.len(), 6);
        assert_eq!(lib.covers.len(), 6);
        assert!(lib.entries.iter().all(|e| lib.covers.contains_key(&e.folder)));
        assert_eq!(lib.entries[0].title, "Flute vs Trombone");
        assert_eq!(lib.selected.as_ref(), Some(&lib.entries[1].folder));
        assert_eq!(lib.entries[1].title, "Tuba vs Bass drum");
        assert_eq!(
            lib.entries[3].problem.as_deref(),
            Some("Made by a newer version of the app.")
        );
        assert_eq!(lib.entries.iter().filter(|e| e.problem.is_some()).count(), 1);
        assert!(!lib.scanning && lib.problem.is_none());
        assert_eq!(AppState::showing(vm.clone()).vm, vm);
    }

    #[test]
    fn the_empty_and_missing_fixtures_have_no_videos() {
        let vm = empty();
        assert_eq!(vm.view, View::Library);
        assert!(vm.library.entries.is_empty() && vm.library.problem.is_none());
        assert_eq!(AppState::showing(vm.clone()).vm, vm);
        let vm = missing();
        assert!(vm.library.entries.is_empty());
        assert_eq!(vm.library.problem, Some(missing_text(&vm)));
        assert_eq!(vm.folder, FolderState::Missing);
        assert_eq!(AppState::showing(vm.clone()).vm, vm);
    }
}
