//! The unit tests of the music helpers and of the context.

use std::fmt;
use std::sync::Arc;

use bf_core::dmath::{self, PI};
use bf_core::event::{Event, Value};
use bf_core::recording::Array;
use bf_core::rng::Generator;

use super::testkit::{TestBed, TestVoice, context, episode};
use super::*;
use crate::SRF;
use crate::dsp;
use crate::mix::{Mix, Sig};
use crate::synth::noise;
use crate::synth::{self, FILTER_O};
use crate::voice::{PpNoise, Voice};

/// A float as Python's `float.hex()` prints it.
fn hex(text: &str) -> f64 {
    bf_testdata::parse_hex_f64(text)
}

/// The message of an error.
fn refused<T: fmt::Debug>(r: Result<T, MusicError>) -> String {
    r.expect_err("it was not refused").to_string()
}

#[test]
fn hashes_are_pythons_on_odd_arguments() {
    // Printed by the kit on the reference PC: h01(-1), h01(1e30), h01(-2.5, 7), h01(0.0005), h01(0.0015),
    // h01(-1e30, 2). A negative int wraps as Python's `&` wraps it; a float past 2**64 keeps its low bits;
    // 0.5 and 1.5 thousandths round to the even one (0 and 2).
    assert_eq!(h01(&[-1.0]), hex("0x1.6ca1cc5aa2763p-1"));
    assert_eq!(h01(&[1e30]), hex("0x1.ecfe83b5e5a48p-4"));
    assert_eq!(h01(&[-2.5, 7.0]), hex("0x1.b2ed4e3c999d8p-1"));
    assert_eq!(h01(&[0.0005]), h01(&[0.0]));
    assert_eq!(h01(&[0.0005]), hex("0x1.b9e279aa86e58p-2"));
    assert_eq!(h01(&[0.0015]), hex("0x1.7f908c2017f83p-1"));
    assert_eq!(h01(&[-1e30, 2.0]), hex("0x1.b6035718dd38ap-2"));
    // h01_37(-7, 3), h01_37(0.0005), h01_37(1e30), h01_37(3, -1.5).
    assert_eq!(h01_37(&[-7.0, 3.0]), hex("0x1.62bc478bc8489p-1"));
    assert_eq!(h01_37(&[0.0005]), hex("0x1.918a9f3158192p-1"));
    assert_eq!(h01_37(&[1e30]), hex("0x1.1c5b67e767eacp-5"));
    assert_eq!(h01_37(&[3.0, -1.5]), hex("0x1.e66a76ae8c971p-1"));
    // No argument: the seed constant itself.
    assert_eq!(
        h01(&[]),
        (0x9E37_79B9_7F4A_7C15u64 >> 11) as f64 / 9_007_199_254_740_992.0
    );
    // A float beyond 2**117 has no low bits left: it hashes as 0 does.
    assert_eq!(h01(&[1e40]), h01(&[0.0]));
    assert_eq!(h01(&[-1e300]), h01(&[0.0]));
}

#[test]
#[should_panic(expected = "cannot convert float NaN to integer")]
fn a_hash_argument_must_be_a_number() {
    h01(&[1.0, f64::NAN]);
}

#[test]
#[should_panic(expected = "cannot convert float inf to integer")]
fn a_hash_argument_must_be_finite() {
    h01_37(&[f64::INFINITY]);
}

#[test]
fn thinning_follows_the_hp() {
    // At full HP nothing is dropped; at none, some notes are, never two in a row.
    let mut th = Thinner::new(5.0);
    assert!((0..400).all(|s| th.keep(f64::from(s), 1.0)));
    let mut th = Thinner::new(5.0);
    let kept: Vec<bool> = (0..400).map(|s| th.keep(f64::from(s), 0.0)).collect();
    let dropped = kept.iter().filter(|k| !**k).count();
    assert!((60..=140).contains(&dropped), "{dropped} of 400 dropped at 0 HP");
    assert!(!kept.windows(2).any(|w| w == [false, false]));
    // Printed by the kit: Thinner(5).keep(3, 0.5) is False; with a NaN HP the chance is the full 0.30
    // (Python's min keeps its first argument), slots 0..12.
    assert!(!Thinner::new(5.0).keep(3.0, 0.5));
    let mut th = Thinner::new(5.0);
    let nan: Vec<bool> = (0..12).map(|s| th.keep(f64::from(s), f64::NAN)).collect();
    assert_eq!(
        nan,
        [
            true, true, false, true, false, true, false, true, true, false, true, false
        ]
    );
    // thin_37: nothing at full HP, nothing right after a drop, nothing for a NaN HP.
    assert!((0..400).all(|s| !thin_37(f64::from(s), 3.0, 1.0, false)));
    assert!((0..400).all(|s| !thin_37(f64::from(s), 3.0, 0.0, true)));
    assert!((0..400).any(|s| thin_37(f64::from(s), 3.0, 0.0, false)));
    assert!(!thin_37(0.0, 3.0, f64::NAN, false));
    // Above full HP Python raises only if it gets as far as the comparison.
    assert!(!thin_37(0.0, 3.0, 1.5, true));
}

#[test]
#[should_panic(expected = "Thinner.keep: hp01 is 1.5, above 1")]
fn a_thinner_refuses_more_than_full_hp() {
    Thinner::new(5.0).keep(0.0, 1.5);
}

#[test]
#[should_panic(expected = "thin_37: hp is 1.5, above 1")]
fn thin_37_refuses_more_than_full_hp() {
    thin_37(0.0, 3.0, 1.5, false);
}

#[test]
fn q16_snaps_to_the_even_sixteenth() {
    let s16 = hex("0x1.cb4b4b4b4b4b4p-4");
    // Printed by the kit: q16(-0.03), q16(0.5 * S16), q16(1.5 * S16), q16(2.5 * S16).
    assert_eq!(q16(-0.03, s16).to_bits(), 0.0f64.to_bits());
    assert_eq!(q16(0.5 * s16, s16).to_bits(), 0.0f64.to_bits());
    assert_eq!(q16(1.5 * s16, s16), 2.0 * s16);
    assert_eq!(q16(2.5 * s16, s16), 2.0 * s16);
    assert_eq!(q16(1.0, 0.25), 1.0);
    assert!(q16(f64::NAN, s16).is_nan());
}

