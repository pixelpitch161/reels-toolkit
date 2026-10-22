"""Domino Symphony - soundtrack. Everything is synthesised and original.

    python audio.py        -> audio.wav, prints section levels, the domino-band margin and the quietest window

The dominoes play the song, from the same table as the picture (layout.npz):
- every domino strike (t0) is a short plastic/wood click, density-balanced so a 1,000-clicks/s field swells
  without swamping the mix; the last domino of each lane lands with a lower clack;
- every chime bar is a glockenspiel note struck by its domino (melody on the beat, arpeggios on the off 16ths);
- the growing staircase booms one bass note lower per step (D3 C#3 B2 A2 G2 F#2 E2) into the giant's D slam;
- the holdout knocks on the plank each time it rocks back; the two spiral lanes thud into the post.
Music (original, D major, 120 BPM = the domino grid): a dissonant swell resolving as the lines lock (0-8),
pads + bass (8-20), kick from 14, the field swell (20-26), a snare build matched to the spiral's 16ths ->
triplets -> 32nds (26-32), suspense with a riser and snare roll on an A pedal (32-38), the staircase booms,
the drop at 40 (kick, clap, bass, pads, chimes on the frame lanes), the final D chord at 52, then the rewind:
every domino un-clicks backwards into the loop."""
import math
import os
import sys
import numpy as np
from scipy.ndimage import minimum_filter1d, uniform_filter1d
from scipy.signal import lfilter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit import audio as A
import timeline as T
from boxes import Boxes

SR = A.SR
NS = int(T.DUR * SR)
B = Boxes()
DOM = B.is_dom
rng = np.random.default_rng(2026)


def tt(n):
    return np.arange(n) / SR


# ---------------------------------------------------------------- instruments
def make_click(seed, lo=2300, hi=4300, dec=0.006, low=1.0):
    r = np.random.default_rng(seed)
    n = int(0.035 * SR)
    t = tt(n)
    burst = A.hp(r.normal(0, 1, n), 2500) * np.exp(-t / 0.0011)
    f1 = r.uniform(lo, hi)
    body = (np.sin(2 * np.pi * f1 * t + r.uniform(0, 6)) * np.exp(-t / dec)
            + 0.5 * np.sin(2 * np.pi * f1 * r.uniform(1.45, 1.75) * t) * np.exp(-t / (dec * 0.6))
            + 0.45 * low * np.sin(2 * np.pi * r.uniform(780, 1250) * t) * np.exp(-t / (dec * 1.3)))
    x = burst * 0.8 + body * 0.6
    x *= np.clip(t / 0.0002, 0, 1)
    return x / (np.abs(x).max() + 1e-9)


CLICK_PL = [make_click(s) for s in range(24)]                                 # plastic: trunk, fan, lanes, frame
CLICK_WD = [make_click(100 + s, 1500, 2700, 0.008, 1.4) for s in range(16)]   # wood-ish: the field
CLICK_SP = [make_click(200 + s, 2800, 5000, 0.004, 0.7) for s in range(16)]   # spiral: brighter, tighter


def clack(seed):
    r = np.random.default_rng(seed)
    n = int(0.08 * SR)
    t = tt(n)
    x = A.bp(r.normal(0, 1, n), 500, 3000) * np.exp(-t / 0.006) + np.sin(2 * np.pi * r.uniform(420, 640) * t) * np.exp(-t / 0.02)
    return x / np.abs(x).max()


CLACKS = [clack(300 + s) for s in range(8)]


def chime(m, dur=1.8, bright=1.0):
    n = int(dur * SR)
    t = tt(n)
    f = A.mtof(m)
    x = np.zeros(n)
    for ratio, amp, dk in ((1.0, 1.0, 1.0), (2.756, 0.32 * bright, 0.4), (5.404, 0.12 * bright, 0.18), (8.933, 0.05 * bright, 0.09)):
        if f * ratio < 15000:
            x += amp * np.sin(2 * np.pi * f * ratio * t) * np.exp(-t / (0.55 * dk * (1.0 if m < 84 else 0.75)))
    x += A.hp(np.random.default_rng(int(m)).normal(0, 1, n), 4000) * np.exp(-t / 0.0015) * 0.25
    return x * np.clip(t / 0.0006, 0, 1)


