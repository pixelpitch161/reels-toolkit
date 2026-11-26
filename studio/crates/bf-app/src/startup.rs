//! Start-up guards: one copy of the app at a time, and the checks a start makes before the person needs anything
//! (a graphics card that draws with DirectX 12, ffmpeg, the output folder, the app's own data folders).
//!
//! Every check returns what it found, as a kind and in words ([`Found`]); what to show, and whether to go on, is
//! the caller's (the start-up wiring in `main.rs`). A check never panics, never overwrites anything, and writes
//! nothing outside the app's data folders except one probe file of its own, which it removes at once. The only
//! other thing it deletes is an earlier start's leftover probe in a data folder it checks (an empty plain file named
//! `.bf-start-check-<digits>-<digits>.tmp`, see `remove_leftover_probes`). A check
//! that asks the system runs on a thread of its own and is waited for at most a fixed time (the `*_WAIT`
//! constants); a check still running then is answered [`Late::Slow`] and its thread finishes by itself later.
//!
//! The graphics check never makes a second device for the window: [`check_graphics`] only lists the DirectX 12
//! adapters (see [`dx12_adapters`]), and [`window_graphics`] reads the adapter the window itself chose.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use eframe::wgpu;
use sha2::{Digest, Sha256};

use crate::model::library::FolderState;
use crate::store::library::folder_state;

/// What the person reads when the window could not be opened for want of a graphics card (Windows' message box).
pub const NO_GRAPHICS: &str = "BALLFIGHT Studio could not open its window, because Windows did not offer it a \
    graphics card to draw with (it needs DirectX 12). Update the graphics driver from the maker of your graphics \
    card, restart the PC, then start BALLFIGHT Studio again.";

/// The notice for a window drawn by Windows' software drawing (WARP) rather than a graphics card.
pub const SOFTWARE_DRAWING: &str = "This PC draws the window without its graphics card, so the preview may be \
    slow. Updating the graphics driver usually fixes this.";

/// What a second start says when another copy holds the lock but has no window to bring forward (it is still
/// closing, or hung): for when [`bring_forward`] answers false.
pub const ALREADY_OPEN: &str = "BALLFIGHT Studio is already open or still closing. Wait a moment, then start it again.";

/// The longest [`check_graphics`] waits for Windows to list its DirectX 12 adapters.
pub const GRAPHICS_WAIT: Duration = Duration::from_secs(10);
/// The longest [`check_ffmpeg`] waits for the ffmpeg search (which asks ffmpeg for its version and its encoders).
pub const FFMPEG_WAIT: Duration = Duration::from_secs(10);
/// The longest [`check_output_folder`] and [`check_data_folder`] wait for a folder's drive to answer.
pub const FOLDER_WAIT: Duration = Duration::from_secs(5);

/// The window class winit 0.30 gives every window (eframe names none), so [`bring_forward`] never picks a File
/// Explorer window that shows a folder named "BALLFIGHT Studio".
const WINDOW_CLASS: &str = "Window Class";

/// How many probe names a folder check tries before it gives up (each one already taken is skipped).
const PROBE_TRIES: u32 = 16;

/// What one check found: what kind of answer it is, and the same in words (whole plain sentences).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found<K> {
    pub kind: K,
    pub words: String,
}

impl<K> Found<K> {
    fn new(kind: K, words: impl Into<String>) -> Found<K> {
        Found {
            kind,
            words: words.into(),
        }
    }
}

/// Why a check gave no answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Late {
    /// It was still running when its wait ran out.
    Slow(Duration),
    /// It stopped with a fault (or its thread could not be started).
    Broke,
}

/// "10 seconds": a wait in words.
fn wait_words(wait: Duration) -> String {
    match wait.as_secs() {
        0 => format!("{} milliseconds", wait.as_millis()),
        1 => "1 second".to_string(),
        n => format!("{n} seconds"),
    }
}

/// Runs `work` on a thread named `name` and waits for it at most `limit`. A panic in `work` (or a thread that
/// cannot be started) is [`Late::Broke`]; still running at `limit` is [`Late::Slow`], and the thread is left to end
/// by itself.
fn within<T: Send + 'static>(
    name: &str,
    limit: Duration,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, Late> {
    let (send, answer) = mpsc::sync_channel(1);
    let started = std::thread::Builder::new().name(name.to_string()).spawn(move || {
        // A panic drops `send` without a word, which the wait below reads as a fault.
        let _ = send.send(work());
    });
    if started.is_err() {
        return Err(Late::Broke);
    }
    match answer.recv_timeout(limit) {
        Ok(found) => Ok(found),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(Late::Slow(limit)),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(Late::Broke),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// One copy at a time.

/// Holds the app's named lock until dropped.
pub struct Claim {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
}

// The handle is only ever closed, which any thread may do; the lock is never waited on or owned by a thread.
unsafe impl Send for Claim {}

impl Drop for Claim {
    fn drop(&mut self) {
        #[cfg(windows)]
        if !self.handle.is_null() {
            // SAFETY: the handle came from CreateMutexW and is closed once, here.
            unsafe { windows_sys::Win32::Foundation::CloseHandle(self.handle) };
        }
    }
}

/// Whether this start is the first copy of the app for its settings folder.
pub enum Instance {
    /// The first: keep the claim for as long as the app runs.
    First(Claim),
    /// Another copy with the same settings folder is running.
    Second,
}

/// The lock's name for `config_dir`: "Local\BALLFIGHT Studio " and the first 16 hex digits of the SHA-256 of the
/// folder's absolute path in lower case, without a separator at its end. Nothing on disk is read.
pub fn lock_name(config_dir: &Path) -> String {
    let absolute = std::path::absolute(config_dir).unwrap_or_else(|_| config_dir.to_path_buf());
    let text = absolute.to_string_lossy().to_lowercase();
    let text = text.trim_end_matches(['\\', '/']);
    let digest = Sha256::digest(text.as_bytes());
    let hex: String = digest[..8].iter().map(|byte| format!("{byte:02x}")).collect();
    format!("Local\\BALLFIGHT Studio {hex}")
}

/// Claims the app's named lock for `config_dir` (a Windows named mutex, never owned or waited on: only whether it
/// already existed counts). A lock that cannot be made at all counts as the first copy, so a start is never refused
/// for it. Off Windows every claim is the first.
pub fn claim(config_dir: &Path) -> Instance {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError};
        use windows_sys::Win32::System::Threading::CreateMutexW;
        let name = wide(&lock_name(config_dir));
        // SAFETY: `name` is a NUL-terminated wide string that lives across the call; no security attributes.
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        // SAFETY: read straight after the call it describes.
        let existed = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        if existed && !handle.is_null() {
            // SAFETY: our own handle to the other copy's lock, closed once.
            unsafe { CloseHandle(handle) };
            return Instance::Second;
        }
        Instance::First(Claim { handle })
    }
    #[cfg(not(windows))]
    {
        let _ = config_dir;
        Instance::First(Claim {})
    }
}

