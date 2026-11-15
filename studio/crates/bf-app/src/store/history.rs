//! The matchup history's file, `config/history.toml`: every fight the app has made, kept by the app itself (moving
//! or deleting a video never changes it). With no file yet, the history starts from the episodes made before the
//! app (`assets/history_start.toml`, made once from the matchup tracker by `studio/scripts/history_from_tracker.py`).

use std::io;
use std::path::Path;

use super::{clear_unread, keep_copy, mark_unread, move_aside, next_aside_name, read_patiently, write_atomic};
use crate::model::Notice;
use crate::model::history::History;

/// The bundled starting rows.
const STARTER: &str = include_str!("../../assets/history_start.toml");

/// The history the app starts with when it has none: the earlier episodes, all ticked as uploaded.
///
/// # Panics
/// When the bundled file cannot be read (a test pins it on every build).
pub fn starter() -> History {
    toml::from_str(STARTER).unwrap_or_else(|e| panic!("the bundled matchup history cannot be read: {e}"))
}

/// Reads the history file. A missing file gives the starter rows and no notice. A file that cannot be understood
/// is moved aside as `history.corrupt-N.toml` (or, if another program holds it, copied there) and the starter rows
/// are used, with a notice that names the kept copy. A file that cannot be opened at all gives the starter rows and
/// a notice and stays where it is; the writer moves it aside as `history.unread-N.toml` before it saves over it.
pub fn load_history(path: &Path) -> (History, Option<Notice>) {
    let bytes = match read_patiently(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            clear_unread(path);
            return (starter(), None);
        }
        Err(_) => {
            mark_unread(path);
            let text = format!(
                "The matchup history file could not be opened, so the list shows only the earlier episodes for now. \
                 It may be open in another program: close that program and start the app again to get your history \
                 back. If the history changes first, the old file is kept as {}.",
                next_aside_name(path, "unread")
            );
            return (starter(), Some(Notice { text }));
        }
    };
    clear_unread(path);
    let parsed = std::str::from_utf8(&bytes)
        .ok()
        .and_then(|text| toml::from_str::<History>(text).ok());
    if let Some(history) = parsed {
        return (history, None);
    }
    let text = match move_aside(path, "corrupt").or_else(|_| keep_copy(path, "corrupt", &bytes)) {
        Ok(aside) => format!(
            "The matchup history could not be read, so it starts again from the earlier episodes. The old file was \
             kept as {}.",
            aside.file_name().unwrap_or_default().to_string_lossy()
        ),
        Err(_) => {
            mark_unread(path);
            format!(
                "The matchup history could not be read, so it starts again from the earlier episodes. The old file \
                 could not be moved aside, so it stays where it is for now; before the history is saved over it, it \
                 is kept as {}.",
                next_aside_name(path, "unread")
            )
        }
    };
    (starter(), Some(Notice { text }))
}

/// Writes the history file whole and atomically, making its folder if needed.
pub fn save_history(path: &Path, h: &History) -> io::Result<()> {
    let text = toml::to_string(h).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    write_atomic(path, text.as_bytes())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::store::{TempRoot, files_in};

    #[test]
    fn the_starter_holds_the_27_earlier_episodes() {
        let h = starter();
        assert_eq!(h.fights.len(), 27);
        assert_eq!(h.next_id, 27);
        let roster: Vec<&str> = bf_content::roster().iter().map(|f| f.name.as_str()).collect();
        for row in &h.fights {
            assert!(row.uploaded, "{row:?}");
            for name in &row.fighters {
                assert!(!bf_core::fightfile::RETIRED.contains(&name.as_str()), "{name}");
                assert!(roster.contains(&name.as_str()), "{name} is not on the roster");
            }
            let winner = row.winner.as_ref().unwrap();
            assert!(row.fighters.contains(winner), "{row:?}");
        }
        let bagpipes = h.fights.iter().find(|row| row.seed == Some(1442)).unwrap();
        assert_eq!(bagpipes.fighters, ["BAGPIPES".to_string(), "ACCORDION".to_string()]);
        assert_eq!(bagpipes.winner.as_deref(), Some("ACCORDION"));
        // 2026-10-24 12:00 UTC.
        assert_eq!(bagpipes.made, 1_792_843_200);
    }

    #[test]
    fn a_missing_file_starts_from_the_starter() {
        let root = TempRoot::new("a_missing_file_starts_from_the_starter");
        assert_eq!(load_history(&root.paths().history()), (starter(), None));
    }

    #[test]
    fn a_saved_history_loads_back() {
        let root = TempRoot::new("a_saved_history_loads_back");
        let path = root.paths().history();
        let mut h = starter();
        h.record(["FLUTE".to_string(), "TROMBONE".to_string()], 2210, None, 1_763_208_000);
        h.set_uploaded(3, false);
        save_history(&path, &h).unwrap();
        assert_eq!(load_history(&path), (h, None));
    }

    #[test]
    fn a_damaged_file_is_kept_aside_and_the_starter_used() {
        let root = TempRoot::new("a_damaged_file_is_kept_aside_and_the_starter_used");
        let paths = root.paths();
        fs::create_dir_all(&paths.config).unwrap();
        fs::write(paths.history(), "not toml [").unwrap();
        let (h, notice) = load_history(&paths.history());
        assert_eq!(h, starter());
        assert!(notice.unwrap().text.contains("history.corrupt-1.toml"));
        assert_eq!(files_in(&paths.config), vec!["history.corrupt-1.toml"]);
    }
}
