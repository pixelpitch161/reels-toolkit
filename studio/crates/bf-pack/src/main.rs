//! `ballfight-pack`: the packaging commands.
//!
//! - `fetch-ffmpeg --cache <dir> --out <dir>` downloads the pinned ffmpeg into the cache (or uses the checked copy
//!   already there) and puts its files and ABOUT.txt into the out folder.
//! - `package --exe <exe> --out <folder>` makes the package folder (the program folder as installed, with its
//!   MANIFEST.txt). It never downloads: ffmpeg must already be in the cache.
//! - `install --exe <exe> --from <owner's clone>` packages, makes the app's own source copy for updates, installs
//!   into `%LOCALAPPDATA%\Programs\BALLFIGHT Studio` and makes the Start menu and desktop shortcuts. `--root
//!   <folder>` installs under that folder instead (program folder, shortcuts and data all inside it).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use bf_pack::PackError;
use bf_pack::fetch::Download;
use bf_pack::install::{Layout, find_git, install, shortcuts_said, source_copy};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ballfight-pack", about = "Packaging for BALLFIGHT Studio.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Download the pinned ffmpeg (or use the checked copy in the cache) and put its files into the out folder.
    FetchFfmpeg {
        /// Where the archive and its unpacked files are kept between runs.
        #[arg(long)]
        cache: PathBuf,
        /// The folder that receives ffmpeg.exe, ffprobe.exe, LICENSE.txt and ABOUT.txt.
        #[arg(long)]
        out: PathBuf,
    },
    /// Make the package folder for a built exe. Downloads nothing.
    Package {
        /// The release build of the app (ballfight-studio.exe).
        #[arg(long)]
        exe: PathBuf,
        /// A new or empty folder, outside any cargo build folder.
        #[arg(long)]
        out: PathBuf,
        /// The checkout the icon, guide, ffmpeg pin and version come from (default: the one this tool was built
        /// from).
        #[arg(long)]
        from: Option<PathBuf>,
        /// The ffmpeg cache (default: the app's downloads folder on this PC).
        #[arg(long)]
        cache: Option<PathBuf>,
    },
    /// Package, then install with shortcuts and the source copy for updates. Downloads nothing but what git fetches.
    Install {
        /// The release build of the app (ballfight-studio.exe).
        #[arg(long)]
        exe: PathBuf,
        /// The owner's clone: the source of the package, and whose `origin` updates follow.
        #[arg(long)]
        from: PathBuf,
        /// Install under this folder instead of this PC's real folders (for trying it out).
        #[arg(long)]
        root: Option<PathBuf>,
        /// The ffmpeg cache (default: the app's downloads folder on this PC).
        #[arg(long)]
        cache: Option<PathBuf>,
    },
}

/// A download that never happens: packaging and installing use only what is already in the cache.
struct NoDownload;

impl Download for NoDownload {
    fn fetch(&self, _url: &str, _max_bytes: u64, _to: &Path) -> Result<(), String> {
        Err("packaging never downloads; run fetch-ffmpeg first to fill the cache".to_string())
    }
}

fn default_cache() -> Result<PathBuf, PackError> {
    Layout::system()
        .map(|l| l.data.join("downloads"))
        .ok_or_else(|| PackError::Refused("Windows did not say where the app's data folder is.".to_string()))
}

/// The checked ffmpeg files for the pin in `tree`, from the cache only.
fn checked_ffmpeg(tree: &Path, cache: &Path) -> Result<PathBuf, PackError> {
    let pin_path = tree.join("studio").join("packaging").join("ffmpeg.toml");
    let text = std::fs::read_to_string(&pin_path)
        .map_err(|e| PackError::Io(format!("Could not read {}: {e}.", pin_path.display())))?;
    let pin = bf_pack::ffmpeg::parse_pin(&text)?;
    bf_pack::ffmpeg::ensure(&pin, cache, &NoDownload)
}

/// A new folder in the temp folder for a package made on the way to an install.
fn temp_work() -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("ballfight-pack-{}-{stamp}", std::process::id()))
}

fn run_install(exe: &Path, from: &Path, root: Option<&Path>, cache: &Path) -> Result<Vec<String>, PackError> {
    let layout = match root {
        Some(r) => Layout::under(r),
        None => Layout::system()
            .ok_or_else(|| PackError::Refused("Windows did not say where this user's folders are.".to_string()))?,
    };
    let verified = checked_ffmpeg(from, cache)?;
    let work = temp_work();
    let pkg = work.join("package");
    let manifest = bf_pack::package::package(exe, from, &verified, &pkg)?;
    let done = (|| {
        let git = find_git()
            .ok_or_else(|| PackError::Refused("Git is not on this PC, so updates could not be set up.".to_string()))?;
        let config = source_copy(&git, from, &layout.data, &|line| println!("{line}"))?;
        install(&pkg, &layout, &config)?;
        let mut lines = vec![format!(
            "Installed BALLFIGHT Studio {} in {}.",
            manifest.version,
            layout.programs.display()
        )];
        // The install has worked by now: a shortcut that could not be made is a note, not a failure.
        lines.push(shortcuts_said(&layout));
        lines.push(format!(
            "Updates follow the copy of the source in {}.",
            config.source.display()
        ));
        Ok(lines)
    })();
    // The temporary package goes again, file by file, by its own manifest.
    let _ = bf_pack::remove::remove_by_manifest(&pkg);
    let _ = std::fs::remove_dir(&work);
    done
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        // A bad embedded pin is a sentence and exit 1, never a panic.
        Command::FetchFfmpeg { cache, out } => {
            bf_pack::ffmpeg::fetch_command(bf_pack::ffmpeg::PIN_TEXT, &cache, &out, &bf_pack::fetch::Curl)
        }
        Command::Package { exe, out, from, cache } => (|| {
            let tree = from.unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join(".."));
            let cache = match cache {
                Some(c) => c,
                None => default_cache()?,
            };
            let verified = checked_ffmpeg(&tree, &cache)?;
            let m = bf_pack::package::package(&exe, &tree, &verified, &out)?;
            let mut lines: Vec<String> = m
                .files
                .iter()
                .map(|(p, bytes, _)| format!("{bytes:>12}  {p}"))
                .collect();
            let total: u64 = m.files.iter().map(|(_, b, _)| b).sum();
            lines.push(format!("{total:>12}  in all, {} files", m.files.len()));
            lines.push(format!(
                "The package for version {} is ready in {}.",
                m.version,
                out.display()
            ));
            Ok(lines)
        })(),
        Command::Install { exe, from, root, cache } => (|| {
            let cache = match cache {
                Some(c) => c,
                None => default_cache()?,
            };
            run_install(&exe, &from, root.as_deref(), &cache)
        })(),
    };
    match result {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