def boom(m, k):
    """staircase step k (0..7): a struck slab - pitched thump + sub + wood slap, bigger each step"""
    dur = 0.9 + 0.12 * k
    n = int(dur * SR)
    t = tt(n)
    f = A.mtof(m)
    fs = f * (1 + 0.8 * np.exp(-t / 0.03))
    ph = 2 * np.pi * np.cumsum(fs) / SR
    x = np.sin(ph) * np.exp(-t / (0.25 + 0.05 * k))
    x += 0.8 * np.sin(np.pi * np.cumsum(fs) / SR) * np.exp(-t / (0.3 + 0.06 * k))
    x += 0.5 * np.sin(2 * ph + 0.3) * np.exp(-t / 0.12)
    x += A.lp(np.random.default_rng(k).normal(0, 1, n), 400 + 100 * k) * np.exp(-t / 0.05) * 0.8
    x += A.bp(np.random.default_rng(10 + k).normal(0, 1, n), 800, 4000) * np.exp(-t / 0.008) * 0.5
    return x / np.abs(x).max()


def thud(f0=70, dur=0.6, seed=0):
    n = int(dur * SR)
    t = tt(n)
    fs = f0 * (1 + 1.2 * np.exp(-t / 0.02))
    x = np.sin(2 * np.pi * np.cumsum(fs) / SR) * np.exp(-t / 0.16)
    x += A.lp(np.random.default_rng(seed).normal(0, 1, n), 900) * np.exp(-t / 0.025) * 0.6
    x += A.bp(np.random.default_rng(seed + 1).normal(0, 1, n), 1200, 5000) * np.exp(-t / 0.004) * 0.3
    return x / np.abs(x).max()


def knock(seed):
    n = int(0.12 * SR)
    t = tt(n)
    r = np.random.default_rng(seed)
    x = np.sin(2 * np.pi * r.uniform(820, 980) * t) * np.exp(-t / 0.018) + 0.6 * np.sin(2 * np.pi * 1900 * t) * np.exp(-t / 0.006)
    x += A.bp(r.normal(0, 1, n), 1500, 6000) * np.exp(-t / 0.003) * 0.5
    return x / np.abs(x).max()


def bass_note(m, dur, cut=700, vel=1.0):
    n = int(dur * SR)
    t = tt(n)
    f = A.mtof(m)
    x = A.saw(f, n, det=0.004, voices=2) * 0.6 + np.sin(2 * np.pi * f * t)
    x = A.lp(x, cut) * A.adsr(n, 0.004, 0.12, 0.7, min(0.08, dur * 0.3))
    return x * vel


def pad_chord(ms, dur, cut=1400, att=0.25):
    return A.pad(ms, dur, cut=cut, att=att)


def riser(dur, lo=300, hi=6000):
    n = int(dur * SR)
    x = A.whoosh(dur, lo, hi, rise=True, seed=8)
    env = np.linspace(0.2, 1, n) ** 2
    return x * env


def rev(x):
    return x[::-1].copy()


# ---------------------------------------------------------------- dominoes -> clicks
def domino_clicks(mix):
    t0 = B.t0[DOM]
    sec = B.sec[DOM]
    idx = np.where(DOM)[0]
    order = np.argsort(t0)
    ts = t0[order]
    # local density: clicks within +-50 ms
    lo = np.searchsorted(ts, ts - 0.05)
    hi = np.searchsorted(ts, ts + 0.05)
    dens = (hi - lo).astype(float)
    dens_of = np.empty_like(dens)
    dens_of[order] = dens
    SG = {0: 1.5, 1: 1.6, 2: 1.4, 3: 1.05, 4: 0.85, 5: 0.85, 6: 1.6, 7: 2.6, 8: 2.6, 9: 0.0, 10: 0.0, 11: 0.8}
    for j, i in enumerate(idx):
        s = sec[j]
        g = SG.get(int(s), 0.8)
        if g == 0:
            continue
        g *= max(1.0, dens_of[j]) ** -0.42
        t = t0[j]
        if s == 11:                                   # frame lanes: 32nds, accent the 16ths
            q = int(round((t - T.T_DROP) * 32)) % 4
            g *= 1.6 if q == 0 else 0.75
            bank = CLICK_PL
        elif s in (6, 7):
            bank = CLICK_WD
        elif s == 8:
            bank = CLICK_SP
        else:
            bank = CLICK_PL
        pan = float(np.clip(B.cx[i] / 1.8, -0.8, 0.8))
        mix.place('click', bank[rng.integers(len(bank))], t, g, pan)
        if B.nxt[i] < 0:                              # lane end: lands on the floor
            mix.place('click', CLACKS[rng.integers(8)], t + B.tc[i], g * 1.4, pan)


