//! The Settings view's view models.

use std::path::PathBuf;

use super::base;
use crate::model::View;
use crate::seam::upkeep::{sample_removal_words, sample_removed_words};
use crate::state::upkeep::RemovalView;
use crate::state::{AppState, ViewModel};
use crate::update::{UpdateState, UpdateView};

/// The version line the fixtures show (the real one names the commit, which changes with every commit).
pub const APP_LINE: &str = "0.1.0 (a842e11, 12 Nov 2026)";

/// The fault reports folder of the made-up install.
pub fn reports() -> PathBuf {
    PathBuf::from(r"C:\Users\owner\AppData\Local\BALLFIGHT Studio\data\logs")
}

/// The base with Settings open: output folder `Videos\BALLFIGHT`, 7 workers on 8 cores, Best quality, ffmpeg found
/// as "ffmpeg 9.0.2, found on this PC" (the engine's whole line: the version and where it came from), and the app
/// looking for a newer version (`Checking`), with its fault reports folder known and no guide.
pub fn vm() -> ViewModel {
    let mut vm = base::vm();
    vm.view = View::Settings;
    vm.cores = 8;
    vm.settings.workers = 7;
    vm.ffmpeg = Some(Ok("ffmpeg 9.0.2, found on this PC".to_string()));
    vm.app_line = APP_LINE.to_string();
    vm.update = UpdateView {
        state: UpdateState::Checking,
        previous: None,
        previous_button: None,
        reports: Some(reports()),
        guide: None,
        close: None,
        note: None,
    };
    vm.status = AppState::showing(vm.clone()).status();
    vm
}

/// [`vm`] in `state`, as an installed copy with its guide and with version 0.0.9 kept to go back to.
pub fn with_update(state: UpdateState) -> ViewModel {
    let mut vm = vm();
    vm.update.state = state;
    vm.update.guide = Some(PathBuf::from(
        r"C:\Users\owner\AppData\Local\Programs\BALLFIGHT Studio\guide\index.html",
    ));
    vm.update.previous = Some("0.0.9".to_string());
    vm.update.previous_button = Some("Go back to version 0.0.9".to_string());
    vm
}

/// The seven states of an update, each with a name.
pub fn each_update_state() -> Vec<(&'static str, ViewModel)> {
    let failed = crate::update::check_failed("git could not reach the repository");
    vec![
        ("not installed", vm_not_installed()),
        ("checking", vm()),
        ("up to date", with_update(UpdateState::UpToDate)),
        (
            "available",
            with_update(UpdateState::Available {
                version: "0.2.0".to_string(),
            }),
        ),
        (
            "building",
            with_update(UpdateState::Building {
                version: "0.2.0".to_string(),
                seconds: 187,
                doing: "Building bf-app.".to_string(),
            }),
        ),
        (
            "ready",
            with_update(UpdateState::Ready {
                version: "0.2.0".to_string(),
            }),
        ),
        ("failed", with_update(UpdateState::Failed { text: failed })),
    ]
}

/// A copy that was not installed: it does not update itself and cannot be removed.
pub fn vm_not_installed() -> ViewModel {
    let mut vm = vm();
    vm.update.state = UpdateState::NotInstalled;
    vm
}

/// An installed, up-to-date copy asking whether to remove it.
pub fn asking_removal() -> ViewModel {
    let mut vm = with_update(UpdateState::UpToDate);
    vm.removal = RemovalView::Asking(sample_removal_words());
    vm
}

/// An installed copy just removed, showing what was removed before it closes.
pub fn removed() -> ViewModel {
    let mut vm = with_update(UpdateState::UpToDate);
    vm.update.close = Some(sample_removed_words());
    vm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_fixture_has_8_cores_and_ffmpeg() {
        let vm = vm();
        assert_eq!(vm.view, View::Settings);
        assert_eq!((vm.cores, vm.settings.workers), (8, 7));
        assert_eq!(vm.ffmpeg, Some(Ok("ffmpeg 9.0.2, found on this PC".to_string())));
        assert_eq!(vm.settings.encode, crate::model::EncodeChoice::Best);
        assert_eq!(vm.update.state, UpdateState::Checking, "the fixtures keep Checking");
        assert_eq!(vm.app_line, APP_LINE);
        assert_eq!(AppState::showing(vm.clone()).vm, vm);
        for (name, vm) in each_update_state()
            .into_iter()
            .chain([("asking", asking_removal()), ("removed", removed())])
        {
            assert_eq!(AppState::showing(vm.clone()).vm, vm, "{name}");
        }
    }
}
