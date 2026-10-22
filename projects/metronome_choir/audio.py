"""The Metronome Choir - soundtrack. Everything synthesised; the song is original (D major, 120 BPM).

The physics drives the sound:
- every tick in sim.npz rings that metronome's bell at its exact time: each section has a two-note pair
  (left end / right end of the swing), so an unsynced riser is a short, quiet bell-tick wash (~400 ticks/s with
  all 200) and a locked riser rings its two-note line out of it, louder because 40 bells now land together;
  the 5 stacked lines alternate D major (left ticks) and Asus2 (right ticks): the choir's harmony;
- the soloist carries a bell carousel that steps to the next melody note on every one of ITS ticks, so before it
  locks the melody drags against the beat; the carousel is set so slot 0 rings on the drop;
- the kick enters on the choir's own pulse (crowd beats from the whole-choir mean phase) as the risers pull
  together; the song from 38 s is locked to the ticks (left ticks on whole seconds = 120 BPM);
- the riser lifts, can rolls, clunks, the platform sway rumble and the final thump follow motion.py.
Loop: everything past 60 s is folded onto the start (the post-jolt chaos continues straight into 0 s).

    python audio.py      -> audio.wav, stems/sci.wav, stems/music.wav; prints levels and the bell-band check"""
import math
import os
import sys
import numpy as np
from scipy.io import wavfile
from scipy.signal import fftconvolve, lfilter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit import audio as A
from reelkit.audio import SR, tt, mtof, lp, hp, bp, expenv, adsr, saw
import timeline as T
import layout as LY
import motion as MO

rng = np.random.default_rng(16)
DUR = T.DUR
EXTRA = 6.0
NS = int(DUR * SR)
NT = int((DUR + EXTRA) * SR)
SIM = MO.SIM
TICK_T, TICK_I, TICK_R = SIM['tick_t'], SIM['tick_i'], SIM['tick_right']
G = T.grid


# ------------------------------------------------------------------ instruments
def bell(m, dur=1.6, short=False):
    """tuned bell bar (vibraphone-like partials 1, 2, 3, 4 with a hint of 2.76) so 200 of them stack into clean chords"""
    n = int(dur * SR)
    t = tt(n)
    f = mtof(m)
    k = (1.15 - 0.4 * (m - 50) / 30) * (0.16 if short else 1.0)
    x = (np.sin(2 * np.pi * f * t) * np.exp(-t / (0.62 * k))
         + 0.32 * np.sin(2 * np.pi * 2.0 * f * t) * np.exp(-t / (0.3 * k))
         + 0.14 * np.sin(2 * np.pi * 3.0 * f * t) * np.exp(-t / (0.2 * k))
         + 0.24 * np.sin(2 * np.pi * 4.0 * f * t) * np.exp(-t / (0.09 * k))
         + 0.07 * np.sin(2 * np.pi * 2.76 * f * t) * np.exp(-t / (0.12 * k)))
    if m < 56:
        x += 0.3 * np.sin(2 * np.pi * 3.0 * f * t) * np.exp(-t / (0.25 * k))      # low bells: an audible upper partial
    x += hp(np.random.default_rng(m).normal(0, 1, n), 3000) * np.exp(-t / 0.0015) * 0.08    # mallet tick
    x *= np.clip(t / 0.0012, 0, 1)
    return x / np.abs(x).max()


def glock(m, dur=1.4):
    """the soloist's carousel: a bright glockenspiel bar"""
    n = int(dur * SR)
    t = tt(n)
    f = mtof(m)
    x = (np.sin(2 * np.pi * f * t) * np.exp(-t / 0.55) + 0.28 * np.sin(2 * np.pi * 2.71 * f * t) * np.exp(-t / 0.12)
         + 0.1 * np.sin(2 * np.pi * 5.15 * f * t) * np.exp(-t / 0.04))
    x += hp(np.random.default_rng(m).normal(0, 1, n), 4000) * np.exp(-t / 0.002) * 0.15
    x *= np.clip(t / 0.0008, 0, 1)
    return x / np.abs(x).max()