def chimes(mix):
    bars = np.where((B.kind == 1) & (B.sec == 12))[0]
    for i in bars:
        m = int(B.pitch[i])
        if m < 0:
            continue
        lead = (m >= 78)
        g = 0.42 if lead else 0.26
        if B.t0[i] >= T.T_DROP:
            g *= 0.75                                  # two sides play together (stereo pair)
        pan = float(np.clip(B.cx[i] / 1.7, -0.9, 0.9))
        mix.place('chime', chime(m, 1.6 if lead else 1.1), B.t0[i], g, pan)
    # final bar: both last frame dominoes land on it -> a D major chime chord
    for m, g in ((74, 0.5), (78, 0.4), (81, 0.36), (86, 0.5), (90, 0.28)):
        mix.place('chime', chime(m, 3.5, 1.2), T.T_END, g)


# ---------------------------------------------------------------- events
def events(mix):
    mix.place('fx', knock(1) * 0.8, 0.0, 0.8)                                   # the paw tap
    mix.place('fx', riser(1.05, 120, 2500), 0.0, 0.22)                           # tension into the explosion
    n = int(1.05 * SR)
    hum = A.lp(sum(A.saw(A.mtof(m), n, det=0.008, voices=2) for m in (38, 45)) / 2, 300) * np.linspace(0.4, 1, n)
    mix.place('music', hum, 0.0, 0.1)
    mix.place('fx', thud(55, 1.2, 3), T.T_FAN, 1.0)                              # bass hit as the line explodes
    mix.place('drums', A.crash(2.0), T.T_FAN, 0.22)
    # the field: a clatter swell under the 1,000 clicks/s (noise shaped by the number of falling columns)
    n = int(6.4 * SR)
    t = tt(n)
    act = np.interp(T.T_FIELD + t, [20, 22, 24, 26, 26.4], [0.1, 1.0, 1.0, 0.15, 0])
    sw = A.bp(np.random.default_rng(40).normal(0, 1, n), 1200, 7000) * act
    mix.place('click', np.stack([sw, A.bp(np.random.default_rng(41).normal(0, 1, n), 1200, 7000) * act], 1), T.T_FIELD, 0.11)
    mix.place('fx', thud(70, 0.7, 5), T.T_HOLD, 1.0)                             # inner lane hits the post
    mix.place('fx', thud(78, 0.6, 6), T.T_HOLD2, 0.85)                           # outer lane
    for k, tk in enumerate(B.holdout_knocks()):
        mix.place('fx', knock(10 + k), tk, 0.55 if tk < 35 else 0.65)
    # teeter creak (quiet)
    n = int(1.1 * SR)
    t = tt(n)
    cr = A.bp(np.random.default_rng(4).normal(0, 1, n), 700, 1600) * (0.5 + 0.5 * np.sin(2 * np.pi * 7 * t)) * np.sin(np.pi * t / 1.1)
    mix.place('fx', cr, 36.5, 0.12)
    for k, (tb, m) in enumerate(T.STAIR_BOOMS):
        g = 0.55 + 0.06 * k
        mix.place('boom', boom(m, k), tb, g)
        mix.place('boom', boom(m + 12, k) * 0.4, tb, g * 0.5)
    mix.place('fx', A.whoosh(0.5, 200, 2500, rise=False, seed=12), 39.5, 0.35)   # the giant falls
    mix.place('boom', thud(34, 2.4, 9), T.T_DROP, 1.0)                           # the slam
    mix.place('drums', A.crash(3.0), T.T_DROP, 0.5)


