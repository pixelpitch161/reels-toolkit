//! `ballfight-pack`: the packaging commands. `fetch-ffmpeg --cache <dir> --out <dir>` downloads the pinned ffmpeg
//! into the cache (or uses the checked copy already there) and puts its files and ABOUT.txt into the out folder.

use std::path::PathBuf;
use std::process::ExitCode;

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
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::FetchFfmpeg { cache, out } => {
            let pin = bf_pack::ffmpeg::pin();
            bf_pack::ffmpeg::ensure(&pin, &cache, &bf_pack::fetch::Curl)
                .and_then(|verified| bf_pack::ffmpeg::place(&pin, &verified, &out))
                .map(|()| format!("ffmpeg {} is ready in {}.", pin.version, out.display()))
        }
    };
    match result {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
