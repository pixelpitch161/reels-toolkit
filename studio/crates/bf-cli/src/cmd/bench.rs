//! `ballfight bench <fight.toml> [--seed N | --rec F] [--out FOLDER]`: how fast this PC fights, draws and sounds a
//! video, measured on the fight file's fight (milestone 1's measurements, design section 7's targets).
//!
//! The parts, in order (`--only` picks some; each starts with the PC's overall CPU use over the second before it, so
//! a busy PC shows in the numbers):
//!
//! - `fights`: fights a second with the recording off (fight, story and score, as the search does) on seeds 1 to
//!   `--seeds` (2,000), on all workers and on one.
//! - `frame`: one frame on one thread, every `--every`-th frame (10): mean, 95th percentile (numpy's linear rule)
//!   and the slowest; and the picture's set-up.
//! - `stages`: the frame by stage, every `--stage-every`-th frame (122): the time to reach each stage less the time
//!   to reach the one before it, as a mean in milliseconds and a share of the frame.
//! - `draw`: all 1,830 frames on all workers, without the encode.
//! - `sound`: the soundtrack build.
//! - `render` (only with `--out`): the whole render job into a new folder inside `--out`, with the encode, by
//!   stage, with the encoder setting `--encode` (best, the default, or faster), which it reports with the preset
//!   and the mp4's size. Without `--out` nothing is written anywhere. The render stops at a failed check like any render, so
//!   time it on a fight that passes them (the example episode:
//!   `--rec reference/fights/flute_vs_trombone/ep392.bfrec`; the Studio's own fight of seed 392 fails `levels`).
//!
//! Text by default; `--json` prints one JSON value with every number. Progress goes to stderr.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bf_export::{EncodeSpeed, ExportError, JobEvent, JobOpts, run_job};
use bf_picture::config::NF;
use bf_picture::ring::{Picture, Stage};
use bf_sim::Setup;
use bf_story::{BeatSheet, Halt};
use serde_json::{Map, Value, json};

use crate::{
    Ctx, Done, Fail, encode_speed, fighters, load_fight, need_ko, note, num, say, say_json, stopped, the_fight,
};

/// The parts, in the order they run.
const PARTS: [&str; 6] = ["fights", "frame", "stages", "draw", "sound", "render"];

/// The stages part times every this many-th frame by default (15 frames, each stage the best of three).
const STAGE_EVERY: usize = 122;

#[derive(clap::Args)]
pub struct Args {
    /// The fight file.
    fight: PathBuf,
    /// The seed to draw and sound (default: the fight file's).
    #[arg(long, value_name = "N", conflicts_with = "rec")]
    seed: Option<u64>,
    /// Draw and sound this recording instead of fighting the seed.
    #[arg(long, value_name = "FILE")]
    rec: Option<PathBuf>,
    /// Also render the whole video, with the encode, into a new folder inside this one (the only thing bench
    /// writes).
    #[arg(long, value_name = "FOLDER")]
    out: Option<PathBuf>,
    /// Only these parts, separated by commas (fights, frame, stages, draw, sound, render; default: all, render
    /// only with --out).
    #[arg(long, value_name = "PARTS", value_delimiter = ',')]
    only: Option<Vec<String>>,
    /// The seeds the fights part fights (1 to N).
    #[arg(long, value_name = "N", default_value_t = 2000, value_parser = clap::value_parser!(u64).range(1..=1_000_000))]
    seeds: u64,
    /// The frame part times every N-th frame.
    #[arg(long, value_name = "N", default_value_t = 10, value_parser = clap::value_parser!(u64).range(1..=NF as u64))]
    every: u64,
    /// The stages part times every N-th frame.
    #[arg(
        long,
        value_name = "N",
        default_value_t = STAGE_EVERY as u64,
        value_parser = clap::value_parser!(u64).range(1..=NF as u64)
    )]
    stage_every: u64,
    /// The encoder setting of the render part: best (the series' own, the default) or faster.
    #[arg(long, value_name = "SETTING", default_value = "best", value_parser = encode_speed)]
    pub encode: EncodeSpeed,
}

