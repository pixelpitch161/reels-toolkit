//! The gothic horror's classic arrangement: the bed of #35's `audio.py` (`bed()` and the bed's lines of `main()`)
//! on the kit's beat sheet (the VS card on bar 0, the count-in on bar 1, FIGHT! on bar 2), section by section as
//! `kit.music.score()` calls them. Quick-ported by ear: the same instruments, patterns and levels, not the same
//! samples.
//!
//! **The sections.** #35's 32 bars are the burn (to bar 13), the climb (bars 14 to 21) and the pound (22 on); a fight
//! of another length keeps those shares of its bars ([`Section::at`]).

use bf_core::dmath;
use bf_core::harmony::HarmonyError;
use bf_core::py::py_max;
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, noise_seed, riser};
use bf_sound::synth::{self, FILTER_O, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{AAH, HAH, bell, cello, choir, drone, heart, pedal, roll, tick, timp, voicing};

/// How much louder the bars under the solo play: the episode's master lifted its victory 3 dB (`GAIN_PTS`), the
/// kit's has no such lift, so the bed does it.
const VICTORY_LIFT: f64 = 1.5;

/// The name the bed's errors carry.
const H: &str = "gothic_horror";

/// The bed's own buses: `BUS_GAIN` and the reverb sends of #35's `master()`.
const BUSES: [(&str, (f64, f64)); 6] = [
    ("choir", (1.6, 0.6)),
    ("drone", (1.0, 0.0)),
    ("bell", (1.0, 0.7)),
    ("perc", (1.0, 0.0)),
    ("strings", (1.2, 0.3)),
    ("drums", (1.6, 0.12)),
];

/// `CELLO_PAT`: the climb's ostinato in 8ths, from the bar's root (a minor-sixth sting in it).
const CELLO_PAT: [i32; 8] = [0, 0, 7, 0, 8, 0, 7, 12];

/// Where a bar sits in the slow burn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Burn,
    Climb,
    Pound,
}

impl Section {
    /// The section of bar `b` of an episode of `n_bars` bars (#35: the climb from bar 14 of 32, the pound from 22).
    fn at(b: usize, n_bars: usize) -> Section {
        let climb = (n_bars as f64 * 14.0 / 32.0).round() as usize;
        let pound = (n_bars as f64 * 22.0 / 32.0).round() as usize;
        if b >= pound {
            Section::Pound
        } else if b >= climb {
            Section::Climb
        } else {
            Section::Burn
        }
    }

    /// The first bar of the pound.
    fn pound_from(n_bars: usize) -> usize {
        (n_bars as f64 * 22.0 / 32.0).round() as usize
    }
}

/// The gothic horror's bed.
#[derive(Clone, Debug, Default)]
pub struct GothicHorrorClassic;

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

/// The key's tonic at or above `lo` (D3 = 50 for `lo` 48 in D).
fn tonic(ctx: &MusicCtx, lo: i32) -> i32 {
    lo + (ctx.harmony.key_pc - lo).rem_euclid(12)
}

/// The bell's note on the chord: the tonic (D3), or the dominant's root a fourth below it on the dominant (A2).
fn bell_note(ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
    let dominant = (ctx.harmony.key_pc + 7).rem_euclid(12);
    let root = ctx.harmony.root_above(ch, 0).map_err(bad)?.rem_euclid(12);
    Ok(if root == dominant {
        45 + (dominant - 45).rem_euclid(12)
    } else {
        tonic(ctx, 48)
    })
}

/// The chord's tones from `lo` up, the root doubled an octave down: the title's choir "hah" (D3 A3 D4 F4 in D).
fn choir_chord(ctx: &MusicCtx, ch: &str, lo: i32) -> Result<Vec<i32>, MusicError> {
    let root = ctx.harmony.root_above(ch, lo).map_err(bad)?;
    let mut ms = vec![root];
    ms.extend(ctx.harmony.chord_tones(root + 1, root + 15, ch).map_err(bad)?);
    ms.truncate(4);
    Ok(ms)
}

/// The VS crack: band-passed noise with a low rumble under it (#35's, seeds 91 and 92).
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

impl GothicHorrorClassic {
    /// The bed.
    pub fn new() -> GothicHorrorClassic {
        GothicHorrorClassic
    }

