//! The Balkan fire's classic arrangement: the bed of #28's `audio.py` (`bed()` and the bed's lines of `main()`) on
//! the kit's beat sheet (the VS card on bar 0, the count-in on bar 1, FIGHT! on bar 2), section by section as
//! `kit.music.score()` calls them. Quick-ported by ear: the same instruments, patterns and levels, not the same
//! samples. #28 had no VS card: its title hits are the band's oom-pah on the tonic (bass, accordion, a cimbalom
//! flourish). The few strokes #28 played on the DRUMS fighter's own kit (the pickup into FIGHT!, the crash, the last
//! bar's kicks and roll) are a plain snare, kick and crash here, so the bed sounds the same under any pair.

use bf_core::dmath;
use bf_core::harmony::HarmonyError;
use bf_core::py::py_max;
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, noise_seed, riser};
use bf_sound::synth::{self, FILTER_O, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{MUSETTE, bass, cimb, cimbalom, reed, roll, stab, sticks, upright, voicing};

/// How much louder the bars under the solo play: the episode's master lifted its victory 3 dB (`GAIN_PTS`), the
/// kit's has no such lift, so the bed does it.
const VICTORY_LIFT: f64 = 1.5;

/// The name the bed's errors carry.
const H: &str = "balkan_fire";

/// The bed's own buses: `BUS_GAIN` and the reverb sends of #28's `master()`, and a quiet `perc` for the few snare,
/// stick and crash strokes.
const BUSES: [(&str, (f64, f64)); 5] = [
    ("bass", (0.8, 0.0)),
    ("accordion", (2.0, 0.4)),
    ("pad", (1.2, 0.5)),
    ("cimbalom", (1.4, 0.6)),
    ("perc", (1.0, 0.15)),
];

/// The Balkan fire's bed.
#[derive(Clone, Debug, Default)]
pub struct BalkanFireClassic;

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

fn put(mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64, pan: f64) {
    mix.place(bus, &Sig::Mono(x), t, gain, pan);
}

/// `T.CHORDS[b]`.
fn chord<'a>(ctx: &'a MusicCtx, b: usize) -> Result<&'a str, MusicError> {
    let chords = &ctx.harmony.chords;
    chords.get(b).map(String::as_str).ok_or_else(|| {
        MusicError::new(
            H,
            format!("bar {b} is past the episode's last bar ({} bars)", chords.len()),
        )
    })
}

/// The key's tonic at or above `lo` (D1 = 26 for `lo` 24 in D).
fn tonic(ctx: &MusicCtx, lo: i32) -> i32 {
    lo + (ctx.harmony.key_pc - lo).rem_euclid(12)
}

/// The chord from `lo`'s root up: its root, then its tones over two octaves (#28's stinger: D3 A3 D4 F4 A4).
fn spread(ctx: &MusicCtx, ch: &str, lo: i32, n: usize) -> Result<Vec<i32>, MusicError> {
    let root = ctx.harmony.root_above(ch, lo).map_err(bad)?;
    let mut ms = vec![root];
    let tones = ctx.harmony.chord_tones(root + 1, root + 24, ch).map_err(bad)?;
    // the fifth first, as #28 voiced it (root, fifth, octave, third, fifth)
    let fifth = tones.iter().copied().find(|m| m - root == 7);
    ms.extend(fifth);
    ms.extend(tones.iter().copied().filter(|m| Some(*m) != fifth && m - root > 7));
    ms.truncate(n);
    Ok(ms)
}

/// The VS crack: band-passed noise with a low rumble under it (seeds 91 and 92, as the kit's beds).
fn crack() -> Vec<f64> {
    let nn = (1.4 * SRF) as usize;
    let tt = synth::tt(nn);
    let mut draw = vec![0.0; nn];
    Generator::new(91).fill_normal(0.0, 1.0, &mut draw);
    let crack = synth::bp(&draw, 1500.0, 9000.0, FILTER_O);
    Generator::new(92).fill_normal(0.0, 1.0, &mut draw);
    let rumble = synth::lp(&draw, 160.0, FILTER_O);
    (0..nn)
        .map(|i| {
            let crack = crack[i] * dmath::exp(-tt[i] / 0.035);
            let rumble = rumble[i] * (tt[i] / 0.02).clamp(0.0, 1.0) * dmath::exp(-tt[i] / 0.5) * 3.0;
            crack * 0.9 + rumble
        })
        .collect()
}