#[test]
fn fit_cuts_pads_and_fades() {
    let ones = vec![1.0; 5000];
    // Cut to 2,400 samples; the last 800 (a third) fade to nothing.
    let y = fit(&ones, 0.05, FIT_FADE).unwrap();
    assert_eq!(y.len(), 2400);
    assert_eq!((y[1599], y[1600], y[2399]), (1.0, 1.0, 0.0));
    assert_eq!(y[1601], dsp::linspace(1.0, 0.0, 800)[1]);
    // Padded with silence: a short note keeps its samples up to the fade.
    let y = fit(&[1.0; 10], 0.05, FIT_FADE).unwrap();
    assert_eq!(y.len(), 2400);
    assert_eq!((y[9], y[10], y[2399]), (1.0, 0.0, 0.0));
    // A short fade: 240 samples.
    let y = fit(&ones, 0.05, 0.005).unwrap();
    assert_eq!((y[2159], y[2160], y[2399]), (1.0, 1.0, 0.0));
    assert!(y[2161] < 1.0);
    // What Python gives at the edges (run on the reference PC): no samples at all for a cap of 0; three
    // samples untouched (a fade of one sample is a factor of 1); a negative cap cuts the end off.
    assert!(fit(&ones, 0.0, FIT_FADE).unwrap().is_empty());
    assert_eq!(fit(&ones, 3.0 / 48000.0, FIT_FADE).unwrap(), [1.0, 1.0, 1.0]);
    let y = fit(&ones, -0.01, FIT_FADE).unwrap();
    assert_eq!(y.len(), 4520);
    assert!((y[4518] - 0.00066445).abs() < 1e-8 && y[4519] == 0.0);
    assert!(fit(&ones, -0.2, FIT_FADE).unwrap().is_empty());
    assert_eq!(fit(&ones, -4997.0 / 48000.0, FIT_FADE).unwrap(), [1.0, 1.0, 1.0]);
}

#[test]
fn fit_refuses_what_python_refuses() {
    let ones = vec![1.0; 5000];
    let no_room = "leave no room for a fade";
    assert!(refused(fit(&ones, 1.0 / 48000.0, FIT_FADE)).contains(no_room));
    assert!(refused(fit(&ones, 2.5 / 48000.0, FIT_FADE)).contains(no_room));
    assert!(refused(fit(&ones, -4999.0 / 48000.0, FIT_FADE)).contains(no_room));
    assert!(refused(fit(&ones, 0.05, 0.0)).contains(no_room));
    assert!(refused(fit(&ones, 0.05, 1e-6)).contains(no_room));
    assert_eq!(refused(fit(&ones, 0.05, -0.01)), "fit: a fade of -0.01 s is below zero");
    assert_eq!(refused(fit(&ones, f64::NAN, FIT_FADE)), "fit: cap is NaN, not a number");
    assert_eq!(
        refused(fit(&ones, 0.05, f64::INFINITY)),
        "fit: fade is inf, not a number"
    );
    assert_eq!(
        refused(fit(&ones, 1e9, FIT_FADE)),
        "fit: cap of 1000000000 s is longer than 600 s"
    );
}

#[test]
fn a_warp_of_nothing_is_the_note() {
    let x: Vec<f64> = (0..2000).map(|i| dmath::sin(f64::from(i) * 0.05)).collect();
    // No slide, no bend, no vibrato: the read position is every sample in turn.
    assert_eq!(gliss(&x, 0.0, 0.05).unwrap(), x);
    assert_eq!(bend(&x, 0.0, 0.0, 0.2).unwrap(), x);
    assert_eq!(vibrato(&x, 0.0, VIBRATO_RATE, VIBRATO_DELAY).unwrap(), x);
    // Sliding up into the note reads slower at first: the end of the note is not reached, and nothing past
    // it is read. Falling away (a negative slide) reads faster: the note runs out and silence follows.
    let up = gliss(&x, 12.0, 0.02).unwrap();
    assert_eq!((up.len(), up[0]), (2000, x[0]));
    assert!(up[1999] != 0.0 && up[1999] != x[1999]);
    let down = gliss(&x, -12.0, 0.02).unwrap();
    assert_eq!(down[0], x[0]);
    assert!(down[1900..].iter().all(|v| *v == 0.0) && down[1000] != 0.0);
    // A bend starts at t0: before it the note is untouched.
    let bent = bend(&x, 2.0, 0.01, 0.03).unwrap();
    assert_eq!(bent[..480], x[..480]);
    assert!(bent[481..].iter().zip(&x[481..]).any(|(a, b)| a != b));
    // The vibrato comes in after its delay.
    let wobbly = vibrato(&x, 0.5, 5.0, 0.01).unwrap();
    assert_eq!(wobbly[..481], x[..481]);
    assert!(wobbly[481..] != x[481..]);
    for r in [
        gliss(&[], 2.0, 0.035),
        bend(&[], 2.0, 0.0, 0.2),
        vibrato(&[], 0.35, 5.5, 0.15),
    ] {
        assert!(refused(r).ends_with(": the note has no samples"));
    }
}

#[test]
fn wah_needs_a_sample_per_band() {
    let x: Vec<f64> = (0..4800).map(|i| dmath::sin(f64::from(i) * 0.1)).collect();
    let y = wah(&x, &[(0.0, 0.05)]).unwrap();
    assert_eq!(y.len(), 4800);
    assert!(y.iter().all(|v| v.is_finite()) && y.iter().any(|v| v.abs() > 0.1));
    // No span at all: the band stays at 450 Hz.
    assert_eq!(wah(&x, &[]).unwrap().len(), 4800);
    assert_eq!(wah(&x[..48], &[]).unwrap().len(), 48);
    assert_eq!(
        refused(wah(&x[..47], &[])),
        "wah: 47 sample(s) are too few for 48 bands"
    );
    assert_eq!(refused(wah(&[], &[])), "wah: 0 sample(s) are too few for 48 bands");
    // A span from a time that is not a number never opens (no comparison with it holds), as in Python; one
    // from minus infinity makes the centre NaN, which no filter takes.
    assert_eq!(wah(&x, &[(f64::NAN, 0.1)]).unwrap(), wah(&x, &[]).unwrap());
    assert!(refused(wah(&x, &[(f64::NEG_INFINITY, 0.1)])).starts_with("wah: a band from NaN to NaN Hz"));
}

