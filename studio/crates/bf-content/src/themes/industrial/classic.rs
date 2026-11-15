//! The industrial bed's classic arrangement: the bed of `projects/ballfight_barisax_vs_bassclarinet/audio.py`'s
//! `main()` (the parts that belong to the bed, not to BARI SAX or BASS CLARINET), section by section as the score
//! calls them. A quick port: by ear, not sample for sample.
//!
//! **The fight's windows.** The episode reads the run: the kick steps back on the beats where a swing honk bottoms
//! out, the bari's busy stretches (a hook up on the crane, CABLE SNAP's hoist) and a machine hit under each captioned
//! ability. The bed is made before it sees the run, so [`Site::of`] reads them from `ctx.rec` and `ctx.show` when the
//! fight is played. Any fighter's swings and hoists count, so the site answers whoever swings.

use std::collections::BTreeMap;

use bf_core::harmony::HarmonyError;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, event_is, event_number, riser, thunder};
use bf_sound::synth::{self, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{GRIND_16, PIPE_PAT, anvil, backbeat, grind, grind_root, hiss, kick, pipe, sheet, tick, wrench};

/// The name the bed's errors carry.
const H: &str = "industrial";

/// The bed's own buses, which the K.O. tape-stops.
const BUSES: [&str; 3] = ["drums", "metal", "grind"];

/// How much louder the site plays after the K.O. (the holding bars, the victory and the build): #40 had the
/// fighters' solo and pump over it; under other fighters it fell 4 to 5 dB below the fight, so the bed carries it.
const VICTORY_LIFT: f64 = 1.7;

/// The industrial bed.
#[derive(Clone, Debug, Default)]
pub struct IndustrialClassic;

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

fn chord<'a>(ctx: &'a MusicCtx, b: usize) -> Result<&'a str, MusicError> {
    let chords = &ctx.harmony.chords;
    chords
        .get(b % chords.len().max(1))
        .map(String::as_str)
        .ok_or_else(|| MusicError::new(H, format!("bar {b} has no chord ({} bars)", chords.len())))
}

fn put(mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64, pan: f64) {
    mix.place(bus, &Sig::Mono(x), t, gain, pan);
}

/// What the bed reads from the run before it plays the fight.
#[derive(Clone, Debug)]
struct Site {
    /// `duck`: per bar, the beats where a swing honk bottoms out (the kick steps back there).
    duck: BTreeMap<usize, Vec<usize>>,
    /// `busy_windows()`: a ball up on the crane (a swing's hoist to its end, CABLE SNAP's hoist to the crash).
    busy: Vec<(f64, f64)>,
    /// How much louder the bars play: 1, or [`VICTORY_LIFT`] after the K.O.
    lift: f64,
}

impl Default for Site {
    fn default() -> Site {
        Site {
            duck: BTreeMap::new(),
            busy: Vec::new(),
            lift: 1.0,
        }
    }
}

impl Site {
    /// The bars after the K.O., [`VICTORY_LIFT`] louder.
    fn lifted() -> Site {
        Site {
            lift: VICTORY_LIFT,
            ..Site::default()
        }
    }
}

impl Site {
    fn of(ctx: &MusicCtx) -> Site {
        let g = &ctx.show.grid;
        let t_ko = ctx.show.t_ko;
        let ev = &ctx.rec.events;
        let mut site = Site::default();
        for (i, e) in ev.iter().enumerate() {
            if e.t > t_ko {
                break;
            }
            match e.kind.as_str() {
                "swing" => {
                    let (Ok(a), Ok(b)) = (event_number(H, e, 11), event_number(H, e, 12)) else {
                        continue;
                    };
                    site.busy.push((a - 0.3, b + 0.05));
                    if event_is(H, e, 13, 0).unwrap_or(false) {
                        let tb = ctx.q16((a + b) / 2.0);
                        let bar = (tb / g.bar).floor().max(0.0) as usize;
                        let beat = ((tb - bar as f64 * g.bar) / g.beat).round() as usize % 4;
                        site.duck.entry(bar).or_default().push(beat);
                    }
                }
                "snap_aim" => {
                    let crash = ev[i..]
                        .iter()
                        .find(|q| q.kind == "crater" && q.who() == e.who())
                        .map_or(e.t + 1.0, |q| q.t);
                    site.busy.push((e.t - 0.02, crash + 0.3));
                }
                _ => {}
            }
        }
        site
    }

    fn busy_at(&self, t: f64) -> bool {
        self.busy.iter().any(|&(a, b)| a <= t && t <= b)
    }
}

impl IndustrialClassic {
    /// The bed.
    pub fn new() -> IndustrialClassic {
        IndustrialClassic
    }

