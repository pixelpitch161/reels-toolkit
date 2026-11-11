//! What the score calls: a fighter's part ([`Sound`]) and a theme's bed ([`Bed`]).
//!
//! `kit/music.py score()` builds every soundtrack in one fixed order (the VS title card, the count-in, the fight
//! to the K.O., the K.O., the winner's solo, the build into the title slam on sample 0) and at each point calls
//! the theme's bed (`kit/moods/<mood>.py`, class `Bed`) and the two fighters' parts (`kit/fighters/<name>.py`,
//! class `Sound`, on the base class in `kit/fighters/base.py`). The two traits here are those two classes: the
//! same hooks with the same names, each documented with the Python it mirrors.
//!
//! A hook gets the build's [`MusicCtx`] (Python's `self.M`: the run, the timeline, the notes and helpers) and the
//! mix to place its sounds on; a fighter's hook also gets the bed (Python's `self.M.bed`: a fighter may land a
//! hit of the bed's own drum, [`Bed::boom`]). A hook returns `Err` when a helper it calls does (see
//! [`crate::music`]), and the build stops there.
//!
//! Both traits are object-safe and `Send + Sync`: the score takes `&dyn Bed` and `[&dyn Sound; 2]`.

use std::sync::Arc;

use bf_core::py::round_nd;
use bf_core::recording::Array;

use crate::mix::Mix;
use crate::music::{ColourArgs, MusicCtx, MusicError};

/// What a hook that plays into the mix returns.
pub type Played = Result<(), MusicError>;

/// `Sound.voice`'s default velocity.
pub const SOUND_VOICE_VEL: f64 = 0.85;
/// `Sound.colour`'s default `drive_k`.
pub const SOUND_COLOUR_DRIVE_K: f64 = 1.0;

/// A fighter's part in the score: `kit/fighters/base.py` `Sound(M, k)`. A fighter implements [`Sound::k`] and
/// overrides what it plays; every hook is optional, and the defaults play nothing.
///
/// Python's instance attributes are methods here: `self.k` is [`Sound::k`], and `self.name`, `self.bus`,
/// `self.role` and `self.enemy` are derived from it (and from the context) by provided methods.
pub trait Sound: Send + Sync {
    /// `self.k`: its side, 0 (the left corner) or 1.
    fn k(&self) -> usize;

    /// `bus_gain`: its bus in the master (its own episode's mix). Python's default is 1.5.
    fn bus_gain(&self) -> f64 {
        1.5
    }

    /// `send`: its reverb send. Python's default is 0.35.
    fn send(&self) -> f64 {
        0.35
    }

    /// `band`: its band for the band check, in Hz (it must sit above the bed there). Python's default is
    /// `(200, 4000)`.
    fn band(&self) -> (f64, f64) {
        (200.0, 4000.0)
    }

    /// `colour_args`: how it thins and dirties as its HP drops (`music.colour`'s keyword arguments). Python's
    /// default is `dict(cut0=90, cut_span=520, drive=4.5)`.
    fn colour_args(&self) -> ColourArgs {
        ColourArgs::default()
    }