/// `text` as a NUL-terminated wide string.
#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Brings the app's window titled `title` to the front, restoring it first when it is minimised (a maximised
/// window stays maximised). False when no such window is found or Windows refuses; the caller then shows
/// [`ALREADY_OPEN`]. Off Windows, false.
///
/// It never waits on the other copy: the restore is posted (`ShowWindowAsync`), so a hung, minimised first copy
/// cannot hang this start; `FindWindowW`, `IsIconic` and `SetForegroundWindow` send the other window no message
/// that has to be answered.
///
/// Accepted: two copies on different data folders (`--data-dir`) share the title, so a second start of one of them
/// may raise the other's window.
pub fn bring_forward(title: &str) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            FindWindowW, IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindowAsync,
        };
        let (class, title) = (wide(WINDOW_CLASS), wide(title));
        // SAFETY: both strings are NUL-terminated and live across the calls; a null window is checked first.
        unsafe {
            let window = FindWindowW(class.as_ptr(), title.as_ptr());
            if window.is_null() {
                return false;
            }
            if IsIconic(window) != 0 {
                ShowWindowAsync(window, SW_RESTORE);
            }
            SetForegroundWindow(window) != 0
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (title, WINDOW_CLASS);
        false
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Graphics.

/// What the graphics check found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Graphics {
    /// A graphics card draws.
    Card,
    /// Only Windows' software drawing.
    Software,
    /// Nothing to draw with through DirectX 12.
    NoCard,
    /// Windows did not answer in time, or the check broke.
    NoAnswer,
    /// Not on Windows.
    NotWindows,
}

/// Whether `info` is a software adapter (drawing on the processor, such as Windows' WARP).
pub fn software_adapter(info: &wgpu::AdapterInfo) -> bool {
    info.device_type == wgpu::DeviceType::Cpu
}

/// The words for a window that could not be opened: [`NO_GRAPHICS`], then where the details were saved, if they
/// were.
pub fn no_window_text(report: Option<&Path>) -> String {
    match report {
        Some(file) => format!("{NO_GRAPHICS} The details were saved in {}.", file.display()),
        None => NO_GRAPHICS.to_string(),
    }
}

/// What the DirectX 12 adapters in `adapters` offer: the first graphics card, else software drawing, else nothing.
pub fn graphics(adapters: &[wgpu::AdapterInfo]) -> Found<Graphics> {
    if let Some(card) = adapters.iter().find(|a| !software_adapter(a)) {
        return Found::new(
            Graphics::Card,
            format!("DirectX 12 can draw with the graphics card {}.", card.name.trim()),
        );
    }
    if adapters.is_empty() {
        Found::new(Graphics::NoCard, NO_GRAPHICS)
    } else {
        Found::new(Graphics::Software, SOFTWARE_DRAWING)
    }
}

/// What the window's own adapter is (asks the system nothing: the window already chose it).
pub fn window_graphics(info: &wgpu::AdapterInfo) -> Found<Graphics> {
    if software_adapter(info) {
        return Found::new(Graphics::Software, SOFTWARE_DRAWING);
    }
    let through = match info.backend {
        wgpu::Backend::Dx12 => "DirectX 12",
        wgpu::Backend::Vulkan => "Vulkan",
        wgpu::Backend::Gl => "OpenGL",
        wgpu::Backend::Metal => "Metal",
        _ => "another graphics interface",
    };
    Found::new(
        Graphics::Card,
        format!(
            "The window draws with the graphics card {} through {through}.",
            info.name.trim()
        ),
    )
}

/// What the graphics check's "no answer" words ask the person to do.
const RESTART: &str = "Restart the PC, then start BALLFIGHT Studio again.";

/// The words for a graphics check that gave no answer.
pub fn graphics_late(late: Late) -> Found<Graphics> {
    let words = match late {
        Late::Slow(wait) => format!(
            "Windows did not say within {} which graphics cards can draw with DirectX 12. {RESTART}",
            wait_words(wait)
        ),
        Late::Broke => {
            format!("The check for a graphics card that draws with DirectX 12 stopped with a fault. {RESTART}")
        }
    };
    Found::new(Graphics::NoAnswer, words)
}

/// Asks Windows for its DirectX 12 adapters ([`dx12_adapters`], on a thread of its own), waiting at most
/// [`GRAPHICS_WAIT`]. Off Windows, [`Graphics::NotWindows`] without asking.
pub fn check_graphics() -> Found<Graphics> {
    if !cfg!(windows) {
        return Found::new(Graphics::NotWindows, "The graphics check runs only on Windows.");
    }
    match within("startup: graphics", GRAPHICS_WAIT, dx12_adapters) {
        Ok(adapters) => graphics(&adapters),
        Err(late) => graphics_late(late),
    }
}

