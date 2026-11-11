//! Takes: `LoadTake`, the picture and the sound of one seed.
//!
//! **How a take is made.** `LoadTake` starts one worker thread. In order it makes the draft's fight file, fights
//! the seed with recording on, makes the show (a fight with no K.O. ends here, in words), reads the story and its
//! verdict, builds the picture and sends `TakePicture` (the story as the interface shows it and the engine's own
//! frames, [`PictureSource`]). Then it builds the soundtrack, runs the three sound checks (`levels`, `loudness`,
//! `band`, beside each other) and makes the lanes, and sends `TakeSound` with the master as `f32` and the Music
//! view. A fight that fits the story can still fail a check: that is a check line on Music, never a failed take.
//!
//! **The newest take wins.** A newer `LoadTake` silences every older take at once (its cancel flag): an older
//! worker stops before its next stage and sends nothing more. A soundtrack being built cannot be interrupted, so
//! one that is replaced is finished and thrown away. The new worker first waits (a millisecond at most) for any
//! older one that is in the middle of sending an event, so nothing of an older take ever arrives after the first
//! event of a newer one. Only one soundtrack is built at a time (a turn the workers pass on, never a lock held by
//! the handler): a replaced take that is still waiting for its turn gives up, so quick clicks through the seeds
//! never build several half-gigabyte soundtracks together. A new take may therefore wait for the replaced build
//! (about 4 s) before its own starts.
//!
//! **Kept takes.** The last [`KEEP`] finished takes are kept, keyed by the fight file's text, so going back to a
//! seed answers at once. A kept take holds the picture, the story view, the `f32` master and the Music view: the
//! `f64` master and the stems (about half a gigabyte) are dropped as soon as the checks and the lanes are made.
//!
//! **Failures.** Every failure is `TakeFailed` with a plain sentence. A panic on the worker is caught there and is
//! also `TakeFailed` ("Something went wrong while making the take, so it stopped. Try again."), unless a newer take
//! has replaced it. The engine's ending silences every take; a soundtrack being built then finishes by itself.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use bf_core::show::{Show, ShowError};
use bf_export::checks::{BandPlan, CheckResult, as_wav16, band, levels, loudness};
use bf_picture::ring::Picture;
use bf_sound::master::Soundtrack;
use bf_story::{BeatSheet, Candidate, Story};

use super::convert::{self, plain};
use super::search::story_view;
use super::{Ctx, panic_sentence};
use crate::model::TakeId;
use crate::model::draft::Draft;
use crate::model::story::StoryView;
use crate::model::take::{AudioTake, CheckLine, Frame, FrameSource, Lane, LaneGroup, MusicView};
use crate::seam::{Command, Event};
use crate::wording;

/// How many finished takes are kept.
pub const KEEP: usize = 3;
/// What a take's worker does, in the panic sentence.
const WHAT: &str = "making the take";
/// A beat is sounding when its RMS is above -50 dBFS: its mean square above (10 ^ (-50 / 20)) ^ 2.
const SOUNDING_MS: f64 = 1e-5;
/// How long a new take waits at most for an older one to finish sending an event.
const HUSH_WAIT: Duration = Duration::from_secs(1);

/// The frames of a take: the engine's picture, drawn on demand on any thread.
///
/// Each frame is `Picture::frame_reported`, exactly as the render job draws it, so the preview shows the frames the
/// video will hold. A frame whose report says a shape could not be drawn the way the reference does is a frame the
/// render refuses; the preview refuses it too, by panicking, which is how a frame source says a frame failed: the
/// preview's scheduler catches it, marks the frame failed and keeps its thread (widget rule 11). `Picture::frame`
/// is not used because it stops a debug build on such a frame and lets a release build show it.
pub struct PictureSource(pub Arc<Picture>);

impl FrameSource for PictureSource {
    fn frames(&self) -> u32 {
        u32::try_from(self.0.nf()).unwrap_or(u32::MAX)
    }

    fn draw(&self, f: u32) -> Frame {
        let (img, _, report) = self.0.frame_reported(f as usize);
        assert!(
            report.hull_errors == 0,
            "frame {f}: {} of its shapes could not be drawn the way the reference does",
            report.hull_errors
        );
        Frame {
            w: u32::try_from(img.w).unwrap_or(u32::MAX),
            h: u32::try_from(img.h).unwrap_or(u32::MAX),
            rgb: img.data.into(),
        }
    }
}

