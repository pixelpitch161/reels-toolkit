"""Bounce Royale soundtrack: mood A, dark synthwave. Four-on-the-floor in A minor at 118.03 BPM (30 bars in 61 s,
so the groove is in phase at the loop point), | Am | F | Dm | E | with an F-E turnaround into the loop.
Everything is synthesised and original, and the run drives it: every sound below comes from the cached run
(sim.npz) through timeline.py.

    every wall bounce      plays its colour's instrument, quantised to 32nds: pitch from how high on the ring it hit
                           (chord tones, low at the bottom, high at the top), pan from which side it hit
        RUBY  drums        snare + clap up top, pitched toms in the middle, low toms at the bottom
        JADE  bass synth   a driving octave bass: each bounce sets the note, 16th pulses until its next bounce
        COBALT arp synth   each bounce fires a 4-note arpeggio up the chord, with a ping-pong echo
        AMBER lead synth   each bounce is a lead note, held until its next bounce
    repaints               an accent (louder, brighter, a zap layer)
    volume                 follows each colour's share of the wall; a pop cuts its instrument out
    the arena (bed)        kick from bar 1, offbeat hats (16ths after the first pop), a pumping dark pad,
                           and the shrinking ring as one long riser that releases at the win
    pops / win             crash + a falling "deflate" in the colour's pitch + glass shards; the win: a big hit,
                           then the winner's solo motif (its bounces step through a motif)
    launches, collisions   a laser "fwip" per launch, a glassy tick per ball-ball collision (quiet)
    loop                   the last bar is a snare roll + riser into frame 0, where the snap-open hits

    python audio.py        # -> audio.wav, prints section levels and the bounces-over-the-bed band check"""
import math
import os
import sys
import numpy as np
from scipy.signal import lfilter
from scipy.ndimage import uniform_filter1d, minimum_filter1d

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit import audio as A
import timeline as T

SR = A.SR
DUR = T.DUR
NS = int(DUR * SR)
BEAT, BAR, S16, S32 = T.BEAT, T.BAR, T.S16, T.S32
NBAR = int(round(DUR / BAR))
RNG = np.random.default_rng(24)

# | Am | F | Dm | E | ..., the last two bars F | E lead back into Am at frame 0
CHORDS = {'Am': [57, 60, 64], 'F': [53, 57, 60], 'Dm': [50, 53, 57], 'E': [52, 56, 59]}
ROOT = {'Am': 33, 'F': 29, 'Dm': 26, 'E': 28}
PROG = ['Am', 'F', 'Dm', 'E']


def chord_at(t):
    b = int((t % DUR) // BAR) % NBAR
    if b == NBAR - 2:
        return 'F'
    if b == NBAR - 1:
        return 'E'
    return PROG[b % 4]


def tt(n):
    return np.arange(n) / SR


def f_of(m):
    return 440.0 * 2 ** ((m - 69) / 12)


class Buses:
    """periodic buses: anything placed past 61 s wraps onto the start, so the loop seam is seamless"""
    def __init__(self):
        self.b = {}

    def get(self, k):
        if k not in self.b:
            self.b[k] = np.zeros((NS + 10 * SR, 2))
        return self.b[k]

    def place(self, k, sig, t, gain=1.0, pan=0.0):
        b = self.get(k)
        i0 = int(round((t % DUR) * SR))
        a = (max(-1.0, min(1.0, pan)) + 1) * math.pi / 4
        st = np.stack([sig * math.cos(a), sig * math.sin(a)], 1) * gain
        n = min(len(st), len(b) - i0)
        b[i0:i0 + n] += st[:n]

    def add(self, k, st):
        self.get(k)[:len(st)] += st

    def fold(self):
        for k, b in self.b.items():
            b[:len(b) - NS] += b[NS:]
            self.b[k] = b[:NS].copy()


_C = {}


def cached(key, fn):
    if key not in _C:
        _C[key] = fn()
    return _C[key]


# ------------------------------------------------------------------ voices
def v_kick():
    return cached('kick', lambda: A.kick(1.0, dec=0.26, f0=170, f1=44) * 0.9)


def v_hat(open_=False):
    return cached(('hat', open_), lambda: A.hat(0.03, open_))


def v_snare(acc):
    def mk():
        n = int(0.35 * SR)
        t = tt(n)
        body = np.sin(2 * np.pi * 185 * t) * np.exp(-t / 0.06) * 0.6
        nz = A.bp(np.random.default_rng(2).normal(0, 1, n), 1200, 7000) * np.exp(-t / 0.13) * 0.8
        x = body + nz
        cl = A.clap()
        if acc:
            x[:len(cl)] += cl * 0.9
        return x
    return cached(('sn', acc), mk)


def v_tom(m):
    def mk():
        n = int(0.5 * SR)
        t = tt(n)
        f0 = f_of(m)
        f = f0 * (1 + 0.6 * np.exp(-t / 0.025))
        x = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.2)
        x += np.sin(2 * np.pi * np.cumsum(f * 2.01) / SR) * np.exp(-t / 0.06) * 0.3
        x += A.bp(np.random.default_rng(4).normal(0, 1, n), 300, 3000) * np.exp(-t / 0.015) * 0.35
        return x
    return cached(('tom', m), mk)