# ---------------------------------------------------------------- music
def music(mix):
    bar = T.BAR
    beat = T.BEAT
    # 0-4: dissonant low cluster swelling with the chaos, a rising wash; resolves into D at 4
    n = int(4.2 * SR)
    t = tt(n)
    clus = sum(A.saw(A.mtof(m), n, det=0.01, voices=3) for m in (38, 39, 45, 50, 51)) / 5
    clus = A.lp(clus, 500) * np.clip((t - 0.9) / 3.0, 0, 1) ** 1.5 * np.clip((4.2 - t) / 0.3, 0, 1)
    mix.place('music', clus, 0.0, 0.3)
    # 4-8: the rhythm emerging - a low pulse on every beat, louder as the lanes lock
    for k in range(8):
        tb = 4.0 + k * beat
        mix.place('drums', A.kick(0.8, 0.2, 110, 50), tb, 0.25 + 0.04 * k)
    mix.place('fx', riser(3.0, 200, 5000), 1.0, 0.16)
    # pads 4-52 (one chord per bar), brighter as it builds
    for b in range(2, 26):
        c = T.CHORD_OF_BAR[b]
        cut = 900 if b < 4 else 1300 if b < 13 else 1700
        g = 0.20 if b < 4 else 0.24
        if 16 <= b <= 18:
            continue                               # suspense section has its own pedal
        mix.place('music', pad_chord(T.PAD[c], bar + 0.25, cut=cut, att=0.35 if b == 2 else 0.12), b * bar, g)
    # 32-38: A pedal (A2 + E3 + A3) with a slow tremolo, opening filter
    n = int(6.0 * SR)
    t = tt(n)
    ped = sum(A.saw(A.mtof(m), n, det=0.006, voices=3) for m in (45, 52, 57, 64)) / 4
    ped = A.lp(ped, 900) * (0.75 + 0.25 * np.sin(2 * np.pi * 4 * t)) * np.clip(t / 0.4, 0, 1)
    mix.place('music', ped, 32.0, 0.11)
    # bass: roots, whole notes 8-14, quarters 14-32, 8ths in the drop
    for b in range(4, 26):
        c = T.CHORD_OF_BAR[b]
        r = T.ROOT[c]
        if 16 <= b <= 19:
            continue
        if b < 7:
            mix.place('bass', bass_note(r, bar * 0.95, 400), b * bar, 0.5)
        elif b < 20:
            for k in range(4):
                mix.place('bass', bass_note(r, beat * 0.9, 500), b * bar + k * beat, 0.42)
        else:
            for k in range(8):
                m = r + (12 if k % 2 else 0)
                mix.place('bass', bass_note(m, beat * 0.45, 900), b * bar + k * beat / 2, 0.5)
    mix.place('bass', bass_note(38, 4.0, 500), T.T_END, 0.6)
    # drums
    for b in range(7, 16):                                   # kick from 14 (bar 7), through the field and the build
        for k in range(4):
            mix.place('drums', A.kick(), b * bar + k * beat, 0.62)
    for k in range(16):                                      # spiral build: snare 8ths -> triplets -> 16ths, rising
        pass
    # spiral build: rolls matched to the spiral (16ths -> triplets -> 32nds), a rising tom carries it so the
    # domino clicks keep the 2-6 kHz band; a quiet snare on top
    def tom(f, dec=0.09):
        n = int(0.25 * SR)
        t = tt(n)
        fs = f * (1 + 0.5 * np.exp(-t / 0.015))
        return np.sin(2 * np.pi * np.cumsum(fs) / SR) * np.exp(-t / dec) + A.lp(np.random.default_rng(3).normal(0, 1, n), 2000) * np.exp(-t / 0.01) * 0.2
    for (ta, tb_, step) in ((26.0, 28.0, 0.25), (28.0, 30.0, 1 / 6), (30.0, 32.0, 0.125)):
        tb = ta
        while tb < tb_ - 1e-6:
            u = (tb - 26.0) / 6.0
            mix.place('drums', tom(110 + 110 * u), tb, 0.30 + 0.25 * u)
            mix.place('drums', A.snare(0.06), tb, 0.05 + 0.08 * u)
            tb += step
    mix.place('fx', riser(6.0, 300, 7000), 26.0, 0.11)
    # 32-38 suspense: soft snare roll accelerating, riser, heartbeat sub on the beats
    tb, rate = 32.25, 4.0
    while tb < 38.0:
        mix.place('drums', A.snare(0.05), tb, 0.04 + 0.08 * ((tb - 32) / 6) ** 1.5)
        rate = 4.0 + 28.0 * ((tb - 32) / 6) ** 2
        tb += 1.0 / rate
    mix.place('fx', riser(6.0, 250, 5000), 32.0, 0.10)
    for b in range(64, 76):
        tb = b * beat
        mix.place('bass', np.sin(2 * np.pi * 45 * tt(int(0.25 * SR))) * np.exp(-tt(int(0.25 * SR)) / 0.08), tb, 0.35)
    # 40-52 drop: kick 4/4, clap 2 & 4, open hats off-beats
    for b in range(20, 26):
        for k in range(4):
            tk = b * bar + k * beat
            mix.place('drums', A.kick(1.1), tk, 0.75)
            if k % 2:
                mix.place('drums', A.clap(), tk, 0.42)
            mix.place('drums', A.hat(open_=True), tk + beat / 2, 0.1)
    mix.place('drums', A.crash(2.5), 48.0, 0.3)
    mix.place('drums', A.crash(3.5), T.T_END, 0.5)
    mix.place('drums', A.kick(1.2), T.T_END, 0.8)
    # 52-56 ring-out: final D pad + a slow chime arpeggio tag
    mix.place('music', pad_chord(T.PAD['D'] + [74], 5.0, cut=1500, att=0.05), T.T_END, 0.32)
    for k, m in enumerate([74, 78, 81, 86, 81, 78, 86, 90, 86, 81, 78, 74]):
        mix.place('chime', chime(m, 1.2), T.T_END + 0.5 + k * 0.25, 0.34 * (1 - k / 16))
    for k in range(7):
        mix.place('drums', A.kick(0.9), T.T_END + 0.5 + k * beat, 0.55 * (1 - k / 9))
        mix.place('drums', A.hat(), T.T_END + 0.75 + k * beat, 0.12)


