//! The automatic fixes: when a video would fail one of its checks, the problem is put right where it is and the
//! render goes on, instead of stopping at the check.
//!
//! | Check | Fix |
//! | --- | --- |
//! | `focus` | the lens's sharp zone is widened on the frames that need it ([`bf_picture::ring::Picture::widen_focus`]) |
//! | `safezone` | the count-in (3, 2, 1, FIGHT!) is kept inside the safe zone and the checks run again ([`bf_picture::ring::Picture::keep_counts_inside`], in [`crate::job`]); anything still outside it is only noted, since the safe zone never stops a render |
//! | `band` | a fighter covered by the music is turned up there ([`lift_fighters`]) |
//! | `levels` | the loudness is evened out ([`crate::checks::even_out`]) |
//! | `loop_audio` | the end of the sound is brought to the level of the start and joined to it without a click ([`join_loop_audio`]) |
//! | `flash` | a frame that would make too many flashes is softened toward the one before ([`FlashGuard`]) |
//! | `loop_picture` | a one-off jump near the end is spread over a few frames, and the end is blended into the first frame ([`join_loop_picture`]) |
//!
//! Each fix leaves a video that passes already exactly as it was, and says in a plain sentence what it changed,
//! which is added to that check's summary. A check that still fails after its fix (or that cannot measure its
//! input at all, a fault in the app) still stops the render.

use std::collections::{BTreeMap, VecDeque};

use bf_core::dmath;
use bf_core::show::Show;
use bf_picture::{Gray32, Rgb8};
use bf_sound::master::{MASTER_CEILING, Soundtrack};

use crate::checks::evenout::limit;
use crate::checks::flash::{AREA, FPS, LIMIT, rel_lum};
use crate::checks::loopcheck::{HEAD_S as HEAD_S_PICTURE, TAIL_S as TAIL_S_PICTURE, steps};
use crate::checks::{BandPlan, CheckResult, Kept, SRF, as_wav16, band_of, loop_audio, loop_picture, median};
use bf_sound::polish::loud;

/// What a fix changed: the check it is for and a plain sentence.
#[derive(Clone, Debug, PartialEq)]
pub struct Fixed {
    /// The check (`band`, `levels`, ...).
    pub name: &'static str,
    /// What was changed, as a sentence.
    pub note: String,
}

impl Fixed {
    fn new(name: &'static str, note: String) -> Fixed {
        Fixed { name, note }
    }
}

/// `results` with each fix's sentence added to the summary of its check.
pub fn with_notes(mut results: Vec<CheckResult>, fixes: &[Fixed]) -> Vec<CheckResult> {
    for fix in fixes {
        for r in results.iter_mut().filter(|r| r.name == fix.name) {
            r.summary = format!("{} {}", r.summary, fix.note);
        }
    }
    results
}

// ---------------------------------------------------------------- the sound
/// Every fix of the sound, in order: the fighters lifted above the music ([`lift_fighters`]), the loudness evened
/// out ([`crate::checks::even_out`]), then the loop point joined ([`join_loop_audio`]). A soundtrack that passes
/// every sound check already is not touched.
pub fn fix_sound(st: &mut Soundtrack, show: &Show, plan: &BandPlan) -> Vec<Fixed> {
    let mut fixes = Vec::new();
    if let Some(note) = lift_fighters(st, show, plan) {
        fixes.push(Fixed::new("band", note));
    }
    if let Some(e) = crate::checks::even_out(&st.master) {
        fixes.push(Fixed::new("levels", e.note()));
        st.master = e.master;
    }
    if let Some(note) = join_loop_audio(&mut st.master) {
        fixes.push(Fixed::new("loop_audio", note));
    }
    // Under the Composed arrangement the loudness target comes last (sound upgrade, F1), so it holds whatever a
    // fix did; a soundtrack no fix touched is already on it.
    if plan.composed && !fixes.is_empty() {
        let mut stems: Vec<&mut Vec<[f64; 2]>> = st.stems.values_mut().collect();
        let r = loud::to_target(&mut st.master, &mut stems, loud::TARGET, loud::PEAK);
        fixes.push(Fixed::new(
            "loudness",
            format!(
                "After the fixes the soundtrack was brought back to {:.1} LUFS with its true peak at {:.1} dB.",
                r.lufs, r.true_peak
            ),
        ));
    }
    fixes
}

