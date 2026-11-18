//! The soundtrack of an Engine 2 fight (milestone E5): the current score, built on the shadow recording
//! (`bf2_direct::shadow`), with the music slowing down in a slow-mo.
//!
//! [`build_soundtrack2`] builds the score on the video clock (`V`: every event where the picture shows it). When the
//! fight file's `music.slowmo` is `follow` (the default) and the slow-mo has moved the K.O. off the fight clock, it
//! builds the score a second time on the fight clock (`M`: as if there were no slow-mo), resamples `M` through the
//! time map up to the K.O. (varispeed: pitch follows the speed, like tape), and splices it onto `V` at the K.O.
//! sample with a 5 ms crossfade. From the K.O. on the soundtrack is `V` sample for sample, so the stinger, the solo,
//! the last bars and the loop seam stay on the video grid. `steady`, or a fight whose clocks agree at the K.O., is
//! `V` alone. The master is `V`'s build's.

use std::panic::{AssertUnwindSafe, catch_unwind};

use bf_core::fightfile::{FightFile, SlowmoMusic};
use bf_core::py::round_half_even;
use bf_core::recording::Recording;
use bf_core::show::Show;
use bf_sound::SRF;
use bf_sound::master::{Levels, Soundtrack};
use bf_sound::mix::Mix;
use bf_sound::warp::{music_src, splice, warp_mix};
use bf2_direct::{Clock, TimeMap, shadow};
use bf2_sim::{FighterDef, Recording2};

use crate::error::ContentError;
use crate::soundtrack::{SoundBuild, panic_text, validate};

/// An Engine 2 fight's soundtrack, with what it was built from.
pub struct Sound2 {
    /// The master and its stems, each `bf_sound::NA` frames.
    pub soundtrack: Soundtrack,
    /// The show on the video clock (its captions and K.O. are where the picture shows them).
    pub show: Show,
    /// The shadow recording on the video clock.
    pub shadow: Recording,
    /// The master's bus levels (the video-clock build's).
    pub levels: Levels,
    /// How many scores were built: 1 (the video clock alone) or 2 (the fight clock too, warped to the K.O.).
    pub builds: u8,
}

/// The fight clock's K.O. and the video clock's agree to within half a sample: no slow-mo moved it, so the
/// fight-clock build would be the video-clock build.
fn clocks_agree(rec: &Recording2, ko_video: f64) -> bool {
    rec.outcome.t_ko.is_some_and(|t| (t - ko_video).abs() <= 0.5 / SRF)
}

/// The K.O. seam: the sample of the K.O. on the video clock.
fn seam_of(ko_video: f64) -> usize {
    round_half_even(ko_video * SRF).max(0.0) as usize
}

/// The spliced mix before the master, its master and levels (the video-clock build's) and the build count.
type Scored = (Mix, Soundtrack, Levels, u8);

/// The scores of [`build_soundtrack2`] and their splice, inside its panic catch.
fn score2(
    rec: &Recording2,
    defs: [&FighterDef; 2],
    map: &TimeMap,
    ff: &FightFile,
    show_v: &Show,
    sv: &Recording,
) -> Result<Scored, ContentError> {
    let bv = SoundBuild::new(show_v, sv, ff)?;
    let v = bv.score()?;
    let ko_video = show_v.t_ko;
    let (mix, builds) = if ff.music.slowmo == SlowmoMusic::Follow && !clocks_agree(rec, ko_video) {
        let grid = ff.grid();
        let sf = shadow(rec, defs, map, Clock::Fight, &grid);
        let show_f = Show::new(&sf, ff)?;
        let m = SoundBuild::new(&show_f, &sf, ff)?.score()?;
        let seam = seam_of(ko_video);
        let fight_t = |t: f64| map.fight_t(t);
        let src = music_src(grid.t_fight, &fight_t);
        (splice(&warp_mix(&m, &src, seam), &v, seam), 2)
    } else {
        (v, 1)
    };
    let (soundtrack, levels) = (bv.master(&mix), bv.levels());
    Ok((mix, soundtrack, levels, builds))
}