fn kick(vel: f64) -> Vec<f64> {
    synth::kick(1.0, 0.32, 150.0, 46.0).iter().map(|v| v * vel).collect()
}

impl BalkanFireClassic {
    /// The bed.
    pub fn new() -> BalkanFireClassic {
        BalkanFireClassic
    }

    /// `bed(mix, b, intensity, light)`: one bar of the drum-less bed: the oom-pah bass and accordion, the bellows
    /// pad (from bar 3), the cimbalom shimmer once the fire builds.
    fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, light: bool, lift: f64) -> Played {
        let put = |mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, g: f64, pan: f64| put(mix, bus, x, t, g * lift, pan);
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        let t0 = b as f64 * bar;
        let ch = chord(ctx, b)?;
        let n = ctx.harmony.chords.len();
        let nxt = chord(ctx, (b + 1) % n)?;
        let (r, f5) = bass(&ctx.harmony, ch).map_err(bad)?;
        // Oom: the bass on 1 and 3, a chromatic pickup into the next bar later in the fight.
        put(mix, "bass", upright(r, 0.42, 1.0), t0, 0.62, 0.0);
        put(mix, "bass", upright(f5, 0.36, 0.9), t0 + 2.0 * beat, 0.52, 0.0);
        if intensity > 1.25 && !light {
            let target = bass(&ctx.harmony, nxt).map_err(bad)?.0;
            put(mix, "bass", upright(target - 1, 0.18, 0.7), t0 + 3.5 * beat, 0.40, 0.0);
        }
        // Pah: accordion stabs on 2 and 4 (8th-note "pah-pah" when the fire builds).
        let vo = voicing(&ctx.harmony, ch).map_err(bad)?;
        for bt in [1.0, 3.0] {
            put(
                mix,
                "accordion",
                stab(&vo, 0.20),
                t0 + bt * beat,
                if light { 0.20 } else { 0.24 },
                0.0,
            );
            if intensity > 1.35 && !light {
                put(mix, "accordion", stab(&vo, 0.12), t0 + (bt + 0.5) * beat, 0.13, 0.0);
            }
        }
        // The bellows pad under it (from bar 3).
        if b >= 3 || light {
            let ms = [vo[0] - 12, vo[0], vo[2 % vo.len()]];
            put(mix, "pad", reed(&ms, bar, 0.25, 0.3, 1400.0, MUSETTE), t0, 0.05, 0.0);
        }
        // The cimbalom shimmer: 16th arpeggios, building through the back half.
        if intensity > 1.2 && !light {
            let k = ((intensity - 1.2) / 0.3).min(1.0);
            let tones = cimb(&ctx.harmony, ch).map_err(bad)?;
            for st in 0..16usize {
                if st % 2 == 1 && k < 0.6 {
                    continue;
                }
                let m = tones[(st * 3 + b) % tones.len()];
                let vel = 0.7 + if st % 4 == 0 { 0.3 } else { 0.0 };
                let pan = 0.3 * dmath::sin(st as f64);
                put(mix, "cimbalom", cimbalom(m, vel), t0 + st as f64 * s16, 0.05 * k, pan);
            }
        }
        Ok(())
    }
}

