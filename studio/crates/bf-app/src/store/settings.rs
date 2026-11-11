//! The settings file, `config/settings.toml`: where videos are saved and how many threads the engine may use.

use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{clear_unread, keep_copy, mark_unread, move_aside, next_aside_name, read_patiently, write_atomic};
use crate::model::{Notice, Settings};

/// The settings file's format. A file with a higher number was written by a newer app and is moved aside.
pub const SETTINGS_FORMAT: u32 = 1;

/// The settings as the file holds them.
#[derive(Serialize)]
struct SettingsFile<'a> {
    format: u32,
    output_folder: &'a Path,
    workers: usize,
}

/// The settings a first run starts with: videos go in `<Videos>/BALLFIGHT` (else `<home>/Videos/BALLFIGHT`), and
/// the engine uses every core but one, and at least one.
pub fn default_settings(cores: usize) -> Settings {
    Settings {
        output_folder: default_output_folder(),
        workers: cores.saturating_sub(1).max(1),
    }
}

/// `<Videos>/BALLFIGHT`; without a Videos folder `<home>/Videos/BALLFIGHT`; without a home `Videos/BALLFIGHT`
/// beside the app. Only works the path out: the folder is made when the first video is saved.
fn default_output_folder() -> PathBuf {
    let videos = directories::UserDirs::new().and_then(|dirs| {
        dirs.video_dir()
            .map(Path::to_path_buf)
            .or_else(|| Some(dirs.home_dir().join("Videos")))
    });
    videos.unwrap_or_else(|| PathBuf::from("Videos")).join("BALLFIGHT")
}

/// What a settings file turned out to hold.
enum Read {
    Good(Settings),
    /// Written by a newer app.
    Newer,
    /// Not text, not TOML, or missing a value.
    Corrupt,
}

/// Reads the file's text. `cores` bounds `workers`.
fn parse(bytes: &[u8], cores: usize) -> Read {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Read::Corrupt;
    };
    let Ok(table) = text.parse::<toml::Table>() else {
        return Read::Corrupt;
    };
    // The format is looked at first: a newer file may hold its other values differently.
    match table.get("format").and_then(toml::Value::as_integer) {
        Some(format) if format > i64::from(SETTINGS_FORMAT) => return Read::Newer,
        Some(format) if format == i64::from(SETTINGS_FORMAT) => {}
        _ => return Read::Corrupt,
    }
    let (Some(folder), Some(workers)) = (
        table.get("output_folder").and_then(toml::Value::as_str),
        table.get("workers").and_then(toml::Value::as_integer),
    ) else {
        return Read::Corrupt;
    };
    let output_folder = if folder.is_empty() {
        default_output_folder()
    } else {
        PathBuf::from(folder)
    };
    let most = i64::try_from(cores.max(1)).unwrap_or(i64::MAX);
    let workers = usize::try_from(workers.clamp(1, most)).unwrap_or(1);
    Read::Good(Settings { output_folder, workers })
}

/// Reads the settings file. A missing file gives the defaults and no notice. A file that cannot be understood or was
/// written by a newer app gives the defaults and a notice in words, and is moved aside as `settings.corrupt-N.toml`
/// or `settings.newer-N.toml` rather than deleted (or, if another program holds it, copied there). A file that
/// cannot be opened at all gives the defaults and a notice, and is left where it is; the writer moves it aside as
/// `settings.unread-N.toml` before it saves settings over it (or refuses to save). `workers` is kept between 1 and
/// `cores`. The output folder is not looked at: a missing one is reported where videos are saved.
pub fn load_settings(path: &Path, cores: usize) -> (Settings, Option<Notice>) {
    let defaults = default_settings(cores);
    let bytes = match read_patiently(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            clear_unread(path);
            return (defaults, None);
        }
        Err(_) => {
            // Kept where it is; the writer moves it aside before it saves settings over it.
            mark_unread(path);
            let text = format!(
                "Your settings file could not be opened, so the app is using its defaults for now. It may be open in \
                 another program: close that program and start the app again to get your settings back. If you \
                 change a setting first, the old file is kept as {}.",
                next_aside_name(path, "unread")
            );
            return (defaults, Some(Notice { text }));
        }
    };
    clear_unread(path);
    let (tag, lead, kept_as) = match parse(&bytes, cores) {
        Read::Good(settings) => return (settings, None),
        Read::Newer => (
            "newer",
            "Your settings were saved by a newer version of BALLFIGHT Studio, so the app is using its defaults.",
            "The file was kept as",
        ),
        Read::Corrupt => (
            "corrupt",
            "Your settings file could not be read, so the app is using its defaults.",
            "The old file was kept as",
        ),
    };
    let text = match move_aside(path, tag).or_else(|_| keep_copy(path, tag, &bytes)) {
        Ok(kept) => format!(
            "{lead} {kept_as} {}.",
            kept.file_name().unwrap_or_default().to_string_lossy()
        ),
        Err(_) => format!(
            "{lead} It could not be moved aside, so copy {} somewhere safe before you change a setting.",
            path.display()
        ),
    };
    (defaults, Some(Notice { text }))
}

