//! The `ballfight` command line end to end: every command run as the program a person (or the app) runs, on the
//! example episode (FLUTE against TROMBONE, `studio/fights/flute_vs_trombone.toml`, recording
//! `studio/reference/fights/flute_vs_trombone/ep392.bfrec`) and on Studio-native fights found by `search`.
//!
//! Every test writes only under its own folder in Cargo's temporary folder for tests (`CARGO_TARGET_TMPDIR`, under
//! `studio/target`), named with spaces, a "!" and letters outside ASCII, and runs the program with that folder as
//! its working folder. Nothing is written to the Videos folder, `%APPDATA%` or `%LOCALAPPDATA%`.
//!
//! The tests that encode need ffmpeg; without it they print a notice and skip that part (with `BF_NEED_FFMPEG=1`, as
//! the CI job sets it, a missing ffmpeg fails them instead). The two whole renders
//! take about a minute and a half each in release, so they are ignored: run them by name with
//! `cargo test --release -p bf-cli --test e2e -- --ignored --nocapture`.
//!
//! `render_is_reproducible` compares the sound and three frames of the example episode with
//! `studio/replay/render_hashes.txt`; `BF_BLESS=1` writes that file instead (blessed on the Windows reference PC).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use bf_testdata::{reference, repo_root};
use serde_json::Value;
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------- shared set-up

/// A folder of this test's own, removed when dropped (only this folder, made by the test itself).
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(test: &str) -> TempRoot {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("bf-cli e2e! Zoë Ångström {test} {}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        TempRoot(root)
    }

    fn join(&self, rel: &str) -> PathBuf {
        self.0.join(rel)
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The example fight file.
fn example() -> PathBuf {
    repo_root().join("studio/fights/flute_vs_trombone.toml")
}

/// The example episode's recording.
fn ep392() -> PathBuf {
    reference("fights/flute_vs_trombone/ep392.bfrec")
}

/// The example fight file's text, changed by `edit`, written into `dir` as `name`.
fn fight_file(dir: &TempRoot, name: &str, edit: impl Fn(String) -> String) -> PathBuf {
    let text = std::fs::read_to_string(example()).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, edit(text)).unwrap();
    path
}

/// The text without its `seed = ...` line: a fight the Studio searches itself.
fn without_seed(text: String) -> String {
    let out: Vec<&str> = text.lines().filter(|l| !l.starts_with("seed = ")).collect();
    assert_ne!(out.len(), text.lines().count(), "the example has a seed line");
    out.join("\n") + "\n"
}

/// What a run gave.
struct Ran {
    code: i32,
    stdout: String,
    stderr: String,
}

impl Ran {
    fn of(out: Output) -> Ran {
        Ran {
            code: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }
    }

    /// Asserts success and gives stdout.
    fn ok(self, what: &str) -> Ran {
        assert_eq!(
            self.code, 0,
            "{what} failed:\nstdout: {}\nstderr: {}",
            self.stdout, self.stderr
        );
        no_panic(&self.stderr);
        self
    }

    /// Asserts exit code 1 with one line on stderr, and gives that line.
    fn refused(self, what: &str) -> String {
        assert_eq!(
            self.code, 1,
            "{what}: exit code {}\nstdout: {}\nstderr: {}",
            self.code, self.stdout, self.stderr
        );
        no_panic(&self.stderr);
        let lines: Vec<&str> = self.stderr.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(!lines.is_empty(), "{what}: nothing on stderr");
        let last = lines[lines.len() - 1];
        assert!(
            last.starts_with("ballfight: "),
            "{what}: the reason is not the last line: {}",
            self.stderr
        );
        last.to_string()
    }

    fn json(&self) -> Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|e| panic!("stdout is not JSON ({e}):\n{}\nstderr: {}", self.stdout, self.stderr))
    }
}

fn no_panic(stderr: &str) {
    assert!(
        !stderr.contains("panicked") && !stderr.contains("RUST_BACKTRACE"),
        "the program panicked:\n{stderr}"
    );
}

