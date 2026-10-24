"""#31 BALLFIGHT #5: Turntables vs Synth - soundtrack v2 (BALLFIGHT v2 redesign; mood: dark breakbeat), all
synthesised, original. v1 (frisbees vs satellites) is kept as audio_v1_backup.py.

The two fighters are the leads, both from the honest run:
  TURNTABLES  THE RING IS THE RECORD: every scratch IS the floor's motion - the record position under the DJ's hand is
              the same curve that jerks the floor in the picture (render.scratch_q), played through a vocal-free stab,
              so what you see and what you hear move together; a thump under each jerk; every rope slam is an organ stab
              in the decks' own Pixel Pitch voice (doubled when leading); REWIND is a backspin with the tape whirring
              backwards; BASS DROP a sub drop that lands with the record. As its HP drops the stabs thin and dirty.
  SYNTH       PATCH CABLES: every voltage pulse is an arpeggio note in the synth's own Pixel Pitch voice, one per cable
              (the first cable plays the arp, the others the notes a third above, quieter); the tone follows the
              waveform on the cables: sine (LV1), square (LV2), saw (LV3); a plug that lands clicks in with an accent, a
              snapped cable cracks, the leash thrums; LFO doubles the pulse rate and a wobble bass joins; SAWTOOTH is a
              supersaw sweep with an accent per cut; WHITE NOISE a noise blast. As its HP drops the arp skips notes.
Bed: F minor at 137.70 BPM (35 bars = 61 s): a chopped, crunchy original breakbeat, a reese bass, cold detuned pads.
The count-in bar drops on FIGHT!; at the K.O. the bed tape-stops with the hit-stop, the loser's part breaks apart, the
crowd roars, the winner's stinger lands, then its solo over the bare break; bar 34 builds into the "3" on sample 0.
Every hit is snapped to the nearest 16th except the scratches (they follow the floor, which is already on the grid).
Tails past the loop fold onto the start.

    python audio.py            -> audio.wav (whole AAC frames) + stems in check/ for the band check"""
import math
import os
import sys
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', '..'))
sys.path.insert(0, HERE)
sys.path.append(os.path.join(HERE, '..', 'ballfight'))
sys.path.append(os.path.join(HERE, '..', 'which_instrument_wins'))
from reelkit import audio as A
from reelkit.ui import impact_landings
import timeline as T
import render as RD
import styles as ST
import pp_bridge as B

SR = A.SR
RUN, EV = RD.RUN, RD.EV
NAMES, WIN, LOSE, T_KO = RD.NAMES, RD.WIN, RD.LOSE, RD.T_KO
KT, KS = NAMES.index('TURNTABLES'), NAMES.index('SYNTH')
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames: loops sample-perfectly
S16 = T.S16
TT = B.module('turntables60_lowpoly')                   # the decks' own scratch motions (read-only use of the series)
KIND = {'baby': TT.BABY, 'chirp': TT.CHIRP, 'trans': TT.TRANS, 'flare': TT.FLARE, 'tear': TT.TEAR, 'scrib': TT.SCRIB,
        'back': TT.BACKSPIN, 'fwd': TT.FWD}


def q16(t):
    return round(t / S16) * S16


def h01(*xs):
    """deterministic hash in [0, 1) (thinning decisions are part of the score, not random each run)"""
    v = 0
    for x in xs:
        v = (v * 1000003 + int(x * 1000) + 7) % 2147483647
    return ((v * 48271) % 2147483647) / 2147483647.0


# ---------------------------------------------------------------- instrument voices (cached, level-matched)
_VC, _GAIN = {}, {}