def v_bass_pulse(m, bright):
    def mk():
        n = int(0.12 * SR)
        t = tt(n)
        f = f_of(m)
        x = A.saw(f, n, det=0.008, voices=2) + 0.6 * np.sin(2 * np.pi * f / 2 * t)
        cut0 = 2600 if bright else 1300
        out = np.zeros(n)
        hop = 240
        for i in range(0, n, hop):                      # quick filter envelope (the "pluck" of a synthwave bass)
            c = 220 + cut0 * math.exp(-i / SR / 0.035)
            seg = x[max(0, i - 1024):i + hop]
            out[i:i + hop] = A.lp(seg, c)[-(min(i + hop, n) - i):]
        return out * A.adsr(n, 0.002, 0.05, 0.75, 0.02)
    return cached(('bp', m, bright), mk)


def v_arp(m, acc):
    def mk():
        n = int(0.32 * SR)
        t = tt(n)
        f = f_of(m)
        x = np.where((t * f) % 1.0 < 0.3, 1.0, -1.0) * 0.6 + np.sin(2 * np.pi * f * t) * 0.5
        x = A.lp(x, 4200 if acc else 3000)
        return x * A.expenv(n, 0.09, 0.002)
    return cached(('arp', m, acc), mk)


def v_lead(m, dur, acc):
    key = ('ld', m, round(dur, 2), acc)

    def mk():
        n = int((dur + 0.25) * SR)
        t = tt(n)
        f = f_of(m) * (1 + 0.006 * np.sin(2 * np.pi * 5.5 * t) * np.clip((t - 0.12) / 0.2, 0, 1))
        ph = np.cumsum(f) / SR
        x = sum(2 * ((ph * (1 + d) + o) % 1.0) - 1 for d, o in ((-0.006, 0.1), (0.0, 0.5), (0.006, 0.8))) / 3
        x = A.lp(x, 3800 if acc else 2800)
        env = np.clip(t / 0.012, 0, 1) * (0.7 + 0.3 * np.exp(-t / 0.15))
        env *= np.clip((dur + 0.25 - t) / 0.25, 0, 1)
        return x * env
    return cached(key, mk)


def v_zap(f0=900):
    def mk():
        n = int(0.09 * SR)
        t = tt(n)
        f = f0 * (1 + 2.5 * np.exp(-t / 0.02))
        return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.03)
    return cached(('zap', f0), mk)


def v_fwip():
    def mk():
        n = int(0.16 * SR)
        t = tt(n)
        f = 300 * (6.0 ** (t / 0.16))
        return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.sin(np.pi * t / 0.16) ** 2
    return cached('fwip', mk)


def v_tick():
    def mk():
        n = int(0.06 * SR)
        t = tt(n)
        return (np.sin(2 * np.pi * 2400 * t) + 0.6 * np.sin(2 * np.pi * 3610 * t)) * np.exp(-t / 0.018)
    return cached('tick', mk)