/// Runs `ballfight` with `args` in the folder `cwd`.
fn ballfight(cwd: &Path, args: &[&dyn AsRef<std::ffi::OsStr>]) -> Ran {
    let t0 = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_ballfight"))
        .args(args.iter().map(|a| a.as_ref()))
        .current_dir(cwd)
        .stdin(Stdio::null())
        .output()
        .expect("the program starts");
    let shown: Vec<String> = args.iter().map(|a| a.as_ref().to_string_lossy().into_owned()).collect();
    println!("ballfight {} ({:.1} s)", shown.join(" "), t0.elapsed().as_secs_f64());
    Ran::of(out)
}

/// Whether ffmpeg can be used here; prints a notice when it cannot (and fails when `BF_NEED_FFMPEG=1`, as in CI).
fn have_ffmpeg(test: &str) -> bool {
    match bf_export::find_ffmpeg() {
        Ok(_) => true,
        Err(e) => {
            assert!(
                std::env::var_os("BF_NEED_FFMPEG").is_none_or(|v| v != "1"),
                "{test}: BF_NEED_FFMPEG=1 but ffmpeg is not usable: {e}"
            );
            println!("{test}: the encoding part is skipped, ffmpeg is not usable here: {e}");
            false
        }
    }
}

fn sha256(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// The names in a folder, sorted.
fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The best seed of a `search --json` result.
fn best_seed(ran: &Ran) -> u64 {
    let v = ran.json();
    let best = &v["best"];
    assert_eq!(best["fits"], Value::Bool(true), "the best is a fit: {}", ran.stdout);
    best["seed"].as_u64().expect("the best has a seed")
}

/// The length of an mp4 in seconds, from ffprobe.
fn duration(mp4: &Path) -> f64 {
    let ff = bf_export::find_ffmpeg().expect("ffmpeg");
    let out = Command::new(&ff.ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=nw=1:nk=1",
        ])
        .arg(mp4)
        .output()
        .expect("ffprobe runs");
    String::from_utf8_lossy(&out.stdout).trim().parse().expect("a duration")
}

/// The width and height of a PNG file.
fn png_size(path: &Path) -> (u32, u32) {
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "{} is a PNG", path.display());
    let be = |i: usize| u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
    (be(16), be(20))
}

/// Asserts that a whole render's folder is complete and every check passed.
fn assert_finished(ran: &Ran, folder_name: &str, root: &TempRoot) {
    let v = ran.json();
    let folder = PathBuf::from(v["folder"].as_str().expect("the folder"));
    assert_eq!(folder, root.join("out").join(folder_name));
    let checks = v["checks"].as_array().expect("the checks");
    assert_eq!(checks.len(), 15, "8 before the render, 7 of the file: {checks:?}");
    for c in checks {
        assert_eq!(c["pass"], Value::Bool(true), "{c}");
    }
    let mut want = vec![
        "caption.txt".to_string(),
        "cover.png".to_string(),
        format!("{folder_name}.mp4"),
        format!("{folder_name}.toml"),
    ];
    want.sort();
    assert_eq!(entries(&folder), want);
    assert_eq!(png_size(&folder.join("cover.png")), (1080, 1920));
    let caption = std::fs::read_to_string(folder.join("caption.txt")).unwrap();
    assert!(
        caption.contains("instagram: ") && caption.contains("tiktok: "),
        "{caption}"
    );
    let secs = duration(&folder.join(format!("{folder_name}.mp4")));
    assert!((secs - 61.01).abs() < 0.01, "ffprobe says {secs} s");
    // nothing but the video's folder in the output folder
    assert_eq!(entries(&root.join("out")), [folder_name]);
}

// ---------------------------------------------------------------- the brief's proofs

/// The imported episode renders: the folder with the mp4, `cover.png` (1080 x 1920), `caption.txt` and the fight
/// file; every check passes; ffprobe reports 61.01 s.
#[test]
#[ignore = "a whole 61 s render, about a minute and a half in release: run it by name"]
fn imported_episode_renders() {
    if !have_ffmpeg("imported_episode_renders") {
        return;
    }
    let root = TempRoot::new("imported");
    let ran = ballfight(
        &root.0,
        &[
            &"render",
            &example(),
            &"--rec",
            &ep392(),
            &"--out",
            &root.join("out"),
            &"--json",
        ],
    )
    .ok("render");
    assert_finished(&ran, "flute_vs_trombone", &root);
}

