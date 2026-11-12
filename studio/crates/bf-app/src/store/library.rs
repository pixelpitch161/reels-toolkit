//! The Library: the videos already made under the output folder, their cover thumbnails, and whether the output
//! folder can take a new video.

use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use chrono::TimeZone as _;

use super::{pause_before_retry, retry_busy};
use crate::model::library::{FolderState, LibraryEntry};
use crate::model::take::Frame;

/// The file that makes a folder a video folder.
const COVER: &str = "cover.png";

/// A cover's size, the thumbnail's size, and the side of the block of cover pixels each thumbnail pixel is made of.
const COVER_W: usize = 1080;
const COVER_H: usize = 1920;
const THUMB_W: usize = 180;
const THUMB_H: usize = 320;
const BLOCK: usize = 6;

/// The largest fight file read: a real one is a few kilobytes.
const MAX_FIGHT_FILE: u64 = 1 << 20;

/// The colours of a video whose fight file could not be read: two greys, since colour belongs to fighters.
const NO_COLORS: [[u8; 3]; 2] = [[0x80, 0x80, 0x80]; 2];

/// How many probe names `folder_state` tries before it gives up.
const PROBE_TRIES: u32 = 64;

const FIGHT_FILE_MISSING: &str = "The fight file is missing, so this video cannot be reopened.";
const TWO_FIGHT_FILES: &str = "This folder holds more than one fight file, so the app cannot tell which one to reopen.";
const FIGHT_FILE_UNREADABLE: &str = "The fight file could not be read, so this video cannot be reopened.";
const FOLDER_UNREADABLE: &str = "This video's folder could not be read, so it cannot be reopened.";
const NOT_A_PNG: &str = "The cover could not be read as a PNG picture.";
const NO_DATE: &str = "Date unknown";

/// What a video's fight file says about it, as the engine reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FightSummary {
    /// "Flute vs Trombone".
    pub title: String,
    /// The two fighters' colours, left then right.
    pub colors: [[u8; 3]; 2],
    pub seed: Option<u64>,
}

/// One entry for each video under `root`, newest first (by the cover's modified time, then by folder).
///
/// A video is a direct subfolder whose name does not start with "." and that holds a file `cover.png`. Its fight file
/// is the one `.toml` file in it (names starting with "." are not counted), given to `read` as text. No fight file,
/// more than one, a fight file that cannot be read as text, a folder that cannot be listed, or an `Err` from `read`
/// each give the entry a problem in words; the title then falls back to the folder's name. A folder name that is not
/// valid Unicode is shown with its invalid parts replaced.
///
/// Links are never followed, so no entry's path leads outside the output folder (later steps delete and reopen
/// through these paths): a subfolder that is a link or junction is not listed, and a `cover.png` or fight file that
/// is a link counts as missing.
///
/// The errors are about `root` itself: missing (or a file in its place), "The output folder {path} is missing.";
/// there but impossible to list, "The output folder {path} could not be read. {plain reason}". Anything inside it
/// that cannot be read is skipped or becomes that video's problem, so one bad video folder never hides the others.
pub fn scan(root: &Path, read: &dyn Fn(&str) -> Result<FightSummary, String>) -> Result<Vec<LibraryEntry>, String> {
    let missing = || Err(format!("The output folder {} is missing.", root.display()));
    let unreadable = |err: &io::Error| {
        Err(format!(
            "The output folder {} could not be read. {}",
            root.display(),
            unreadable_reason(err)
        ))
    };
    match fs::metadata(root) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => return missing(),
        Err(err) if err.kind() == io::ErrorKind::NotFound => return missing(),
        Err(err) => return unreadable(&err),
    }
    let dir = match fs::read_dir(root) {
        Ok(dir) => dir,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return missing(),
        Err(err) => return unreadable(&err),
    };
    let mut entries: Vec<LibraryEntry> = dir
        .filter_map(Result::ok)
        // A `DirEntry`'s file type does not follow links: a link or junction is not a folder here.
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| video(entry.path(), &entry.file_name().to_string_lossy(), read))
        .collect();
    entries.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.folder.cmp(&b.folder)));
    Ok(entries)
}

/// Why the output folder could not be listed, in words the owner can act on.
fn unreadable_reason(err: &io::Error) -> &'static str {
    if err.kind() == io::ErrorKind::PermissionDenied {
        "Your Windows account is not allowed to open it. Choose another folder, or ask for access to it."
    } else {
        "Check that its drive is connected, or choose another folder."
    }
}

/// The entry for the subfolder `folder` (not a link) called `name`, or None when it is not a video.
fn video(folder: PathBuf, name: &str, read: &dyn Fn(&str) -> Result<FightSummary, String>) -> Option<LibraryEntry> {
    if name.starts_with('.') {
        return None;
    }
    // `symlink_metadata` does not follow a link: a linked cover is not a file.
    let cover = fs::symlink_metadata(folder.join(COVER))
        .ok()
        .filter(fs::Metadata::is_file)?;
    let modified = cover
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_secs());
    let mut entry = LibraryEntry {
        folder,
        title: name.to_string(),
        colors: NO_COLORS,
        seed: None,
        modified,
        fight_path: None,
        problem: None,
    };
    let problem = match fight_files(&entry.folder).as_deref() {
        None => FOLDER_UNREADABLE.to_string(),
        Some([]) => FIGHT_FILE_MISSING.to_string(),
        Some([path]) => {
            entry.fight_path = Some(path.clone());
            match read_text(path)
                .ok_or_else(|| FIGHT_FILE_UNREADABLE.to_string())
                .and_then(|text| read(&text))
            {
                Ok(summary) => {
                    if !summary.title.trim().is_empty() {
                        entry.title = summary.title;
                    }
                    entry.colors = summary.colors;
                    entry.seed = summary.seed;
                    return Some(entry);
                }
                Err(problem) => problem,
            }
        }
        Some(_) => TWO_FIGHT_FILES.to_string(),
    };
    entry.problem = Some(problem);
    Some(entry)
}

