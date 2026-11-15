//! The layer engine of the Composed arrangement: the pieces every theme's layered bed is made of, so a theme's
//! arrangement is a data table and not new code.
//!
//! - An [`Instrument`] is one of the theme's own sounds (its kit, its bass, its organ, ...): it strikes a note.
//! - A [`Layer`] is a pattern generator ([`LayerKind`]: drum groove, bass, chord comp, pad, arp, percussion) on one
//!   instrument, with density levels 1 to 3 (0 is silent).
//! - An [`ArrangementTable`] says, for each part of the song and each story moment, which layers play and how
//!   densely: a fill at each lead change, a breakdown at a desperation move, every layer in for the final
//!   exchange, a band hit under each captioned ability, the victory section and the build into frame 0. The K.O.
//!   bar tape-stops: what would play after the K.O. is dropped and the last beat before it sinks in pitch.
//! - A [`LayeredBed`] implements [`Bed`] from a table and the theme's classic bed, to which it hands every section
//!   it does not change (the title, the count-in, FIGHT!, the K.O. hit, the stinger, the bars after the K.O., the
//!   clashes and the hooks such as `boom` and `hit`).
//!
//! The story moments come in as [`Cue`]s, one per moment on the sixteenth grid. They are made from the story
//! signal (`StorySignal::from(ctx)`, `music/signal.rs`) when the Composed mode is wired into the build.
//!
//! Everything is deterministic: the only variation is drawn from the music seed, the bar and the layer.

use std::sync::Arc;

use crate::mix::{Mix, Sig};
use crate::music::{MusicCtx, MusicError, Samples};
use crate::traits::{Bed, BedHook, Played};

/// General MIDI drum numbers a drum or percussion [`Instrument`] is struck with.
pub const KICK: i32 = 36;
pub const SNARE: i32 = 38;
pub const CLAP: i32 = 39;
pub const HAT: i32 = 42;
pub const CRASH: i32 = 49;
pub const SHAKER: i32 = 70;

/// The highest density level.
pub const MAX_DENSITY: u8 = 3;

/// One of the theme's own sounds.
pub trait Instrument: Send + Sync {
    /// One note: MIDI note `midi` (a drum number for a kit) held `dur` seconds at velocity `vel` (0 to 1).
    fn strike(&self, ctx: &MusicCtx, midi: i32, dur: f64, vel: f64) -> Samples;
}

/// What a layer plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerKind {
    /// Kick, snare and hats.
    Drums,
    /// The chord's root, and at the top density its other tones.
    Bass,
    /// Chord stabs.
    Comp,
    /// The chord held through the bar.
    Pad,
    /// The chord's tones one after another.
    Arp,
    /// Shaker and clap.
    Perc,
}

impl LayerKind {
    fn tonal(self) -> bool {
        !matches!(self, LayerKind::Drums | LayerKind::Perc)
    }
}

/// A layer of the arrangement.
#[derive(Clone)]
pub struct Layer {
    /// Its name in the [`ArrangementTable`].
    pub name: &'static str,
    pub kind: LayerKind,
    pub inst: Arc<dyn Instrument>,
    /// The bus it plays on, and that bus's `(gain, reverb send)` in the master.
    pub bus: &'static str,
    pub bus_mix: (f64, f64),
    /// The gain and pan each note is placed with.
    pub gain: f64,
    pub pan: f64,
    /// The lowest and highest MIDI note a tonal layer plays.
    pub range: (i32, i32),
}

/// Layer names and their density levels.
pub type Levels = &'static [(&'static str, u8)];

/// Which layers play where. Every name must be one of the bed's layers.
#[derive(Clone, Copy, Debug)]
pub struct ArrangementTable {
    /// From FIGHT! to the K.O.
    pub fight: Levels,
    /// The layers that play their fill in a bar with a lead change (one that is silent there comes in at 2).
    pub fill: &'static [&'static str],
    /// The only layers in a bar with a desperation move.
    pub breakdown: Levels,
    /// The layers of the final exchange (all of them, as a rule).
    pub final_exchange: Levels,
    /// The layers that play a band hit under each captioned ability.
    pub ability: &'static [&'static str],
    /// Under the winner's solo.
    pub victory: Levels,
    /// The last bar, the build into frame 0.
    pub build: Levels,
}