def click():
    n = int(0.03 * SR)
    t = tt(n)
    x = 0.8 * np.sin(2 * np.pi * 2350 * t) * np.exp(-t / 0.004) + hp(np.random.default_rng(3).normal(0, 1, n), 2500) * np.exp(-t / 0.0015)
    return x / np.abs(x).max()


def kick(dec=0.3, f0=140, f1=46):
    n = int(0.5 * SR)
    t = tt(n)
    f = f1 + (f0 - f1) * np.exp(-t / 0.035)
    s = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / dec)
    s += hp(np.random.default_rng(1).normal(0, 1, n), 3000) * np.exp(-t / 0.004) * 0.3
    return np.tanh(s * 1.5) / np.tanh(1.5)


def sub_boom(dur=1.6, f0=60, f1=32):
    n = int(dur * SR)
    t = tt(n)
    f = f1 + (f0 - f1) * np.exp(-t / 0.15)
    return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / (dur * 0.35))


def thud(f0=120, f1=55, dec=0.18):
    return kick(dec, f0, f1)


def knock(f=420, dec=0.05):
    n = int(0.2 * SR)
    t = tt(n)
    x = np.sin(2 * np.pi * f * t) * np.exp(-t / dec) + 0.5 * np.sin(2 * np.pi * f * 2.3 * t) * np.exp(-t / (dec * 0.5))
    x += bp(np.random.default_rng(int(f)).normal(0, 1, n), 800, 4000) * np.exp(-t / 0.006) * 0.6
    return x / np.abs(x).max()


def clink(f):
    n = int(0.5 * SR)
    t = tt(n)
    x = sum(np.sin(2 * np.pi * f * r * t) * np.exp(-t / d) for r, d in ((1, 0.18), (2.32, 0.09), (4.25, 0.05), (6.1, 0.03)))
    return x / np.abs(x).max()


def roll_noise(dur, cut=220, rate=13.0, seed=0):
    n = int(dur * SR)
    x = lp(np.random.default_rng(seed).normal(0, 1, n), cut, 4) * (0.6 + 0.4 * np.sin(2 * np.pi * rate * tt(n)))
    return x * np.sin(np.linspace(0, np.pi, n)) / (np.abs(x).max() + 1e-9)


def bass_note(m, dur, cut=600):
    n = int(dur * SR)
    t = tt(n)
    f = mtof(m)
    x = 0.9 * np.sin(2 * np.pi * f * t) + 0.35 * lp(saw(f, n, det=0.004, voices=2), cut)
    return x * adsr(n, 0.003, 0.09, 0.7, 0.03)


def pad(ms, dur, cut=900, att=0.5):
    n = int(dur * SR)
    x = sum(saw(mtof(m), n, det=0.006, voices=3) for m in ms) / max(1, len(ms))
    return lp(x, cut) * adsr(n, att, 0.3, 0.85, min(0.8, dur * 0.4))


def stab(ms, dur=0.22):
    """offbeat chord stab (soft organ-pluck)"""
    n = int(dur * SR)
    t = tt(n)
    x = sum(np.sin(2 * np.pi * mtof(m) * t) + 0.35 * np.sin(4 * np.pi * mtof(m) * t) * np.exp(-t / 0.05) for m in ms)
    return lp(x / len(ms), 2400) * expenv(n, 0.09, 0.003)


def lead(m, dur):
    n = int(dur * SR)
    t = tt(n)
    f = mtof(m) * (1 + 0.005 * np.sin(2 * np.pi * 5.2 * t) * np.clip((t - 0.12) / 0.2, 0, 1))
    ph = 2 * np.pi * np.cumsum(f) / SR
    x = np.sin(ph) + 0.22 * np.sin(2 * ph) + 0.1 * np.sin(3 * ph)
    return lp(x, 2200) * adsr(n, 0.01, 0.12, 0.7, min(0.15, dur * 0.3))


