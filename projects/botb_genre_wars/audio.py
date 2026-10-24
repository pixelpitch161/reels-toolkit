"""Genre Wars soundtrack. Every band note comes from music.notes() (the same list that drives the cone pumps and
the sparkles), i.e. from the moves of the cached battle. Four genre arrangements of one original song:

    ORCHESTRA  timpani + cymbal, galloping cellos, violin pad + brass stabs, french horns
    ROCK       kit, pick bass, 3-3-2 power chords, distorted lead guitar
    EDM        four-on-the-floor, off-beat sub bass, supersaw stabs, saw lead
    JAZZ       swung ride + brushes, walking upright bass, rootless piano comping, tenor sax

Each layer is calibrated so every genre stem sounds balanced ON ITS OWN (the winner's solo is that stem) and all
four stems are equally loud. Per-note gains follow stage share (music.gains): bass + harmony gently, drums and
melody steeply, so the groove and the tune belong to the leader and the song audibly morphs at every lead change
(plus a stinger in the new leader's style). A knocked-out genre dives in pitch and drops out. Around the band: the
bell, the crowd, the win hit, the crack + falling tiles at 60.4 s, a riser into the opening burst, caption thuds.
Buses are folded into one periodic 61 s buffer with circular reverb so the loop seam is clean.

    python audio.py        # -> audio.wav + section levels + band margins + quietest window"""
import math
import os
import sys
import numpy as np
from scipy.signal import lfilter
from scipy.ndimage import minimum_filter1d, uniform_filter1d

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit import audio as A
import timeline as T
import music as MU

SR = A.SR
NS = int(round(T.DUR * SR))
RNG = np.random.default_rng(7)
TARGET_DB = -11.5           # RMS of the solo section after mastering
GEN = ['orch', 'rock', 'edm', 'jazz']


def tt(n):
    return np.arange(n) / SR