/// A story moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CueKind {
    LeadChange,
    Ability,
    Desperation,
    FinalExchange,
}

/// A story moment in bar `bar` on sixteenth `s16` (0 to 15) of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cue {
    pub bar: usize,
    pub s16: u32,
    pub kind: CueKind,
}

/// The part of the song a bar is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Fight,
    Victory,
    Build,
}

/// What one bar plays: each sounding layer (by its index in the bed) and its density, whether the fill plays, and
/// the sixteenths of the band hits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BarPlan {
    pub layers: Vec<(usize, u8)>,
    pub fill: Vec<usize>,
    pub hits: Vec<(usize, u32)>,
}

/// One note of a pattern: its sixteenth, note, length in sixteenths and velocity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub s16: f64,
    pub midi: i32,
    pub len: f64,
    pub vel: f64,
}

/// A number in `[0, 1)` from the seed, the bar, the layer and a salt (splitmix64).
fn draw(seed: u64, bar: usize, layer: usize, salt: u64) -> f64 {
    let mut z = seed
        ^ (bar as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (layer as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ salt.wrapping_mul(0x94D0_49BB_1331_11EB);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

fn hit(s16: u32, midi: i32, len: f64, vel: f64) -> Hit {
    Hit {
        s16: s16 as f64,
        midi,
        len,
        vel,
    }
}

/// The pattern of a layer of kind `kind` at density `d` (1 to 3) over the chord tones `tones` (lowest first; empty
/// for drums), with the fill in its last beat when `fill`. `r(salt)` draws the bar's variation.
pub fn pattern(kind: LayerKind, d: u8, tones: &[i32], fill: bool, r: &dyn Fn(u64) -> f64) -> Vec<Hit> {
    let d = d.min(MAX_DENSITY);
    let mut out = Vec::new();
    if d == 0 {
        return out;
    }
    let root = tones.first().copied().unwrap_or(0);
    let chord: Vec<i32> = tones.iter().copied().take(4).collect();
    match kind {
        LayerKind::Drums => {
            out.extend([hit(0, KICK, 2.0, 0.9), hit(8, KICK, 2.0, 0.85)]);
            out.extend([hit(4, SNARE, 2.0, 0.8), hit(12, SNARE, 2.0, 0.8)]);
            if d >= 2 {
                out.extend(
                    (0..16)
                        .step_by(2)
                        .map(|s| hit(s, HAT, 1.0, if s % 4 == 0 { 0.55 } else { 0.45 })),
                );
            }
            if d >= 3 {
                out.extend((1..16).step_by(2).map(|s| hit(s, HAT, 1.0, 0.3)));
                out.push(hit(if r(1) < 0.5 { 10 } else { 6 }, KICK, 2.0, 0.7));
            }
            if fill {
                out.retain(|h| h.s16 < 12.0);
                out.push(hit(12, KICK, 2.0, 0.85));
                out.extend((12..16).map(|s| hit(s, SNARE, 1.0, 0.5 + 0.12 * (s - 12) as f64)));
            }
        }
        LayerKind::Perc => {
            if d >= 2 {
                out.extend((0..16).map(|s| hit(s, SHAKER, 1.0, if s % 2 == 1 { 0.4 } else { 0.25 })));
            } else {
                out.extend([2, 6, 10, 14].map(|s| hit(s, SHAKER, 1.0, 0.4)));
            }
            if d >= 3 {
                out.extend([hit(4, CLAP, 1.0, 0.7), hit(12, CLAP, 1.0, 0.7)]);
            }
            if fill {
                out.retain(|h| h.s16 < 12.0);
                out.extend((12..16).map(|s| hit(s, CLAP, 1.0, 0.45 + 0.1 * (s - 12) as f64)));
            }
        }
        _ if tones.is_empty() => {}
        LayerKind::Bass => match d {
            1 => out.push(hit(0, root, 15.0, 0.8)),
            2 => {
                let second = if r(2) < 0.5 {
                    root
                } else {
                    *chord.last().unwrap_or(&root)
                };
                out.extend([hit(0, root, 6.0, 0.8), hit(8, second, 6.0, 0.75)]);
            }
            _ => {
                for s in (0..16).step_by(2) {
                    let m = if (s == 6 || s == 14) && chord.len() > 1 {
                        chord[1 + (r(s as u64) * (chord.len() - 1) as f64) as usize]
                    } else {
                        root
                    };
                    out.push(hit(s, m, 1.8, if s % 4 == 0 { 0.85 } else { 0.7 }));
                }
            }
        },
        LayerKind::Comp => {
            let mut at: Vec<u32> = if d == 1 { vec![4, 12] } else { vec![2, 6, 10, 14] };
            if d >= 3 {
                at.push(if r(3) < 0.5 { 7 } else { 15 });
            }
            let len = if d == 1 { 2.0 } else { 1.5 };
            for s in at {
                out.extend(chord.iter().map(|&m| hit(s, m, len, 0.6)));
            }
        }
        LayerKind::Pad => {
            let vel = [0.3, 0.45, 0.6][d as usize - 1];
            out.extend(chord.iter().map(|&m| hit(0, m, 16.0, vel)));
        }
        LayerKind::Arp => {
            let step = [4, 2, 1][d as usize - 1];
            let down = r(4) < 0.5;
            for (i, s) in (0..16).step_by(step).enumerate() {
                let j = i % chord.len();
                let m = if down { chord[chord.len() - 1 - j] } else { chord[j] };
                out.push(hit(s, m, step as f64, if s % 4 == 0 { 0.6 } else { 0.45 }));
            }
        }
    }
    out
}

/// A band hit on sixteenth `s16`: crash and kick on a kit, a clap on percussion, the whole chord on the others.
pub fn band_hit(kind: LayerKind, tones: &[i32], s16: u32) -> Vec<Hit> {
    match kind {
        LayerKind::Drums => vec![hit(s16, CRASH, 8.0, 1.0), hit(s16, KICK, 2.0, 1.0)],
        LayerKind::Perc => vec![hit(s16, CLAP, 1.0, 1.0)],
        _ => tones.iter().take(4).map(|&m| hit(s16, m, 3.0, 0.9)).collect(),
    }
}

/// The Composed arrangement of a theme: its layers and table over its classic bed.
pub struct LayeredBed {
    classic: Box<dyn Bed>,
    layers: Vec<Layer>,
    table: ArrangementTable,
    cues: Vec<Cue>,
    seed: u64,
    buses: Vec<(&'static str, (f64, f64))>,
}

impl LayeredBed {
    /// A layered bed over `classic`.
    ///
    /// # Errors
    /// The table names a layer that is not in `layers`, or a density over [`MAX_DENSITY`].
    pub fn new(
        classic: Box<dyn Bed>,
        layers: Vec<Layer>,
        table: ArrangementTable,
        cues: Vec<Cue>,
        seed: u64,
    ) -> Result<LayeredBed, MusicError> {
        let known = |n: &str| layers.iter().any(|l| l.name == n);
        let levels = [
            table.fight,
            table.breakdown,
            table.final_exchange,
            table.victory,
            table.build,
        ];
        for &(n, d) in levels.iter().flat_map(|l| l.iter()) {
            if !known(n) {
                return Err(MusicError::new(
                    "layers",
                    format!("the table names a layer `{n}` the bed lacks"),
                ));
            }
            if d > MAX_DENSITY {
                return Err(MusicError::new(
                    "layers",
                    format!("`{n}` at density {d}, over {MAX_DENSITY}"),
                ));
            }
        }
        if let Some(n) = table.fill.iter().chain(table.ability).find(|n| !known(n)) {
            return Err(MusicError::new(
                "layers",
                format!("the table names a layer `{n}` the bed lacks"),
            ));
        }
        let mut buses = classic.buses().to_vec();
        for l in &layers {
            if !buses.iter().any(|(b, _)| *b == l.bus) {
                buses.push((l.bus, l.bus_mix));
            }
        }
        Ok(LayeredBed {
            classic,
            layers,
            table,
            cues,
            seed,
            buses,
        })
    }

    fn index(&self, name: &str) -> usize {
        self.layers.iter().position(|l| l.name == name).expect("checked in new")
    }

    /// What bar `bar` of section `section` plays.
    pub fn plan(&self, section: Section, bar: usize) -> BarPlan {
        let cues: Vec<&Cue> = self.cues.iter().filter(|c| c.bar == bar).collect();
        let has = |k: CueKind| cues.iter().any(|c| c.kind == k);
        let t = &self.table;
        let levels = match section {
            Section::Fight if has(CueKind::FinalExchange) => t.final_exchange,
            Section::Fight if has(CueKind::Desperation) => t.breakdown,
            Section::Fight => t.fight,
            Section::Victory => t.victory,
            Section::Build => t.build,
        };
        let mut layers: Vec<(usize, u8)> = levels
            .iter()
            .filter(|(_, d)| *d > 0)
            .map(|&(n, d)| (self.index(n), d))
            .collect();
        let mut fill = Vec::new();
        let mut hits = Vec::new();
        if section == Section::Fight {
            if has(CueKind::LeadChange) {
                for n in t.fill {
                    let i = self.index(n);
                    if !layers.iter().any(|(j, _)| *j == i) {
                        layers.push((i, 2));
                    }
                    fill.push(i);
                }
            }
            for c in cues.iter().filter(|c| c.kind == CueKind::Ability) {
                hits.extend(t.ability.iter().map(|n| (self.index(n), c.s16.min(15))));
            }
        }
        BarPlan { layers, fill, hits }
    }

    /// Plays bar `bar` of `section` into `mix`; nothing at or after `stop` (the K.O.), and the last beat before
    /// it sinks in pitch.
    fn play_bar(&self, ctx: &MusicCtx, mix: &mut Mix, section: Section, bar: usize, stop: Option<f64>) -> Played {
        let grid = &ctx.show.grid;
        let t0 = grid.bar_t(bar as f64);
        let plan = self.plan(section, bar);
        let chord = ctx.harmony.chord_at(t0 + 1e-6).to_string();
        for (i, layer) in self.layers.iter().enumerate() {
            let tones = if layer.kind.tonal() {
                let (lo, hi) = layer.range;
                ctx.harmony
                    .chord_tones(lo, hi, &chord)
                    .map_err(|e| MusicError::new("layers", e.to_string()))?
            } else {
                Vec::new()
            };
            let mut notes = Vec::new();
            if let Some(&(_, d)) = plan.layers.iter().find(|(j, _)| *j == i) {
                let r = |salt: u64| draw(self.seed, bar, i, salt);
                notes = pattern(layer.kind, d, &tones, plan.fill.contains(&i), &r);
            }
            for &(_, s) in plan.hits.iter().filter(|(j, _)| *j == i) {
                notes.extend(band_hit(layer.kind, &tones, s));
            }
            for h in notes {
                let t = t0 + h.s16 * grid.s16;
                let (mut midi, mut vel, mut dur) = (h.midi, h.vel, h.len * grid.s16);
                if let Some(ko) = stop {
                    if t >= ko {
                        continue;
                    }
                    dur = dur.min(ko - t);
                    let sink = (t - (ko - grid.beat)) / grid.beat;
                    if sink > 0.0 {
                        if layer.kind.tonal() {
                            midi -= (sink * 7.0).round() as i32;
                        }
                        vel *= 1.0 - 0.5 * sink;
                    }
                }
                let x = layer.inst.strike(ctx, midi, dur, vel)?;
                mix.place(layer.bus, &Sig::Mono(x), t, layer.gain, layer.pan);
            }
        }
        Ok(())
    }
}

impl Bed for LayeredBed {
    fn hooks(&self) -> &'static [BedHook] {
        self.classic.hooks()
    }
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &self.buses
    }
    fn reverb(&self) -> (f64, f64, f64) {
        self.classic.reverb()
    }
    fn comp(&self) -> (f64, f64) {
        self.classic.comp()
    }
    fn roar_gain(&self) -> f64 {
        self.classic.roar_gain()
    }
    fn doors_gain(&self) -> f64 {
        self.classic.doors_gain()
    }
    fn landing(&self) -> (f64, f64) {
        self.classic.landing()
    }
    fn pop(&self) -> (f64, f64, f64) {
        self.classic.pop()
    }
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        self.classic.boom(ctx, mix, t, g)
    }
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, chord: &str, g: f64, ring: f64) -> Played {
        self.classic.hit(ctx, mix, t, chord, g, ring)
    }
    fn press_roll(&self, ctx: &MusicCtx, mix: &mut Mix, t0: f64, t1: f64, g0: f64, g1: f64) -> Played {
        self.classic.press_roll(ctx, mix, t0, t1, g0, g1)
    }
    fn erupt(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        self.classic.erupt(ctx, mix, t)
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
        self.classic.trailer_hit(ctx, mix, t, taiko_g, seed, braam, braam_g)
    }
    fn has_pit(&self) -> bool {
        self.classic.has_pit()
    }
    fn timp(&self, m: i32) -> Samples {
        self.classic.timp(m)
    }
    fn marimba(&self, m: i32) -> Samples {
        self.classic.marimba(m)
    }
    fn clash(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        self.classic.clash(ctx, mix, t)
    }
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        self.classic.title(ctx, mix, times)
    }
    fn vs(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        self.classic.vs(ctx, mix, t)
    }
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        self.classic.title_rest(ctx, mix)
    }
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        self.classic.count_in(ctx, mix)
    }
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        self.classic.fight_start(ctx, mix)
    }
    /// The layers from FIGHT! up to the K.O. in bar `bar_ko`, which tape-stops at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        for b in ctx.show.grid.fight_bar..=bar_ko {
            let stop = (b == bar_ko).then_some(ctx.show.t_ko);
            self.play_bar(ctx, mix, Section::Fight, b, stop)?;
        }
        Ok(())
    }
    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        self.classic.ko(ctx, mix)
    }
    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        self.classic.stinger(ctx, mix, tw)
    }
    fn after_ko(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        self.classic.after_ko(ctx, mix, bar_ko, tw, t_solo)
    }
    /// The victory layers from bar `b_solo` up to the last bar.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            self.play_bar(ctx, mix, Section::Victory, b, None)?;
        }
        Ok(())
    }
    /// The build layers in the last bar, up to the seam.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        self.play_bar(ctx, mix, Section::Build, grid.n_bars.saturating_sub(1), Some(grid.dur))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::testkit::{TestBed, context, episode};

    /// A toy instrument: a few samples that carry the note and the velocity.
    struct Toy;

    impl Instrument for Toy {
        fn strike(&self, _: &MusicCtx, midi: i32, _: f64, vel: f64) -> Samples {
            Ok(vec![midi as f64 * vel / 100.0; 4])
        }
    }

    fn layer(name: &'static str, kind: LayerKind) -> Layer {
        Layer {
            name,
            kind,
            inst: Arc::new(Toy),
            bus: name,
            bus_mix: (1.0, 0.1),
            gain: 1.0,
            pan: 0.0,
            range: (36, 60),
        }
    }

    const TABLE: ArrangementTable = ArrangementTable {
        fight: &[("drums", 2), ("bass", 1)],
        fill: &["drums"],
        breakdown: &[("bass", 1)],
        final_exchange: &[("drums", 3), ("bass", 3)],
        ability: &["bass"],
        victory: &[("bass", 2)],
        build: &[("drums", 3)],
    };

    fn bed(cues: Vec<Cue>, seed: u64) -> LayeredBed {
        let layers = vec![layer("drums", LayerKind::Drums), layer("bass", LayerKind::Bass)];
        LayeredBed::new(Box::new(TestBed::new((2.0, 0.5, 0.2))), layers, TABLE, cues, seed).unwrap()
    }

    fn cue(bar: usize, s16: u32, kind: CueKind) -> Cue {
        Cue { bar, s16, kind }
    }

    /// Whether bus `bus` has sound in bar `b`.
    fn sounds(ctx: &MusicCtx, mix: &Mix, bus: &str, b: usize) -> bool {
        let grid = &ctx.show.grid;
        let (i0, i1) = (
            (grid.bar_t(b as f64) * crate::SRF) as usize,
            (grid.bar_t(b as f64 + 1.0) * crate::SRF) as usize,
        );
        mix.bus(bus).is_some_and(|x| x[i0..i1].iter().any(|f| f[0] != 0.0))
    }

    #[test]
    fn the_right_layers_sound_in_the_right_bars() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        let fb = ctx.show.grid.fight_bar;
        let bed = bed(
            vec![
                cue(fb + 2, 0, CueKind::Desperation),
                cue(fb + 3, 8, CueKind::FinalExchange),
            ],
            7,
        );
        assert_eq!(bed.plan(Section::Fight, fb).layers, [(0, 2), (1, 1)]);
        assert_eq!(bed.plan(Section::Fight, fb + 2).layers, [(1, 1)]);
        assert_eq!(bed.plan(Section::Fight, fb + 3).layers, [(0, 3), (1, 3)]);
        assert_eq!(bed.plan(Section::Victory, 0).layers, [(1, 2)]);
        let mut mix = Mix::new(ctx.show.grid.dur);
        bed.fight(&ctx, &mut mix, fb + 4).unwrap();
        assert!(sounds(&ctx, &mix, "drums", fb) && sounds(&ctx, &mix, "bass", fb));
        assert!(!sounds(&ctx, &mix, "drums", fb + 2) && sounds(&ctx, &mix, "bass", fb + 2));
        assert!(!sounds(&ctx, &mix, "drums", fb + 5));
        // the layers' buses join the classic bed's
        let names: Vec<&str> = bed.buses().iter().map(|(n, _)| *n).collect();
        assert_eq!(names, ["drums", "fx", "bass"]);
    }

    #[test]
    fn a_lead_change_adds_a_fill_and_an_ability_a_band_hit() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        let fb = ctx.show.grid.fight_bar;
        let bed = bed(
            vec![cue(fb + 1, 4, CueKind::LeadChange), cue(fb + 1, 6, CueKind::Ability)],
            7,
        );
        let plain = bed.plan(Section::Fight, fb);
        let lead = bed.plan(Section::Fight, fb + 1);
        assert!(plain.fill.is_empty() && plain.hits.is_empty());
        assert_eq!((lead.fill.as_slice(), lead.hits.as_slice()), (&[0][..], &[(1, 6)][..]));
        let r = |_: u64| 0.0;
        let snares = |fill| {
            pattern(LayerKind::Drums, 2, &[], fill, &r)
                .iter()
                .filter(|h| h.midi == SNARE && h.s16 >= 12.0)
                .count()
        };
        assert_eq!((snares(false), snares(true)), (1, 4));
    }

    #[test]
    fn the_same_seed_gives_the_same_samples() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        let fb = ctx.show.grid.fight_bar;
        let render = |seed| {
            let bed = bed(vec![cue(fb + 1, 0, CueKind::FinalExchange)], seed);
            let mut mix = Mix::new(ctx.show.grid.dur);
            bed.fight(&ctx, &mut mix, fb + 3).unwrap();
            bed.last_bars(&ctx, &mut mix).unwrap();
            (mix.bus("drums").unwrap().to_vec(), mix.bus("bass").unwrap().to_vec())
        };
        assert_eq!(render(11), render(11));
    }

    #[test]
    fn a_table_naming_a_missing_layer_is_refused() {
        let table = ArrangementTable {
            ability: &["horns"],
            ..TABLE
        };
        let r = LayeredBed::new(Box::new(TestBed::new((2.0, 0.5, 0.2))), vec![], table, vec![], 0);
        assert!(r.is_err());
    }
}