def v_deflate(m):
    n = int(0.9 * SR)
    t = tt(n)
    f = f_of(m) * 2 ** (-2.2 * (t / 0.9) ** 0.7)
    x = A.saw(1, n) * 0
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.4)
    x += A.lp(np.random.default_rng(9).normal(0, 1, n), 900) * np.exp(-t / 0.05) * 0.5
    return x


def v_shards(seed):
    n = int(1.0 * SR)
    t = tt(n)
    r = np.random.default_rng(seed)
    x = np.zeros(n)
    for _ in range(9):
        d = r.uniform(0, 0.35)
        i = int(d * SR)
        fq = r.uniform(3000, 7000)
        tt2 = t[:n - i]
        x[i:] += np.sin(2 * np.pi * fq * tt2) * np.exp(-tt2 / 0.06) * r.uniform(0.3, 1.0)
    return x


def v_boom(dur=1.6, f0=90, f1=32):
    n = int(dur * SR)
    t = tt(n)
    f = f1 + (f0 - f1) * np.exp(-t / 0.07)
    return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / (dur * 0.35)) + A.lp(RNG.normal(0, 1, n), 250) * np.exp(-t / 0.05) * 0.4


def v_stab(ch):
    n = int(2.4 * SR)
    x = sum(A.saw(f_of(m), n, det=0.005, voices=3) for m in ch) / len(ch)
    return A.lp(x, 2400) * A.adsr(n, 0.005, 0.5, 0.4, 1.2)


# ------------------------------------------------------------------ the run -> notes
def share_gain(q, t):
    if T.popped(q, t):
        return 0.0
    c = T.COUNT[T.frame_of(t), q]
    return 0.45 + 0.55 * min(1.6, math.sqrt(max(c, 0.0) / 12.0))


def height_idx(a, n):
    """0..n-1 from how high on the ring the bounce was (bottom = 0)"""
    y = math.sin(a)
    return int(round((y + 1) / 2 * (n - 1)))


def bounce_events():
    """per ball: [(t_quantised, angle, flips, raw t)] with one note per 32nd slot"""
    ev = {q: [] for q in range(4)}
    seen = set()
    for i in range(len(T.HIT_T)):
        t = float(T.HIT_T[i])
        q = int(T.HIT_Q[i])
        if t >= DUR:
            continue
        slot = int(round(t / S32))
        if (q, slot) in seen:
            continue
        seen.add((q, slot))
        ev[q].append((slot * S32, float(T.HIT_A[i]), int(T.HIT_N[i]), t))
    return ev


MOTIF = {   # the winner's solo: its bounces step through these chord-degree offsets (0 = root, 1 = 3rd, 2 = 5th, 3 = octave...)
    0: [0, 2, 1, 3, 2, 4, 3, 1], 1: [0, 0, 3, 2, 0, 4, 3, 2, 0, 1, 2, 3], 2: [0, 1, 2, 3, 4, 3, 2, 1], 3: [3, 4, 2, 3, 1, 2, 0, 1]}


def tones(ch, lo, n):
    """n chord tones from MIDI lo upward"""
    base = CHORDS[ch]
    out, m = [], lo
    while len(out) < n:
        if (m % 12) in [b % 12 for b in base]:
            out.append(m)
        m += 1
    return out