/// Where a stem sounds, in seconds: the beats (`beat_samples` long, the last one possibly shorter) whose RMS over
/// both channels is above -50 dBFS, with neighbouring beats joined into one stretch.
pub fn sounding(stem: &[[f64; 2]], beat_samples: usize) -> Vec<(f32, f32)> {
    if beat_samples == 0 {
        return Vec::new();
    }
    let sr = bf_sound::SR as f64;
    let at = |sample: usize| (sample as f64 / sr) as f32;
    let mut out = Vec::new();
    let mut open: Option<usize> = None;
    for (i, beat) in stem.chunks(beat_samples).enumerate() {
        let start = i * beat_samples;
        let sum: f64 = beat.iter().map(|f| f[0] * f[0] + f[1] * f[1]).sum();
        // a NaN is not sounding
        let loud = sum / (2 * beat.len()) as f64 > SOUNDING_MS;
        match (loud, open) {
            (true, None) => open = Some(start),
            (false, Some(from)) => {
                out.push((at(from), at(start)));
                open = None;
            }
            _ => {}
        }
    }
    if let Some(from) = open {
        out.push((at(from), at(stem.len())));
    }
    out
}

/// A stem's name as shown: sentence case, an underscore read as a space ("band_hits" -> "Band hits").
fn stem_name(stem: &str) -> String {
    wording::display_name(&stem.replace('_', " "))
}

/// The Music screen's lanes: one per stem. A stem named after one of the two fighters (`names`, compared without
/// case) is a `Fighters` lane with that fighter's colour and its name as shown, the fighters first in their order;
/// every other stem is a `Band` lane, named in sentence case. `beat` is the beat in seconds: where a stem sounds is
/// read beat by beat ([`sounding`]).
pub fn lanes(stems: &BTreeMap<String, Vec<[f64; 2]>>, names: [&str; 2], colors: [[u8; 3]; 2], beat: f64) -> Vec<Lane> {
    let beat_samples = {
        let n = (beat * bf_sound::SR as f64).round();
        if n.is_finite() && n >= 1.0 { n as usize } else { 0 }
    };
    let fighter = |stem: &str| names.iter().position(|n| n.to_lowercase() == stem.to_lowercase());
    let mut fighters: Vec<(usize, Lane)> = Vec::new();
    let mut band = Vec::new();
    for (stem, samples) in stems {
        let clips = sounding(samples, beat_samples);
        match fighter(stem) {
            Some(k) => fighters.push((
                k,
                Lane {
                    group: LaneGroup::Fighters,
                    name: wording::display_name(names[k]),
                    color: Some(colors[k]),
                    clips,
                },
            )),
            None => band.push(Lane {
                group: LaneGroup::Band,
                name: stem_name(stem),
                color: None,
                clips,
            }),
        }
    }
    fighters.sort_by_key(|(k, _)| *k);
    fighters.into_iter().map(|(_, lane)| lane).chain(band).collect()
}

/// The fight's moments on the Music screen, in time order: the title card at 0, "Fight" at FIGHT!, each lead
/// change, each fighter's first desperation move (the first use of its low ability), the K.O. and the reset. A
/// time that is not a finite, non-negative number is left out.
pub fn markers(story: &Story, show: &Show) -> Vec<(f32, String)> {
    let mut m: Vec<(f64, &str)> = vec![(0.0, "Title card"), (show.grid.t_fight, "Fight")];
    m.extend(story.lc_t.iter().map(|&t| (t, "Lead change")));
    for (k, name) in show.names.iter().enumerate() {
        let low = bf_content::roster()
            .iter()
            .find(|f| &f.name == name)
            .map(|f| f.low.name.as_str());
        let first = story
            .firsts
            .iter()
            .find(|(_, who, ability)| *who == k && Some(ability.as_str()) == low);
        if let Some(&(t, _, _)) = first {
            m.push((t, "Desperation"));
        }
    }
    m.push((show.t_ko, "K.O."));
    m.push((show.grid.t_reassemble, "Reset"));
    m.retain(|(t, _)| t.is_finite() && *t >= 0.0);
    m.sort_by(|a, b| a.0.total_cmp(&b.0));
    m.into_iter().map(|(t, label)| (t as f32, label.to_string())).collect()
}

/// The title of a check's line on Music.
fn check_title(name: &str) -> String {
    match name {
        "levels" => "Loudness".to_string(),
        "loudness" => "Perceived loudness".to_string(),
        "band" => "Fighters above the music".to_string(),
        other => stem_name(other),
    }
}

/// The check lines of Music: "Loudness" for `levels`, "Perceived loudness" for `loudness`, "Fighters above the
/// music" for `band`, in the order given; each note is the check's own sentence.
pub fn check_lines(results: &[CheckResult]) -> Vec<CheckLine> {
    results
        .iter()
        .map(|r| CheckLine {
            title: check_title(r.name),
            pass: r.pass,
            note: r.summary.clone(),
        })
        .collect()
}

/// A lock that a panic elsewhere cannot poison for good.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// A finished take, as it is kept.
#[derive(Clone)]
struct Kept {
    /// The fight file's text.
    key: String,
    story: StoryView,
    frames: Arc<dyn FrameSource>,
    audio: AudioTake,
    music: MusicView,
}

/// Puts `take` first in `kept`, replacing one of the same key, and returns what no longer fits (to be dropped
/// outside the lock: a picture is large).
fn keep(kept: &mut Vec<Kept>, take: Kept) -> Vec<Kept> {
    kept.retain(|k| k.key != take.key);
    kept.insert(0, take);
    if kept.len() > KEEP {
        kept.split_off(KEEP)
    } else {
        Vec::new()
    }
}

