//! Flutes: the flute and the piccolo (`docs/studio/voice-engine-2-instruments.md`; the sketch `br.rs`, Aedan's
//! picks Fl1 and Pc1).
//!
//! An air jet from the lips crosses the embouchure hole and hits its far edge; the sound in the tube swings it in and
//! out, after the time the jet's ripples take to cross (a delay the player keeps at a part of the period, quicker
//! for a louder note), and the flow it splits into the tube drives the tube (the jet-drive model). The jet splits on
//! the edge as a soft curve; the edge sheds vortices at large swings. The tube is its own resonances (a peak at every
//! harmonic of the fingered length, weaker up to the tone holes' cutoff). The jet is turbulent: its noise is in the
//! loop (breath that sounds through the tube's notes) and some is heard directly. A faster jet locks to the next
//! resonance up (the octave), and the fingering takes the tube's first resonance for the low octave, its second for
//! the next, its third and fourth above. The player's embouchure holds the jet's swing across the edge near a level
//! that rises with strength, adjusting over 30 ms: a small swing is a sweet round tone, a large one hard and bright.
//! A player sets it for the note before the tongue lets the air go, so a note from rest starts on its breath and its
//! tone grows out of it (the embouchure the ear found each note settling on comes with its trim).
//!
//! The player's ear: each note is first tried alone for 0.42 s and the tube's length trimmed (as a flautist rolls
//! the flute in or out) until it sounds in tune, read finely from the spectrum. The techniques are the player's:
//! a breathy start, flutter tongue, harmonics (a lower note's fingering overblown), a whistle tone (hardly any
//! breath, a high resonance), a jet whistle (the hole covered and blown hard), key clicks (the pads slapped shut with
//! no breath), singing while playing, air alone.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use bf_core::dmath;

use super::brass::{
    self, BLOCK, CHOKED, Ln, OnePole, Svf, Tongue, down, held, hp1, hz, lp1, pitch_of, pow2, seed, smooth, swells,
    unit, vib_of,
};
use super::play::{Mute, Play, Technique};

/// A flute's make-up (the sketch's settings, by its names).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FluteParts {
    /// The pick's name, for its tried notes (`Fl1`).
    pub name: &'static str,
    /// How many resonances, the first one's sharpness, where their sharpness peaks (falling above with the radiation
    /// and the holes), the tone holes' cutoff (Hz), cents per resonance (the head joint's tuning).
    pub nmodes: usize,
    pub q1: f64,
    pub fq: f64,
    pub fcut: f64,
    pub inh: f64,
    /// The jet's crossing as a part of the period, its swing per unit of sound in the hole, its aim off the edge
    /// (even harmonics), its drive, the vortices at the edge.
    pub ftau: f64,
    pub jgain: f64,
    pub yoff: f64,
    pub kp: f64,
    pub kl: f64,
    /// Softest to loudest: the jet's speed.
    pub vmin: f64,
    pub vmax: f64,
    /// Turbulence in the jet, its colour (Hz), the jet's hiss heard directly.
    pub noise: f64,
    pub nfc: f64,
    pub hiss: f64,
    /// The radiation: a high-pass (the flow's change) and a top, Hz.
    pub rad_fc: f64,
    pub rad_lp: f64,
    /// A clean (tongued) start, a breathy one, the stop, s.
    pub att: f64,
    pub batt: f64,
    pub rel: f64,
    /// A tongued start from rest with the embouchure already set for the note (each note in the app): the jet up
    /// to speed in, s.
    pub att_set: f64,
    /// The ear listens to each note from this long after its start, s.
    pub tun_a: f64,
    /// The player's own vibrato (cents, Hz, the wait), mostly in the breath: its swing at that depth.
    pub vib_cents: f64,
    pub vib_hz: f64,
    pub vib_wait: f64,
    pub vib_breath: f64,
    /// How many steps of the tube to one of the sample rate.
    pub os: usize,
    /// The lowest note (C4 on a flute).
    pub low: f64,
    /// The jet amplifies ripples best near this multiple of the note, over a band this sharp.
    pub jbf: f64,
    pub jq: f64,
    /// A whistle tone's jet speed and drive against a normal note's; a key slapped shut: its knock.
    pub wh_u: f64,
    pub wh_k: f64,
    pub click: f64,
    /// The embouchure: the jet's swing held near `a0` + `a1` x strength (in jet widths), adjusting over `agc` s.
    pub agc: f64,
    pub a0: f64,
    pub a1: f64,
    /// The soft limiter: the flute's level that the round's page played at full scale.
    pub full: f64,
    /// The sound's level out.
    pub gain: f64,
    /// The ear's trims made offline, if there are any.
    pub table: Option<&'static FluteTable>,
}

/// The ear's trims (cents) for plain notes, made offline by the ear's own trial passes: each note from `low` to
/// `high` at each strength step from `vels.0` to `vels.1` (in twentieths), a strength between two steps taking the
/// trim between theirs. `embs`, laid out alike: the embouchure each note settled on in the same passes (the jet's
/// gain, log), which the player sets before the note starts.
#[derive(Debug, PartialEq)]
pub struct FluteTable {
    pub low: u8,
    pub high: u8,
    pub vels: (u8, u8),
    pub rows: &'static [f64],
    pub embs: &'static [f64],
}

/// The sample rate the tables were made at (at another, every note is tried).
const TABLE_RATE: f64 = 48_000.0;