/// The `.toml` files in `folder` (any letter case; not names starting with "."; plain files only, never a link), or
/// None if it cannot be listed.
fn fight_files(folder: &Path) -> Option<Vec<PathBuf>> {
    let files = fs::read_dir(folder)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            !name.starts_with('.') && name.ends_with(".toml")
        })
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.path())
        .collect();
    Some(files)
}

/// A fight file's text (without a leading byte-order mark), or None if it cannot be opened, is not UTF-8 or is over
/// `MAX_FIGHT_FILE`.
fn read_text(path: &Path) -> Option<String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .ok()?
        .take(MAX_FIGHT_FILE + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_FIGHT_FILE {
        return None;
    }
    let text = String::from_utf8(bytes).ok()?;
    Some(text.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(text))
}

/// A 1080 x 1920 cover as a 180 x 320 thumbnail: each pixel the mean of a 6 x 6 block, rounded to the nearest.
///
/// The size is checked from the header before any pixels are read. Every PNG colour type is read: palette, grey and
/// 16-bit pictures are first turned into 8-bit colour or grey (a grey pixel gives equal red, green and blue), and
/// any alpha is ignored. An interlaced cover is read whole. Anything that is not a PNG, or is cut short, is an error
/// in words.
pub fn thumbnail(cover_png: &[u8]) -> Result<Frame, String> {
    let not_a_png = |_| NOT_A_PNG.to_string();
    let mut decoder = png::Decoder::new(io::Cursor::new(cover_png));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let (w, h) = {
        let info = decoder.read_header_info().map_err(not_a_png)?;
        (info.width, info.height)
    };
    if (w as usize, h as usize) != (COVER_W, COVER_H) {
        return Err(format!("The cover is {w} x {h}, not {COVER_W} x {COVER_H}."));
    }
    let mut reader = decoder.read_info().map_err(not_a_png)?;
    let channels = match reader.output_color_type() {
        (png::ColorType::Grayscale, png::BitDepth::Eight) => 1,
        (png::ColorType::GrayscaleAlpha, png::BitDepth::Eight) => 2,
        (png::ColorType::Rgb, png::BitDepth::Eight) => 3,
        (png::ColorType::Rgba, png::BitDepth::Eight) => 4,
        _ => return Err(NOT_A_PNG.to_string()),
    };
    let size = reader.output_buffer_size().ok_or_else(|| NOT_A_PNG.to_string())?;
    let mut pixels = vec![0u8; size];
    let out = reader.next_frame(&mut pixels).map_err(not_a_png)?;
    if (out.width as usize, out.height as usize) != (COVER_W, COVER_H) || out.line_size < COVER_W * channels {
        return Err(NOT_A_PNG.to_string());
    }
    let stride = out.line_size;
    let block = (BLOCK * BLOCK) as u32;
    let mut rgb = Vec::with_capacity(THUMB_W * THUMB_H * 3);
    for ty in 0..THUMB_H {
        for tx in 0..THUMB_W {
            let mut sum = [0u32; 3];
            for dy in 0..BLOCK {
                let row = (ty * BLOCK + dy) * stride;
                for dx in 0..BLOCK {
                    let px = &pixels[row + (tx * BLOCK + dx) * channels..][..channels];
                    for (c, total) in sum.iter_mut().enumerate() {
                        *total += u32::from(px[if channels >= 3 { c } else { 0 }]);
                    }
                }
            }
            rgb.extend(sum.map(|total| ((total + block / 2) / block) as u8));
        }
    }
    Ok(Frame {
        w: THUMB_W as u32,
        h: THUMB_H as u32,
        rgb: rgb.into(),
    })
}

/// Whether the output folder can take a video called `name`: `Missing` when `root` is not a folder; `NotWritable`
/// with the reason in words when a probe file cannot be created and deleted in it; else `Ok`, with `taken` when
/// `<root>/<name>` exists (false for an empty name).
pub fn folder_state(root: &Path, name: &str) -> FolderState {
    match fs::metadata(root) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => return FolderState::Missing,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return FolderState::Missing,
        Err(_) => {
            return FolderState::NotWritable(format!(
                "The output folder {} could not be checked. Check that its drive is connected, or choose another \
                 folder.",
                root.display()
            ));
        }
    }
    if let Err(reason) = probe(root) {
        return FolderState::NotWritable(reason);
    }
    // Anything there, even a broken link, takes the name; so does a name that cannot be checked.
    let taken = !name.is_empty()
        && fs::symlink_metadata(root.join(name)).map_or_else(|err| err.kind() != io::ErrorKind::NotFound, |_| true);
    FolderState::Ok { taken }
}