/// The kept take of `key`, moved first.
fn kept_take(kept: &mut Vec<Kept>, key: &str) -> Option<Kept> {
    let i = kept.iter().position(|k| k.key == key)?;
    let take = kept.remove(i);
    kept.insert(0, take.clone());
    Some(take)
}

/// One take's right to send: silenced by a newer take or the engine's ending. While it is sending, `speaking` is
/// set, so a newer take can wait until it is done.
struct Voice {
    quiet: Arc<AtomicBool>,
    speaking: AtomicBool,
}

/// Clears `speaking` when the send is over, even by a panic.
struct Said<'a>(&'a AtomicBool);

impl Drop for Said<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl Voice {
    /// Whether this take may still send.
    fn heard(&self, ctx: &Ctx) -> bool {
        !self.quiet.load(Ordering::SeqCst) && !ctx.stopping()
    }

    /// Sends `e` unless the take has been silenced; whether it was sent. `speaking` is set before the flag is read
    /// (both sequentially consistent), so a newer take that set the flag and then sees `speaking` clear knows this
    /// take will send nothing more.
    fn say(&self, ctx: &Ctx, e: Event) -> bool {
        self.speaking.store(true, Ordering::SeqCst);
        let _said = Said(&self.speaking);
        if self.heard(ctx) {
            ctx.emit(e);
            true
        } else {
            false
        }
    }
}

/// The turn to build a soundtrack: one at a time.
struct SoundTurn<'a>(&'a AtomicBool);

