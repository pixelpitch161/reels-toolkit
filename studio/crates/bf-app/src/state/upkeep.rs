//! Updates, the guide and removal, as the state keeps them (plan M6, Task 13): what Settings' "Updates" and
//! "Remove" rows and the toolbar's "Update" button show, and which requests reach the upkeep seam
//! (`seam::upkeep`).
//!
//! The rules:
//! - "Restart now" and "Go back" are refused while a video renders (`update::RENDERING`); "Go back" also while a new
//!   version is being built.
//! - Removing asks first: `AskRemove` shows what would be removed and what is kept (Task 11's `removal_words`,
//!   worked out off the interface thread, afresh each time it is asked), `KeepApp` goes back, and only
//!   `Upkeep(Remove)` while that question is shown sends the removal. It is refused while a video renders, while a
//!   new version is being built, and in a copy that was not installed. Leaving Settings closes the question.
//! - Every answer of the upkeep counts (`Upkeep::answered`): a removal that was not done returns to the question with
//!   its sentence, however often the same sentence comes back.
//! - Once the app is to close (`UpdateView::close`: after a removal, or after "Restart now" or "Go back" started the
//!   program that finishes them), nothing else is sent: the closing words and "Close BALLFIGHT Studio" are all that
//!   is left to use.
//! - `AppState::poll_upkeep` copies the seam's view in: after every action and event, and once a frame from the app's
//!   `logic`; the seam is told whether a video renders at the same moments.

use super::{AppState, ViewModel};
use crate::model::View;
use crate::seam::upkeep::Upkeep;
use crate::update::worker::Request;
use crate::update::{RENDERING, UpdateState};

/// The toolbar's and Settings' words for a copy that was not installed.
pub const NOT_INSTALLED: &str = "This copy was not installed, so it cannot update itself.";
/// Why such a copy cannot be removed.
pub const NOTHING_TO_REMOVE: &str = "This copy was not installed, so it cannot remove the app.";
/// Why the app is not removed while a video renders.
pub const REMOVE_RENDERING: &str = "A video is rendering. Remove the app when it is done, or cancel it first.";
/// Why the app is not removed, and does not go back, while a new version is being built.
pub const WAIT_FOR_BUILD: &str = "A new version is being built. Stop building first, or wait until it is done.";
/// Why nothing can be used once the app is to close.
pub const CLOSING: &str = "BALLFIGHT Studio is closing, so nothing more can be done here.";
/// While the app looks for a newer version.
pub const CHECKING: &str = "Looking for a newer version.";
/// When there is none.
pub const UP_TO_DATE: &str = "This is the newest version.";
/// While the app works out what a removal would do.
pub const LOOKING_AT_REMOVAL: &str = "Looking at what would be removed.";
/// After "Remove the app", until it is done.
pub const REMOVING: &str = "Removing BALLFIGHT Studio.";
/// A removal that was not done and gave no reason.
pub const NOT_REMOVED: &str = "BALLFIGHT Studio was not removed.";

/// "Version {v} is ready to be built on this PC. ..." One wording for every build: the first build of an install
/// took about 6 minutes and a later one about 2 (`m6-measurements.md` section 3), so no single number is promised.
pub fn available(version: &str) -> String {
    format!(
        "Version {version} is ready to be built on this PC. It takes a few minutes, longer the first time, and you \
         can keep working."
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
    /// The question: what would be removed and what is kept, then "Remove the app" and "Keep it"; `problem` is why
    /// the last "Remove the app" did not remove it.
    Asking {
        words: Vec<String>,
        problem: Option<String>,
    },
    /// Why it cannot be removed, then "Keep it".
    Refused(String),
    /// "Remove the app" was clicked; waiting for the answer.
    Removing {
        /// The question's words, shown again if the removal is not done.
        words: Vec<String>,
        /// The upkeep's count of answers when it was sent: any later answer is this one's.
        answered_before: u64,
    },
}

impl RemovalView {
    /// The question, as `AskRemove` first shows it.
    pub fn asking(words: Vec<String>) -> RemovalView {
        RemovalView::Asking { words, problem: None }
    }
}

/// Whether the app is to close: nothing more is sent then.
pub fn closing(vm: &ViewModel) -> bool {
    vm.update.close.is_some()
}

/// Why "Restart now" (and "Go back") cannot be used now; `None` when it can.
pub fn why_not_restart(vm: &ViewModel) -> Option<&'static str> {
    if closing(vm) {
        return Some(CLOSING);
    }
    vm.queue.rendering().is_some().then_some(RENDERING)
}