#[test]
fn colour_thins_and_drives_with_the_hp() {
    let x: Vec<f64> = (0..4800).map(|i| 0.5 * dmath::sin(f64::from(i) * 0.05)).collect();
    let args = ColourArgs::default();
    // At full HP against full HP: a 90 Hz high-pass, no drive (tanh(1.2 y) / tanh(1.2)), no gain.
    let (y, g, lead) = colour_hp(&x, 100.0, 100.0, COLOUR_DRIVE_K, &args).unwrap();
    assert_eq!((g, lead), (1.0, 0.0));
    let plain = synth::hp(&x, 90.0, FILTER_O);
    let want: Vec<f64> = plain
        .iter()
        .map(|&v| dmath::np_tanh(1.0 * v * 1.2) / dmath::np_tanh(1.2) * 1.0)
        .collect();
    assert_eq!(y, want);
    // The gain follows the lead: 6 dB for a full bar, kept between -4 and +3.5 dB.
    let gain = |own: f64, other: f64| colour_hp(&x, own, other, 1.0, &args).unwrap().1;
    assert_eq!(gain(100.0, 0.0), dmath::pow(10.0, 3.5 / 20.0));
    assert_eq!(gain(0.0, 100.0), dmath::pow(10.0, -4.0 / 20.0));
    assert_eq!(gain(60.0, 50.0), dmath::pow(10.0, 6.0 * ((60.0 - 50.0) / 100.0) / 20.0));
    // A bass line keeps its fundamental: its cutoff starts at 30 Hz, not 90.
    let low = ColourArgs { low: true, ..args };
    assert_eq!(
        colour_hp(&x, 100.0, 100.0, 1.0, &low).unwrap().0,
        synth::hp(&x, 30.0, FILTER_O)
            .iter()
            .map(|&v| dmath::np_tanh(v * 1.2) / dmath::np_tanh(1.2))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        refused(colour_hp(&[], 50.0, 50.0, 1.0, &args)),
        "colour: the note has no samples"
    );
    assert_eq!(
        refused(colour_hp(&x, 100.5, 50.0, 1.0, &args)),
        "colour: an HP of 100.5 is above 100"
    );
    let wild = ColourArgs { cut0: 30000.0, ..args };
    assert_eq!(
        refused(colour_hp(&x, 100.0, 50.0, 1.0, &wild)),
        "colour: a cutoff of 30000 Hz is not between 0 and 24000 Hz"
    );
    let none = ColourArgs { cut0: 0.0, ..args };
    assert!(refused(colour_hp(&x, 100.0, 50.0, 1.0, &none)).contains("a cutoff of 0 Hz"));
}

#[test]
fn effects_have_pythons_lengths() {
    assert_eq!(clank(84.0, CLANK_SEED, CLANK_DUR, &CLANK_DECAYS).unwrap().len(), 19200);
    assert_eq!(
        hiss(HISS_DUR, HISS_SEED, HISS_LO, HISS_HI, HISS_DEC).unwrap().len(),
        4320
    );
    assert_eq!(roar(0.5, ROAR_SEED).unwrap().len(), 24000);
    assert_eq!(thunder(0.25, THUNDER_SEED, THUNDER_CRACK).unwrap().len(), 12000);
    assert_eq!(riser(0.5, RISER_OVER, RISER_SEED).unwrap().len(), 24576);
    // No partials: Python's sum is the int 0, and the click alone is left.
    let click = clank(84.0, 3, 0.1, &[]).unwrap();
    let noise = synth::bp(&noise(3, 4800), 2000.0, 8000.0, FILTER_O);
    assert_eq!(click[100], 0.3 * noise[100] * dmath::exp(-(100.0 / 48000.0) / 0.005));
    // The crack is the generator's second draw: without it the rumble is the same.
    let with = thunder(0.25, 4, true).unwrap();
    let without = thunder(0.25, 4, false).unwrap();
    assert!(with != without);
    let mut r = Generator::new(4);
    let mut first = vec![0.0; 12000];
    r.fill_normal(0.0, 1.0, &mut first);
    let low = synth::lp(&first, 140.0, FILTER_O);
    let t = 6000.0 / 48000.0;
    let rumble = low[6000] * 3.0 * 1.0 * dmath::exp(-t / (0.25 * 0.35));
    assert_eq!(
        without[6000],
        rumble * (0.7 + 0.3 * dmath::sin(2.0 * PI * 3.1 * t + 4.0))
    );
}

#[test]
fn effects_refuse_what_python_refuses() {
    let nothing = "is under one sample: there is nothing to filter";
    assert!(refused(clank(84.0, 0, 0.0, &CLANK_DECAYS)).contains(nothing));
    assert!(refused(hiss(1e-6, 0, HISS_LO, HISS_HI, HISS_DEC)).contains(nothing));
    assert!(refused(roar(0.0, ROAR_SEED)).contains(nothing));
    assert!(refused(thunder(0.0, 0, true)).contains(nothing));
    assert_eq!(
        refused(riser(19.5 / 48000.0, 0.0, RISER_SEED)),
        "riser: 19 sample(s) are too few for 20 bands"
    );
    assert_eq!(riser(20.5 / 48000.0, 0.0, RISER_SEED).unwrap().len(), 20);
    assert_eq!(refused(roar(-1.0, ROAR_SEED)), "roar: a length of -1 s is below zero");
    assert_eq!(refused(roar(f64::NAN, 3)), "roar: a length is NaN, not a number");
    assert_eq!(
        refused(thunder(601.0, 0, true)),
        "thunder: a length of 601 s is longer than 600 s"
    );
    assert_eq!(
        refused(hiss(0.1, 0, 9000.0, 3000.0, 0.03)),
        "hiss: a band from 9000 to 3000 Hz does not rise between 0 and 24000 Hz"
    );
    assert!(refused(hiss(0.1, 0, 0.0, 3000.0, 0.03)).starts_with("hiss: a band from 0 to 3000 Hz"));
    assert!(refused(hiss(0.1, 0, 3000.0, 24000.0, 0.03)).starts_with("hiss: a band from 3000 to 24000 Hz"));
    assert!(refused(hiss(0.1, 0, f64::NAN, 3000.0, 0.03)).starts_with("hiss: a band from NaN"));
    // A riser that rises over no time divides 0 by 0 at its first sample. With under 40 samples that sample
    // is the first band's centre, and no filter takes it; with more, every sample comes out NaN (Python's
    // too: the saw's phase is a running sum from that first NaN).
    assert!(refused(riser(0.0, 25.5 / 48000.0, RISER_SEED)).starts_with("riser: a band from NaN"));
    assert!(riser(0.0, 0.01, RISER_SEED).unwrap().iter().all(|v| v.is_nan()));
}