def _fit(x, cap):
    n = int(cap * SR)
    x = np.asarray(x, float)[:n].copy()
    if len(x) < n:
        x = np.pad(x, (0, n - len(x)))
    fd = min(len(x) // 3, int(0.03 * SR))
    x[-fd:] *= np.linspace(1, 0, fd)
    return x


def inst_gain(name):
    if name not in _GAIN:
        lo, hi = B.entrant(name)[3]
        x = B.voice(name, (lo + hi) // 2, 0.8, 0.2)[:int(0.3 * SR)]
        _GAIN[name] = min(4.0, 0.16 / (np.sqrt((A.hp(x, 120) ** 2).mean()) + 1e-6))
    return _GAIN[name]


def note(name, midi, dur=0.18, cap=0.5, vel=0.85):
    key = (name, int(midi), round(dur, 3), cap, round(vel, 2))
    if key not in _VC:
        _VC[key] = _fit(B.voice(name, int(midi), vel, dur), cap) * inst_gain(name)
    return _VC[key]


def arp_note(m, vel=0.8, dur=0.12):
    """one arpeggiator note in the synth's voice, cut before its built-in echo (which sits off this tempo)"""
    return note('SYNTH', m, dur, 0.34, vel)


def stab(m, vel=0.85, dur=0.12):
    """an organ stab in the decks' voice"""
    return note('TURNTABLES', m, dur, 0.4, vel)


def hp_at(k, t):
    return RD.hp_at(k, t)


def colour(x, k, t, drive_k=1.0):
    """the fighter's tone at time t: thinner + dirtier as its HP drops; returns (signal, gain, lead)"""
    hp = hp_at(k, t) / 100.0
    lead = (hp_at(k, t) - hp_at(1 - k, t)) / 100.0
    cut = 90 + 520 * (1 - hp) ** 1.6
    y = A.hp(x, cut)
    drive = 1 + 4.5 * drive_k * (1 - hp) ** 2
    y = np.tanh(drive * y * 1.2) / np.tanh(drive * 1.2) * (1.0 / (1 + 0.15 * (drive - 1)))
    g = 10 ** (float(np.clip(6.0 * lead, -4.0, 3.5)) / 20)
    return y, g, lead


# ---------------------------------------------------------------- scratches (the decks' motions, a vocal-free stab)
def _stab_src():
    """the 'record' under the needle: a chopped F-minor synth-brass stab, then a held tail (long enough to backspin)"""
    n = int(2.4 * SR)
    t = A.tt(n)
    x = np.zeros(n)
    for m in (53, 56, 60, 65, 68):
        x += A.saw(A.mtof(m), n, 0.004, 3)
    env = np.exp(-t / 0.5) * np.clip(t / 0.004, 0, 1)
    y = A.lp(x * env, 2600) + 0.6 * A.bp(x * np.exp(-t / 0.08), 1500, 5000)
    return y / (np.abs(y).max() + 1e-9)


SRC = _stab_src()
_SCR = {}


def scratch(kind, d, s0=0.06, friction=0.05, seed=0):
    """the record pushed back and forth under the needle (the Pixel Pitch deck's own motion curves, crossfader cuts
    included) playing our stab; backspins start further into the record"""
    key = (kind, round(d, 3), seed)
    if key in _SCR:
        return _SCR[key]
    n = int((d + 0.03) * SR)
    t = A.tt(n)
    q, g = TT.qmotion(KIND[kind], np.minimum(t, d), d)
    if kind == 'back':
        s0 = 1.75
    vel = np.gradient(q, 1 / SR)
    idx = (s0 + q) * SR
    y = np.interp(idx, np.arange(len(SRC)), SRC, left=0.0, right=0.0) * np.clip(np.abs(vel), 0, 1.6) ** 0.7
    y = y + A.bp(np.random.default_rng(seed + 11).normal(0, 1, n), 400, 3000) * np.clip(np.abs(vel), 0, 4) * friction
    from scipy.signal import lfilter
    a = math.exp(-1 / (0.0025 * SR))
    y *= lfilter([1 - a], [1, -a], g)
    fd = int(0.015 * SR)
    y[-fd:] *= np.linspace(1, 0, fd)
    _SCR[key] = y
    return y


def tick(seed=0):
    """a vinyl tick: a ricochet off the rope"""
    n = int(0.03 * SR)
    t = A.tt(n)
    return A.bp(np.random.default_rng(seed).normal(0, 1, n), 2500, 9000) * np.exp(-t / 0.004)


def floor_scratch(t0, d, step=1.0 / SR):
    """the scratch the picture shows: the record position under the hand is render.scratch_q (the floor's own curve),
    sampled at audio rate and played through the stab; backward-first scratches hold further into the record"""
    n = int((d + 0.03) * SR)
    tt_ = t0 + np.arange(n) * step
    q = np.array([RD.scratch_q(x) for x in tt_])
    s0 = 0.03 + max(0.0, -float(q.min()))
    vel = np.gradient(q, step)
    y = np.interp((s0 + q) * SR, np.arange(len(SRC)), SRC, left=0.0, right=0.0) * np.clip(np.abs(vel), 0, 1.6) ** 0.7
    y = y + A.bp(np.random.default_rng(int(t0 * 100)).normal(0, 1, n), 400, 3000) * np.clip(np.abs(vel), 0, 4) * 0.05
    fd = int(0.012 * SR)
    y[-fd:] *= np.linspace(1, 0, fd)
    return y


_LV = {}


def arp_lv(m, lvl, vel=0.8):
    """an arp note in the synth's voice, toned like the waveform on its cables: sine (LV1), square (LV2), saw (LV3)"""
    key = (int(m), int(lvl), round(vel, 2))
    if key not in _LV:
        x = arp_note(m, vel)
        n = len(x)
        f = A.mtof(m)
        env = np.abs(x).max() * A.expenv(n, 0.12, 0.003)
        if lvl <= 1:
            y = A.lp(x, 1100) * 0.8 + np.sin(2 * np.pi * f * A.tt(n)) * env * 0.5
        elif lvl == 2:
            y = x * 0.85 + A.lp(A.square(f, n), 3500) * env * 0.35
        else:
            y = x * 0.85 + A.lp(A.saw(f, n, 0.006, 2), 5000) * env * 0.4
        _LV[key] = y
    return _LV[key]


def jack_click(seed=0):
    """a patch cable plugging in: a metallic click and a tiny hum"""
    n = int(0.08 * SR)
    t = A.tt(n)
    x = A.bp(np.random.default_rng(seed).normal(0, 1, n), 2000, 9000) * np.exp(-t / 0.005)
    x += 0.4 * np.sin(2 * np.pi * 120 * t) * np.exp(-t / 0.03)
    return x


def zip_out(dur=0.16, seed=0):
    """the cable unspooling behind a plug: a short rising zip"""
    n = int(dur * SR)
    t = A.tt(n)
    f = 800 + 3000 * t / dur
    x = np.sign(np.sin(2 * np.pi * np.cumsum(f) / SR)) * 0.3 + A.bp(np.random.default_rng(seed).normal(0, 1, n), 1500, 6000)
    return x * np.sin(np.pi * t / dur) * 0.6


def cable_snap(seed=0):
    """a snapped patch cable: a crack, then a falling electric zap"""
    n = int(0.4 * SR)
    t = A.tt(n)
    crack = A.bp(np.random.default_rng(seed).normal(0, 1, n), 1500, 10000) * np.exp(-t / 0.02)
    f = 1800 * np.exp(-t / 0.08) + 90
    zap = np.sign(np.sin(2 * np.pi * np.cumsum(f) / SR)) * np.exp(-t / 0.12) * 0.5
    return crack + A.lp(zap, 5000)


# ---------------------------------------------------------------- bed synths (original)
def kick(v=1.0):
    return A.kick(1.0, 0.30, 140, 44) * v


_SN = None


def snare(v=1.0):
    global _SN
    if _SN is None:
        a, b = A.snare(0.12) * 0.8, A.clap() * 0.25
        n = max(len(a), len(b))
        _SN = np.pad(a, (0, n - len(a))) + np.pad(b, (0, n - len(b)))
    return _SN * v


def crunch(x, bits=6):
    """the 'sampled break' grit: soft clip + a little bit reduction"""
    y = np.tanh(1.8 * x) / np.tanh(1.8)
    q = 2 ** bits
    return np.round(y * q) / q * 0.6 + y * 0.4


def reese(m, dur, vel=1.0, cut0=320, cut1=900):
    """reese bass: three detuned saws an octave apart from a sub sine, the filter opening and closing over the note"""
    n = int(dur * SR)
    f = A.mtof(m)
    t = A.tt(n)
    x = A.saw(f, n, 0.012, 3) + 0.5 * A.saw(2 * f, n, 0.008, 2)
    out = np.zeros(n)
    nb = 12
    for k in range(nb):                                  # a slow filter swell in blocks
        a, b = k * n // nb, (k + 1) * n // nb
        c = cut0 + (cut1 - cut0) * math.sin(math.pi * (k + 0.5) / nb)
        out[a:b] = A.lp(x[max(0, a - 1024):b], c, 2)[-(b - a):]
    sub = np.sin(2 * np.pi * f * t)
    y = np.tanh(1.4 * (0.55 * out + 0.7 * sub))
    return y * A.adsr(n, 0.008, 0.1, 0.9, min(0.06, dur * 0.3)) * vel


def cold_pad(ms, dur, cut=1100):
    n = int(dur * SR)
    x = sum(A.saw(A.mtof(m), n, 0.006, 3) for m in ms) / len(ms)
    return A.lp(x, cut) * A.adsr(n, 0.35, 0.3, 0.8, min(0.4, dur * 0.3))


def wobble(m, dur, rate_beats=0.5):
    """LFO: a wobble bass, the filter swinging once per half beat"""
    n = int(dur * SR)
    f = A.mtof(m)
    t = A.tt(n)
    x = A.saw(f, n, 0.01, 2)
    lfo = 0.5 - 0.5 * np.cos(2 * np.pi * t / (rate_beats * T.BEAT))
    out = np.zeros(n)
    nb = int(dur / 0.02)
    for k in range(nb):
        a, b = k * n // nb, (k + 1) * n // nb
        out[a:b] = A.lp(x[max(0, a - 512):b], 150 + 1800 * float(lfo[(a + b) // 2]), 2)[-(b - a):]
    return np.tanh(2.0 * out) * A.adsr(n, 0.01, 0.1, 0.9, 0.05)


def supersaw_sweep(dur, m0=53, m1=89):
    n = int(dur * SR)
    t = A.tt(n)
    f = A.mtof(m0) * (A.mtof(m1) / A.mtof(m0)) ** (t / dur)
    ph = np.cumsum(f) / SR
    x = sum(2 * ((ph * (1 + d) + d * 3) % 1.0) - 1 for d in (-0.01, -0.004, 0.0, 0.004, 0.01)) / 5
    return A.hp(x, 300) * np.clip(t / 0.02, 0, 1) * np.exp(-np.maximum(t - dur * 0.6, 0) / (dur * 0.2))


def noise_blast(dur=1.6):
    n = int(dur * SR)
    t = A.tt(n)
    x = np.random.default_rng(21).normal(0, 1, n)
    y = A.hp(x, 1200) * 0.7 + A.bp(x, 300, 3000) * 0.5
    return y * np.clip(t / 0.01, 0, 1) * np.exp(-t / (dur * 0.35)) * (1 + 0.5 * np.sign(np.sin(2 * np.pi * 9 * t)))


def sub_drop(dur=1.2, f0=95, f1=28):
    n = int(dur * SR)
    t = A.tt(n)
    f = f1 + (f0 - f1) * np.exp(-t / 0.25)
    x = np.sin(2 * np.pi * np.cumsum(f) / SR)
    return np.tanh(2.2 * x) * np.exp(-t / (dur * 0.6)) * np.clip(t / 0.002, 0, 1)


def rewind_whirr(dur):
    """tape whirring backwards: a chirpy, falling-then-rising band of noise + a pitched squeal"""
    n = int(dur * SR)
    t = A.tt(n)
    u = t / dur
    x = np.random.default_rng(31).normal(0, 1, n)
    out = np.zeros(n)
    nb = 16
    for k in range(nb):
        a, b = k * n // nb, (k + 1) * n // nb
        fc = 900 + 5000 * (1 - abs(2 * (k + 0.5) / nb - 1))
        out[a:b] = A.bp(x[max(0, a - 1024):b], fc * 0.6, min(fc * 1.5, 20000))[-(b - a):]
    f = 600 + 2400 * (1 - abs(2 * u - 1)) + 300 * np.sin(2 * np.pi * 23 * t)
    sq = np.sin(2 * np.pi * np.cumsum(f) / SR) * 0.4
    return (out * 0.6 + sq) * np.sin(np.pi * np.clip(u, 0, 1)) ** 0.5


def riser(dur, over=0.012, seed=11):
    """a noise + saw riser that builds right up to (and `over` s past) its end: no fade, so the loop seam stays loud"""
    n = int((dur + over) * SR)
    t = A.tt(n)
    u = np.clip(t / dur, 0, 1.05)
    x = np.random.default_rng(seed).normal(0, 1, n)
    f = 300 * (20 ** u)
    ph = 2 * np.pi * np.cumsum(55 * 2 ** (u * 2.5)) / SR
    sweep = (2 * ((ph / (2 * np.pi)) % 1) - 1)
    out = np.zeros(n)
    nb = 20
    for k in range(nb):
        a, b = int(k * n / nb), int((k + 1) * n / nb)
        fc = float(f[min(n - 1, (a + b) // 2)])
        out[a:b] = A.bp(x[max(0, a - 2048):b], fc * 0.7, min(fc * 1.4, SR / 2 - 200))[-(b - a):]
    return (out + 0.35 * A.lp(sweep, 2500)) * u ** 2


def roar(dur, seed=3):
    n = int(dur * SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    x = A.bp(x, 300, 2600) * 0.5 + A.bp(x, 900, 5000) * 0.25
    t = A.tt(n)
    env = np.clip(t / 0.25, 0, 1) * np.exp(-np.maximum(t - 0.6, 0) / (dur * 0.45))
    return x * env


# ---------------------------------------------------------------- the bed
BASS_ROOT = {'Fm': 29, 'Db': 25, 'Bbm': 34, 'C': 24}
PAD = {'Fm': (53, 56, 60, 63), 'Db': (49, 53, 56, 60), 'Bbm': (49, 53, 58, 61), 'C': (48, 52, 55, 58)}
# the break: (16th, piece, vel); B = the chop variant on every 4th bar (a stutter into the next bar)
BREAK_A = [(0, 'K', 1.0), (2, 'H', .5), (4, 'S', 1.0), (6, 'H', .5), (7, 'G', .35), (8, 'H', .55), (10, 'K', .95),
           (11, 'K', .6), (12, 'S', 1.0), (14, 'H', .5), (15, 'G', .4)]
BREAK_B = [(0, 'K', 1.0), (2, 'H', .5), (4, 'S', 1.0), (6, 'K', .7), (7, 'G', .35), (8, 'H', .55), (10, 'K', .95),
           (12, 'S', 1.0), (13, 'S', .5), (14, 'S', .7), (15, 'S', .85)]


def break_bar(mix, b, k=1.0, light=False):
    t0 = b * T.BAR
    pat = BREAK_B if b % 4 == 3 and not light else BREAK_A
    for st, pc, v in pat:
        tt_ = t0 + st * S16 + (0.012 if st % 2 else 0.0)            # a touch of swing on the off 16ths
        if pc == 'K':
            mix.place('break', crunch(kick(v)), tt_, 0.55 * k)
        elif pc == 'S':
            mix.place('break', crunch(snare(v)), tt_, 0.42 * k, pan=0.05)
        elif pc == 'G':
            mix.place('break', crunch(snare(v) * 0.6), tt_, 0.25 * k, pan=-0.1)
        else:
            mix.place('break', A.hat(0.03) * v, tt_, 0.22 * k, pan=0.25)
        if pc == 'H' and st in (2, 10) and not light:
            mix.place('break', A.hat(0.03) * 0.3, tt_ + S16, 0.16 * k, pan=0.25)


def bed(mix, b, intensity=1.0, light=False):
    t0 = b * T.BAR
    ch = T.CHORDS[b]
    r = BASS_ROOT[ch]
    break_bar(mix, b, 1.0, light)
    # reese: the root held over the bar with a push on the "and" of 3, an octave jump into the next bar later on
    mix.place('bass', reese(r, 2.5 * T.BEAT, 1.0), t0, 0.40)
    mix.place('bass', reese(r, 1.0 * T.BEAT, 0.8, 400, 1300), t0 + 2.5 * T.BEAT, 0.34)
    if intensity > 1.3 and not light:
        mix.place('bass', reese(r + 12, 0.45 * T.BEAT, 0.7, 500, 1500), t0 + 3.5 * T.BEAT, 0.26)
    if not light:
        mix.place('pad', cold_pad(PAD[ch], T.BAR + 0.1), t0, 0.07 + 0.03 * min(1.0, intensity - 1.0))


def main():
    mix = A.Mix(T.DUR + 4.0)
    # ------------------------------------------------ count-in (bar 0, on the dominant): a chirp + a synth stab per count
    for j, (tc, txt) in enumerate(T.COUNT[:3]):
        tc = max(tc, 0.004)                                # the "3" 4 ms into the file: no click at the seam
        mix.place('decks', scratch('chirp', 0.22, seed=j), tc, 0.30, pan=-0.2)
        for m in (60, 64, 67):
            mix.place('synth', arp_note(m, 0.9, 0.18), tc, 0.20, pan=0.2)
        mix.place('bass', reese(24, 0.36, 0.9), tc, 0.40)
        mix.place('break', crunch(kick(0.9)), tc, 0.45)
    for st, v in ((12, 0.45), (13, 0.6), (14, 0.75), (15, 0.9)):   # the pickup into FIGHT!
        mix.place('break', crunch(snare(v)), st * S16, 0.36)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), 3 * T.BEAT, 0.14)
    mix.place('fx', A.crash(2.0), T.BAR, 0.18)
    # ------------------------------------------------ the bed: bars 1 .. the K.O.
    bar_ko = int(T_KO // T.BAR)
    for b in range(1, bar_ko + 1):
        bed(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
    mix.place('fx', A.whoosh(3.6, 300, 6000), 44.0 - 3.6, 0.10)
    for k in ('break', 'bass', 'pad'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ TURNTABLES: the scratch IS the floor's motion; rope slams = stabs
    for e in EV:
        if e[1] == 'scratch' and e[0] <= T_KO + 0.01:
            t0, d, stg, sgn = e[0], e[3], e[4], e[5]
            sig, g, lead = colour(floor_scratch(t0, d), KT, t0, 0.8)
            mix.place('decks', sig, t0, 0.34 * g, pan=-0.2)
        elif e[1] == 'jerk' and e[0] <= T_KO + 0.01:
            mix.place('fx', A.thud(0.18, 110, 50, seed=int(e[0] * 13)), e[0], 0.07 * min(1.4, e[6] / 1.3))
    slots = set()
    for e in EV:
        if e[0] > T_KO + 0.01:
            break
        if e[1] == 'hit' and e[2] == KT and e[7] == 'scratch':      # a rope slam: an organ stab on the chord
            tq = q16(e[0])
            s_ = round(tq / S16)
            if s_ in slots:
                continue
            slots.add(s_)
            ch = T.chord_at(tq)
            tones = [m for m in T.scale_notes(60, 79, ch) if m % 12 in T.CHORD_PCS[ch]]
            pan = float(np.clip(e[5] / 3.5, -0.6, 0.6))
            for j, m in enumerate(tones[:3]):
                sig, g, lead = colour(stab(m, 0.95), KT, e[0], 1.0)
                mix.place('decks', sig, tq, 0.16 * g, pan=pan)
            if lead > 0.04:                                         # the leader: the root doubled an octave down
                mix.place('decks', colour(stab(tones[0] - 12, 0.8), KT, e[0])[0], tq, 0.12 * g, pan=pan)
            mix.place('fx', A.thud(0.3, 130, 55, seed=int(e[0] * 7)), tq, 0.14)
            mix.place('fx', tick(int(e[0] * 100) % 50), tq, 0.08, pan=pan)
    # ------------------------------------------------ SYNTH: a voltage pulse per arp note, the cable waveform's tone
    by_tick = {}
    for e in EV:
        if e[1] == 'volt' and e[0] <= T_KO + 0.01:
            by_tick.setdefault(round(q16(e[0]) / S16), []).append(e)
    for slot, evs in sorted(by_tick.items()):
        tq = slot * S16
        hp = hp_at(KS, tq) / 100.0
        if h01(tq, 3) < 0.55 * (1 - hp) ** 1.6:                    # skips notes when hurt
            continue
        n = evs[0][4]
        ch = T.chord_at(tq)
        lvl = int(RUN['LV'][RD.sim_i(tq), KS])
        for j, e in enumerate(evs):
            m = T.ARP[ch][(n + 2 * j) % 8]
            sig, g, lead = colour(arp_lv(m, lvl, 0.8 if j == 0 else 0.6), KS, tq, 1.0)
            mix.place('synth', sig, tq, (0.22 if j == 0 else 0.10) * g, pan=0.2 + 0.15 * math.sin(j * 1.7))
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'hit' and e[2] == KS:
            tq = q16(t)
            how = e[7]
            pan = float(np.clip(e[5] / 3.5, -0.6, 0.6))
            ch = T.chord_at(tq)
            lvl = int(RUN['LV'][RD.sim_i(tq), KS])
            if how == 'plug':                                      # a plug lands: a jack click + an accent
                mix.place('fx', jack_click(int(t * 31)), t, 0.16, pan=pan)
                mix.place('synth', colour(arp_lv(T.ARP[ch][0] + 12, lvl, 1.0), KS, t)[0], tq, 0.22, pan=pan)
            elif how == 'lash':                                    # a SAWTOOTH cut
                m = T.ARP[ch][int(e[8]) % 8] + 12
                mix.place('synth', colour(arp_lv(m, 3, 1.0), KS, t)[0], tq, 0.26, pan=pan)
                mix.place('fx', A.thud(0.2, 160, 60, seed=int(t * 7)), tq, 0.10)
            elif how == 'wave':
                mix.place('fx', A.thud(0.45, 110, 40), tq, 0.36)
        elif kind == 'plug_fire' and e[2] == KS:                   # the cable unspools
            mix.place('fx', zip_out(0.16, int(t * 17)), t, 0.05, pan=0.2)
        elif kind == 'snap':
            mix.place('fx', cable_snap(int(t * 23)), t, 0.30, pan=float(np.clip(e[4] / 3.5, -0.6, 0.6)))
        elif kind == 'leash':
            mix.place('fx', A.thud(0.14, 75, 45, seed=int(t * 9)), t, 0.035, pan=float(np.clip(e[3] / 3.5, -0.6, 0.6)))
        elif kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.06 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            mix.place('fx', A.chime(91, 0.25), t, 0.05)
        elif kind == 'ability':
            who, name = e[2], e[3]
            tq = q16(t)
            if name == 'SCRATCH':
                mix.place('decks', scratch('scrib', 0.4, seed=3), tq, 0.20, pan=-0.2)
            elif name == 'REWIND':
                d = T.BEAT * RD.CFG[who]['abilities']['unlock2']['zip_beats']
                mix.place('decks', scratch('back', round(d + 0.1, 3), seed=7), t, 0.40, pan=-0.1)
                mix.place('fx', rewind_whirr(d + 0.1), t, 0.20)
            elif name == 'BASS DROP':
                mix.place('fx', A.whoosh(2 * T.BEAT, 6000, 300, rise=False), tq, 0.12)
            elif name == 'LFO':
                dur = RD.CFG[who]['abilities']['unlock1']['dur']
                mix.place('bass', wobble(BASS_ROOT[T.chord_at(tq)] + 12, dur), tq, 0.20)
            elif name == 'SAWTOOTH':
                mix.place('synth', supersaw_sweep(T.BEAT * 1.0), tq, 0.16)
            elif name == 'WHITE NOISE':
                mix.place('fx', noise_blast(1.6), tq, 0.20)
                mix.place('synth', colour(arp_note(89, 1.0, 0.3), KS, t, 2.0)[0], tq, 0.16)
        elif kind == 'slam':
            ts = e[0]
            mix.place('bass', sub_drop(1.3), ts, 0.62)
            mix.place('fx', A.thud(0.6, 90, 32), ts, 0.50)
            mix.place('decks', stab(53, 1.0, 0.3), ts, 0.24)
            mix.place('fx', A.crash(1.5), ts, 0.10)
    # ------------------------------------------------ the K.O. moment
    mix.place('fx', A.crash(2.0), T_KO, 0.22)
    mix.place('bass', sub_drop(1.4, 80, 26), T_KO, 0.62)
    mix.place('fx', A.thud(0.9, 120, 30), T_KO, 0.7)
    loser = NAMES[LOSE]
    if loser == 'SYNTH':
        for j, m in enumerate((84, 79, 75, 72, 68, 63, 60, 56)):          # the arp falls apart, note by note
            x = np.tanh(3.0 * arp_note(m, 0.9, 0.1))
            mix.place('synth', x, RD.T_HS + 0.07 * j, 0.20 * (1 - j / 10), pan=(j - 3.5) / 8)
    else:
        mix.place('decks', A.tape_stop(np.stack([scratch('baby', 1.2, seed=9)] * 2, 1), 0.0, 1.0)[:, 0], RD.T_HS, 0.3)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.16)
    # the winner's stinger on "(WINNER) WINS"
    tw = q16(T_KO + 1.3)
    if NAMES[WIN] == 'TURNTABLES':
        mix.place('solo', scratch('back', 0.6, seed=2), tw - 0.6, 0.30)
        for m in (53, 60, 65, 68):
            mix.place('solo', stab(m, 1.0, 0.4), tw, 0.20)
    else:
        for m in (65, 72, 77, 80):
            mix.place('solo', arp_note(m, 1.0, 0.4), tw, 0.20)
    mix.place('bass', reese(29, 1.2), tw, 0.5)
    mix.place('fx', A.crash(1.8), tw, 0.14)
    solo0 = RD.T_SOLO
    mix.place('fx', A.whoosh(solo0 - tw - 0.1, 300, 5000), tw + 0.1, 0.10)
    # ------------------------------------------------ victory: the bare break + bass, and the winner's solo
    b_solo = int(round(solo0 / T.BAR))
    for b in range(b_solo, 34):
        bed(mix, b, light=True)
    if NAMES[WIN] == 'TURNTABLES':
        for bt, kind, ln in T.SOLO['TURNTABLES']:
            ts = solo0 + bt * T.BEAT
            if kind == 'stab':
                ch = T.chord_at(ts)
                for m in sorted(T.scale_notes(60, 72, ch), key=lambda x: (x % 12 not in T.CHORD_PCS[ch], x))[:3]:
                    mix.place('solo', stab(m, 0.95, min(0.3, ln * T.BEAT)), ts, 0.16)
            else:
                mix.place('solo', scratch(kind, round(ln * T.BEAT * 0.95, 3), seed=int(bt * 4)), ts, 0.36,
                          pan=-0.15 + 0.3 * h01(bt))
    else:
        for bt, m, d in T.SOLO['SYNTH']:
            mix.place('solo', arp_note(m, 0.95, max(0.1, d * T.BEAT * 0.9)), solo0 + bt * T.BEAT, 0.34)
    # the loser reassembles: its pieces fly home as a rising run, a glass "pop" when it's whole
    run = [65, 68, 72, 75, 77, 80, 84, 89]
    for j, m in enumerate(run):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
        if loser == 'SYNTH':
            mix.place('synth', arp_note(m, 0.7), tj, 0.18)
        else:
            mix.place('decks', stab(m - 12, 0.7), tj, 0.16)
    mix.place('fx', A.chime(95, 0.5) + A.chime(102, 0.5) * 0.5, T.T_GLIDE, 0.12)
    # ------------------------------------------------ bar 34: build into the "3" on sample 0
    t34 = 34 * T.BAR
    for st in range(16):
        v = 0.3 + 0.7 * st / 15
        mix.place('break', crunch(snare(v)), t34 + st * S16, 0.30 * (0.5 + 0.5 * v))
        if st % 2:
            mix.place('break', crunch(snare(v * 0.6)), t34 + (st + 0.5) * S16, 0.18 * v)
    mix.place('fx', riser(T.DUR - t34), t34, 0.22)
    for bt in range(4):
        mix.place('bass', reese((24, 26, 28, 31)[bt], 0.9 * T.BEAT, 0.9), t34 + bt * T.BEAT, 0.40)
    mix.place('pad', cold_pad((48, 52, 55, 58, 64), T.DUR - t34 + 0.02, 1600), t34, 0.08)
    for st in (0, 8, 12, 14):
        mix.place('break', crunch(kick(0.9)), t34 + st * S16, 0.45)
    # caption landings: soft thuds
    for tl, k, nch in impact_landings(RD.CAPTIONS):
        mix.place('fx', A.thud(), tl, 0.08 if k == 0 else 0.06)
    out, stems = master(mix)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    for k, s in stems.items():
        A.write(os.path.join(HERE, 'check', f'stem_{k}.wav'), s)
    spans = [(0, T.BAR), (T.BAR, 8), (8, 20), (20, 32), (32, 44), (44, T_KO), (T_KO, solo0), (solo0, 58), (58, 61)]
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


BUS_GAIN = {'break': 1.6, 'decks': 3.2, 'synth': 2.0, 'solo': 1.7, 'fx': 1.4, 'bass': 0.9, 'pad': 0.9}
# section automation (dB), applied AFTER the compressor so the limiter can't flatten the build: the count-in at full,
# the early fight pulled back, rising through the final exchange, the victory section loud, the end close to the start
GAIN_PTS = [(0, 0.0), (1.55, 0.0), (2.1, -2.6), (20, -2.3), (30, -0.9), (40, 1.3), (44, 2.3), (50.6, 3.3), (52.2, 2.8),
            (53.5, 3.3), (58.0, 3.3), (60.0, 1.6), (61.02, 0.6)]


def master(mix, thr=0.22, ceiling=0.89):
    """Mix.master with the overflow past the loop (tails, reverb) folded onto the start; NA samples (whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('synth', 0.5), ('solo', 0.45), ('fx', 0.4), ('decks', 0.3),
                                                ('pad', 0.6), ('break', 0.1)))
    wet = A.reverb(send) * 0.20
    full = sum(buses.values()) + wet

    def fold(x):
        y = x[:NS].copy()
        tail = x[NS:]
        y[:len(tail)] += tail[:NS]
        return y
    m = fold(full)
    from scipy.signal import lfilter
    env = np.sqrt(lfilter([0.002], [1, -0.998], (np.concatenate([m[-SR:], m]) ** 2).mean(1)))[SR:]
    gain = np.where(env > thr, (thr / np.maximum(env, 1e-9)) ** 0.35, 1.0)
    g = np.interp(np.arange(NS) / SR, [p[0] for p in GAIN_PTS], [p[1] for p in GAIN_PTS])
    gl = (10 ** (g / 20))
    gain = (gain * gl)[:, None]
    m = m * gain
    s1 = ceiling / (np.percentile(np.abs(m), 99.97) + 1e-9)     # the rare transient above it is soft-clipped
    m = np.tanh(m * s1 * 1.1) / np.tanh(1.1)
    s2 = ceiling / (np.abs(m).max() + 1e-9)
    m *= s2
    stems = {}
    for k in buses:
        stems[k] = fold(buses[k]) * gain * s1 * s2
    return m, stems


if __name__ == '__main__':
    main()
