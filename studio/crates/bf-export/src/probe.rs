//! The checks of the finished file (`kit/checks.py verify()`): ffprobe's view of it, then the loop and the flash
//! checks on the frames and the sound decoded from it, as `reelkit.loopcheck` and `reelkit.flashcheck` run them
//! on an mp4.
//!
//! | Check (`name`) | Passes when |
//! | --- | --- |
//! | `mp4_size` | the file holds at least 64 KiB |
//! | `mp4_streams` | exactly one h264 video stream, 1080 x 1920 at 30 fps, and one aac stream, 48 kHz stereo |
//! | `mp4_keyframes` | the first and the last frame shown are keyframes (the loop seam decodes cleanly) |
//! | `mp4_length` | the frames decoded are the frames rendered, and the sound is the whole soundtrack (2,928,640 samples) |
//! | `mp4_flash` | [`crate::checks::flash`]'s rule on every decoded frame (streamed: five frames back at most) |
//! | `mp4_loop_picture` | [`crate::checks::loop_picture`] on the decoded last 5 s and first second |
//! | `mp4_loop_audio` | [`crate::checks::loop_audio`] on the decoded sound (mono, 16 bit, as the kit decodes it) |
//!
//! The decoded frames are scaled back to 360 x 640 with `flags=area`, as the kit does. Every name starts with
//! `mp4_`, so a check of the finished file is never taken for the same check before the render. A check that
//! cannot read the file fails with what ffmpeg or ffprobe said.

use std::collections::VecDeque;
use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bf_picture::config::{FPS, H, NF, UP, W};
use bf_picture::{Gray32, Rgb8};
use bf_sound::NA;

use crate::checks::flash::{AREA, LIMIT, rel_lum};
use crate::checks::loopcheck::{HEAD_S, TAIL_S};
use crate::checks::{CheckResult, loop_audio, loop_picture};
use crate::encode::{Ffmpeg, Tail, end, ended, run_tool, spawn};

/// The smallest file that can hold a video.
const MIN_BYTES: u64 = 64 * 1024;

/// How long ffprobe or a sound decode may take.
const PROBE_LIMIT: Duration = Duration::from_secs(120);

/// How long the decode of the frames may take (it takes about 5 s for a whole video on the reference PC).
const DECODE_LIMIT: Duration = Duration::from_secs(600);

/// A flag that is never set, for the calls that cannot be cancelled.
static NEVER: AtomicBool = AtomicBool::new(false);

/// Every check of a finished 61 s video at `mp4`, in the order of the module's table.
pub fn verify_mp4(ff: &Ffmpeg, mp4: &Path) -> Vec<CheckResult> {
    verify_until(ff, mp4, NF, true, &NEVER).unwrap_or_default()
}

/// The checks of a clip of `nf` frames: those of [`verify_mp4`] but the two loop checks (a clip does not loop).
pub fn verify_clip(ff: &Ffmpeg, mp4: &Path, nf: usize) -> Vec<CheckResult> {
    verify_until(ff, mp4, nf, false, &NEVER).unwrap_or_default()
}

/// The checks of a file of `nf` frames (`whole`: with the two loop checks); `None` once `cancel` is set (it is
/// looked at while the frames and the sound are decoded, which ffmpeg is then stopped for).
pub(crate) fn verify_until(
    ff: &Ffmpeg,
    mp4: &Path,
    nf: usize,
    whole: bool,
    cancel: &AtomicBool,
) -> Option<Vec<CheckResult>> {
    let mut out = vec![size(mp4)];
    out.push(streams(ff, mp4));
    out.push(keyframes(ff, mp4, nf));
    let picture = decode_picture(ff, mp4, cancel, DECODE_LIMIT);
    let sound = decode_sound(ff, mp4, cancel);
    if cancel.load(Ordering::SeqCst) {
        return None;
    }
    out.push(length(&picture, &sound, nf));
    out.push(match &picture {
        Ok(p) => p.flash.clone(),
        Err(why) => unreadable("mp4_flash", "Flashes in the finished video", why),
    });
    if whole {
        out.push(match &picture {
            Ok(p) => {
                let tail: Vec<Rgb8> = p.tail.iter().cloned().collect();
                renamed("mp4_loop_picture", loop_picture(&tail, &p.head))
            }
            Err(why) => unreadable("mp4_loop_picture", "Loop point of the finished video (picture)", why),
        });
        out.push(match &sound {
            Ok(x) => renamed("mp4_loop_audio", loop_audio(x)),
            Err(why) => unreadable("mp4_loop_audio", "Loop point of the finished video (sound)", why),
        });
    }
    Some(out)
}