#[test]
fn hp_follows_the_fight() {
    let ep = episode();
    let (ctx, _) = context(&ep);
    let grid = &ep.show.grid;
    assert_eq!((ep.show.win, ctx.nsim()), (1, 1394));
    // Printed by the kit on the reference PC (`M.hp_at(k, t)` as float.hex(), k = 0 and 1).
    let full = "0x1.9000000000000p+6";
    let python: [(f64, [&str; 2]); 11] = [
        (0.0, [full, full]),
        (grid.t_fight, [full, full]),
        (3.6, [full, full]),
        (12.0, ["0x1.6e66666666664p+6", "0x1.53851eb851ebap+6"]),
        (ep.show.t_ko - 0.001, ["0x0.0p+0", "0x1.051eb851eb8c8p+2"]),
        (ep.show.t_ko, ["0x0.0p+0", "0x1.051eb851eb8c8p+2"]),
        (ep.show.t_ko + 0.5, ["0x0.0p+0", "0x1.051eb851eb8c8p+2"]),
        (grid.t_glide + 0.3, ["0x1.cf0b8f96f633fp+3", "0x1.1f5b40df3c895p+4"]),
        (grid.t_glide + 0.5, ["0x1.6c56f471782b8p+5", "0x1.7e1d5703929a8p+5"]),
        (grid.t_rev, [full, full]),
        (60.9, [full, full]),
    ];
    for (t, want) in python {
        for (k, want) in want.iter().enumerate() {
            assert_eq!(ctx.hp_at(k, t).to_bits(), hex(want).to_bits(), "hp_at({k}, {t})");
        }
    }
    // sim_i: frame 0 up to FIGHT!, the last frame from the K.O. on.
    assert_eq!((ctx.sim_i(0.0), ctx.sim_i(grid.t_fight), ctx.sim_i(12.0)), (0, 0, 252));
    assert_eq!((ctx.sim_i(ep.show.t_ko), ctx.sim_i(1e9)), (1393, 1393));
    assert_eq!((ctx.sim_i(f64::NAN), ctx.sim_i(-1e9)), (0, 0));
    // A time that is not a number is past the K.O. for Python's comparisons: the HP it ended on.
    assert_eq!(ctx.hp_at(0, f64::NAN), 0.0);
    assert_eq!(ctx.q16(1.0), q16(1.0, grid.s16));
}

#[test]
#[should_panic(expected = "hp_at: fighter 2")]
fn hp_is_of_one_of_the_two() {
    let ep = episode();
    context(&ep).0.hp_at(2, 12.0);
}

#[test]
fn the_context_reads_the_fight_file() {
    let ep = episode();
    let (ctx, _) = context(&ep);
    // The harmony the kit builds for the episode: E harmonic minor, ['Em', 'B7'] + LOOP * 4.
    assert_eq!((ctx.harmony.key_pc, ctx.harmony.loop_start), (4, 2));
    assert_eq!(ctx.harmony.chords.len(), 34);
    assert_eq!(ctx.harmony.chords[..4], ["Em", "B7", "Em", "Am"]);
    assert_eq!(ctx.harmony.loop_, ep.ff.music.chord_loop);
    assert!(format!("{ctx:?}").contains("FLUTE"));
    // A scale the kit does not know, and a recording without HP.
    let mut ff = ep.ff.clone();
    ff.music.scale = "wholetone".into();
    let voices = || -> [Arc<dyn Voice>; 2] { [TestVoice::new("FLUTE", 0), TestVoice::new("TROMBONE", 0)] };
    let cfg = [&ep.cfg[0], &ep.cfg[1]];
    let e = refused(MusicCtx::new(&ep.show, &ep.rec, &ff, cfg, voices(), Vec::new()));
    assert!(e.starts_with("music: ") && e.contains("wholetone"), "{e}");
    let mut rec = ep.rec.clone();
    rec.arrays.remove("HP");
    let no_hp = "music: the recording has no HP of two fighters at every frame";
    assert_eq!(
        refused(MusicCtx::new(&ep.show, &rec, &ep.ff, cfg, voices(), Vec::new())),
        no_hp
    );
    // An HP of another shape, of no frames, or shorter than it says.
    let mut with = |shape: Vec<usize>, len: usize| {
        let data = vec![50.0; len];
        rec.arrays.insert("HP".into(), Array::F64 { shape, data });
        refused(MusicCtx::new(&ep.show, &rec, &ep.ff, cfg, voices(), Vec::new()))
    };
    assert_eq!(with(vec![4, 3], 12), no_hp);
    assert_eq!(with(vec![8], 8), no_hp);
    assert_eq!(with(vec![0, 2], 0), no_hp);
    assert_eq!(
        with(vec![5, 2], 7),
        "music: the recording's HP does not have the length it states"
    );
}

#[test]
fn a_note_is_made_once_and_its_gain_after_it() {
    let ep = episode();
    let (ctx, voices) = context(&ep);
    let start = ctx.noise().unwrap();
    let x = ctx.note("FLUTE", 76.9, 0.2, 0.3, 0.8).unwrap();
    // int(midi), then the level-matching note: the middle of the register, velocity 0.8, 0.2 s.
    assert_eq!(voices[0].calls(), [(76.0, 0.8, 0.2), (66.0, 0.8, 0.2)]);
    assert!(voices[1].calls().is_empty());
    assert_eq!(x.len(), 14400);
    // fit(voice, cap) * gain, with the noise the note drew FIRST and the gain note's after it.
    let mut noise = start.clone();
    let own = voices[0].note(&mut noise, 76.0, 0.8, 0.2);
    let level = voices[0].note(&mut noise, 66.0, 0.8, 0.2);
    let squares: Vec<f64> = synth::hp(&level[..14400], 120.0, FILTER_O)
        .iter()
        .map(|v| v * v)
        .collect();
    let gain = 0.16 / (dsp::mean(&squares).sqrt() + 1e-6);
    assert_eq!(ctx.inst_gain("FLUTE").unwrap(), gain);
    let want: Vec<f64> = fit(&own, 0.3, FIT_FADE).unwrap().iter().map(|v| v * gain).collect();
    assert_eq!(x.to_vec(), want);
    assert_eq!(ctx.noise().unwrap().0.clone().random(), noise.0.random());

    // The same key again: the same samples, no voice call, no noise.
    let calls = voices[0].calls().len();
    let before = ctx.noise().unwrap().0.random();
    for (midi, dur, vel) in [(76.9, 0.2, 0.8), (76.0, 0.2004, 0.8), (76.2, 0.1996, 0.804)] {
        let again = ctx.note("FLUTE", midi, dur, 0.3, vel).unwrap();
        assert!(Arc::ptr_eq(&again, &x), "midi {midi}, dur {dur}, vel {vel}");
    }
    assert_eq!(voices[0].calls().len(), calls);
    assert_eq!(ctx.noise().unwrap().0.random(), before);
    // Another key (the cap, the rounded dur, the rounded vel, the note): another note, with the arguments
    // as they were passed, and no second gain note.
    ctx.note("FLUTE", 76.0, 0.2, 0.31, 0.8).unwrap();
    ctx.note("FLUTE", 76.0, 0.2006, 0.3, 0.8).unwrap();
    ctx.note("FLUTE", 76.0, 0.2, 0.3, 0.806).unwrap();
    ctx.note("FLUTE", 77.0, 0.2, 0.3, 0.8).unwrap();
    assert_eq!(
        voices[0].calls()[calls..],
        [
            (76.0, 0.8, 0.2),
            (76.0, 0.8, 0.2006),
            (76.0, 0.806, 0.2),
            (77.0, 0.8, 0.2)
        ]
    );
    // The other voice has its own gain, measured on its own first note.
    ctx.note("TROMBONE", 52.0, 0.2, 0.3, 0.8).unwrap();
    assert_eq!(voices[1].calls(), [(52.0, 0.8, 0.2), (66.0, 0.8, 0.2)]);
    let log = ctx.first_notes().unwrap();
    assert_eq!(log.len(), 6);
    assert_eq!(
        log[0],
        NoteCall {
            name: "FLUTE".into(),
            midi: 76,
            dur: 0.2,
            cap: 0.3,
            vel: 0.8,
            own: None
        }
    );
    assert_eq!(
        (log[2].dur, log[3].vel, log[5].name.as_str()),
        (0.2006, 0.806, "TROMBONE")
    );

    // A new build: nothing cached, the stream where it is put.
    ctx.restart(start.clone()).unwrap();
    assert!(ctx.first_notes().unwrap().is_empty());
    let again = ctx.note("FLUTE", 76.9, 0.2, 0.3, 0.8).unwrap();
    assert!(!Arc::ptr_eq(&again, &x));
    assert_eq!(again, x);
    // Measuring the gain before the first note puts other noise under the note.
    ctx.restart(start).unwrap();
    ctx.inst_gain("FLUTE").unwrap();
    assert!(ctx.note("FLUTE", 76.9, 0.2, 0.3, 0.8).unwrap() != x);
}