/// The one call that asks the system: a wgpu instance with the DirectX 12 backend alone (no Vulkan or OpenGL, ruling
/// M2-R27), its adapters listed and their information read, then the adapters and the instance dropped before it
/// returns. No adapter is requested and no device is made by this code; wgpu's DirectX 12 backend opens each listed
/// adapter's D3D12 device to read its limits and closes it with the adapter (D3D12 keeps one device per adapter per
/// process, so even a listing still running when the window opens shares the window's device instead of fighting
/// it). Empty off Windows.
pub fn dx12_adapters() -> Vec<wgpu::AdapterInfo> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        // No debug layer or validation, even in a debug build: a listing that ends after the window's device exists
        // must not change how that device was set up.
        flags: wgpu::InstanceFlags::empty(),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    ready(instance.enumerate_adapters(wgpu::Backends::DX12))
        .iter()
        .map(wgpu::Adapter::get_info)
        .collect()
}

/// The output of `future`, which on a native backend is ready when first polled (wgpu lists the adapters before it
/// returns the future). The same as `app`'s own, which is private.
fn ready<F: std::future::Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(out) = future.as_mut().poll(&mut cx) {
            return out;
        }
        std::thread::yield_now();
    }
}

// ---------------------------------------------------------------------------------------------------------------
// ffmpeg.

/// What the ffmpeg check found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ffmpeg {
    /// ffmpeg and ffprobe can be used.
    Ready,
    /// None was found.
    Missing,
    /// One was found but cannot be used (the app's own copy is broken, or another lacks what is needed).
    Broken,
    /// The search did not end in time, or broke.
    NoAnswer,
}

/// What `bf_export`'s search found, in words: the version and where it came from; for a broken copy (the app's own
/// or another) `bf_export`'s own sentence, which names the program and what is wrong; for none at all the words the
/// queue already uses, unless `own` names the app's own ffmpeg folder as expected (see [`expected_own_folder`]).
///
/// When none was found (bf-export's sentence starts "ffmpeg was not found" or "ffprobe was not found"):
/// - `own` is `Some(dir)`: "The app's own ffmpeg is missing from {dir}, so no video can be made; install the app
///   again." (the same sentence bf-export gives when only ffprobe is there);
/// - `own` is `None`: "ffmpeg was not found on this PC, so videos cannot be saved yet. Install ffmpeg, then start the
///   app again." (today's words).
pub fn ffmpeg_found(found: Result<bf_export::Ffmpeg, bf_export::ExportError>, own: Option<&Path>) -> Found<Ffmpeg> {
    match found {
        Ok(f) => {
            let words = match f.source {
                bf_export::FfmpegSource::WithTheApp => format!("{} comes with the app.", f.version),
                bf_export::FfmpegSource::Setting | bf_export::FfmpegSource::Path => {
                    format!("{} was found on this PC.", f.version)
                }
            };
            Found::new(Ffmpeg::Ready, words)
        }
        Err(e) => {
            let said = e.to_string();
            let not_found = said.starts_with("ffmpeg was not found") || said.starts_with("ffprobe was not found");
            if not_found
                && !bf_export::is_about_the_apps_own(&said)
                && let Some(dir) = own
            {
                Found::new(
                    Ffmpeg::Missing,
                    format!(
                        "The app's own ffmpeg is missing from {}, so no video can be made; {}.",
                        dir.display(),
                        bf_export::REINSTALL
                    ),
                )
            } else if not_found && !bf_export::is_about_the_apps_own(&said) {
                Found::new(
                    Ffmpeg::Missing,
                    "ffmpeg was not found on this PC, so videos cannot be saved yet. Install ffmpeg, then start the \
                     app again.",
                )
            } else {
                Found::new(Ffmpeg::Broken, said)
            }
        }
    }
}

/// The words for an ffmpeg check that gave no answer.
pub fn ffmpeg_late(late: Late) -> Found<Ffmpeg> {
    let words = match late {
        Late::Slow(wait) => format!(
            "ffmpeg did not answer within {}, so it is not known yet whether videos can be saved.",
            wait_words(wait)
        ),
        Late::Broke => {
            "The check for ffmpeg stopped with a fault, so it is not known yet whether videos can be saved.".to_string()
        }
    };
    Found::new(Ffmpeg::NoAnswer, words)
}

/// Runs `bf_export::find_ffmpeg` (the whole search: the setting, the app's own copy, the PATH) on a thread of its
/// own, waiting at most [`FFMPEG_WAIT`]. A search still running then goes on by itself: `bf_export` ends and waits for
/// each ffmpeg it asks within 20 seconds, two questions at most.
pub fn check_ffmpeg() -> Found<Ffmpeg> {
    let program_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let own = program_dir.as_deref().and_then(expected_own_folder);
    match within("startup: ffmpeg", FFMPEG_WAIT, bf_export::find_ffmpeg) {
        Ok(found) => ffmpeg_found(found, own.as_deref()),
        Err(late) => ffmpeg_late(late),
    }
}

/// The app's own ffmpeg folder (`bf_export::APP_FFMPEG_DIR` in `program_dir`) when the program is expected to have
/// one: that folder is there (even empty, which bf-export passes over), or `program_dir` holds the package's
/// `MANIFEST.txt` (an installed copy, Task 11). `None` for a developer's build without either.
pub fn expected_own_folder(program_dir: &Path) -> Option<PathBuf> {
    let dir = program_dir.join(bf_export::APP_FFMPEG_DIR);
    let there = |p: &Path| fs::symlink_metadata(p).is_ok();
    (there(&dir) || there(&program_dir.join("MANIFEST.txt"))).then_some(dir)
}