    /// `band_bar(mix, b, dense, light, duck)`: one bar of the construction site. `dense` 0..1 thickens the
    /// arrangement (16th ticks, a third kick, pipe ghosts, grinder fills, sheet metal every 2 bars), never the level.
    fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, dense: f64, light: bool, site: &Site) -> Played {
        let put = |mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64, pan: f64| {
            put(mix, bus, x, t, gain * site.lift, pan)
        };
        let g = &ctx.show.grid;
        let (beat, s16) = (g.beat, g.s16);
        let t0 = b as f64 * g.bar;
        let ch = chord(ctx, b)?;
        let tones = ctx.harmony.chord_tones(67, 81, ch).map_err(bad)?;
        let duck = site.duck.get(&b).map_or(&[][..], Vec::as_slice);
        let bu = b as u64;
        for bt in 0..4usize {
            let tb = t0 + bt as f64 * beat;
            let btu = bt as u64;
            if bt == 0 || bt == 2 {
                put(
                    mix,
                    "drums",
                    kick(),
                    tb,
                    if duck.contains(&bt) { 0.12 } else { 0.34 },
                    0.0,
                );
            } else if dense > 0.8 && !light {
                put(mix, "drums", anvil(bu + btu + 7), tb + beat / 2.0, 0.10, -0.2);
            }
            if bt == 2 && dense > 0.4 && !light && !site.busy_at(tb + beat / 2.0) {
                put(mix, "drums", kick(), tb + beat / 2.0, 0.18, 0.0);
            }
            if bt == 1 || bt == 3 {
                put(mix, "drums", backbeat(bu * 4 + btu), tb, 0.34, 0.0);
            }
            for e in 0..4u64 {
                let te = tb + e as f64 * s16;
                if e % 2 == 0 {
                    put(mix, "drums", tick(bu + btu + e, e == 2 && dense > 0.3), te, 0.11, 0.3);
                } else if dense > 0.5 && !light {
                    put(mix, "drums", tick(bu + e, false), te, 0.05, 0.35);
                }
            }
        }
        if !tones.is_empty() {
            for (j, ix) in PIPE_PAT.iter().enumerate() {
                let m = tones[ix % tones.len()];
                let t = t0 + j as f64 * beat / 2.0;
                let gain = if j % 2 == 0 { 0.15 } else { 0.11 };
                put(
                    mix,
                    "metal",
                    pipe(m, bu + j as u64),
                    t,
                    gain,
                    -0.3 + 0.08 * (j % 3) as f64,
                );
                if dense > 0.3 && j % 2 == 1 && !light {
                    put(
                        mix,
                        "metal",
                        pipe((m + 7).min(86), bu + j as u64 + 5),
                        t + s16,
                        0.05,
                        0.3,
                    );
                }
            }
        }
        let root = grind_root(ch).map_err(bad)?;
        let fills = dense > 0.6 && !light;
        for p16 in GRIND_16
            .iter()
            .copied()
            .chain(if fills { vec![14, 15] } else { vec![] })
        {
            let t = t0 + p16 as f64 * s16;
            if p16 >= 14 && site.busy_at(t) {
                continue;
            }
            put(
                mix,
                "grind",
                grind(root, 0.075, p16 as u64),
                t,
                if p16 > 0 { 0.11 } else { 0.13 },
                0.2,
            );
        }
        if b % 4 == 2 || (dense > 0.5 && b.is_multiple_of(2) && !light) {
            put(mix, "metal", sheet(bu % 5, 1.2), t0, 0.9, 0.0);
        }
        if b % 4 == 1 && !light {
            put(mix, "fx", wrench(0.38, bu), t0 + 3.0 * beat, 0.16, -0.25);
            put(mix, "fx", hiss(0.5, bu), t0 + 3.5 * beat, 0.12, 0.3);
        }
        Ok(())
    }

    /// `band_hit(mix, t, ch, g, metal)`: a unison machine hit: the kick, sheet metal, the anvil, a pipe chord, a
    /// grinder stab and a deep thud. `metal` scales the bright parts.
    fn hit_at(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, metal: f64) -> Played {
        let ts = (t * 10.0).max(0.0) as u64;
        put(mix, "drums", kick(), t, 0.5 * g, 0.0);
        put(mix, "drums", backbeat(ts), t, 0.30 * g * metal, 0.0);
        if metal > 0.0 {
            put(
                mix,
                "metal",
                sheet((t * 7.0).max(0.0) as u64 % 5, 1.6),
                t,
                1.1 * g * metal,
                0.0,
            );
        }
        let tones = ctx.harmony.chord_tones(67, 79, ch).map_err(bad)?;
        for (j, &m) in tones.iter().take(3).enumerate() {
            put(
                mix,
                "metal",
                pipe(m, j as u64),
                t,
                0.16 * g * metal.max(0.5),
                (j as f64 - 1.0) * 0.3,
            );
        }
        let root = grind_root(ch).map_err(bad)?;
        put(mix, "grind", grind(root, 0.28, 1), t, 0.16 * g * metal.max(0.4), 0.2);
        let seed = (t * 50.0).max(0.0) as u64 + 3;
        put(mix, "fx", synth::thud(0.6, 110.0, 36.0, seed), t, 0.15 * g, 0.0);
        Ok(())
    }
}

