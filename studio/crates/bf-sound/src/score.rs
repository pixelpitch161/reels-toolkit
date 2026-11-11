//! `kit/music.py score()`: the whole soundtrack as a mix, before the master.
//!
//! The theme's bed ([`Bed`]) under the two fighters' parts ([`Sound`]), in the order every episode builds it:
//!
//! - bar 0: the VS title card: the bed's hit under each word, the left fighter's title blast, VS (the bed's
//!   crack), the right fighter's blast, the doors;
//! - bar 1: the count-in (the bed and each fighter's count notes), FIGHT!;
//! - the fight: the bed's bars to the K.O. (it tape-stops there), each fighter's part, the shared event sounds;
//! - the K.O.: the bed's hit, the loser falls apart, the crowd; the winner's stinger, the bed until the solo,
//!   the solo over the bed's victory bars, the loser's pieces fly home, the glass pop;
//! - the end: the bed's build (and each fighter's role in it) into the title slam on sample 0, the doors
//!   shutting, and a soft thud where each caption line lands.
//!
//! **The order of the calls is part of the sound.** A note is synthesised the first time it is asked for, and
//! every note of a voice with noise moves the shared noise stream on ([`crate::voice::PpNoise`]): the same hooks
//! in another order give other samples. Sounds are also added onto a bus in this order, and a float sum depends
//! on its order. Each block below names the lines of `score()` it mirrors.

use bf_core::py::{floor_div, round_half_even};
use bf_core::rng::Generator;
use bf_core::show::{IMPACT_FALL, IMPACT_STAGGER, impact_landings};

use crate::mix::{Mix, Sig};
use crate::music::{MusicCtx, MusicError, ROAR_SEED, roar, shared_events};
use crate::synth::{self, FILTER_O, THUD_DUR, THUD_F0, THUD_F1, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED};
use crate::traits::{Bed, Sound};

/// The seed of the noise of the doors sliding shut (`np.random.default_rng(93)`).
pub const DOORS_SEED: u64 = 93;

/// How far past the end of the loop the doors' noise runs, in seconds (`T.T_CLOSE + 0.012`): no fade at the seam.
pub const DOORS_OVER: f64 = 0.012;

/// How much longer than the episode the mix is, in seconds (`A.Mix(T.DUR + 4.0)`).
pub const MIX_ROOM: f64 = 4.0;