#[test]
fn a_parts_own_note_helper_has_its_fade_cache_and_gain() {
    // An older part's own `_fit` (Sound.note_helper, #31's SYNTH: a 0.03 s fade): its own cache and its own gain
    // table, the voice called first and its gain measured after it, logged with its fade.
    let ep = episode();
    let (ctx, voices) = context(&ep);
    let start = ctx.noise().unwrap();
    let own = OwnNote { side: 0, fade: 0.03 };
    let x = ctx.own_note(own, "FLUTE", 76.9, 0.2, 0.3, 0.8).unwrap();
    assert_eq!(voices[0].calls(), [(76.0, 0.8, 0.2), (66.0, 0.8, 0.2)]);
    let mut noise = start.clone();
    let raw = voices[0].note(&mut noise, 76.0, 0.8, 0.2);
    let level = voices[0].note(&mut noise, 66.0, 0.8, 0.2);
    let squares: Vec<f64> = synth::hp(&level[..14400], 120.0, FILTER_O)
        .iter()
        .map(|v| v * v)
        .collect();
    let gain = py_min(4.0, 0.16 / (dsp::mean(&squares).sqrt() + 1e-6));
    let want: Vec<f64> = fit(&raw, 0.3, 0.03).unwrap().iter().map(|v| v * gain).collect();
    assert_eq!(x.to_vec(), want);
    assert_ne!(fit(&raw, 0.3, 0.03).unwrap(), fit(&raw, 0.3, FIT_FADE).unwrap());
    // its own cache: the same key again is the same note, no voice call
    let again = ctx.own_note(own, "FLUTE", 76.0, 0.2, 0.3, 0.8).unwrap();
    assert!(Arc::ptr_eq(&again, &x));
    assert_eq!(voices[0].calls().len(), 4); // (2 of the build's, 2 of this test's own above)
    // the kit's note of the same key is another note, with its own gain measured again (two gain tables)
    ctx.note("FLUTE", 76.9, 0.2, 0.3, 0.8).unwrap();
    assert_eq!(voices[0].calls().len(), 6);
    // the other side's helper has its own cache and gain too
    ctx.own_note(OwnNote { side: 1, fade: 0.03 }, "FLUTE", 76.0, 0.2, 0.3, 0.8)
        .unwrap();
    assert_eq!(voices[0].calls().len(), 8);
    let log = ctx.first_notes().unwrap();
    assert_eq!(
        log.iter().map(|c| c.own).collect::<Vec<_>>(),
        [Some(own), None, Some(OwnNote { side: 1, fade: 0.03 })]
    );
    let bad = OwnNote {
        side: 0,
        fade: f64::NAN,
    };
    assert!(ctx.own_note(bad, "FLUTE", 70.0, 0.2, 0.3, 0.8).is_err());
}

#[test]
fn a_note_refuses_what_it_cannot_play() {
    let ep = episode();
    let (ctx, voices) = context(&ep);
    let start = ctx.noise().unwrap().0.random();
    let note = |midi: f64, dur: f64, cap: f64, vel: f64| refused(ctx.note("FLUTE", midi, dur, cap, vel));
    assert_eq!(note(f64::NAN, 0.2, 0.3, 0.8), "note: midi is NaN, not a number");
    assert_eq!(note(76.0, f64::INFINITY, 0.3, 0.8), "note: dur is inf, not a number");
    assert_eq!(note(76.0, 0.2, f64::NAN, 0.8), "note: cap is NaN, not a number");
    assert_eq!(note(76.0, 0.2, 0.3, f64::NAN), "note: vel is NaN, not a number");
    assert_eq!(
        note(76.0, 700.0, 0.3, 0.8),
        "note: a held length (dur) of 700 s is longer than 600 s"
    );
    assert_eq!(
        note(76.0, 0.2, 1.0 / 48000.0, 0.8),
        "note: 1 sample(s) with a fade of 0.04 s leave no room for a fade (3 samples and a fade of one sample \
         are the least)"
    );
    assert_eq!(
        note(76.0, 0.2, 1e9, 0.8),
        "note: cap of 1000000000 s is longer than 600 s"
    );
    assert_eq!(
        refused(ctx.note("TUBA", 40.0, 0.2, 0.3, 0.8)),
        "note: \"TUBA\" is not a voice of this fight (the fighters play \"FLUTE\" and \"TROMBONE\", the theme none)"
    );
    assert!(refused(ctx.inst_gain("TUBA")).starts_with("inst_gain: \"TUBA\" is not a voice of this fight"));
    // Nothing was synthesised, nothing drawn, nothing logged.
    assert!(voices[0].calls().is_empty() && ctx.first_notes().unwrap().is_empty());
    assert_eq!(ctx.noise().unwrap().0.random(), start);
    // A note of no held length and a note of no length are notes (Python plays both).
    assert_eq!(ctx.note("FLUTE", 76.0, 0.0, 0.3, 0.8).unwrap().len(), 14400);
    assert!(ctx.note("FLUTE", 76.0, 0.2, 0.0, 0.8).unwrap().is_empty());
    // A pitch the voice cannot make: the sine of an infinite frequency is not a number.
    let e = note(1e6, 0.2, 0.3, 0.8);
    assert!(
        e.starts_with("note: FLUTE gives samples that are not numbers for MIDI note 1000000"),
        "{e}"
    );
}

