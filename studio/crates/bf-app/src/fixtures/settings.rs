//! The Settings view's view model.

use super::base;
use crate::model::View;
use crate::state::{AppState, ViewModel};

/// The base with Settings open: output folder `Videos\BALLFIGHT`, 7 workers on 8 cores, ffmpeg found as
/// "ffmpeg 9.0.2".
pub fn vm() -> ViewModel {
    let mut vm = base::vm();
    vm.view = View::Settings;
    vm.cores = 8;
    vm.settings.workers = 7;
    vm.ffmpeg = Some(Ok("ffmpeg 9.0.2".to_string()));
    vm.status = AppState::showing(vm.clone()).status();
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
        assert_eq!(vm.ffmpeg, Some(Ok("ffmpeg 9.0.2".to_string())));
        assert_eq!(vm.roster.as_ref().map(|r| r.app_version.as_str()), Some("0.1.0"));
        assert_eq!(AppState::showing(vm.clone()).vm, vm);
    }
}