def pluck(m, dur=0.35):
    n = int(dur * SR)
    t = tt(n)
    f = mtof(m)
    return (np.sin(2 * np.pi * f * t) + 0.3 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.05)) * expenv(n, 0.1, 0.002)


def riser(dur):
    n = int(dur * SR)
    t = tt(n)
    sweep = np.sin(2 * np.pi * np.cumsum(160 * (8.0 ** (t / dur))) / SR) * (t / dur) ** 2 * 0.3
    return A.whoosh(dur, 300, 7000, rise=True) * np.linspace(0.2, 1, n) ** 1.6 + sweep


CLAP = hp(A.clap(), 900)
HAT = A.hat()
OHAT = A.hat(open_=True)
SNARE = hp(A.snare(), 250)
CRASH = A.crash(2.4)
SHAKER = hp(np.random.default_rng(11).normal(0, 1, int(0.07 * SR)), 6000) * np.exp(-tt(int(0.07 * SR)) / 0.016) * 0.4


class Mix2(A.Mix):
    def __init__(self):
        super().__init__(DUR + EXTRA)


mix = Mix2()


def train_place(bus, sig, times, gains, pans):
    """sum many copies of sig at fractional times with gains/pans via impulse trains + one FFT convolution"""
    if not len(times):
        return
    L = np.zeros(NT + 2)
    R = np.zeros(NT + 2)
    a = (np.asarray(pans) + 1) * math.pi / 4
    gl, gr = np.asarray(gains) * np.cos(a), np.asarray(gains) * np.sin(a)
    pos = np.asarray(times) * SR
    i0 = np.floor(pos).astype(int)
    fr = pos - i0
    ok = (i0 >= 0) & (i0 < NT)
    for arr, g in ((L, gl), (R, gr)):
        np.add.at(arr, i0[ok], g[ok] * (1 - fr[ok]))
        np.add.at(arr, i0[ok] + 1, g[ok] * fr[ok])
    out = np.stack([fftconvolve(L[:NT], sig)[:NT], fftconvolve(R[:NT], sig)[:NT]], 1)
    b = mix.bus(bus)
    b[:NT] += out


# ------------------------------------------------------------------ 1) the bells (straight from the simulation)
def sec_R_at(times):
    f = np.clip(np.round(times * T.FPS).astype(int), 0, len(MO.ZSEC) - 1)
    return np.abs(MO.ZSEC[f])


def bells():
    keep = TICK_T < DUR
    tt_, ti, tr = TICK_T[keep], TICK_I[keep], TICK_R[keep]
    risers = ti < LY.NR
    sec = LY.SEC_ALL[ti]
    G_LOCK = [0.62, 0.46, 0.40, 0.36, 0.34]          # per-section coherent level (low bells need more)
    G_CHAOS = 0.075
    # lock weight per tick: the section's order parameter at that moment
    Rk = sec_R_at(tt_)
    w = np.zeros(len(tt_))
    w[risers] = np.clip((Rk[risers, sec[risers]] - 0.55) / 0.4, 0, 1) ** 1.5
    pan = np.where(risers, np.clip(LY.LX[np.minimum(ti, LY.NR - 1)] / 6.0, -0.75, 0.75), 0.0)
    nsum = 0
    for k in range(T.NSEC):
        nk = len(LY.MEMBERS[k])
        for side, m in ((False, T.SEC_NOTES[k][0]), (True, T.SEC_NOTES[k][1])):
            sel = risers & (sec == k) & (tr == side)
            train_place('bells', bell(m), tt_[sel], w[sel] * G_LOCK[k] / nk, pan[sel])
            train_place('bells', bell(m, 0.5, short=True), tt_[sel], (1 - w[sel]) * G_CHAOS, pan[sel])
            nsum += sel.sum()
    # mechanical ticks: a crisp click per tick (a rattle in the chaos, a sharp tick once locked)
    sel = risers
    train_place('ticks', click(), tt_[sel], (w[sel] * 1.0 / 40 + (1 - w[sel]) * 0.07), pan[sel])
    return nsum


