//! Updates, the guide and removal, as the state keeps them (plan M6, Task 13): what Settings' "Updates" and
//! "Remove" rows and the toolbar's "Update" button show, and which requests reach the upkeep seam
//! (`seam::upkeep`).
//!
//! The rules:
//! - "Restart now" and "Go back" are refused while a video renders (`update::RENDERING`); "Go back" also while a new
//!   version is being built.
//! - Removing asks first: `AskRemove` shows what would be removed and what is kept (Task 11's `removal_words`,
//!   worked out off the interface thread), `KeepApp` goes back, and only `Upkeep(Remove)` while that question is
//!   shown sends the removal. It is refused while a video renders, while a new version is being built, and in a copy
//!   that was not installed.
//! - The seam's view is copied in after every action and event and once a frame (`player_view`); the seam is told
//!   whether a video renders at the same moments.

use super::{AppState, ViewModel};
use crate::model::View;
use crate::seam::upkeep::Upkeep;
use crate::update::worker::Request;
use crate::update::{RENDERING, UpdateState};

/// The toolbar's and Settings' words for a copy that was not installed.
pub const NOT_INSTALLED: &str = "This copy was not installed, so it cannot update itself.";
/// Why such a copy cannot be removed.
pub const NOTHING_TO_REMOVE: &str = "This copy was not installed, so there is nothing to remove.";
/// Why the app is not removed while a video renders.
pub const REMOVE_RENDERING: &str = "A video is rendering. Remove the app when it is done, or cancel it first.";
/// Why the app is not removed, and does not go back, while a new version is being built.
pub const WAIT_FOR_BUILD: &str = "A new version is being built. Stop building first, or wait until it is done.";
/// While the app looks for a newer version.
pub const CHECKING: &str = "Looking for a newer version.";
/// When there is none.
pub const UP_TO_DATE: &str = "This is the newest version.";
/// While the app works out what a removal would do.
pub const LOOKING_AT_REMOVAL: &str = "Looking at what would be removed.";
/// After "Remove the app", until it is done.
pub const REMOVING: &str = "Removing BALLFIGHT Studio.";
/// How long a build takes, in whole minutes (Task 12's warm build, rounded up: `m6-measurements.md` section 3).
pub const BUILD_MINUTES: u32 = 2;

/// "Version {v} is ready to be built on this PC. ..."
pub fn available(version: &str) -> String {
    format!(
        "Version {version} is ready to be built on this PC. It takes about {BUILD_MINUTES} minutes, and you can keep \
         working."
    )
}

/// "Building version {v}: {mm:ss} so far."
pub fn building(version: &str, seconds: u64) -> String {
    format!(
        "Building version {version}: {:02}:{:02} so far.",
        seconds / 60,
        seconds % 60
    )
}

/// "Version {v} is ready."
pub fn ready(version: &str) -> String {
    format!("Version {version} is ready.")
}

/// What the "Remove" row shows.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum RemovalView {
    /// The button "Remove BALLFIGHT Studio from this PC".
    #[default]
    Closed,
    /// Working out what would be removed.
    Looking,
    /// The question: what would be removed and what is kept, then "Remove the app" and "Keep it".
    Asking(Vec<String>),
    /// Why it cannot be removed, then "Keep it".
    Refused(String),
    /// "Remove the app" was clicked; waiting for the removal's words.
    Removing {
        /// The seam's note when it was sent: a different note afterwards is the removal's refusal.
        note_before: Option<String>,
    },
}

/// Why "Restart now" (and "Go back") cannot be used now; `None` when it can.
pub fn why_not_restart(vm: &ViewModel) -> Option<&'static str> {
    vm.queue.rendering().is_some().then_some(RENDERING)
}

/// Why "Go back to version W" cannot be used now; `None` when it can.
pub fn why_not_go_back(vm: &ViewModel) -> Option<&'static str> {
    if matches!(vm.update.state, UpdateState::Building { .. }) {
        return Some(WAIT_FOR_BUILD);
    }
    why_not_restart(vm)
}

/// Why the app cannot be removed now; `None` when it can.
pub fn why_not_remove(vm: &ViewModel) -> Option<&'static str> {
    match vm.update.state {
        UpdateState::NotInstalled => Some(NOTHING_TO_REMOVE),
        UpdateState::Building { .. } => Some(WAIT_FOR_BUILD),
        _ if vm.queue.rendering().is_some() => Some(REMOVE_RENDERING),
        _ => None,
    }
}

impl AppState {
    /// Gives the state the upkeep it talks to (`main.rs`: the real one, for an installed copy's real start).
    pub fn set_upkeep(&mut self, upkeep: Box<dyn Upkeep>) {
        self.upkeep = upkeep;
        self.sync_upkeep();
    }

    /// The requests the upkeep has been sent (the sample keeps them; the real one keeps none).
    pub fn upkeep_sent(&self) -> Vec<Request> {
        self.upkeep.sent()
    }

    /// `Upkeep(r)`: a request for the upkeep, unless the rules above refuse it.
    pub(super) fn upkeep_request(&mut self, r: Request) {
        let refused = match r {
            Request::Restart => why_not_restart(&self.vm).is_some(),
            Request::GoBack => why_not_go_back(&self.vm).is_some(),
            Request::Remove => !matches!(self.vm.removal, RemovalView::Asking(_)) || why_not_remove(&self.vm).is_some(),
            _ => false,
        };
        if refused {
            return;
        }
        if r == Request::Remove {
            self.vm.removal = RemovalView::Removing {
                note_before: self.vm.update.note.clone(),
            };
        }
        self.upkeep.send(r);
    }

    /// `AskRemove`: works out what a removal would do, to ask first.
    pub(super) fn ask_remove(&mut self) {
        if why_not_remove(&self.vm).is_none() && matches!(self.vm.removal, RemovalView::Closed) {
            self.vm.removal = RemovalView::Looking;
            self.upkeep.plan_removal();
        }
    }

    /// `KeepApp`: the question goes away and nothing is removed.
    pub(super) fn keep_app(&mut self) {
        if !matches!(self.vm.removal, RemovalView::Removing { .. }) {
            self.vm.removal = RemovalView::Closed;
        }
    }

    /// Copies the upkeep's view in and tells it whether a video renders. A removal's answer becomes the question;
    /// once the app is to close (`close`), Settings is shown, where the closing words are.
    pub(super) fn sync_upkeep(&mut self) {
        self.upkeep.set_rendering(self.vm.queue.rendering().is_some());
        self.vm.update = self.upkeep.view();
        match &self.vm.removal {
            RemovalView::Looking => match self.upkeep.removal() {
                Some(Ok(words)) => self.vm.removal = RemovalView::Asking(words),
                Some(Err(text)) => self.vm.removal = RemovalView::Refused(text),
                None => {}
            },
            RemovalView::Removing { note_before }
                if self.vm.update.close.is_none()
                    && self.vm.update.note.is_some()
                    && self.vm.update.note != *note_before =>
            {
                self.vm.removal = RemovalView::Closed;
            }
            _ => {}
        }
        if self.vm.update.close.as_ref().is_some_and(|words| !words.is_empty()) {
            self.vm.view = View::Settings;
        }
    }
}