# ---------------------------------------------------------------- rewind: every domino un-clicks backwards
def rewind(mix):
    ts = np.linspace(T.T_REW, T.T_REW_END, 20000)
    tau = np.array([T.scene_time(t) for t in ts])
    t0 = np.sort(B.t0[DOM])
    tr = np.interp(-t0, -tau, ts)          # tau decreases with t
    lo = np.searchsorted(tr, tr - 0.02)
    hi = np.searchsorted(tr, tr + 0.02)
    dens = (hi - lo).astype(float)
    for k, (t, d) in enumerate(zip(tr, dens)):
        if t <= T.T_REW + 0.01:
            continue
        c = rev(CLICK_PL[k % 24][:int(0.02 * SR)])
        mix.place('click', c, t - 0.02, 0.9 * max(1.0, d) ** -0.5)
    w = A.whoosh(3.6, 200, 7000, rise=True, seed=21)
    mix.place('fx', w, T.T_REW - 0.1, 0.5)
    # tape-reverse of the final chord swelling in as the rewind starts (no gap after the ring-out)
    n = int(2.0 * SR)
    ch = sum(chime(m, 2.0, 1.0)[:n] for m in (74, 78, 81, 86))
    mix.place('chime', rev(ch), T.T_REW - 0.6, 0.35)
    mix.place('music', rev(pad_chord(T.PAD['D'], 2.0, cut=1600, att=0.05)), T.T_REW - 0.3, 0.3)
    mix.place('drums', rev(A.crash(1.6)), T.T_REW_END - 1.6, 0.35)