def soloist():
    s_ticks = TICK_T[(TICK_I == LY.SOLO)]
    m_drop = int(np.argmin(np.abs(s_ticks - T.T_DROP)))
    notes = {}
    for m, t in enumerate(s_ticks):
        if t >= DUR:
            continue
        slot = (m - m_drop) % len(T.MELODY)
        note = T.MELODY[slot]
        if t < 33.8:
            g = 0.11                                        # buried in the wash
        elif t < T.T_DROP - 0.05:
            g = 0.85                                        # the spotlight: the pickup drags against the beat
        elif t < T.T_JOLT:
            g = 0.9
            b = round((t - T.T_DROP) / T.BEAT)
            if abs(t - G(76 + b)) < 0.03:
                t = G(76 + b)                               # locked: within a few ms of the grid; sit exactly on it
        else:
            g = 0.16
        mix.place('ticks', click(), t, 0.05 if t < 33.8 or t >= T.T_JOLT else 0.12)
        if note is None:
            continue
        notes.setdefault(note, ([], []))
        notes[note][0].append(t)
        notes[note][1].append(g)
    for note, (ts, gs) in notes.items():
        train_place('solo', glock(note), np.array(ts), np.array(gs), np.zeros(len(ts)))
    return m_drop


nb = bells()
m_drop = soloist()


# ------------------------------------------------------------------ 2) motion sounds
def motion_fx():
    for k in range(T.NSEC):
        a, b = T.LIFT[k]
        if k == 3:
            continue                                   # alto + soprano lift together: one set of sounds (louder)
        both = 1.4 if k == 2 else 1.0
        pan = 0.0
        mix.place('fx', A.whoosh(0.6, 300, 2600), a - 0.05, 0.1 * both, pan)
        d = (b - 0.3) - (a + 0.2)
        rl = roll_noise(d + 0.2, 240, 13, seed=k)
        mix.place('fx', np.stack([rl, rl * 0.35], 1), a + 0.2, 0.5 * both)
        mix.place('fx', np.stack([rl * 0.35, rl], 1), a + 0.22, 0.5 * both)
        mix.place('fx', thud(130, 60, 0.16), b, 0.42 * both)
        mix.place('fx', knock(380 + 40 * k), b + 0.004, 0.22 * both)
        mix.place('fx', knock(300 + 40 * k), b + 0.1, 0.07 * both)
    # the hoist: whoosh, giant cans rumble in, big clunk
    a, b = T.PLAT
    mix.place('fx', A.whoosh(0.8, 200, 2000), a - 0.05, 0.16)
    d = (b - 0.35) - (a + 0.45)
    rl = roll_noise(d + 0.3, 140, 7, seed=9)
    mix.place('fx', np.stack([rl, rl * 0.4], 1), a + 0.45, 0.9)
    mix.place('fx', np.stack([rl * 0.4, rl], 1), a + 0.47, 0.9)
    mix.place('fx', sub_boom(1.4, 70, 34), b, 0.7)
    mix.place('fx', thud(110, 48, 0.25), b, 0.6)
    mix.place('fx', knock(260, 0.08), b + 0.006, 0.25)
    # sway rumble: follows the boards' and platform's speed
    fr = np.arange(int(DUR * T.FPS))
    vel = np.zeros(len(fr))
    prev = None
    for f in fr:
        t = f / T.FPS
        x = [MO.board_state(k, t)[1] for k in range(T.NSEC)] + [MO.plat_state(t)[1] * 3]
        if prev is not None:
            vel[f] = sum(abs(a_ - b_) for a_, b_ in zip(x, prev)) * T.FPS
        prev = x
    env = np.interp(tt(NS), fr / T.FPS, vel)
    rum = lp(np.random.default_rng(4).normal(0, 1, NS), 150, 4) * np.clip(env / 1.5, 0, 1.2)
    mix.place('fx', np.stack([rum, rum], 1), 0.0, 0.5)
    # kick-out + thump at the end
    rl = roll_noise(0.5, 300, 22, seed=12)
    mix.place('fx', np.stack([rl, rl * 0.3], 1), T.T_KICK, 0.6)
    mix.place('fx', np.stack([rl * 0.3, rl], 1), T.T_KICK + 0.02, 0.6)
    mix.place('fx', A.whoosh(0.35, 3000, 400, rise=False), T.T_JOLT - 0.33, 0.12)
    mix.place('fx', sub_boom(1.8, 66, 30), T.T_JOLT, 1.0)
    mix.place('fx', thud(100, 42, 0.3), T.T_JOLT, 0.8)
    mix.place('fx', bp(np.random.default_rng(6).normal(0, 1, int(0.5 * SR)), 200, 3000) * np.exp(-tt(int(0.5 * SR)) / 0.08), T.T_JOLT, 0.25)
    for j in range(10):                                 # the riser cans clatter out
        mix.place('fx', clink(900 + 170 * (j % 5)), T.T_JOLT + 0.05 + 0.045 * j, 0.06, -0.8 if j % 2 else 0.8)