impl SoundTurn<'_> {
    /// Waits for the turn; `None` once the take is silenced while it waits.
    fn take<'a>(turn: &'a AtomicBool, me: &Voice, ctx: &Ctx) -> Option<SoundTurn<'a>> {
        loop {
            if !me.heard(ctx) {
                return None;
            }
            if turn
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return Some(SoundTurn(turn));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for SoundTurn<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// What this handler keeps between commands: the takes still running (to silence them), the kept takes and the
/// turn to build a soundtrack.
#[derive(Default)]
pub struct State {
    voices: Vec<Arc<Voice>>,
    kept: Arc<Mutex<Vec<Kept>>>,
    sound_turn: Arc<AtomicBool>,
}

/// Takes one command and returns at once; the take is made on its own thread.
pub fn handle(state: &mut State, ctx: &Ctx, command: Command) {
    let Command::LoadTake { take, draft } = command else {
        return;
    };
    // A voice whose worker has ended is held by this state alone (newer workers may hold it a while longer).
    state.voices.retain(|v| Arc::strong_count(v) > 1);
    for v in &state.voices {
        v.quiet.store(true, Ordering::SeqCst);
    }
    let voice = Arc::new(Voice {
        quiet: ctx.cancel_flag(),
        speaking: AtomicBool::new(false),
    });
    let worker = Worker {
        take,
        me: voice.clone(),
        older: state.voices.clone(),
        kept: state.kept.clone(),
        sound_turn: state.sound_turn.clone(),
    };
    if ctx.spawn(
        "engine: take",
        WHAT,
        move |ctx| worker.run(&ctx, &draft),
        move |problem| Event::TakeFailed { take, problem },
    ) {
        state.voices.push(voice);
    }
}

/// Tests: a take with this number panics as it starts.
#[cfg(test)]
const PANICS: TakeId = 9_999;
/// Tests: a take with this number panics once a newer take has replaced it.
#[cfg(test)]
const PANICS_REPLACED: TakeId = 9_998;

/// One take's worker.
struct Worker {
    take: TakeId,
    me: Arc<Voice>,
    /// The takes before it, already silenced.
    older: Vec<Arc<Voice>>,
    kept: Arc<Mutex<Vec<Kept>>>,
    sound_turn: Arc<AtomicBool>,
}

impl Worker {
    fn run(&self, ctx: &Ctx, draft: &Draft) {
        let deadline = Instant::now() + HUSH_WAIT;
        for v in &self.older {
            while v.speaking.load(Ordering::SeqCst) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        let fail = |problem: String| {
            self.me.say(
                ctx,
                Event::TakeFailed {
                    take: self.take,
                    problem,
                },
            )
        };
        match catch_unwind(AssertUnwindSafe(|| self.make(ctx, draft))) {
            Ok(Ok(())) => {}
            Ok(Err(problem)) => {
                fail(problem);
            }
            Err(_) => {
                fail(panic_sentence(WHAT));
            }
        }
    }

    /// Makes the take, sending its events; the words of a failure. Returns quietly once the take is silenced.
    fn make(&self, ctx: &Ctx, draft: &Draft) -> Result<(), String> {
        let (me, take) = (&*self.me, self.take);
        #[cfg(test)]
        {
            if take == PANICS {
                panic!("on purpose");
            }
            if take == PANICS_REPLACED {
                let deadline = Instant::now() + Duration::from_secs(10);
                while me.heard(ctx) && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(2));
                }
                panic!("on purpose, after it was replaced");
            }
        }
        if !me.heard(ctx) {
            return Ok(());
        }
        let ff = convert::fight_file(draft)?;
        let Some(seed) = ff.seed else {
            return Err("Pick a seed first.".to_string());
        };
        let key = ff.to_toml();
        if let Some(k) = kept_take(&mut lock(&self.kept), &key) {
            if me.say(
                ctx,
                Event::TakePicture {
                    take,
                    story: k.story,
                    frames: k.frames,
                },
            ) {
                me.say(
                    ctx,
                    Event::TakeSound {
                        take,
                        audio: k.audio,
                        music: k.music,
                    },
                );
            }
            return Ok(());
        }

        // the fight, its show and its story
        let setup = bf_sim::setup_from(&ff, bf_content::roster(), seed)
            .map_err(|e| plain(&format!("This fight cannot be fought: {e}")))?;
        if !me.heard(ctx) {
            return Ok(());
        }
        let rec = bf_sim::run(&setup, true).map_err(|e| plain(&format!("Seed {seed} cannot be fought: {e}")))?;
        let show = Show::new(&rec, &ff).map_err(|e| match e {
            ShowError::NoKo => {
                format!("Seed {seed} has no K.O. before 58 s, so it cannot be made into a video. Pick another seed.")
            }
            other => plain(&other),
        })?;
        let sheet = BeatSheet::series(setup.fighters);
        let lows = [setup.fighters[0].low.name.as_str(), setup.fighters[1].low.name.as_str()];
        let story = bf_story::story(&rec, &show.grid, &sheet, lows);
        let verdict = bf_story::score(&story, &sheet);
        let candidate = Candidate { seed, story, verdict };
        let view = story_view(&candidate, show.grid.t_fight);
        let colors = setup.fighters.map(|f| f.color);
        if !me.heard(ctx) {
            return Ok(());
        }

        // the picture
        let rec = Arc::new(rec);
        let picture = bf_content::build_picture(Arc::clone(&rec), &ff).map_err(|e| plain(&e))?;
        let frames: Arc<dyn FrameSource> = Arc::new(PictureSource(Arc::new(picture)));
        let sent = me.say(
            ctx,
            Event::TakePicture {
                take,
                story: view.clone(),
                frames: frames.clone(),
            },
        );
        if !sent {
            return Ok(());
        }

        // the sound: one soundtrack at a time, its f64 buffers dropped as soon as the checks and lanes are made
        let Some(turn) = SoundTurn::take(&self.sound_turn, me, ctx) else {
            return Ok(());
        };
        let st = bf_content::build_soundtrack(&rec, &ff).map_err(|e| plain(&e))?;
        if !me.heard(ctx) {
            // finished, and thrown away
            return Ok(());
        }
        let names = [show.names[0].as_str(), show.names[1].as_str()];
        let (checks, lanes) = sound_views(&st, &show, &ff, names, colors);
        let audio = AudioTake {
            samples: st.master.iter().map(|f| [f[0] as f32, f[1] as f32]).collect(),
        };
        drop(st);
        drop(turn);
        let music = MusicView {
            checks,
            lanes,
            markers: markers(&candidate.story, &show),
        };
        if !me.heard(ctx) {
            return Ok(());
        }
        let gone = keep(
            &mut lock(&self.kept),
            Kept {
                key,
                story: view,
                frames,
                audio: audio.clone(),
                music: music.clone(),
            },
        );
        drop(gone);
        me.say(ctx, Event::TakeSound { take, audio, music });
        Ok(())
    }
}