def f_of(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def nz(n, seed=None):
    return (np.random.default_rng(seed) if seed is not None else RNG).normal(0, 1, n)


def ph_of(f):
    return np.cumsum(f) / SR


def sawp(ph):
    return 2 * (ph % 1) - 1


def ens(m, n, det, voices, vib=0.0, vib_f=5.5, vib_d=0.15, bend=0.0, bend_t=0.05):
    """detuned saw ensemble with optional delayed vibrato and an onset pitch scoop (semitones)"""
    t = tt(n)
    f0 = f_of(m) * 2 ** (bend * np.exp(-t / bend_t) / 12)
    f0 = f0 * (1 + vib * np.sin(2 * math.pi * vib_f * t) * np.clip((t - vib_d) / 0.2, 0, 1))
    out = np.zeros(n)
    for v in range(voices):
        d = 1 + det * (v - (voices - 1) / 2)
        out += sawp(ph_of(f0 * d) + v * 0.37)
    return out / voices


# ================================================================== ORCHESTRA
def o_timp(m, vel, roll=False):
    n = int((0.5 if roll else 1.6) * SR)
    t = tt(n)
    f = f_of(m) * (1 + 0.03 * np.exp(-t / 0.05))
    x = sum(a * np.sin(2 * math.pi * np.cumsum(f * r) / SR) * np.exp(-t / d) for r, a, d in
            ((1, 1.0, 0.9), (1.5, 0.5, 0.55), (1.99, 0.3, 0.4), (2.44, 0.18, 0.22)))
    x += A.lp(nz(n), 900) * np.exp(-t / 0.02) * 0.6
    if roll:
        x *= np.exp(-t / 0.18)
    return x * vel


def o_cym(vel):
    n = int(2.6 * SR)
    t = tt(n)
    x = A.hp(nz(n, 21), 3000) * np.exp(-t / 1.1) * 0.6
    x += sum(np.sin(2 * math.pi * f * t + f) * np.exp(-t / 0.9) * 0.05 for f in (3870, 5210, 6630, 7990))
    return x * np.clip(t / 0.004, 0, 1) * vel


def o_cello(m, dur, vel):
    n = int((dur + 0.05) * SR)
    x = ens(m, n, 0.004, 3) + 0.3 * ens(m - 12, n, 0.003, 2)
    x = A.lp(x, 1900) + A.bp(nz(n), 1500, 4000) * 0.03
    return x * A.adsr(n, 0.008, 0.05, 0.75, 0.035) * vel


def o_vlns(ch, dur, vel):
    name, root, tri, rl = MU.CHORDS[ch]
    n = int((dur + 0.35) * SR)
    x = np.zeros(n)
    for m in [tri[0] + 12, tri[1] + 12, tri[2] + 12, tri[0] + 24]:
        x += ens(m, n, 0.006, 4, vib=0.004, vib_d=0.2)
    x = A.lp(x / 4, 3300)
    return x * A.adsr(n, 0.16, 0.2, 0.85, 0.32) * vel


def o_brass(ch, dur, vel):
    name, root, tri, rl = MU.CHORDS[ch]
    n = int((dur + 0.12) * SR)
    t = tt(n)
    x = sum(ens(m, n, 0.005, 2) for m in [tri[0], tri[1], tri[2], tri[0] - 12]) / 4
    k = np.exp(-t / 0.06)
    x = A.lp(x, 2600) * k + A.lp(x, 900) * (1 - k)
    x = np.tanh(x * 2.2) / 1.6
    return x * A.adsr(n, 0.018, 0.08, 0.7, 0.08) * vel


def o_horn(m, dur, vel):
    n = int((dur + 0.14) * SR)
    t = tt(n)
    x = ens(m, n, 0.004, 2, vib=0.005, vib_d=0.18, bend=-0.4, bend_t=0.03)
    k = np.clip(t / 0.09, 0, 1)
    x = A.lp(x, 700) * (1 - k) + A.lp(x, 1700) * k
    x = np.tanh(x * 1.6)
    return x * A.adsr(n, 0.035, 0.1, 0.85, 0.11) * vel


def o_pizz(m, vel):
    n = int(0.35 * SR)
    t = tt(n)
    x = A.lp(ens(m, n, 0.003, 2), 1600) * np.exp(-t / 0.12) + A.bp(nz(n), 400, 1200) * np.exp(-t / 0.01) * 0.3
    return x * vel


# ================================================================== ROCK
def r_kick(vel):
    n = int(0.42 * SR)
    t = tt(n)
    f = 52 + 110 * np.exp(-t / 0.03)
    x = np.sin(2 * math.pi * np.cumsum(f) / SR) * np.exp(-t / 0.22) + A.hp(nz(n), 3000) * np.exp(-t / 0.004) * 0.45
    return np.tanh(x * 1.4) * vel


def r_snare(vel):
    n = int(0.36 * SR)
    t = tt(n)
    x = A.bp(nz(n), 1500, 8000) * np.exp(-t / 0.15) * 0.85
    x += (np.sin(2 * math.pi * 185 * t) + 0.5 * np.sin(2 * math.pi * 320 * t)) * np.exp(-t / 0.05) * 0.7
    return x * vel


def r_hat(vel):
    n = int(0.09 * SR)
    t = tt(n)
    return A.hp(nz(n), 8000) * np.exp(-t / 0.028) * 0.6 * vel


def r_crash(vel):
    n = int(2.2 * SR)
    t = tt(n)
    x = A.hp(nz(n, 22), 4500) * np.exp(-t / 0.8) * 0.6
    x += sum(np.sin(2 * math.pi * f * t) * np.exp(-t / 0.6) * 0.04 for f in (4210, 5530, 7020))
    return x * vel


def r_tom(m, vel):
    n = int(0.4 * SR)
    t = tt(n)
    f = f_of(m) * (1 + 0.5 * np.exp(-t / 0.03))
    return (np.sin(2 * math.pi * np.cumsum(f) / SR) * np.exp(-t / 0.2) + A.bp(nz(n), 400, 3000) * np.exp(-t / 0.02) * 0.3) * vel


def r_bass(m, dur, vel):
    n = int((dur + 0.04) * SR)
    t = tt(n)
    x = sawp(ph_of(np.full(n, f_of(m))))
    k = np.exp(-t / 0.06)
    x = A.lp(x, 2400) * k + A.lp(x, 700) * (1 - k) + 0.6 * np.sin(2 * math.pi * f_of(m) * t)
    return np.tanh(x * 1.3) * A.adsr(n, 0.003, 0.12, 0.7, 0.03) * vel


def _dist_chord(ms, n, drive, cut):
    x = sum(ens(m, n, 0.004, 2) for m in ms) / len(ms)
    x = np.tanh(x * drive)
    x = A.lp(x, cut, 4)
    return A.hp(x, 90)


def r_pchord(root, dur, vel, mute=False):
    n = int((dur + (0.03 if mute else 0.08)) * SR)
    t = tt(n)
    x = _dist_chord([root, root + 7, root + 12], n, 7.0, 1500 if mute else 4200)
    e = np.exp(-t / 0.09) if mute else A.adsr(n, 0.003, 0.2, 0.75, 0.06)
    return x * e * vel * (0.9 if mute else 1.0)


def r_lead(m, dur, vel):
    n = int((dur + 0.1) * SR)
    x = ens(m, n, 0.003, 2, vib=0.012, vib_f=5.2, vib_d=0.15, bend=-1.0, bend_t=0.025)
    x = np.tanh(x * 7)
    x = A.hp(A.lp(x, 3800, 4), 220)
    return x * A.adsr(n, 0.004, 0.1, 0.85, 0.08) * vel


def r_pick(m, vel):
    n = int(0.3 * SR)
    t = tt(n)
    return A.lp(np.tanh(ens(m, n, 0.003, 2) * 4), 2200) * np.exp(-t / 0.1) * vel


# ================================================================== EDM
def e_kick(vel):
    n = int(0.5 * SR)
    t = tt(n)
    f = 54 + 150 * np.exp(-t / 0.035)
    x = 0.75 * np.sin(2 * math.pi * np.cumsum(f) / SR) * np.exp(-t / 0.24) + A.hp(nz(n), 4000) * np.exp(-t / 0.003) * 0.5
    x += np.sin(2 * math.pi * 130 * t) * np.exp(-t / 0.03) * 0.55
    return np.tanh(x * 1.8) * vel


def e_clap(vel, seed=3):
    n = int(0.3 * SR)
    t = tt(n)
    env = sum(np.exp(-np.clip(t - d, 0, None) / 0.008) * (t >= d) for d in (0, 0.011, 0.023)) + np.exp(-t / 0.1) * 0.6
    return A.bp(nz(n, seed), 900, 5500) * env * 0.6 * vel


def e_hat(vel, open_=False):
    n = int((0.3 if open_ else 0.06) * SR)
    t = tt(n)
    return A.hp(nz(n), 8500 if open_ else 10000) * np.exp(-t / (0.1 if open_ else 0.018)) * (0.5 if open_ else 0.3) * vel


def e_bass(m, dur, vel):
    n = int((dur + 0.03) * SR)
    t = tt(n)
    f = f_of(m)
    x = 0.5 * (sawp(ph_of(np.full(n, f * 1.006))) + sawp(ph_of(np.full(n, f * 0.994)) + 0.3))
    x = A.lp(x, 1500, 4) * 1.0 + np.sin(2 * math.pi * f * t) * 0.4
    return np.tanh(x * 1.5) * A.adsr(n, 0.004, 0.08, 0.8, 0.03) * vel


def e_saw(ch, dur, vel):
    name, root, tri, rl = MU.CHORDS[ch]
    n = int((dur + 0.08) * SR)
    x = sum(ens(m, n, 0.011, 7) for m in [tri[0] + 12, tri[1] + 12, tri[2] + 12, tri[0] + 24]) / 4
    x = A.hp(A.lp(x, 6500), 260)
    return x * A.adsr(n, 0.003, 0.16, 0.45, 0.06) * vel


def e_lead(m, dur, vel):
    n = int((dur + 0.08) * SR)
    t = tt(n)
    x = 0.6 * ens(m, n, 0.006, 3) + 0.4 * np.where((ph_of(np.full(n, f_of(m - 12)))) % 1 < 0.5, 1.0, -1.0)
    k = np.exp(-t / 0.12)
    x = A.lp(x, 6000) * k + A.lp(x, 2600) * (1 - k)
    return x * A.adsr(n, 0.003, 0.08, 0.8, 0.06) * vel


def e_pluck(m, vel):
    n = int(0.3 * SR)
    t = tt(n)
    return A.lp(ens(m, n, 0.008, 3), 3000) * np.exp(-t / 0.09) * vel


# ================================================================== JAZZ
def j_ride(vel):
    n = int(1.1 * SR)
    t = tt(n)
    x = sum(a * np.sin(2 * math.pi * f * t + f) * np.exp(-t / d) for f, a, d in
            ((2980, 0.4, 0.7), (4390, 0.3, 0.55), (5870, 0.22, 0.45), (7540, 0.15, 0.35), (9230, 0.1, 0.25)))
    x += A.hp(nz(n), 6000) * np.exp(-t / 0.25) * 0.25 + A.hp(nz(n), 3000) * np.exp(-t / 0.006) * 0.5
    return x * vel


def j_phat(vel):
    n = int(0.08 * SR)
    t = tt(n)
    return (A.hp(nz(n), 6000) * np.exp(-t / 0.018) + A.bp(nz(n), 400, 1200) * np.exp(-t / 0.008) * 0.4) * 0.6 * vel


def j_kick(vel):
    n = int(0.3 * SR)
    t = tt(n)
    return np.sin(2 * math.pi * (58 + 30 * np.exp(-t / 0.02)) * t) * np.exp(-t / 0.13) * 0.8 * vel


def j_brush(vel):
    n = int(0.25 * SR)
    t = tt(n)
    return A.bp(nz(n), 1200, 6000) * np.clip(t / 0.012, 0, 1) * np.exp(-t / 0.09) * 0.7 * vel


def j_ubass(m, dur, vel):
    n = int((dur + 0.08) * SR)
    t = tt(n)
    f = f_of(m) * (1 + 0.012 * np.exp(-t / 0.02))
    ph = ph_of(f)
    x = np.sin(2 * math.pi * ph) + 0.35 * np.sin(4 * math.pi * ph) + 0.15 * np.sin(6 * math.pi * ph)
    x = x * (0.35 + 0.65 * np.exp(-t / 0.25)) + A.lp(nz(n), 350) * np.exp(-t / 0.02) * 0.5
    return x * A.adsr(n, 0.004, 0.05, 1.0, 0.06) * vel


def j_piano_note(m, n, vel):
    t = tt(n)
    f = f_of(m)
    x = np.zeros(n)
    B = 0.0004
    for k in range(1, 9):
        fk = k * f * math.sqrt(1 + B * k * k)
        if fk > 14000:
            break
        a = 1 / k ** 1.25
        d = 1.1 / (1 + 0.6 * k)
        x += a * (np.sin(2 * math.pi * fk * t) + 0.6 * np.sin(2 * math.pi * fk * 1.0012 * t)) * np.exp(-t / d)
    x += A.bp(nz(n), 2000, 5000) * np.exp(-t / 0.005) * 0.15
    return x * np.clip(t / 0.002, 0, 1) * vel / 1.6


def j_piano(ch, dur, vel):
    name, root, tri, rl = MU.CHORDS[ch]
    n = int((dur + 0.18) * SR)
    x = sum(j_piano_note(m, n, 1.0) for m in rl) / 3
    return x * A.adsr(n, 0.001, 0.05, 1.0, 0.16) * vel


def j_sax(m, dur, vel):
    n = int((dur + 0.08) * SR)
    t = tt(n)
    f = f_of(m) * 2 ** (-0.6 * np.exp(-t / 0.04) / 12) * (1 + 0.008 * np.sin(2 * math.pi * 5.0 * t) * np.clip((t - 0.2) / 0.2, 0, 1))
    ph = ph_of(f)
    x = 0.6 * sawp(ph) + 0.4 * np.where(ph % 1 < 0.32, 1.0, -1.0)
    x = A.bp(x, 380, 1700) + A.bp(x, 1700, 3200) * 0.45
    x = np.tanh(x * 2.0) + A.bp(nz(n), 1200, 4000) * 0.05
    return A.lp(x, 4200) * A.adsr(n, 0.025, 0.1, 0.85, 0.07) * vel


def j_keys(m, vel):
    n = int(0.6 * SR)
    return j_piano_note(m, n, vel) * np.exp(-tt(n) / 0.3)


# ================================================================== dispatch
_CACHE = {}


def voice(q, kind, m, ch, dur, vel):
    vel = round(vel * 20) / 20
    key = (kind, m, ch, round(dur, 3), vel)
    if key in _CACHE:
        return _CACHE[key]
    k = kind
    if k == 'timp': x = o_timp(m, vel)
    elif k == 'troll': x = o_timp(m, vel, roll=True)
    elif k == 'ocym': x = o_cym(vel)
    elif k == 'cello': x = o_cello(m, dur, vel)
    elif k == 'vlns': x = o_vlns(ch, dur, vel)
    elif k == 'brass': x = o_brass(ch, dur, vel)
    elif k == 'horn': x = o_horn(m, dur, vel)
    elif k == 'pizz': x = o_pizz(m, vel)
    elif k == 'rkick': x = r_kick(vel)
    elif k == 'rsnare': x = r_snare(vel)
    elif k == 'rhat': x = r_hat(vel)
    elif k == 'crash': x = r_crash(vel)
    elif k == 'rtom': x = r_tom(m, vel)
    elif k == 'rbass': x = r_bass(m, dur, vel)
    elif k == 'pchord': x = r_pchord(m, dur, vel)
    elif k == 'pmute': x = r_pchord(m, dur, vel, mute=True)
    elif k == 'glead': x = r_lead(m, dur, vel)
    elif k == 'gpick': x = r_pick(m, vel)
    elif k == 'ekick': x = e_kick(vel)
    elif k == 'clap': x = e_clap(vel)
    elif k == 'eohat': x = e_hat(vel, True)
    elif k == 'ehat': x = e_hat(vel)
    elif k == 'ebass': x = e_bass(m, dur, vel)
    elif k == 'saw': x = e_saw(ch, dur, vel)
    elif k == 'elead': x = e_lead(m, dur, vel)
    elif k == 'epluck': x = e_pluck(m, vel)
    elif k == 'ride': x = j_ride(vel)
    elif k == 'phat': x = j_phat(vel)
    elif k == 'jkick': x = j_kick(vel)
    elif k == 'brush': x = j_brush(vel)
    elif k == 'ubass': x = j_ubass(m, dur, vel)
    elif k == 'piano': x = j_piano(ch, dur, vel)
    elif k == 'sax': x = j_sax(m, dur, vel)
    elif k == 'keys': x = j_keys(m, vel)
    else:
        raise KeyError(kind)
    if len(_CACHE) < 8000:
        _CACHE[key] = x
    return x


# per genre, per layer: pan and the layer's level inside its own stem (RMS dB of the ungated part, before the
# stems are matched to each other). These set how each genre sounds ALONE.
LAYER_DB = {
    0: dict(drums=-23.0, bass=-21.5, harm=-20.0, lead=-20.5),
    1: dict(drums=-19.5, bass=-21.5, harm=-21.0, lead=-21.0),
    2: dict(drums=-19.5, bass=-22.0, harm=-21.0, lead=-20.5),
    3: dict(drums=-25.0, bass=-20.0, harm=-22.5, lead=-20.0),
}
PAN = {0: dict(drums=0.0, bass=0.15, harm=-0.25, lead=0.22, free=0.0),
       1: dict(drums=0.0, bass=0.0, harm=-0.35, lead=0.3, free=0.0),
       2: dict(drums=0.0, bass=0.0, harm=0.25, lead=-0.15, free=0.0),
       3: dict(drums=0.15, bass=-0.1, harm=0.35, lead=-0.3, free=0.0)}
STEM_DB = -15.0             # every genre stem's RMS (all layers, ungated) before mixing
FREE_DB = -23.0             # the opening's free notes per genre (8 notes/s)
REVERB = {'orch': 0.5, 'rock': 0.14, 'edm': 0.22, 'jazz': 0.3, 'fx': 0.45, 'crowd': 0.3}
BUS_GAIN = {'orch': 1.0, 'rock': 1.0, 'edm': 1.0, 'jazz': 1.0, 'fx': 0.75, 'crowd': 0.5}


def phone(x):
    """what a phone speaker roughly hears: 150 Hz - 8 kHz (levels are matched on this, not raw RMS, so a sub-heavy
    stem can't win the calibration with energy nobody hears)"""
    return A.bp(x, 150, 8000)


def _rms(x):
    return float(np.sqrt((phone(x) ** 2).mean()) + 1e-12)


def calibrate():
    """render each genre's layers ungated over one 8-bar phrase and set their gains (LAYER_DB, then STEM_DB)"""
    n = int(8 * T.BAR * SR) + SR
    cal = {}
    for q in range(4):
        layers = {}
        for L in MU.LAYERS:
            buf = np.zeros(n)
            for bar in range(8):
                for s, m, Ln, kind in MU.part_notes(q, L, bar):
                    t = bar * T.BAR + s * T.S16
                    x = voice(q, kind, m, bar % 4, Ln * T.S16, 0.85)
                    i0 = int(t * SR)
                    k = min(len(x), n - i0)
                    buf[i0:i0 + k] += x[:k]
            buf = buf[:int(8 * T.BAR * SR)]
            layers[L] = buf
            cal[(q, L)] = 10 ** (LAYER_DB[q][L] / 20) / _rms(buf)
        stem = sum(layers[L] * cal[(q, L)] for L in MU.LAYERS)
        k = 10 ** (STEM_DB / 20) / _rms(stem)
        for L in MU.LAYERS:
            cal[(q, L)] *= k
        # the opening's free notes: their own voice, calibrated on a burst of chord tones (8 per second)
        kind = ['pizz', 'gpick', 'epluck', 'keys'][q]
        buf = np.zeros(int(2.4 * SR))
        for k in range(16):
            x = voice(q, kind, [57, 60, 64, 69][k % 4], 0, 0.2, 0.8)
            i0 = int(k * 0.125 * SR)
            m = min(len(x), len(buf) - i0)
            buf[i0:i0 + m] += x[:m]
        cal[(q, 'free')] = 10 ** (FREE_DB / 20) / _rms(buf[:2 * SR])
    return cal


class Buses:
    def __init__(self):
        self.b = {}

    def get(self, name):
        if name not in self.b:
            self.b[name] = np.zeros((NS + 8 * SR, 2))
        return self.b[name]

    def place(self, name, sig, t, gain=1.0, pan=0.0):
        b = self.get(name)
        i0 = int(round((t % T.DUR) * SR))
        a = (pan + 1) * math.pi / 4
        st = np.stack([sig * math.cos(a), sig * math.sin(a)], 1) * gain
        n = min(len(st), len(b) - i0)
        b[i0:i0 + n] += st[:n]

    def fold(self):
        for k, b in self.b.items():
            b[:len(b) - NS] += b[NS:]
            b[NS:] = 0
            self.b[k] = b[:NS]


# ================================================================== the band
def band(B, cal):
    notes = MU.notes()
    LI = {L: i for i, L in enumerate(MU.LAYERS)}
    for n in notes:
        q, L, t = int(n['team']), str(n['layer']), float(n['t'])
        if L == 'free':
            g = 1.0
        else:
            g = MU.gains(min(t, T.DUR - 0.01))[q, LI[L]]
        if g < 0.01:
            continue
        x = voice(q, str(n['kind']), int(n['midi']), int(n['ch']), float(n['dur']), float(n['vel']))
        B.place(GEN[q], x, t, g * cal[(q, L)], PAN[q][L])


def stinger(B, cal, t, q):
    """a lead change: a short hit in the new leader's style on the next beat (the song switches genre)"""
    tb = T.GRID0 + math.ceil((t - T.GRID0) / T.BEAT - 1e-6) * T.BEAT
    bar = int((tb - T.GRID0) // T.BAR)
    ch = bar % 4
    name, root, tri, rl = MU.CHORDS[ch]
    c = lambda L: cal[(q, L)]
    G = GEN[q]
    if q == 0:
        B.place(G, o_timp(root - 12, 1.0), tb, c('drums') * 1.3)
        B.place(G, o_cym(1.0), tb, c('drums') * 1.0)
        B.place(G, o_brass(ch, T.BEAT * 2, 1.0), tb, c('harm') * 1.4, -0.1)
    elif q == 1:
        B.place(G, r_pchord(root - 12, T.BEAT * 2, 1.0), tb, c('harm') * 1.6, -0.2)
        B.place(G, r_crash(1.0), tb, c('drums') * 1.2)
        B.place(G, r_kick(1.0), tb, c('drums') * 1.2)
    elif q == 2:
        n = int(0.45 * SR)
        tq = tt(n)
        zap = sawp(ph_of(1800 * np.exp(-tq / 0.08) + 60)) * np.exp(-tq / 0.2)
        B.place(G, A.lp(zap, 5000), tb - 0.05, c('harm') * 1.2)
        B.place(G, e_kick(1.0), tb, c('drums') * 1.3)
        B.place(G, e_saw(ch, T.BEAT, 1.0), tb, c('harm') * 1.5, 0.2)
    else:
        B.place(G, j_piano(ch, T.BEAT * 2, 1.0), tb, c('harm') * 1.5, 0.3)
        B.place(G, j_ride(1.0), tb, c('drums') * 1.6)
        B.place(G, j_sax(tri[2] + 12 - 12, T.BEAT * 1.5, 1.0), tb, c('lead') * 1.2, -0.3)
    B.place('crowd', crowd_swell(1.4, 'cheer', 5 + int(t)), t, 0.5)


def power_down(B, cal):
    """knocked out: the genre's sound dives in pitch and closes up over 1.1 s; crowd 'ooh', boom"""
    for t0, q in T.OUTS:
        bar = int((t0 - T.GRID0) // T.BAR)
        root = MU.chord(bar)[1]
        n = int(1.1 * SR)
        tq = tt(n)
        m = {0: root, 1: root, 2: root - 12, 3: root - 12}[q]
        f = f_of(m) * 2 ** (-2.5 * (tq / 1.1) ** 1.4)
        ph = ph_of(f)
        x = 0.6 * sawp(ph) + 0.4 * sawp(ph * 1.5)
        if q == 1:
            x = np.tanh(x * 6)
        x = A.lp(x, {0: 2200, 1: 3000, 2: 4000, 3: 1800}[q]) * (1 - tq / 1.1) ** 1.2
        B.place(GEN[q], x, t0, cal[(q, 'harm')] * 1.4)
        B.place('fx', boom(), t0, 0.7)
        B.place('crowd', crowd_swell(1.6, 'ooh'), t0 + 0.05, 0.9)


def boom(dur=1.2):
    n = int(dur * SR)
    t = tt(n)
    f = 38 + 60 * np.exp(-t / 0.08)
    return np.sin(2 * math.pi * np.cumsum(f) / SR) * np.exp(-t / 0.4) + np.sin(2 * math.pi * 150 * t) * np.exp(-t / 0.06) * 0.3


def bell():
    n = int(1.6 * SR)
    t = tt(n)
    return sum(a * np.sin(2 * math.pi * 1180 * r * t) * np.exp(-t / d) for r, a, d in
               ((1, 1.0, 0.6), (2.41, 0.6, 0.35), (3.88, 0.35, 0.2), (5.4, 0.2, 0.12), (0.5, 0.25, 0.8)))


def crowd_swell(dur, kind='cheer', seed=0):
    r = np.random.default_rng(11 + seed)
    n = int(dur * SR)
    t = tt(n)
    x = r.normal(0, 1, n)
    if kind == 'ooh':
        e = np.sin(np.pi * np.clip(t / dur, 0, 1)) ** 1.5
    else:
        e = np.clip(t / 0.25, 0, 1) * np.exp(-np.clip(t - 0.6, 0, None) / (dur * 0.45))
    y = A.bp(x, 300, 3500)
    flutter = 1 + 0.3 * np.sin(2 * math.pi * 7 * t + r.uniform(0, 6)) * np.sin(2 * math.pi * 2.3 * t)
    return A.lp(y, 2400) * e * flutter * 0.5


def crowd_bed(B):
    r = np.random.default_rng(3)
    y = A.bp(r.normal(0, 1, NS), 250, 2200)
    u = tt(NS)
    y = y * (1 + 0.25 * np.sin(2 * math.pi * u / (T.DUR / 20)) * np.sin(2 * math.pi * u / (T.DUR / 9)))
    B.place('crowd', y * 0.06, 0.0, 1.0)


def fx(B):
    B.place('fx', A.whoosh(1.5, 400, 7000), MU.LOCK1 - 1.5, 0.35)        # lock-in riser + hit
    B.place('fx', e_kick(1.0), MU.LOCK1, 0.35)
    B.place('fx', bell(), T.T_BELL, 0.45)
    B.place('fx', bell(), T.T_BELL + 0.22, 0.38)
    B.place('fx', boom(), T.T_BELL, 0.6)
    B.place('fx', A.whoosh(1.0, 300, 6000), T.T_BELL - 1.0, 0.3)
    B.place('crowd', crowd_swell(2.5, 'cheer', 1), T.T_BELL + 0.05, 0.9)
    if len(T.OUTS) >= 2 and T.T_WIN is not None:                        # the duel: a riser into the win
        t2 = T.OUTS[1][0] + 2.0
        dur = T.T_WIN - t2
        if dur > 1.0:
            ri = A.whoosh(dur, 300, 6000) * np.linspace(0.2, 1.0, int(dur * SR)) ** 2
            B.place('fx', ri, t2, 0.5)
    if T.T_WIN is not None:
        n = int(2.4 * SR)
        tq = tt(n)
        hit = A.hp(RNG.normal(0, 1, n), 2500) * np.exp(-tq / 0.6) * 0.6 + boom(2.4)[:n]
        B.place('fx', hit, T.T_WIN, 0.75)
        B.place('crowd', crowd_swell(5.0, 'cheer', 2), T.T_WIN + 0.05, 1.0)
        for k, dt in enumerate(FIREWORK_DT):
            m = int(0.5 * SR)
            pop = A.bp(RNG.normal(0, 1, m), 800, 6000) * np.exp(-tt(m) / 0.05) + A.hp(RNG.normal(0, 1, m), 5000) * np.exp(-tt(m) / 0.25) * 0.3
            B.place('fx', pop, T.T_WIN + dt, 0.3, pan=RNG.uniform(-0.6, 0.6))
    # caption thuds
    th = A.thud()
    for tl, k, nch in T.LANDINGS:
        B.place('fx', th, tl, 0.32 if k == 0 else 0.22)
    # the crack, falling tiles, riser into the opening burst (wraps over the seam), boom on frame 0
    m = int(0.6 * SR)
    crack = A.hp(RNG.normal(0, 1, m), 2000) * np.exp(-tt(m) / 0.03) + sum(np.sin(2 * math.pi * f * tt(m)) * np.exp(-tt(m) / 0.15) * 0.2 for f in (2300, 3700, 5100))
    B.place('fx', crack, T.T_CRACK, 0.55)
    for k in range(70):
        tk = T.T_CRACK + 0.12 + RNG.uniform(0, 0.42)
        mm = int(0.06 * SR)
        tick = np.sin(2 * math.pi * RNG.uniform(1800, 4200) * tt(mm)) * np.exp(-tt(mm) / 0.01)
        B.place('fx', tick, tk, 0.12, pan=RNG.uniform(-0.7, 0.7))
    B.place('fx', seam_riser(1.8), T.T_RESTART - 1.8, RISER_GAIN)
    B.place('crowd', crowd_swell(1.8, 'ooh', 9)[::-1].copy(), T.T_RESTART - 1.8, 0.8)
    B.place('fx', boom(0.9), T.T_RESTART, 0.45)          # wraps to 0.0: the seeds drop on the boom


FIREWORK_DT = [0.35, 1.4, 2.6, 3.9, 5.2, 6.4]
RISER_GAIN = 0.8


def seam_riser(dur):
    """noise + tone sweep that keeps building right up to the loop seam (no fade-out), so the sound at the end of
    the file runs straight into the boom on frame 0"""
    n = int(dur * SR)
    u = np.linspace(0, 1, n, endpoint=False)
    x = np.random.default_rng(61).normal(0, 1, n)
    out = np.zeros(n)
    nb = 24
    for k in range(nb):
        a, b = int(k * n / nb), int((k + 1) * n / nb)
        fc = 250 * (8000 / 250) ** ((k + 0.5) / nb)
        out[a:b] = A.bp(x[max(0, a - 2048):b], fc * 0.75, min(fc * 1.3, SR / 2 - 100))[-(b - a):]
    tone = np.sin(2 * math.pi * np.cumsum(180 * 2 ** (3 * u ** 1.5)) / SR) * 0.35
    snare = np.zeros(n)                              # an accelerating snare roll on top
    tr = 0.0
    while tr < dur:
        i0 = int(tr * SR)
        hit = r_snare(1.0)[:n - i0] * 0.25
        snare[i0:i0 + len(hit)] += hit
        tr += 0.13 * (1 - tr / dur) ** 1.5 + 0.03
    y = (out + tone + snare) * u ** 1.6
    fd = int(0.006 * SR)                             # a 6 ms tail so the seam doesn't click (the boom takes over)
    y[-fd:] *= np.linspace(1, 0, fd) ** 2
    return y


def seg_tape_stop(x, t0, length):
    i0, n = int(t0 * SR), int(length * SR)
    seg = x[i0:i0 + 2 * n].copy()
    rate = np.linspace(1, 0, n) ** 1.3
    pos = np.cumsum(rate)
    out = np.stack([np.interp(pos, np.arange(len(seg)), seg[:, c]) for c in range(2)], 1)
    x[i0:i0 + n] = out * np.linspace(1, 0, n)[:, None] ** 0.6


def level_genres(B, max_db=5.0, win=1.2):
    """per-genre leveler: each genre bus is brought up (never down) toward the level its stage share calls for
    (music.gains x the layer balance), so the loudest genre really is the one with the most stage even when its
    moves land sparsely; smoothed over ~1.5 s, so stutters stay audible as gaps"""
    hop = SR // 50
    nh = NS // hop
    ef = {q: np.array([10 ** (LAYER_DB[q][L] / 10) for L in MU.LAYERS]) for q in range(4)}
    for q in range(4):
        ef[q] = ef[q] / ef[q].sum()
    for q in range(4):
        k = GEN[q]
        x = phone(B.b[k].mean(1))
        e = np.sqrt(uniform_filter1d(x ** 2, int(win * SR), mode='wrap')[::hop][:nh] + 1e-12)
        exp_ = np.zeros(nh)
        for i in range(nh):
            t = i * hop / SR
            g = MU.gains(t)[q]
            exp_[i] = 10 ** (STEM_DB / 20) * math.sqrt(float((ef[q] * g ** 2).sum()))
        c = np.where(exp_ > 1e-4, np.clip(0.85 * exp_ / e, 1.0, 10 ** (max_db / 20)), 1.0)
        c = uniform_filter1d(c, 60, mode='wrap')
        gs = np.interp(np.arange(NS), np.arange(nh) * hop, c)
        B.b[k] *= gs[:, None]


def level_band(B, max_db=3.0, win=1.5):
    """slow leveler on the genre buses (circular): stalled stretches keep their energy; stutters stay audible"""
    band = sum(B.b[k] for k in GEN if k in B.b).mean(1)
    hop = SR // 50
    e = np.sqrt(uniform_filter1d(band ** 2, int(win * SR), mode='wrap')[::hop] + 1e-12)
    ref = np.sqrt((band[int((T.T_BELL + 2) * SR):int((T.T_WIN or 50) * SR)] ** 2).mean())
    g = np.clip(0.8 * ref / e, 1.0, 10 ** (max_db / 20))
    g = uniform_filter1d(g, 25, mode='wrap')
    gs = np.interp(np.arange(NS), np.arange(len(g)) * hop, g)
    for k in GEN:
        if k in B.b:
            B.b[k] *= gs[:, None]


def master(B):
    dry = sum(b * BUS_GAIN.get(k, 1.0) for k, b in B.b.items())
    send = sum(B.b[k] * v * BUS_GAIN.get(k, 1.0) for k, v in REVERB.items() if k in B.b)
    wet = A.reverb(np.concatenate([send, send]), dur=2.2, decay=0.55)[NS:]
    x = A.hp(dry + wet * 0.22, 32)
    e1 = T.OUTS[0][0] if T.OUTS else 31.0
    # the build: the fill sits low, the fight a little higher, a step up at the first knockout (the mix gets
    # louder as bands drop out), the duel and the solo on top
    pts = [(0, -3.0), (MU.LOCK0, -3.5), (MU.LOCK1, -5.5), (T.T_BELL - 0.2, -4.5), (T.T_BELL, -3.0), (e1 - 0.1, -2.0),
           (e1 + 0.4, 0.3), ((T.T_WIN or 52) - 0.1, 0.8), (T.T_WIN or 52, 1.5), (T.T_CRACK, 1.5), (T.DUR, 0.0)]
    g = np.interp(tt(NS), [p[0] for p in pts], [p[1] for p in pts])
    x = x * (10 ** (g / 20))[:, None]
    ref_a, ref_b = (T.T_WIN or 52) + 1.0, T.T_CRACK
    ref = np.sqrt((x[int(ref_a * SR):int(ref_b * SR)] ** 2).mean())
    thr = ref * 10 ** (-6 / 20)
    xl = np.concatenate([x[-SR:], x])
    a = math.exp(-1.0 / (0.08 * SR))
    e = np.sqrt(lfilter([1 - a], [1, -a], (xl ** 2).mean(1)).clip(1e-12))[SR:]
    x = x * np.where(e > thr, (thr / e) ** 0.35, 1.0)[:, None]
    ref = np.sqrt((x[int(ref_a * SR):int(ref_b * SR)] ** 2).mean())
    x = x * (10 ** (TARGET_DB / 20) / ref)
    lim = 0.86
    gl = np.minimum(1.0, lim / np.maximum(np.abs(x).max(1), 1e-9))
    L = int(0.003 * SR)
    gl = uniform_filter1d(minimum_filter1d(gl, 2 * L + 1, mode='wrap'), 2 * L + 1, mode='wrap')
    hop = SR // 1000
    gb = gl[:len(gl) // hop * hop].reshape(-1, hop).min(1)
    rel = math.exp(-1.0 / 60.0)
    y = np.empty_like(gb)
    cur = 1.0
    for _ in range(2):
        for i, v in enumerate(gb):
            cur = v if v < cur else min(v, 1 - (1 - cur) * rel)
            y[i] = cur
    gs = np.interp(np.arange(len(gl)), np.arange(len(y)) * hop + hop // 2, y)
    x = x * np.minimum(gl, gs)[:, None]
    x = np.tanh(x * 1.02) / np.tanh(1.02) * 0.98
    return np.clip(x, -0.95, 0.95)


def solo_previews(cal, bars=8):
    """each genre's stem alone (ungated, 8 bars) -> check/solo_<genre>.wav, to judge every stem on its own"""
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    n = int(bars * T.BAR * SR) + 2 * SR
    for q in range(4):
        buf = np.zeros((n, 2))
        for L in MU.LAYERS:
            for bar in range(bars):
                for s, m, Ln, kind in MU.part_notes(q, L, bar):
                    t = bar * T.BAR + s * T.S16
                    x = voice(q, kind, m, bar % 4, Ln * T.S16, 0.95) * cal[(q, L)]
                    a = (PAN[q][L] + 1) * math.pi / 4
                    i0 = int(t * SR)
                    k = min(len(x), n - i0)
                    buf[i0:i0 + k, 0] += x[:k] * math.cos(a)
                    buf[i0:i0 + k, 1] += x[:k] * math.sin(a)
        send = buf * REVERB[GEN[q]]
        buf = buf + A.reverb(send, dur=2.2, decay=0.55) * 0.22
        buf *= 10 ** (-12 / 20) / np.sqrt((buf ** 2).mean())
        A.write(os.path.join(HERE, 'check', f'solo_{GEN[q]}.wav'), np.clip(np.tanh(buf * 1.02), -0.95, 0.95))


def main():
    cal = calibrate()
    B = Buses()
    band(B, cal)
    for tl, q in T.LEADS[1:]:
        if T.T_WIN is None or tl < T.T_WIN:
            stinger(B, cal, tl, q)
    for k in GEN:                                     # the crack: the winner's solo tape-stops into the new run
        seg_tape_stop(B.get(k), T.T_CRACK, T.T_RESTART - T.T_CRACK)
    power_down(B, cal)
    crowd_bed(B)
    fx(B)
    B.fold()
    level_genres(B)
    level_band(B)
    stems = {k: v.copy() for k, v in B.b.items()}
    out = master(B)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    if '--solos' in sys.argv:
        solo_previews(cal)
    print('section levels (dBFS rms / peak):')
    e = [0, 2.6, MU.LOCK1, 8.13, T.T_BELL, T.T_BELL + 2.4] + [t for t, q in T.OUTS] + [T.T_CRACK, T.DUR]
    e = sorted(set(round(x, 2) for x in e))
    spans = list(zip(e[:-1], e[1:]))
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'  {a:5.2f}-{b:5.2f}s  rms {rms:6.1f}  peak {pk:5.1f}')
    x = out.mean(1)
    w = int(0.5 * SR)
    r = np.array([np.sqrt((x[i:i + w] ** 2).mean()) for i in range(0, NS - w, w)])
    print(f'quietest 0.5 s window: {20 * np.log10(r.min() + 1e-12):.1f} dBFS at {r.argmin() * 0.5:.1f} s; loudest {20 * np.log10(r.max()):.1f}')
    bandsum = sum(stems[k] for k in GEN if k in stems).mean(1)
    bed = (stems['crowd'] * BUS_GAIN['crowd'] + stems['fx'] * BUS_GAIN['fx']).mean(1)
    for a, b in spans:
        sa, sb = int(a * SR), int(b * SR)
        print(f'  band over crowd+fx {a:5.2f}-{b:5.2f}s: {20 * np.log10(np.sqrt((bandsum[sa:sb] ** 2).mean()) / np.sqrt((bed[sa:sb] ** 2).mean() + 1e-12)):+.1f} dB')
    print('genre share of the band (rms dB) per section:')
    for a, b in spans:
        sa, sb = int(a * SR), int(b * SR)
        print(f'  {a:5.2f}-{b:5.2f}s ' + '  '.join(f'{GEN[q]} {20 * np.log10(np.sqrt((stems[GEN[q]][sa:sb] ** 2).mean()) + 1e-12):6.1f}' for q in range(4)))


if __name__ == '__main__':
    main()