impl Bed for BalkanFireClassic {
    /// A slam lands on the low bass and the thud (DOUBLE KICK's stomp in #28).
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom]
    }

    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &BUSES
    }

    /// `A.reverb(send) * 0.20` (the default room).
    fn reverb(&self) -> (f64, f64, f64) {
        (synth::REVERB_DUR, synth::REVERB_DECAY, 0.20)
    }

    fn comp(&self) -> (f64, f64) {
        (0.22, 0.35)
    }

    fn roar_gain(&self) -> f64 {
        0.16
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

    /// A big drum hit: the bass's lowest tonic, a kick and the thud.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        put(mix, "bass", upright(tonic(ctx, 24), 0.8, 1.0), t, g * 0.9, 0.0);
        put(mix, "perc", kick(1.0), t, g * 0.6, 0.0);
        put(mix, "fx", synth::thud(0.6, 90.0, 32.0, 7), t, g, 0.0);
        Ok(())
    }

    /// `A.chime(91, 0.25)` at 0.05.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", synth::chime(91.0, 0.25), t, 0.05, 0.0);
        Ok(())
    }

    /// The band on every word: the bass and an accordion stab on the tonic, a cimbalom flourish, a thud.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = chord(ctx, 0)?;
        let (r, _) = bass(&ctx.harmony, ch).map_err(bad)?;
        let vo = voicing(&ctx.harmony, ch).map_err(bad)?;
        let tones = cimb(&ctx.harmony, ch).map_err(bad)?;
        let s32 = ctx.show.grid.s16 / 2.0;
        for tw in times {
            put(mix, "bass", upright(r, 0.5, 1.0), tw, 0.62, 0.0);
            put(mix, "accordion", stab(&vo, 0.25), tw, 0.26, 0.0);
            for (j, &m) in tones.iter().enumerate() {
                put(mix, "cimbalom", cimbalom(m, 0.9), tw + j as f64 * s32, 0.07, 0.0);
            }
            let seed = noise_seed(H, tw * 50.0)? + 3;
            put(mix, "fx", synth::thud(0.8, 120.0, 34.0, seed), tw, 0.40, 0.0);
        }
        Ok(())
    }

    /// VS: a thunder crack where it splits the divider, and a crash.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", crack(), t, 0.36, 0.0);
        put(mix, "perc", synth::crash(1.6), t, 0.10, 0.0);
        Ok(())
    }

    /// The bellows hold the tonic under the card (never quiet), a cimbalom run up on beat 4 as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let ch = chord(ctx, 0)?;
        let vo = voicing(&ctx.harmony, ch).map_err(bad)?;
        let ms = [vo[0] - 12, vo[0], vo[2 % vo.len()]];
        put(
            mix,
            "pad",
            reed(&ms, 3.0 * grid.beat, 0.05, 0.3, 1400.0, MUSETTE),
            0.0,
            0.07,
            0.0,
        );
        let tones = cimb(&ctx.harmony, ch).map_err(bad)?;
        for st in 12..16usize {
            let m = tones[(st - 12) % tones.len()];
            put(mix, "cimbalom", cimbalom(m, 0.8), st as f64 * grid.s16, 0.06, 0.0);
        }
        Ok(())
    }

    /// Bar 1, on the dominant: the sticks, the bass and an accordion stab on each count, a snare pickup into
    /// FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (bar, s16) = (grid.bar, grid.s16);
        let ch = chord(ctx, 1)?;
        let (r, _) = bass(&ctx.harmony, ch).map_err(bad)?;
        let vo = voicing(&ctx.harmony, ch).map_err(bad)?;
        for (tc, _) in &grid.count[..3] {
            put(mix, "perc", sticks(1.0), *tc, 0.30, 0.0);
            put(mix, "bass", upright(r, 0.4, 1.0), *tc, 0.62, 0.0);
            put(mix, "accordion", stab(&vo, 0.16), *tc, 0.20, 0.0);
        }
        for (st, v) in [(12.0, 0.45), (13.0, 0.6), (14.0, 0.75), (15.0, 0.9)] {
            let x = synth::snare(0.1).iter().map(|s| s * v).collect();
            put(mix, "perc", x, bar + st * s16, 0.30, 0.0);
        }
        Ok(())
    }

    /// FIGHT!: a crash and the band's low tonic.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.grid.t_fight;
        put(mix, "perc", synth::crash(2.0), t, 0.16, 0.0);
        put(mix, "fx", synth::thud(0.7, 110.0, 34.0, 7), t, 0.30, 0.0);
        Ok(())
    }

    /// The bed from FIGHT! to the K.O., the fire building (`intensity = 1 + 0.6 * b / bar_ko`), a whoosh into the
    /// back half, a tape stop at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let grid = &ctx.show.grid;
        for b in grid.fight_bar..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.6 * (b as f64 / bar_ko.max(1) as f64), false, 1.0)?;
        }
        // #28's whoosh 3.6 s before 44 s (bar 26 of 36).
        let back = (grid.n_bars as f64 * 26.0 / 36.0).round() * grid.bar;
        if back < ctx.show.t_ko {
            let whoosh = synth::whoosh(3.6, 300.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
            put(mix, "fx", whoosh, py_max(0.0, back - 3.6), 0.10, 0.0);
        }
        for bus in ["bass", "accordion", "pad", "cimbalom", "perc"] {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    /// The K.O.: a crash, the bass's lowest tonic, the thud.
    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.t_ko;
        put(mix, "perc", synth::crash(2.4), t, 0.24, 0.0);
        put(mix, "bass", upright(tonic(ctx, 24), 1.4, 1.0), t, 0.62, 0.0);
        put(mix, "fx", synth::thud(0.9, 120.0, 30.0, 7), t, 0.7, 0.0);
        Ok(())
    }

    /// The band's tonic chord and the bass under the winner's stinger.
    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        let ch = chord(ctx, 0)?;
        let ms = spread(ctx, ch, 48, 5)?;
        put(
            mix,
            "accordion",
            reed(&ms, 1.2, 0.01, 0.5, 2600.0, MUSETTE),
            tw,
            0.20,
            0.0,
        );
        let (r, _) = bass(&ctx.harmony, ch).map_err(bad)?;
        put(mix, "bass", upright(r, 0.9, 1.0), tw, 0.6, 0.0);
        Ok(())
    }

    /// A whoosh from just after the stinger to the solo.
    fn after_ko(&self, _: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let dur = py_max(0.3, t_solo - tw - 0.1);
        put(
            mix,
            "fx",
            synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED),
            tw + 0.1,
            0.10,
            0.0,
        );
        Ok(())
    }

    /// The oom-pah under the solo, up to the build: the fire still going (#28 played a lighter one and lifted it 3 dB
    /// in its master; the kit's master has no lift, so the bed plays [`VICTORY_LIFT`] louder).
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        let n = ctx.show.grid.n_bars;
        for b in b_solo..n.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.4, false, VICTORY_LIFT)?;
        }
        Ok(())
    }

    /// The last bar builds into the title slam on sample 0: a snare roll, a riser, the bass climbing the chord, the
    /// accordion's off-beats, a pad swell and kicks on 1, 3, 4 and the last 8th.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, dur, s16) = (grid.beat, grid.bar, grid.dur, grid.s16);
        let t_last = (grid.n_bars as f64 - 1.0) * bar;
        let rest = dur - t_last;
        let ch = chord(ctx, grid.n_bars.saturating_sub(1))?;
        put(mix, "perc", roll(rest + 0.03, 0.15, s16 / 2.0), t_last, 0.30, 0.0);
        put(mix, "fx", riser(rest, RISER_OVER, RISER_SEED)?, t_last, 0.22, 0.0);
        let (r, _) = bass(&ctx.harmony, ch).map_err(bad)?;
        let tones = ctx.harmony.chord_tones(r, r + 11, ch).map_err(bad)?;
        let climb = [
            r,
            tones[1 % tones.len()],
            tones[2 % tones.len()],
            tones[3 % tones.len()].max(r + 10),
        ];
        let vo = voicing(&ctx.harmony, ch).map_err(bad)?;
        for (bt, m) in climb.into_iter().enumerate() {
            let tb = t_last + bt as f64 * beat;
            put(mix, "bass", upright(m, 0.36, 1.0), tb, 0.55, 0.0);
            put(mix, "accordion", stab(&vo, 0.18), tb + beat / 2.0, 0.18, 0.0);
        }
        let ms = spread(ctx, ch, 45, 5)?;
        put(
            mix,
            "pad",
            reed(&ms, rest + 0.02, 0.6, 0.001, 1800.0, MUSETTE),
            t_last,
            0.07,
            0.0,
        );
        for st in [0.0, 8.0, 12.0, 14.0] {
            put(mix, "perc", kick(0.9), t_last + st * s16, 0.42, 0.0);
        }
        Ok(())
    }
}