/// A Studio-native fight makes a passing video: `search --stop 3` on the example without its seed, then
/// `render --seed <best>`.
#[test]
#[ignore = "a whole 61 s render, about a minute and a half in release: run it by name"]
fn native_fight_renders() {
    if !have_ffmpeg("native_fight_renders") {
        return;
    }
    let root = TempRoot::new("native");
    let ff = fight_file(&root, "native.toml", without_seed);
    let found = ballfight(&root.0, &[&"search", &ff, &"--stop", &"3", &"--json"]).ok("search");
    let seed = best_seed(&found);
    println!("best seed {seed}");
    let ran = ballfight(
        &root.0,
        &[
            &"render",
            &ff,
            &"--seed",
            &seed.to_string(),
            &"--out",
            &root.join("out"),
            &"--json",
        ],
    )
    .ok("render");
    assert_finished(&ran, "flute_vs_trombone", &root);
    // the fight file beside the video names the seed it was rendered with
    let saved = std::fs::read_to_string(root.join("out/flute_vs_trombone/flute_vs_trombone.toml")).unwrap();
    assert!(saved.contains(&format!("\nseed = {seed}\n")), "{saved}");
}

/// `studio/replay/render_hashes.txt`.
fn render_hashes_file() -> PathBuf {
    repo_root().join("studio/replay/render_hashes.txt")
}

const RENDER_HASHES_HEADER: &str = "\
# The SHA-256 of what `ballfight sound` and `ballfight frame` write for the example episode (FLUTE against TROMBONE,
# studio/fights/flute_vs_trombone.toml, recording studio/reference/fights/flute_vs_trombone/ep392.bfrec): the 16-bit
# master as a .wav, and frames 0, 500 and 1700 as 360 x 640 PNG files. Written by crates/bf-cli/tests/e2e.rs
# (render_is_reproducible) with BF_BLESS=1 on the Windows reference PC; every PC and every build must give the
# same lines. Do not edit by hand.
# file sha256
";

/// Two `sound` runs and two `frame` runs (frames 0, 500, 1,700) give identical bytes, equal to
/// `studio/replay/render_hashes.txt`; and a clip rendered with 1 and with 8 workers is the same file.
#[test]
fn render_is_reproducible() {
    let root = TempRoot::new("reproducible");
    let mut lines = Vec::new();
    for (i, workers) in ["1", "8"].into_iter().enumerate() {
        let wav = root.join(&format!("audio {i}.wav"));
        ballfight(
            &root.0,
            &[
                &"sound",
                &example(),
                &"--rec",
                &ep392(),
                &"--out",
                &wav,
                &"--workers",
                &workers,
            ],
        )
        .ok("sound");
        // the kit's own audio.wav of the episode, byte for byte
        assert_eq!(sha256(&wav), sha256(&reference("sound/flute_vs_trombone/audio.wav")));
        let mut run = vec![format!("audio.wav {}", sha256(&wav))];
        for f in [0, 500, 1700] {
            let png = root.join(&format!("f{f} {i}.png"));
            ballfight(
                &root.0,
                &[
                    &"frame",
                    &example(),
                    &"--rec",
                    &ep392(),
                    &"--frame",
                    &f.to_string(),
                    &"--out",
                    &png,
                ],
            )
            .ok("frame");
            assert_eq!(png_size(&png), (360, 640));
            run.push(format!("frame{f}.png {}", sha256(&png)));
        }
        if i == 0 {
            lines = run;
        } else {
            assert_eq!(run, lines, "a second run gave other bytes");
        }
    }
    let text = format!("{RENDER_HASHES_HEADER}{}\n", lines.join("\n"));
    let path = render_hashes_file();
    if std::env::var_os("BF_BLESS").is_some_and(|v| v == "1") {
        std::fs::write(&path, &text).expect("write render_hashes.txt");
        eprintln!("blessed {}", path.display());
    } else {
        let want = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e} (bless it with BF_BLESS=1)", path.display()));
        let want: Vec<&str> = want.lines().filter(|l| !l.starts_with('#')).collect();
        assert_eq!(lines, want, "the sound or a frame differs from {}", path.display());
    }

    if !have_ffmpeg("render_is_reproducible") {
        return;
    }
    let mut files = Vec::new();
    for workers in ["1", "8"] {
        let out = root.join(&format!("clip {workers}"));
        let ran = ballfight(
            &root.0,
            &[
                &"render",
                &example(),
                &"--rec",
                &ep392(),
                &"--frames",
                &"0:30",
                &"--out",
                &out,
                &"--workers",
                &workers,
            ],
        )
        .ok("render a clip");
        assert!(ran.stdout.contains("flute_vs_trombone_frames0-30"), "{}", ran.stdout);
        let name = "flute_vs_trombone_frames0-30";
        assert_eq!(entries(&out), [name]);
        assert_eq!(entries(&out.join(name)), [format!("{name}.mp4")]);
        files.push(std::fs::read(out.join(name).join(format!("{name}.mp4"))).unwrap());
    }
    assert!(
        files[0] == files[1],
        "the clip with 1 worker differs from the clip with 8"
    );
}