/// The probe file's name for try `n`: starts with ".", so `scan` never takes it for a video. It names this process
/// and this thread, so two checks running at once never try the same name: on Windows a name whose file is still
/// being deleted (an antivirus scan may hold it a moment) cannot be made again, and the check would then wrongly
/// say the folder cannot be written to.
fn probe_path(root: &Path, n: u32) -> PathBuf {
    root.join(format!(
        ".bf-write-check-{}-{}-{n}.tmp",
        std::process::id(),
        PROBE_THREAD.with(|tag| *tag)
    ))
}

/// The next thread's tag in probe names.
static NEXT_PROBE_THREAD: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

thread_local! {
    /// This thread's tag in probe names: the same for every check on it, never another thread's.
    static PROBE_THREAD: u64 = NEXT_PROBE_THREAD.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
thread_local! {
    /// Tests only: while set, the probe on this thread acts as if its file could not be deleted (and leaves it).
    static PROBE_DELETE_FAILS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Said when the probe file at `path` could not be deleted.
fn left_behind(root: &Path, path: &Path) -> String {
    format!(
        "The output folder {} let the app make a test file but not delete it. Delete {} yourself, or choose another \
         folder.",
        root.display(),
        path.display()
    )
}

/// Creates a probe file of its own in `root` and deletes it at once. A name already taken (by anyone) is skipped,
/// never written over or deleted. Deleting is tried again for a moment if another program (an antivirus scan) holds
/// the new file; if it still cannot be deleted, says so and names the file left behind.
fn probe(root: &Path) -> Result<(), String> {
    let cannot_write = |err: Option<&io::Error>| {
        if err.is_some_and(|err| err.kind() == io::ErrorKind::StorageFull) {
            format!(
                "The drive that holds the output folder {} is full. Free some space, or choose another folder.",
                root.display()
            )
        } else {
            format!(
                "The output folder {} cannot be written to. Check that it is not read-only and that the drive has \
                 free space, or choose another folder.",
                root.display()
            )
        }
    };
    for n in 0..PROBE_TRIES {
        let path = probe_path(root, n);
        match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                drop(file);
                #[cfg(test)]
                if PROBE_DELETE_FAILS.with(std::cell::Cell::get) {
                    return Err(left_behind(root, &path));
                }
                return match retry_busy(|| fs::remove_file(&path), &mut pause_before_retry) {
                    Ok(()) => Ok(()),
                    Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
                    Err(_) => Err(left_behind(root, &path)),
                };
            }
            // A name already taken is skipped. On Windows a name whose file is still being deleted (or that is a
            // folder) answers "access denied" instead: a clash too, so the next name is tried (a folder that
            // truly cannot be written refuses every name, and the tries are bounded).
            Err(err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::AlreadyExists | io::ErrorKind::PermissionDenied
                ) =>
            {
                continue;
            }
            Err(err) => return Err(cannot_write(Some(&err))),
        }
    }
    Err(cannot_write(None))
}