// ---------------------------------------------------------------------------------------------------------------
// Folders.

/// What a folder check found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Folder {
    /// It is there and a file can be written in it.
    Ready,
    /// It is not there (the output folder only: the data folders are made when missing).
    Missing,
    /// A file (or something else that is not a folder) has its name.
    NotAFolder,
    /// It cannot be made or written to (or its drive is full).
    NotWritable,
    /// Its drive did not answer in time, or the check broke.
    NoAnswer,
}

/// What the output folder's state means at start, in words (the queue's own words for a missing folder; the
/// Render check's reason for one that cannot be written).
pub fn output_found(root: &Path, state: FolderState) -> Found<Folder> {
    let at = root.display();
    match state {
        FolderState::Ok { .. } => Found::new(Folder::Ready, format!("The output folder {at} can be written to.")),
        FolderState::Missing => Found::new(
            Folder::Missing,
            format!("The output folder {at} is missing. Create it, or choose another folder."),
        ),
        FolderState::NotWritable(reason) => Found::new(Folder::NotWritable, reason),
        FolderState::Unknown => Found::new(
            Folder::NoAnswer,
            format!(
                "The output folder {at} could not be checked. Check that its drive is connected, or choose another folder."
            ),
        ),
    }
}

/// The words for a folder check that gave no answer; `what` is "output" or "data".
fn folder_late(dir: &Path, late: Late, what: &str) -> Found<Folder> {
    let at = dir.display();
    let (subject, then) = if what == "output" {
        (format!("The output folder {at}"), "or choose another folder")
    } else {
        (format!("The folder {at}"), "then start BALLFIGHT Studio again")
    };
    let words = match late {
        Late::Slow(wait) => format!(
            "{subject} did not answer within {}. Check that its drive is connected, {then}.",
            wait_words(wait)
        ),
        Late::Broke => format!("{subject} could not be checked. Check that its drive is connected, {then}."),
    };
    Found::new(Folder::NoAnswer, words)
}

/// Checks the output folder `root` as the Render screen does (`store::library::folder_state`: the folder is never
/// made here; a probe file of its own is made and removed, a taken name skipped), on a thread of its own, waiting at
/// most [`FOLDER_WAIT`]. A file in the folder's place is named as such.
pub fn check_output_folder(root: &Path) -> Found<Folder> {
    let dir = root.to_path_buf();
    let looked = within("startup: output folder", FOLDER_WAIT, move || {
        if fs::metadata(&dir).is_ok_and(|m| !m.is_dir()) {
            None
        } else {
            Some(folder_state(&dir, ""))
        }
    });
    match looked {
        Ok(Some(state)) => output_found(root, state),
        Ok(None) => Found::new(
            Folder::NotAFolder,
            format!(
                "The output folder {} is a file, not a folder. Choose another folder.",
                root.display()
            ),
        ),
        Err(late) => folder_late(root, late, "output"),
    }
}

/// The probe file's name for try `n`: starts with "." like the Render check's, and names the start.
fn probe_name(n: u32) -> String {
    format!(".bf-start-check-{}-{n}.tmp", std::process::id())
}

/// Said when the probe file at `path` in `dir` could not be removed.
fn left_behind(dir: &Path, path: &Path) -> String {
    format!(
        "BALLFIGHT Studio could make a test file in {} but not delete it. Delete {} yourself, then start BALLFIGHT \
         Studio again.",
        dir.display(),
        path.display()
    )
}

/// Said when `dir` cannot be made or written to (`full`: its drive is full).
fn data_cannot_write(dir: &Path, full: bool) -> String {
    if full {
        format!(
            "BALLFIGHT Studio cannot keep its files in {}, because its drive is full. Free some space, then start \
             BALLFIGHT Studio again.",
            dir.display()
        )
    } else {
        format!(
            "BALLFIGHT Studio cannot keep its files in {}, because that folder cannot be written to. Check that the \
             drive is connected and has free space, then start BALLFIGHT Studio again.",
            dir.display()
        )
    }
}