# ---------------------------------------------------------------- master
def master(mix, gains, sends, gain_pts, ceiling=0.89, drive_db=4.5, comp_thr=0.13):
    dry = sum(b * gains.get(k, 1.0) for k, b in mix.buses.items())
    send = sum(mix.buses[k] * v for k, v in sends.items() if k in mix.buses)
    x = dry + A.reverb(send, dur=2.6, decay=0.7) * 0.22
    # wrap everything past 60 s onto the start (seamless loop)
    tail = x[NS:NS + SR * 5].copy()
    x = x[:NS].copy()
    x[:len(tail)] += tail
    g = np.interp(tt(NS), [p[0] for p in gain_pts], [p[1] for p in gain_pts])
    x *= (10 ** (g / 20))[:, None]
    a = math.exp(-1.0 / (0.08 * SR))
    env = np.sqrt(lfilter([1 - a], [1, -a], (x ** 2).mean(1)).clip(1e-12))
    x *= np.where(env > comp_thr, (comp_thr / env) ** 0.5, 1.0)[:, None]
    x *= 10 ** (drive_db / 20)
    lim = 0.84
    pk = np.abs(x).max(1)
    gl = np.minimum(1.0, lim / np.maximum(pk, 1e-9))
    L = int(0.003 * SR)
    gl = uniform_filter1d(minimum_filter1d(gl, 2 * L + 1), 2 * L + 1)
    hop = SR // 1000
    gb = gl[:len(gl) // hop * hop].reshape(-1, hop).min(1)
    rel = math.exp(-1.0 / 60.0)
    y = np.empty_like(gb)
    cur = 1.0
    for i, v in enumerate(gb):
        cur = v if v < cur else min(v, 1 - (1 - cur) * rel)
        y[i] = cur
    gs = np.interp(np.arange(len(gl)), np.arange(len(y)) * hop + hop // 2, y)
    x *= np.minimum(gl, gs)[:, None]
    x = np.tanh(x * 1.05) / np.tanh(1.05)
    x *= ceiling / (np.abs(x).max() + 1e-9)
    fd = int(0.003 * SR)
    x[:fd] *= np.linspace(0, 1, fd)[:, None]
    x[-fd:] *= np.linspace(1, 0, fd)[:, None]
    return x


GAINS = {'click': 1.0, 'chime': 1.0, 'music': 4.5, 'bass': 0.7, 'drums': 0.9, 'fx': 0.9, 'boom': 1.0}
SENDS = {'chime': 0.5, 'music': 0.8, 'fx': 0.35, 'boom': 0.3, 'click': 0.06}
GAIN_PTS = [(0, 3.0), (7.5, 3.0), (8.5, 0.0), (19.5, 0.0), (20.2, 2.5), (25.6, 3.0), (26.4, 0.5), (31.5, 1.5), (32.0, 0.0), (60, 0.0)]


def main():
    mix = A.Mix(T.DUR)
    domino_clicks(mix)
    chimes(mix)
    events(mix)
    music(mix)
    rewind(mix)
    # tame the click bus's crest factor (fast compressor) so 1,000 clicks/s don't make the master limiter pump
    cb = mix.buses['click']
    lev = np.abs(cb).max(1)
    att, relc = math.exp(-1.0 / (0.0008 * SR)), math.exp(-1.0 / (0.06 * SR))
    env = np.empty_like(lev)
    e = 0.0
    for i in range(0, len(lev), 48):
        v = lev[i:i + 48].max()
        e = v if v > e else e * relc ** 48
        env[i:i + 48] = e
    env = uniform_filter1d(env, 96)
    thr = 0.22
    mix.buses['click'] = cb * np.minimum(1.0, (thr / np.maximum(env, 1e-6)) ** 0.7)[:, None]
    # carve the music out of the domino band (2-6 kHz) so clicks and chimes stay on top
    for k in ('music', 'bass', 'drums'):
        bb = mix.buses[k]
        mix.buses[k] = bb - (0.8 if k != 'drums' else 0.6) * A.bp(bb, 2000, 6000)
    out = master(mix, GAINS, SENDS, GAIN_PTS)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    report(mix, out)


def report(mix, out):
    spans = [(0, 1), (1, 4), (4, 8), (8, 14), (14, 20), (20, 26), (26, 32), (32, 35.5), (35.5, 38), (38, 40), (40, 46), (46, 52),
             (52, 56), (56, 60)]
    print('section levels:')
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'  {a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')
    from scipy.signal import butter, sosfilt
    sos = butter(4, [2000, 6000], 'bp', fs=SR, output='sos')

    def stem(keep):
        return sum(mix.buses[k][:NS] * GAINS.get(k, 1.0) for k in keep if k in mix.buses).mean(1)
    ys = sosfilt(sos, stem(['click', 'chime']))
    yr = sosfilt(sos, stem(['music', 'bass', 'drums', 'fx', 'boom']))
    lvl = lambda y, a, b: 20 * np.log10(np.sqrt((y[int(a * SR):int(b * SR)] ** 2).mean()) + 1e-12)
    print('domino band 2-6 kHz (clicks + chimes) vs everything else:')
    print('  ' + '  '.join(f'{a:.0f}-{b:.0f}:{lvl(ys, a, b) - lvl(yr, a, b):+.0f}' for a, b in spans))
    x = out.mean(1)
    w = int(0.5 * SR)
    r = np.sqrt(np.convolve(x * x, np.ones(w) / w, mode='valid'))[::SR // 20]
    print(f'quietest 0.5 s window: {20 * np.log10(r.min() + 1e-12):.1f} dBFS at {np.argmin(r) / 20:.2f} s')


if __name__ == '__main__':
    main()