/// "Nov 10, 2026": the day `secs` (seconds since 1970, UTC) falls on in local time, or "Date unknown" for a time
/// the calendar cannot show.
pub fn local_date(secs: u64) -> String {
    i64::try_from(secs)
        .ok()
        .and_then(|secs| chrono::Local.timestamp_opt(secs, 0).earliest())
        .map_or_else(|| NO_DATE.to_string(), |time| time.format("%b %-d, %Y").to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    use super::super::{TempRoot, files_in};
    use super::*;

    const COLORS: [[u8; 3]; 2] = [[0xde, 0xe8, 0xf6], [0x78, 0xf0, 0xb8]];

    /// Reads a fight file whose text is its title.
    fn read_title(text: &str) -> Result<FightSummary, String> {
        Ok(FightSummary {
            title: text.trim().to_string(),
            colors: COLORS,
            seed: Some(text.len() as u64),
        })
    }

    fn set_modified(path: &Path, secs: u64) {
        let file = fs::OpenOptions::new().write(true).open(path).unwrap();
        file.set_modified(UNIX_EPOCH + Duration::from_secs(secs)).unwrap();
    }

    /// A video folder under `root`: a cover dated `secs`, and the fight files given as (name, text).
    fn video(root: &Path, name: &str, fights: &[(&str, &[u8])], secs: u64) -> PathBuf {
        let folder = root.join(name);
        fs::create_dir_all(&folder).unwrap();
        let cover = folder.join("cover.png");
        fs::write(&cover, b"a cover").unwrap();
        set_modified(&cover, secs);
        for (file, text) in fights {
            fs::write(folder.join(file), text).unwrap();
        }
        folder
    }

    /// A PNG made in the test with the `png` crate.
    fn png(
        w: u32,
        h: u32,
        color: png::ColorType,
        depth: png::BitDepth,
        pixel: impl Fn(u32, u32) -> Vec<u8>,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        for y in 0..h {
            for x in 0..w {
                data.extend(pixel(x, y));
            }
        }
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, w, h);
        encoder.set_color(color);
        encoder.set_depth(depth);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&data).unwrap();
        writer.finish().unwrap();
        bytes
    }

    /// The CRC-32 a PNG chunk ends with, so a test can change a header and keep the file valid.
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &byte in bytes {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    fn rgb_at(frame: &Frame, x: u32, y: u32) -> [u8; 3] {
        let at = ((y * frame.w + x) * 3) as usize;
        [frame.rgb[at], frame.rgb[at + 1], frame.rgb[at + 2]]
    }

    /// Windows only: denies everyone (the well-known SID `S-1-1-0`, whatever the system's language) some rights on a
    /// folder, and gives them back when dropped (before the test's temp folder is removed). A deny for everyone binds
    /// an administrator too (a deny for the user's name alone did not on a hosted runner's administrator account).
    /// Needs no admin rights: the owner of a folder may always change who can use it, so the rights always come back.
    #[cfg(windows)]
    struct Denied {
        path: PathBuf,
        user: String,
    }

    #[cfg(windows)]
    impl Denied {
        fn new(path: &Path, rights: &str) -> Denied {
            let user = "*S-1-1-0".to_string();
            let out = std::process::Command::new("icacls")
                .arg(path)
                .arg("/deny")
                .arg(format!("{user}:({rights})"))
                .output()
                .unwrap();
            assert!(out.status.success(), "{out:?}");
            Denied {
                path: path.to_path_buf(),
                user,
            }
        }
    }

    #[cfg(windows)]
    impl Drop for Denied {
        fn drop(&mut self) {
            let out = std::process::Command::new("icacls")
                .arg(&self.path)
                .arg("/remove:d")
                .arg(&self.user)
                .output();
            // A folder left denied cannot be removed by the test: say so where it shows.
            match out {
                Ok(out) if out.status.success() => {}
                Ok(out) => eprintln!(
                    "could not give back the rights on {}: {}{}",
                    self.path.display(),
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                ),
                Err(err) => eprintln!("could not run icacls for {}: {err}", self.path.display()),
            }
        }
    }

    /// Whether the deny on `path` holds for this account. An account that can still list a folder everyone was
    /// denied (one that reads past permissions, as an administrator with backup rights can) cannot show the case:
    /// the test says so in its output and checks nothing more.
    #[cfg(windows)]
    fn denied_binds(path: &Path) -> bool {
        let binds = fs::read_dir(path).is_err();
        if !binds {
            eprintln!(
                "not run here: this account can read a folder it was denied (an administrator account): {}",
                path.display()
            );
        }
        binds
    }

    /// Windows only: a junction at `link` pointing at the folder `target` (no admin rights needed).
    #[cfg(windows)]
    fn junction(link: &Path, target: &Path) {
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap();
        assert!(made.status.success(), "{made:?}");
    }

    /// A raw Adam7-interlaced 8-bit RGB PNG, written by hand (the `png` crate does not write interlaced pictures):
    /// unfiltered rows in stored (uncompressed) deflate blocks.
    fn interlaced_png(w: u32, h: u32, pixel: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
        const PASSES: [(u32, u32, u32, u32); 7] = [
            (0, 0, 8, 8),
            (4, 0, 8, 8),
            (0, 4, 4, 8),
            (2, 0, 4, 4),
            (0, 2, 2, 4),
            (1, 0, 2, 2),
            (0, 1, 1, 2),
        ];
        let mut raw = Vec::new();
        for (x0, y0, dx, dy) in PASSES {
            for y in (y0..h).step_by(dy as usize) {
                if x0 >= w {
                    continue;
                }
                raw.push(0);
                for x in (x0..w).step_by(dx as usize) {
                    raw.extend(pixel(x, y));
                }
            }
        }
        let mut zlib = vec![0x78, 0x01];
        let chunks: Vec<&[u8]> = raw.chunks(65_535).collect();
        for (i, chunk) in chunks.iter().enumerate() {
            zlib.push(u8::from(i + 1 == chunks.len()));
            let len = chunk.len() as u16;
            zlib.extend(len.to_le_bytes());
            zlib.extend((!len).to_le_bytes());
            zlib.extend(*chunk);
        }
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in &raw {
            a = (a + u32::from(byte)) % 65_521;
            b = (b + a) % 65_521;
        }
        zlib.extend(((b << 16) | a).to_be_bytes());
        let mut file = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let mut chunk = |kind: &[u8; 4], data: &[u8]| {
            file.extend((data.len() as u32).to_be_bytes());
            let mut body = kind.to_vec();
            body.extend(data);
            file.extend(&body);
            file.extend(crc32(&body).to_be_bytes());
        };
        let mut header = Vec::new();
        header.extend(w.to_be_bytes());
        header.extend(h.to_be_bytes());
        header.extend([8, 2, 0, 0, 1]);
        chunk(b"IHDR", &header);
        chunk(b"IDAT", &zlib);
        chunk(b"IEND", &[]);
        file
    }

    #[test]
    fn scan_finds_videos_newest_first() {
        let root = TempRoot::new("scan_finds_videos_newest_first");
        let older = video(
            &root.0,
            "tuba_vs_bass_drum",
            &[("tuba_vs_bass_drum.toml", b"Tuba vs Bass drum")],
            1_794_225_600,
        );
        let newer = video(
            &root.0,
            "flute_vs_trombone",
            &[("flute_vs_trombone.toml", b"Flute vs Trombone\n")],
            1_794_312_000,
        );
        // A folder still being written, and one without a cover.
        video(&root.0, ".tmp-x", &[("x.toml", b"X vs Y")], 1_794_400_000);
        fs::create_dir(root.0.join("no cover")).unwrap();
        fs::write(root.0.join("no cover").join("no_cover.toml"), b"No vs Cover").unwrap();
        // A stray file at the top.
        fs::write(root.0.join("notes.txt"), b"mine").unwrap();
        let entries = scan(&root.0, &read_title).unwrap();
        assert_eq!(
            entries,
            vec![
                LibraryEntry {
                    folder: newer.clone(),
                    title: "Flute vs Trombone".to_string(),
                    colors: COLORS,
                    seed: Some(18),
                    modified: 1_794_312_000,
                    fight_path: Some(newer.join("flute_vs_trombone.toml")),
                    problem: None,
                },
                LibraryEntry {
                    folder: older.clone(),
                    title: "Tuba vs Bass drum".to_string(),
                    colors: COLORS,
                    seed: Some(17),
                    modified: 1_794_225_600,
                    fight_path: Some(older.join("tuba_vs_bass_drum.toml")),
                    problem: None,
                },
            ]
        );
    }

    /// A finished video whose name was taken while it was made is saved as `<name>_kept-<k>` (bf-export's
    /// `output.rs`), holding the files named after `<name>`: the Library lists it as a normal video, which reopens.
    #[test]
    fn a_kept_video_shows_as_a_normal_video() {
        let root = TempRoot::new("a_kept_video_shows_as_a_normal_video");
        let first = video(
            &root.0,
            "flute_vs_trombone",
            &[("flute_vs_trombone.toml", b"Flute vs Trombone")],
            1_794_000_000,
        );
        let kept = video(
            &root.0,
            "flute_vs_trombone_kept-1",
            &[
                ("flute_vs_trombone.toml", b"Flute vs Trombone again"),
                ("flute_vs_trombone.mp4", b"video"),
                ("caption.txt", b"caption"),
            ],
            1_794_000_100,
        );
        let entries = scan(&root.0, &read_title).unwrap();
        assert_eq!(
            entries,
            vec![
                LibraryEntry {
                    folder: kept.clone(),
                    title: "Flute vs Trombone again".to_string(),
                    colors: COLORS,
                    seed: Some(23),
                    modified: 1_794_000_100,
                    fight_path: Some(kept.join("flute_vs_trombone.toml")),
                    problem: None,
                },
                LibraryEntry {
                    folder: first.clone(),
                    title: "Flute vs Trombone".to_string(),
                    colors: COLORS,
                    seed: Some(17),
                    modified: 1_794_000_000,
                    fight_path: Some(first.join("flute_vs_trombone.toml")),
                    problem: None,
                },
            ]
        );
    }

    #[test]
    fn a_folder_without_a_fight_file_cannot_be_reopened() {
        let root = TempRoot::new("a_folder_without_a_fight_file_cannot_be_reopened");
        let folder = video(&root.0, "oboe_vs_bassoon", &[], 1_794_000_000);
        let entries = scan(&root.0, &read_title).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].folder, folder);
        assert_eq!(entries[0].title, "oboe_vs_bassoon");
        assert_eq!(entries[0].seed, None);
        assert_eq!(entries[0].fight_path, None);
        assert_eq!(entries[0].modified, 1_794_000_000);
        assert_eq!(
            entries[0].problem.as_deref(),
            Some("The fight file is missing, so this video cannot be reopened.")
        );
    }

    #[test]
    fn an_unreadable_fight_file_is_named() {
        let root = TempRoot::new("an_unreadable_fight_file_is_named");
        let folder = video(
            &root.0,
            "alto_sax_vs_tenor_sax",
            &[("alto_sax_vs_tenor_sax.toml", b"format = 2")],
            1_794_000_000,
        );
        let read = |_: &str| Err("this app reads fight files up to format 1".to_string());
        let entries = scan(&root.0, &read).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "alto_sax_vs_tenor_sax");
        assert_eq!(entries[0].fight_path, Some(folder.join("alto_sax_vs_tenor_sax.toml")));
        assert_eq!(
            entries[0].problem.as_deref(),
            Some("this app reads fight files up to format 1")
        );
    }

    #[test]
    fn two_fight_files_cannot_be_reopened() {
        let root = TempRoot::new("two_fight_files_cannot_be_reopened");
        video(
            &root.0,
            "cello_vs_contrabass",
            &[
                ("cello_vs_contrabass.toml", b"Cello vs Contrabass"),
                ("copy.TOML", b"Copy"),
            ],
            1_794_000_000,
        );
        let entries = scan(&root.0, &read_title).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "cello_vs_contrabass");
        assert_eq!(entries[0].fight_path, None);
        assert_eq!(entries[0].problem.as_deref(), Some(TWO_FIGHT_FILES));
    }

    #[test]
    fn odd_folders_never_stop_the_scan() {
        let root = TempRoot::new("odd_folders_never_stop_the_scan");
        // A cover that is a folder: not a video.
        fs::create_dir_all(root.0.join("cover is a folder").join("cover.png")).unwrap();
        // A folder named like a fight file, and a hidden copy, beside the real one.
        let one = video(
            &root.0,
            "one real fight file",
            &[("trumpet_vs_horn.toml", b"Trumpet vs Horn"), (".backup.toml", b"Old")],
            1_794_000_003,
        );
        fs::create_dir(one.join("folder.toml")).unwrap();
        // A fight file that is not text.
        let binary = video(&root.0, "not text", &[("x.toml", &[0xff, 0xfe, 0x00])], 1_794_000_002);
        // A fight file with no title.
        video(&root.0, "no title", &[("x.toml", b"   ")], 1_794_000_001);
        let entries = scan(&root.0, &read_title).unwrap();
        let titles: Vec<&str> = entries.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, vec!["Trumpet vs Horn", "not text", "no title"]);
        assert_eq!(entries[0].fight_path, Some(one.join("trumpet_vs_horn.toml")));
        assert_eq!(entries[0].problem, None);
        assert_eq!(entries[1].fight_path, Some(binary.join("x.toml")));
        assert_eq!(entries[1].problem.as_deref(), Some(FIGHT_FILE_UNREADABLE));
        assert_eq!(entries[2].problem, None);
    }

    #[cfg(windows)]
    #[test]
    fn a_name_that_is_not_unicode_is_still_listed() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        let root = TempRoot::new("a_name_that_is_not_unicode_is_still_listed");
        // A lone surrogate: a name Windows allows but that is not valid Unicode.
        let name = OsString::from_wide(&[0xd800, u16::from(b'x')]);
        let folder = root.0.join(&name);
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("cover.png"), b"a cover").unwrap();
        let entries = scan(&root.0, &|_| Err("no".to_string())).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].folder, folder);
        assert_eq!(entries[0].title, "\u{fffd}x");
        assert_eq!(entries[0].problem.as_deref(), Some(FIGHT_FILE_MISSING));
    }

    #[cfg(windows)]
    #[test]
    fn a_folder_that_cannot_be_read_is_named() {
        let root = TempRoot::new("a_folder_that_cannot_be_read_is_named");
        video(&root.0, "readable", &[("readable.toml", b"Readable")], 1_794_000_001);
        let locked = video(&root.0, "locked", &[("locked.toml", b"Locked")], 1_794_000_000);
        let _denied = Denied::new(&locked, "RD");
        if !denied_binds(&locked) {
            return;
        }
        let entries = scan(&root.0, &read_title).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title, "Readable");
        assert_eq!(entries[1].folder, locked);
        assert_eq!(entries[1].title, "locked");
        assert_eq!(entries[1].fight_path, None);
        assert_eq!(entries[1].problem.as_deref(), Some(FOLDER_UNREADABLE));
    }

    #[cfg(windows)]
    #[test]
    fn links_never_lead_out_of_the_output_folder() {
        let root = TempRoot::new("links_never_lead_out_of_the_output_folder");
        let videos = root.0.join("videos");
        fs::create_dir(&videos).unwrap();
        // A video folder outside the output folder, reached through a junction inside it: not listed.
        let outside = video(&root.0, "outside", &[("outside.toml", b"Outside")], 1_794_000_000);
        junction(&videos.join("linked video"), &outside);
        // A real video folder whose cover and fight file are junctions to folders outside: they count as missing.
        let real = videos.join("real");
        fs::create_dir(&real).unwrap();
        junction(&real.join("cover.png"), &outside);
        let inner = video(&videos, "inner", &[], 1_794_000_001);
        junction(&inner.join("inner.toml"), &outside);
        let entries = scan(&videos, &read_title).unwrap();
        assert_eq!(entries.len(), 1, "{entries:?}");
        assert_eq!(entries[0].folder, inner);
        assert_eq!(entries[0].fight_path, None);
        assert_eq!(entries[0].problem.as_deref(), Some(FIGHT_FILE_MISSING));
        // Links to files need rights a plain account may not have; when this PC allows them, they count as missing.
        let file_link = inner.join("file link.toml");
        match std::os::windows::fs::symlink_file(outside.join("outside.toml"), &file_link) {
            Ok(()) => {
                let entries = scan(&videos, &read_title).unwrap();
                assert_eq!(entries[0].problem.as_deref(), Some(FIGHT_FILE_MISSING));
            }
            Err(err) => eprintln!("file links are not allowed here ({err}); that part was not run"),
        }
        assert_eq!(fs::read(outside.join("outside.toml")).unwrap(), b"Outside");
    }

    #[cfg(windows)]
    #[test]
    fn an_output_folder_that_cannot_be_read_is_named() {
        let root = TempRoot::new("an_output_folder_that_cannot_be_read_is_named");
        let videos = root.0.join("videos");
        video(
            &videos,
            "flute_vs_trombone",
            &[("f.toml", b"Flute vs Trombone")],
            1_794_000_000,
        );
        let denied = Denied::new(&videos, "RD");
        if !denied_binds(&videos) {
            return;
        }
        let result = scan(&videos, &read_title);
        drop(denied);
        let text = result.unwrap_err();
        assert!(text.starts_with("The output folder"), "{text}");
        assert!(text.contains(&videos.display().to_string()), "{text}");
        assert_eq!(
            text,
            format!(
                "The output folder {} could not be read. Your Windows account is not allowed to open it. Choose \
                 another folder, or ask for access to it.",
                videos.display()
            )
        );
        // Once it can be read again, its videos are back.
        assert_eq!(scan(&videos, &read_title).unwrap().len(), 1);
    }

    #[test]
    fn a_missing_output_folder_is_named() {
        let root = TempRoot::new("a_missing_output_folder_is_named");
        let missing = root.0.join("Vidéos").join("BALLFIGHT");
        assert_eq!(
            scan(&missing, &read_title),
            Err(format!("The output folder {} is missing.", missing.display()))
        );
        // A file where the folder should be.
        let file = root.0.join("a file");
        fs::write(&file, b"mine").unwrap();
        assert_eq!(
            scan(&file, &read_title),
            Err(format!("The output folder {} is missing.", file.display()))
        );
        assert_eq!(scan(&root.0, &read_title), Ok(Vec::new()));
    }

    #[test]
    fn folder_states() {
        let root = TempRoot::new("folder_states");
        assert_eq!(
            folder_state(&root.0.join("missing"), "flute_vs_trombone"),
            FolderState::Missing
        );
        fs::write(root.0.join("a file"), b"mine").unwrap();
        assert_eq!(
            folder_state(&root.0.join("a file"), "flute_vs_trombone"),
            FolderState::Missing
        );
        let videos = root.0.join("videos");
        fs::create_dir(&videos).unwrap();
        assert_eq!(
            folder_state(&videos, "flute_vs_trombone"),
            FolderState::Ok { taken: false }
        );
        fs::create_dir(videos.join("flute_vs_trombone")).unwrap();
        assert_eq!(
            folder_state(&videos, "flute_vs_trombone"),
            FolderState::Ok { taken: true }
        );
        assert_eq!(folder_state(&videos, ""), FolderState::Ok { taken: false });
        // The probe file is gone each time.
        assert_eq!(files_in(&videos), vec!["flute_vs_trombone"]);
    }

    #[test]
    fn the_probe_never_touches_a_file_already_there() {
        let root = TempRoot::new("the_probe_never_touches_a_file_already_there");
        let theirs = probe_path(&root.0, 0);
        fs::write(&theirs, b"theirs").unwrap();
        assert_eq!(folder_state(&root.0, "x"), FolderState::Ok { taken: false });
        assert_eq!(fs::read(&theirs).unwrap(), b"theirs");
        assert_eq!(files_in(&root.0).len(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn a_name_that_answers_access_denied_is_skipped() {
        let root = TempRoot::new("a_name_that_answers_access_denied_is_skipped");
        // A folder where the first probe file would go: Windows refuses to create the file with "access denied",
        // as it does for a file still being deleted.
        let blocked = probe_path(&root.0, 0);
        fs::create_dir(&blocked).unwrap();
        let err = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&blocked)
            .unwrap_err();
        assert_eq!(
            err.kind(),
            io::ErrorKind::PermissionDenied,
            "the case this test stands for"
        );
        assert_eq!(folder_state(&root.0, "x"), FolderState::Ok { taken: false });
        assert!(blocked.is_dir(), "what was there is left alone");
        assert_eq!(files_in(&root.0).len(), 1);
    }

    #[test]
    fn checks_at_once_on_several_threads_never_share_a_probe_name() {
        let root = TempRoot::new("checks_at_once_on_several_threads_never_share_a_probe_name");
        let mine = probe_path(&root.0, 0);
        assert_eq!(mine, probe_path(&root.0, 0), "one thread keeps its name");
        let theirs = std::thread::scope(|s| s.spawn(|| probe_path(&root.0, 0)).join().unwrap());
        assert_ne!(mine, theirs);
        // Eight checks at once, many times over: every one finds the folder writable and leaves nothing behind.
        std::thread::scope(|s| {
            for _ in 0..8 {
                s.spawn(|| {
                    for _ in 0..40 {
                        assert_eq!(folder_state(&root.0, "x"), FolderState::Ok { taken: false });
                    }
                });
            }
        });
        assert_eq!(files_in(&root.0), Vec::<String>::new());
    }

    #[cfg(windows)]
    #[test]
    fn a_folder_that_cannot_be_written_is_named() {
        let root = TempRoot::new("a_folder_that_cannot_be_written_is_named");
        let videos = root.0.join("videos");
        fs::create_dir(&videos).unwrap();
        let denied = Denied::new(&videos, "WD,AD");
        let state = folder_state(&videos, "flute_vs_trombone");
        drop(denied);
        let FolderState::NotWritable(reason) = state else {
            panic!("{state:?}");
        };
        assert_eq!(
            reason,
            format!(
                "The output folder {} cannot be written to. Check that it is not read-only and that the drive has \
                 free space, or choose another folder.",
                videos.display()
            )
        );
        assert!(files_in(&videos).is_empty());
    }

    #[test]
    fn thumbnail_is_180x320() {
        let cover = png(1080, 1920, png::ColorType::Rgb, png::BitDepth::Eight, |x, _| {
            if x < 540 { vec![200, 0, 0] } else { vec![0, 0, 200] }
        });
        let frame = thumbnail(&cover).unwrap();
        assert_eq!((frame.w, frame.h), (180, 320));
        assert_eq!(frame.rgb.len(), 180 * 320 * 3);
        assert_eq!(rgb_at(&frame, 0, 0), [200, 0, 0]);
        assert_eq!(rgb_at(&frame, 89, 160), [200, 0, 0]);
        assert_eq!(rgb_at(&frame, 90, 160), [0, 0, 200]);
        assert_eq!(rgb_at(&frame, 179, 319), [0, 0, 200]);
    }

    #[test]
    fn each_thumbnail_pixel_is_the_mean_of_its_block() {
        // Within each 6 x 6 block the red runs 0 to 35 (mean 17.5, rounded to 18), the green is the block's column.
        let cover = png(1080, 1920, png::ColorType::Rgba, png::BitDepth::Eight, |x, y| {
            vec![((y % 6) * 6 + x % 6) as u8, (x / 6) as u8, 7, 0]
        });
        let frame = thumbnail(&cover).unwrap();
        assert_eq!(rgb_at(&frame, 0, 0), [18, 0, 7]);
        assert_eq!(rgb_at(&frame, 179, 319), [18, 179, 7]);
    }

    #[test]
    fn grey_and_16_bit_covers_are_read_too() {
        let grey = png(1080, 1920, png::ColorType::Grayscale, png::BitDepth::Eight, |_, y| {
            vec![if y < 960 { 10 } else { 250 }]
        });
        let frame = thumbnail(&grey).unwrap();
        assert_eq!(rgb_at(&frame, 0, 0), [10, 10, 10]);
        assert_eq!(rgb_at(&frame, 0, 319), [250, 250, 250]);
        let deep = png(1080, 1920, png::ColorType::Rgb, png::BitDepth::Sixteen, |_, _| {
            vec![200, 0, 0, 0, 100, 0]
        });
        assert_eq!(rgb_at(&thumbnail(&deep).unwrap(), 5, 5), [200, 0, 100]);
    }

    #[test]
    fn a_palette_cover_is_read() {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, 1080, 1920);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_palette(vec![200, 0, 0, 0, 0, 200]);
        let indices: Vec<u8> = (0..1920 * 1080).map(|i| u8::from(i % 1080 >= 540)).collect();
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&indices).unwrap();
        writer.finish().unwrap();
        let frame = thumbnail(&bytes).unwrap();
        assert_eq!(rgb_at(&frame, 0, 0), [200, 0, 0]);
        assert_eq!(rgb_at(&frame, 179, 319), [0, 0, 200]);
    }

    #[test]
    fn an_interlaced_cover_is_read_whole() {
        let cover = interlaced_png(1080, 1920, |x, y| {
            [((y % 6) * 6 + x % 6) as u8, (x / 6) as u8, ((y / 6) % 256) as u8]
        });
        // The hand-made file really is interlaced.
        assert_eq!(cover[28], 1);
        let frame = thumbnail(&cover).unwrap();
        for (tx, ty) in [(0, 0), (1, 1), (7, 3), (90, 200), (179, 319)] {
            assert_eq!(rgb_at(&frame, tx, ty), [18, tx as u8, (ty % 256) as u8], "({tx}, {ty})");
        }
    }

    #[test]
    fn a_probe_file_that_cannot_be_deleted_is_named() {
        let root = TempRoot::new("a_probe_file_that_cannot_be_deleted_is_named");
        PROBE_DELETE_FAILS.with(|fails| fails.set(true));
        let state = folder_state(&root.0, "x");
        PROBE_DELETE_FAILS.with(|fails| fails.set(false));
        let left = probe_path(&root.0, 0);
        assert_eq!(
            state,
            FolderState::NotWritable(format!(
                "The output folder {} let the app make a test file but not delete it. Delete {} yourself, or choose \
                 another folder.",
                root.0.display(),
                left.display()
            ))
        );
        // The file named is the one left behind, and nothing else is there.
        assert_eq!(
            files_in(&root.0),
            vec![left.file_name().unwrap().to_string_lossy().into_owned()]
        );
    }

    #[test]
    fn a_wrong_sized_cover_is_an_error() {
        let small = png(100, 100, png::ColorType::Rgb, png::BitDepth::Eight, |_, _| {
            vec![1, 2, 3]
        });
        assert_eq!(
            thumbnail(&small),
            Err("The cover is 100 x 100, not 1080 x 1920.".to_string())
        );
        // A header that claims a huge picture is refused before anything is made for it.
        let mut huge = png(1, 1, png::ColorType::Rgb, png::BitDepth::Eight, |_, _| vec![1, 2, 3]);
        huge[16..24].copy_from_slice(&[0, 0, 0x75, 0x30, 0, 0, 0x75, 0x30]);
        let crc = crc32(&huge[12..29]);
        huge[29..33].copy_from_slice(&crc.to_be_bytes());
        assert_eq!(
            thumbnail(&huge),
            Err("The cover is 30000 x 30000, not 1080 x 1920.".to_string())
        );
        assert_eq!(thumbnail(b"not a png"), Err(NOT_A_PNG.to_string()));
        let cover = png(1080, 1920, png::ColorType::Rgb, png::BitDepth::Eight, |_, _| {
            vec![1, 2, 3]
        });
        assert_eq!(thumbnail(&cover[..cover.len() / 2]), Err(NOT_A_PNG.to_string()));
    }

    #[test]
    fn dates_read_naturally() {
        let noon = chrono::Local.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).single().unwrap();
        assert_eq!(local_date(noon.timestamp() as u64), "Nov 10, 2026");
        let noon = chrono::Local.with_ymd_and_hms(2026, 11, 5, 12, 0, 0).single().unwrap();
        assert_eq!(local_date(noon.timestamp() as u64), "Nov 5, 2026");
        // Far beyond any date the calendar can show.
        assert_eq!(local_date(u64::MAX), NO_DATE);
        assert_eq!(local_date(i64::MAX as u64), NO_DATE);
    }
}