motion_fx()


# ------------------------------------------------------------------ 3) pad: dissonant wash -> D major as the risers lock
def lock_fraction():
    f = np.arange(len(MO.FL_SEC))
    v = MO.FL_SEC[:, :LY.NR].mean(1)
    return f / T.FPS, v


ft, lf = lock_fraction()
t0p, t1p = 3.0, T.T_DROP
n0, n1 = int(t0p * SR), int(t1p * SR)
tv = np.arange(n1 - n0) / SR + t0p
lfc = np.interp(tv, ft, lf)
res = np.clip((lfc - 0.15) / 0.75, 0, 1)
fade = np.clip((tv - t0p) / 1.5, 0, 1) * (1 - np.clip((tv - (t1p - 0.6)) / 0.6, 0, 1))
tension = pad([38, 39, 45, 46, 52], (n1 - n0) / SR, cut=650, att=1.0)
resolve = pad([38, 45, 50, 54, 57], (n1 - n0) / SR, cut=900, att=1.0)
pw = (tension * (1 - res) + resolve * res) * fade
mix.place('pad', np.stack([pw, pw], 1), t0p, 0.5)


# ------------------------------------------------------------------ 4) the pulse emerges (30-38 s), the drop (38-58 s)
def crowd_beats():
    z = MO.ZG
    un = np.unwrap(np.angle(z))
    k = np.floor((un - np.pi / 2) / np.pi)
    out = []
    for f in range(1, len(un)):
        if k[f] > k[f - 1]:
            target = k[f] * np.pi + np.pi / 2
            out.append((f - 1 + (target - un[f - 1]) / (un[f] - un[f - 1])) / T.FPS)
    return np.array(out)


beats = crowd_beats()
KICK = kick()
for tb in beats:
    if 29.6 <= tb < T.T_DROP - 0.05:
        R = abs(MO.ZG[MO.fidx(tb)])
        g = 0.75 * np.clip((R - 0.85) / 0.12, 0, 1) * (0.6 if tb > 36 else 1.0)
        mix.place('drums', lp(KICK, 1200), tb, g)
        if tb >= 31.8:
            mix.place('drums', HAT, tb + T.BEAT / 2, 0.12 * np.clip((tb - 31.8) / 2, 0, 1), 0.3)
        if tb >= 32.0 and tb < 36.0:
            ch = T.CHORD_NOTES['D']
            mix.place('bass', bass_note(ch[0] - 12, 0.4, 420), tb, 0.35 * np.clip((tb - 32) / 1.5, 0, 1))
# "199 in sync": a low hit as the groove pulls back, then riser + snare roll into the drop
mix.place('fx', sub_boom(1.2), 34.0, 0.45)
mix.place('fx', CRASH, 34.0, 0.08)
mix.place('fx', riser(T.T_DROP - 34.4), 34.4, 0.22)
for kk in range(8):
    mix.place('drums', SNARE, T.T_DROP - 2.0 + kk * 0.125, 0.08 + 0.08 * kk / 7)