def balls(B):
    ev = bounce_events()
    win_k = 0
    for q in range(4):
        E = ev[q]
        for j, (tq, a, flips, traw) in enumerate(E):
            g = share_gain(q, traw)
            after_win = T.T_WIN is not None and traw >= T.T_WIN
            if g <= 0 and not after_win:
                continue
            if after_win:
                if q != T.WINNER or traw > DUR - 1.2:
                    continue
                g = 1.15 * (1 - T.ease((traw - (DUR - 2.6)) / 1.4))
            acc = flips > 0
            ga = g * (1.35 if acc else 1.0)
            pan = 0.55 * math.cos(a)
            ch = chord_at(tq)
            nxt = E[j + 1][0] if j + 1 < len(E) else tq + BAR
            if q == 0:                                   # RUBY drums
                y = math.sin(a)
                if y > 0.35:
                    B.place('ruby', v_snare(acc), tq, 0.55 * ga, pan)
                else:
                    tm = tones(ch, 40, 5)[height_idx(a, 5)] + (0 if y > -0.35 else -5)
                    B.place('ruby', v_tom(tm), tq, 0.62 * ga, pan)
                if after_win:
                    B.place('ruby', v_tom(tones(ch, 43, 6)[MOTIF[0][win_k % 8]]), tq + S16, 0.5 * ga, -pan)
                    win_k += 1
            elif q == 1:                                 # JADE octave bass
                if after_win:
                    m = tones(ch, ROOT[ch] + 12, 6)[MOTIF[1][win_k % 12]]
                    win_k += 1
                else:
                    m = tones(ch, ROOT[ch] + 12, 4)[height_idx(a, 4)]
                hold = min(nxt - tq, BAR if not after_win else BEAT)
                k = 0
                while k * S16 < hold - 1e-6:
                    tp = tq + k * S16
                    mm = m + (12 if k % 2 else 0)
                    if chord_at(tp) != ch and k > 0:
                        break
                    B.place('jade', v_bass_pulse(mm, acc and k < 2 or after_win), tp, 0.5 * ga * (0.86 ** k if not after_win else 0.95), pan * 0.3)
                    k += 1
                if after_win:
                    B.place('jade', v_lead(m + 24, min(0.4, nxt - tq), True), tq, 0.16 * ga, pan)
            elif q == 2:                                 # COBALT arp
                ts = tones(ch, 57, 9)
                i0 = height_idx(a, 5)
                if after_win:
                    i0 = MOTIF[2][win_k % 8]
                    win_k += 1
                for k in range(4):
                    m = ts[min(len(ts) - 1, i0 + k)]
                    B.place('cobalt', v_arp(m, acc), tq + k * S16, 0.42 * ga * (1 - 0.12 * k), pan)
                    B.place('cobalt', v_arp(m, False), tq + k * S16 + 3 * S16, 0.15 * ga, -pan)
            else:                                        # AMBER lead
                ts = tones(ch, 57, 7)
                m = ts[MOTIF[3][win_k % 8]] if after_win else ts[height_idx(a, 7)]
                if after_win:
                    win_k += 1
                dur = max(0.12, min(1.0, nxt - tq - 0.02))
                B.place('amber', v_lead(m, round(dur / S32) * S32, acc), tq, 0.36 * ga, pan * 0.6)
            if acc and not after_win:
                B.place('fx', v_zap(700 + 300 * q), tq, 0.06 * min(3, flips), pan)
    for q, t in enumerate(T.T_LAUNCH):
        B.place('fx', v_fwip(), t + 0.02, 0.22, [0.5, 0.5, -0.5, -0.5][q])
    for i, t in enumerate(T.COLL_T):
        if t < DUR - 1:
            B.place('fx', v_tick(), t, 0.07, 0.6 * float(T.COLL_P[i][0]))


def bed(B):
    t_end_kick = (NBAR - 1) * BAR                        # the last bar is the roll + riser into the loop
    for b in range(int(round(DUR / BEAT))):
        t = b * BEAT
        if BAR <= t < t_end_kick:
            B.place('kick', v_kick(), t, 0.85)
        if 2 * BAR <= t < t_end_kick:
            B.place('hat', v_hat(), t + BEAT / 2, 0.32)
            dense = t >= T.POPS[0][0] or (T.T_WIN and t >= T.T_WIN)
            if dense:
                B.place('hat', v_hat(), t + S16, 0.12)
                B.place('hat', v_hat(), t + 3 * S16, 0.14)
            if T.POPS[1][0] <= t < (T.T_WIN or 99) and b % 2 == 1:
                for k in range(4):
                    B.place('hat', v_hat(), t + k * S32 + BEAT / 2, 0.08)
    # pumping dark pad
    n = NS
    t = tt(n)
    x = np.zeros(n)
    for b in range(NBAR):
        ch = chord_at(b * BAR + 0.01)
        i0, i1 = int(b * BAR * SR), min(n, int((b + 1) * BAR * SR) + int(0.05 * SR))
        seg = sum(A.saw(f_of(m - 12), i1 - i0, det=0.007, voices=3) for m in CHORDS[ch]) / 3
        seg += 0.5 * np.sin(2 * np.pi * f_of(ROOT[ch]) * tt(i1 - i0))
        w = np.ones(i1 - i0)
        fd = int(0.04 * SR)
        w[:fd] = np.linspace(0, 1, fd)
        w[-fd:] = np.linspace(1, 0, fd)
        x[i0:i1] += seg[:i1 - i0] * w
    x = A.lp(x, 900)
    pump = 1 - 0.65 * np.exp(-((t % BEAT) / 0.11)) * ((t >= BAR) & (t < t_end_kick))
    B.add('pad', np.stack([x * pump, x * pump], 1))