fn renamed(name: &'static str, mut r: CheckResult) -> CheckResult {
    r.name = name;
    r
}

/// The failed result of a check that could not read the file.
fn unreadable(name: &'static str, title: &str, why: &str) -> CheckResult {
    CheckResult::failed(
        name,
        format!("{title}: the finished file could not be read ({why}), so it was not saved; render it again."),
    )
}

// ---------------------------------------------------------------- ffprobe's view
fn size(mp4: &Path) -> CheckResult {
    const TITLE: &str = "Size of the finished video";
    match std::fs::metadata(mp4) {
        Ok(m) if m.len() >= MIN_BYTES => CheckResult::new(
            "mp4_size",
            true,
            format!("{TITLE}: the file holds {:.1} MB.", m.len() as f64 / 1e6),
            &[("bytes", m.len() as f64)],
        ),
        Ok(m) => CheckResult::new(
            "mp4_size",
            false,
            format!(
                "{TITLE}: the file holds only {} bytes, too little for a video, so it was not saved; render it again.",
                m.len()
            ),
            &[("bytes", m.len() as f64)],
        ),
        Err(e) => unreadable("mp4_size", TITLE, &e.to_string()),
    }
}

/// `ffprobe -v error <args> <mp4>`'s output.
fn ffprobe(ff: &Ffmpeg, args: &[&str], mp4: &Path) -> Result<String, String> {
    let mut all: Vec<&OsStr> = vec!["-v".as_ref(), "error".as_ref()];
    all.extend(args.iter().map(OsStr::new));
    all.push(mp4.as_os_str());
    let ran = run_tool(&ff.ffprobe, &all, PROBE_LIMIT, &NEVER).map_err(|why| format!("ffprobe: {why}"))?;
    if !ran.status.success() {
        return Err(format!("ffprobe: {}", ran.said()));
    }
    Ok(String::from_utf8_lossy(&ran.stdout).into_owned())
}

/// The fields of one `-of compact=p=0` line: `key=value|key=value`.
fn fields(line: &str) -> Vec<(&str, &str)> {
    line.trim().split('|').filter_map(|kv| kv.split_once('=')).collect()
}