    /// `self.name`: the fighter's name, `M.NAMES[k]`.
    fn name<'a>(&self, ctx: &'a MusicCtx) -> &'a str {
        &ctx.show.names[self.k()]
    }

    /// `self.bus`: the bus it plays on, `self.name.lower()`.
    fn bus(&self, ctx: &MusicCtx) -> String {
        self.name(ctx).to_lowercase()
    }

    /// `self.role`: its role in the arrangement, `M.T.ROLES.get(self.name, 'melody')`.
    fn role<'a>(&self, ctx: &'a MusicCtx) -> &'a str {
        ctx.ff.music.roles.get(self.name(ctx)).map_or("melody", String::as_str)
    }

    /// `self.enemy`: the other side, `1 - k`.
    fn enemy(&self) -> usize {
        1 - self.k()
    }

    /// `voice(m, dur, vel=0.85)`: one note in its voice (a debuff on it can play a sour twin in it). Default: its
    /// Pixel Pitch voice, `M.note(M.CFG[k]['pp'], m, round(dur, 3), round(dur + 0.2, 2), round(vel, 1))`.
    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        let name = &ctx.cfg[self.k()].pp;
        ctx.note(name, m, round_nd(dur, 3), round_nd(dur + 0.2, 2), round_nd(vel, 1))
    }

    /// `detune(t)`: the OUT OF TUNE depth on it at `t` (0 to 1): the engine's recorded status, `RUN['DET']`
    /// (per frame and fighter: the time left and the depth). 0 outside the fight, when the time left is not above
    /// zero, and for a recording without a `DET` array of that shape (no fighter of Milestone 1 records one).
    fn detune(&self, ctx: &MusicCtx, t: f64) -> f64 {
        if !(ctx.show.grid.t_fight <= t && t < ctx.show.t_ko) {
            return 0.0;
        }
        let at = (ctx.sim_i(t) * 2 + self.k()) * 2;
        let is_det = |shape: &[usize], len: usize| shape.len() == 3 && shape[1..] == [2, 2] && at + 1 < len;
        let (left, depth) = match ctx.rec.arrays.get("DET") {
            Some(Array::F64 { shape, data }) if is_det(shape, data.len()) => (data[at], data[at + 1]),
            Some(Array::I64 { shape, data }) if is_det(shape, data.len()) => (data[at] as f64, data[at + 1] as f64),
            _ => return 0.0,
        };
        if left > 0.0 { depth } else { 0.0 }
    }

    /// `colour(x, t, drive_k=1.0, **kw)`: its tone at `t`: `M.colour(x, k, t, drive_k, **a)` with `a` its
    /// `colour_args` updated by `kw`. Here the caller passes the whole set: [`Sound::colour_args`] as it is, or
    /// with fields replaced (`ColourArgs { low: true, ..self.colour_args() }`). Returns `(signal, gain, lead)`.
    fn colour(
        &self,
        ctx: &MusicCtx,
        x: &[f64],
        t: f64,
        drive_k: f64,
        args: &ColourArgs,
    ) -> Result<(Vec<f64>, f64, f64), MusicError> {
        ctx.colour(x, self.k(), t, drive_k, args)
    }

    // The score, in the order kit.music calls it.

    /// `prepare()`: before any fighter's part: cross-fighter set-up (#53: a line override for the enemy, see
    /// [`MusicCtx::line_override`]). Default: nothing.
    fn prepare(&self, ctx: &MusicCtx, bed: &dyn Bed) -> Played {
        let _ = (ctx, bed);
        Ok(())
    }

    /// `title_blast(mix, t)`: its hit as its name slams in on the VS card (the fight file's title hits).
    fn title_blast(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let _ = (ctx, bed, mix, t);
        Ok(())
    }

    /// `count_in(mix)`: its notes on the count-in (bar 1) and the pickup into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        let _ = (ctx, bed, mix);
        Ok(())
    }

    /// `play(mix)`: its part in the fight, from the run's events.
    fn play(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        let _ = (ctx, bed, mix);
        Ok(())
    }

    /// `fall_apart(mix)`: it lost: the sound of it shattering (from the end of the hit-stop, `P.T_HS`).
    fn fall_apart(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        let _ = (ctx, bed, mix);
        Ok(())
    }

    /// `stinger(mix, tw)`: it won: its stinger on "(WINNER) WINS", at `tw`.
    fn stinger(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let _ = (ctx, bed, mix, tw);
        Ok(())
    }

    /// `solo(mix, t0)`: it won: its solo (the fight file's) from `t0`, stopping at the reassembly.
    fn solo(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let _ = (ctx, bed, mix, t0);
        Ok(())
    }

    /// `reassemble(mix)`: it lost: its pieces fly home as a rising run.
    fn reassemble(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        let _ = (ctx, bed, mix);
        Ok(())
    }

    /// `last_bar(mix)`: its part in the build into the title slam (a role, such as the bass climbing the
    /// dominant).
    fn last_bar(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        let _ = (ctx, bed, mix);
        Ok(())
    }
}