/// A voice that plays nothing at all.
struct Mute;

impl Voice for Mute {
    fn note(&self, _: &mut PpNoise, _: f64, _: f64, _: f64) -> Vec<f64> {
        Vec::new()
    }

    fn register(&self) -> (i32, i32) {
        (-3, 0)
    }

    fn name(&self) -> &str {
        "FLUTE"
    }
}

#[test]
fn a_voice_without_sound_has_no_gain() {
    let ep = episode();
    let ctx = MusicCtx::new(
        &ep.show,
        &ep.rec,
        &ep.ff,
        [&ep.cfg[0], &ep.cfg[1]],
        [Arc::new(Mute), TestVoice::new("TROMBONE", 0)],
        Vec::new(),
    )
    .unwrap();
    // (lo + hi) // 2 floors: (-3 + 0) // 2 is -2.
    assert_eq!(
        refused(ctx.inst_gain("FLUTE")),
        "inst_gain: FLUTE gives no sound for MIDI note -2, the middle of its register"
    );
    assert!(refused(ctx.note("FLUTE", 76.0, 0.2, 0.3, 0.8)).starts_with("note: FLUTE gives no sound"));
}

#[test]
fn line_overrides_are_kept_by_fighter() {
    let ep = episode();
    let (ctx, _) = context(&ep);
    assert!(ctx.line_overrides("TROMBONE").unwrap().is_empty());
    ctx.line_override(1, 10.0, 12.0, 10.0, &[(0.0, Some(52), 1.0), (1.0, None, 1.0)])
        .unwrap();
    ctx.line_override(1, 20.0, 22.0, 20.5, &[]).unwrap();
    let got = ctx.line_overrides("TROMBONE").unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(
        got[0],
        LineOverride {
            t0: 10.0,
            t1: 12.0,
            tz: 10.0,
            notes: vec![(0.0, Some(52), 1.0), (1.0, None, 1.0)]
        }
    );
    assert_eq!((got[1].t0, got[1].tz), (20.0, 20.5));
    assert!(ctx.line_overrides("FLUTE").unwrap().is_empty());
    // They belong to the fight, not to one pass over its notes.
    ctx.restart(PpNoise::start()).unwrap();
    assert_eq!(ctx.line_overrides("TROMBONE").unwrap().len(), 2);
}

/// One event of a recording.
fn event(t: f64, kind: &str, fields: Vec<Value>) -> Event {
    Event {
        t,
        kind: kind.into(),
        fields,
    }
}

#[test]
fn shared_events_place_the_rings_sounds() {
    let mut ep = episode();
    let t_ko = ep.show.t_ko;
    let (f, i) = (Value::Float, Value::Int);
    ep.rec.events = vec![
        // [t, 'rope', who, ., ., e[5], e[6], e[7]]: e[5] pans the thud, e[7] is how hard the ball hit.
        event(5.0, "rope", vec![i(1), i(1), i(-1), f(-0.7), f(-2.4), f(2.6)]),
        event(6.0, "hit", vec![i(1), i(0), f(1.4)]),
        event(7.0, "clash", vec![i(-1), f(-0.6), f(-0.7)]),
        event(8.01, "block", vec![i(0)]),
        event(9.0, "parry", vec![i(1)]),
        // Far to the right and hard: the pan stops at 0.6, the level at 1.5 times; a whole number counts.
        event(10.0, "rope", vec![i(0), i(0), i(1), f(5.0), f(0.0), i(8)]),
        // Up to 10 ms past the K.O. an event still sounds; the first one later ends the list, whatever
        // follows it (the kit's loop breaks there).
        event(t_ko + 0.005, "parry", vec![]),
        event(t_ko + 0.02, "rope", vec![]),
        event(11.0, "parry", vec![]),
    ];
    let (ctx, _) = context(&ep);
    let bed = TestBed::new((0.05, 0.02, 0.2));
    let mut mix = Mix::new(52.0);
    shared_events(&ctx, &bed, &mut mix).unwrap();
    assert_eq!(mix.bus_names(), ["fx", "clash"]);
    assert_eq!(*bed.clashes.lock().unwrap(), [7.0]);

    // The same placements, written out as the kit writes them.
    let mut want = Mix::new(52.0);
    let ring = |m: f64, seed: u64| Sig::Mono(clank(m, seed, CLANK_DUR, &CLANK_DECAYS).unwrap());
    let thud = |seed: u64| Sig::Mono(synth::thud(0.22, 95.0, 55.0, seed));
    want.place("fx", &thud(50), 5.0, 0.05 * (2.6 / 2.6), -0.7 / 3.5);
    want.place("fx", &ring(84.0, 40), ctx.q16(8.01), 0.06, 0.0);
    want.place("fx", &ring(88.0, 81), 9.0, 0.04, 0.0);
    want.place("fx", &thud(100), 10.0, 0.05 * 1.5, 0.6);
    want.place(
        "fx",
        &ring(88.0, ((t_ko + 0.005) * 9.0) as u64),
        t_ko + 0.005,
        0.04,
        0.0,
    );
    assert!(mix.bus("fx") == want.bus("fx"));
    // The block rings on the 16th, not where it happened.
    assert!(ctx.q16(8.01) != 8.01);
    let at = (ctx.q16(8.01) * SRF).round() as usize;
    let fx = mix.bus("fx").unwrap();
    assert!(fx[at] != [0.0, 0.0] && fx[at - 1] == [0.0, 0.0]);
}

