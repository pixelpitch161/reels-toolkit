"""Maze Race soundtrack, mood B (heist chase): D minor, 121.97 BPM (31 bars in 61 s). Original and synthesised.

The race plays the music. Every cell an explorer uncovers is the next note of its part, on the beat grid (the
fairness schedule puts each explorer on its own 16th: FLOOD on the beat, SEEKER on the &, DIVER on the a; 2 per beat
in the final stretch):
    FLOOD   strings: a staccato ostinato on the chord (root-5-8-5); every 4th ring of its flood surges (accent + octave)
    DIVER   bass: a picked spy line whose pitch follows its stack depth (climbs as it digs); a dead end is a bonk
            and a quick descending run while the thread winds back
    SEEKER  marimba: pitch from how close the cell is to the centre (closer = higher); a dead end near the centre is
            a wooden bonk
    MARBLE  drums: every wall hit is a hit (impact speed = velocity, quantised to the 16th grid) over a tight kick/hat
            bed that starts when its gate opens
Finishers land a chord in their instrument (the winner with a crash and a boom); bars 26-28 play the winning route as a
melody (one note per route cell, rising toward the centre, all four instruments in); bar 29 resolves; bar 30 is the
pickup into GO. The file is mixed as a loop (2860 AAC frames = 61.013 s, see NA): tails past the end wrap onto the start."""
import math
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit import audio as A
import maze as M
import timeline as T

SR = A.SR
BAR, BEAT, S16 = T.BAR, T.BEAT, T.S16
rng = np.random.default_rng(23)

# ------------------------------------------------------------------ harmony (one chord per bar)
DM, BB, AM, GM, FM = [50, 53, 57], [46, 50, 53], [45, 49, 52], [43, 46, 50], [41, 45, 48]
PROG = ([DM, DM, BB, AM] * 3 + [GM, GM, BB, AM] + [DM, DM, BB, AM] + [DM, BB, GM, AM] + [DM, AM] + [DM, BB, AM] + [DM] + [AM])
assert len(PROG) == 31
DMIN = [0, 2, 3, 5, 7, 8, 10]                         # D natural minor (from D)
PENT = [0, 3, 5, 7, 10]                               # D minor pentatonic


