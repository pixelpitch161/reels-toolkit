//! The frame of the Studio's own themes (the batch of 2026-11-16: themes with no kit mood behind them). A kit theme's
//! classic bed reproduces the kit's; a Studio theme has none to reproduce, so its bed is made from its own layers:
//! [`Frame`] plays the sections the layer engine hands to a classic bed (the VS card, the count-in, FIGHT!, the K.O.,
//! the stinger, the bars after the K.O., the build into frame 0, the clashes and every hook) with the theme's
//! `drums`, `bass` and `chords` layers, and both arrangements are a [`LayeredBed`] over it: the Classic one on the
//! theme's plain table with no story cues (a simpler cut), the Composed one on its full table, cued by the story.
//!
//! Every Studio theme has all five hooks, so any pair may play it.

use bf_sound::mix::{Mix, Sig};
use bf_sound::music::groove::Groove;
use bf_sound::music::layers::{
    ArrangementTable, CRASH, Cue, KICK, Layer, LayerKind, LayeredBed, SNARE, pattern, root_first,
};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, riser};
use bf_sound::synth::{self, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use crate::error::ContentError;

pub mod voices;

/// What a Studio theme sets besides its layers: its room, its master compressor and the levels of the show's
/// shared sounds.
#[derive(Clone, Copy, Debug)]
pub struct Room {
    pub reverb: (f64, f64, f64),
    pub comp: (f64, f64),
    pub roar: f64,
    pub doors: f64,
    pub landing: (f64, f64),
    pub pop: (f64, f64, f64),
}

/// The room most Studio themes use.
pub const ROOM: Room = Room {
    reverb: (1.6, 0.45, 0.18),
    comp: (0.15, 0.62),
    roar: 0.22,
    doors: 0.45,
    landing: (0.08, 0.06),
    pop: (95.0, 102.0, 0.12),
};

/// The frame: the classic bed's sections, played by the theme's layers.
pub struct Frame {
    name: &'static str,
    layers: Vec<Layer>,
    buses: Vec<(&'static str, (f64, f64))>,
    room: Room,
}

/// The levels the frame plays its grooves at.
type Levels<'a> = &'a [(&'a str, u8)];

impl Frame {
    /// A frame for the theme `name` on its `layers` (which must include `drums`, `bass` and `chords`).
    pub fn new(name: &'static str, layers: Vec<Layer>, room: Room) -> Frame {
        let mut buses: Vec<(&'static str, (f64, f64))> = Vec::new();
        for l in &layers {
            if l.bus != "fx" && !buses.iter().any(|(b, _)| *b == l.bus) {
                buses.push((l.bus, l.bus_mix));
            }
        }
        for n in ["drums", "bass", "chords"] {
            assert!(
                layers.iter().any(|l| l.name == n),
                "{name}: a Studio theme needs a `{n}` layer"
            );
        }
        Frame {
            name,
            layers,
            buses,
            room,
        }
    }

    fn layer(&self, name: &str) -> &Layer {
        self.layers.iter().find(|l| l.name == name).expect("checked in new")
    }

    fn tones(&self, ctx: &MusicCtx, l: &Layer, chord: &str) -> Result<Vec<i32>, MusicError> {
        let (lo, hi) = l.range;
        let tones = ctx
            .harmony
            .chord_tones(lo, hi, chord)
            .map_err(|e| MusicError::new(self.name, e.to_string()))?;
        Ok(if l.kind == LayerKind::Bass {
            root_first(tones, chord)
        } else {
            tones
        })
    }

    /// One note of the layer `name` at `t`, its gain times `g`.
    #[allow(clippy::too_many_arguments)]
    fn note(&self, ctx: &MusicCtx, mix: &mut Mix, name: &str, midi: i32, t: f64, dur: f64, vel: f64, g: f64) -> Played {
        let l = self.layer(name);
        let x = l.inst.strike(ctx, midi, dur, vel.min(1.0))?;
        mix.place(l.bus, &Sig::Mono(x), t.max(0.0), l.gain * g, l.pan);
        Ok(())
    }

    /// A band hit on `chord` at `t`: kick and crash, the bass's root and the chord.
    fn band_hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, chord: &str, g: f64) -> Played {
        let beat = ctx.show.grid.beat;
        self.note(ctx, mix, "drums", KICK, t, beat, 0.95, g)?;
        self.note(ctx, mix, "drums", CRASH, t, 2.0 * beat, 0.8, g)?;
        let bass = self.tones(ctx, self.layer("bass"), chord)?;
        if let Some(&r) = bass.first() {
            self.note(ctx, mix, "bass", r, t, 2.0 * beat, 0.9, g)?;
        }
        for m in self.tones(ctx, self.layer("chords"), chord)?.into_iter().take(4) {
            self.note(ctx, mix, "chords", m, t, 1.5 * beat, 0.8, g)?;
        }
        Ok(())
    }

    /// Bar `b` of a plain groove: each named layer's pattern at its level, on the chord at the bar's start.
    fn groove(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, levels: Levels) -> Played {
        let grid = &ctx.show.grid;
        let t0 = grid.bar_t(b as f64);
        let chord = ctx.harmony.chord_at(t0 + 1e-6).to_string();
        for &(name, d) in levels {
            let l = self.layer(name);
            let tones = if matches!(l.kind, LayerKind::Drums | LayerKind::Perc) {
                Vec::new()
            } else {
                self.tones(ctx, l, &chord)?
            };
            for h in pattern(l.kind, d, &tones, false, &|_| 0.5) {
                let t = t0 + h.s16 * grid.s16;
                if t < grid.dur {
                    let x = l.inst.strike(ctx, h.midi, h.len * grid.s16, h.vel)?;
                    mix.place(l.bus, &Sig::Mono(x), t, l.gain, l.pan);
                }
            }
        }
        Ok(())
    }

    fn roll(&self, ctx: &MusicCtx, mix: &mut Mix, t0: f64, t1: f64, g0: f64, g1: f64) -> Played {
        let step = ctx.show.grid.s16 / 2.0;
        let n = ((t1 - t0) / step).floor().max(0.0) as usize;
        for i in 0..n {
            let f = i as f64 / n.max(1) as f64;
            self.note(
                ctx,
                mix,
                "drums",
                SNARE,
                t0 + i as f64 * step,
                step,
                0.6,
                g0 + (g1 - g0) * f,
            )?;
        }
        Ok(())
    }

    fn first_chord(ctx: &MusicCtx) -> String {
        ctx.harmony
            .chords
            .first()
            .cloned()
            .unwrap_or_else(|| ctx.harmony.chord_at(0.0).to_string())
    }
}

impl Bed for Frame {
    fn hooks(&self) -> &'static [BedHook] {
        &BedHook::ALL
    }
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &self.buses
    }
    fn reverb(&self) -> (f64, f64, f64) {
        self.room.reverb
    }
    fn comp(&self) -> (f64, f64) {
        self.room.comp
    }
    fn roar_gain(&self) -> f64 {
        self.room.roar
    }
    fn doors_gain(&self) -> f64 {
        self.room.doors
    }
    fn landing(&self) -> (f64, f64) {
        self.room.landing
    }
    fn pop(&self) -> (f64, f64, f64) {
        self.room.pop
    }

    /// A band hit and a low thud.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        let ch = ctx.harmony.chord_at(t + 0.01).to_string();
        self.band_hit(ctx, mix, t, &ch, 0.9 * g)?;
        mix.place("fx", &Sig::Mono(synth::thud(0.9, 110.0, 36.0, 7)), t, 0.25 * g, 0.0);
        Ok(())
    }

    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, chord: &str, g: f64, _ring: f64) -> Played {
        self.band_hit(ctx, mix, t, chord, g)
    }

    fn press_roll(&self, ctx: &MusicCtx, mix: &mut Mix, t0: f64, t1: f64, g0: f64, g1: f64) -> Played {
        self.roll(ctx, mix, t0, t1, g0, g1)
    }

    fn erupt(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        self.note(ctx, mix, "drums", CRASH, t, 1.0, 1.0, 1.2)?;
        mix.place("fx", &Sig::Mono(synth::crash(2.0)), t, 0.12, 0.0);
        Ok(())
    }

    fn trailer_hit(
        &self,
        ctx: &MusicCtx,
        mix: &mut Mix,
        t: f64,
        taiko_g: f64,
        seed: i64,
        braam: Option<f64>,
        braam_g: f64,
    ) -> Played {
        self.note(ctx, mix, "drums", KICK, t, 0.5, 1.0, 1.2 * taiko_g)?;
        let thud = synth::thud(1.0, 90.0, 30.0, seed.unsigned_abs());
        mix.place("fx", &Sig::Mono(thud), t, 0.3 * taiko_g, 0.0);
        if let Some(d) = braam {
            let ch = ctx.harmony.chord_at(t + 0.01).to_string();
            if let Some(&r) = self.tones(ctx, self.layer("bass"), &ch)?.first() {
                self.note(ctx, mix, "bass", r, t, d, 1.0, 1.5 * braam_g)?;
            }
        }
        Ok(())
    }

    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        mix.place("fx", &Sig::Mono(synth::chime(91.0, 0.25)), t, 0.05, 0.0);
        Ok(())
    }

    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = Self::first_chord(ctx);
        for (tw, g) in times.into_iter().zip([0.85, 0.8, 0.8]) {
            self.band_hit(ctx, mix, tw, &ch, g)?;
        }
        Ok(())
    }

    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        mix.place("fx", &Sig::Mono(synth::thud(0.8, 160.0, 40.0, 91)), t, 0.35, 0.0);
        mix.place("fx", &Sig::Mono(synth::crash(1.2)), t, 0.08, 0.0);
        Ok(())
    }

    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        self.groove(ctx, mix, 0, &[("drums", 1), ("bass", 1), ("chords", 1)])?;
        let (d0, d1) = grid.t_doors;
        let whoosh = synth::whoosh(d1 - d0 + 0.1, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        mix.place("fx", &Sig::Mono(whoosh), (d0 - 0.05).max(0.0), 0.14, 0.0);
        Ok(())
    }

    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        self.groove(ctx, mix, 1, &[("bass", 1)])?;
        let ch = ctx.harmony.chord_at(grid.bar + 1e-6).to_string();
        for (tc, _) in &grid.count[..3] {
            self.band_hit(ctx, mix, *tc, &ch, 0.7)?;
        }
        let t3 = grid.bar + 3.0 * grid.beat;
        self.roll(ctx, mix, t3, grid.t_fight, 0.4, 0.9)?;
        let whoosh = synth::whoosh(grid.beat, 400.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
        mix.place("fx", &Sig::Mono(whoosh), t3, 0.12, 0.0);
        Ok(())
    }

    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let ch = Self::first_chord(ctx);
        self.band_hit(ctx, mix, ctx.show.grid.t_fight, &ch, 1.1)
    }

    /// (The layered bed plays the fight itself; this is the plain groove, for completeness.)
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        for b in ctx.show.grid.fight_bar..bar_ko {
            self.groove(ctx, mix, b, &[("drums", 2), ("bass", 2), ("chords", 2)])?;
        }
        Ok(())
    }

    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.t_ko;
        let ch = Self::first_chord(ctx);
        self.band_hit(ctx, mix, t, &ch, 1.0)?;
        mix.place("fx", &Sig::Mono(synth::thud(1.6, 120.0, 30.0, 51)), t, 0.35, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        let ch = Self::first_chord(ctx);
        self.band_hit(ctx, mix, tw, &ch, 1.0)
    }

    /// The band holds its breath but never goes quiet: the kit, the bass and the chords at their lightest.
    fn after_ko(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize, _tw: f64, t_solo: f64) -> Played {
        let b_solo = (t_solo / ctx.show.grid.bar).round().max(0.0) as usize;
        for b in bar_ko + 1..b_solo {
            self.groove(ctx, mix, b, &[("drums", 1), ("bass", 1), ("chords", 1)])?;
        }
        Ok(())
    }

    /// (The layered bed plays the victory itself.)
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            self.groove(ctx, mix, b, &[("drums", 1), ("bass", 1), ("chords", 1)])?;
        }
        Ok(())
    }

    /// A snare roll through the last bar's back half and a riser into the title slam on sample 0.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let t_last = grid.bar_t(grid.n_bars.saturating_sub(1) as f64);
        self.roll(ctx, mix, t_last + 2.0 * grid.beat, grid.dur - 1e-3, 0.3, 0.9)?;
        let r = riser(grid.dur - t_last, RISER_OVER, RISER_SEED)?;
        mix.place("fx", &Sig::Mono(r), t_last, 0.2, 0.0);
        Ok(())
    }
}