#[test]
fn shared_events_refuse_an_event_they_cannot_read() {
    let mut ep = episode();
    let (f, i) = (Value::Float, Value::Int);
    let bed = TestBed::new((0.05, 0.02, 0.2));
    let mut run = |events: Vec<Event>| -> Result<(), MusicError> {
        ep.rec.events = events;
        let (ctx, _) = context(&ep);
        shared_events(&ctx, &bed, &mut Mix::new(1.0))
    };
    // A rope bounce without its speed, and one whose pan is a word.
    assert_eq!(
        refused(run(vec![event(5.0, "rope", vec![i(1), i(1), i(-1), f(-0.7), f(-2.4)])])),
        "shared_events: the rope event at 5 s has no number as element 7 (it has None)"
    );
    let e = refused(run(vec![event(
        5.0,
        "rope",
        vec![i(1), i(1), i(-1), Value::Str("left".into()), f(-2.4), f(2.6)],
    )]));
    assert!(
        e.contains("has no number as element 5 (it has Some(Str(\"left\")))"),
        "{e}"
    );
    // A time before the start is no noise seed (numpy refuses a seed below zero).
    assert_eq!(
        refused(run(vec![event(-2.0, "parry", vec![])])),
        "shared_events: -18 is not a noise seed"
    );
    // What the bed refuses comes back as it is.
    assert_eq!(
        refused(run(vec![event(-1.0, "clash", vec![])])),
        "clash: a clash at -1 s"
    );
    // Nothing to read, nothing wrong: other kinds are not this function's.
    assert_eq!(run(vec![event(5.0, "hit", vec![]), event(6.0, "ko", vec![])]), Ok(()));
    assert_eq!(run(Vec::new()), Ok(()));
}

#[test]
fn a_note_below_zero_follows_python() {
    let ep = episode();
    let (ctx, voices) = context(&ep);
    // A held length a little below zero is a note: what the voice rings on past it (the test voice 0.1 s,
    // so 0.05 s are left here), padded to the cap.
    let x = ctx.note("FLUTE", 76.0, -0.05, 0.3, 0.8).unwrap();
    assert_eq!(x.len(), 14400);
    assert_eq!(voices[0].calls()[0], (76.0, 0.8, -0.05));
    assert!(x[2399] != 0.0 && x[2400..].iter().all(|v| *v == 0.0));
    // A held length the voice cannot hold is refused before anything is drawn (Python raises there).
    let before = ctx.noise().unwrap().0.random();
    assert_eq!(
        refused(ctx.note("FLUTE", 77.0, -0.2, 0.3, 0.8)),
        "note: FLUTE has no note for a held length (dur) of -0.2 s: it would end before it starts"
    );
    assert_eq!(ctx.noise().unwrap().0.random(), before);
    assert_eq!(voices[0].calls().len(), 2);

    // A cap below zero cuts that much off the end of what the voice gave (Python's slice `x[:n]`), then
    // fades what is left: here 14,400 samples less 4,800.
    let whole = ctx.note("TROMBONE", 52.0, 0.2, 0.3, 0.8).unwrap();
    let cut = ctx.note("TROMBONE", 52.0, 0.2, -0.1, 0.8).unwrap();
    assert_eq!((whole.len(), cut.len()), (14400, 9600));
    assert_eq!(cut[..7680], whole[..7680]);
    assert!(cut[7681] != whole[7681] && cut[9599] == 0.0);
    // More than there is: an empty note.
    assert!(ctx.note("TROMBONE", 52.0, 0.2, -1.0, 0.8).unwrap().is_empty());
    // 1 or 2 samples left have no room for the fade: found once the voice has given its note.
    let calls = voices[1].calls().len();
    let e = refused(ctx.note("TROMBONE", 53.0, 0.2, -14398.5 / 48000.0, 0.8));
    assert_eq!(
        e,
        "note: 2 sample(s) with a fade of 0.04 s leave no room for a fade (3 samples and a fade of one sample \
         are the least)"
    );
    assert_eq!(voices[1].calls().len(), calls + 1);
}

#[test]
fn the_voices_must_be_the_fighters() {
    let ep = episode();
    let cfg = [&ep.cfg[0], &ep.cfg[1]];
    let swapped: [Arc<dyn Voice>; 2] = [TestVoice::new("TROMBONE", 0), TestVoice::new("FLUTE", 0)];
    assert_eq!(
        refused(MusicCtx::new(&ep.show, &ep.rec, &ep.ff, cfg, swapped, Vec::new())),
        "music: voice 0 is \"TROMBONE\", but FLUTE plays \"FLUTE\""
    );
    let other: [Arc<dyn Voice>; 2] = [TestVoice::new("FLUTE", 0), TestVoice::new("TUBA", 0)];
    assert_eq!(
        refused(MusicCtx::new(&ep.show, &ep.rec, &ep.ff, cfg, other, Vec::new())),
        "music: voice 1 is \"TUBA\", but TROMBONE plays \"TROMBONE\""
    );
}

/// A voice that breaks on MIDI note 13, after it has drawn its noise.
struct Shaky;

impl Voice for Shaky {
    fn note(&self, noise: &mut PpNoise, midi: f64, _: f64, _: f64) -> Vec<f64> {
        let x = noise.nz(10);
        assert!(midi != 13.0, "the voice broke");
        x
    }

    fn register(&self) -> (i32, i32) {
        (60, 72)
    }

    fn name(&self) -> &str {
        "FLUTE"
    }
}

#[test]
fn a_voice_that_panicked_ends_the_build() {
    let ep = episode();
    let ctx = MusicCtx::new(
        &ep.show,
        &ep.rec,
        &ep.ff,
        [&ep.cfg[0], &ep.cfg[1]],
        [Arc::new(Shaky), TestVoice::new("TROMBONE", 0)],
        Vec::new(),
    )
    .unwrap();
    assert!(ctx.note("FLUTE", 76.0, 0.2, 0.3, 0.8).is_ok());
    // Formatting a context does not wait for the lock: it works while a note is being made.
    {
        let _making_a_note = ctx.state.lock().unwrap();
        assert!(format!("{ctx:?}").contains("[\"FLUTE\", \"TROMBONE\"]"));
    }
    let broke = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ctx.note("FLUTE", 13.0, 0.2, 0.3, 0.8)));
    assert!(broke.is_err());
    // The voice had drawn noise before it broke, so every later note would be silently another note: nothing
    // of this build is handed out any more, not even a note that was cached.
    let dead = "music: a voice panicked while a note was being made: the noise stream and the notes of this \
                build cannot be trusted any more";
    assert_eq!(refused(ctx.note("FLUTE", 76.0, 0.2, 0.3, 0.8)), dead);
    assert_eq!(refused(ctx.note("TROMBONE", 52.0, 0.2, 0.3, 0.8)), dead);
    assert_eq!(refused(ctx.inst_gain("FLUTE")), dead);
    assert_eq!(refused(ctx.noise()), dead);
    assert_eq!(refused(ctx.first_notes()), dead);
    assert_eq!(refused(ctx.restart(PpNoise::start())), dead);
    assert_eq!(refused(ctx.line_override(0, 1.0, 2.0, 1.0, &[])), dead);
    assert_eq!(refused(ctx.line_overrides("FLUTE")), dead);
    assert!(format!("{ctx:?}").contains("FLUTE"));
}