/// The show of the Engine 2 fight `rec` (fought by `defs`, left first) on the video clock, shown through `map`
/// under `ff`, and the video-clock shadow it is made from: what [`build_soundtrack2`] scores and the UI layer
/// (`bf2_render::Ui2`) draws. Cheap next to the score (no sound is built), so the app's preview makes it alone.
///
/// # Errors
/// [`ContentError::Show`] for a fight with no K.O. (`NoKo`) or a recording that is not the fight file's.
pub fn video_show2(
    rec: &Recording2,
    defs: [&FighterDef; 2],
    map: &TimeMap,
    ff: &FightFile,
) -> Result<(Show, Recording), ContentError> {
    let sv = shadow(rec, defs, map, Clock::Video, &ff.grid());
    let show = Show::new(&sv, ff)?;
    Ok((show, sv))
}

/// The soundtrack of the Engine 2 fight `rec` (fought by `defs`, left first), shown through the time map `map`,
/// under the fight file `ff`:
///
/// 1. [`validate`] the fight file;
/// 2. the video-clock shadow and its show;
/// 3. the video-clock score `V`;
/// 4. under `music.slowmo = "follow"`, when the clocks differ at the K.O.: the fight-clock score `M`, warped through
///    the music clock up to the K.O. seam and spliced onto `V` there;
/// 5. mastered with `V`'s build.
///
/// The same recording, map and fight file give the same samples on every run and thread count. A panic in the
/// sound code is caught and comes back as [`ContentError::Sound`], as in [`crate::build_soundtrack`].
///
/// # Errors
/// What [`validate`] refuses; [`ContentError::Show`] for a fight with no K.O. (`NoKo`) or a recording that is not
/// the fight file's; [`ContentError::Sound`] for what the score cannot play.
pub fn build_soundtrack2(
    rec: &Recording2,
    defs: [&FighterDef; 2],
    map: &TimeMap,
    ff: &FightFile,
) -> Result<Sound2, ContentError> {
    validate(ff)?;
    let (show, sv) = video_show2(rec, defs, map, ff)?;
    let scored = catch_unwind(AssertUnwindSafe(|| score2(rec, defs, map, ff, &show, &sv)));
    let (_, soundtrack, levels, builds) = match scored {
        Ok(result) => result?,
        Err(payload) => return Err(ContentError::Sound(panic_text(payload))),
    };
    Ok(Sound2 {
        soundtrack,
        show,
        shadow: sv,
        levels,
        builds,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, OnceLock};

    use bf_sound::music::signal::{MomentKind, StorySignal};
    use bf_sound::{NA, SR};
    use bf2_direct::DirectCfg;
    use bf2_sim::{Cue, EvKind, Rules2, Setup2};

    use super::*;

    fn dummy_file() -> FightFile {
        FightFile::parse(include_str!("../../../fights/engine2/test_dummy.toml")).expect("the dummy's fight file")
    }

    fn defs(ff: &FightFile) -> [Arc<FighterDef>; 2] {
        [0, 1].map(|k| crate::fighters2::def(&ff.fighters[k]).expect("an Engine 2 fighter"))
    }

    /// The dummy fight of `seed` under `ff`, set up as `bf_engine::fight` sets it up.
    fn fight(ff: &FightFile, seed: u64, record: bool) -> Recording2 {
        let mut rules = Rules2::new(ff.grid().t_fight);
        rules.dmg_scale = ff.dmg_scale;
        rules.dmg_side = [ff.handicap[0], ff.handicap[1]];
        bf2_sim::run(
            &Setup2 {
                fighters: defs(ff),
                rules,
                seed,
            },
            record,
        )
        .expect("the dummy fights")
    }

    /// Whether the fight has a `SlowMo` cue before its K.O.
    fn slows(rec: &Recording2) -> bool {
        let ko = rec
            .events
            .iter()
            .find(|e| matches!(e.kind, EvKind::Ko { .. }))
            .map(|e| e.step);
        ko.is_some_and(|ko| {
            rec.events
                .iter()
                .any(|e| e.step < ko && matches!(e.kind, EvKind::Cue(Cue::SlowMo { .. })))
        })
    }

    /// A K.O. fight of the dummy file (the first of seeds 1 to 200 with a slow-mo before its K.O.), shown through
    /// the default time map (`slow`) and through one with the slow-mo cues off (`plain`: the K.O. cinematic stays,
    /// and the clocks agree at the K.O.). Every dummy K.O. in those seeds has a slow-mo before it (DUMMY BUSTER's).
    struct Fixture {
        ff: FightFile,
        slow: (Recording2, TimeMap),
        plain: (Recording2, TimeMap),
    }

    fn fixture() -> &'static Fixture {
        static F: OnceLock<Fixture> = OnceLock::new();
        F.get_or_init(|| {
            let ff = dummy_file();
            let ko = |r: &Recording2| r.outcome.t_ko.is_some() && r.outcome.winner.is_some();
            let seed = (1..=200)
                .find(|&s| {
                    let r = fight(&ff, s, false);
                    ko(&r) && slows(&r)
                })
                .expect("a seed in 1 to 200 with a slow-mo before the K.O.");
            let rec = fight(&ff, seed, true);
            let map = |cfg: &DirectCfg| TimeMap::from_recording(&rec, cfg);
            let (slow, plain) = (
                map(&DirectCfg::default()),
                map(&DirectCfg {
                    slowmo: false,
                    ..DirectCfg::default()
                }),
            );
            Fixture {
                slow: (rec.clone(), slow),
                plain: (rec, plain),
                ff,
            }
        })
    }

    fn build(ff: &FightFile, (rec, map): &(Recording2, TimeMap)) -> Result<Sound2, ContentError> {
        let d = defs(ff);
        build_soundtrack2(rec, [&d[0], &d[1]], map, ff)
    }

    /// The follow build of the slow-mo fight, with its spliced mix.
    fn follow() -> &'static (Sound2, Mix) {
        static F: OnceLock<(Sound2, Mix)> = OnceLock::new();
        F.get_or_init(|| {
            let fx = fixture();
            let (rec, map) = &fx.slow;
            let d = defs(&fx.ff);
            let s = build(&fx.ff, &fx.slow).expect("the follow soundtrack");
            let scored = score2(rec, [&d[0], &d[1]], map, &fx.ff, &s.show, &s.shadow).expect("scores");
            (s, scored.0)
        })
    }

    fn steady_file() -> FightFile {
        let mut ff = dummy_file();
        ff.music.slowmo = SlowmoMusic::Steady;
        ff
    }

    /// A single video-clock build of the slow-mo fight, by hand: its mix and its master.
    fn by_hand() -> &'static (Mix, Soundtrack) {
        static F: OnceLock<(Mix, Soundtrack)> = OnceLock::new();
        F.get_or_init(|| {
            let fx = fixture();
            let (rec, map) = &fx.slow;
            let d = defs(&fx.ff);
            let sv = shadow(rec, [&d[0], &d[1]], map, Clock::Video, &fx.ff.grid());
            let show = Show::new(&sv, &fx.ff).unwrap();
            let b = SoundBuild::new(&show, &sv, &fx.ff).unwrap();
            let v = b.score().unwrap();
            let master = b.master(&v);
            (v, master)
        })
    }

    fn same(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|(x, y)| x[0].to_bits() == y[0].to_bits() && x[1].to_bits() == y[1].to_bits())
    }

    #[test]
    fn soundtrack2_is_the_right_length() {
        let (s, _) = follow();
        assert_eq!(s.builds, 2);
        assert_eq!(s.soundtrack.master.len(), NA);
        assert!(!s.soundtrack.stems.is_empty());
        for (bus, x) in &s.soundtrack.stems {
            assert_eq!(x.len(), NA, "{bus}");
        }
        assert_eq!(Some(s.show.t_ko), fixture().slow.1.ko_video_t());
    }

    #[test]
    fn steady_is_one_build() {
        let fx = fixture();
        let s = build(&steady_file(), &fx.slow).unwrap();
        assert_eq!(s.builds, 1);
        let (_, hand) = by_hand();
        assert!(same(&s.soundtrack.master, &hand.master));
        assert_eq!(
            s.soundtrack.stems.keys().collect::<Vec<_>>(),
            hand.stems.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn no_slowmo_is_one_build() {
        let fx = fixture();
        let s = build(&fx.ff, &fx.plain).unwrap();
        assert_eq!(s.builds, 1);
        assert_eq!(s.soundtrack.master.len(), NA);
    }

    /// Zero crossings per second of the left channel of `x` over `a..b` (samples).
    fn zcr(x: &[[f64; 2]], a: usize, b: usize) -> f64 {
        let n = x[a..b]
            .windows(2)
            .filter(|w| (w[0][0] < 0.0) != (w[1][0] < 0.0))
            .count();
        n as f64 / ((b - a) as f64 / SRF)
    }

    /// The longest stretch of video time before the K.O. where the fight runs at under half speed, in samples.
    fn slow_span(map: &TimeMap, ko_video: f64) -> (usize, usize) {
        let (mut best, mut cur) = ((0, 0), None);
        let to = seam_of(ko_video) - SR / 10;
        for i in (seam_of(map.video_t(0.0))..to).step_by(SR / 100) {
            let slow = map.rate(i as f64 / SRF) < 0.5;
            match (slow, cur) {
                (true, None) => cur = Some(i),
                (false, Some(a)) => {
                    if i - a > best.1 - best.0 {
                        best = (a, i);
                    }
                    cur = None;
                }
                _ => {}
            }
        }
        if let Some(a) = cur
            && to - a > best.1 - best.0
        {
            best = (a, to);
        }
        best
    }

    #[test]
    fn follow_slows_the_fight_music() {
        let fx = fixture();
        let (s, _) = follow();
        let (_, steady) = by_hand();
        let (a, b) = slow_span(&fx.slow.1, s.show.t_ko);
        assert!(b - a > SR / 5, "a slow-mo of {} samples", b - a);
        // the loudest bed stem there in the steady build (the bed plays on through the slow-mo in both builds; the
        // fighters' parts and the effects place other notes in the two)
        let energy = |x: &[[f64; 2]]| x[a..b].iter().map(|f| f[0] * f[0]).sum::<f64>();
        let parts: Vec<String> = fx.ff.fighters.iter().map(|n| n.to_lowercase()).collect();
        let (bus, _) = steady
            .stems
            .iter()
            .filter(|(bus, _)| !["fx", "solo"].contains(&bus.as_str()) && !parts.contains(bus))
            .max_by(|x, y| energy(x.1).total_cmp(&energy(y.1)))
            .unwrap();
        let (f, v) = (zcr(&s.soundtrack.stems[bus], a, b), zcr(&steady.stems[bus], a, b));
        assert!(f < 0.6 * v, "{bus}: {f} crossings a second in follow, {v} in steady");
    }

    /// The largest sample-to-sample step of `x` (either channel) from sample `a` to `b`.
    fn max_step(x: &[[f64; 2]], a: usize, b: usize) -> f64 {
        (a..b)
            .map(|i| (x[i + 1][0] - x[i][0]).abs().max((x[i + 1][1] - x[i][1]).abs()))
            .fold(0.0, f64::max)
    }

    /// Review Focus 1: the largest step within 2 ms of the seam is no larger than 1.5x the largest step in the 50 ms
    /// before it, or than the video build's own there: the K.O. lands on the seam, and the video build already rises
    /// into it in the last 1.5 ms (measured: 0.117 there against 0.071 in the 50 ms before), so the splice may step
    /// no harder than either side of it would alone.
    #[test]
    fn ko_after_slowmo_is_clean() {
        let (s, _) = follow();
        let (_, v) = by_hand();
        let seam = seam_of(s.show.t_ko);
        let x = &s.soundtrack.master;
        let ms = SR / 1000;
        let before = max_step(x, seam - 52 * ms, seam - 2 * ms);
        let near = max_step(x, seam - 2 * ms, seam + 2 * ms);
        let own = max_step(&v.master, seam - 2 * ms, seam + 2 * ms);
        assert!(
            near <= 1.5 * before.max(own),
            "{near} at the seam, {before} before it, {own} in the video build there"
        );
        // the fade itself: no step past the larger side's
        let fade = max_step(x, seam - 2 * ms, seam);
        let v_fade = max_step(&v.master, seam - 2 * ms, seam);
        assert!(fade <= 1.5 * before.max(v_fade), "{fade} in the fade");
    }

    #[test]
    fn post_ko_is_on_the_video_grid() {
        let (s, mix) = follow();
        let (v, _) = by_hand();
        let seam = seam_of(s.show.t_ko);
        for bus in v.bus_names() {
            let (a, b) = (mix.bus(bus).unwrap(), v.bus(bus).unwrap());
            assert!(same(&a[seam..], &b[seam..]), "{bus}");
        }
        // and before the fade the slow-mo has changed the music
        let fade0 = seam - bf_sound::warp::SEAM_FADE;
        assert!(
            v.bus_names()
                .iter()
                .any(|b| !same(&mix.bus(b).unwrap()[..fade0], &v.bus(b).unwrap()[..fade0]))
        );
    }

    #[test]
    fn classic_still_builds() {
        let fx = fixture();
        let mut ff = dummy_file();
        ff.music.arrangement = crate::themes::CLASSIC.into();
        let s = build(&ff, &fx.plain).unwrap();
        assert_eq!(s.soundtrack.master.len(), NA);
        assert!(s.soundtrack.master.iter().any(|f| f[0] != 0.0));
    }

    #[test]
    fn story_signal_reads_the_shadow() {
        let fx = fixture();
        let (s, _) = follow();
        let b = SoundBuild::new(&s.show, &s.shadow, &fx.ff).unwrap();
        let signal = StorySignal::from(&b.ctx).unwrap();
        let count = |f: &dyn Fn(&MomentKind) -> bool| signal.moments.iter().filter(|m| f(&m.kind)).count();
        assert_eq!(count(&|k| *k == MomentKind::Ko), 1);
        let cfgs = [0, 1].map(|k| crate::fighters2::cfg(&fx.ff.fighters[k]).unwrap());
        let sheet = bf_story::BeatSheet::series(cfgs);
        let d = defs(&fx.ff);
        let lows = [bf2_direct::low_name(&d[0]), bf2_direct::low_name(&d[1])];
        let story = bf_story::story_at(&s.shadow, fx.ff.grid().t_fight, &sheet, lows);
        assert_eq!(
            count(&|k| matches!(k, MomentKind::LeadChange { .. })),
            story.lead_changes as usize
        );
    }

    /// Review Focus 5: 1 and 8 threads give the same bits.
    #[test]
    fn soundtrack2_is_deterministic() {
        let fx = fixture();
        let (s, _) = follow();
        for threads in [1, 8] {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
            let t = pool.install(|| build(&fx.ff, &fx.slow)).unwrap();
            assert!(same(&t.soundtrack.master, &s.soundtrack.master), "{threads} threads");
            for (bus, x) in &t.soundtrack.stems {
                assert!(same(x, &s.soundtrack.stems[bus]), "{threads} threads: {bus}");
            }
        }
    }

    /// Review Focus 4: a fight without a K.O. is refused in a sentence.
    #[test]
    fn no_ko_is_a_sentence() {
        let mut ff = dummy_file();
        ff.dmg_scale = 0.01;
        let rec = fight(&ff, 3, true);
        assert!(rec.outcome.t_ko.is_none());
        let map = TimeMap::from_recording(&rec, &DirectCfg::default());
        let e = build(&ff, &(rec, map)).err().expect("refused");
        assert_eq!(e, ContentError::Show(bf_core::show::ShowError::NoKo));
        let text = e.to_string();
        assert!(!text.is_empty() && !text.contains('\n'), "{text}");
    }

    /// On Engine 2 the TRUMPET plays its Engine 2 part (the one that reads the Engine 2 shadow): its stem sounds.
    #[test]
    fn trumpet_plays_its_engine2_part() {
        let ff = FightFile::parse(include_str!("../../../fights/engine2/trumpet_vs_test_dummy.toml")).unwrap();
        let rec = fight(&ff, 7, true);
        let map = TimeMap::from_recording(&rec, &DirectCfg::default());
        let s = build(&ff, &(rec, map)).unwrap();
        let names: Vec<&String> = s.soundtrack.stems.keys().collect();
        let stem = s
            .soundtrack
            .stems
            .iter()
            .find(|(b, _)| b.contains("TRUMPET") || b.contains("trumpet"))
            .unwrap_or_else(|| panic!("no trumpet stem in {names:?}"));
        // it sounds through the fight itself (FIGHT! to the K.O.), not only in the title and the ending
        let (a, b) = (
            (s.show.grid.t_fight * 48_000.0) as usize,
            (s.show.t_ko * 48_000.0) as usize,
        );
        let loud = stem.1[a..b].iter().filter(|f| f[0].abs() > 0.01).count();
        assert!(loud > 48_000, "the trumpet sounds for only {loud} samples of the fight");
    }
}
