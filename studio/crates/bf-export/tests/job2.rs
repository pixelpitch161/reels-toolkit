//! Engine 2's render job end to end: a short clip of the TEST DUMMY mirror (`fights/engine2/test_dummy.toml`)
//! encoded for real at 1080 x 1920, and a cancel that leaves nothing behind. The tests print a notice and pass when
//! ffmpeg is not installed. Each writes only under its own folder in Cargo's temporary folder for tests.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bf_core::fightfile::FightFile;
use bf_export::job2::{SILENCE, run_job2};
use bf_export::{EncodeSpeed, ExportError, JobEvent, JobOpts, find_ffmpeg, stage};
use bf_testdata::repo_root;
use bf2_sim::{Recording2, Rules2, Setup2};

static LOCK: Mutex<()> = Mutex::new(());

fn dummy() -> (FightFile, Arc<Recording2>) {
    let path = repo_root().join("studio/fights/engine2/test_dummy.toml");
    let ff = FightFile::parse(&std::fs::read_to_string(&path).unwrap()).expect("the dummy's file parses");
    let def = |k: usize| bf_content::fighters2::def(&ff.fighters[k]).expect("an Engine 2 fighter");
    let mut rules = Rules2::new(ff.grid().t_fight);
    rules.dmg_scale = ff.dmg_scale;
    let setup = Setup2 {
        fighters: [def(0), def(1)],
        rules,
        seed: ff.seed.expect("the file names its seed"),
    };
    let rec = bf2_sim::run(&setup, true).expect("the dummy fights");
    (ff, Arc::new(rec))
}

fn root(test: &str) -> PathBuf {
    let r = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("job2 {test} é!"));
    let _ = std::fs::remove_dir_all(&r);
    std::fs::create_dir_all(&r).unwrap();
    r
}

fn probe(mp4: &Path, entries: &str, stream: &str) -> String {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            stream,
            "-show_entries",
            entries,
            "-of",
            "csv=p=0",
        ])
        .arg(mp4)
        .output()
        .expect("ffprobe runs");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn engine2_job_writes_a_61s_1080x1920_video() {
    let _l = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Ok(ffmpeg) = find_ffmpeg() else {
        eprintln!("ffmpeg is not installed: skipped");
        return;
    };
    let (ff, rec) = dummy();
    let out = root("video");
    let opts = JobOpts {
        out_root: out.clone(),
        workers: 1,
        // 2 s of the fight (the 61 s soundtrack is whole, as in a current-engine clip).
        frames: Some(600..660),
        speed: EncodeSpeed::Faster,
    };
    let events = Mutex::new(Vec::new());
    let on = |e: JobEvent| events.lock().unwrap().push(e);
    let folder = run_job2(&ffmpeg, &ff, rec, &opts, &on, &AtomicBool::new(false)).expect("the clip renders");
    let mp4 = std::fs::read_dir(&folder)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|x| x == "mp4"))
        .expect("an mp4");
    assert_eq!(
        probe(&mp4, "stream=codec_name,width,height,r_frame_rate", "v:0"),
        "h264,1080,1920,30/1"
    );
    assert_eq!(probe(&mp4, "stream=codec_name", "a:0"), "aac");
    assert_eq!(probe(&mp4, "stream=nb_frames", "v:0"), "60");
    let audio: f64 = probe(&mp4, "stream=duration", "a:0").parse().unwrap();
    assert!((audio - 61.0).abs() < 0.1, "the whole silent soundtrack: {audio} s");
    let events = events.into_inner().unwrap();
    assert_eq!(events.first(), Some(&JobEvent::Stage(SILENCE)));
    assert!(events.contains(&JobEvent::Stage(stage::FRAMES)));
    assert!(events.iter().any(|e| matches!(
        e,
        JobEvent::Progress {
            done: 60,
            total: 60,
            ..
        }
    )));
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn cancel_leaves_no_file() {
    let _l = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Ok(ffmpeg) = find_ffmpeg() else {
        eprintln!("ffmpeg is not installed: skipped");
        return;
    };
    let (ff, rec) = dummy();
    let out = root("cancel");
    let opts = JobOpts {
        out_root: out.clone(),
        workers: 1,
        frames: None,
        speed: EncodeSpeed::Faster,
    };
    let cancel = AtomicBool::new(false);
    // Cancelled once 5 frames have gone to ffmpeg.
    let on = |e: JobEvent| {
        if let JobEvent::Progress { done: 5, .. } = e {
            cancel.store(true, Ordering::SeqCst);
        }
    };
    let e = run_job2(&ffmpeg, &ff, rec, &opts, &on, &cancel).expect_err("cancelled");
    assert!(matches!(e, ExportError::Cancelled(_)), "{e}");
    let left: Vec<_> = std::fs::read_dir(&out).unwrap().collect();
    assert!(left.is_empty(), "nothing is left: {left:?}");
    std::fs::remove_dir_all(&out).unwrap();
}