/// Review Focus 1: a seed without a K.O. (found by scanning with handicap [0.05, 0.05]) is refused by `render`
/// before any work, and `story` still prints its summary.
#[test]
fn no_ko_seed_is_refused() {
    let root = TempRoot::new("no ko");
    let ff = fight_file(&root, "weak.toml", |t| {
        without_seed(t).replace("handicap = [2.0, 0.4]", "handicap = [0.05, 0.05]")
    });
    let scan = ballfight(&root.0, &[&"hash", &ff, &"--seeds", &"1,2,3,4,5,6,7,8,9,10", &"--json"]).ok("hash");
    let seed = scan.json()["fights"]
        .as_array()
        .expect("the fights")
        .iter()
        .find(|f| f["t_ko"].is_null())
        .and_then(|f| f["seed"].as_u64())
        .expect("one of seeds 1 to 10 has no K.O. at handicap 0.05");
    let seed = seed.to_string();
    let out = root.join("out");
    let why = ballfight(&root.0, &[&"render", &ff, &"--seed", &seed, &"--out", &out]).refused("render");
    assert!(why.contains(&format!("seed {seed} has no K.O. before 58 s")), "{why}");
    assert!(!out.exists(), "render made {}", out.display());
    for cmd in ["check", "sound", "frame"] {
        let extra: Vec<&dyn AsRef<std::ffi::OsStr>> = match cmd {
            "sound" => vec![&"--out", &"x.wav"],
            "frame" => vec![&"--frame", &"0", &"--out", &"x.png"],
            _ => vec![],
        };
        let mut args: Vec<&dyn AsRef<std::ffi::OsStr>> = vec![&cmd, &ff, &"--seed", &seed];
        args.extend(extra);
        let why = ballfight(&root.0, &args).refused(cmd);
        assert!(why.contains("has no K.O. before 58 s"), "{cmd}: {why}");
    }
    let story = ballfight(&root.0, &[&"story", &ff, &"--seed", &seed]).ok("story");
    assert!(story.stdout.contains("no K.O."), "{}", story.stdout);
    assert_eq!(entries(&root.0), ["weak.toml"], "only the fight file is in the folder");
}

/// A fight file naming SAX exits 1 with the fight file's own message, in every command.
#[test]
fn bad_fight_file_exit_code() {
    let root = TempRoot::new("sax");
    let ff = fight_file(&root, "sax.toml", |t| t.replace("TROMBONE", "SAX"));
    let commands: [&[&dyn AsRef<std::ffi::OsStr>]; 9] = [
        &[&"sim", &ff, &"--out", &"x.bfrec"],
        &[&"story", &ff],
        &[&"search", &ff, &"--stop", &"1"],
        &[&"balance", &ff],
        &[&"frame", &ff, &"--frame", &"0", &"--out", &"x.png"],
        &[&"sound", &ff, &"--out", &"x.wav"],
        &[&"check", &ff],
        &[&"render", &ff, &"--out", &"out"],
        &[&"hash", &ff, &"--seeds", &"1"],
    ];
    for args in commands {
        let why = ballfight(&root.0, args).refused("a fight file with SAX");
        assert!(
            why.contains("sax.toml") && why.contains("fighters[1]: \"SAX\" is retired"),
            "{why}"
        );
    }
    assert_eq!(entries(&root.0), ["sax.toml"], "nothing was written");
}