/// A theme's classic arrangement: the `Bed(M)` class of a mood module (`kit/moods/big_top.py` and its siblings).
/// The kit has no base class for it: every mood provides all of this, so nothing here has a default.
///
/// The first seven methods are the class attributes the master and the score read; the rest are the sections,
/// which `kit.music` calls in order (and [`Bed::boom`], which the fighters call).
pub trait Bed: Send + Sync {
    /// `buses`: the bed's own buses, each with its `(gain, reverb send)`, in the order the mood lists them (the
    /// master adds the reverb sends up in that order).
    fn buses(&self) -> &[(&'static str, (f64, f64))];

    /// `reverb`: the room, `(dur, decay, wet)`: the length and the decay time of the impulse response in
    /// seconds, and how much of the wet signal goes into the master.
    fn reverb(&self) -> (f64, f64, f64);

    /// `comp`: the master compressor, `(threshold, exponent)`.
    fn comp(&self) -> (f64, f64);

    /// `roar_gain`: the gain of the crowd's roar at the K.O.
    fn roar_gain(&self) -> f64;

    /// `doors_gain`: the gain of the doors sliding shut into the title slam.
    fn doors_gain(&self) -> f64;

    /// `landing`: the gains of the caption landing thuds, `(first line, others)`.
    fn landing(&self) -> (f64, f64);

    /// `pop`: the glass pop when the loser is whole again: two chimes (MIDI notes) and the gain,
    /// `(low, high, gain)`.
    fn pop(&self) -> (f64, f64, f64);

    /// `boom(mix, t, g)`: a big drum hit at `t` with gain `g` (a slam landing). Fighters call it.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played;

    /// `clash(mix, t)`: the two balls clashing at `t` (`kit.music.shared_events` calls it for every `clash`
    /// event).
    fn clash(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64) -> Played;

    /// `title(mix, times)`: bar 0, the VS title card: a hit under each of its three words (the left name, VS,
    /// the right name) at `times`.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played;

    /// `vs(mix, t)`: VS: the crack where it splits the divider.
    fn vs(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64) -> Played;

    /// `title_rest(mix)`: what holds under the card (it is never quiet) and leads into the doors opening.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played;

    /// `count_in(mix)`: bar 1: the count-in on the second chord, a pickup and a roll into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played;

    /// `fight_start(mix)`: FIGHT!: the band comes in.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played;

    /// `fight(mix, bar_ko)`: the bed from FIGHT! to the K.O. in bar `bar_ko` (`int(T_KO // T.BAR)`), which it
    /// tape-stops there.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played;

    /// `ko(mix)`: the K.O. hit.
    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played;

    /// `stinger(mix, tw)`: the bed under the winner's stinger on "(WINNER) WINS", at `tw`.
    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played;

    /// `after_ko(mix, bar_ko, tw, t_solo)`: from the stinger at `tw` to the solo at `t_solo`.
    fn after_ko(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize, tw: f64, t_solo: f64) -> Played;

    /// `victory(mix, b_solo)`: the lighter bars under the winner's solo, from bar `b_solo`
    /// (`int(round(t_solo / T.BAR))`) up to the build.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played;

    /// `last_bars(mix)`: the build into the title slam on sample 0.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played;
}

#[cfg(test)]
mod tests {
    use bf_core::event::Value;

    use super::*;
    use crate::music::testkit::{TestBed, context, episode};

    /// A fighter that only says which side it is on.
    struct Plain(usize);

    impl Sound for Plain {
        fn k(&self) -> usize {
            self.0
        }
    }

    #[test]
    fn the_defaults_are_base_pys() {
        let ep = episode();
        let (ctx, voices) = context(&ep);
        let (a, b) = (Plain(0), Plain(1));
        assert_eq!((a.bus_gain(), a.send(), a.band()), (1.5, 0.35, (200.0, 4000.0)));
        assert_eq!(a.colour_args(), ColourArgs::default());
        assert_eq!(
            (a.name(&ctx), a.bus(&ctx), a.enemy()),
            ("FLUTE", "flute".to_string(), 1)
        );
        assert_eq!(
            (b.name(&ctx), b.bus(&ctx), b.enemy()),
            ("TROMBONE", "trombone".to_string(), 0)
        );
        // The fight file's roles; a fighter it names none for plays the melody.
        assert_eq!((a.role(&ctx), b.role(&ctx)), ("melody", "bass"));
        let mut ff = ep.ff.clone();
        ff.music.roles.clear();
        let other = MusicCtx::new(&ep.show, &ep.rec, &ff, ctx.cfg, ctx.voices.clone()).unwrap();
        assert_eq!(b.role(&other), "melody");

        // voice: its Pixel Pitch voice, the held length to 3 places, the cap 0.2 s longer to 2, the velocity
        // to 1. (The kit's base class asks for ('TROMBONE', 52, 0.25, 0.45, 0.8) here.)
        let x = b.voice(&ctx, 52.0, 0.2504, 0.84).unwrap();
        assert_eq!(x.len(), 21600);
        assert_eq!(voices[1].calls()[0], (52.0, 0.8, 0.25));
        let first = &ctx.first_notes()[0];
        assert_eq!(
            (first.name.as_str(), first.midi, first.dur, first.cap, first.vel),
            ("TROMBONE", 52, 0.25, 0.45, 0.8)
        );
        // colour: music.colour for its own side.
        let args = ColourArgs {
            low: true,
            ..b.colour_args()
        };
        assert_eq!(
            b.colour(&ctx, &x, 30.0, 0.8, &args).unwrap(),
            ctx.colour(&x, 1, 30.0, 0.8, &args).unwrap()
        );
        assert!(a.colour(&ctx, &x, 30.0, 0.8, &args).unwrap() != b.colour(&ctx, &x, 30.0, 0.8, &args).unwrap());

        // The sections play nothing.
        let mut mix = Mix::new(1.0);
        let s: &dyn Sound = &a;
        let quiet = TestBed::new((2.0, 0.5, 0.2));
        let bed: &dyn Bed = &quiet;
        s.prepare(&ctx, bed).unwrap();
        s.title_blast(&ctx, bed, &mut mix, 0.0).unwrap();
        s.count_in(&ctx, bed, &mut mix).unwrap();
        s.play(&ctx, bed, &mut mix).unwrap();
        s.fall_apart(&ctx, bed, &mut mix).unwrap();
        s.stinger(&ctx, bed, &mut mix, 51.0).unwrap();
        s.solo(&ctx, bed, &mut mix, 52.0).unwrap();
        s.reassemble(&ctx, bed, &mut mix).unwrap();
        s.last_bar(&ctx, bed, &mut mix).unwrap();
        assert!(mix.bus_names().is_empty());
    }

    #[test]
    fn detune_reads_the_recorded_status() {
        let ep = episode();
        // The episode's fighters record no DET: never out of tune.
        let (ctx, _) = context(&ep);
        assert_eq!((Plain(0).detune(&ctx, 12.0), Plain(1).detune(&ctx, 1.0)), (0.0, 0.0));
        drop(ctx);
        // With one: per frame and fighter, the time left and the depth.
        let mut rec = ep.rec.clone();
        let frames = rec.frames();
        let mut data = vec![0.0; frames * 4];
        let frame = 252; // the frame at 12.0 s
        data[(frame * 2 + 1) * 2..][..2].copy_from_slice(&[0.5, 0.7]); // fighter 1: 0.5 s left, depth 0.7
        data[(frame * 2) * 2..][..2].copy_from_slice(&[0.0, 0.9]); // fighter 0: none left
        rec.arrays.insert(
            "DET".into(),
            Array::F64 {
                shape: vec![frames, 2, 2],
                data,
            },
        );
        let old = context(&ep).0;
        let ctx = MusicCtx::new(&ep.show, &rec, &ep.ff, old.cfg, old.voices.clone()).unwrap();
        assert_eq!(ctx.sim_i(12.0), frame);
        assert_eq!((Plain(1).detune(&ctx, 12.0), Plain(0).detune(&ctx, 12.0)), (0.7, 0.0));
        assert_eq!(Plain(1).detune(&ctx, 13.0), 0.0);
        // Only during the fight: not before FIGHT!, not from the K.O. on.
        let last = (frames - 1) * 2 + 1;
        if let Some(Array::F64 { data, .. }) = rec.arrays.get_mut("DET") {
            data[..4].copy_from_slice(&[1.0, 0.3, 1.0, 0.3]);
            data[last * 2..][..2].copy_from_slice(&[1.0, 0.4]);
        }
        let ctx = MusicCtx::new(&ep.show, &rec, &ep.ff, old.cfg, old.voices.clone()).unwrap();
        let grid = &ep.show.grid;
        assert_eq!(Plain(1).detune(&ctx, grid.t_fight - 0.01), 0.0);
        assert_eq!(Plain(1).detune(&ctx, grid.t_fight), 0.3);
        assert_eq!(Plain(1).detune(&ctx, ep.show.t_ko - 0.001), 0.4);
        assert_eq!(Plain(1).detune(&ctx, ep.show.t_ko), 0.0);
        assert_eq!(Plain(1).detune(&ctx, f64::NAN), 0.0);
        // Whole numbers are read as numbers; anything of another shape is not a DET.
        rec.arrays.insert(
            "DET".into(),
            Array::I64 {
                shape: vec![frames, 2, 2],
                data: vec![2; frames * 4],
            },
        );
        let ctx = MusicCtx::new(&ep.show, &rec, &ep.ff, old.cfg, old.voices.clone()).unwrap();
        assert_eq!(Plain(0).detune(&ctx, 12.0), 2.0);
        rec.arrays
            .insert("DET".into(), Array::Nested(Value::List(vec![Value::Null; frames])));
        let ctx = MusicCtx::new(&ep.show, &rec, &ep.ff, old.cfg, old.voices.clone()).unwrap();
        assert_eq!(Plain(0).detune(&ctx, 12.0), 0.0);
    }
}