/// No fighter is lifted by more than this, in dB.
pub const LIFT_MOST_DB: f64 = 12.0;
/// A lifted fighter is aimed this far above the bed, in dB, so it passes with room to spare.
const LIFT_SPARE_DB: f64 = 1.5;
/// The lift eases in and out over this long on either side of its section, in seconds.
const LIFT_EASE_S: f64 = 0.25;

/// The sections of the band check: FIGHT! to 10 s, 10 to 20, 20 to 32, 32 to 44, 44 s to the K.O.
fn band_spans(show: &Show) -> [(f64, f64); 5] {
    [
        (show.grid.t_fight, 10.0),
        (10.0, 20.0),
        (20.0, 32.0),
        (32.0, 44.0),
        (44.0, show.t_ko),
    ]
}

/// Turns a fighter up in each section of the fight where the band check finds it under the music in its own
/// band, by what it lacks and a little more (at most [`LIFT_MOST_DB`]), easing in and out around the section.
/// Its stem and the master get the same lift, and a limiter keeps the master's peaks under its ceiling. Repeats
/// until the check passes (at most four rounds). `None` when the check passes already, or when nothing can be
/// lifted (a fighter that never sounds: the check then says why).
pub fn lift_fighters(st: &mut Soundtrack, show: &Show, plan: &BandPlan) -> Option<String> {
    let sounds = [
        (plan.sounds[0].0.as_str(), plan.sounds[0].1),
        (plan.sounds[1].0.as_str(), plan.sounds[1].1),
    ];
    let mut r = band_of(st, show, plan);
    if r.pass {
        return None;
    }
    // a check that could not measure (a fighter with no sound bus: it never placed a sound) has nothing to lift
    // and says why itself
    if r.numbers.is_empty() {
        return None;
    }
    let spans = band_spans(show);
    let original: Vec<Vec<[f64; 2]>> = sounds
        .iter()
        .map(|(bus, _)| st.stems.get(*bus).cloned())
        .collect::<Option<_>>()?;
    let master = st.master.clone();
    let mut lift = [[0.0f64; 5]; 2];
    for _ in 0..4 {
        let mut changed = false;
        for (k, (bus, _)) in sounds.iter().enumerate() {
            for (i, total) in lift[k].iter_mut().enumerate() {
                let Some(&g) = r.numbers.get(&format!("{bus} while it sounds {}", i + 1)) else {
                    continue;
                };
                let floor = if plan.composed { plan.floor } else { 0.0 };
                if g <= floor && *total < LIFT_MOST_DB {
                    *total = (*total + LIFT_SPARE_DB + floor - g).min(LIFT_MOST_DB);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
        let mut m = master.clone();
        for (k, (bus, _)) in sounds.iter().enumerate() {
            let gains = lift_gains(&lift[k], &spans, master.len());
            let stem: Vec<[f64; 2]> = original[k]
                .iter()
                .zip(&gains)
                .map(|(f, &g)| [f[0] * g, f[1] * g])
                .collect();
            for ((o, s), orig) in m.iter_mut().zip(&stem).zip(&original[k]) {
                o[0] += s[0] - orig[0];
                o[1] += s[1] - orig[1];
            }
            st.stems.insert((*bus).to_string(), stem);
        }
        st.master = limit(&m, MASTER_CEILING);
        r = band_of(st, show, plan);
        if r.pass {
            break;
        }
    }
    let mut parts = Vec::new();
    for (k, row) in lift.iter().enumerate() {
        let most = row.iter().copied().fold(0.0, f64::max);
        if most > 0.0 {
            let first = row.iter().position(|&l| l > 0.0).unwrap_or(0);
            let last = row.iter().rposition(|&l| l > 0.0).unwrap_or(first);
            parts.push(format!(
                "the {} by up to {most:.1} dB between {:.0} and {:.0} s",
                show.names[k], spans[first].0, spans[last].1
            ));
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(format!(
        "Where the music covered a fighter it was turned up automatically before the render ({}).",
        parts.join(", ")
    ))
}

/// The gain of every sample for the lifts `lift` (dB per section of `spans`): each section's lift is held over
/// it and eased in and out over [`LIFT_EASE_S`] on either side; where two overlap the larger counts.
fn lift_gains(lift: &[f64; 5], spans: &[(f64, f64); 5], n: usize) -> Vec<f64> {
    let mut db = vec![0.0f64; n];
    for (&l, &(a, b)) in lift.iter().zip(spans) {
        if l <= 0.0 || !(a.is_finite() && b.is_finite()) || b <= a {
            continue;
        }
        let i0 = ((a - LIFT_EASE_S) * SRF).max(0.0) as usize;
        let i1 = (((b + LIFT_EASE_S) * SRF).max(0.0) as usize).min(n);
        for (i, v) in db.iter_mut().enumerate().take(i1).skip(i0) {
            let t = i as f64 / SRF;
            let w = if t < a {
                ease((t - (a - LIFT_EASE_S)) / LIFT_EASE_S)
            } else if t > b {
                ease(((b + LIFT_EASE_S) - t) / LIFT_EASE_S)
            } else {
                1.0
            };
            *v = v.max(l * w);
        }
    }
    db.into_iter().map(|d| dmath::pow(10.0, d / 20.0)).collect()
}

/// A raised-cosine ease from 0 to 1 over `u` in 0 to 1.
fn ease(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    0.5 - 0.5 * dmath::cos(std::f64::consts::PI * u)
}

/// The end of the sound is moved to the level of the start over its last 0.3 s.
const TAIL_S: f64 = 0.3;
/// The click at the loop point is smoothed away over the last 10 ms.
const CLICK_S: f64 = 0.01;

/// Joins the end of the soundtrack to its start when the loop check of the sound would fail: the last 0.3 s
/// are brought to the level of the first 20 ms (at most 24 dB), and the last 10 ms are eased onto the first
/// sample so the loop point has no click. A limiter keeps the peaks under the ceiling. `None` when the check
/// passes already or cannot measure the soundtrack (it then says why).
pub fn join_loop_audio(master: &mut Vec<[f64; 2]>) -> Option<String> {
    let r = loop_audio(&as_wav16(master));
    if r.pass || !r.numbers.contains_key("fails") {
        return None;
    }
    let n = master.len();
    let mut parts = Vec::new();
    if let (Some(&la), Some(&lb)) = (r.numbers.get("last_20ms_db"), r.numbers.get("first_20ms_db"))
        && (la - lb).abs() > 6.0
    {
        let shift = (lb - la).clamp(-24.0, 24.0);
        let (t0, t1) = (
            n.saturating_sub((TAIL_S * SRF) as usize),
            n.saturating_sub((0.02 * SRF) as usize),
        );
        for (i, f) in master.iter_mut().enumerate().skip(t0) {
            let u = if t1 > t0 {
                (i - t0) as f64 / (t1 - t0) as f64
            } else {
                1.0
            };
            let g = dmath::pow(10.0, shift * ease(u) / 20.0);
            f[0] *= g;
            f[1] *= g;
        }
        *master = limit(master, MASTER_CEILING);
        parts.push(format!(
            "the last {:.1} s {} by {:.1} dB to meet the start",
            TAIL_S,
            if shift > 0.0 { "raised" } else { "lowered" },
            shift.abs()
        ));
    }
    let after = loop_audio(&as_wav16(master));
    if !after.pass && n > 1 {
        let first = master[0];
        let len = ((CLICK_S * SRF) as usize).min(n - 1);
        let start = n - len;
        let step = [first[0] - master[n - 1][0], first[1] - master[n - 1][1]];
        for (j, f) in master[start..].iter_mut().enumerate() {
            let w = ease((j + 1) as f64 / len as f64);
            for c in 0..2 {
                f[c] = (f[c] + step[c] * w).clamp(-MASTER_CEILING, MASTER_CEILING);
            }
        }
        parts.push(format!(
            "the last {:.0} ms eased onto the first sample",
            CLICK_S * 1000.0
        ));
    }
    if parts.is_empty() {
        return None;
    }
    Some(format!(
        "The loop point of the sound was joined automatically before the render ({}).",
        parts.join(", and ")
    ))
}

// ---------------------------------------------------------------- the picture
/// `a` mixed with `b`: `(1 - t) a + t b`, rounded. Pictures of other sizes give `a`.
pub fn mix(a: &Rgb8, b: &Rgb8, t: f64) -> Rgb8 {
    if (a.w, a.h, a.c) != (b.w, b.h, b.c) {
        return a.clone();
    }
    let t = t.clamp(0.0, 1.0);
    let data = a
        .data
        .iter()
        .zip(&b.data)
        .map(|(&x, &y)| (f64::from(x) * (1.0 - t) + f64::from(y) * t).round().clamp(0.0, 255.0) as u8)
        .collect();
    Rgb8::from_vec(a.w, a.h, a.c, data)
}

/// Joins the end of the video to its start when the loop check of the picture would fail. `tail` is the last
/// frames of the video (the check's last 5 s) and `head` its first second, as the check takes them; the fixed
/// frames are written back into `tail`.
///
/// A one-off jump in the tail (a cut that looks like the video starting over early) is spread over the frames
/// before it, each blended a little more into the frame after the jump, until no step is more than 3 times
/// normal. Then, if the last frame still does not flow into the first, the last frames are blended more and
/// more into frame 0 (6, 10, 15, 24, 36, 60, 90 or 120 of them, the fewest that pass). `None` when the check passes already
/// or cannot compare the frames (it then says why).
pub fn join_loop_picture(tail: &mut [Rgb8], head: &[Rgb8]) -> Option<String> {
    let r = loop_picture(tail, head);
    if r.pass || !r.numbers.contains_key("fails") || head.is_empty() || tail.len() < 2 {
        return None;
    }
    let mut parts = Vec::new();
    // the one-off jumps, spread out
    let st = steps(tail, head);
    let normal = median(&st).max(0.5);
    let ntail = tail.len() - 1;
    let big: Vec<(usize, f64)> = st
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s / normal))
        .filter(|&(_, q)| q > 4.0)
        .collect();
    let recurring = |q: f64| big.iter().filter(|&&(_, q2)| q / 1.6 <= q2 && q2 <= q * 1.6).count() >= 3;
    let jumps: Vec<(usize, f64)> = big
        .iter()
        .copied()
        .filter(|&(i, q)| i < ntail && q > 6.0 && !recurring(q))
        .collect();
    for &(i, q) in &jumps {
        let w = ((q / 3.0).ceil() as usize).clamp(2, 20).min(i + 1);
        let after = tail[i + 1].clone();
        for j in 1..=w {
            let at = i + 1 - j;
            tail[at] = mix(&tail[at], &after, (w + 1 - j) as f64 / (w + 1) as f64);
        }
        parts.push(format!(
            "a jump {:.2} s before the end spread over {w} frames",
            (tail.len() - i - 1) as f64 / FPS as f64
        ));
    }
    if !loop_picture(tail, head).pass {
        let before: Vec<Rgb8> = tail.to_vec();
        let n = tail.len();
        let mut used = 0;
        for m in [6usize, 10, 15, 24, 36, 60, 90, 120] {
            let m = m.min(n);
            tail.clone_from_slice(&before);
            for j in 0..m {
                let at = n - m + j;
                tail[at] = mix(&before[at], &head[0], (j + 1) as f64 / (m + 1) as f64);
            }
            used = m;
            if loop_picture(tail, head).pass {
                break;
            }
        }
        parts.push(format!("the last {used} frames blended into the first frame"));
    }
    Some(format!(
        "The loop point of the picture was joined automatically before the render ({}).",
        parts.join(", and ")
    ))
}

/// The picture fixes on the frames the checks drew (`kept`, from [`crate::checks::pre_render_keeping`]): the loop
/// point joined ([`join_loop_picture`]) when `loop_picture` failed, then the flashes softened ([`FlashGuard`], from
/// the first frame of the flash check's stretch `span`) when `flash` failed. The two checks are run again on the
/// fixed frames and their results replaced in `results`. Gives what was fixed and every frame that changed, by
/// number: the video must show those instead of drawing them again.
pub fn fix_frames(
    results: &mut [CheckResult],
    kept: &mut Kept,
    nf: usize,
    span: (usize, usize),
) -> (Vec<Fixed>, BTreeMap<usize, Rgb8>) {
    let mut fixes = Vec::new();
    let mut changed = BTreeMap::new();
    let failed = |results: &[CheckResult], name: &str| results.iter().any(|r| r.name == name && !r.pass);
    let frames = |kept: &Kept, range: std::ops::Range<usize>| -> Option<Vec<Rgb8>> {
        range.map(|f| kept.get(&f).map(|(img, _)| img.clone())).collect()
    };
    let put = |kept: &mut Kept, changed: &mut BTreeMap<usize, Rgb8>, f: usize, img: Rgb8| {
        if let Some(held) = kept.get_mut(&f)
            && held.0 != img
        {
            held.0 = img.clone();
            changed.insert(f, img);
        }
    };
    let fps = FPS;
    let nt = ((TAIL_S_PICTURE * fps as f64) as usize).min(nf);
    let nh = ((HEAD_S_PICTURE * fps as f64) as usize).min(nf);
    if failed(results, "loop_picture")
        && let (Some(mut tail), Some(head)) = (frames(kept, nf - nt..nf), frames(kept, 0..nh))
        && let Some(note) = join_loop_picture(&mut tail, &head)
    {
        let r = loop_picture(&tail, &head);
        for (j, img) in tail.into_iter().enumerate() {
            put(kept, &mut changed, nf - nt + j, img);
        }
        replace(results, r);
        fixes.push(Fixed::new("loop_picture", note));
    }
    let (f0, f1) = (span.0.min(nf), span.1.min(nf));
    if failed(results, "flash")
        && f0 < f1
        && let Some(run) = frames(kept, f0..f1)
    {
        let mut guard = FlashGuard::rule();
        let out: Vec<Rgb8> = run.into_iter().map(|img| guard.pass(img)).collect();
        let lum: Vec<Gray32> = out.iter().map(rel_lum).collect();
        replace(results, crate::checks::flash::flash_from(&lum, f0));
        for (j, img) in out.into_iter().enumerate() {
            put(kept, &mut changed, f0 + j, img);
        }
        fixes.push(Fixed::new(
            "flash",
            format!(
                "{} frames that flashed too often were softened automatically before the render.",
                guard.softened
            ),
        ));
    }
    (fixes, changed)
}

/// `results` with the result of the same check replaced by `r`.
fn replace(results: &mut [CheckResult], r: CheckResult) {
    if let Some(old) = results.iter_mut().find(|old| old.name == r.name) {
        *old = r;
    }
}

/// Keeps a run of frames under the flash rule as they are given, in order: a frame that would make a second hold
/// more than `limit` flashes (as [`crate::checks::flash`] counts them over the frames given so far) is softened
/// toward the frame before it (half, then more), so the flash becomes a fade. A run that keeps to the rule
/// already comes out exactly as it went in.
pub struct FlashGuard {
    limit: usize,
    /// The last five frames given out, with their luminance.
    last: VecDeque<(Rgb8, Gray32)>,
    /// The transitions so far (`(frame, sign)`, runs of one sign merged), as `analyse` makes them.
    events: Vec<(usize, i32)>,
    /// The next frame's number in the run.
    i: usize,
    /// The frames that were softened.
    pub softened: usize,
}

impl FlashGuard {
    /// A guard that allows `limit` flashes in any second ([`LIMIT`] is the rule).
    pub fn new(limit: usize) -> FlashGuard {
        FlashGuard {
            limit,
            last: VecDeque::with_capacity(6),
            events: Vec::new(),
            i: 0,
            softened: 0,
        }
    }

    /// A guard for the rule: [`LIMIT`] flashes in any second.
    pub fn rule() -> FlashGuard {
        FlashGuard::new(LIMIT)
    }

    /// The next frame as it is to be shown: `img` itself, or `img` softened toward the frame before.
    pub fn pass(&mut self, img: Rgb8) -> Rgb8 {
        let lum = rel_lum(&img);
        let (area, sign) = self.measure(&lum);
        if self.allows(area, sign) || self.last.is_empty() {
            return self.keep(img, lum, area, sign);
        }
        let prev = self.last.back().map(|(f, _)| f.clone()).unwrap_or_else(|| img.clone());
        self.softened += 1;
        for t in [0.5, 0.7, 0.85] {
            let soft = mix(&img, &prev, t);
            let l = rel_lum(&soft);
            let (a, s) = self.measure(&l);
            if self.allows(a, s) {
                return self.keep(soft, l, a, s);
            }
        }
        // held on the frame before: no new change at all against it
        let l = rel_lum(&prev);
        let (a, s) = self.measure(&l);
        self.keep(prev, l, a, s)
    }

    fn keep(&mut self, img: Rgb8, lum: Gray32, area: f64, sign: i32) -> Rgb8 {
        if area >= AREA {
            merge(&mut self.events, (self.i, sign));
        }
        self.i += 1;
        if self.last.len() == 5 {
            self.last.pop_front();
        }
        self.last.push_back((img.clone(), lum));
        img
    }

    /// `analyse`'s area and sign of a frame against the up to five frames before it.
    fn measure(&self, lum: &Gray32) -> (f64, i32) {
        let (mut best, mut sg) = (0.0f64, 0);
        for (_, then) in self.last.iter().rev() {
            let (now, then) = (&lum.data, &then.data);
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
        (best, sg)
    }

    /// Whether a frame with this area and sign keeps every second at or under the limit.
    fn allows(&self, area: f64, sign: i32) -> bool {
        if area < AREA {
            return true;
        }
        let mut ev = self.events.clone();
        merge(&mut ev, (self.i, sign));
        worst(&flashes(&ev)) <= self.limit
    }
}

/// `analyse`'s merge: a transition of the same sign as the last, at most 5 frames after it, replaces it.
fn merge(ev: &mut Vec<(usize, i32)>, e: (usize, i32)) {
    match ev.last_mut() {
        Some(last) if last.1 == e.1 && e.0 - last.0 <= 5 => *last = e,
        _ => ev.push(e),
    }
}

/// `analyse`'s flashes: pairs of opposing transitions.
fn flashes(ev: &[(usize, i32)]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut k = 0;
    while k + 1 < ev.len() {
        if ev[k].1 != ev[k + 1].1 {
            out.push(ev[k].0);
            k += 2;
        } else {
            k += 1;
        }
    }
    out
}

/// The most flashes that start within any one second.
fn worst(flashes: &[usize]) -> usize {
    flashes
        .iter()
        .map(|&a| flashes.iter().filter(|&&b| a <= b && b < a + FPS).count())
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::band;
    use crate::checks::flash;

    fn plain(v: u8) -> Rgb8 {
        Rgb8::filled(8, 8, &[v, v, v])
    }

    #[test]
    fn the_guard_leaves_a_calm_run_alone() {
        let mut g = FlashGuard::rule();
        for f in 0..90 {
            let img = plain((f % 40) as u8 + 60);
            assert_eq!(g.pass(img.clone()), img);
        }
        assert_eq!(g.softened, 0);
    }

    #[test]
    fn the_guard_softens_a_strobe_until_it_passes() {
        let frames: Vec<Rgb8> = (0..90).map(|f| plain(if f % 4 < 2 { 0 } else { 255 })).collect();
        let before: Vec<Gray32> = frames.iter().map(rel_lum).collect();
        assert!(!flash(&before).pass);
        let mut g = FlashGuard::rule();
        let out: Vec<Rgb8> = frames.into_iter().map(|f| g.pass(f)).collect();
        let after: Vec<Gray32> = out.iter().map(rel_lum).collect();
        let r = flash(&after);
        assert!(r.pass, "{}", r.summary);
        assert!(g.softened > 0);
    }

    #[test]
    fn a_seam_that_jumps_is_blended_into_frame_0() {
        // a slow drift that ends far from where it starts
        let mut tail: Vec<Rgb8> = (0..150).map(|f| plain(130 + (f / 10) as u8)).collect();
        let head: Vec<Rgb8> = (0..30).map(|f| plain(100 + (f / 10) as u8)).collect();
        assert!(!loop_picture(&tail, &head).pass);
        let note = join_loop_picture(&mut tail, &head).expect("a fix");
        let r = loop_picture(&tail, &head);
        assert!(r.pass, "{} / {note}", r.summary);
    }

    #[test]
    fn a_one_off_cut_is_spread_out() {
        let mut tail: Vec<Rgb8> = (0..150)
            .map(|f| plain(if f < 100 { 100 } else { 160 } + (f % 2) as u8))
            .collect();
        let head: Vec<Rgb8> = (0..30).map(|f| plain(160 + (f % 2) as u8)).collect();
        assert!(!loop_picture(&tail, &head).pass);
        join_loop_picture(&mut tail, &head).expect("a fix");
        let r = loop_picture(&tail, &head);
        assert!(r.pass, "{}", r.summary);
    }

    fn tone(n: usize, hz: f64, amp: f64) -> Vec<[f64; 2]> {
        (0..n)
            .map(|i| {
                let v = amp * dmath::sin(std::f64::consts::TAU * hz * i as f64 / SRF);
                [v, v]
            })
            .collect()
    }

    #[test]
    fn a_fighter_under_the_bed_is_lifted_until_it_passes() {
        let rec = bf_core::recording::Recording::read(&bf_testdata::reference("fights/flute_vs_trombone/ep392.bfrec"))
            .unwrap();
        let path = bf_testdata::repo_root().join("studio/fights/flute_vs_trombone.toml");
        let ff = bf_core::fightfile::FightFile::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
        let show = Show::new(&rec, &ff).unwrap();
        let n = 12 * 48_000;
        let mut stems = std::collections::BTreeMap::new();
        stems.insert("flute".to_string(), tone(n, 1000.0, 0.2));
        stems.insert("trombone".to_string(), tone(n, 200.0, 0.05));
        let bed: Vec<[f64; 2]> = tone(n, 1100.0, 0.1)
            .iter()
            .zip(tone(n, 220.0, 0.1))
            .map(|(a, b)| [a[0] + b[0], a[1] + b[1]])
            .collect();
        stems.insert("drums".to_string(), bed);
        let mut st = Soundtrack {
            master: vec![[0.0, 0.0]; n],
            stems,
        };
        let plan = BandPlan {
            sounds: [
                ("flute".to_string(), (600.0, 3200.0)),
                ("trombone".to_string(), (70.0, 700.0)),
            ],
            bed_buses: vec!["drums".to_string(), "fx".to_string()],
            composed: false,
            floor: 0.0,
        };
        let sounds = [("flute", (600.0, 3200.0)), ("trombone", (70.0, 700.0))];
        assert!(!band(&st, &show, sounds, &["drums", "fx"]).pass);
        let flute = st.stems["flute"].clone();
        let note = lift_fighters(&mut st, &show, &plan).expect("a lift");
        let r = band(&st, &show, sounds, &["drums", "fx"]);
        assert!(r.pass, "{} / {note}", r.summary);
        assert!(
            note.contains("the TROMBONE by up to") && !note.contains("FLUTE"),
            "{note}"
        );
        // the fighter that was above the bed is not touched, and the master got the same lift
        assert_eq!(st.stems["flute"], flute);
        assert!(st.master.iter().any(|f| f[0] != 0.0));
        // a passing soundtrack is left alone
        assert!(lift_fighters(&mut st, &show, &plan).is_none());
    }

    /// Code review of 2026-11-23, F7: a fighter with no sound bus (it never placed a sound) once made the lift
    /// panic; the band check's own sentence says why it fails instead.
    #[test]
    fn a_fighter_with_no_sound_bus_is_not_lifted() {
        let rec = bf_core::recording::Recording::read(&bf_testdata::reference("fights/flute_vs_trombone/ep392.bfrec"))
            .unwrap();
        let path = bf_testdata::repo_root().join("studio/fights/flute_vs_trombone.toml");
        let ff = bf_core::fightfile::FightFile::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
        let show = Show::new(&rec, &ff).unwrap();
        let n = 12 * 48_000;
        let mut stems = std::collections::BTreeMap::new();
        stems.insert("flute".to_string(), tone(n, 1000.0, 0.2));
        stems.insert("drums".to_string(), tone(n, 220.0, 0.1));
        let mut st = Soundtrack {
            master: vec![[0.0, 0.0]; n],
            stems,
        };
        let plan = BandPlan {
            sounds: [
                ("flute".to_string(), (600.0, 3200.0)),
                ("trombone".to_string(), (70.0, 700.0)),
            ],
            bed_buses: vec!["drums".to_string()],
            composed: false,
            floor: 0.0,
        };
        let sounds = [("flute", (600.0, 3200.0)), ("trombone", (70.0, 700.0))];
        assert!(!band(&st, &show, sounds, &["drums"]).pass);
        let before = st.clone();
        assert!(lift_fighters(&mut st, &show, &plan).is_none());
        assert_eq!(st.stems, before.stems);
        assert_eq!(st.master, before.master);
    }

    #[test]
    fn a_click_and_a_step_at_the_loop_point_are_joined() {
        let n = 48_000 * 2;
        let mut x: Vec<[f64; 2]> = (0..n)
            .map(|i| {
                let v = 0.3 * dmath::sin(std::f64::consts::TAU * 220.0 * i as f64 / SRF);
                let v = if i > n - 4_800 { v * 0.05 + 0.2 } else { v };
                [v, v]
            })
            .collect();
        assert!(!loop_audio(&as_wav16(&x)).pass);
        let note = join_loop_audio(&mut x).expect("a fix");
        let r = loop_audio(&as_wav16(&x));
        assert!(r.pass, "{} / {note}", r.summary);
        // a passing soundtrack is left alone
        let mut again = x.clone();
        assert!(join_loop_audio(&mut again).is_none());
        assert_eq!(again, x);
    }
}