def riser(B):
    """the shrinking ring = one long riser (noise band climbing with the ring), released at the win; plus the
    loop riser and snare roll in the last bar"""
    t0 = T.P.t_shrink0
    t1 = T.T_WIN or 50.0
    n = int((t1 - t0) * SR)
    nz = np.random.default_rng(31).normal(0, 1, n)
    out = np.zeros(n)
    hop = SR // 20
    for i in range(0, n, hop):
        u = i / n
        fc = 350 * (9.0 ** u)
        seg = nz[max(0, i - 2048):i + hop]
        out[i:i + hop] = A.bp(seg, fc * 0.7, fc * 1.4)[-(min(i + hop, n) - i):]
    env = np.clip(np.arange(n) / (2.0 * SR), 0, 1) * (0.25 + 0.75 * (np.arange(n) / n) ** 1.5)
    tone = np.sin(2 * np.pi * np.cumsum(220 * 2 ** (2 * np.arange(n) / n)) / SR) * 0.15
    B.place('riser', (out + tone) * env, t0, 0.55)
    # last bar: a 16th -> 32nd snare roll and a noise riser into the snap at frame 0
    tb = (NBAR - 1) * BAR
    k = 0
    tr = tb
    while tr < DUR - 1e-6:
        u = (tr - tb) / BAR
        B.place('roll', v_snare(False), tr, 0.18 + 0.35 * u)
        tr += S16 if u < 0.5 else S32
        k += 1
    n2 = int(BAR * SR)
    nz2 = np.random.default_rng(32).normal(0, 1, n2)
    r2 = A.hp(nz2, 900) * np.linspace(0, 1, n2) ** 2.5
    B.place('roll', r2, tb, 0.6)


def hits(B):
    for tp, q, p in T.POPS:
        B.place('fx', A.crash(2.0), tp, 0.45)
        B.place('fx', v_deflate([45, 33, 69, 64][q]), tp, 0.55, 0.4 * p[0])
        B.place('fx', v_shards(q), tp, 0.18, 0.4 * p[0])
    if T.T_WIN is not None:
        B.place('fx', v_boom(), T.T_WIN, 0.85)
        B.place('fx', A.crash(3.0), T.T_WIN, 0.5)
        B.place('stab', v_stab([45, 57, 60, 64, 69]), T.T_WIN, 0.5)
    # the snap-open at frame 0 (placed at 61.0: wraps onto the start)
    B.place('fx', v_boom(1.4, 110, 36), DUR, 0.9)
    B.place('fx', A.crash(2.2), DUR, 0.35)
    B.place('fx', v_zap(400), DUR, 0.3)
    th = A.thud()
    for t0, k, nch in T.LANDINGS:
        B.place('fx', th, t0, 0.3 if k == 0 else 0.22)


BUS_GAIN = {'kick': 0.62, 'hat': 0.42, 'pad': 0.26, 'riser': 0.32, 'roll': 0.5, 'ruby': 0.95, 'jade': 0.8, 'cobalt': 0.85,
            'amber': 0.9, 'fx': 0.7, 'stab': 0.6}