const FLUTE: FluteParts = FluteParts {
    name: "flute",
    nmodes: 12,
    q1: 40.0,
    fq: 1500.0,
    fcut: 2200.0,
    inh: -2.0,
    ftau: 0.30,
    jgain: 10.0,
    yoff: 0.7,
    kp: 1.0,
    kl: 0.05,
    vmin: 0.75,
    vmax: 1.25,
    noise: 0.25,
    nfc: 2500.0,
    hiss: 0.03,
    rad_fc: 400.0,
    rad_lp: 9000.0,
    att: 0.025,
    batt: 0.09,
    rel: 0.06,
    att_set: 0.012,
    tun_a: 0.22,
    vib_cents: 6.0,
    vib_hz: 5.2,
    vib_wait: 0.2,
    vib_breath: 0.08,
    os: 2,
    low: 60.0,
    jbf: 1.0,
    jq: 3.0,
    wh_u: 0.55,
    wh_k: 1.0,
    click: 50.0,
    agc: 0.03,
    a0: 1.0,
    a1: 1.6,
    full: 1.0,
    gain: 1.0,
    table: None,
};

impl FluteParts {
    /// Fl1, the silver concert flute: clear and sweet, a little breath in the tone, bright when blown hard.
    pub const FL1: FluteParts = FluteParts {
        name: "Fl1",
        full: super::brass_tune::FL1_FULL,
        gain: super::brass_tune::FL1_GAIN,
        table: super::brass_tune::FL1,
        ..FLUTE
    };
    /// Pc1, the silver piccolo: bright, sparkling and piercing at the top.
    pub const PC1: FluteParts = FluteParts {
        name: "Pc1",
        fq: 3000.0,
        fcut: 4400.0,
        q1: 34.0,
        low: 74.0,
        vib_hz: 5.8,
        vib_cents: 5.0,
        rad_fc: 800.0,
        full: super::brass_tune::PC1_FULL,
        gain: super::brass_tune::PC1_GAIN,
        table: super::brass_tune::PC1,
        ..FLUTE
    };

    /// Resonance n's part of the tube's first.
    fn ratio(&self, n: usize) -> f64 {
        n as f64 * pow2(self.inh * (n as f64 - 1.0) / 1200.0)
    }
}

/// A flute technique: how the jet and the fingers play the note.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Tech {
    Normal,
    /// A harmonic on resonance n.
    Harm(usize),
    /// A whistle tone on resonance n.
    Whistle(usize),
    Jet,
    Click,
    /// Sung at this pitch (MIDI).
    Sing(f64),
    Air,
}

/// A flute's note: the sketch's note and its technique.
#[derive(Clone, Debug)]
struct FNote {
    n: Ln,
    tech: Tech,
}

impl FNote {
    fn of(p: &FluteParts, play: &Play, t: f64) -> FNote {
        let (sfz, swell, swell2) = swells(play);
        let midi = play.midi.clamp(0.0, 127.0);
        let tech = match play.technique {
            Technique::Harmonic(k) => Tech::Harm(usize::from(k).clamp(2, 12)),
            Technique::WhistleTone => Tech::Whistle(4),
            Technique::JetWhistle => Tech::Jet,
            Technique::KeyClick => Tech::Click,
            Technique::Sung(k) => Tech::Sing(midi + f64::from(k)),
            Technique::AirTone => Tech::Air,
            _ => Tech::Normal,
        };
        let bend = play
            .bend
            .filter(|b| b.cents.is_finite() && b.at.is_finite() && b.over.is_finite())
            .map(|b| (b.cents.clamp(-2400.0, 2400.0), b.at, b.over));
        let slur = play.slur_from.filter(|s| s.is_finite()).map(|s| s.clamp(0.0, 127.0));
        let (glide, bend) = match bend {
            Some((c, _, over)) if c == 0.0 && slur.is_some() => (Some(over.max(0.0)), None),
            b => (None, b),
        };
        let mut n = Ln {
            t,
            midi,
            dur: held(play),
            vel: if play.vel.is_finite() {
                play.vel.clamp(0.0, 1.2)
            } else {
                0.0
            },
            tongue: if play.technique == Technique::Breath {
                Tongue::B
            } else {
                Tongue::T
            },
            acc: unit(play.accent),
            sfz,
            tie: play.tie,
            slur,
            glide,
            from: 0.0,
            bend,
            bend2: None,
            swell,
            swell2,
            vib: vib_of(play, (p.vib_cents, p.vib_hz, p.vib_wait)),
            shake: false,
            flutter: play.technique == Technique::Flutter,
            growl: false,
            half: 0.0,
            part: None,
            val: None,
            rel: if matches!(play.mute, Mute::Choked | Mute::Clipped) {
                CHOKED
            } else {
                p.rel
            },
        };
        if let Some(s) = play
            .scoop
            .filter(|b| b.cents.is_finite() && b.at.is_finite() && b.over.is_finite())
        {
            let c = s.cents.clamp(-2400.0, 2400.0);
            n.from = c;
            n.bend2 = n.bend;
            n.bend = Some((-c, s.at, s.over));
        }
        FNote { n, tech }
    }

    /// Which resonance of the fingered tube plays it: the first octave on the first, the second overblown to the
    /// octave, the third on the third and fourth.
    fn mode(&self, p: &FluteParts) -> usize {
        match self.tech {
            Tech::Harm(k) => k.max(2),
            Tech::Whistle(k) => k.max(4),
            _ => {
                let d = self.n.midi - p.low;
                if d < 14.0 {
                    1
                } else if d < 26.0 {
                    2
                } else if d < 31.0 {
                    3
                } else {
                    4
                }
            }
        }
    }
}

/// How long a note's embouchure, set before it, waits for the note to sound before raising the jet's gain, s.
const SET_WAIT: f64 = 0.08;