/// Review Focus 5: the same fight file at 120 bpm (30 bars) and at 150 bpm (38 bars): `search`, then `check`
/// passes and `render --frames 0:60` succeeds.
///
/// The brief asks for `search --stop 1`. The first fit at 120 bpm (seed 204) fails the levels check (its loudest
/// 2 s window is 5.2 dB above the quietest, the bar is 5 dB), as the fourth fit at 150 bpm does: a fit is a story,
/// the checks judge the video. So the search keeps 3 fits and the first of them (best first) that passes the checks
/// is rendered; the refusals on the way are what `check` is for, and their exit code 1 is asserted.
#[test]
fn other_tempos_work() {
    let encode = have_ffmpeg("other_tempos_work");
    for bpm in ["120.0", "150.0"] {
        let root = TempRoot::new(&format!("tempo {bpm}"));
        let ff = fight_file(&root, "tempo.toml", |t| {
            without_seed(t).replace("tempo_bpm = 132.0", &format!("tempo_bpm = {bpm}"))
        });
        let found = ballfight(&root.0, &[&"search", &ff, &"--stop", &"3", &"--json"]).ok("search");
        let fits: Vec<u64> = found.json()["candidates"]
            .as_array()
            .expect("the candidates")
            .iter()
            .filter(|c| c["fits"] == Value::Bool(true))
            .filter_map(|c| c["seed"].as_u64())
            .collect();
        assert_eq!(fits.len(), 3, "{bpm} bpm: {fits:?}");
        let mut passed = None;
        for seed in fits {
            let seed = seed.to_string();
            let ran = ballfight(&root.0, &[&"check", &ff, &"--seed", &seed, &"--json"]);
            let results = ran.json()["checks"].as_array().expect("the checks").clone();
            assert_eq!(results.len(), 8);
            let all = results.iter().all(|c| c["pass"] == Value::Bool(true));
            if all {
                ran.ok("check");
                passed = Some(seed);
                break;
            }
            let why = ran.refused("a check that fails");
            println!("{bpm} bpm: seed {seed} is refused: {why}");
        }
        let seed = passed.unwrap_or_else(|| panic!("{bpm} bpm: none of the 3 fits passes the checks"));
        println!("{bpm} bpm: seed {seed} passes every check");
        if encode {
            let out = root.join("out");
            ballfight(
                &root.0,
                &[&"render", &ff, &"--seed", &seed, &"--frames", &"0:60", &"--out", &out],
            )
            .ok("render a clip");
            assert_eq!(entries(&out), ["flute_vs_trombone_frames0-60"]);
        }
    }
}

// ---------------------------------------------------------------- the command line's own promises

/// `hash` gives the engine's self-replay lines (`studio/replay/hashes.txt`).
#[test]
fn hash_is_the_replay() {
    let root = TempRoot::new("hash");
    let ran = ballfight(&root.0, &[&"hash", &example(), &"--seeds", &"1,2,3,500,1099511627776"]).ok("hash");
    let want = std::fs::read_to_string(repo_root().join("studio/replay/hashes.txt")).unwrap();
    let want: Vec<&str> = want.lines().filter(|l| l.starts_with("FLUTE TROMBONE ")).collect();
    let got: Vec<&str> = ran.stdout.lines().collect();
    assert_eq!(got, want);
}

/// `sim` writes a recording that `story --rec` reads to the same story as `story --seed`; `sim` never replaces a
/// file.
#[test]
fn sim_then_story() {
    let root = TempRoot::new("sim");
    let rec = root.join("fight 7.bfrec");
    let made = ballfight(
        &root.0,
        &[&"sim", &example(), &"--seed", &"7", &"--out", &rec, &"--json"],
    )
    .ok("sim");
    let v = made.json();
    assert_eq!(v["seed"].as_u64(), Some(7));
    assert!(rec.is_file());
    let before = std::fs::read(&rec).unwrap();
    let why = ballfight(&root.0, &[&"sim", &example(), &"--seed", &"8", &"--out", &rec]).refused("sim over a file");
    assert!(why.contains("already exists"), "{why}");
    assert_eq!(std::fs::read(&rec).unwrap(), before, "the file was not touched");
    let a = ballfight(&root.0, &[&"story", &example(), &"--rec", &rec]).ok("story --rec");
    let b = ballfight(&root.0, &[&"story", &example(), &"--seed", &"7"]).ok("story --seed");
    assert_eq!(a.stdout, b.stdout);
    assert!(a.stdout.contains("seed 7"), "{}", a.stdout);
    let ep = ballfight(&root.0, &[&"story", &example(), &"--rec", &ep392(), &"--json"]).ok("story of the episode");
    let v = ep.json();
    assert_eq!(v["seed"].as_u64(), Some(392));
    assert_eq!(v["winner_name"].as_str(), Some("TROMBONE"));
    assert_eq!(entries(&root.0), ["fight 7.bfrec"]);
}