#[test]
fn a_line_override_is_for_one_of_the_two() {
    let ep = episode();
    let (ctx, _) = context(&ep);
    assert_eq!(
        refused(ctx.line_override(2, 1.0, 2.0, 1.0, &[])),
        "line_override: fighter 2 (there are two, 0 and 1)"
    );
    assert!(ctx.line_overrides("FLUTE").unwrap().is_empty());
}

/// The theme's own voices (`hot_jazz` plays the TUBA's): `note` finds a fighter's voice first, then the theme's,
/// and every voice shares the build's one noise stream, note cache, gains and log, as the kit's `note` does (one
/// `_VC` and one `_GAIN` keyed by the voice's name, one `drums60_lowpoly._rng`).
#[test]
fn a_themes_voices_share_the_build() {
    let ep = episode();
    let theme_ctx = |tuba: &Arc<TestVoice>, own_trombone: &Arc<TestVoice>, flute: &Arc<TestVoice>| {
        let theme: Vec<Arc<dyn Voice>> = vec![tuba.clone(), own_trombone.clone()];
        MusicCtx::new(
            &ep.show,
            &ep.rec,
            &ep.ff,
            [&ep.cfg[0], &ep.cfg[1]],
            [flute.clone(), TestVoice::new("TROMBONE", 0)],
            theme,
        )
        .unwrap()
    };
    let (tuba, own, flute) = (
        TestVoice::new("TUBA", 30),
        TestVoice::new("TROMBONE", 9),
        TestVoice::new("FLUTE", 100),
    );
    let ctx = theme_ctx(&tuba, &own, &flute);
    let f = ctx.note("FLUTE", 76.0, 0.2, 0.3, 0.8).unwrap();
    let t = ctx.note("TUBA", 40.0, 0.2, 0.3, 0.8).unwrap();
    // the theme's voice: its note, then its gain's middle note (after the note's own voice call, as the kit's
    // `fit(B.voice(...)) * inst_gain(name)` evaluates)
    assert_eq!(tuba.calls(), [(40.0, 0.8, 0.2), (66.0, 0.8, 0.2)]);
    // the same key again: the cached samples, no call, nothing drawn
    let noise = ctx.noise().unwrap().0.random();
    assert!(Arc::ptr_eq(&t, &ctx.note("TUBA", 40.0, 0.2004, 0.3, 0.8).unwrap()));
    assert_eq!(tuba.calls().len(), 2);
    assert_eq!(ctx.noise().unwrap().0.random(), noise);
    // a name both a fighter and the theme have is the fighter's voice (the left corner's first)
    ctx.note("TROMBONE", 40.0, 0.2, 0.3, 0.8).unwrap();
    assert!(own.calls().is_empty());
    // one log of first-time notes, in the order they were made
    let names: Vec<String> = ctx.first_notes().unwrap().into_iter().map(|n| n.name).collect();
    assert_eq!(names, ["FLUTE", "TUBA", "TROMBONE"]);
    // one noise stream: the TUBA's note drew after the FLUTE's, so asking for it first gives other samples
    let (tuba2, own2, flute2) = (
        TestVoice::new("TUBA", 30),
        TestVoice::new("TROMBONE", 9),
        TestVoice::new("FLUTE", 100),
    );
    let other = theme_ctx(&tuba2, &own2, &flute2);
    let t2 = other.note("TUBA", 40.0, 0.2, 0.3, 0.8).unwrap();
    assert!(t2[..] != t[..], "the theme's voice draws from the shared stream");
    let f2 = other.note("FLUTE", 76.0, 0.2, 0.3, 0.8).unwrap();
    assert!(f2[..] != f[..]);
    // a voice nobody plays is refused in words, nothing drawn
    let noise = ctx.noise().unwrap().0.random();
    assert_eq!(
        refused(ctx.note("PIANO", 60.0, 0.2, 0.3, 0.8)),
        "note: \"PIANO\" is not a voice of this fight (the fighters play \"FLUTE\" and \"TROMBONE\", the theme others)"
    );
    assert_eq!(ctx.noise().unwrap().0.random(), noise);
    // a theme voice listed twice is refused
    let twice: Vec<Arc<dyn Voice>> = vec![TestVoice::new("TUBA", 0), TestVoice::new("TUBA", 0)];
    let e = MusicCtx::new(
        &ep.show,
        &ep.rec,
        &ep.ff,
        [&ep.cfg[0], &ep.cfg[1]],
        [TestVoice::new("FLUTE", 0), TestVoice::new("TROMBONE", 0)],
        twice,
    );
    assert_eq!(refused(e), "music: the theme lists one of its voices twice");
}

/// A voice whose every note is one draw of the alto generator, held for 0.5 s.
struct AltoDraw;

impl Voice for AltoDraw {
    fn note(&self, noise: &mut PpNoise, _: f64, _: f64, _: f64) -> Vec<f64> {
        vec![noise.1.alto.random() - 0.5; 24_000]
    }

    fn register(&self) -> (i32, i32) {
        (72, 93)
    }

    fn name(&self) -> &str {
        "FLUTE"
    }
}

#[test]
fn a_reseed_through_with_noise_is_what_later_notes_draw_from() {
    let ep = episode();
    let ctx = MusicCtx::new(
        &ep.show,
        &ep.rec,
        &ep.ff,
        [&ep.cfg[0], &ep.cfg[1]],
        [Arc::new(AltoDraw), TestVoice::new("TROMBONE", 0)],
        Vec::new(),
    )
    .unwrap();
    // the part's reseed, on the build's own noise (not a copy)
    let drawn = ctx
        .with_noise(|n| {
            n.1.alto = Generator::new(72_250);
            n.1.alto.random()
        })
        .unwrap();
    let mut want = Generator::new(72_250);
    assert_eq!(drawn.to_bits(), want.random().to_bits());
    // the next note draws on from there: the note's own draw, then the level-matching note's
    let x = ctx.note("FLUTE", 76.0, 0.2, 0.3, 0.8).unwrap();
    let first = want.random() - 0.5;
    let gain = x[0] / first;
    assert!(gain.is_finite() && gain > 0.0, "{gain}");
    want.random();
    let next = want.random().to_bits();
    assert_eq!(ctx.noise().unwrap().1.alto.random().to_bits(), next);
    // a copy from noise() changes nothing in the build
    let mut copy = ctx.noise().unwrap();
    copy.1.alto.random();
    assert_eq!(ctx.noise().unwrap().1.alto.random().to_bits(), next);
}