    /// `bed(mix, b, sec, light)`: one bar: the choir pad, the drone and the bell; unless light, the clock's 8ths,
    /// the low strings and the section's drums.
    fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, sec: Section, light: bool, lift: f64) -> Played {
        let put = |mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, g: f64, pan: f64| put(mix, bus, x, t, g * lift, pan);
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        let t0 = b as f64 * bar;
        let ch = chord(ctx, b)?;
        let vo = voicing(&ctx.harmony, ch).map_err(bad)?;
        let root = pedal(&ctx.harmony, ch).map_err(bad)?;
        let burn = sec == Section::Burn;
        let ms = if burn {
            vec![vo[0] - 12, vo[0], vo[2]]
        } else {
            vec![vo[0] - 12, vo[0], vo[1], vo[2]]
        };
        let g = match sec {
            Section::Burn => 0.10,
            Section::Climb => 0.13,
            Section::Pound => 0.15,
        } * if light { 0.8 } else { 1.0 };
        let att = if burn { 0.5 } else { 0.3 };
        put(mix, "choir", choir(&ms, bar, att, 0.5, AAH, 1.0, b as u64), t0, g, 0.0);
        let low = if root > 30 { root - 12 } else { root };
        put(mix, "drone", drone(low, bar + 0.3, 1.0), t0, 0.10, 0.0);
        // The bell: every bar, and its octave on 3 once the burn is over.
        let bell_m = bell_note(ctx, ch)?;
        put(mix, "bell", bell(bell_m, 3.5, 1.0), t0, 0.16, 0.0);
        if !burn && !light {
            put(mix, "bell", bell(bell_m + 12, 2.0, 1.0), t0 + 2.0 * beat, 0.07, 0.0);
        }
        if light {
            return Ok(());
        }
        // The clock ticks the 8ths (tick-tock), quieter once the drums pound.
        let tick_g = match sec {
            Section::Burn => 0.05,
            Section::Climb => 0.045,
            Section::Pound => 0.035,
        };
        for e8 in 0..8usize {
            let on = e8 % 2 == 0;
            let x = tick(if on { 1.0 } else { 0.7 }, on);
            put(
                mix,
                "perc",
                x,
                t0 + e8 as f64 * beat / 2.0,
                tick_g,
                if on { 0.35 } else { -0.35 },
            );
        }
        // The low strings: the root pulsing in the burn, the ostinato from the climb.
        for (e8, step) in CELLO_PAT.into_iter().enumerate() {
            let (m, v) = if burn {
                (root, if e8 % 2 == 0 { 0.9 } else { 0.6 })
            } else {
                (root + step, if [0, 3, 6].contains(&e8) { 1.0 } else { 0.7 })
            };
            let g = if burn { 0.045 } else { 0.07 };
            put(
                mix,
                "strings",
                cello(m, 0.15, 1.0),
                t0 + e8 as f64 * beat / 2.0,
                g * v,
                0.0,
            );
        }
        // The drums.
        let tim = if root < 40 { root + 12 } else { root };
        match sec {
            Section::Burn => {
                // A heartbeat: lub-dub on 1, again on 3.
                for bt in [0.0, 2.0] {
                    put(mix, "drums", heart(1.0), t0 + bt * beat, 0.55, 0.0);
                    put(mix, "drums", heart(0.7), t0 + bt * beat + s16 * 1.5, 0.40, 0.0);
                }
            }
            Section::Climb => {
                put(mix, "drums", synth::kick(0.9, 0.3, 150.0, 46.0), t0, 0.50, 0.0);
                put(
                    mix,
                    "drums",
                    synth::kick(0.8, 0.3, 150.0, 46.0),
                    t0 + 2.5 * beat,
                    0.40,
                    0.0,
                );
                for bt in [1usize, 3] {
                    let x = synth::thud(0.35, 150.0, 70.0, (b * 4 + bt) as u64);
                    put(mix, "drums", x, t0 + bt as f64 * beat, 0.30, 0.0);
                }
                put(mix, "drums", timp(tim, 0.8, 1.4), t0, 0.22, 0.0);
            }
            Section::Pound => {
                // Four on the floor, timpani, staccato choir hits on the off-beats.
                for bt in 0..4usize {
                    put(
                        mix,
                        "drums",
                        synth::kick(1.0, 0.32, 150.0, 46.0),
                        t0 + bt as f64 * beat,
                        0.55,
                        0.0,
                    );
                    let hit = choir(
                        &[vo[1] + 12, vo[2] + 12],
                        0.09,
                        0.01,
                        0.08,
                        HAH,
                        1.0,
                        (b * 4 + bt) as u64,
                    );
                    put(mix, "choir", hit, t0 + (bt as f64 + 0.5) * beat, 0.07, 0.0);
                }
                put(mix, "drums", timp(tim, 1.0, 1.4), t0, 0.30, 0.0);
                put(mix, "drums", timp(tim + 7, 0.7, 1.4), t0 + 2.0 * beat, 0.20, 0.0);
                for st in [14usize, 15] {
                    let x = synth::thud(0.25, 180.0, 90.0, (b * 9 + st) as u64);
                    put(mix, "drums", x, t0 + st as f64 * s16, 0.22, 0.0);
                }
                for bt in [1.0, 3.0] {
                    put(mix, "drums", synth::snare(0.12), t0 + bt * beat, 0.10, 0.0);
                }
            }
        }
        Ok(())
    }
}