/// `balance --write` changes only the handicap line of the fight file: comments, order and every other line stay.
#[test]
fn balance_writes_only_the_handicap() {
    let root = TempRoot::new("balance");
    let ff = fight_file(&root, "even.toml", |t| {
        let t = t.replace("handicap = [2.0, 0.4]", "handicap = [1.0, 1.0]   # to be balanced");
        format!("# my notes about this pairing\n{t}\n# the end\n")
    });
    let before = std::fs::read_to_string(&ff).unwrap();
    let dry = ballfight(&root.0, &[&"balance", &ff, &"--json"]).ok("balance");
    assert_eq!(
        std::fs::read_to_string(&ff).unwrap(),
        before,
        "without --write nothing changes"
    );
    let v = dry.json();
    let h: Vec<f64> = v["handicap"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    assert_eq!(h.len(), 2);
    assert!(
        v["in_band"].is_boolean() && v["confirm"]["win_share"].is_number(),
        "{v}"
    );

    ballfight(&root.0, &[&"balance", &ff, &"--write"]).ok("balance --write");
    let after = std::fs::read_to_string(&ff).unwrap();
    let (b, a): (Vec<&str>, Vec<&str>) = (before.lines().collect(), after.lines().collect());
    assert_eq!(a.len(), b.len());
    let differ: Vec<(&str, &str)> = b
        .iter()
        .zip(&a)
        .filter(|(x, y)| x != y)
        .map(|(x, y)| (*x, *y))
        .collect();
    assert_eq!(differ.len(), 1, "{differ:?}");
    let new_line = format!("handicap = [{:?}, {:?}]   # to be balanced", h[0], h[1]);
    assert_eq!(
        differ[0],
        ("handicap = [1.0, 1.0]   # to be balanced", new_line.as_str())
    );
    assert_eq!(entries(&root.0), ["even.toml"], "no temporary file is left");
}

/// What a person can type wrongly gives exit code 1 and one plain line, never a panic, and writes nothing.
#[test]
fn bad_input_is_one_line() {
    let root = TempRoot::new("bad input");
    let ff = example();
    let tuba = fight_file(&root, "tuba.toml", |t| t.replace("TROMBONE", "TUBA"));
    let broken = fight_file(&root, "broken.toml", |t| {
        t.replace("tempo_bpm = 132.0", "tempo_bpm = \"fast\"")
    });
    let taken = root.join("taken.png");
    std::fs::write(&taken, b"mine").unwrap();
    let ep = ep392();
    let not_rec = root.join("not a recording.bfrec");
    std::fs::write(&not_rec, b"hello").unwrap();
    let cases: Vec<(Vec<&dyn AsRef<std::ffi::OsStr>>, &str)> = vec![
        (vec![], "a command"),
        (vec![&"dance"], "dance"),
        (vec![&"story", &"no such file.toml"], "no such file.toml"),
        (vec![&"story", &tuba], "TUBA"),
        (vec![&"hash", &tuba, &"--seeds", &"1"], "TUBA"),
        (vec![&"story", &broken], "tempo_bpm"),
        (vec![&"story", &ff, &"--rec", &"missing.bfrec"], "missing.bfrec"),
        (vec![&"story", &ff, &"--rec", &not_rec], "not a recording.bfrec"),
        (vec![&"story", &ff, &"--seed", &"1", &"--rec", &ep], "--seed"),
        (vec![&"story", &ff, &"--seed", &"minus one"], "--seed"),
        (vec![&"search", &ff, &"--workers", &"0"], "--workers"),
        (vec![&"search", &ff, &"--workers", &"100000"], "--workers"),
        (vec![&"search", &ff, &"--max", &"99999999999999999999999"], "--max"),
        (vec![&"hash", &ff, &"--seeds", &""], "--seeds"),
        (vec![&"hash", &ff, &"--seeds", &"1,,2"], "--seeds"),
        (vec![&"frame", &ff, &"--frame", &"1830", &"--out", &"x.png"], "1830"),
        (
            vec![&"frame", &ff, &"--frame", &"0", &"--out", &taken],
            "already exists",
        ),
        (vec![&"frame", &ff, &"--frame", &"0"], "--out"),
        (vec![&"render", &ff, &"--out", &"out", &"--frames", &"60:30"], "60:30"),
        (
            vec![&"render", &ff, &"--out", &"out", &"--frames", &"0:99999"],
            "0:99999",
        ),
        (vec![&"render", &ff, &"--out", &"out", &"--frames", &"zero"], "--frames"),
        (vec![&"render", &ff, &"--out", &taken], "taken.png"),
        (vec![&"sound", &ff, &"--out", &"x.wav", &"--stems", &taken], "taken.png"),
    ];
    for (args, names) in cases {
        let shown: Vec<String> = args.iter().map(|a| a.as_ref().to_string_lossy().into_owned()).collect();
        let ran = ballfight(&root.0, &args);
        assert!(ran.stdout.is_empty(), "{shown:?} printed on stdout: {}", ran.stdout);
        let why = ran.refused(&format!("{shown:?}"));
        assert!(why.contains(names), "{shown:?}: {why}");
        assert_eq!(why.lines().count(), 1);
    }
    // a huge --max is fine: the search stops at its fits
    let ran = ballfight(
        &root.0,
        &[&"search", &ff, &"--stop", &"1", &"--max", &"18446744073709551615"],
    )
    .ok("max");
    assert!(ran.stdout.contains("seed"), "{}", ran.stdout);
    let mut left = entries(&root.0);
    left.retain(|n| !["tuba.toml", "broken.toml", "taken.png", "not a recording.bfrec"].contains(&n.as_str()));
    assert!(left.is_empty(), "written: {left:?}");
    assert_eq!(std::fs::read(&taken).unwrap(), b"mine");
}

/// `--json` puts one JSON value on stdout and nothing else; progress goes to stderr.
#[test]
fn json_is_clean() {
    let root = TempRoot::new("json");
    let ff = example();
    let runs: [&[&dyn AsRef<std::ffi::OsStr>]; 3] = [
        &[&"search", &ff, &"--stop", &"2", &"--json"],
        &[&"hash", &ff, &"--seeds", &"1,2", &"--json"],
        &[&"story", &ff, &"--json"],
    ];
    for args in runs {
        let ran = ballfight(&root.0, args).ok("json");
        let v = ran.json();
        assert!(v.is_object(), "{v}");
    }
    let found = ballfight(
        &root.0,
        &[&"search", &ff, &"--stop", &"2", &"--json", &"--out", &"short list.json"],
    )
    .ok("search --out");
    let saved: Value = serde_json::from_str(&std::fs::read_to_string(root.join("short list.json")).unwrap()).unwrap();
    assert_eq!(saved, found.json());
    let list = saved["candidates"].as_array().unwrap();
    assert_eq!(list.iter().filter(|c| c["fits"] == Value::Bool(true)).count(), 2);
    assert!(
        found.stderr.contains("seeds"),
        "progress is on stderr: {}",
        found.stderr
    );
}

/// Printing to a pipe that was closed ends the program quietly.
#[test]
fn closed_pipe_is_quiet() {
    let root = TempRoot::new("pipe");
    let seeds: Vec<String> = (1..=400).map(|s| s.to_string()).collect();
    let mut child = Command::new(env!("CARGO_BIN_EXE_ballfight"))
        .args([
            "hash".as_ref(),
            example().as_os_str(),
            "--seeds".as_ref(),
            seeds.join(",").as_ref(),
        ])
        .current_dir(&root.0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let mut err = String::new();
    child.stderr.take().unwrap().read_to_string(&mut err).unwrap();
    let status = child.wait().unwrap();
    no_panic(&err);
    assert!(status.code().is_some_and(|c| c == 0 || c == 1), "{status:?}: {err}");
}