fn streams(ff: &Ffmpeg, mp4: &Path) -> CheckResult {
    const TITLE: &str = "Streams of the finished video";
    let text = match ffprobe(
        ff,
        &[
            "-show_entries",
            "stream=codec_type,codec_name,width,height,r_frame_rate,sample_rate,channels",
            "-of",
            "compact=p=0",
        ],
        mp4,
    ) {
        Ok(t) => t,
        Err(why) => return unreadable("mp4_streams", TITLE, &why),
    };
    let lines: Vec<Vec<(&str, &str)>> = text.lines().filter(|l| !l.trim().is_empty()).map(fields).collect();
    let get = |s: &[(&str, &str)], k: &str| s.iter().find(|(key, _)| *key == k).map(|(_, v)| v.to_string());
    let of = |kind: &str| {
        lines
            .iter()
            .filter(|s| get(s, "codec_type").as_deref() == Some(kind))
            .collect::<Vec<_>>()
    };
    let (video, audio) = (of("video"), of("audio"));
    let size = format!("{}", W * UP);
    let height = format!("{}", H * UP);
    let rate = format!("{FPS}/1");
    let mut wrong = Vec::new();
    if lines.len() != 2 || video.len() != 1 || audio.len() != 1 {
        wrong.push(format!(
            "it holds {} video and {} sound streams of {} where one of each is wanted",
            video.len(),
            audio.len(),
            lines.len()
        ));
    }
    if let Some(v) = video.first() {
        let want = [
            ("codec_name", "h264"),
            ("width", size.as_str()),
            ("height", height.as_str()),
            ("r_frame_rate", rate.as_str()),
        ];
        for (k, w) in want {
            let got = get(v, k).unwrap_or_default();
            if got != w {
                wrong.push(format!("the video's {k} is \"{got}\" where {w} is wanted"));
            }
        }
    }
    if let Some(a) = audio.first() {
        for (k, w) in [("codec_name", "aac"), ("sample_rate", "48000"), ("channels", "2")] {
            let got = get(a, k).unwrap_or_default();
            if got != w {
                wrong.push(format!("the sound's {k} is \"{got}\" where {w} is wanted"));
            }
        }
    }
    if wrong.is_empty() {
        CheckResult::new(
            "mp4_streams",
            true,
            format!("{TITLE}: h264 video at {size} x {height}, 30 fps, and aac sound at 48 kHz in stereo."),
            &[("streams", lines.len() as f64)],
        )
    } else {
        CheckResult::new(
            "mp4_streams",
            false,
            format!(
                "{TITLE}: {}, so it was not saved; check that ffmpeg is a full build, then render it again.",
                wrong.join("; ")
            ),
            &[("streams", lines.len() as f64)],
        )
    }
}

fn keyframes(ff: &Ffmpeg, mp4: &Path, nf: usize) -> CheckResult {
    const TITLE: &str = "Keyframes of the finished video";
    let text = match ffprobe(
        ff,
        &[
            "-select_streams",
            "v:0",
            "-show_entries",
            "packet=pts,flags",
            "-of",
            "csv=p=0",
        ],
        mp4,
    ) {
        Ok(t) => t,
        Err(why) => return unreadable("mp4_keyframes", TITLE, &why),
    };
    // the packets come in decoding order; the frames are shown in the order of their pts
    let mut frames: Vec<(i64, bool)> = text
        .lines()
        .filter_map(|l| {
            let (pts, flags) = l.trim().split_once(',')?;
            Some((pts.trim().parse().ok()?, flags.trim().starts_with('K')))
        })
        .collect();
    frames.sort_unstable();
    let keys = frames.iter().filter(|f| f.1).count();
    let numbers = [("packets", frames.len() as f64), ("keyframes", keys as f64)];
    match (frames.first(), frames.last()) {
        (Some(first), Some(last)) if first.1 && last.1 && frames.len() == nf => CheckResult::new(
            "mp4_keyframes",
            true,
            format!("{TITLE}: the first and the last of the {nf} frames are keyframes, so the loop seam is clean."),
            &numbers,
        ),
        _ => CheckResult::new(
            "mp4_keyframes",
            false,
            format!(
                "{TITLE}: of {} frames ({nf} were rendered) the first or the last is not a keyframe, so the loop \
                 seam would not be clean and the file was not saved; render it again.",
                frames.len()
            ),
            &numbers,
        ),
    }
}

// ---------------------------------------------------------------- the decoded frames and sound
/// What the frames decoded from a file gave.
struct Picture {
    frames: usize,
    flash: CheckResult,
    head: Vec<Rgb8>,
    tail: VecDeque<Rgb8>,
}