/// One flautist playing a line of notes (in time order) at the tube's rate `fs`: what is heard (the tube and the
/// jet's hiss), the tube alone, and the embouchure at the end (the jet's gain, log). `corr` is each note's tube trim,
/// cents. `emb` is the embouchure the player sets before the line's first note (as the ear found it holding the
/// note): each start from rest then lets the jet go at once (`att_set`), and the embouchure is not raised while the
/// note is still growing (it waits for the note to reach its swing, `SET_WAIT` at most), so the tone grows out of the
/// breath at its own pace. `None` is the sketch's way, the ear's trial passes: the embouchure relaxed at rest and
/// adjusting from the first breath, which overdrives a start from rest (the jet's gain wound up while the tube is
/// still quiet, the swing overshooting several times over for the first tens of ms).
fn line(
    fl: &FluteParts,
    notes: &[FNote],
    corr: &[f64],
    n_s: usize,
    seed: u64,
    fs: f64,
    emb: Option<f64>,
) -> (Vec<f64>, Vec<f64>, f64) {
    // two draws a step (the jet's turbulence and its hiss), one a block on a flutter
    let flutter = notes.iter().any(|f| f.n.flutter);
    let draws = 2 * n_s + if flutter { n_s / BLOCK + 1 } else { 0 };
    brass::ahead(seed, draws, |rng| line_in(fl, notes, corr, n_s, rng, fs, emb))
}