pub fn run(a: &Args, ctx: &Ctx) -> Done {
    let parts = chosen(a)?;
    let ff = load_fight(&a.fight)?;
    if let Some(out) = &a.out
        && std::fs::metadata(out).is_ok_and(|m| !m.is_dir())
    {
        return Err(Fail(format!(
            "the output folder {} is not a folder (a file has that name), so nothing was measured; choose another.",
            out.display()
        )));
    }
    let has = |p: &str| parts.contains(&p);
    let mut report = Map::new();
    report.insert("machine".into(), machine(ctx));
    // text goes out part by part, so what was measured stays on screen if a later part fails
    let show = |lines: &[String]| {
        if !ctx.json {
            lines.iter().for_each(|l| say(l));
        }
    };
    show(&[machine_line(ctx)]);

    if has("fights") {
        let (v, line) = fights(a, &ff, ctx)?;
        report.insert("fights".into(), v);
        show(&[line]);
    }
    let needs_fight = ["frame", "stages", "draw", "sound", "render"].iter().any(|p| has(p));
    if needs_fight {
        stopped(ctx, "the bench")?;
        let rec = the_fight(&ff, &a.fight, a.rec.as_deref(), a.seed, true)?;
        need_ko(&rec)?;
        let seed = rec.seed;
        report.insert("seed".into(), json!(seed));
        show(&[format!("fight: seed {seed}")]);
        let rec = Arc::new(rec);
        if has("frame") || has("stages") || has("draw") {
            let busy = cpu_busy();
            let t0 = Instant::now();
            let pic = bf_content::build_picture(Arc::clone(&rec), &ff)
                .map_err(|e| Fail(format!("{}: {e}.", a.fight.display())))?;
            let setup_ms = ms_since(t0);
            // the sprites and the lens's tables are made on first use: not part of a frame's time
            drop(pic.frame_reported(NF / 3));
            if has("frame") {
                let (v, line) = frame(a, &pic, ctx, setup_ms, busy)?;
                report.insert("frame".into(), v);
                show(&[line]);
            }
            if has("stages") {
                let (v, more) = stages(a, &pic, ctx)?;
                report.insert("stages".into(), v);
                show(&more);
            }
            if has("draw") {
                let (v, line) = draw_all(&pic, ctx)?;
                report.insert("draw".into(), v);
                show(&[line]);
            }
        }
        if has("sound") {
            stopped(ctx, "the bench")?;
            let busy = cpu_busy();
            note("bench: building the soundtrack");
            let t0 = Instant::now();
            let st =
                bf_content::build_soundtrack(&rec, &ff).map_err(|e| Fail(format!("{}: {e}.", a.fight.display())))?;
            let s = t0.elapsed().as_secs_f64();
            drop(st);
            report.insert(
                "sound".into(),
                json!({"cpu_busy": opt(busy), "soundtrack_build_s": num(s)}),
            );
            show(&[format!("sound: soundtrack_build_s {s:.2} s{}", busy_words(busy))]);
        }
        if has("render") {
            let mut ff = ff.clone();
            ff.seed = Some(seed);
            let (v, more) = render(&ff, rec, a.out.clone().unwrap_or_default(), a.encode, ctx)?;
            report.insert("render".into(), v);
            show(&more);
        }
    }
    if ctx.json {
        say_json(&Value::Object(report));
    }
    Ok(())
}

/// The parts asked for, in run order.
fn chosen(a: &Args) -> Result<Vec<&'static str>, Fail> {
    let Some(only) = &a.only else {
        return Ok(PARTS
            .iter()
            .copied()
            .filter(|&p| p != "render" || a.out.is_some())
            .collect());
    };
    for p in only {
        if !PARTS.contains(&p.trim()) {
            return Err(Fail(format!(
                "--only: {:?} is not a part of the bench; the parts are {}.",
                p,
                PARTS.join(", ")
            )));
        }
    }
    let parts: Vec<&'static str> = PARTS
        .iter()
        .copied()
        .filter(|p| only.iter().any(|q| q.trim() == *p))
        .collect();
    if parts.contains(&"render") && a.out.is_none() {
        return Err(Fail::new(
            "--only render needs --out FOLDER, the folder the video is rendered into.",
        ));
    }
    Ok(parts)
}

// ---------------------------------------------------------------- the parts