/// The check lines and the lanes of a soundtrack: `band` (the slowest, about 3 s), `levels` with `loudness`, and
/// the lanes run beside each other. The sound checks read the master as its 16-bit file holds it, as
/// `bf_export::checks::pre_render` does.
fn sound_views(
    st: &Soundtrack,
    show: &Show,
    ff: &bf_core::fightfile::FightFile,
    names: [&str; 2],
    colors: [[u8; 3]; 2],
) -> (Vec<CheckLine>, Vec<Lane>) {
    let plan = BandPlan::of(ff);
    std::thread::scope(|s| {
        let band_r = s.spawn(|| match &plan {
            Ok(plan) => {
                let sounds = [
                    (plan.sounds[0].0.as_str(), plan.sounds[0].1),
                    (plan.sounds[1].0.as_str(), plan.sounds[1].1),
                ];
                let bed: Vec<&str> = plan.bed_buses.iter().map(String::as_str).collect();
                band(st, show, sounds, &bed)
            }
            Err(e) => CheckResult {
                name: "band",
                pass: false,
                summary: plain(&format!("The fighters could not be measured against the music: {e}")),
                numbers: BTreeMap::new(),
            },
        });
        let loud_r = s.spawn(|| {
            let master = as_wav16(&st.master);
            [levels(&master), loudness(&master, show)]
        });
        let lanes = lanes(&st.stems, names, colors, show.grid.beat);
        let [levels_r, loudness_r] = loud_r.join().unwrap_or_else(|p| std::panic::resume_unwind(p));
        let band_r = band_r.join().unwrap_or_else(|p| std::panic::resume_unwind(p));
        (check_lines(&[levels_r, loudness_r, band_r]), lanes)
    })
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::engine::tests::starting_draft;
    use crate::engine::{test_engine, wait_for};
    use crate::seam::EngineHandle;

    /// The starting draft under the kit's handicap for this pair, 2.0 and 0.4, with `seed`.
    fn draft(seed: Option<u64>) -> Draft {
        let mut d = starting_draft();
        d.handicap = [2.0, 0.4];
        d.seed = seed;
        d
    }

    /// The first seeds (from 1) whose fight under `d` has a K.O. (`ko`) or has none (`!ko`): a short search.
    fn seeds_where(d: &Draft, ko: bool, n: usize) -> Vec<u64> {
        let ff = convert::fight_file(d).expect("a fight file");
        let base = bf_sim::setup_from(&ff, bf_content::roster(), 0).expect("a set-up");
        let sheet = BeatSheet::series(base.fighters);
        let mut found = Vec::new();
        for seed in 1..2_000 {
            let c = bf_story::fight(&bf_sim::Setup { seed, ..base.clone() }, &sheet).expect("a fight");
            if c.story.winner.is_some() == ko {
                found.push(seed);
                if found.len() == n {
                    return found;
                }
            }
        }
        panic!("no such seed under 2,000");
    }

    fn load(engine: &EngineHandle, take: TakeId, d: &Draft) {
        engine.send(Command::LoadTake { take, draft: d.clone() });
    }

    /// The take an event is about.
    fn take_of(e: &Event) -> Option<TakeId> {
        match e {
            Event::TakePicture { take, .. } | Event::TakeSound { take, .. } | Event::TakeFailed { take, .. } => {
                Some(*take)
            }
            _ => None,
        }
    }

    /// Every take event that arrives within `seconds`, or until `stop` accepts one (included).
    fn gather(engine: &EngineHandle, seconds: f64, mut stop: impl FnMut(&Event) -> bool) -> Vec<Event> {
        let deadline = Instant::now() + Duration::from_secs_f64(seconds);
        let mut out = Vec::new();
        while Instant::now() < deadline {
            while let Some(e) = engine.poll() {
                if take_of(&e).is_some() {
                    let done = stop(&e);
                    out.push(e);
                    if done {
                        return out;
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        out
    }

    fn close(engine: EngineHandle, root: std::path::PathBuf) {
        engine.send(Command::Shutdown);
        drop(engine);
        _ = std::fs::remove_dir_all(root);
    }

    // ------------------------------------------------------------ units

    #[test]
    fn sounding_joins_neighbouring_beats() {
        let mut stem = vec![[0.0; 2]; 1_600];
        for (i, f) in stem.iter_mut().enumerate().take(800).skip(400) {
            let v = 0.1 * (2.0 * PI * 440.0 * i as f64 / 48_000.0).sin();
            *f = [v, v];
        }
        assert_eq!(
            sounding(&stem, 100),
            vec![((400.0 / 48_000.0) as f32, (800.0 / 48_000.0) as f32)]
        );
        // a quiet beat in between splits it; a loud last beat runs to the end
        let mut split = stem.clone();
        for f in &mut split[500..600] {
            *f = [0.0, 0.0];
        }
        split[1_550] = [0.5, 0.5];
        let at = |s: f64| (s / 48_000.0) as f32;
        assert_eq!(
            sounding(&split, 100),
            vec![
                (at(400.0), at(500.0)),
                (at(600.0), at(800.0)),
                (at(1_500.0), at(1_600.0))
            ]
        );
        // under -50 dBFS is silence; nothing at all, or a beat of 0 samples, sounds nowhere
        let hush = vec![[0.003, 0.003]; 400];
        assert!(sounding(&hush, 100).is_empty());
        assert_eq!(sounding(&vec![[0.0032, 0.0032]; 400], 100), vec![(0.0, at(400.0))]);
        assert!(sounding(&[], 100).is_empty());
        assert!(sounding(&stem, 0).is_empty());
        let nan = vec![[f64::NAN, 0.0]; 200];
        assert!(sounding(&nan, 100).is_empty());
    }

    #[test]
    fn a_fighters_stem_is_a_fighter_lane() {
        let loud = vec![[0.2, 0.2]; 9_600];
        let quiet = vec![[0.0, 0.0]; 9_600];
        let stems: BTreeMap<String, Vec<[f64; 2]>> = [
            ("band_hits", &loud),
            ("calliope", &quiet),
            ("flute", &loud),
            ("Trombone", &loud),
        ]
        .into_iter()
        .map(|(n, s)| (n.to_string(), s.clone()))
        .collect();
        let got = lanes(&stems, ["TROMBONE", "FLUTE"], [[1, 2, 3], [4, 5, 6]], 0.05);
        let shown: Vec<(LaneGroup, &str, Option<[u8; 3]>)> =
            got.iter().map(|l| (l.group, l.name.as_str(), l.color)).collect();
        assert_eq!(
            shown,
            [
                (LaneGroup::Fighters, "Trombone", Some([1, 2, 3])),
                (LaneGroup::Fighters, "Flute", Some([4, 5, 6])),
                (LaneGroup::Band, "Band hits", None),
                (LaneGroup::Band, "Calliope", None),
            ]
        );
        assert_eq!(got[0].clips, vec![(0.0, 0.2)]);
        assert!(got[3].clips.is_empty());
        // a beat that is no number of samples gives no clips, not a panic
        assert!(
            lanes(&stems, ["TROMBONE", "FLUTE"], [[0; 3]; 2], f64::NAN)
                .iter()
                .all(|l| l.clips.is_empty())
        );
    }

    #[test]
    fn check_lines_are_named_for_music() {
        let r = |name: &'static str, pass: bool| CheckResult {
            name,
            pass,
            summary: format!("{name} said this."),
            numbers: BTreeMap::new(),
        };
        let lines = check_lines(&[r("levels", false), r("loudness", true), r("band", true)]);
        assert_eq!(
            lines,
            [
                CheckLine {
                    title: "Loudness".to_string(),
                    pass: false,
                    note: "levels said this.".to_string(),
                },
                CheckLine {
                    title: "Perceived loudness".to_string(),
                    pass: true,
                    note: "loudness said this.".to_string(),
                },
                CheckLine {
                    title: "Fighters above the music".to_string(),
                    pass: true,
                    note: "band said this.".to_string(),
                },
            ]
        );
    }

    /// A dummy frame source for the kept takes.
    struct Blank;

    impl FrameSource for Blank {
        fn frames(&self) -> u32 {
            1
        }

        fn draw(&self, _: u32) -> Frame {
            Frame {
                w: 1,
                h: 1,
                rgb: Arc::from(vec![0u8; 3]),
            }
        }
    }

    fn kept(key: &str) -> Kept {
        Kept {
            key: key.to_string(),
            story: crate::seam::sample::story(1),
            frames: Arc::new(Blank),
            audio: AudioTake {
                samples: Arc::from(Vec::new()),
            },
            music: crate::seam::sample::music(),
        }
    }

    #[test]
    fn the_last_three_takes_are_kept() {
        let mut list = Vec::new();
        for key in ["a", "b", "c"] {
            assert!(keep(&mut list, kept(key)).is_empty());
        }
        let keys = |list: &Vec<Kept>| list.iter().map(|k| k.key.clone()).collect::<Vec<_>>();
        assert_eq!(keys(&list), ["c", "b", "a"]);
        // going back moves a take first
        assert_eq!(kept_take(&mut list, "a").map(|k| k.key), Some("a".to_string()));
        assert_eq!(keys(&list), ["a", "c", "b"]);
        assert!(kept_take(&mut list, "z").is_none());
        // a fourth pushes out the oldest; the same key again replaces itself
        let gone = keep(&mut list, kept("d"));
        assert_eq!(keys(&gone), ["b"]);
        assert_eq!(keys(&list), ["d", "a", "c"]);
        assert!(keep(&mut list, kept("c")).is_empty());
        assert_eq!(keys(&list), ["c", "d", "a"]);
    }

    #[test]
    fn markers_follow_the_story() {
        let d = draft(None);
        let seed = seeds_where(&d, true, 1)[0];
        let ff = convert::fight_file(&draft(Some(seed))).expect("a fight file");
        let setup = bf_sim::setup_from(&ff, bf_content::roster(), seed).expect("a set-up");
        let rec = bf_sim::run(&setup, false).expect("the fight");
        let show = Show::new(&rec, &ff).expect("a show");
        let lows = [setup.fighters[0].low.name.clone(), setup.fighters[1].low.name.clone()];
        let story = Story {
            seed,
            winner: Some(show.win),
            t_ko: Some(show.t_ko),
            lead_changes: 2,
            lc_t: vec![30.0, 20.0],
            winner_hp: 10.0,
            firsts: vec![
                (12.0, 1, "SOMETHING ELSE".to_string()),
                (25.0, 1, lows[1].clone()),
                (37.0, 1, lows[1].clone()),
                // fighter 1's low, but used by fighter 0: not fighter 0's desperation
                (26.0, 0, lows[1].clone()),
                (38.5, 0, lows[0].clone()),
            ],
            caps: Vec::new(),
            max_gap: 3.0,
            lows: Vec::new(),
            lead: Vec::new(),
        };
        let m = markers(&story, &show);
        let want: Vec<(f32, String)> = [
            (0.0, "Title card"),
            (show.grid.t_fight, "Fight"),
            (20.0, "Lead change"),
            (25.0, "Desperation"),
            (30.0, "Lead change"),
            (38.5, "Desperation"),
            (show.t_ko, "K.O."),
            (show.grid.t_reassemble, "Reset"),
        ]
        .into_iter()
        .map(|(t, l)| (t as f32, l.to_string()))
        .collect();
        assert_eq!(m, want);
        assert!(show.grid.t_fight < 20.0 && 38.5 < show.t_ko && show.t_ko < show.grid.t_reassemble);
    }

    #[test]
    fn a_take_without_a_seed_or_a_fight_fails_in_words() {
        let (engine, root) = test_engine();
        load(&engine, 1, &draft(None));
        let e = wait_for(&engine, 10.0, |e| take_of(e).is_some());
        assert!(
            matches!(&e, Event::TakeFailed { take: 1, problem } if problem == "Pick a seed first."),
            "{e:?}"
        );
        let mut d = draft(Some(5));
        d.handicap = [0.0, 1.0];
        load(&engine, 2, &d);
        let Event::TakeFailed { take: 2, problem } = wait_for(&engine, 10.0, |e| take_of(e).is_some()) else {
            panic!("not a failed take");
        };
        assert!(
            problem.starts_with("This fight cannot be used: ") && problem.ends_with('.'),
            "{problem}"
        );
        close(engine, root);
    }

    #[test]
    fn a_panic_fails_the_take_in_words() {
        let (engine, root) = test_engine();
        load(&engine, PANICS, &draft(Some(1)));
        let e = wait_for(&engine, 10.0, |e| take_of(e).is_some());
        assert!(
            matches!(&e, Event::TakeFailed { take: PANICS, problem }
                if problem == "Something went wrong while making the take, so it stopped. Try again."),
            "{e:?}"
        );
        // the engine keeps making takes
        load(&engine, 2, &draft(None));
        assert!(matches!(
            wait_for(&engine, 10.0, |e| take_of(e).is_some()),
            Event::TakeFailed { take: 2, .. }
        ));
        close(engine, root);
    }

    #[test]
    fn a_replaced_take_that_panics_says_nothing() {
        let (engine, root) = test_engine();
        load(&engine, PANICS_REPLACED, &draft(Some(1)));
        std::thread::sleep(Duration::from_millis(50));
        load(&engine, 2, &draft(None));
        let events = gather(&engine, 1.5, |_| false);
        let takes: Vec<Option<TakeId>> = events.iter().map(take_of).collect();
        assert_eq!(takes, [Some(2)], "{events:?}");
        close(engine, root);
    }

    // ------------------------------------------------------------ through the engine (slow in debug)

    #[test]
    #[cfg_attr(debug_assertions, ignore = "slow in debug; run with --release --include-ignored")]
    fn a_seed_with_no_ko_fails_in_words() {
        let mut d = draft(None);
        d.handicap = [0.05, 0.05];
        let seed = seeds_where(&d, false, 1)[0];
        d.seed = Some(seed);
        let (engine, root) = test_engine();
        load(&engine, 1, &d);
        let e = wait_for(&engine, 30.0, |e| take_of(e).is_some());
        let want =
            format!("Seed {seed} has no K.O. before 58 s, so it cannot be made into a video. Pick another seed.");
        assert!(
            matches!(&e, Event::TakeFailed { take: 1, problem } if *problem == want),
            "{e:?}"
        );
        close(engine, root);
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore = "slow in debug; run with --release --include-ignored")]
    fn a_take_arrives_picture_first() {
        let seed = seeds_where(&draft(None), true, 1)[0];
        let (engine, root) = test_engine();
        let asked = Instant::now();
        load(&engine, 1, &draft(Some(seed)));
        let mut picture_at = None;
        let events = gather(&engine, 120.0, |e| {
            if matches!(e, Event::TakePicture { .. }) {
                picture_at = Some(asked.elapsed());
            }
            matches!(e, Event::TakeSound { .. } | Event::TakeFailed { .. })
        });
        let sound_at = asked.elapsed();
        eprintln!(
            "seed {seed}: LoadTake to TakePicture {:.2} s, to TakeSound {:.2} s",
            picture_at.map_or(f64::NAN, |d| d.as_secs_f64()),
            sound_at.as_secs_f64()
        );
        let [
            Event::TakePicture { take: 1, story, frames },
            Event::TakeSound { take: 1, audio, music },
        ] = &events[..]
        else {
            panic!("not a picture then a sound: {events:?}");
        };
        assert_eq!(story.seed, seed);
        assert!(story.t_ko.is_some() && story.winner.is_some());
        assert_eq!(frames.frames(), 1_830);
        let f = frames.draw(0);
        assert_eq!((f.w, f.h, f.rgb.len()), (360, 640, 691_200));
        assert_eq!(audio.samples.len(), 2_928_640);
        let titles: Vec<&str> = music.checks.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(titles, ["Loudness", "Perceived loudness", "Fighters above the music"]);
        assert!(music.checks.iter().all(|c| c.note.ends_with('.')), "{:?}", music.checks);
        let roster = bf_content::roster();
        let color = |name: &str| roster.iter().find(|f| f.name == name).map(|f| f.color);
        let fighters: Vec<(&str, Option<[u8; 3]>)> = music
            .lanes
            .iter()
            .filter(|l| l.group == LaneGroup::Fighters)
            .map(|l| (l.name.as_str(), l.color))
            .collect();
        assert_eq!(fighters, [("Flute", color("FLUTE")), ("Trombone", color("TROMBONE"))]);
        assert!(music.lanes.iter().any(|l| l.group == LaneGroup::Band));
        assert!(
            music
                .lanes
                .iter()
                .all(|l| l.clips.iter().all(|&(a, b)| 0.0 <= a && a < b && b <= 61.1))
        );
        assert_eq!(music.markers.first().map(|m| m.1.as_str()), Some("Title card"));
        assert!(
            music.markers.windows(2).all(|w| w[0].0 <= w[1].0),
            "{:?}",
            music.markers
        );
        let ko = music.markers.iter().find(|m| m.1 == "K.O.").expect("a K.O. marker");
        assert_eq!(ko.0, story.t_ko.unwrap() as f32);
        close(engine, root);
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore = "slow in debug; run with --release --include-ignored")]
    fn frames_are_the_engines_own() {
        let seed = seeds_where(&draft(None), true, 1)[0];
        let d = draft(Some(seed));
        let (engine, root) = test_engine();
        load(&engine, 1, &d);
        let Event::TakePicture { frames, .. } = wait_for(&engine, 60.0, |e| take_of(e).is_some()) else {
            panic!("not a picture");
        };
        let ff = convert::fight_file(&d).expect("a fight file");
        let setup = bf_sim::setup_from(&ff, bf_content::roster(), seed).expect("a set-up");
        let rec = bf_sim::run(&setup, true).expect("the fight");
        let pic = bf_content::build_picture(Arc::new(rec), &ff).expect("the picture");
        assert_eq!(&frames.draw(500).rgb[..], &pic.frame(500).data[..]);
        close(engine, root);
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore = "slow in debug; run with --release --include-ignored")]
    fn the_newest_take_wins() {
        let seeds = seeds_where(&draft(None), true, 3);
        let (engine, root) = test_engine();
        // two in a row: once the second is heard from, the first is silent
        load(&engine, 1, &draft(Some(seeds[0])));
        load(&engine, 2, &draft(Some(seeds[1])));
        let first = wait_for(&engine, 60.0, |e| take_of(e).is_some());
        assert_eq!(take_of(&first), Some(2), "{first:?}");
        assert!(matches!(first, Event::TakePicture { .. }), "{first:?}");
        // a third while the second builds its soundtrack: the second's sound never comes
        load(&engine, 3, &draft(Some(seeds[2])));
        let third = wait_for(&engine, 60.0, |e| take_of(e).is_some());
        assert_eq!(take_of(&third), Some(3), "{third:?}");
        let late = gather(&engine, 3.0, |_| false);
        assert!(late.iter().all(|e| take_of(e) == Some(3)), "{late:?}");
        // the third still gets its sound, once the thrown-away soundtrack is done
        let rest = gather(&engine, 120.0, |e| matches!(e, Event::TakeSound { .. }));
        assert!(rest.iter().all(|e| take_of(e) == Some(3)), "{rest:?}");
        assert!(
            matches!(rest.last(), Some(Event::TakeSound { take: 3, .. })),
            "{rest:?}"
        );
        close(engine, root);
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore = "slow in debug; run with --release --include-ignored")]
    fn going_back_is_instant() {
        let seeds = seeds_where(&draft(None), true, 2);
        let (engine, root) = test_engine();
        let mut kept_sound = None;
        for (take, seed) in [(1, seeds[0]), (2, seeds[1])] {
            load(&engine, take, &draft(Some(seed)));
            let events = gather(&engine, 120.0, |e| matches!(e, Event::TakeSound { .. }));
            assert!(matches!(events.last(), Some(Event::TakeSound { .. })), "{events:?}");
            if take == 1 {
                kept_sound = events.into_iter().last();
            }
        }
        let asked = Instant::now();
        load(&engine, 3, &draft(Some(seeds[0])));
        let events = gather(&engine, 1.0, |e| matches!(e, Event::TakeSound { .. }));
        let took = asked.elapsed();
        let [
            Event::TakePicture { take: 3, story, .. },
            Event::TakeSound { take: 3, audio, music },
        ] = &events[..]
        else {
            panic!("not both within 1 s: {events:?}");
        };
        eprintln!("going back took {:.3} s", took.as_secs_f64());
        assert_eq!(story.seed, seeds[0]);
        let Some(Event::TakeSound {
            audio: was_audio,
            music: was_music,
            ..
        }) = kept_sound
        else {
            unreachable!()
        };
        assert!(Arc::ptr_eq(&audio.samples, &was_audio.samples), "the kept sound itself");
        assert_eq!(*music, was_music);
        close(engine, root);
    }
}