fn line_in(
    fl: &FluteParts,
    notes: &[FNote],
    corr: &[f64],
    n_s: usize,
    rng: &mut brass::Ahead,
    fs: f64,
    emb: Option<f64>,
) -> (Vec<f64>, Vec<f64>, f64) {
    let dt = 1.0 / fs;
    let nm = fl.nmodes;
    let mut modes = vec![Svf::default(); nm];
    let mut yn = vec![0.0; nm];
    let dmax = (0.02 * fs) as usize;
    let mut vbuf = vec![0.0; dmax + 4];
    let vl = vbuf.len();
    let mut wi = 0usize;
    let mut nz_lp = OnePole::lp(fl.nfc, fs);
    let mut nz_hp = OnePole::lp(300.0, fs);
    let mut hs_lp = OnePole::lp(6000.0, fs);
    let mut jet_lp = OnePole::lp(fs * 0.12, fs);
    let mut jet_bp = Svf::default();
    // the embouchure (the jet's gain, log), where it rests between breaths, and whether a note set before it is
    // still growing (since when); `set` marks a start from rest with the embouchure set
    let rest = emb.unwrap_or(0.0);
    let (mut lg, mut a_ms, mut a_tgt) = (rest, 0.0f64, 1.0f64);
    let (mut growing, mut grow_t, mut set) = (false, 0.0f64, false);
    jet_bp.set(1000.0, 1.0, fs);
    let k_jet = 1.0 - dmath::exp(-dt / 0.003);
    let k_tau = 1.0 - dmath::exp(-dt / 0.01);
    let mut out = vec![0.0; n_s];
    let mut tube = vec![0.0; n_s];
    let (mut qj_prev, mut v_prev) = (0.0, 0.0);
    let mut idx: Option<usize> = None;
    let mut cur_f = -1.0f64;
    let (mut seg_from, mut seg_t0, mut seg_tb) = (0.0, 0.0, 0.01);
    let mut cur_tau = 0.0f64;
    let mut uj_s = 0.0;
    let (mut uj_tgt, mut tau_tgt) = (0.0, 0.001);
    let (mut gain, mut yoff, mut noise, mut hiss, mut kp) = (fl.jgain, fl.yoff, fl.noise, fl.hiss, fl.kp);
    let (mut flut_ph, mut sing_ph, mut sing_f, mut sing_a) = (0.0, 0.0, 0.0, 0.0);
    let mut click = 0.0;
    let mut gate_prev = 0.0;
    for k in 0..n_s {
        let now = k as f64 * dt;
        if k % BLOCK == 0 {
            let mut ni = idx;
            while ni.map_or(0, |i| i + 1) < notes.len() && notes[ni.map_or(0, |i| i + 1)].n.t <= now {
                ni = Some(ni.map_or(0, |i| i + 1));
            }
            if ni != idx {
                let i = ni.unwrap_or(0);
                let fnn = &notes[i];
                let n = &fnn.n;
                let ftube = hz(n.midi) / fl.ratio(fnn.mode(fl)) * pow2(corr[i] / 1200.0);
                let slurred = n.slur.is_some() && i > 0 && notes[i - 1].n.tie;
                let fresh = cur_f < 0.0 || (!slurred && gate_prev < 0.05);
                seg_from = if fresh { dmath::log2(ftube) } else { cur_f };
                set = emb.is_some() && fresh;
                if set {
                    growing = true;
                    grow_t = n.t;
                }
                seg_t0 = n.t;
                seg_tb = if slurred { n.glide.unwrap_or(0.008) } else { 0.004 };
                if fnn.tech == Tech::Click {
                    click = 1.0;
                }
                idx = ni;
            }
            if let Some(i) = idx {
                let fnn = &notes[i];
                let n = &fnn.n;
                let tau = now - n.t;
                let mode = fnn.mode(fl);
                let (cents, _, _) = brass::note_bend(n, tau);
                let ftube = hz(n.midi) / fl.ratio(mode) * pow2((corr[i] + cents) / 1200.0);
                let tb = dmath::log2(ftube);
                let xb = smooth((now - seg_t0) / seg_tb);
                cur_f = seg_from + (tb - seg_from) * xb;
                // the jet: the player keeps its crossing a part of the period, faster for louder
                let fnote = hz(n.midi + cents / 100.0);
                jet_bp.set(fnote * fl.jbf, fl.jq, fs);
                let velk = fl.vmin + (fl.vmax - fl.vmin) * n.vel.clamp(0.0, 1.2);
                a_tgt = fl.a0 + fl.a1 * n.vel.clamp(0.0, 1.2);
                let mut uj = velk;
                let tj = fl.ftau / fnote;
                gain = fl.jgain;
                yoff = fl.yoff;
                noise = fl.noise;
                hiss = fl.hiss;
                kp = fl.kp;
                sing_a = 0.0;
                match fnn.tech {
                    Tech::Whistle(_) => {
                        // a whisper of a jet, barely moving: a thin pure whistle on a high resonance
                        uj *= fl.wh_u;
                        noise *= 0.15;
                        hiss *= 0.1;
                        kp *= fl.wh_k;
                    }
                    Tech::Jet => {
                        // the hole covered and blown hard: a rush of air through the tube, no jet tone
                        uj *= 1.6;
                        gain *= 0.0;
                        noise *= 18.0;
                        hiss *= 4.0;
                    }
                    Tech::Air => {
                        // breath only: the jet aimed off the edge, the tube's notes in the hiss
                        gain *= 0.05;
                        noise *= 8.0;
                        hiss *= 5.0;
                        uj *= 0.8;
                    }
                    Tech::Click => uj *= 0.0,
                    Tech::Sing(m) => {
                        sing_a = 0.35;
                        sing_f = hz(m);
                    }
                    Tech::Normal | Tech::Harm(_) => {}
                }
                // the start: a clean tongued "tu", a breathy "hoo", or a slur's flowing breath
                let slurred = n.slur.is_some() && i > 0 && notes[i - 1].n.tie;
                let mut gate = if slurred {
                    1.0
                } else if n.tongue == Tongue::B {
                    smooth(tau / fl.batt)
                } else {
                    let at = if set { fl.att_set } else { fl.att } * (1.0 - 0.5 * n.acc);
                    let g = smooth(tau / at);
                    g * (1.0 + (0.15 + 0.4 * n.acc) * dmath::exp(-(tau / 0.04)))
                };
                if n.tongue == Tongue::B && !slurred {
                    noise *= 1.0 + 3.0 * dmath::exp(-(tau / 0.15));
                }
                if let Some((a, z, over)) = n.sfz {
                    gate *= 1.0 + (a - 1.0) * dmath::exp(-(tau / 0.05));
                    gate *= z + (1.0 - z) * dmath::exp(-(tau / over));
                }
                for (a, z, over) in [n.swell, n.swell2].into_iter().flatten() {
                    gate *= a + (z - a) * smooth(tau / over);
                }
                if !n.tie && tau > n.dur {
                    gate *= 1.0 - smooth((tau - n.dur) / n.rel);
                }
                if n.flutter {
                    flut_ph += 26.0 * BLOCK as f64 * dt * (1.0 + 0.1 * rng.gauss());
                    let s = 0.5 + 0.5 * dmath::sin(2.0 * dmath::PI * flut_ph);
                    gate *= 1.0 - 0.6 * s * smooth(tau / 0.05);
                    noise *= 1.0 + 1.5 * s;
                }
                if let Some((vc, r, d)) = n.vib {
                    // a flute's vibrato is mostly in the breath
                    let grow = smooth((tau - d) / 0.35);
                    gate *= 1.0
                        + fl.vib_breath
                            * (vc / fl.vib_cents.max(1.0)).min(3.0)
                            * grow
                            * dmath::sin(2.0 * dmath::PI * r * (tau - d).max(0.0));
                }
                if fnn.tech == Tech::Jet {
                    // the whoosh: a hard push that dies, the tube swept by the fingers
                    gate = smooth(tau / 0.01) * dmath::exp(-(tau / (n.dur * 0.6).max(0.05)));
                }
                gate_prev = gate;
                uj_tgt = uj * gate.max(0.0);
                tau_tgt = tj;
                // the tube
                let f1 = pow2(cur_f);
                for (m, (mode, y)) in modes.iter_mut().zip(yn.iter_mut()).enumerate() {
                    let f = f1 * fl.ratio(m + 1);
                    let x = f / fl.fq;
                    let q = (fl.q1 * (f / f1.max(1.0)).sqrt().min(2.5)) / (1.0 + x * x * 0.6);
                    let x4 = {
                        let r = f / fl.fcut;
                        let r2 = r * r;
                        r2 * r2
                    };
                    let yy = 1.0 / (1.0 + x4) / (1.0 + 0.15 * m as f64);
                    let (f, yy) = if f > 0.45 * fs { (0.45 * fs, 0.0) } else { (f, yy) };
                    mode.set(f, q.max(3.0), fs);
                    *y = yy;
                }
            } else {
                uj_tgt = 0.0;
            }
        }
        uj_s += (uj_tgt - uj_s) * k_jet;
        cur_tau += (tau_tgt - cur_tau) * k_tau;
        // the jet's crossing, as a delay
        let dl = (cur_tau * fs).clamp(1.0, dmax as f64);
        let ri = wi as f64 + vl as f64 - dl;
        let i0 = ri.floor() as usize % vl;
        let i1 = (i0 + 1) % vl;
        let fr = ri - ri.floor();
        let vd = vbuf[i0] * (1.0 - fr) + vbuf[i1] * fr;
        // turbulence in the jet
        let turb = nz_hp.hpt(nz_lp.lpt(rng.gauss())) * noise * 3.0;
        let mut drive = uj_s;
        if sing_a > 0.0 {
            sing_ph += sing_f * dt;
            let s = dmath::sin(2.0 * dmath::PI * sing_ph) + 0.3 * dmath::sin(4.0 * dmath::PI * sing_ph);
            drive *= 1.0 + sing_a * s;
        }
        let ac = gain * dmath::exp(lg) * jet_bp.tick(vd) / (drive + 0.05);
        a_ms += (ac * ac - a_ms) * (dt / 0.01).min(1.0);
        if drive > 0.1 {
            let a_est = (2.0 * a_ms).sqrt() + 1e-6;
            let step = dt / fl.agc * dmath::ln(a_tgt / a_est).clamp(-3.0, 3.0);
            if growing && (a_est >= a_tgt || now - grow_t > SET_WAIT) {
                growing = false;
            }
            if !growing || step < 0.0 {
                lg += step;
            }
            lg = lg.clamp(-3.0, 0.7);
        } else {
            lg += (rest - lg) * (dt / 0.05).min(1.0);
        }
        let eta = ac + yoff + turb;
        let split = dmath::tanh(eta);
        let qj = jet_lp.lpt(split) * drive;
        // the jet's flow change drives the tube; the hole's edge sheds vortices at large swings
        let mut dp = kp * (qj - qj_prev) * fs / 2000.0 * drive;
        qj_prev = qj;
        if drive > 0.05 {
            dp -= fl.kl * v_prev * v_prev.abs() * 40.0;
        }
        if click > 0.0 {
            dp += click * fl.click * if click > 0.9 { 1.0 } else { 0.0 };
            click *= 0.98;
            if click < 0.01 {
                click = 0.0;
            }
        }
        let mut v = 0.0;
        for (mode, y) in modes.iter_mut().zip(&yn) {
            v += y * mode.tick(dp);
        }
        vbuf[wi] = v;
        wi = (wi + 1) % vl;
        v_prev = v;
        tube[k] = v;
        let hs = hs_lp.lpt(rng.gauss()) * hiss * drive * drive;
        let mut o = v + hs;
        if sing_a > 0.0 {
            o += 0.02 * drive * dmath::sin(2.0 * dmath::PI * sing_ph);
        }
        out[k] = o;
    }
    (out, tube, lg)
}