/// Why "Go back to version W" cannot be used now; `None` when it can.
pub fn why_not_go_back(vm: &ViewModel) -> Option<&'static str> {
    if matches!(vm.update.state, UpdateState::Building { .. }) && !closing(vm) {
        return Some(WAIT_FOR_BUILD);
    }
    why_not_restart(vm)
}

/// Why the app cannot be removed now; `None` when it can.
pub fn why_not_remove(vm: &ViewModel) -> Option<&'static str> {
    match vm.update.state {
        _ if closing(vm) => Some(CLOSING),
        UpdateState::NotInstalled => Some(NOTHING_TO_REMOVE),
        UpdateState::Building { .. } => Some(WAIT_FOR_BUILD),
        _ if vm.queue.rendering().is_some() => Some(REMOVE_RENDERING),
        _ => None,
    }
}

impl AppState {
    /// Gives the state the upkeep it talks to (`main.rs`: the one for this start, `seam::upkeep::for_this_start`).
    pub fn set_upkeep(&mut self, upkeep: Box<dyn Upkeep>) {
        self.upkeep = upkeep;
        self.poll_upkeep();
    }

    /// The requests the upkeep has been sent (the sample keeps them; the real one keeps none).
    pub fn upkeep_sent(&self) -> Vec<Request> {
        self.upkeep.sent()
    }

    /// `Upkeep(r)`: a request for the upkeep, unless the rules above refuse it.
    pub(super) fn upkeep_request(&mut self, r: Request) {
        if closing(&self.vm) {
            return;
        }
        let refused = match r {
            Request::Restart => why_not_restart(&self.vm).is_some(),
            Request::GoBack => why_not_go_back(&self.vm).is_some(),
            Request::Remove => {
                !matches!(self.vm.removal, RemovalView::Asking { .. }) || why_not_remove(&self.vm).is_some()
            }
            _ => false,
        };
        if refused {
            return;
        }
        if r == Request::Remove
            && let RemovalView::Asking { words, .. } = &self.vm.removal
        {
            self.vm.removal = RemovalView::Removing {
                words: words.clone(),
                answered_before: self.upkeep.answered(),
            };
        }
        self.upkeep.send(r);
    }

    /// `AskRemove`: works out what a removal would do, to ask first.
    pub(super) fn ask_remove(&mut self) {
        if closing(&self.vm) {
            return;
        }
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

    /// Copies the upkeep's view in and tells it whether a video renders: after every action and event (`dispatch`,
    /// `apply`), and once a frame from the app's `logic`, since the upkeep's work (a look, a build, a removal plan)
    /// ends on threads of its own. A removal plan's answer becomes the question; a removal's answer that did not
    /// remove the app returns to it with its sentence; once the app is to close with words to say, Settings is shown,
    /// where they are; a question left open when Settings is left is closed.
    pub fn poll_upkeep(&mut self) {
        self.upkeep.set_rendering(self.vm.queue.rendering().is_some());
        self.vm.update = self.upkeep.view();
        let answered = self.upkeep.answered();
        match &self.vm.removal {
            RemovalView::Looking => match self.upkeep.removal() {
                Some(Ok(words)) => self.vm.removal = RemovalView::asking(words),
                Some(Err(text)) => self.vm.removal = RemovalView::Refused(text),
                None => {}
            },
            RemovalView::Removing { words, answered_before }
                if answered > *answered_before && self.vm.update.close.is_none() =>
            {
                let problem = self.vm.update.note.clone().unwrap_or_else(|| NOT_REMOVED.to_string());
                self.vm.removal = RemovalView::Asking {
                    words: words.clone(),
                    problem: Some(problem),
                };
            }
            _ => {}
        }
        if self.vm.update.close.as_ref().is_some_and(|words| !words.is_empty()) {
            self.vm.view = View::Settings;
        }
        if self.vm.view != View::Settings
            && matches!(
                self.vm.removal,
                RemovalView::Looking | RemovalView::Asking { .. } | RemovalView::Refused(_)
            )
        {
            self.vm.removal = RemovalView::Closed;
        }
    }
}