/// `score()`: the whole soundtrack of the fight in `ctx` as a mix: `bed` under the parts of `sounds` (the left
/// corner's first), every call in the kit's order.
///
/// # Errors
/// A hook of the bed or of a part fails (see [`crate::music`]), `sounds` are not the two sides in order, or the
/// K.O. or the solo falls before the start of the episode (Python counts bars from the end then).
pub fn score(ctx: &MusicCtx, bed: &dyn Bed, sounds: [&dyn Sound; 2]) -> Result<Mix, MusicError> {
    const H: &str = "score";
    for (k, s) in sounds.iter().enumerate() {
        if s.k() != k {
            return Err(MusicError::new(
                H,
                format!("part {k} plays side {}: the parts go left corner first", s.k()),
            ));
        }
    }
    let show = ctx.show;
    let grid = &show.grid;
    let (beat, bar) = (grid.beat, grid.bar);
    let place = |mix: &mut Mix, x: Vec<f64>, t: f64, gain: f64| mix.place("fx", &Sig::Mono(x), t, gain, 0.0);
    // A bar count Python gets with `int(...)`: it must be a bar of the episode, not one before its start.
    let bars = |what: &str, v: f64| -> Result<usize, MusicError> {
        if (0.0..1e9).contains(&v) {
            Ok(v as usize)
        } else {
            Err(MusicError::new(H, format!("{what} is in bar {v}")))
        }
    };

    // mix = A.Mix(T.DUR + 4.0); a, b = SOUNDS
    let mut mix = Mix::new(grid.dur + MIX_ROOM);
    let [a, b] = sounds;

    // ---- bar 0: the VS title card, one band hit per word (music.py 310-317)
    // The left name 4 ms in (no click at the seam), VS, the right name.
    let (t1, t2, t3) = (0.004, beat, 2.0 * beat);
    bed.title(ctx, &mut mix, [t1, t2, t3])?;
    a.title_blast(ctx, bed, &mut mix, t1)?;
    bed.vs(ctx, &mut mix, t2)?;
    b.title_blast(ctx, bed, &mut mix, t3)?;
    bed.title_rest(ctx, &mut mix)?;
    let doors_open = synth::whoosh(
        grid.t_doors.1 - grid.t_doors.0 + 0.1,
        300.0,
        5000.0,
        WHOOSH_RISE,
        WHOOSH_SEED,
    );
    place(&mut mix, doors_open, grid.t_doors.0 - 0.05, 0.14);

    // ---- bar 1: the count-in on the second chord, FIGHT! (music.py 318-323)
    bed.count_in(ctx, &mut mix)?;
    for s in sounds {
        s.count_in(ctx, bed, &mut mix)?;
    }
    let into_fight = synth::whoosh(beat, 400.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
    place(&mut mix, into_fight, bar + 3.0 * beat, 0.12);
    bed.fight_start(ctx, &mut mix)?;

    // ---- the fight: the bed to the K.O. (tape-stopped there), the fighters (music.py 324-331)
    // bar_ko = int(T_KO // T.BAR)
    let bar_ko = bars("the K.O.", floor_div(show.t_ko, bar))?;
    // The fighters' stops, which a bed reads inside `fight` (dark tango's `stop_times`: each part's `stops()` in
    // side order; the bed sorts and filters them). A part's `stops()` reads the run only, so asking before the
    // bed's fight is asking at the same point.
    let stops: Vec<f64> = sounds.iter().flat_map(|s| s.stops(ctx)).collect();
    ctx.set_fighter_stops(stops)?;
    bed.fight(ctx, &mut mix, bar_ko)?;
    // #53: a fighter can set up cross-fighter music first (a line override for the other one).
    for s in sounds {
        s.prepare(ctx, bed)?;
    }
    // Each part with the other side's at hand (M.SOUNDS[self.enemy]).
    for (k, s) in sounds.iter().enumerate() {
        s.play_with(ctx, bed, &mut mix, sounds[1 - k])?;
    }
    shared_events(ctx, bed, &mut mix)?;

    // ---- the K.O. (music.py 332-340)
    bed.ko(ctx, &mut mix)?;
    sounds[show.lose].fall_apart(ctx, bed, &mut mix)?;
    place(&mut mix, roar(5.5, ROAR_SEED)?, show.t_ko + 0.2, bed.roar_gain());
    // The winner's stinger on "(WINNER) WINS".
    let tw = ctx.q16(show.t_ko + 1.3);
    sounds[show.win].stinger(ctx, bed, &mut mix, tw)?;
    bed.stinger(ctx, &mut mix, tw)?;
    let t_solo = show.t_solo;
    bed.after_ko(ctx, &mut mix, bar_ko, tw, t_solo)?;

    // ---- victory: the bed's lighter bars and the winner's solo (music.py 341-346)
    // bed.victory(mix, int(round(t_solo / T.BAR)))
    bed.victory(ctx, &mut mix, bars("the solo", round_half_even(t_solo / bar))?)?;
    sounds[show.win].solo(ctx, bed, &mut mix, t_solo)?;
    // The loser's pieces fly home, a glass pop when it is whole.
    sounds[show.lose].reassemble(ctx, bed, &mut mix)?;
    let (p_lo, p_hi, p_g) = bed.pop();
    // A.chime(p_lo, 0.5) + A.chime(p_hi, 0.5) * 0.5
    let pop: Vec<f64> = synth::chime(p_lo, 0.5)
        .iter()
        .zip(synth::chime(p_hi, 0.5))
        .map(|(lo, hi)| lo + hi * 0.5)
        .collect();
    place(&mut mix, pop, grid.t_glide, p_g);

    // ---- the build into the title slam on sample 0 (music.py 347-358)
    bed.last_bars(ctx, &mut mix)?;
    for s in sounds {
        s.last_bar(ctx, bed, &mut mix)?;
    }
    // The doors sliding shut into the title slam: nc = int((T.T_CLOSE + 0.012) * SR).
    let nc = ((grid.t_close + DOORS_OVER) * crate::SR as f64) as usize;
    let mut noise = vec![0.0; nc];
    Generator::new(DOORS_SEED).fill_normal(0.0, 1.0, &mut noise);
    let tcl = synth::tt(nc);
    let doors: Vec<f64> = synth::bp(&noise, 400.0, 5000.0, FILTER_O)
        .iter()
        .zip(&tcl)
        .map(|(v, t)| {
            let u = t / grid.t_close;
            v * (u * u)
        })
        .collect();
    place(&mut mix, doors, grid.dur - grid.t_close, bed.doors_gain());
    // Caption landings: soft thuds, the first line of a caption a little louder than the others.
    let (g0, g1) = bed.landing();
    let landings = impact_landings(&show.captions, IMPACT_STAGGER, IMPACT_FALL);
    if !landings.is_empty() {
        // A.thud() is the same sound every time.
        let thud = Sig::Mono(synth::thud(THUD_DUR, THUD_F0, THUD_F1, THUD_SEED));
        for landing in landings {
            let gain = if landing.line == 0 { g0 } else { g1 };
            mix.place("fx", &thud, landing.t, gain, 0.0);
        }
    }
    Ok(mix)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::music::testkit::{TestBed, context, episode};
    use crate::traits::Played;

    /// A part that writes down which of its hooks ran, with the time it was given.
    struct Logged<'a>(usize, &'a Mutex<Vec<String>>);

    impl Logged<'_> {
        fn wrote(&self, what: &str) -> Played {
            self.1.lock().expect("the log").push(format!("{}.{what}", self.0));
            Ok(())
        }
    }

    impl Sound for Logged<'_> {
        fn k(&self) -> usize {
            self.0
        }
        fn prepare(&self, _: &MusicCtx, _: &dyn Bed) -> Played {
            self.wrote("prepare")
        }
        fn title_blast(&self, _: &MusicCtx, _: &dyn Bed, _: &mut Mix, t: f64) -> Played {
            self.wrote(&format!("title_blast({t:.3})"))
        }
        fn count_in(&self, _: &MusicCtx, _: &dyn Bed, _: &mut Mix) -> Played {
            self.wrote("count_in")
        }
        fn play(&self, _: &MusicCtx, _: &dyn Bed, _: &mut Mix) -> Played {
            self.wrote("play")
        }
        fn play_with(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix, enemy: &dyn Sound) -> Played {
            self.wrote(&format!("play_with({})", enemy.k()))?;
            self.play(ctx, bed, mix)
        }
        fn stops(&self, _: &MusicCtx) -> Vec<f64> {
            let _ = self.wrote("stops");
            vec![10.0 + self.0 as f64, 5.0]
        }
        fn fall_apart(&self, _: &MusicCtx, _: &dyn Bed, _: &mut Mix) -> Played {
            self.wrote("fall_apart")
        }
        fn stinger(&self, _: &MusicCtx, _: &dyn Bed, _: &mut Mix, tw: f64) -> Played {
            self.wrote(&format!("stinger({tw:.3})"))
        }
        fn solo(&self, _: &MusicCtx, _: &dyn Bed, _: &mut Mix, t0: f64) -> Played {
            self.wrote(&format!("solo({t0:.3})"))
        }
        fn reassemble(&self, _: &MusicCtx, _: &dyn Bed, _: &mut Mix) -> Played {
            self.wrote("reassemble")
        }
        fn last_bar(&self, _: &MusicCtx, _: &dyn Bed, _: &mut Mix) -> Played {
            self.wrote("last_bar")
        }
    }

    #[test]
    fn the_parts_are_called_in_the_kits_order() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        let log = Mutex::new(Vec::new());
        let (a, b) = (Logged(0, &log), Logged(1, &log));
        let bed = TestBed::new((2.0, 0.5, 0.2));
        let mix = score(&ctx, &bed, [&a, &b]).unwrap();
        // The TROMBONE (side 1) won the episode's fight: the stinger and the solo are its, the FLUTE falls
        // apart and flies home. The VS card's words are 4 ms in, on beat 2 and on beat 3.
        let want = [
            "0.title_blast(0.004)",
            "1.title_blast(0.897)",
            "0.count_in",
            "1.count_in",
            "0.stops",
            "1.stops",
            "0.prepare",
            "1.prepare",
            "0.play_with(1)",
            "0.play",
            "1.play_with(0)",
            "1.play",
            "0.fall_apart",
            "1.stinger(51.357)",
            "1.solo(52.029)",
            "0.reassemble",
            "0.last_bar",
            "1.last_bar",
        ];
        assert_eq!(*log.lock().unwrap(), want);
        // The fighters' stops, left corner first, as each part gave them (the bed sorts and filters them).
        assert_eq!(ctx.fighter_stops().unwrap(), [10.0, 5.0, 11.0, 5.0]);
        // The test bed plays nothing but its clash marks; the score's own sounds are on `fx`: the doors, the
        // whoosh into FIGHT!, the crowd, the pop, the doors shutting and the caption thuds.
        assert_eq!(mix.bus_names(), ["fx", "clash"]);
        assert_eq!(bed.clashes.lock().unwrap().len(), 9);
        let fx = mix.bus("fx").unwrap();
        assert_eq!(fx.len(), (65.0 * 48000.0) as usize + 6 * 48000);
        let loud = |t: f64| {
            let i = (t * 48000.0) as usize;
            fx[i..i + 4800].iter().any(|f| f[0].abs() > 1e-4)
        };
        // The doors open under the card, the crowd roars after the K.O., the doors shut into the loop point.
        assert!(loud(1.6) && loud(ep.show.t_ko + 0.5) && loud(60.85));
        // Each caption line lands with a thud: the K.O. caption's is 0.1 s after the K.O.
        assert!(loud(ep.show.t_ko + 0.1));
        // The doors' noise runs 12 ms past the 61 s (no fade at the seam) and stops there.
        let end = 61 * 48000 + 576;
        assert!(fx[end - 500..end].iter().all(|f| f[0] != 0.0));
        assert!(fx[end..].iter().all(|f| *f == [0.0, 0.0]));
    }

    #[test]
    fn the_parts_must_be_the_two_sides_in_order() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        let log = Mutex::new(Vec::new());
        let (a, b) = (Logged(0, &log), Logged(1, &log));
        let bed = TestBed::new((2.0, 0.5, 0.2));
        assert_eq!(
            score(&ctx, &bed, [&b, &a]).unwrap_err().to_string(),
            "score: part 0 plays side 1: the parts go left corner first"
        );
        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn a_failing_hook_stops_the_score() {
        struct Broken;
        impl Sound for Broken {
            fn k(&self) -> usize {
                1
            }
            fn count_in(&self, _: &MusicCtx, _: &dyn Bed, _: &mut Mix) -> Played {
                Err(MusicError::new("count_in", "no notes"))
            }
        }
        let ep = episode();
        let (ctx, _) = context(&ep);
        let log = Mutex::new(Vec::new());
        let a = Logged(0, &log);
        let bed = TestBed::new((2.0, 0.5, 0.2));
        assert_eq!(
            score(&ctx, &bed, [&a, &Broken]).unwrap_err().to_string(),
            "count_in: no notes"
        );
        assert_eq!(*log.lock().unwrap(), ["0.title_blast(0.004)", "0.count_in"]);
    }

    /// The kit's base `Sound` on both sides: the bed alone (a theme's bed-alone gate). Nothing is placed on a
    /// fighter's bus, and the master's levels still list both buses with the base class's numbers.
    #[test]
    fn two_base_sounds_leave_the_bed_alone() {
        use crate::master::Levels;
        use crate::traits::BaseSound;
        let ep = episode();
        let (ctx, voices) = context(&ep);
        let bed = TestBed::new((2.0, 0.5, 0.2));
        let (a, b) = (BaseSound { k: 0 }, BaseSound { k: 1 });
        let mix = score(&ctx, &bed, [&a, &b]).unwrap();
        assert_eq!(mix.bus_names(), ["fx", "clash"]);
        assert!(voices.iter().all(|v| v.calls().is_empty()), "no note is asked for");
        assert!(ctx.fighter_stops().unwrap().is_empty());
        let levels = Levels::of(&ctx, [&a, &b], &bed, &[]);
        assert_eq!(levels.get("flute"), Some((1.5, 0.35)));
        assert_eq!(levels.get("trombone"), Some((1.5, 0.35)));
    }
}