/// A finer reading of a pitch near `f`: the peak of `x`'s windowed spectrum within 40 cents of it (the
/// autocorrelation's peak, read between whole-sample lags, can be several cents out for high notes).
fn pitch_refine(x: &[f64], fs: f64, f: f64) -> f64 {
    let n = x.len();
    if n < 64 || f <= 0.0 {
        return f;
    }
    let win: Vec<f64> = (0..n)
        .map(|i| 0.5 - 0.5 * dmath::cos(2.0 * dmath::PI * i as f64 / (n - 1) as f64))
        .collect();
    let mag = |fr: f64| -> f64 {
        let w = 2.0 * dmath::PI * fr / fs;
        let (mut re, mut im) = (0.0, 0.0);
        let (c1, s1) = (dmath::cos(w), dmath::sin(w));
        let (mut c, mut s) = (1.0f64, 0.0f64);
        for (xi, wi) in x.iter().zip(&win) {
            let v = xi * wi;
            re += v * c;
            im -= v * s;
            let c2 = c * c1 - s * s1;
            s = s * c1 + c * s1;
            c = c2;
        }
        dmath::ln((re * re + im * im).sqrt().max(1e-30))
    };
    let cents: Vec<f64> = (-20..=20).map(|k| f64::from(k) * 2.0).collect();
    let m: Vec<f64> = cents.iter().map(|c| mag(f * pow2(c / 1200.0))).collect();
    let mut k = 0;
    for i in 1..m.len() {
        if m[i] > m[k] {
            k = i;
        }
    }
    if k == 0 || k == m.len() - 1 {
        return f;
    }
    let (a, b, c) = (m[k - 1], m[k], m[k + 1]);
    let den = a - 2.0 * b + c;
    let off = if den.abs() > 1e-12 { 0.5 * (a - c) / den } else { 0.0 };
    f * pow2((cents[k] + 2.0 * off) / 1200.0)
}

const TRIAL_SEED: u64 = 17;

/// The player's ear on one note alone, held: the tube trimmed (three tries at most) until it sounds in tune, and the
/// embouchure the note settled on in the last try (none for the techniques with no note to hold).
fn tune(fl: &FluteParts, f: &FNote, fs: f64) -> (f64, Option<f64>) {
    if matches!(f.tech, Tech::Jet | Tech::Air | Tech::Click) {
        return (0.0, None);
    }
    let fs_s = fs * fl.os as f64;
    let n = &f.n;
    let mut g = n.sfz.map_or(1.0, |s| s.1);
    for (a, z, _) in [n.swell, n.swell2].into_iter().flatten() {
        // a flute's pitch follows the breath: a swelling note is tuned at its middle
        g *= 0.5 * (a + z);
    }
    let mut t = FNote {
        n: Ln {
            t: 0.0,
            dur: 0.45,
            tie: false,
            slur: None,
            glide: None,
            from: 0.0,
            bend: None,
            bend2: None,
            swell: None,
            swell2: None,
            vib: None,
            flutter: false,
            sfz: None,
            tongue: Tongue::T,
            ..n.clone()
        },
        tech: if matches!(f.tech, Tech::Sing(_)) {
            Tech::Normal
        } else {
            f.tech
        },
    };
    if (g - 1.0).abs() > 0.05 {
        let velk = (fl.vmin + (fl.vmax - fl.vmin) * n.vel.clamp(0.0, 1.2)) * g;
        t.n.vel = ((velk - fl.vmin) / (fl.vmax - fl.vmin)).clamp(0.0, 1.2);
    }
    let target = hz(n.midi);
    let (mut c, mut emb) = (0.0, 0.0);
    for _ in 0..3 {
        let (_, tube, e) = line(
            fl,
            std::slice::from_ref(&t),
            &[c],
            (0.42 * fs_s) as usize,
            TRIAL_SEED,
            fs_s,
            None,
        );
        emb = e;
        let a = (fl.tun_a * fs_s) as usize;
        let e = ((0.40 * fs_s) as usize).min(tube.len());
        let (fr, _) = pitch_of(&tube[a..e], fs_s, target);
        if fr <= 0.0 {
            break;
        }
        let fr = pitch_refine(&tube[a..e], fs_s, fr);
        let off = 1200.0 * dmath::log2(target / fr);
        if off.abs() > 150.0 {
            break;
        }
        c += off;
        if off.abs() < 1.0 {
            break;
        }
    }
    (c, Some(emb))
}