def chord_at(t):
    return PROG[min(30, max(0, int(t // BAR)))]


def in_scale(m):
    """snap a midi note to D natural minor"""
    pc = (m - 62) % 12
    best = min(DMIN, key=lambda s: abs(s - pc))
    return m - pc + best


# ------------------------------------------------------------------ instruments
def env(n, a, d):
    t = A.tt(n)
    return np.clip(t / max(a, 1e-4), 0, 1) * np.exp(-t / d)


def strings_stac(m, accent=False):
    n = int(0.42 * SR)
    f = A.mtof(m)
    x = A.saw(f, n, det=0.005, voices=3) * 0.7 + A.saw(f * 2, n, det=0.004, voices=2) * 0.18
    x = A.lp(x, 3200 if accent else 2300)
    e = env(n, 0.006, 0.16 if accent else 0.11)
    out = x * e
    if accent:
        out += A.lp(A.saw(f / 2, n, det=0.004, voices=3), 1400) * env(n, 0.01, 0.2) * 0.6
    return out


def bass_pick(m, ghost=False):
    n = int(0.4 * SR)
    f = A.mtof(m)
    t = A.tt(n)
    raw = A.saw(f, n) * 0.6 + A.square(f, n, 0.4) * 0.3
    # pick: a fast filter sweep, short decay
    seg = []
    out = np.zeros(n)
    cut = 300 + 2600 * np.exp(-t / 0.05)
    for k in range(8):
        a, b = k * n // 8, (k + 1) * n // 8
        out[a:b] = A.lp(raw[max(0, a - 512):b], float(cut[(a + b) // 2]))[-(b - a):]
    out = out * env(n, 0.003, 0.08 if ghost else 0.17)
    out += np.sin(2 * np.pi * f * t) * env(n, 0.003, 0.2) * (0.25 if ghost else 0.6)
    return out


def marimba(m, vel=1.0):
    n = int(0.7 * SR)
    f = A.mtof(m)
    t = A.tt(n)
    x = np.sin(2 * np.pi * f * t) * np.exp(-t / 0.32)
    x += 0.45 * np.sin(2 * np.pi * f * 3.93 * t) * np.exp(-t / 0.045)
    x += 0.12 * np.sin(2 * np.pi * f * 9.9 * t) * np.exp(-t / 0.012)
    x += 0.2 * A.bp(rng.normal(0, 1, n), 1500, 6000) * np.exp(-t / 0.004)
    return x * np.clip(t / 0.0015, 0, 1) * vel


def bonk(f0=180.0, f1=70.0):
    """a dull wooden bonk (a dead end)"""
    n = int(0.3 * SR)
    t = A.tt(n)
    f = f1 + (f0 - f1) * np.exp(-t / 0.03)
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.09)
    x += 0.4 * A.bp(rng.normal(0, 1, n), 400, 1800) * np.exp(-t / 0.012)
    return x


def tom(f0, dec=0.22):
    n = int(0.45 * SR)
    t = A.tt(n)
    f = f0 * (1 + 0.6 * np.exp(-t / 0.03))
    return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / dec) + 0.25 * A.bp(rng.normal(0, 1, n), 200, 3000) * np.exp(-t / 0.01)


def rim():
    n = int(0.12 * SR)
    t = A.tt(n)
    return (np.sin(2 * np.pi * 1700 * t) * 0.6 + A.bp(rng.normal(0, 1, n), 2000, 7000)) * np.exp(-t / 0.018)


def boom(dur=1.6):
    n = int(dur * SR)
    t = A.tt(n)
    f = 38 + 60 * np.exp(-t / 0.08)
    return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.5) + 0.3 * A.lp(rng.normal(0, 1, n), 300) * np.exp(-t / 0.2)


def riser(dur):
    n = int(dur * SR)
    x = A.whoosh(dur, 200, 7000, rise=True, seed=9)
    return x * np.linspace(0.2, 1, n) ** 2


def fit(x, n, offset=0):
    """pad / trim a mono signal to n samples, starting at sample `offset`"""
    out = np.zeros(n)
    m = min(len(x), n - offset)
    if m > 0:
        out[offset:offset + m] = x[:m]
    return out


def chord_stab(ms, kind):
    """a finisher's chord in its own instrument"""
    n = int(1.6 * SR)
    out = np.zeros(n)
    for m in ms:
        if kind == 0:
            x = fit(strings_stac(m + 12, accent=True), n) + fit(A.pad([m + 12], 1.6, cut=2600, att=0.01), n) * 0.5
        elif kind == 1:
            x = fit(bass_pick(m - 12), n) + fit(A.pad([m], 1.6, cut=1200, att=0.01), n) * 0.5
        elif kind == 2:
            x = fit(marimba(m + 12), n) + fit(marimba(m + 24, 0.6), n, int(0.06 * SR))
        else:
            x = fit(tom(A.mtof(m - 12)), n)
        out += x
    return out / len(ms)


# ------------------------------------------------------------------ the mix
# AAC codes 1024-sample frames and 61.0 s isn't a whole number of them, so the encoder would pad the last one with
# zeros (a 13 ms dropout before the loop). The soundtrack is therefore built as a loop exactly 2860 AAC frames long
# (61.0133 s): tails wrap at that length, the mp4 carries it untrimmed (mux.py) and it restarts sample-perfectly.
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024


def main():
    mix = A.Mix(T.DUR)
    mix.NS = NA
    fins = [f if f is not None else 1e9 for f in T.FIN]

    # FLOOD: strings ostinato, every 4th ring surges
    ost = [0, 7, 12, 7]
    for k, e in enumerate(T.STEPS[0]):
        t = e['t']
        ch = chord_at(t)
        m = ch[0] + 12 + ost[k % 4]
        acc = bool(e['ring']) and e['depth'] % 4 == 0
        mix.place('flood', strings_stac(m, acc), t, 2.1 if acc else 1.5, pan=-0.35)

    # DIVER: picked bass from its stack depth; dead ends bonk and run down
    climb = [0, 3, 5, 7, 10, 12, 15, 17, 19, 22]
    prev_m = None
    for e in T.STEPS[1]:
        t = e['t']
        ch = chord_at(t)
        m = ch[0] - 12 + climb[(e['depth'] - 1) % 10]
        if e['dead']:
            mix.place('diver', bonk(150, 60), t, 0.5, pan=0.3)
            top = prev_m if prev_m is not None else m + 5
            for j in range(min(4, len(e['back']))):
                mix.place('diver', bass_pick(top - 3 * (j + 1), ghost=True), t + 0.03 + j * S16 * 0.35, 0.22, pan=0.25)
        mix.place('diver', bass_pick(m), t, 0.55, pan=0.15)
        prev_m = m

    # SEEKER: marimba from closeness to the centre
    hmax = M.centre_h((0, 0))
    scale = [62 + o + 12 * k for k in range(3) for o in PENT]
    for e in T.STEPS[2]:
        t = e['t']
        h = M.centre_h(e['cell'])
        idx = int(round((1 - h / hmax) * (len(scale) - 3)))
        m = scale[max(0, min(len(scale) - 1, idx))]
        if e['dead']:
            near = M.centre_h(e['dead_at']) <= 3.6
            mix.place('seeker', bonk(260 if near else 200, 110), t, 0.55 if near else 0.32, pan=0.45)
        mix.place('seeker', marimba(m), t, 0.42, pan=0.4)

    # MARBLE: wall hits on the 16th grid, over a tight bed from its gate
    mix.place('fx', rim(), M.MARBLE_RELEASE, 0.28)                                           # the gate clacks open
    mix.place('fx', tom(220, 0.08), M.MARBLE_RELEASE, 0.18)
    slots = {}
    for (t, v, kind, x, z) in T.HITS:
        if t > T.T_STOP:
            continue
        k = int(round(t / S16))
        if k not in slots or slots[k][0] < v:
            slots[k] = (v, kind, x)
    for k, (v, kind, x) in slots.items():
        t = k * S16
        g = min(1.0, 0.25 + v / 6.0)
        pan = (x / M.N - 0.5) * 0.8
        if v > 4.5:
            mix.place('drums', A.snare(0.16), t, 0.5 * g, pan)
            mix.place('drums', A.crash(1.0), t, 0.08 * g, pan)
        elif kind == 'h':
            mix.place('drums', A.snare(0.1), t, 0.36 * g, pan)
        else:
            mix.place('drums', tom(90 + 130 * (x / M.N)), t, 0.42 * g, pan)
        mix.place('drums', rim(), t, 0.12 * g, pan)
    nb = int(T.DUR / BEAT)
    for b in range(4, nb):
        t = b * BEAT
        bar = b // 4
        if bar >= 29:
            continue
        mix.place('bed', A.kick(), t, 0.36 if b % 2 == 0 else 0.26)
        if bar >= 2:
            for s in range(4 if bar >= T.T_FINAL / BAR else 2):
                mix.place('bed', A.hat(), t + s * (S16 if bar >= T.T_FINAL / BAR else 2 * S16), 0.09 if s % 2 == 0 else 0.055, 0.2)
        if bar >= 4 and b % 4 in (1, 3):
            mix.place('bed', A.clap(), t, 0.16)
        if bar >= 16 and bar < 20 and b % 4 == 3:
            for s in range(1, 4):
                mix.place('drums', A.snare(0.06), t + s * S16, 0.12 + 0.04 * s)

    # the desk clock ticks on every beat (its second hand ticks on screen)
    for b in range(int(T.DUR / BEAT)):
        n = int(0.05 * SR)
        tk = A.bp(rng.normal(0, 1, n), 2500 if b % 2 == 0 else 1900, 6500) * np.exp(-A.tt(n) / 0.006)
        mix.place('bed', tk, b * BEAT, 0.11 if b < 4 else 0.09, 0.45)
    # bed: a low pad and sub on the bar roots (quiet, under the explorers)
    for bar in range(1, 31):
        t = bar * BAR
        ch = PROG[bar]
        if bar < 29:
            mix.place('pad', A.pad([m + 12 for m in ch], BAR, cut=1100, att=0.2), t, 0.07 if bar < 20 else 0.1)
            mix.place('pad', A.bass(ch[0] - 12, BAR * 0.95), t, 0.16)

    # final stretch: riser into the first finish
    if T.T_WIN is not None:
        a = T.T_FINAL + 2 * BAR
        mix.place('fx', riser(T.T_WIN - a), a, 0.2)
    # finishers
    for q, f in enumerate(T.FIN):
        if f is None:
            continue
        ch = chord_at(f)
        top = T.PLACE[q] == 0
        mix.place('fx', chord_stab([m + 12 for m in ch] + [ch[0] + 24], q), f, 0.5 if top else 0.32, pan=[-0.3, 0.2, 0.35, -0.2][q])
        mix.place('drums', A.crash(2.2), f, 0.3 if top else 0.14)
        if top:
            mix.place('fx', boom(), f, 0.55)

    # the winning route (bars 26-28): one note per route cell, rising toward the centre, ending on D
    if T.ROUTE:
        L = len(T.ROUTE)
        prev = None
        notes = []
        for k, (cell, rt) in enumerate(zip(T.ROUTE, T.ROUTE_T)):
            ch = chord_at(rt)
            base = 62 + int(round(12 * k / max(1, L - 1)))
            if prev is not None:
                dr, dc = cell[0] - prev[0], cell[1] - prev[1]
                base += {(-1, 0): 2, (1, 0): -2, (0, 1): 1, (0, -1): -1}.get((dr, dc), 0)
            m = in_scale(base)
            if k == L - 1:
                m = 74
            notes.append((rt, m))
            prev = cell
        last_t = None
        for rt, m in notes:
            if last_t is not None and abs(rt - last_t) < 1e-6:
                continue
            last_t = rt
            mix.place('seeker', marimba(m), rt, 0.4, 0.2)
            mix.place('flood', strings_stac(m - 12), rt, 0.7, -0.3)
        for bar in (26, 27, 28):
            t = bar * BAR
            for b in range(4):
                mix.place('diver', bass_pick(PROG[bar][0] - 12 + (12 if b % 2 else 0)), t + b * BEAT, 0.45, 0.15)
                mix.place('diver', bass_pick(PROG[bar][0] - 12 + 7), t + b * BEAT + 3 * S16, 0.3, 0.15)
                mix.place('drums', A.kick(), t + b * BEAT, 0.6)
                if b % 2:
                    mix.place('drums', A.snare(0.12), t + b * BEAT, 0.3)
                for s in range(4):
                    mix.place('drums', A.hat(), t + b * BEAT + s * S16, 0.1)
            mix.place('pad', A.pad([m + 12 for m in PROG[bar]], BAR, cut=1800, att=0.1), t, 0.12)
    # bar 29: resolve (Dm add9); bar 30: the pickup into GO (A, strings swell, snare roll)
    t29, t30 = 29 * BAR, 30 * BAR
    mix.place('pad', A.pad([62, 65, 69, 76], 2 * BAR, cut=2200, att=0.05), t29, 0.16)
    mix.place('seeker', marimba(74), t29, 0.4)
    mix.place('fx', chord_stab([62, 65, 69, 74], 0), t29, 0.32)
    mix.place('drums', A.kick(), t29, 0.6)
    mix.place('drums', A.crash(2.5), t29, 0.18)
    for b in range(4):                                     # bar 29 keeps driving under the question
        tb = t29 + b * BEAT
        mix.place('drums', A.kick(), tb, 0.6)
        if b % 2:
            mix.place('drums', A.clap(), tb, 0.24)
        for s_ in range(4):
            mix.place('drums', A.hat(), tb + s_ * S16, 0.1 if s_ % 2 == 0 else 0.06, 0.2)
        mix.place('diver', bass_pick(38 + (12 if b % 2 else 0)), tb, 0.42, 0.15)
        mix.place('flood', strings_stac([62, 69, 74, 69][b]), tb + 2 * S16, 0.8, -0.35)
    for b in range(4):                                     # bar 30: the pickup (kick on 1 and 3, a fill into GO)
        tb = t30 + b * BEAT
        if b % 2 == 0:
            mix.place('drums', A.kick(), tb, 0.55)
        mix.place('drums', A.hat(), tb + 2 * S16, 0.08, 0.2)
    for k, f0 in enumerate([200, 170, 140, 115]):
        mix.place('drums', tom(f0), t30 + 3 * BEAT + k * S16, 0.32)
    mix.place('pad', A.pad([57, 61, 64, 67], BAR + 0.5, cut=1600, att=0.9), t30, 0.16)     # rings over the seam
    sw = A.crash(1.4)[::-1] * np.linspace(0.3, 1, int(1.4 * SR)) ** 2                      # a reversed cymbal into GO
    mix.place('fx', sw, T.DUR - 1.4, 0.55)
    for b in range(4):
        mix.place('diver', bass_pick(45 if b < 3 else 40), t30 + b * BEAT, 0.4, 0.15)
    for s in range(8):
        mix.place('drums', A.snare(0.05), t30 + 2 * BEAT + s * S16 * 0.5 + (s >= 4) * 0.0, 0.08 + 0.025 * s)
    mix.place('fx', riser(BAR * 0.9), t30 + BAR * 0.1, 0.16)

    # caption thuds (quiet)
    for tl, k, n in T.LANDINGS:
        mix.place('fx', A.thud(), tl, 0.14 if k == 0 else 0.09)

    out = master_loop(mix)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), [(0, 2), (2, 8), (8, 20), (20, 39.4), (39.4, 51.2), (51.2, 57), (57, 61)]):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


def master_loop(mix):
    """Mix.master, but as a loop: everything that rings past 61.0 s (reverb included) wraps onto the start, and there
    are no fades at the file edges, so the last sample flows into the first"""
    NS = mix.NS
    gains = {'flood': 1.0, 'diver': 1.0, 'seeker': 1.0, 'drums': 0.9, 'bed': 0.9, 'pad': 1.0, 'fx': 1.0}
    dry = sum(b * gains.get(k, 1.0) for k, b in mix.buses.items())
    send = sum(mix.buses[k] * v for k, v in {'flood': 0.6, 'seeker': 0.5, 'pad': 1.0, 'fx': 0.6}.items() if k in mix.buses)
    wet = A.reverb(send, dur=2.2, decay=0.5) * 0.2
    full = dry + wet
    out = full[:NS].copy()
    tail = full[NS:]
    m = min(len(tail), NS)
    out[:m] += tail[:m]
    g = np.interp(A.tt(NS), [0, 26.0, T.T_FINAL, T.T_WIN or 44.0, T.T_ROUTE0, T.T_ROUTE1, 59.0, T.DUR],
                  [0, 0, 2.2, 3.9, 4.5, 5.5, 4.2, 0])
    out = out * (10 ** (g / 20))[:, None]
    for k in ('flood', 'diver', 'seeker', 'drums', 'bed', 'pad'):
        b = mix.buses[k][:NS]
        print(f'  bus {k:6s} rms {20 * np.log10(np.sqrt((b ** 2).mean()) + 1e-12):6.1f}')
    thr = 0.3
    e2 = (out ** 2).mean(1)
    from scipy.signal import lfilter
    # run the envelope over a wrapped copy so the compressor state at frame 0 matches the end
    pre = int(2 * SR)
    ee = np.sqrt(lfilter([0.002], [1, -0.998], np.concatenate([e2[-pre:], e2]))[pre:])
    gain = np.where(ee > thr, (thr / np.maximum(ee, 1e-9)) ** 0.45, 1.0)
    out = out * gain[:, None]
    out *= 0.89 / (np.abs(out).max() + 1e-9)
    out = np.tanh(out * 1.1) / np.tanh(1.1)
    out *= 0.89 / (np.abs(out).max() + 1e-9)
    return out


if __name__ == '__main__':
    main()