for kk in range(16):
    mix.place('drums', SNARE, T.T_DROP - 1.0 + kk * 0.0625, 0.16 + 0.2 * kk / 15)

# the song
mix.place('fx', sub_boom(1.8), T.T_DROP, 0.8)
mix.place('drums', CRASH, T.T_DROP, 0.32)
mix.place('drums', CRASH, G(76 + 16), 0.22)
mix.place('drums', CRASH, G(76 + 28), 0.26)
E8 = T.BEAT / 2
kick_times = []
for bar in range(T.SONG_BARS):
    t0 = T.T_DROP + bar * T.BAR
    ch = T.SONG_CHORDS[bar]
    root, triad = T.CHORD_NOTES[ch]
    big = bar >= 7
    for b in range(4):
        tb = t0 + b * T.BEAT
        mix.place('drums', KICK, tb, 0.9)
        kick_times.append(tb)
        if b in (1, 3):
            mix.place('drums', CLAP, tb, 0.42, -0.05)
            mix.place('drums', SNARE, tb, 0.12, 0.08)
        mix.place('drums', OHAT if bar >= 2 else HAT, tb + E8, 0.13 if bar >= 2 else 0.16, 0.3)
        if bar >= 4:
            for s in (1, 3):
                mix.place('drums', SHAKER, tb + s * T.BEAT / 4, 0.18, -0.4)
        # house bass: root on the beat, octave on the offbeat
        mix.place('bass', bass_note(root - 12, E8 * 0.9), tb, 0.5)
        mix.place('bass', bass_note(root, E8 * 0.8), tb + E8, 0.36)
        # offbeat chord stabs (kept below the bells' attack band)
        mix.place('keys', stab([m - 12 for m in triad] + [triad[0]]), tb + E8, 0.55, 0.15 if b % 2 else -0.15)
    mix.place('pad', pad([root, root + 7] + triad, T.BAR + 0.3, cut=1100, att=0.08), t0, 0.4)
    # the melody doubled by a soft lead (the bells carry it, this gives it body)
    for b in range(4):
        m = T.MELODY[bar * 4 + b]
        if m is not None:
            mix.place('lead', lead(m - 12, T.BEAT * 0.95), t0 + b * T.BEAT, 0.3 if not big else 0.36)
    if big:                                            # final chorus: sparkle arpeggio
        tones = triad + [triad[0] + 12]
        for kk in range(16):
            mm = tones[[0, 1, 2, 3, 2, 1, 2, 3][kk % 8]] + 12
            mix.place('keys', pluck(mm, 0.25), t0 + kk * T.BEAT / 4, 0.07, 0.5 if kk % 2 else -0.5)

# sidechain pump on music under the kick
duck = np.ones(NT)
for tk in kick_times:
    i0 = int(tk * SR)
    n = int(0.22 * SR)
    duck[i0:i0 + n] = np.minimum(duck[i0:i0 + n], 1 - 0.5 * np.exp(-tt(n) / 0.07))
for name in ('bass', 'keys', 'pad', 'lead'):
    b = mix.bus(name)
    b[:NT] *= duck[:, None]
# the thump cuts the song dead
for name in ('drums', 'bass', 'keys', 'pad', 'lead'):
    b = mix.bus(name)
    i0 = int((T.T_JOLT - 0.01) * SR)
    b[i0:] = 0
    i1 = int((T.T_JOLT - 0.06) * SR)
    b[i1:i0] *= np.linspace(1, 0, i0 - i1)[:, None]

# ------------------------------------------------------------------ 5) room tone (very low)
n = NT
amb = lp(np.random.default_rng(3).normal(0, 1, n), 500) * 0.02
mix.place('amb', np.stack([amb, np.roll(amb, 700)], 1), 0.0, 1.0)

# carve the bells' band out of the music
MUSIC = ['drums', 'bass', 'keys', 'pad', 'lead', 'fx']
for name in ('keys', 'pad', 'lead'):
    b = mix.bus(name)
    b -= 0.55 * bp(b, 300, 2600)