/// Fights a second on seeds 1 to `--seeds`, record off, on all workers and on one.
fn fights(a: &Args, ff: &bf_core::fightfile::FightFile, ctx: &Ctx) -> Result<(Value, String), Fail> {
    let cfg = fighters(ff)?;
    let sheet = BeatSheet::series(cfg);
    let base =
        bf_sim::setup_from(ff, bf_content::roster(), 0).map_err(|e| Fail(format!("{}: {e}.", a.fight.display())))?;
    let setup_for = |seed: u64| Setup { seed, ..base.clone() };
    let rate = |workers: usize| -> Result<(f64, Option<f64>), Fail> {
        stopped(ctx, "the bench")?;
        let busy = cpu_busy();
        note(&format!("bench: {} fights on {workers} worker(s)", a.seeds));
        let t0 = Instant::now();
        bf_story::stats_until(&setup_for, &sheet, 1..a.seeds + 1, workers, ctx.cancel).map_err(|h| match h {
            Halt::Cancelled => Fail::new("the bench was stopped (Ctrl+C)."),
            Halt::Failed(why) => Fail(format!("{}: {why}.", a.fight.display())),
        })?;
        Ok((a.seeds as f64 / t0.elapsed().as_secs_f64(), busy))
    };
    let (all, busy_all) = rate(ctx.workers)?;
    let (one, busy_one) = rate(1)?;
    let v = json!({
        "seeds": a.seeds,
        "workers": ctx.workers,
        "fights_per_second": num(all),
        "cpu_busy": opt(busy_all),
        "one_worker": {"fights_per_second": num(one), "cpu_busy": opt(busy_one)},
    });
    let line = format!(
        "fights: fights_per_second {all:.0} on {} workers{}, {one:.0} on one{} ({} seeds, record off)",
        ctx.workers,
        busy_words(busy_all),
        busy_words(busy_one),
        a.seeds
    );
    Ok((v, line))
}

/// One frame on one thread, every `--every`-th frame.
fn frame(a: &Args, pic: &Picture, ctx: &Ctx, setup_ms: f64, busy: Option<f64>) -> Result<(Value, String), Fail> {
    let every = usize::try_from(a.every).unwrap_or(10);
    let frames: Vec<usize> = (0..NF).step_by(every).collect();
    note(&format!("bench: {} frames on one thread", frames.len()));
    let mut times = Vec::with_capacity(frames.len());
    for &f in &frames {
        stopped(ctx, "the bench")?;
        let t0 = Instant::now();
        drop(pic.frame_reported(f));
        times.push(ms_since(t0));
    }
    let (mean, p95, max) = (
        mean(&times),
        percentile(&times, 95.0),
        times.iter().copied().fold(0.0, f64::max),
    );
    let v = json!({
        "cpu_busy": opt(busy),
        "every": every,
        "frames": frames.len(),
        "frame_ms_mean": num(mean),
        "frame_ms_p95": num(p95),
        "frame_ms_max": num(max),
        "picture_setup_ms": num(setup_ms),
    });
    let line = format!(
        "frame: frame_ms_p95 {p95:.1} ms, mean {mean:.1} ms, slowest {max:.1} ms (one thread, every {every}th \
         frame: {} frames){}; picture set-up {setup_ms:.0} ms",
        frames.len(),
        busy_words(busy)
    );
    Ok((v, line))
}

/// The frame by stage: for every `--stage-every`-th frame, the time to reach each stage less the time to reach the
/// one before it.
fn stages(a: &Args, pic: &Picture, ctx: &Ctx) -> Result<(Value, Vec<String>), Fail> {
    let every = usize::try_from(a.stage_every).unwrap_or(STAGE_EVERY);
    let frames: Vec<usize> = (0..NF).step_by(every).collect();
    let busy = cpu_busy();
    note(&format!("bench: {} frames stage by stage on one thread", frames.len()));
    // each time is the best of three, so another program's moment on the core does not land on one stage
    let best = |job: &dyn Fn()| -> f64 {
        (0..3)
            .map(|_| {
                let t0 = Instant::now();
                job();
                ms_since(t0)
            })
            .fold(f64::INFINITY, f64::min)
    };
    let mut sum = vec![0.0; Stage::ALL.len()];
    let mut whole = 0.0;
    for &f in &frames {
        let mut before = 0.0;
        for (k, stage) in Stage::ALL.iter().enumerate() {
            stopped(ctx, "the bench")?;
            let upto = best(&|| drop(pic.stage_buffers(f, *stage)));
            sum[k] += upto - before;
            before = upto;
        }
        whole += best(&|| drop(pic.frame_reported(f)));
    }
    let n = frames.len() as f64;
    let whole = whole / n;
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "stages: one thread, every {every}th frame ({} frames), a frame {whole:.1} ms{}:",
        frames.len(),
        busy_words(busy)
    )];
    for (k, stage) in Stage::ALL.iter().enumerate() {
        let ms = sum[k] / n;
        let share = 100.0 * ms / whole;
        rows.push(json!({"stage": stage.name(), "ms": num(ms), "share": num(share)}));
        lines.push(format!("  {:<10} {ms:7.1} ms {share:5.1} %", stage.name()));
    }
    let v = json!({
        "cpu_busy": opt(busy),
        "every": every,
        "frames": frames.len(),
        "frame_ms_mean": num(whole),
        "stages": rows,
    });
    Ok((v, lines))
}

