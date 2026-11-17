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
        return (tidy(history), None);
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

/// A history as read from a file, made safe to show: a row naming a retired fighter (only a hand-edited file can
/// hold one) is left out, and a row whose id another row already has gets a new id, so every row is told apart.
fn tidy(mut h: History) -> History {
    let retired = |name: &str| bf_core::fightfile::RETIRED.contains(&name);
    h.fights
        .retain(|row| !row.fighters.iter().any(|f| retired(f)) && !row.winner.as_deref().is_some_and(retired));
    let mut next = h
        .fights
        .iter()
        .map(|row| row.id.saturating_add(1))
        .max()
        .unwrap_or(0)
        .max(h.next_id);
    let mut seen = std::collections::BTreeSet::new();
    for row in &mut h.fights {
        if !seen.insert(row.id) {
            row.id = next;
            seen.insert(next);
            next = next.saturating_add(1);
        }
    }
    h.next_id = next;
    h
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
        h.record(
            ["FLUTE".to_string(), "TROMBONE".to_string()],
            bf_core::Engine::One,
            2210,
            None,
            1_763_208_000,
        );
        h.set_uploaded(3, false);
        save_history(&path, &h).unwrap();
        assert_eq!(load_history(&path), (h, None));
    }

    #[test]
    fn a_hand_edited_file_loses_retired_names_and_duplicate_ids() {
        let root = TempRoot::new("a_hand_edited_file_loses_retired_names_and_duplicate_ids");
        let path = root.paths().history();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let text = r#"next_id = 1
[[fights]]
id = 0
fighters = ["GUITAR", "PIANO"]
made = 1
uploaded = true
[[fights]]
id = 0
fighters = ["FLUTE", "TUBA"]
made = 2
[[fights]]
id = 0
fighters = ["FLUTE", "OBOE"]
made = 3
"#;
        fs::write(&path, text).unwrap();
        let (h, notice) = load_history(&path);
        assert_eq!(notice, None);
        let ids: Vec<(u64, &str)> = h.fights.iter().map(|r| (r.id, r.fighters[1].as_str())).collect();
        assert_eq!(ids, vec![(0, "TUBA"), (1, "OBOE")]);
        assert_eq!(h.next_id, 2);
    }

    /// The history file cannot be opened at start (another program holds it): the starter rows are used, and before
    /// the first save the owner's file is kept as `history.unread-1.toml`, never written over.
    #[cfg(windows)]
    #[test]
    fn an_unread_history_is_kept_before_the_first_save() {
        use std::os::windows::fs::OpenOptionsExt;

        use crate::store::Writer;
        let root = TempRoot::new("an_unread_history_is_kept_before_the_first_save");
        let paths = root.paths();
        let path = paths.history();
        let mut own = starter();
        own.record(
            ["FLUTE".to_string(), "TROMBONE".to_string()],
            bf_core::Engine::One,
            2210,
            None,
            1_794_744_000,
        );
        save_history(&path, &own).unwrap();
        let before = fs::read(&path).unwrap();
        let held = fs::OpenOptions::new().read(true).share_mode(0).open(&path).unwrap();
        let (h, notice) = load_history(&path);
        drop(held);
        assert_eq!(h, starter());
        let text = notice.unwrap().text;
        assert!(
            text.starts_with("The matchup history file could not be opened"),
            "{text}"
        );
        assert!(text.contains("history.unread-1.toml"), "{text}");
        let writer = Writer::start(paths.clone());
        writer.history(&h);
        writer.flush();
        assert!(writer.problems().is_empty());
        assert_eq!(fs::read(paths.config.join("history.unread-1.toml")).unwrap(), before);
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