/// Decodes every frame at 360 x 640 (`scale=360:640:flags=area`, as `reelkit.flashcheck` does), streaming:
/// the flash rule sees each frame once, the first second and the last 5 s are kept for the loop check.
///
/// A watchdog thread ends ffmpeg once `cancel` is set or after `limit`, so a hung ffmpeg cannot hang the job.
fn decode_picture(ff: &Ffmpeg, mp4: &Path, cancel: &AtomicBool, limit: Duration) -> Result<Picture, String> {
    let scale = format!("scale={W}:{H}:flags=area");
    let args: [&OsStr; 11] = [
        "-v".as_ref(),
        "error".as_ref(),
        "-i".as_ref(),
        mp4.as_os_str(),
        "-vf".as_ref(),
        scale.as_ref(),
        "-f".as_ref(),
        "rawvideo".as_ref(),
        "-pix_fmt".as_ref(),
        "rgb24".as_ref(),
        "-".as_ref(),
    ];
    let mut child = spawn(&ff.ffmpeg, &args, Stdio::null()).map_err(|e| format!("ffmpeg could not be started: {e}"))?;
    let tail_lines = Tail::read(child.stderr.take().expect("piped"));
    let mut out = child.stdout.take().expect("piped");
    let child = Arc::new(Mutex::new(child));
    let finished = AtomicBool::new(false);
    let timed_out = AtomicBool::new(false);
    let nh = (HEAD_S * f64::from(FPS)) as usize;
    let nt = (TAIL_S * f64::from(FPS)) as usize;
    let mut scan = FlashScan::default();
    let mut head = Vec::new();
    let mut tail = VecDeque::new();
    let mut frames = 0usize;
    let mut buf = vec![0u8; W * H * 3];
    let read = std::thread::scope(|s| {
        s.spawn(|| {
            let t0 = Instant::now();
            while !finished.load(Ordering::SeqCst) {
                let over = t0.elapsed() > limit;
                if over || cancel.load(Ordering::SeqCst) {
                    timed_out.store(over, Ordering::SeqCst);
                    let _ = child.lock().unwrap_or_else(|e| e.into_inner()).kill();
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let read = loop {
            match read_full(&mut out, &mut buf) {
                Ok(0) => break Ok(()),
                Ok(n) if n == buf.len() => {
                    let img = Rgb8::from_vec(W, H, 3, buf.clone());
                    scan.push(rel_lum(&img));
                    if frames < nh {
                        head.push(img.clone());
                    }
                    tail.push_back(img);
                    if tail.len() > nt {
                        tail.pop_front();
                    }
                    frames += 1;
                }
                Ok(n) => break Err(format!("the last frame was cut short at {n} bytes")),
                Err(e) => break Err(format!("reading ffmpeg's frames failed: {e}")),
            }
        };
        finished.store(true, Ordering::SeqCst);
        read
    });
    drop(out);
    let status = end(&mut child.lock().unwrap_or_else(|e| e.into_inner()));
    let lines = tail_lines.finish();
    if timed_out.load(Ordering::SeqCst) {
        return Err(format!(
            "ffmpeg did not finish decoding it within {} s",
            limit.as_secs()
        ));
    }
    if cancel.load(Ordering::SeqCst) {
        return Err("the decode was stopped because the render was cancelled".to_string());
    }
    read?;
    match status {
        Some(s) if s.success() => {}
        Some(s) => return Err(format!("ffmpeg could not decode it: {}", ended(&s, &lines))),
        None => return Err("ffmpeg could not be waited for".to_string()),
    }
    Ok(Picture {
        frames,
        flash: scan.finish(),
        head,
        tail,
    })
}

/// Reads until `buf` is full or the stream ends; the bytes read.
fn read_full(r: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(n)
}

/// The sound as `reelkit.loopcheck` decodes it from an mp4 (`-vn -ac 1 -ar 48000`, 16 bit, divided by 32,768),
/// as two equal channels.
fn decode_sound(ff: &Ffmpeg, mp4: &Path, cancel: &AtomicBool) -> Result<Vec<[f64; 2]>, String> {
    let args: [&OsStr; 14] = [
        "-v".as_ref(),
        "error".as_ref(),
        "-i".as_ref(),
        mp4.as_os_str(),
        "-vn".as_ref(),
        "-ac".as_ref(),
        "1".as_ref(),
        "-ar".as_ref(),
        "48000".as_ref(),
        "-f".as_ref(),
        "s16le".as_ref(),
        "-acodec".as_ref(),
        "pcm_s16le".as_ref(),
        "-".as_ref(),
    ];
    let ran = run_tool(&ff.ffmpeg, &args, PROBE_LIMIT, cancel).map_err(|why| format!("ffmpeg: {why}"))?;
    if !ran.status.success() {
        return Err(format!("ffmpeg could not decode its sound: {}", ran.said()));
    }
    Ok(ran
        .stdout
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| {
            let m = f64::from(i16::from_le_bytes(*b)) / 32768.0;
            [m, m]
        })
        .collect())
}

fn length(picture: &Result<Picture, String>, sound: &Result<Vec<[f64; 2]>, String>, nf: usize) -> CheckResult {
    const TITLE: &str = "Length of the finished video";
    let (frames, samples) = match (picture, sound) {
        (Ok(p), Ok(x)) => (p.frames, x.len()),
        (Err(why), _) | (_, Err(why)) => return unreadable("mp4_length", TITLE, why),
    };
    let numbers = [("frames", frames as f64), ("samples", samples as f64)];
    let secs = |n: usize| n as f64 / 48_000.0;
    if frames == nf && samples == NA {
        CheckResult::new(
            "mp4_length",
            true,
            format!(
                "{TITLE}: {frames} frames ({:.2} s) and {:.3} s of sound, the whole soundtrack.",
                frames as f64 / f64::from(FPS),
                secs(samples)
            ),
            &numbers,
        )
    } else {
        CheckResult::new(
            "mp4_length",
            false,
            format!(
                "{TITLE}: it holds {frames} frames and {:.3} s of sound where {nf} frames and {:.3} s were made, \
                 so it was not saved; render it again.",
                secs(samples),
                secs(NA)
            ),
            &numbers,
        )
    }
}

// ---------------------------------------------------------------- the flash rule, streamed
/// `reelkit.flashcheck.analyse` over a whole video one frame at a time: the same rule as
/// [`crate::checks::flash::analyse`] (each frame against the five before it), keeping five planes, not all.
#[derive(Default)]
pub(crate) struct FlashScan {
    back: VecDeque<Gray32>,
    area: Vec<f64>,
    sign: Vec<i32>,
}

impl FlashScan {
    /// The next frame's luminance.
    pub fn push(&mut self, y: Gray32) {
        let (mut best, mut sg) = (0.0f64, 0);
        for k in 1..=self.back.len() {
            let then = &self.back[self.back.len() - k];
            let (now, then) = (&y.data, &then.data);
            if now.len() != then.len() || now.is_empty() {
                continue;
            }
            let (mut up, mut dn) = (0usize, 0usize);
            for (&a, &b) in now.iter().zip(then) {
                let d = a - b;
                let dark = a < 0.8f32 || b < 0.8f32;
                if d >= 0.1f32 && dark {
                    up += 1;
                }
                if d <= -0.1f32 && dark {
                    dn += 1;
                }
            }
            let (up, dn) = (up as f64 / now.len() as f64, dn as f64 / now.len() as f64);
            if up > best {
                (best, sg) = (up, 1);
            }
            if dn > best {
                (best, sg) = (dn, -1);
            }
        }
        self.area.push(best);
        self.sign.push(sg);
        self.back.push_back(y);
        if self.back.len() > 5 {
            self.back.pop_front();
        }
    }

    /// The flashes found and the most in any one second: `analyse`'s `(flashes, worst)`.
    pub fn flashes(&self) -> (Vec<usize>, usize) {
        let fps = FPS as usize;
        let mut ev: Vec<(usize, i32)> = Vec::new();
        for i in (0..self.area.len()).filter(|&i| self.area[i] >= AREA) {
            match ev.last_mut() {
                Some(last) if last.1 == self.sign[i] && i - last.0 <= 5 => *last = (i, self.sign[i]),
                _ => ev.push((i, self.sign[i])),
            }
        }
        let mut flashes = Vec::new();
        let mut k = 0;
        while k + 1 < ev.len() {
            if ev[k].1 != ev[k + 1].1 {
                flashes.push(ev[k].0);
                k += 2;
            } else {
                k += 1;
            }
        }
        let worst = flashes
            .iter()
            .map(|&a| flashes.iter().filter(|&&b| a <= b && b < a + fps).count())
            .max()
            .unwrap_or(0);
        (flashes, worst)
    }

    /// The result, with the numbers [`crate::checks::flash::flash`] gives.
    pub fn finish(self) -> CheckResult {
        const TITLE: &str = "Flashes in the finished video";
        if self.area.is_empty() {
            return CheckResult::failed(
                "mp4_flash",
                format!(
                    "{TITLE}: no frame could be decoded, so nothing was checked and it was not saved; render it again."
                ),
            );
        }
        let fps = FPS as usize;
        let (flashes, worst) = self.flashes();
        let mut i = 0;
        for (j, v) in self.area.iter().enumerate() {
            if *v > self.area[i] {
                i = j;
            }
        }
        let pass = worst <= LIMIT;
        let summary = if pass {
            format!(
                "{TITLE}: no second of the {} frames holds more than {LIMIT} full-screen flashes (the worst holds \
                 {worst}), and the largest share of the picture changing brightness at once is {:.1} % (at {:.2} s).",
                self.area.len(),
                100.0 * self.area[i],
                i as f64 / fps as f64
            )
        } else {
            let start = flashes
                .iter()
                .copied()
                .find(|&s| flashes.iter().filter(|&&b| s <= b && b < s + fps).count() == worst)
                .unwrap_or(0);
            format!(
                "{TITLE}: the second from {:.2} s holds {worst} flashes over a quarter of the picture or more where \
                 at most {LIMIT} are allowed, so it was not saved; soften or spread out the bright effects there.",
                start as f64 / fps as f64
            )
        };
        CheckResult::new(
            "mp4_flash",
            pass,
            summary,
            &[
                ("general_flashes", flashes.len() as f64),
                ("worst_in_1s", worst as f64),
                ("largest_area", self.area[i]),
                ("largest_area_frame", i as f64),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::flash::{analyse, flash};

    /// An 8 x 8 plane: its first `lit` pixels at `hi`, the rest at `lo`.
    fn plane(lit: usize, hi: f32, lo: f32) -> Gray32 {
        Gray32::from_vec(8, 8, 1, (0..64).map(|p| if p < lit { hi } else { lo }).collect())
    }

    #[test]
    fn the_streamed_rule_is_analyses() {
        // steady, then a strobe over half the frame, then a slow drift, then a strobe on a quarter, then odd values
        let mut planes = Vec::new();
        for f in 0..120usize {
            let p = match f {
                0..20 => plane(0, 0.0, 0.2),
                20..40 => plane(32, if f % 2 == 0 { 0.9 } else { 0.1 }, 0.2),
                40..60 => plane(64, 0.2 + 0.01 * (f - 40) as f32, 0.0),
                60..90 => plane(16, if f % 3 == 0 { 0.75 } else { 0.05 }, 0.3),
                _ => plane(f % 64, 0.85, 0.79),
            };
            planes.push(p);
        }
        let want = analyse(&planes, FPS as usize);
        let mut scan = FlashScan::default();
        for p in &planes {
            scan.push(p.clone());
        }
        assert_eq!(scan.area, want.area);
        assert_eq!(scan.flashes(), (want.flashes.clone(), want.worst));
        assert!(want.worst > LIMIT, "the strobe fails: {}", want.worst);
        let got = scan.finish();
        let kit = flash(&planes);
        assert_eq!((got.pass, &got.numbers), (kit.pass, &kit.numbers));
        assert!(!got.pass && got.summary.contains("0.67 s"), "{}", got.summary);
        assert!(kit.summary.contains("0.67 s"), "{}", kit.summary);

        // a calm run passes the same way
        let calm: Vec<Gray32> = (0..40).map(|f| plane(f, 0.5, 0.45)).collect();
        let mut scan = FlashScan::default();
        calm.iter().for_each(|p| scan.push(p.clone()));
        let (got, kit) = (scan.finish(), flash(&calm));
        assert_eq!((got.pass, &got.numbers), (kit.pass, &kit.numbers));
        assert!(got.pass);
    }

    #[test]
    fn compact_lines_are_split() {
        assert_eq!(
            fields("codec_name=h264|width=1080\r\n"),
            [("codec_name", "h264"), ("width", "1080")]
        );
    }
}