impl Bed for GothicHorrorClassic {
    /// A slam lands on the timpani and the low thud (TOCCATA's stomp in #35).
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom]
    }

    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &BUSES
    }

    /// A cathedral: `A.reverb(send, dur=3.2, decay=0.7) * 0.24`.
    fn reverb(&self) -> (f64, f64, f64) {
        (3.2, 0.7, 0.24)
    }

    fn comp(&self) -> (f64, f64) {
        (0.22, 0.35)
    }

    fn roar_gain(&self) -> f64 {
        0.16
    }

    fn doors_gain(&self) -> f64 {
        0.55
    }

    fn landing(&self) -> (f64, f64) {
        (0.08, 0.06)
    }

    fn pop(&self) -> (f64, f64, f64) {
        (95.0, 102.0, 0.12)
    }

    /// A big drum hit: the low timpani on the tonic, the thud and a low bell.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        let low = tonic(ctx, 36);
        put(mix, "drums", timp(low, 1.0, 1.4), t, g * 0.7, 0.0);
        put(mix, "fx", synth::thud(0.7, 90.0, 30.0, 7), t, g, 0.0);
        put(mix, "bell", bell(low, 3.0, 1.0), t, g * 0.35, 0.0);
        Ok(())
    }

    /// `A.chime(91, 0.25)` at 0.05.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", synth::chime(91.0, 0.25), t, 0.05, 0.0);
        Ok(())
    }

    /// The band on every word: a timpani, a boom, the bell and a choir "hah" on the tonic.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = chord(ctx, 0)?;
        let ms = choir_chord(ctx, ch, 48)?;
        let low = tonic(ctx, 36);
        for tw in times {
            put(mix, "drums", timp(low, 1.0, 1.2), tw, 0.42, 0.0);
            let seed = noise_seed(H, tw * 50.0)? + 3;
            put(mix, "fx", synth::thud(0.9, 120.0, 32.0, seed), tw, 0.50, 0.0);
            put(mix, "bell", bell(tonic(ctx, 48), 2.2, 1.0), tw, 0.16, 0.0);
            let seed = noise_seed(H, tw * 9.0)?;
            put(
                mix,
                "choir",
                choir(&ms, 0.30, 0.008, 0.25, HAH, 1.0, seed),
                tw,
                0.16,
                0.0,
            );
        }
        Ok(())
    }

    /// VS: a thunder crack where it splits the divider, and a crash.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", crack(), t, 0.40, 0.0);
        put(mix, "drums", synth::crash(1.6), t, 0.10, 0.0);
        Ok(())
    }

    /// A choir pad and the drone under the whole card (never quiet), a timpani pickup on beat 4 as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let ms = choir_chord(ctx, chord(ctx, 0)?, 48)?;
        put(
            mix,
            "choir",
            choir(&ms, 3.0 * grid.beat, 0.05, 0.3, AAH, 1.0, 0),
            0.0,
            0.10,
            0.0,
        );
        put(mix, "drone", drone(tonic(ctx, 24), grid.bar + 0.2, 1.0), 0.0, 0.10, 0.0);
        let dom = tonic(ctx, 36) + 7;
        for (st, v) in [(12.0, 0.5), (13.0, 0.62), (14.0, 0.78), (15.0, 0.95)] {
            put(mix, "drums", timp(dom, v, 0.5), st * grid.s16, 0.30, 0.0);
        }
        Ok(())
    }

    /// Bar 1, on the dominant: the bell and a timpani on each count, the choir on the chord, a roll into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        let ch = chord(ctx, 1)?;
        let root = ctx.harmony.root_above(ch, 45).map_err(bad)?;
        for (j, (tc, _)) in grid.count[..3].iter().enumerate() {
            let up = if j == 2 { 12 } else { 0 };
            put(mix, "bell", bell(root + up, 2.0, 1.0), *tc, 0.20, 0.0);
            put(mix, "drums", timp(root, 0.9, 1.4), *tc, 0.30, 0.0);
        }
        let mut ms = vec![root];
        ms.extend(voicing(&ctx.harmony, ch).map_err(bad)?);
        put(mix, "choir", choir(&ms, bar, 0.2, 0.05, AAH, 1.0, 1), bar, 0.09, 0.0);
        put(
            mix,
            "drums",
            roll(beat, root, 0.3, grid.s16 / 2.0),
            bar + 3.0 * beat,
            0.30,
            0.0,
        );
        Ok(())
    }

    /// FIGHT!: a crash, a kick and the low bell.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.grid.t_fight;
        put(mix, "drums", synth::crash(2.0), t, 0.16, 0.0);
        put(mix, "drums", synth::kick(1.0, 0.4, 150.0, 46.0), t, 0.6, 0.0);
        put(mix, "bell", bell(tonic(ctx, 36), 5.0, 1.0), t, 0.24, 0.0);
        Ok(())
    }

    /// The bed from FIGHT! to the K.O. (the burn, the climb, the pound), a crash and a whoosh where the pound
    /// arrives, a tape stop at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let grid = &ctx.show.grid;
        let n = grid.n_bars;
        for b in grid.fight_bar..=bar_ko {
            self.bar(ctx, mix, b, Section::at(b, n), false, 1.0)?;
        }
        let pound = Section::pound_from(n);
        if pound <= bar_ko {
            let t = pound as f64 * grid.bar;
            put(mix, "drums", synth::crash(2.5), t, 0.18, 0.0);
            let whoosh = synth::whoosh(3.6, 300.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
            put(mix, "fx", whoosh, py_max(0.0, t - 3.6), 0.10, 0.0);
        }
        for bus in ["choir", "drone", "bell", "perc", "strings", "drums"] {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    /// The K.O.: a crash, the low timpani, the thud.
    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.t_ko;
        put(mix, "drums", synth::crash(2.5), t, 0.26, 0.0);
        put(mix, "drums", timp(tonic(ctx, 36), 1.0, 2.0), t, 0.5, 0.0);
        put(mix, "fx", synth::thud(0.9, 120.0, 30.0, 7), t, 0.7, 0.0);
        Ok(())
    }

    /// The bell tolls under the winner's stinger.
    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        put(mix, "bell", bell(tonic(ctx, 48), 4.0, 1.0), tw, 0.22, 0.0);
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

    /// The bed under the solo: the climb (#35 played a lighter one and lifted it 3 dB in its master; the kit's
    /// master has no lift, so the bed plays the full climb, [`VICTORY_LIFT`] louder).
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        let n = ctx.show.grid.n_bars;
        for b in b_solo..n.saturating_sub(1) {
            self.bar(ctx, mix, b, Section::Climb, false, VICTORY_LIFT)?;
        }
        Ok(())
    }

    /// The last bar builds into the title slam on sample 0: a timpani roll, a riser, a choir swell, the bell, the
    /// cellos climbing the chord and a kick on every beat.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, dur) = (grid.beat, grid.bar, grid.dur);
        let t_last = (grid.n_bars as f64 - 1.0) * bar;
        let ch = chord(ctx, grid.n_bars.saturating_sub(1))?;
        let root = ctx.harmony.root_above(ch, 45).map_err(bad)?;
        let rest = dur - t_last;
        put(
            mix,
            "drums",
            roll(rest + 0.03, root, 0.15, grid.s16 / 2.0),
            t_last,
            0.32,
            0.0,
        );
        put(mix, "fx", riser(rest, RISER_OVER, RISER_SEED)?, t_last, 0.20, 0.0);
        let mut ms = vec![root];
        ms.extend(ctx.harmony.chord_tones(root + 12, root + 24, ch).map_err(bad)?);
        put(
            mix,
            "choir",
            choir(&ms, rest + 0.02, 1.2, 0.001, AAH, 1.0, 2),
            t_last,
            0.14,
            0.0,
        );
        put(mix, "bell", bell(root, 2.0, 1.0), t_last, 0.2, 0.0);
        // The cellos climb the chord: root, third, fifth, octave (A1 C#2 E2 A2 on #35's dominant).
        let low = root - 12;
        let tones = ctx.harmony.chord_tones(low, low + 11, ch).map_err(bad)?;
        let climb = [low, tones[1 % tones.len()], tones[2 % tones.len()], low + 12];
        for (bt, m) in climb.into_iter().enumerate() {
            let tb = t_last + bt as f64 * beat;
            put(mix, "strings", cello(m, 0.2, 1.0), tb, 0.08, 0.0);
            put(mix, "drums", synth::kick(0.9, 0.3, 150.0, 46.0), tb, 0.45, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sections_keep_the_episodes_shares() {
        assert_eq!(Section::at(13, 32), Section::Burn);
        assert_eq!(Section::at(14, 32), Section::Climb);
        assert_eq!(Section::at(22, 32), Section::Pound);
        assert_eq!(Section::pound_from(32), 22);
    }
}