impl Bed for IndustrialClassic {
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::Erupt]
    }

    /// `BUS_GAIN` and the reverb sends of the episode's master, for the bed's buses.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[("drums", (1.0, 0.15)), ("metal", (1.0, 0.4)), ("grind", (1.0, 0.15))]
    }

    /// A big shed: the clangs ring.
    fn reverb(&self) -> (f64, f64, f64) {
        (1.8, 0.5, 0.20)
    }

    fn comp(&self) -> (f64, f64) {
        (0.15, 0.62)
    }

    fn roar_gain(&self) -> f64 {
        0.22
    }

    fn doors_gain(&self) -> f64 {
        0.45
    }

    fn landing(&self) -> (f64, f64) {
        (0.08, 0.06)
    }

    fn pop(&self) -> (f64, f64, f64) {
        (95.0, 102.0, 0.12)
    }

    /// A big hit (a crater, a slam): a full machine hit on the chord there.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        let ch = ctx.harmony.chord_at(t + 0.01).to_string();
        self.hit_at(ctx, mix, t, &ch, g, 1.0)
    }

    /// A crash (LIGHTS OUT's eruption): thunder and a sheet-metal crash.
    fn erupt(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", thunder(1.6, (t * 10.0).max(0.0) as u64, true)?, t, 0.45, 0.0);
        put(mix, "metal", sheet(4, 1.4), t, 1.0, 0.0);
        Ok(())
    }

    /// The two balls meeting: a light thud.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(
            mix,
            "fx",
            synth::thud(0.15, 200.0, 90.0, (t * 3.0).max(0.0) as u64),
            t,
            0.05,
            0.0,
        );
        Ok(())
    }

    /// A machine hit under every word of the VS card: the first full, the other two lighter (v2: less clang).
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = chord(ctx, 0)?.to_string();
        for (j, tw) in times.into_iter().enumerate() {
            let (g, metal) = if j == 0 { (0.8, 0.75) } else { (0.72, 0.5) };
            self.hit_at(ctx, mix, tw, &ch, g, metal)?;
        }
        Ok(())
    }

    /// VS: the thunder crack where it splits the divider.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", thunder(1.4, 91, true)?, t, 0.40, 0.0);
        Ok(())
    }

    /// The groove plays under the card (a sparser bar), an air wrench as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let g = &ctx.show.grid;
        self.bar(ctx, mix, 0, 0.6, false, &Site::default())?;
        let (d0, d1) = g.t_doors;
        put(mix, "fx", wrench(d1 - d0 + 0.1, 3), d0 - 0.05, 0.18, 0.0);
        let w = synth::whoosh(d1 - d0 + 0.1, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", w, d0 - 0.05, 0.12, 0.0);
        Ok(())
    }

    /// The count-in on the second chord: a machine hit on each count (no clang), an anvil fill and a hiss into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let g = &ctx.show.grid;
        self.bar(ctx, mix, 1, 0.65, false, &Site::default())?;
        let ch = chord(ctx, 1)?.to_string();
        for (tc, _) in g.count.iter().take(3) {
            self.hit_at(ctx, mix, *tc, &ch, 0.8, 0.55)?;
        }
        for j in 0..4u64 {
            let t = g.bar + 3.0 * g.beat + j as f64 * g.s16;
            put(mix, "drums", anvil(j), t, 0.18 + 0.04 * j as f64, 0.0);
        }
        put(mix, "fx", hiss(g.beat, 9), g.bar + 3.0 * g.beat, 0.10, 0.0);
        Ok(())
    }

    /// FIGHT!: the drop, a machine hit and a rumble.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.grid.t_fight;
        let ch = chord(ctx, ctx.show.grid.fight_bar)?.to_string();
        self.hit_at(ctx, mix, t, &ch, 1.0, 0.8)?;
        put(mix, "fx", thunder(1.6, 7, false)?, t, 0.35, 0.0);
        Ok(())
    }

    /// The site from FIGHT! to the K.O., thickening as it goes, a machine hit under each captioned ability, the
    /// tape stop at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let g = &ctx.show.grid;
        let site = Site::of(ctx);
        let ramp = 18.0 * g.n_bars as f64 / 32.0;
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, ((b - 2) as f64 / ramp).min(1.0), false, &site)?;
        }
        let t_ko = ctx.show.t_ko;
        let mut caps: Vec<f64> = ctx.show.caps_ability.iter().flatten().map(|c| c.t0).collect();
        caps.sort_by(f64::total_cmp);
        for t in caps.into_iter().filter(|&t| t < t_ko) {
            let ch = ctx.harmony.chord_at(t).to_string();
            self.hit_at(ctx, mix, ctx.q16(t), &ch, 0.55, 1.0)?;
        }
        for bus in BUSES {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    /// The K.O.: a full machine hit and a long thunder.
    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        let ch = chord(ctx, 0)?.to_string();
        self.hit_at(ctx, mix, t_ko, &ch, 1.0, 1.0)?;
        put(mix, "fx", thunder(2.4, 51, true)?, t_ko, 0.55, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        let ch = chord(ctx, 0)?.to_string();
        self.hit_at(ctx, mix, tw, &ch, 1.0, 1.0)
    }

    /// The site holds its breath up to the solo, but never goes quiet: ticks, the grinder, the pipes, the kick and
    /// the backbeat, a sheet crash a bar.
    fn after_ko(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize, _tw: f64, t_solo: f64) -> Played {
        let put = |mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64, pan: f64| {
            put(mix, bus, x, t, gain * VICTORY_LIFT, pan)
        };
        let g = &ctx.show.grid;
        let b_solo = (t_solo / g.bar).round().max(0.0) as usize;
        for b in bar_ko + 1..b_solo.min(g.n_bars) {
            let tq = b as f64 * g.bar;
            let ch = chord(ctx, b)?;
            let root = grind_root(ch).map_err(bad)?;
            let bu = b as u64;
            for bt in 0..4u64 {
                let tb = tq + bt as f64 * g.beat;
                put(mix, "drums", tick(bt, false), tb, 0.12, 0.3);
                put(mix, "drums", tick(bt + 1, false), tb + g.beat / 2.0, 0.09, 0.3);
                put(mix, "grind", grind(root, 0.075, bt), tb + 3.0 * g.s16, 0.10, 0.2);
            }
            let tones = ctx.harmony.chord_tones(67, 81, ch).map_err(bad)?;
            let tones: Vec<i32> = tones.into_iter().take(3).collect();
            if !tones.is_empty() {
                for (j, ix) in PIPE_PAT.iter().enumerate() {
                    let m = tones[ix % tones.len()];
                    put(
                        mix,
                        "metal",
                        pipe(m, j as u64),
                        tq + j as f64 * g.beat / 2.0,
                        0.13,
                        -0.2,
                    );
                }
            }
            put(mix, "drums", kick(), tq, 0.42, 0.0);
            put(mix, "drums", kick(), tq + 2.0 * g.beat, 0.36, 0.0);
            put(mix, "drums", backbeat(bu), tq + g.beat, 0.26, 0.0);
            put(mix, "drums", backbeat(bu + 1), tq + 3.0 * g.beat, 0.26, 0.0);
            put(mix, "metal", sheet(bu % 5, 1.2), tq, 0.8, 0.0);
        }
        Ok(())
    }

    /// Lighter machinery under the solo, up to the last two bars.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        let site = Site::lifted();
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(2) {
            self.bar(ctx, mix, b, 0.85, false, &site)?;
        }
        Ok(())
    }

    /// The last two bars: the site returns at full density, a machine hit as both glide home, an anvil roll, a riser
    /// and an air wrench into the title slam on sample 0.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let put = |mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64, pan: f64| {
            put(mix, bus, x, t, gain * VICTORY_LIFT, pan)
        };
        let g = &ctx.show.grid;
        let site = Site::lifted();
        let n = g.n_bars;
        for b in n.saturating_sub(2)..n {
            self.bar(ctx, mix, b, 1.0, false, &site)?;
        }
        let ch = ctx.harmony.chord_at(g.t_glide).to_string();
        self.hit_at(ctx, mix, g.t_glide, &ch, 0.7, 1.0)?;
        let t_last = n.saturating_sub(1) as f64 * g.bar;
        let mut st = 0.0;
        let mut k = 0u64;
        while t_last + 2.0 * g.beat + st < g.dur - 1e-4 {
            put(
                mix,
                "drums",
                anvil(k),
                t_last + 2.0 * g.beat + st,
                0.10 + 0.14 * st / (2.0 * g.beat),
                0.0,
            );
            st += g.s16;
            k += 1;
        }
        put(
            mix,
            "fx",
            riser(g.dur - t_last, RISER_OVER, RISER_SEED)?,
            t_last,
            0.20,
            0.0,
        );
        put(mix, "fx", wrench(0.5, 77), g.dur - 0.55, 0.16, 0.0);
        Ok(())
    }
}