type TriedKey = (&'static str, Vec<u64>);
type Tried = (f64, Option<f64>);

fn tried() -> &'static Mutex<HashMap<TriedKey, Tried>> {
    static TRIED: OnceLock<Mutex<HashMap<TriedKey, Tried>>> = OnceLock::new();
    TRIED.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The trim the ear settles on for `f` and the embouchure the player sets for it: from the table where it has the
/// note, else tried when first asked and kept.
fn tuned(fl: &FluteParts, f: &FNote, fs: f64) -> Tried {
    let n = &f.n;
    let g: Vec<u64> = [n.sfz, n.swell, n.swell2]
        .into_iter()
        .map(|s| s.map_or(u64::MAX, |s| (s.0 + 3.0 * s.1).to_bits()))
        .collect();
    let tech = match f.tech {
        Tech::Harm(k) => 100 + k as u64,
        Tech::Whistle(k) => 200 + k as u64,
        Tech::Jet | Tech::Air | Tech::Click => return (0.0, None),
        Tech::Normal | Tech::Sing(_) => 0,
    };
    if let Some(t) = fl.table
        && fs == TABLE_RATE
        && tech == 0
        && g.iter().all(|&b| b == u64::MAX)
        && n.midi.fract() == 0.0
        && n.midi >= f64::from(t.low)
        && n.midi <= f64::from(t.high)
    {
        let k = n.vel * 20.0;
        let (k0, k1) = (f64::from(t.vels.0), f64::from(t.vels.1));
        if k >= k0 && k <= k1 {
            let nv = usize::from(t.vels.1 - t.vels.0) + 1;
            let row = (n.midi as usize - usize::from(t.low)) * nv;
            let i = ((k - k0).floor() as usize).min(nv - 1);
            let j = (i + 1).min(nv - 1);
            let x = k - k0 - i as f64;
            let between = |r: &[f64]| r[row + i] + (r[row + j] - r[row + i]) * x;
            return (between(t.rows), Some(between(t.embs)));
        }
    }
    let key = (
        fl.name,
        [n.midi.to_bits(), n.vel.to_bits(), tech, fs.to_bits()]
            .into_iter()
            .chain(g)
            .collect(),
    );
    if let Some(c) = tried().lock().ok().and_then(|m| m.get(&key).copied()) {
        return c;
    }
    let c = tune(fl, f, fs);
    if let Ok(mut m) = tried().lock() {
        m.insert(key, c);
    }
    c
}

/// What the player's ear settles on for the note `play` at sample rate `fs`: its tube's trim, cents.
pub fn ear(p: &FluteParts, play: &Play, fs: f64) -> f64 {
    tuned(p, &FNote::of(p, play, 0.0), fs).0
}

/// The ear's trial passes for a plain note (the tables' rows): its trim, cents, and the embouchure it settled on.
pub fn tune_plain(p: &FluteParts, midi: f64, vel: f64, fs: f64) -> (f64, f64) {
    let (c, e) = tune(p, &FNote::of(p, &Play::plain(midi, 0.5, vel), 0.0), fs);
    (c, e.unwrap_or(0.0))
}

const SLUR_PRE: f64 = 0.2;
/// How long the tube rings past the breath's stop, s.
const TAIL: f64 = 0.2;

/// How long the note `play` rings on `p` with no length asked for, seconds, at most `longest`.
pub fn rings_for(p: &FluteParts, play: &Play, longest: f64) -> f64 {
    let dur = held(play);
    let ring = if play.tie {
        dur + 0.012
    } else {
        let rel = if matches!(play.mute, Mute::Choked | Mute::Clipped) {
            CHOKED
        } else {
            p.rel
        };
        dur + rel + TAIL
    };
    ring.clamp(0.05, longest.max(0.05))
}

/// The radiated sound of a line's tube and jet, brought down to `fs` (`n` samples).
fn radiate(fl: &FluteParts, x: &mut [f64], fs: f64, n: usize) -> Vec<f64> {
    let fs_s = fs * fl.os as f64;
    hp1(x, fl.rad_fc, fs_s);
    lp1(x, fl.rad_lp.min(0.45 * fs_s), fs_s);
    down(x, fl.os, n)
}

/// The note `play` on the flute `p`: `len` samples at sample rate `fs`, as repeat `variant` (each repeat with its own
/// breath).
pub fn note(p: &FluteParts, play: &Play, len: usize, variant: u64, fs: f64) -> Vec<f64> {
    if len == 0 {
        return Vec::new();
    }
    let fs_s = fs * p.os as f64;
    let rings = rings_for(p, play, f64::INFINITY);
    let out_n = len.min((rings * fs) as usize + 1);
    let slur = play.slur_from.filter(|s| s.is_finite());
    let pre = if slur.is_some() {
        (SLUR_PRE * fs).round() as usize
    } else {
        0
    };
    let t0 = pre as f64 / fs;
    let f = FNote::of(p, play, t0);
    let mut notes = Vec::new();
    let mut corr = Vec::new();
    // the embouchure is set for the line's first note: the lead-in's when slurred
    let mut emb = None;
    if let Some(s) = slur {
        let mut lead = FNote::of(p, &Play::plain(s.clamp(0.0, 127.0), t0, play.vel), 0.0);
        lead.n.tie = true;
        let (c, e) = tuned(p, &lead, fs);
        corr.push(c);
        emb = Some(e);
        notes.push(lead);
    }
    let (c, e) = tuned(p, &f, fs);
    corr.push(c);
    let emb = emb.unwrap_or(e);
    notes.push(f);
    let n_s = (pre + out_n) * p.os + 32 * p.os + 2;
    let (mut out, _, _) = line(p, &notes, &corr, n_s, seed(variant, play.midi), fs_s, emb);
    let x = radiate(p, &mut out, fs, pre + out_n);
    let mut y = x[pre..].to_vec();
    y.resize(len, 0.0);
    brass::knee(&mut y, p.full);
    y.iter_mut().for_each(|v| *v *= p.gain);
    brass::joins(&mut y, play, fs);
    y
}

/// A line of notes in one breath, as the sketch plays a riff (for measuring the port against it): each note's start
/// and play, its trim given or tried, the radiated sound (before the soft limiter), `len` samples at `fs`.
pub fn phrase(
    p: &FluteParts,
    notes: &[(f64, Play)],
    tunings: Option<&[f64]>,
    seed: u64,
    fs: f64,
    len: usize,
) -> Vec<f64> {
    let fs_s = fs * p.os as f64;
    let ls: Vec<FNote> = notes.iter().map(|(t, play)| FNote::of(p, play, *t)).collect();
    let corr: Vec<f64> = match tunings {
        Some(t) => t.to_vec(),
        None => ls.iter().map(|f| tuned(p, f, fs).0).collect(),
    };
    let n_s = len * p.os + 32 * p.os + 2;
    let (mut out, _, _) = line(p, &ls, &corr, n_s, seed, fs_s, None);
    radiate(p, &mut out, fs, len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::engine2::brass::tests::{FS, at_hz, db, join_marks, joined, pitch, still, upper};
    use crate::voice::engine2::play::{Bend, Swell};

    fn render(p: &FluteParts, play: &Play, variant: u64, fs: f64) -> Vec<f64> {
        let n = (rings_for(p, play, 12.0) * fs) as usize;
        note(p, play, n, variant, fs)
    }

    /// Both picks sit on their notes across their ranges, held, vibrato off (measured within 1.2 cents), and at 44.1
    /// kHz too (within 1.7).
    #[test]
    fn the_flutes_play_in_tune_across_their_ranges() {
        for (p, ms) in [
            (FluteParts::FL1, vec![72.0, 79.0, 86.0, 93.0]),
            (FluteParts::PC1, vec![74.0, 86.0, 98.0]),
        ] {
            for m in ms {
                for fs in [FS, 44_100.0] {
                    let x = render(&p, &still(m, 1.0, 0.7), 0, fs);
                    let c = pitch(&x, hz(m), 0.4, 0.9, fs);
                    assert!(c.abs() < 3.0, "{} {m} at {fs} Hz: {c:.2} cents", p.name);
                }
            }
        }
    }

    /// A strength between the table's steps takes the trim between theirs: within a quarter of a cent of the ear's
    /// own trial passes (measured 0.25 at most over both flutes), the note in tune (measured 1.1 cents at most).
    #[test]
    fn a_strength_between_the_tables_steps_is_in_tune() {
        for (p, m, v) in [
            (FluteParts::FL1, 81.0, 0.625),
            (FluteParts::FL1, 66.0, 0.725),
            (FluteParts::PC1, 95.0, 0.475),
            (FluteParts::PC1, 103.0, 0.875),
        ] {
            let (t, e) = (ear(&p, &Play::plain(m, 0.5, v), FS), tune_plain(&p, m, v, FS).0);
            assert!(
                (t - e).abs() < 0.3,
                "{} {m} at {v}: table {t:.2}, tried {e:.2} cents",
                p.name
            );
            let x = render(&p, &still(m, 1.0, v), 0, FS);
            let c = pitch(&x, hz(m), 0.4, 0.9, FS);
            assert!(c.abs() < 3.0, "{} {m} at {v}: {c:.2} cents", p.name);
        }
    }

    /// Blown harder, the 3rd to 6th harmonics climb against the fundamental (measured 9.4 dB on the flute, 10.5 on
    /// the piccolo, between strengths 0.35 and 0.95).
    #[test]
    fn louder_is_brighter() {
        for (p, m) in [(FluteParts::FL1, 79.0), (FluteParts::PC1, 86.0)] {
            let soft = render(&p, &still(m, 0.6, 0.35), 0, FS);
            let loud = render(&p, &still(m, 0.6, 0.95), 0, FS);
            let (s, l) = (
                upper(&soft, hz(m), 3, 6, 0.2, 0.5, FS),
                upper(&loud, hz(m), 3, 6, 0.2, 0.5, FS),
            );
            assert!(l - s > 4.0, "{}: soft {s:.1} dB, loud {l:.1} dB", p.name);
        }
    }

    /// Every flute technique is finite, within full scale, sounds, and changes the note; the back of the tongue is
    /// the tip's (the sketch's flute tongues them alike: double tonguing is the notes in a row).
    #[test]
    fn every_technique_sounds_and_changes_the_note() {
        let p = FluteParts::FL1;
        let base = Play::plain(79.0, 0.9, 0.7);
        let plain = render(&p, &base, 0, FS);
        let swell = |from: f64, to: f64, over: f64| Some(Swell { from, to, over });
        let bend = |c: f64, at: f64, over: f64| Some(Bend { cents: c, at, over });
        let asks = [
            (
                "breathy start",
                Play {
                    technique: Technique::Breath,
                    ..base
                },
            ),
            (
                "flutter",
                Play {
                    technique: Technique::Flutter,
                    ..base
                },
            ),
            (
                "3rd harmonic",
                Play {
                    technique: Technique::Harmonic(3),
                    ..base
                },
            ),
            (
                "whistle tone",
                Play {
                    technique: Technique::WhistleTone,
                    ..base
                },
            ),
            (
                "jet whistle",
                Play {
                    technique: Technique::JetWhistle,
                    ..base
                },
            ),
            (
                "key click",
                Play {
                    technique: Technique::KeyClick,
                    ..base
                },
            ),
            (
                "sung a fourth under",
                Play {
                    technique: Technique::Sung(-5),
                    ..base
                },
            ),
            (
                "air",
                Play {
                    technique: Technique::AirTone,
                    ..base
                },
            ),
            (
                "sforzando",
                Play {
                    swell: swell(1.6, 0.55, 0.12),
                    ..base
                },
            ),
            (
                "bend",
                Play {
                    bend: bend(-100.0, 0.3, 0.3),
                    ..base
                },
            ),
            (
                "scoop",
                Play {
                    scoop: bend(-100.0, 0.0, 0.1),
                    ..base
                },
            ),
            ("accent", Play { accent: 1.0, ..base }),
            (
                "clipped",
                Play {
                    mute: Mute::Clipped,
                    ..base
                },
            ),
        ];
        let top = 0.97 * p.full * p.gain + 1e-9;
        for (what, play) in asks {
            let x = render(&p, &play, 0, FS);
            assert!(x.iter().all(|v| v.is_finite()), "{what}");
            assert!(x.iter().all(|v| v.abs() <= top), "{what}: over full scale");
            assert!(x.iter().any(|v| v.abs() > 1e-4), "{what}: silent");
            let n = x.len().min(plain.len());
            let d: f64 = x[..n]
                .iter()
                .zip(&plain[..n])
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>();
            let e: f64 = plain[..n].iter().map(|v| v * v).sum();
            assert!(d / e > 0.01, "{what}: the same as plain");
        }
        let back = render(
            &p,
            &Play {
                technique: Technique::BackTongue,
                ..base
            },
            0,
            FS,
        );
        assert_eq!(back, plain);
        // the air tone and the jet whistle are mostly breath: little of the note's fundamental
        for t in [Technique::AirTone, Technique::JetWhistle] {
            let x = render(&p, &Play { technique: t, ..base }, 0, FS);
            let f = at_hz(&x, hz(79.0), 0.2, 0.7, FS) - db(&x, 0.2, 0.7, FS);
            let g = at_hz(&plain, hz(79.0), 0.2, 0.7, FS) - db(&plain, 0.2, 0.7, FS);
            assert!(f < g - 3.0, "{t:?}: {f:.1} against {g:.1}");
        }
    }

    /// A tied note and the note slurred from it join without a click, a gap or a doubled start (measured at most
    /// 0.76, 3.0 dB under and 0.55 dB over).
    #[test]
    fn slurred_and_tied_notes_join_without_a_click() {
        for (p, a, b) in [(FluteParts::FL1, 79.0, 83.0), (FluteParts::PC1, 86.0, 91.0)] {
            let x = render(
                &p,
                &Play {
                    tie: true,
                    ..Play::plain(a, 0.4, 0.7)
                },
                0,
                FS,
            );
            let y = render(
                &p,
                &Play {
                    slur_from: Some(a),
                    ..Play::plain(b, 0.5, 0.7)
                },
                0,
                FS,
            );
            let (sum, s) = joined(&x, &y, 0.4, FS);
            let (step, lo, hi) = join_marks(&sum, s, FS);
            assert!(step < 1.5, "{} {a} to {b}: a click ({step:.2})", p.name);
            assert!(lo > -6.0, "{} {a} to {b}: a gap ({lo:.1} dB)", p.name);
            assert!(hi < 3.0, "{} {a} to {b}: a doubled start ({hi:.1} dB)", p.name);
        }
    }

    /// A note from rest starts on its breath and its tone grows out of it, as a flautist's does: over the first 20 ms
    /// the breath between the harmonics is about as loud as in the held note (measured 4.2 dB under at most; with the
    /// embouchure not set before the note, 10.6 to 33 dB under), and over 20 to 60 ms the 3rd harmonic stands no
    /// higher over the fundamental than in the held note (measured 1.4 dB over at most; not set, 9.3 to 10.3 dB over:
    /// the jet overdriven while the embouchure wound up).
    #[test]
    fn a_note_from_rest_starts_on_its_breath() {
        for (p, m) in [
            (FluteParts::FL1, 72.0),
            (FluteParts::FL1, 84.0),
            (FluteParts::PC1, 84.0),
            (FluteParts::PC1, 96.0),
            (FluteParts::PC1, 103.0),
        ] {
            let x = render(&p, &still(m, 0.6, 0.62), 0, FS);
            let f = hz(m);
            let blip = upper(&x, f, 3, 3, 0.02, 0.06, FS) - upper(&x, f, 3, 3, 0.25, 0.4, FS);
            assert!(
                blip < 4.0,
                "{} {m}: the 3rd harmonic {blip:.1} dB over the held note's",
                p.name
            );
            let between =
                |a: f64, b: f64| [1.5, 2.5, 3.5].iter().map(|k| at_hz(&x, k * f, a, b, FS)).sum::<f64>() / 3.0;
            let held = (0..8)
                .map(|i| {
                    let a = 0.25 + 0.02 * f64::from(i);
                    between(a, a + 0.02)
                })
                .sum::<f64>()
                / 8.0;
            let first = between(0.0, 0.02) - held;
            assert!(
                first > -8.0,
                "{} {m}: the breath {first:.1} dB under the held note's",
                p.name
            );
        }
    }

    /// The same note twice is the same, another repeat its own breath; notes the round never tried (the top, the
    /// bottom, full strength with a full accent) stay finite, within full scale and never fall silent.
    #[test]
    fn repeats_differ_and_untried_notes_stay_sound() {
        for p in [FluteParts::FL1, FluteParts::PC1] {
            let play = Play::plain(84.0, 0.5, 0.7);
            assert_eq!(render(&p, &play, 0, FS), render(&p, &play, 0, FS));
            assert_ne!(render(&p, &play, 0, FS), render(&p, &play, 1, FS));
        }
        for (p, play) in [
            (
                FluteParts::FL1,
                Play {
                    accent: 1.0,
                    ..Play::plain(98.0, 0.6, 1.2)
                },
            ),
            (FluteParts::FL1, Play::plain(60.0, 0.6, 0.9)),
            (FluteParts::PC1, Play::plain(108.0, 0.6, 1.0)),
        ] {
            let x = render(&p, &play, 0, FS);
            assert!(
                x.iter()
                    .all(|v| v.is_finite() && v.abs() <= 0.97 * p.full * p.gain + 1e-9),
                "{}",
                play.midi
            );
            let lv: Vec<f64> = (0..12)
                .map(|k| db(&x, 0.05 * k as f64, 0.05 * (k + 1) as f64, FS))
                .collect();
            let most = lv.iter().copied().fold(f64::MIN, f64::max);
            assert!(
                lv.iter().all(|&v| v > most - 40.0),
                "{} {}: a silence",
                p.name,
                play.midi
            );
        }
    }
}
