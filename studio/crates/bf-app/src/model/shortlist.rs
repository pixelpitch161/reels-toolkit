//! The seed shortlist and the filters it is shown through.

use super::story::StoryView;

/// How a fight must end to pass the filters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Finish {
    Any,
    Close,
    Comeback,
}

/// The shortlist's filters. The engine applies them; the interface only stores and shows them.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterSpec {
    /// Only fights won by this fighter (0 or 1).
    pub winner: Option<usize>,
    /// The K.O. must land between these seconds.
    pub ko: (f64, f64),
    pub min_lead_changes: u32,
    pub finish: Finish,
    /// Only fights where this ability is used: the roster's own name ("FLUTTER TONGUE"), `AbilityLine::key`.
    pub ability: Option<String>,
    /// Also show fights that do not fit the beat sheet.
    pub show_outside: bool,
}

impl Default for FilterSpec {
    /// The beat sheet's own wishes: a K.O. between 49 and 53 s and at least two lead changes.
    fn default() -> Self {
        FilterSpec {
            winner: None,
            ko: (49.0, 53.0),
            min_lead_changes: 2,
            finish: Finish::Any,
            ability: None,
            show_outside: false,
        }
    }
}

/// Where the background search is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SearchState {
    Idle,
    Searching,
    Stopped,
    Full,
}

/// The shortlist as the engine last reported it.
#[derive(Clone, Debug, PartialEq)]
pub struct Shortlist {
    /// Already filtered and ranked, best first.
    pub rows: Vec<StoryView>,
    /// How many fights found so far fit the beat sheet.
    pub fits_total: u32,
    /// How many seeds have been fought.
    pub checked: u64,
    pub state: SearchState,
    /// Threads the search runs on.
    pub workers: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_default_to_the_beat_sheet() {
        assert_eq!(
            FilterSpec::default(),
            FilterSpec {
                winner: None,
                ko: (49.0, 53.0),
                min_lead_changes: 2,
                finish: Finish::Any,
                ability: None,
                show_outside: false,
            }
        );
    }
}