/// All frames on all workers, without the encode.
fn draw_all(pic: &Picture, ctx: &Ctx) -> Result<(Value, String), Fail> {
    stopped(ctx, "the bench")?;
    let busy = cpu_busy();
    note(&format!("bench: all {NF} frames on {} workers", ctx.workers));
    let next = AtomicUsize::new(0);
    let refused = Mutex::new(None);
    let t0 = Instant::now();
    std::thread::scope(|s| {
        for _ in 0..ctx.workers.max(1) {
            s.spawn(|| {
                while !ctx.cancel.load(Ordering::SeqCst) {
                    let f = next.fetch_add(1, Ordering::Relaxed);
                    if f >= NF {
                        break;
                    }
                    let (_, _, report) = pic.frame_reported(f);
                    if report.hull_errors != 0 {
                        *refused.lock().unwrap_or_else(|e| e.into_inner()) = Some(f);
                    }
                }
            });
        }
    });
    let s = t0.elapsed().as_secs_f64();
    stopped(ctx, "the bench")?;
    if let Some(f) = refused.into_inner().unwrap_or_else(|e| e.into_inner()) {
        return Err(Fail(format!(
            "frame {f} cannot be drawn the way the reference does (a shape was refused), so the draw time is not a \
             video's."
        )));
    }
    let v = json!({
        "cpu_busy": opt(busy),
        "workers": ctx.workers,
        "frames": NF,
        "seconds": num(s),
        "frames_per_second": num(NF as f64 / s),
    });
    let line = format!(
        "draw: all {NF} frames on {} workers in {s:.1} s ({:.1} frames a second, no encode){}",
        ctx.workers,
        NF as f64 / s,
        busy_words(busy)
    );
    Ok((v, line))
}