/// Writes the settings file (`format`, `output_folder`, `workers`) atomically, making its folder if needed.
pub fn save_settings(path: &Path, s: &Settings) -> io::Result<()> {
    let file = SettingsFile {
        format: SETTINGS_FORMAT,
        output_folder: &s.output_folder,
        workers: s.workers,
    };
    let text = toml::to_string(&file).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    write_atomic(path, text.as_bytes())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::store::{TempRoot, Writer, files_in};

    fn mine() -> Settings {
        Settings {
            output_folder: PathBuf::from(r"C:\Users\Zoë Ångström\Videos\My BALLFIGHT 動画"),
            workers: 5,
        }
    }

    fn notice(text: &str) -> Option<Notice> {
        Some(Notice { text: text.to_string() })
    }

    /// `notice`, for tests with a local of that name.
    #[cfg(windows)]
    fn notice_of(text: &str) -> Option<Notice> {
        notice(text)
    }

    #[test]
    fn defaults_leave_one_core_free() {
        assert_eq!(default_settings(8).workers, 7);
        assert_eq!(default_settings(1).workers, 1);
        assert_eq!(default_settings(0).workers, 1);
        assert!(default_settings(8).output_folder.ends_with("BALLFIGHT"));
    }

    #[test]
    fn settings_round_trip() {
        let root = TempRoot::new("settings_round_trip");
        let path = root.paths().settings();
        save_settings(&path, &mine()).unwrap();
        assert_eq!(load_settings(&path, 8), (mine(), None));
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("format = 1"), "{text}");
        assert_eq!(files_in(&root.paths().config), vec!["settings.toml"]);
    }

    #[test]
    fn a_missing_file_gives_defaults_and_no_notice() {
        let root = TempRoot::new("a_missing_file_gives_defaults_and_no_notice");
        let path = root.paths().settings();
        assert_eq!(load_settings(&path, 8), (default_settings(8), None));
        assert!(!root.paths().config.exists());
    }

    #[test]
    fn a_missing_output_folder_is_kept_and_not_made() {
        let root = TempRoot::new("a_missing_output_folder_is_kept_and_not_made");
        let path = root.paths().settings();
        let gone = Settings {
            output_folder: root.0.join("unplugged drive").join("BALLFIGHT"),
            workers: 3,
        };
        save_settings(&path, &gone).unwrap();
        assert_eq!(load_settings(&path, 8), (gone.clone(), None));
        assert!(!gone.output_folder.exists());
    }

    #[test]
    fn a_corrupt_file_is_moved_aside() {
        let root = TempRoot::new("a_corrupt_file_is_moved_aside");
        let config = root.paths().config;
        let path = root.paths().settings();
        fs::create_dir_all(&config).unwrap();
        fs::write(&path, "output_folder = [[[").unwrap();
        assert_eq!(
            load_settings(&path, 8),
            (
                default_settings(8),
                notice(
                    "Your settings file could not be read, so the app is using its defaults. The old file was kept \
                     as settings.corrupt-1.toml."
                )
            )
        );
        assert_eq!(
            fs::read(config.join("settings.corrupt-1.toml")).unwrap(),
            b"output_folder = [[["
        );
        assert!(!path.exists());
        fs::write(&path, "workers = ").unwrap();
        let (settings, second) = load_settings(&path, 8);
        assert_eq!(settings, default_settings(8));
        assert!(
            second
                .unwrap()
                .text
                .ends_with("The old file was kept as settings.corrupt-2.toml.")
        );
        assert_eq!(fs::read(config.join("settings.corrupt-2.toml")).unwrap(), b"workers = ");
        assert_eq!(
            files_in(&config),
            vec!["settings.corrupt-1.toml", "settings.corrupt-2.toml"]
        );
    }

    #[test]
    fn a_half_written_file_is_corrupt() {
        let root = TempRoot::new("a_half_written_file_is_corrupt");
        let config = root.paths().config;
        let path = root.paths().settings();
        save_settings(&path, &mine()).unwrap();
        let good = fs::read(&path).unwrap();
        fs::write(&path, &good[..10]).unwrap();
        let (settings, notice) = load_settings(&path, 8);
        assert_eq!(settings, default_settings(8));
        assert!(notice.unwrap().text.starts_with("Your settings file could not be read"));
        assert_eq!(fs::read(config.join("settings.corrupt-1.toml")).unwrap(), &good[..10]);
        assert!(!path.exists());
    }

    #[test]
    fn a_file_that_is_not_text_is_corrupt() {
        let root = TempRoot::new("a_file_that_is_not_text_is_corrupt");
        let path = root.paths().settings();
        fs::create_dir_all(root.paths().config).unwrap();
        fs::write(&path, [0xff, 0xfe, 0x00, 0x66]).unwrap();
        let (settings, notice) = load_settings(&path, 8);
        assert_eq!(settings, default_settings(8));
        assert!(notice.unwrap().text.starts_with("Your settings file could not be read"));
    }

    #[test]
    fn a_newer_file_is_moved_aside() {
        let root = TempRoot::new("a_newer_file_is_moved_aside");
        let config = root.paths().config;
        let path = root.paths().settings();
        fs::create_dir_all(&config).unwrap();
        let newer = "format = 2\noutput_folder = 'E:\\\\Clips'\nworkers = 3\nlanguage = 'fr'\n";
        fs::write(&path, newer).unwrap();
        assert_eq!(
            load_settings(&path, 8),
            (
                default_settings(8),
                notice(
                    "Your settings were saved by a newer version of BALLFIGHT Studio, so the app is using its \
                     defaults. The file was kept as settings.newer-1.toml."
                )
            )
        );
        assert_eq!(fs::read_to_string(config.join("settings.newer-1.toml")).unwrap(), newer);
        assert!(!path.exists());
    }

    #[cfg(windows)]
    fn open_shared(path: &Path, share: u32) -> fs::File {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new().read(true).share_mode(share).open(path).unwrap()
    }

    #[cfg(windows)]
    #[test]
    fn a_corrupt_file_held_open_is_kept_as_a_copy() {
        const FILE_SHARE_READ: u32 = 1;
        let root = TempRoot::new("a_corrupt_file_held_open_is_kept_as_a_copy");
        let config = root.paths().config;
        let path = root.paths().settings();
        fs::create_dir_all(&config).unwrap();
        fs::write(&path, "workers = [").unwrap();
        // Another program reads the file and refuses to let it be moved.
        let held = open_shared(&path, FILE_SHARE_READ);
        let (settings, notice) = load_settings(&path, 8);
        drop(held);
        assert_eq!(settings, default_settings(8));
        assert!(
            notice
                .unwrap()
                .text
                .ends_with("The old file was kept as settings.corrupt-1.toml.")
        );
        assert_eq!(
            fs::read(config.join("settings.corrupt-1.toml")).unwrap(),
            b"workers = ["
        );
        assert_eq!(fs::read(&path).unwrap(), b"workers = [");
    }

    #[cfg(windows)]
    #[test]
    fn a_file_that_cannot_be_opened_is_left_alone() {
        let root = TempRoot::new("a_file_that_cannot_be_opened_is_left_alone");
        let config = root.paths().config;
        let path = root.paths().settings();
        save_settings(&path, &mine()).unwrap();
        let before = fs::read(&path).unwrap();
        let held = open_shared(&path, 0);
        let (settings, notice) = load_settings(&path, 8);
        drop(held);
        assert_eq!(settings, default_settings(8));
        assert_eq!(
            notice,
            notice_of(
                "Your settings file could not be opened, so the app is using its defaults for now. It may be open in \
                 another program: close that program and start the app again to get your settings back. If you \
                 change a setting first, the old file is kept as settings.unread-1.toml."
            )
        );
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(files_in(&config), vec!["settings.toml"]);
    }

    /// Loads `path` while another program holds it with no sharing, so it cannot be opened; returns the old bytes.
    #[cfg(windows)]
    fn load_while_locked(path: &Path) -> Vec<u8> {
        let before = fs::read(path).unwrap();
        let held = open_shared(path, 0);
        let (settings, notice) = load_settings(path, 8);
        drop(held);
        assert_eq!(settings, default_settings(8));
        assert!(
            notice
                .unwrap()
                .text
                .starts_with("Your settings file could not be opened")
        );
        before
    }

    // M2-R9 (a)
    #[cfg(windows)]
    #[test]
    fn an_unread_file_is_kept_before_the_first_save() {
        let root = TempRoot::new("an_unread_file_is_kept_before_the_first_save");
        let paths = root.paths();
        let path = paths.settings();
        save_settings(&path, &mine()).unwrap();
        let before = load_while_locked(&path);
        let writer = Writer::start(paths.clone());
        let changed = Settings {
            workers: 2,
            ..default_settings(8)
        };
        writer.settings(&changed);
        writer.flush();
        assert!(writer.problems().is_empty());
        assert_eq!(fs::read(paths.config.join("settings.unread-1.toml")).unwrap(), before);
        assert_eq!(load_settings(&path, 8), (changed.clone(), None));
        // Kept once: the next save simply writes.
        writer.settings(&changed);
        writer.flush();
        assert_eq!(files_in(&paths.config), vec!["settings.toml", "settings.unread-1.toml"]);
    }

    // M2-R9 (b)
    #[cfg(windows)]
    #[test]
    fn a_save_is_refused_while_the_unread_file_cannot_be_kept() {
        let root = TempRoot::new("a_save_is_refused_while_the_unread_file_cannot_be_kept");
        let paths = root.paths();
        let path = paths.settings();
        save_settings(&path, &mine()).unwrap();
        let before = fs::read(&path).unwrap();
        let held = open_shared(&path, 0);
        assert!(load_settings(&path, 8).1.is_some());
        let writer = Writer::start(paths.clone());
        writer.settings(&default_settings(8));
        writer.flush();
        let problems = writer.problems();
        drop(held);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].starts_with(&format!("Your settings were not saved to {}", path.display())),
            "{}",
            problems[0]
        );
        assert!(problems[0].contains("could not be kept safe"), "{}", problems[0]);
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(files_in(&paths.config), vec!["settings.toml"]);
        // Once the other program lets go, the next save keeps the old file aside and goes ahead.
        let changed = Settings {
            workers: 4,
            ..default_settings(8)
        };
        writer.settings(&changed);
        writer.flush();
        assert!(writer.problems().is_empty());
        assert_eq!(fs::read(paths.config.join("settings.unread-1.toml")).unwrap(), before);
        assert_eq!(load_settings(&path, 8), (changed, None));
    }

    // M2-R9 (c)
    #[test]
    fn a_file_that_loaded_is_never_moved_by_a_save() {
        let root = TempRoot::new("a_file_that_loaded_is_never_moved_by_a_save");
        let paths = root.paths();
        let path = paths.settings();
        save_settings(&path, &mine()).unwrap();
        assert_eq!(load_settings(&path, 8), (mine(), None));
        let writer = Writer::start(paths.clone());
        writer.settings(&default_settings(8));
        writer.flush();
        assert!(writer.problems().is_empty());
        assert_eq!(files_in(&paths.config), vec!["settings.toml"]);
    }

    // M2-R9 (d)
    #[cfg(windows)]
    #[test]
    fn unread_files_are_kept_per_path() {
        let a = TempRoot::new("unread_files_are_kept_per_path a");
        let b = TempRoot::new("unread_files_are_kept_per_path b");
        save_settings(&a.paths().settings(), &mine()).unwrap();
        save_settings(&b.paths().settings(), &mine()).unwrap();
        load_while_locked(&a.paths().settings());
        assert_eq!(load_settings(&b.paths().settings(), 8), (mine(), None));
        let writer_b = Writer::start(b.paths());
        writer_b.settings(&default_settings(8));
        writer_b.flush();
        assert_eq!(files_in(&b.paths().config), vec!["settings.toml"]);
        let writer_a = Writer::start(a.paths());
        writer_a.settings(&default_settings(8));
        writer_a.flush();
        assert!(writer_a.problems().is_empty());
        assert_eq!(
            files_in(&a.paths().config),
            vec!["settings.toml", "settings.unread-1.toml"]
        );
    }

    #[test]
    fn workers_stay_between_1_and_the_cores() {
        let root = TempRoot::new("workers_stay_between_1_and_the_cores");
        let path = root.paths().settings();
        fs::create_dir_all(root.paths().config).unwrap();
        for (written, loaded) in [("0", 1), ("999", 8), ("-4", 1), ("8", 8), ("2", 2)] {
            fs::write(
                &path,
                format!("format = 1\noutput_folder = 'E:\\Clips'\nworkers = {written}\n"),
            )
            .unwrap();
            assert_eq!(
                load_settings(&path, 8),
                (
                    Settings {
                        output_folder: PathBuf::from(r"E:\Clips"),
                        workers: loaded
                    },
                    None
                ),
                "workers = {written}"
            );
        }
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let root = TempRoot::new("unknown_keys_are_ignored");
        let path = root.paths().settings();
        fs::create_dir_all(root.paths().config).unwrap();
        fs::write(
            &path,
            "format = 1\ntheme = 'dark'\noutput_folder = 'E:\\Clips'\nworkers = 4\n[window]\nwidth = 1440\n",
        )
        .unwrap();
        assert_eq!(
            load_settings(&path, 8),
            (
                Settings {
                    output_folder: PathBuf::from(r"E:\Clips"),
                    workers: 4
                },
                None
            )
        );
        assert_eq!(files_in(&root.paths().config), vec!["settings.toml"]);
    }
}