/// A Studio theme's bed: the Classic arrangement on `plain` with no cues, or the Composed one on `table`.
///
/// # Errors
/// A table naming a layer the theme lacks.
pub fn bed(
    name: &'static str,
    layers: fn() -> Vec<Layer>,
    room: Room,
    table: ArrangementTable,
    cues: Vec<Cue>,
    seed: u64,
) -> Result<Box<dyn Bed>, ContentError> {
    let frame = Frame::new(name, layers(), room);
    Ok(Box::new(LayeredBed::new(Box::new(frame), layers(), table, cues, seed)?))
}

/// A Studio theme's Composed bed on `table` with its own band, `groove` (sound upgrade, stage 2: the theme's own
/// patterns, swing and balance). The Classic bed ([`bed`]) never has one.
///
/// # Errors
/// A table naming a layer the theme lacks, or a groove badly written for its layers.
pub fn grooved(
    name: &'static str,
    layers: fn() -> Vec<Layer>,
    room: Room,
    table: ArrangementTable,
    cues: Vec<Cue>,
    seed: u64,
    groove: &'static Groove,
) -> Result<Box<dyn Bed>, ContentError> {
    let frame = Frame::new(name, layers(), room);
    let bed = LayeredBed::new(Box::new(frame), layers(), table, cues, seed)?.with_groove(groove)?;
    Ok(Box::new(bed))
}

/// The plain table a Studio theme's Classic arrangement plays: its layers at a steady level all through.
pub const fn plain(fight: &'static [(&'static str, u8)], build: &'static [(&'static str, u8)]) -> ArrangementTable {
    ArrangementTable {
        fight,
        fill: &[],
        breakdown: fight,
        final_exchange: fight,
        ability: &[],
        victory: fight,
        build,
    }
}