/// The whole render job into a new folder inside `out`, by stage.
fn render(
    ff: &bf_core::fightfile::FightFile,
    rec: Arc<bf_core::recording::Recording>,
    out: PathBuf,
    speed: EncodeSpeed,
    ctx: &Ctx,
) -> Result<(Value, Vec<String>), Fail> {
    stopped(ctx, "the bench")?;
    let busy = cpu_busy();
    note(&format!(
        "bench: rendering the whole video on {} workers, encode {}",
        ctx.workers,
        speed.name()
    ));
    let opts = JobOpts {
        out_root: out,
        workers: ctx.workers,
        frames: None,
        speed,
    };
    let marks: Mutex<Vec<(&'static str, Instant)>> = Mutex::new(Vec::new());
    let passed = AtomicUsize::new(0);
    let checks = AtomicUsize::new(0);
    let on = |ev: JobEvent| match ev {
        JobEvent::Stage(s) => {
            note(&format!("bench: render: {s}"));
            marks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((s, Instant::now()));
        }
        JobEvent::Check(r) => {
            checks.fetch_add(1, Ordering::Relaxed);
            if r.pass {
                passed.fetch_add(1, Ordering::Relaxed);
            }
        }
        JobEvent::Progress { .. } | JobEvent::Done(_) => {}
    };
    let t0 = Instant::now();
    let folder = run_job(ff, rec, &opts, &on, ctx.cancel).map_err(|e| match e {
        ExportError::Cancelled(_) => Fail::new("the bench was stopped (Ctrl+C) while it rendered; nothing was saved."),
        e => Fail(e.to_string()),
    })?;
    let end = Instant::now();
    let total = (end - t0).as_secs_f64();
    let marks = marks.into_inner().unwrap_or_else(|e| e.into_inner());
    let mut rows = Vec::new();
    let mut words = Vec::new();
    for (k, (name, at)) in marks.iter().enumerate() {
        let until = marks.get(k + 1).map_or(end, |m| m.1);
        let s = (until - *at).as_secs_f64();
        rows.push(json!({"stage": name, "seconds": num(s)}));
        words.push(format!("{name} {s:.1} s"));
    }
    let (passed, checks) = (passed.into_inner(), checks.into_inner());
    let mp4 = folder.join(format!(
        "{}.mp4",
        folder
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    ));
    let mp4_bytes = std::fs::metadata(&mp4).map(|m| m.len()).ok();
    let v = json!({
        "cpu_busy": opt(busy),
        "workers": ctx.workers,
        "encode": speed.name(),
        "preset": speed.preset(),
        "mp4_bytes": mp4_bytes,
        "render_s": num(total),
        "stages": rows,
        "checks": checks,
        "checks_passed": passed,
        "folder": folder.display().to_string(),
    });
    let lines = vec![
        format!(
            "render: the whole video with the encode ({}, x264 preset {}) in {total:.1} s on {} workers{} ({passed} \
             of {checks} checks passed)",
            speed.name(),
            speed.preset(),
            ctx.workers,
            busy_words(busy)
        ),
        format!("  {}", words.join(", ")),
        format!("  saved {}", folder.display()),
    ];
    Ok((v, lines))
}

// ---------------------------------------------------------------- numbers

fn ms_since(t0: Instant) -> f64 {
    t0.elapsed().as_secs_f64() * 1e3
}

fn mean(x: &[f64]) -> f64 {
    if x.is_empty() {
        return f64::NAN;
    }
    x.iter().sum::<f64>() / x.len() as f64
}

/// `np.percentile(x, q)`: linear between the two nearest ranks.
fn percentile(x: &[f64], q: f64) -> f64 {
    if x.is_empty() {
        return f64::NAN;
    }
    let mut v = x.to_vec();
    v.sort_by(f64::total_cmp);
    let pos = q / 100.0 * (v.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(v.len() - 1);
    let frac = pos - lo as f64;
    v[lo] + (v[hi] - v[lo]) * frac
}

fn opt(x: Option<f64>) -> Value {
    x.map_or(Value::Null, num)
}

fn busy_words(busy: Option<f64>) -> String {
    busy.map_or(String::new(), |b| format!(" [PC {b:.0} % busy before]"))
}

// ---------------------------------------------------------------- the machine

fn machine(ctx: &Ctx) -> Value {
    json!({
        "cpu": cpu_name(),
        "logical_cores": std::thread::available_parallelism().map_or(1, |n| n.get()),
        "workers": ctx.workers,
        "build": if cfg!(debug_assertions) { "debug" } else { "release" },
    })
}

fn machine_line(ctx: &Ctx) -> String {
    format!(
        "machine: {}, {} logical cores, {} workers, {} build",
        cpu_name().unwrap_or_else(|| "CPU unknown".to_string()),
        std::thread::available_parallelism().map_or(1, |n| n.get()),
        ctx.workers,
        if cfg!(debug_assertions) { "debug" } else { "release" }
    )
}

/// The CPU's name as Windows gives it.
#[cfg(windows)]
fn cpu_name() -> Option<String> {
    use windows_sys::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let key = wide(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0");
    let value = wide("ProcessorNameString");
    let mut buf = [0u16; 256];
    let mut size = u32::try_from(std::mem::size_of_val(&buf)).unwrap_or(0);
    // SAFETY: the strings end in a 0, and `buf` holds `size` bytes, which the call writes at most.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if status != 0 {
        return None;
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let name = String::from_utf16_lossy(&buf[..n]).trim().to_string();
    (!name.is_empty()).then_some(name)
}

#[cfg(not(windows))]
fn cpu_name() -> Option<String> {
    None
}

/// The PC's overall CPU use over the next second, in percent (every core, every program).
#[cfg(windows)]
fn cpu_busy() -> Option<f64> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::GetSystemTimes;
    let read = || -> Option<[u64; 3]> {
        let mut t = [FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        }; 3];
        let [idle, kernel, user] = &mut t;
        // SAFETY: three FILETIMEs the call fills.
        let ok = unsafe { GetSystemTimes(idle, kernel, user) };
        (ok != 0).then(|| t.map(|f| (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime)))
    };
    let a = read()?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    let b = read()?;
    // the kernel time includes the idle time
    let idle = b[0].saturating_sub(a[0]) as f64;
    let all = (b[1].saturating_sub(a[1]) + b[2].saturating_sub(a[2])) as f64;
    (all > 0.0).then(|| (100.0 * (1.0 - idle / all)).clamp(0.0, 100.0))
}

#[cfg(not(windows))]
fn cpu_busy() -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_is_numpys() {
        // np.percentile([1, 2, 4, 8], q) for q = 0, 50, 95, 100
        let x = [8.0, 1.0, 4.0, 2.0];
        assert_eq!(percentile(&x, 0.0), 1.0);
        assert_eq!(percentile(&x, 50.0), 3.0);
        assert!((percentile(&x, 95.0) - 7.4).abs() < 1e-12);
        assert_eq!(percentile(&x, 100.0), 8.0);
        assert_eq!(percentile(&[5.0], 95.0), 5.0);
        assert!(percentile(&[], 95.0).is_nan());
    }
}