REVERB = {'cobalt': 0.45, 'amber': 0.5, 'ruby': 0.2, 'fx': 0.35, 'stab': 0.6, 'pad': 0.3, 'riser': 0.3}
BED = ['kick', 'hat', 'pad', 'riser']
BALLS = ['ruby', 'jade', 'cobalt', 'amber']
TARGET_DB = -13.5


def master(B):
    dry = sum(b * BUS_GAIN.get(k, 1.0) for k, b in B.b.items())
    send = sum(B.b[k] * v * BUS_GAIN.get(k, 1.0) for k, v in REVERB.items() if k in B.b)
    wet = A.reverb(np.concatenate([send, send]), dur=2.4, decay=0.6)[NS:]
    x = dry + wet * 0.22
    win = T.T_WIN or 50.0
    ra, rb = int(16 * SR), int(win * SR)
    ref = np.sqrt((x[ra:rb] ** 2).mean())
    thr = ref * 10 ** (-5 / 20)
    xl = np.concatenate([x[-SR:], x])
    a = math.exp(-1.0 / (0.08 * SR))
    e = np.sqrt(lfilter([1 - a], [1, -a], (xl ** 2).mean(1)).clip(1e-12))[SR:]
    x = x * np.where(e > thr, (thr / e) ** 0.25, 1.0)[:, None]
    ref = np.sqrt((x[ra:rb] ** 2).mean())
    x = x * (10 ** (TARGET_DB / 20) / ref)
    # section gains after the compressor (so the build survives), then only peaks are limited
    pts = [(0, -2.8), (2.0, -3.6), (8, -3.4), (16, -2.6), (24.8, -1.0), (30.0, 0.0), (39.6, 1.4), (win - 0.1, 2.4), (win, 3.2),
           (56.9, 3.0), (59.0, 2.0), (DUR, 0.6)]
    g = np.interp(tt(NS), [p[0] for p in pts], [p[1] for p in pts])
    x = x * (10 ** (g / 20))[:, None]
    lim = 0.88
    gl = np.minimum(1.0, lim / np.maximum(np.abs(x).max(1), 1e-9))
    Lk = int(0.003 * SR)
    gl = uniform_filter1d(minimum_filter1d(gl, 2 * Lk + 1, mode='wrap'), 2 * Lk + 1, mode='wrap')
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
    x = np.tanh(x * 1.02) / np.tanh(1.02) * 0.97
    return np.clip(x, -0.95, 0.95)


def main():
    B = Buses()
    bed(B)
    riser(B)
    balls(B)
    hits(B)
    B.fold()
    stems = {k: v.copy() for k, v in B.b.items()}
    out = master(B)
    path = os.path.join(HERE, 'audio.wav')
    A.write(path, out)
    win = T.T_WIN or 50.0
    p1, p2 = T.POPS[0][0], T.POPS[1][0]
    spans = [(0, 2.03), (2.03, 8.1), (8.1, 16), (16, p1), (p1, p2), (p2, win), (win, 57.0), (57.0, DUR)]
    print('section levels (dBFS rms / peak):')
    for a, b, rms, pk in A.section_levels(path, spans):
        print(f'  {a:5.1f}-{b:5.1f}s  rms {rms:6.1f}  peak {pk:5.1f}')
    from scipy.signal import butter, sosfilt
    balls_ = sum(stems[k] * BUS_GAIN[k] for k in BALLS if k in stems).mean(1)
    bed_ = sum(stems[k] * BUS_GAIN[k] for k in BED if k in stems).mean(1)
    sos = butter(4, [300, 4000], 'bp', fs=SR, output='sos')
    sb, bb = sosfilt(sos, balls_), sosfilt(sos, bed_)
    lv = lambda x: 20 * np.log10(np.sqrt((x ** 2).mean()) + 1e-12)
    for a, b in spans[1:]:
        i, j = int(a * SR), int(b * SR)
        print(f'  bounces over the bed {a:5.1f}-{b:5.1f}s: band 0.3-4 kHz {lv(sb[i:j]) - lv(bb[i:j]):+5.1f} dB, full {lv(balls_[i:j]) - lv(bed_[i:j]):+5.1f} dB')


if __name__ == '__main__':
    main()