# ------------------------------------------------------------------ master (loop-aware)
def limiter(x, ceiling=0.89, look=0.003, rel=0.08):
    a = np.abs(x).max(1)
    need = np.minimum(1.0, ceiling / np.maximum(a, 1e-9))
    nl = int(look * SR)
    from scipy.ndimage import minimum_filter1d
    g = minimum_filter1d(need, 2 * nl + 1)
    # release smoothing (one-pole on the way up)
    al = math.exp(-1 / (rel * SR))
    y = lfilter([1 - al], [1, -al], g)
    g = np.minimum(g, y)
    return x * g[:, None]


def master(buses, gains, sends, pts, drive_db=4.0):
    dry = sum(b * gains.get(k, 1.0) for k, b in buses.items())
    send = sum(buses[k] * v for k, v in sends.items() if k in buses)
    full = dry + A.reverb(send, dur=2.6, decay=0.75) * 0.26
    out = full[:NS].copy()
    ne = min(len(full) - NS, int(EXTRA * SR))
    out[:ne] += full[NS:NS + ne]
    g = np.interp(tt(NS), [p[0] for p in pts], [p[1] for p in pts]) + drive_db
    out = out * (10 ** (g / 20))[:, None]
    pk = np.percentile(np.abs(out), 99.9)
    out = out * (0.6 / pk)
    out = limiter(np.r_[out[-int(0.05 * SR):], out])[int(0.05 * SR):]
    out = np.tanh(out * 1.1) / np.tanh(1.1)
    return out * 0.89 / (np.abs(out).max() + 1e-9), g


GAINS = {'bells': 1.0, 'solo': 1.0, 'ticks': 1.0, 'drums': 0.55, 'bass': 0.6, 'keys': 0.8, 'pad': 0.5, 'lead': 0.5, 'fx': 0.7, 'amb': 1.0}
SENDS = {'bells': 0.55, 'solo': 0.5, 'pad': 0.5, 'lead': 0.4, 'keys': 0.4, 'fx': 0.25}
PTS = [(0, 2.0), (5.5, 2.0), (6.0, 1.5), (9.6, 1.0), (12, 0.6), (20, 0.0), (23.5, -0.4), (27.6, -0.8), (33, -1.4),
       (37.9, -1.4), (38.0, -3.2), (57.9, -3.2), (58.0, 0.0), (59.0, 1.5), (60, 2.0)]


def main():
    out, g = master(mix.buses, GAINS, SENDS, PTS)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    # stems on the same scale (same automation; the limiter's gain is not re-applied, fine for band ratios)
    os.makedirs(os.path.join(HERE, 'stems'), exist_ok=True)
    for name, keys in [('sci', ['bells', 'solo', 'ticks']), ('music', MUSIC + ['amb'])]:
        sub = {k: mix.buses[k] for k in keys if k in mix.buses}
        dry = sum(b * GAINS.get(k, 1.0) for k, b in sub.items())
        send = sum(sub[k] * v for k, v in SENDS.items() if k in sub)
        full = dry + (A.reverb(send, dur=2.6, decay=0.75) * 0.26 if not np.isscalar(send) else 0)
        st = full[:NS].copy()
        st[:int(EXTRA * SR)] += full[NS:NS + int(EXTRA * SR)]
        st = st * (10 ** (g / 20))[:, None]
        wavfile.write(os.path.join(HERE, 'stems', f'{name}.wav'), SR, (np.clip(st * 0.1, -1, 1) * 32767).astype(np.int16))
    print(f'bell ticks placed: {nb}; soloist drop slot index {m_drop}')
    spans = [(0, 3), (3, 6), (6, 9.6), (9.6, 13), (13, 17), (17, 21), (21, 25), (25, 30), (30, 34), (34, 38), (38, 46), (46, 52), (52, 58), (58, 60)]
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


if __name__ == '__main__':
    main()