/// Removes `path`, trying again for a moment while another program (an antivirus scan) holds the new file.
fn remove_patiently(path: &Path) -> io::Result<()> {
    let mut attempt = 0;
    loop {
        match fs::remove_file(path) {
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied && attempt < 5 => {
                std::thread::sleep(Duration::from_millis(25 << attempt));
                attempt += 1;
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
            result => return result,
        }
    }
}

/// Whether `name` is a probe name of some start: ".bf-start-check-", digits, "-", digits, ".tmp".
fn is_probe_name(name: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    name.strip_prefix(".bf-start-check-")
        .and_then(|rest| rest.strip_suffix(".tmp"))
        .and_then(|rest| rest.split_once('-'))
        .is_some_and(|(pid, n)| digits(pid) && digits(n))
}

/// The most entries of a data folder looked at for leftover probes (the app's data folders hold a few dozen).
const LEFTOVER_LOOK: usize = 10_000;

/// Removes the probes earlier starts left in `dir` (one that could not delete its own): only plain files (never a
/// link or a folder) with a probe's name ([`is_probe_name`]) and 0 bytes. Best effort: anything it cannot read or
/// remove is left as it is.
fn remove_leftover_probes(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.take(LEFTOVER_LOOK).flatten() {
        let is_probe = entry.file_name().to_str().is_some_and(is_probe_name);
        // `DirEntry::metadata` does not follow links, so a link is never a plain file here.
        let empty_file = || entry.metadata().is_ok_and(|m| m.is_file() && m.len() == 0);
        if is_probe && empty_file() {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// The data folder check itself: make `dir` when missing, remove leftover probes of earlier starts, then write and
/// remove a probe file of its own.
fn data_folder(dir: &Path) -> Found<Folder> {
    let full = |err: &io::Error| err.kind() == io::ErrorKind::StorageFull;
    if fs::metadata(dir).is_ok_and(|m| !m.is_dir()) {
        return Found::new(
            Folder::NotAFolder,
            format!(
                "BALLFIGHT Studio cannot keep its files in {}, because a file of that name is in the way. Move that \
                 file elsewhere, then start BALLFIGHT Studio again.",
                dir.display()
            ),
        );
    }
    if let Err(err) = fs::create_dir_all(dir) {
        return Found::new(Folder::NotWritable, data_cannot_write(dir, full(&err)));
    }
    remove_leftover_probes(dir);
    for n in 0..PROBE_TRIES {
        let path = dir.join(probe_name(n));
        match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                drop(file);
                return match remove_patiently(&path) {
                    Ok(()) => Found::new(
                        Folder::Ready,
                        format!("BALLFIGHT Studio keeps its files in {}.", dir.display()),
                    ),
                    Err(_) => Found::new(Folder::NotWritable, left_behind(dir, &path)),
                };
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Found::new(Folder::NotWritable, data_cannot_write(dir, full(&err))),
        }
    }
    Found::new(Folder::NotWritable, data_cannot_write(dir, false))
}

/// Checks one of the app's own data folders (settings, cache or logs): made when missing (with its missing parents,
/// all inside the app's data folder), then a probe file of its own made and removed (a taken name skipped), on a
/// thread of its own, waiting at most [`FOLDER_WAIT`].
pub fn check_data_folder(dir: &Path) -> Found<Folder> {
    let owned: PathBuf = dir.to_path_buf();
    match within("startup: data folder", FOLDER_WAIT, move || data_folder(&owned)) {
        Ok(found) => found,
        Err(late) => folder_late(dir, late, "data"),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::store::TempRoot;

    /// Words that keep their capital letter anywhere in a line. A copy of `tests/ui/wording.rs`'s `NAMES` (that list
    /// is private to the UI test program), with "Windows", "DirectX" and "Vulkan" added for these texts. A graphics
    /// card's name is its maker's, not the app's wording: the tests take it out before the check.
    const NAMES: [&str; 25] = [
        "LU",
        "HUD",
        "Update",
        "BALLFIGHT",
        "Studio",
        "K.O.",
        "HP",
        "PC",
        "Instagram",
        "TikTok",
        "Shorts",
        "LV2",
        "LV3",
        "LOW",
        "Match-up",
        "Balance",
        "Seeds",
        "Music",
        "Render",
        "Library",
        "Settings",
        "Queue",
        "Windows",
        "DirectX",
        "Vulkan",
    ];

    /// The capitalised words of `text` that break the sentence-case rule: a copy of `tests/ui/wording.rs`'s `breaks`
    /// (without its note names, fighter names and months, which these texts never hold). After its first word, a
    /// word may start with a capital only after ". " or ": ", when it holds a digit, a backslash or an underscore,
    /// or when it is one of [`NAMES`].
    fn breaks(text: &str) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut bad = Vec::new();
        for pair in words.windows(2) {
            let (before, word) = (pair[0], pair[1]);
            let core = word.trim_start_matches(['"', '\'', '(', '[', '\u{201c}', '\u{2018}']);
            if !core.chars().next().is_some_and(char::is_uppercase) {
                continue;
            }
            if before.ends_with('.') || before.ends_with(':') {
                continue;
            }
            if core.chars().any(|c| c.is_ascii_digit() || c == '\\' || c == '_') {
                continue;
            }
            let bare = core.trim_end_matches([
                ',', '.', ':', ';', ')', ']', '"', '\'', '?', '!', '\u{201d}', '\u{2019}',
            ]);
            let bare = bare.strip_suffix("'s").unwrap_or(bare);
            if NAMES.contains(&core) || NAMES.contains(&bare) {
                continue;
            }
            bad.push(word.to_string());
        }
        bad
    }

    /// Whole plain sentences: sentence case, a capital or "ffmpeg" first, a full stop last, no debug formatting.
    fn reads_plainly(text: &str) {
        assert!(breaks(text).is_empty(), "{text:?} has {:?}", breaks(text));
        assert!(text.ends_with('.'), "{text:?} ends with a full stop");
        let first = text.chars().next().expect("not empty");
        assert!(
            first.is_uppercase() || text.starts_with("ffmpeg "),
            "{text:?} starts a sentence"
        );
        for code in [
            "{", "}", "Some(", "None", "Err(", "Ok(", "::", "Dx12", "Cpu", "Slow(", "Broke",
        ] {
            assert!(!text.contains(code), "{text:?} holds {code:?}");
        }
        assert!(!text.contains("  "), "{text:?} has a double space");
    }

    /// [`reads_plainly`] for a text that names folders under `root`, whose temp name holds capitalised words.
    fn reads_plainly_under(root: &TempRoot, text: &str) {
        reads_plainly(&text.replace(&root.0.display().to_string(), "C:\\root"));
    }

    fn adapter(name: &str, kind: wgpu::DeviceType, backend: wgpu::Backend) -> wgpu::AdapterInfo {
        let mut info = wgpu::AdapterInfo::new(kind, backend);
        info.name = name.to_string();
        info
    }

    fn warp() -> wgpu::AdapterInfo {
        adapter(
            "Microsoft Basic Render Driver",
            wgpu::DeviceType::Cpu,
            wgpu::Backend::Dx12,
        )
    }

    fn card() -> wgpu::AdapterInfo {
        adapter(
            "AMD Radeon RX 7800 XT",
            wgpu::DeviceType::DiscreteGpu,
            wgpu::Backend::Dx12,
        )
    }

    const REPORT: &str = "C:\\Users\\me\\AppData\\Local\\BALLFIGHT Studio\\data\\logs\\start.txt";

    #[test]
    fn one_claim_per_folder() {
        let root = TempRoot::new("one claim");
        let config = root.0.join("config");
        let first = claim(&config);
        assert!(matches!(first, Instance::First(_)), "the first claim is the first copy");
        if cfg!(windows) {
            assert!(
                matches!(claim(&config), Instance::Second),
                "a second claim of the same folder"
            );
        }
        assert!(
            matches!(claim(&root.0.join("other config")), Instance::First(_)),
            "another folder is another lock"
        );
        drop(first);
        assert!(
            matches!(claim(&config), Instance::First(_)),
            "free again once the first is dropped"
        );
    }

    #[test]
    fn the_name_ignores_case() {
        assert_eq!(
            lock_name(Path::new("C:\\A\\config")),
            lock_name(Path::new("c:\\a\\CONFIG"))
        );
        assert_eq!(
            lock_name(Path::new("C:\\A\\config")),
            lock_name(Path::new("C:\\A\\config\\"))
        );
        assert_ne!(
            lock_name(Path::new("C:\\A\\config")),
            lock_name(Path::new("C:\\B\\config"))
        );
        let name = lock_name(Path::new("C:\\A\\config"));
        let hex = name.strip_prefix("Local\\BALLFIGHT Studio ").expect("the lock's name");
        assert_eq!(hex.len(), 16, "{name}");
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "{name}"
        );
    }

    #[test]
    fn a_software_adapter_is_noticed() {
        assert!(software_adapter(&warp()));
        assert!(!software_adapter(&card()));
        let integrated = adapter("Intel Arc", wgpu::DeviceType::IntegratedGpu, wgpu::Backend::Dx12);
        assert!(!software_adapter(&integrated));
        assert_eq!(window_graphics(&warp()).kind, Graphics::Software);
        assert_eq!(window_graphics(&warp()).words, SOFTWARE_DRAWING);
        assert_eq!(window_graphics(&card()).kind, Graphics::Card);
    }

    #[test]
    fn each_graphics_outcome_has_its_words() {
        let none = graphics(&[]);
        assert_eq!(
            none,
            Found {
                kind: Graphics::NoCard,
                words: NO_GRAPHICS.to_string()
            }
        );
        let soft = graphics(&[warp()]);
        assert_eq!(
            soft,
            Found {
                kind: Graphics::Software,
                words: SOFTWARE_DRAWING.to_string()
            }
        );
        // A card wins over the software adapter Windows always lists, wherever it is in the list.
        let both = graphics(&[warp(), card()]);
        assert_eq!(both.kind, Graphics::Card);
        assert_eq!(
            both.words,
            "DirectX 12 can draw with the graphics card AMD Radeon RX 7800 XT."
        );
        // The window's own adapter says which way it draws.
        let vulkan = window_graphics(&adapter(
            "AMD Radeon RX 7800 XT",
            wgpu::DeviceType::DiscreteGpu,
            wgpu::Backend::Vulkan,
        ));
        assert_eq!(
            vulkan.words,
            "The window draws with the graphics card AMD Radeon RX 7800 XT through Vulkan."
        );
        let slow = graphics_late(Late::Slow(GRAPHICS_WAIT));
        assert_eq!(slow.kind, Graphics::NoAnswer);
        assert_eq!(
            slow.words,
            "Windows did not say within 10 seconds which graphics cards can draw with DirectX 12. Restart the PC, \
             then start BALLFIGHT Studio again."
        );
        let broke = graphics_late(Late::Broke);
        assert_eq!(broke.kind, Graphics::NoAnswer);
        assert_eq!(
            broke.words,
            "The check for a graphics card that draws with DirectX 12 stopped with a fault. Restart the PC, then \
             start BALLFIGHT Studio again."
        );
        for found in [none, soft, both, vulkan, slow, broke] {
            reads_plainly(&found.words.replace("AMD Radeon RX 7800 XT", "the card"));
        }
    }

    #[test]
    fn no_window_names_the_report() {
        assert_eq!(no_window_text(None), NO_GRAPHICS);
        let report = Path::new(REPORT);
        assert_eq!(
            no_window_text(Some(report)),
            format!("{NO_GRAPHICS} The details were saved in {}.", report.display())
        );
    }

    #[test]
    fn the_texts_read_plainly() {
        assert_eq!(
            ALREADY_OPEN,
            "BALLFIGHT Studio is already open or still closing. Wait a moment, then start it again."
        );
        for text in [
            NO_GRAPHICS,
            SOFTWARE_DRAWING,
            ALREADY_OPEN,
            &no_window_text(Some(Path::new(REPORT))),
        ] {
            reads_plainly(text);
        }
        assert_eq!(breaks("Windows did not Offer a card"), ["Offer"], "the rule bites");
    }

    /// Asks this PC (read only) and prints what it found; any answer passes on Windows.
    #[test]
    fn this_pc_answers_the_graphics_check() {
        let found = check_graphics();
        if cfg!(windows) {
            // Every answer but "not Windows" is fine here: a PC without DirectX 12 must not fail the test.
            assert_ne!(found.kind, Graphics::NotWindows, "{found:?}");
        } else {
            assert_eq!(found.kind, Graphics::NotWindows);
        }
        println!("{}", found.words);
    }

    #[test]
    fn a_slow_or_broken_check_is_no_answer() {
        let slow = within("startup: test", Duration::from_millis(50), || {
            std::thread::sleep(Duration::from_secs(2));
            1
        });
        assert_eq!(slow, Err(Late::Slow(Duration::from_millis(50))));
        let broke = within("startup: test", Duration::from_secs(5), || -> u32 {
            panic!("on purpose")
        });
        assert_eq!(broke, Err(Late::Broke));
        assert_eq!(within("startup: test", Duration::from_secs(5), || 7), Ok(7));
        for found in [ffmpeg_late(Late::Slow(FFMPEG_WAIT)), ffmpeg_late(Late::Broke)] {
            assert_eq!(found.kind, Ffmpeg::NoAnswer);
            reads_plainly(&found.words);
        }
        for what in ["output", "data"] {
            for late in [Late::Slow(FOLDER_WAIT), Late::Broke] {
                let found = folder_late(Path::new("E:\\Videos\\BALLFIGHT"), late, what);
                assert_eq!(found.kind, Folder::NoAnswer);
                reads_plainly(&found.words);
            }
        }
    }

    #[test]
    fn each_ffmpeg_outcome_has_its_words() {
        let ready = |source| bf_export::Ffmpeg {
            ffmpeg: PathBuf::from("C:\\x\\ffmpeg.exe"),
            ffprobe: PathBuf::from("C:\\x\\ffprobe.exe"),
            version: "ffmpeg 9.0.2".to_string(),
            source,
        };
        let own = ffmpeg_found(Ok(ready(bf_export::FfmpegSource::WithTheApp)), None);
        assert_eq!(
            own,
            Found {
                kind: Ffmpeg::Ready,
                words: "ffmpeg 9.0.2 comes with the app.".to_string()
            }
        );
        let pcs = ffmpeg_found(Ok(ready(bf_export::FfmpegSource::Path)), None);
        assert_eq!(
            pcs,
            Found {
                kind: Ffmpeg::Ready,
                words: "ffmpeg 9.0.2 was found on this PC.".to_string()
            }
        );

        let root = TempRoot::new("ffmpeg words");
        let empty = root.0.join("empty");
        fs::create_dir_all(&empty).unwrap();
        // Nothing on the PATH, and no own folder expected (a developer's build): today's words.
        let missing = ffmpeg_found(
            bf_export::find_ffmpeg_with(None, None, Some(empty.clone().into())),
            None,
        );
        assert_eq!(
            missing,
            Found {
                kind: Ffmpeg::Missing,
                words: "ffmpeg was not found on this PC, so videos cannot be saved yet. Install ffmpeg, then start \
                        the app again."
                    .to_string()
            }
        );
        // Nothing found, but the program has an empty own folder (bf-export passes over it): install again.
        let program = root.0.join("program");
        let own_dir = program.join("ffmpeg");
        fs::create_dir_all(&own_dir).unwrap();
        assert_eq!(expected_own_folder(&program), Some(own_dir.clone()));
        let searched = bf_export::find_ffmpeg_with(None, Some(own_dir.clone()), Some(empty.clone().into()));
        let own_missing = ffmpeg_found(searched, expected_own_folder(&program).as_deref());
        assert_eq!(
            own_missing,
            Found {
                kind: Ffmpeg::Missing,
                words: format!(
                    "The app's own ffmpeg is missing from {}, so no video can be made; install the app again.",
                    own_dir.display()
                )
            }
        );
        assert!(bf_export::is_about_the_apps_own(&own_missing.words));
        // An installed copy (its MANIFEST.txt) without the folder at all expects it too; a bare build does not.
        let installed = root.0.join("installed");
        fs::create_dir_all(&installed).unwrap();
        assert_eq!(expected_own_folder(&installed), None);
        fs::write(installed.join("MANIFEST.txt"), b"").unwrap();
        assert_eq!(expected_own_folder(&installed), Some(installed.join("ffmpeg")));
        // The app's own copy with its ffmpeg gone: broken, in bf-export's words (they end "install the app again").
        fs::write(own_dir.join(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" }), b"").unwrap();
        let broken = ffmpeg_found(
            bf_export::find_ffmpeg_with(None, Some(own_dir.clone()), Some(empty.into())),
            Some(&own_dir),
        );
        assert_eq!(broken.kind, Ffmpeg::Broken);
        assert!(bf_export::is_about_the_apps_own(&broken.words), "{}", broken.words);
        // A file named ffmpeg on the PATH that is not a program: broken, not missing.
        let fake = root.0.join("fake");
        fs::create_dir_all(&fake).unwrap();
        for name in ["ffmpeg", "ffprobe"] {
            let file = if cfg!(windows) {
                format!("{name}.exe")
            } else {
                name.to_string()
            };
            fs::write(fake.join(file), b"not a program").unwrap();
        }
        let not_a_program = ffmpeg_found(bf_export::find_ffmpeg_with(None, None, Some(fake.into())), None);
        assert_eq!(not_a_program.kind, Ffmpeg::Broken, "{}", not_a_program.words);
        for found in [own, pcs, missing, own_missing, broken] {
            reads_plainly_under(&root, &found.words);
        }
    }

    #[test]
    fn each_output_folder_outcome_has_its_words() {
        let root = Path::new("E:\\Videos\\BALLFIGHT");
        let ready = output_found(root, FolderState::Ok { taken: false });
        assert_eq!(
            ready,
            Found {
                kind: Folder::Ready,
                words: "The output folder E:\\Videos\\BALLFIGHT can be written to.".to_string()
            }
        );
        let missing = output_found(root, FolderState::Missing);
        assert_eq!(
            missing,
            Found {
                kind: Folder::Missing,
                words: "The output folder E:\\Videos\\BALLFIGHT is missing. Create it, or choose another folder."
                    .to_string()
            }
        );
        let said = "The output folder E:\\Videos\\BALLFIGHT cannot be written to. Check that it is not read-only and \
                    that the drive has free space, or choose another folder.";
        let not_writable = output_found(root, FolderState::NotWritable(said.to_string()));
        assert_eq!(
            not_writable,
            Found {
                kind: Folder::NotWritable,
                words: said.to_string()
            }
        );
        let unknown = output_found(root, FolderState::Unknown);
        assert_eq!(unknown.kind, Folder::NoAnswer);
        for found in [ready, missing, not_writable, unknown] {
            reads_plainly(&found.words);
        }
    }

    #[test]
    fn the_output_folder_is_checked_and_left_as_it_was() {
        let root = TempRoot::new("output folder");
        let out = root.0.join("Videos").join("BALLFIGHT");
        // Missing: said so, and not made.
        assert_eq!(check_output_folder(&out).kind, Folder::Missing);
        assert!(!out.exists(), "the check never makes the output folder");
        // There: ready, and nothing left in it.
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("a video.mp4"), b"kept").unwrap();
        let ready = check_output_folder(&out);
        assert_eq!(ready.kind, Folder::Ready, "{}", ready.words);
        assert_eq!(fs::read_dir(&out).unwrap().count(), 1, "only the video is there");
        assert_eq!(fs::read(out.join("a video.mp4")).unwrap(), b"kept");
        // A file in its place.
        let file = root.0.join("a file");
        fs::write(&file, b"not a folder").unwrap();
        let not_a_folder = check_output_folder(&file);
        assert_eq!(not_a_folder.kind, Folder::NotAFolder);
        assert_eq!(
            not_a_folder.words,
            format!(
                "The output folder {} is a file, not a folder. Choose another folder.",
                file.display()
            )
        );
        assert_eq!(fs::read(&file).unwrap(), b"not a folder", "the file is untouched");
        reads_plainly_under(&root, &not_a_folder.words);
    }

    #[test]
    fn leftover_probes_go_and_nothing_else() {
        assert!(is_probe_name(".bf-start-check-1234-0.tmp"));
        assert!(is_probe_name(&probe_name(15)));
        for not in [
            ".bf-start-check-1234-0.tmp.bak",
            ".bf-start-check--0.tmp",
            ".bf-start-check-1234-.tmp",
            ".bf-start-check-12a4-0.tmp",
            ".bf-start-check-1234-0-1.tmp",
            "bf-start-check-1234-0.tmp",
            ".bf-write-check-1234-0.tmp",
            "settings.toml",
        ] {
            assert!(!is_probe_name(not), "{not}");
        }

        let root = TempRoot::new("leftovers");
        let dir = root.0.join("config");
        fs::create_dir_all(&dir).unwrap();
        // Earlier starts' leftovers: empty plain files with a probe's name. They go.
        for name in [".bf-start-check-1-0.tmp", ".bf-start-check-99999-15.tmp"] {
            fs::write(dir.join(name), b"").unwrap();
        }
        // Everything else stays, byte for byte.
        let kept: [(&str, &[u8]); 4] = [
            (".bf-start-check-2-0.tmp", b"holds bytes"),
            (".bf-start-check-3-0.tmp.bak", b""),
            (".bf-write-check-4-0.tmp", b""),
            ("settings.toml", b"format = 1\n"),
        ];
        for (name, bytes) in kept {
            fs::write(dir.join(name), bytes).unwrap();
        }
        // A folder with a probe's name is not a file: it stays, with what is in it.
        let folder = dir.join(".bf-start-check-5-0.tmp");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("inside"), b"").unwrap();

        assert_eq!(check_data_folder(&dir).kind, Folder::Ready);
        let mut left: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        let mut want: Vec<String> = kept.iter().map(|(n, _)| n.to_string()).collect();
        want.push(".bf-start-check-5-0.tmp".to_string());
        want.sort();
        assert_eq!(left, want);
        for (name, bytes) in kept {
            assert_eq!(fs::read(dir.join(name)).unwrap(), bytes, "{name}");
        }
        assert!(folder.join("inside").is_file());
    }

    #[test]
    fn a_data_folder_is_made_or_refused_in_words() {
        let root = TempRoot::new("data folder");
        let dir = root.0.join("BALLFIGHT Studio").join("config");
        let made = check_data_folder(&dir);
        assert_eq!(
            made,
            Found {
                kind: Folder::Ready,
                words: format!("BALLFIGHT Studio keeps its files in {}.", dir.display())
            }
        );
        assert!(dir.is_dir(), "a missing data folder is made");
        assert_eq!(
            fs::read_dir(&dir).unwrap().count(),
            0,
            "the probe leaves nothing behind"
        );

        // A file that has a probe's name but holds bytes is skipped, never written over or deleted.
        let taken = dir.join(probe_name(0));
        fs::write(&taken, b"someone else's").unwrap();
        assert_eq!(check_data_folder(&dir).kind, Folder::Ready);
        assert_eq!(fs::read(&taken).unwrap(), b"someone else's");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);

        // A file where the folder should be.
        let file = root.0.join("a file");
        fs::write(&file, b"not a folder").unwrap();
        let in_the_way = check_data_folder(&file);
        assert_eq!(
            in_the_way,
            Found {
                kind: Folder::NotAFolder,
                words: format!(
                    "BALLFIGHT Studio cannot keep its files in {}, because a file of that name is in the way. Move \
                     that file elsewhere, then start BALLFIGHT Studio again.",
                    file.display()
                )
            }
        );
        assert_eq!(fs::read(&file).unwrap(), b"not a folder");
        // A folder under a plain file cannot be made.
        let cannot = check_data_folder(&file.join("config"));
        assert_eq!(cannot.kind, Folder::NotWritable, "{}", cannot.words);
        for found in [made, in_the_way, cannot] {
            reads_plainly_under(&root, &found.words);
        }
        reads_plainly_under(&root, &left_behind(&dir, &taken));
        reads_plainly_under(&root, &data_cannot_write(&dir, true));
        reads_plainly_under(&root, &data_cannot_write(&dir, false));
    }
}
