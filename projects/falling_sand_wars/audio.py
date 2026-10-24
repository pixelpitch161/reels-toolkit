"""Falling Sand Wars soundtrack: mood A, dark and hypnotic. Slow minimal techno at 94.43 BPM (24 bars in 61 s, so
the groove is in phase at the loop point) in D Phrygian. Everything is synthesised and original, and the sand
drives it: every sound below comes from the cached run (sim.npz) through timeline.py.

    drone + heartbeat kick     from frame 0, never silent (low D, a fifth, the Phrygian Eb as tension)
    sand ticks                 metallic ticks from grain landings, grouped per 16th: the sand's own voice
    the four parts             each team's activity (landings, slides, drains) in a 16th slot steps its part:
                               IRON low piano strikes + sub, PUMICE glassy bells, CLAY muted frame drum,
                               GLASS plucked dulcimer; volume follows the sand a team still has; out = silent
    turns                      a deep impact hit on every turn (they land on the beat)
    avalanches                 low tom rolls while the grains slide after a turn
    draining streams           a held tone in each team's pitch, as loud as its stream is thick
    knockouts / win            a low drop when a team goes dark; a stark unresolved D minor(b9) chord at the win,
                               then the winner's motif; the closing pour is one long stream tone into the drone
    caption thuds              reelkit.audio.thud() on every impact caption landing

    python audio.py            # -> audio.wav, prints section levels and the sand-over-music band check"""
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
BEAT, S16, BAR = T.BEAT, T.S16, T.BAR
NSLOT = int(round(DUR / S16))                 # 384 sixteenths
D = T.D
FPS = T.FPS
RNG = np.random.default_rng(21)
D2 = 38                                       # D Phrygian: D Eb F G A Bb C


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
        a = (pan + 1) * math.pi / 4
        st = np.stack([sig * math.cos(a), sig * math.sin(a)], 1) * gain
        n = min(len(st), len(b) - i0)
        b[i0:i0 + n] += st[:n]

    def add(self, k, st):
        self.get(k)[:len(st)] += st

    def fold(self):
        for k, b in self.b.items():
            b[:len(b) - NS] += b[NS:]
            self.b[k] = b[:NS].copy()


# ------------------------------------------------------------------ voices
_C = {}


def cached(key, fn):
    if key not in _C:
        _C[key] = fn()
    return _C[key]


def v_piano(m, vel):
    def mk():
        n = int(2.2 * SR)
        t = tt(n)
        f = f_of(m)
        x = sum((1.0 / k ** 1.25) * np.sin(2 * np.pi * f * k * (1 + 0.0004 * k * k) * t) * np.exp(-t / (1.4 / k ** 0.7))
                for k in range(1, 9))
        x += A.bp(RNG.normal(0, 1, n), 300, 2500) * np.exp(-t / 0.006) * 0.3
        sub = np.sin(2 * np.pi * f / 2 * t) * np.exp(-t / 0.7) * 0.8
        return (x * 0.5 + sub) * np.clip(t / 0.002, 0, 1)
    return cached(('pn', m), mk) * vel


def v_bell(m, vel):
    def mk():
        n = int(1.8 * SR)
        t = tt(n)
        f = f_of(m)
        x = np.sin(2 * np.pi * f * t + 1.2 * np.sin(2 * np.pi * f * 3.5 * t) * np.exp(-t / 0.08)) * np.exp(-t / 0.75)
        x += 0.25 * np.sin(2 * np.pi * f * 2.0 * t) * np.exp(-t / 0.35)
        return x * np.clip(t / 0.001, 0, 1)
    return cached(('bl', m), mk) * vel


def v_drum(kind, vel):
    def mk():
        if kind == 'doum':
            n = int(0.45 * SR)
            t = tt(n)
            f = 70 + 55 * np.exp(-t / 0.03)
            x = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.22) + A.lp(RNG.normal(0, 1, n), 400) * np.exp(-t / 0.03) * 0.4
        else:
            n = int(0.2 * SR)
            t = tt(n)
            x = A.bp(RNG.normal(0, 1, n), 900, 3000) * np.exp(-t / 0.035) * 0.8 + np.sin(2 * np.pi * 320 * t) * np.exp(-t / 0.04) * 0.5
        return x
    return cached(('dr', kind), mk) * vel


def v_dulcimer(m, vel):
    """two detuned Karplus-Strong strings, struck"""
    def mk():
        n = int(1.6 * SR)
        out = np.zeros(n)
        for det in (1.0, 1.004):
            L = SR / (f_of(m) * det)
            Li = int(L)
            x = np.zeros(n)
            x[:Li] = RNG.uniform(-1, 1, Li) * np.hanning(Li)
            a = np.zeros(Li + 2)
            a[0] = 1.0
            a[Li] -= 0.497
            a[Li + 1] -= 0.497
            out += lfilter([1.0], a, x)
        out = A.hp(out, 120)
        return out / (np.abs(out).max() + 1e-9)
    return cached(('dc', m), mk) * vel


def v_tick(vel, seed):
    def mk():
        n = int(0.05 * SR)
        t = tt(n)
        r = np.random.default_rng(seed)
        return A.bp(r.normal(0, 1, n), 3800, 7500) * np.exp(-t / 0.006) + np.sin(2 * np.pi * (4700 + 300 * (seed % 5)) * t) * np.exp(-t / 0.012) * 0.6
    return cached(('tk', seed % 8), mk) * vel


def v_turn():
    def mk():
        n = int(2.0 * SR)
        t = tt(n)
        f = 34 + 40 * np.exp(-t / 0.06)
        boom = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.65)
        clang = sum(a * np.sin(2 * np.pi * 146.8 * r * t) * np.exp(-t / d) for a, r, d in [(0.5, 1.0, 0.9), (0.35, 2.41, 0.6), (0.25, 3.93, 0.35), (0.15, 5.4, 0.2)])
        thud = A.lp(RNG.normal(0, 1, n), 300) * np.exp(-t / 0.05)
        x = boom + clang * 0.35 + thud * 0.5
        return x * np.clip(t / 0.003, 0, 1)
    return cached('turn', mk)


def v_tom(m, vel):
    def mk():
        n = int(0.4 * SR)
        t = tt(n)
        f0 = f_of(m)
        f = f0 * (1 + 0.5 * np.exp(-t / 0.02))
        return (np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.16) + A.bp(RNG.normal(0, 1, n), 200, 1200) * np.exp(-t / 0.02) * 0.3)
    return cached(('tm', m), mk) * vel


def v_heart(vel, dub=False):
    def mk():
        n = int(0.5 * SR)
        t = tt(n)
        f = (52 if not dub else 60) + 30 * np.exp(-t / 0.025)
        x = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / (0.14 if not dub else 0.1))
        x += A.lp(RNG.normal(0, 1, n), 180) * np.exp(-t / 0.02) * 0.3
        return x * np.clip(t / 0.003, 0, 1)
    return cached(('hb', dub), mk) * vel


# ------------------------------------------------------------------ the sand -> slots
def slot_activity():
    """per 16th slot and team: activity = landings + slides/4 + drains*2 (from the cached run)"""
    f_t = np.arange(T.NF) / FPS
    sl = np.minimum((f_t / S16).astype(int), NSLOT - 1)
    act = D['landed'].astype(float) + D['moves'].astype(float) / 4 + D['drained'].astype(float) * 2
    out = np.zeros((NSLOT, 4))
    for q in range(4):
        out[:, q] = np.bincount(sl, weights=act[:, q], minlength=NSLOT)[:NSLOT]
    land = np.zeros(NSLOT)
    land += np.bincount(sl, weights=D['landed'].sum(1).astype(float), minlength=NSLOT)[:NSLOT]
    moves = np.bincount(sl, weights=D['moves'].sum(1).astype(float), minlength=NSLOT)[:NSLOT]
    return out, land, moves


def team_gain(q, t):
    """volume follows how much sand the team still has; out = silent"""
    if T.out_now(q, t):
        return 0.0
    f = T.frame_of(t)
    c = T.COUNT[f, q] / T.NEACH
    if t < T.T_VALVE_SHUT:
        c = max(c, 0.25)
    return 0.4 + 0.6 * min(1.0, c) ** 0.5


# 2-bar (32-step) patterns: (step, midi or kind, velocity)
IRON_P = [(0, D2, 1.0), (3, D2, 0.6), (6, D2 + 1, 0.8), (8, D2, 0.9), (12, D2 + 3, 0.7), (16, D2, 1.0), (19, D2, 0.6),
          (22, D2 + 1, 0.8), (24, D2 + 5, 0.7), (26, D2 + 3, 0.6), (28, D2 + 1, 0.8)]
PUM_P = [(0, 86, 0.9), (3, 81, 0.6), (6, 89, 0.7), (10, 87, 0.6), (13, 81, 0.5), (16, 84, 0.8), (19, 81, 0.6),
         (22, 89, 0.7), (26, 87, 0.7), (29, 86, 0.5)]
CLAY_P = [(0, 'doum', 1.0), (4, 'tek', 0.6), (6, 'doum', 0.7), (10, 'doum', 0.8), (12, 'tek', 0.7), (14, 'tek', 0.45),
          (16, 'doum', 1.0), (20, 'tek', 0.6), (22, 'doum', 0.7), (24, 'tek', 0.5), (26, 'doum', 0.8), (28, 'tek', 0.7), (30, 'tek', 0.5)]
GLASS_P = [(s, m, v) for s, m, v in zip(range(0, 32, 2), [62, 65, 69, 72, 70, 69, 67, 65, 62, 63, 65, 69, 67, 63, 62, 60],
                                         [0.9, 0.6, 0.7, 0.6, 0.8, 0.6, 0.7, 0.5] * 2)]
PATTERNS = [IRON_P, PUM_P, CLAY_P, GLASS_P]
PAN = [-0.15, 0.35, 0.05, -0.35]
ENTER = [6.0, 0.0, 9.0, 0.0]                  # the bass motif (iron) enters at 6 s, the frame drum at 9 s


def voice(q, x, vel):
    if q == 0:
        return v_piano(x, vel)
    if q == 1:
        return v_bell(x, vel)
    if q == 2:
        return v_drum(x, vel)
    return v_dulcimer(x, vel)


def parts(B, act):
    """every slot: a team's step plays if the team was active in that slot"""
    thr = [3.0, 3.0, 3.0, 3.0]
    for s in range(NSLOT):
        t = s * S16
        win = T.T_WIN is not None and t >= T.T_WIN
        for q in range(4):
            if t < ENTER[q] or T.out_now(q, t):
                continue
            step = s % 32
            hits = [p for p in PATTERNS[q] if p[0] == step]
            if not hits:
                continue
            on = act[s, q] >= thr[q]
            if win:
                on = (q == T.WINNER) and t < T.T_ROLL0 + 1.5          # the winner's motif plays alone
            if not on:
                continue
            g = team_gain(q, t) if not win else 0.85
            if t < 9.0:
                g *= 0.45 + 0.55 * t / 9.0                              # opens sparse, builds
            for _, x, vel in hits:
                B.place('p%d' % q, voice(q, x, vel), t, g, PAN[q])


def ticks(B, land):
    for s in range(NSLOT):
        n = land[s]
        if n < 1:
            continue
        g = min(1.0, math.log1p(n) / math.log1p(220)) * 0.55
        B.place('tick', v_tick(1.0, s), s * S16 + 0.004 * (s % 3), g, 0.25 * math.sin(s * 1.7))


def heartbeat(B):
    for b in range(int(round(DUR / BEAT))):
        t = b * BEAT
        vel = 0.75 if b % 4 == 0 else 0.6
        B.place('heart', v_heart(vel), t, 1.0)
        B.place('heart', v_heart(vel * 0.55, True), t + S16, 1.0)
        if 34.0 <= t < 44.5 or (T.T_WIN is not None and 44.5 <= t < T.T_WIN):     # rising pulse: off-beat thumps
            B.place('heart', v_heart(0.35), t + 2 * S16, 1.0)


def drone(B):
    n = NS
    t = tt(n)
    cyc = lambda k: np.sin(2 * np.pi * k * t / DUR)
    x = np.zeros(n)
    for m, a in [(26, 0.6), (38, 0.5), (45, 0.25), (50, 0.15)]:
        f = f_of(m)
        x += a * (2 * ((t * f * (1 + 0.002 * cyc(3))) % 1.0) - 1)
    # filter opening slowly through the pour, breathing on whole cycles (loop-safe)
    out = np.zeros(n)
    hop = SR // 20
    cut = 220 + 140 * (0.5 + 0.5 * cyc(2)) + 260 * np.clip((t - 6) / 14, 0, 1) * (t < T.T_WIN if T.T_WIN else 1)
    for i in range(0, n, hop):
        seg = x[max(0, i - 2048):i + hop]
        out[i:i + hop] = A.lp(seg, float(cut[min(i, n - 1)]))[-(min(i + hop, n) - i):]
    eb = np.sin(2 * np.pi * f_of(51) * t) * 0.08 * (0.5 + 0.5 * cyc(4)) ** 2         # Phrygian Eb shimmer
    sig = out * 0.55 + eb
    B.add('drone', np.stack([sig, sig], 1))


def turns(B):
    for t0 in T.TURNS:
        B.place('fx', v_turn(), t0, 0.9)
    B.place('fx', v_turn(), T.T_HOLE, 1.25)
    B.place('fx', A.whoosh(0.9, 200, 3000, seed=4), T.T_HOLE - 0.85, 0.25)
    if T.T_WIN is None or T.T_ROLL0 > T.T_WIN:
        B.place('fx', v_turn(), T.T_ROLL0, 0.8)


def toms(B, moves):
    """low tom rolls while the grains slide after a turn (moves spike)"""
    sm = uniform_filter1d(moves, 3)
    base = np.median(sm[int(22 / S16):]) + 1
    toms = [D2 + 12, D2 + 8, D2 + 5, D2 + 3]
    for s in range(int(T.T_HOLE / S16), NSLOT):
        t = s * S16
        if T.T_WIN is not None and t >= T.T_WIN:
            break
        k = sm[s] / base
        if k > 2.2:
            vel = min(1.0, (k - 2.2) / 6) * 0.8 + 0.15
            B.place('tom', v_tom(toms[s % 4], vel), t, 0.7, 0.3 * math.sin(s))


def drain_tones(B):
    """each draining stream = a held tone in its team's pitch, as loud as the stream is thick"""
    pitch = [D2, D2 + 31, D2 + 19, D2 + 27]          # IRON D2, PUMICE A4, CLAY A3, GLASS F4
    t = tt(NS)
    fr = D['drained'].astype(float)
    for q in range(4):
        r = uniform_filter1d(fr[:, q], 6, mode='wrap') * FPS          # grains / s
        env = np.clip(r / 400.0, 0, 1) ** 0.6
        e = np.interp(t, (np.arange(T.NF) + 0.5) / FPS, env)
        f = f_of(pitch[q])
        tone = np.sin(2 * np.pi * f * t) + 0.3 * np.sin(2 * np.pi * 2 * f * t) + 0.12 * np.sin(2 * np.pi * 3 * f * t)
        hiss = A.bp(np.random.default_rng(30 + q).normal(0, 1, NS), 1500 + 600 * q, 4500 + 500 * q) * 0.35
        x = (tone * 0.55 + hiss) * e
        B.add('drain', np.stack([x * (1 - 0.2 * q / 3), x * (0.8 + 0.2 * q / 3)], 1))


def knockouts(B):
    for t0, q in T.OUTS:
        n = int(1.4 * SR)
        tq = tt(n)
        f = f_of([D2 + 12, D2 + 31, D2 + 19, D2 + 27][q]) * np.exp(-tq / 0.5)
        drop = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-tq / 0.45)
        B.place('fx', drop, t0, 0.35, PAN[q])
        B.place('fx', v_heart(0.9), t0, 0.8)


def final_chord(B):
    if T.T_WIN is None:
        return
    n = int(5.6 * SR)
    t = tt(n)
    ch = [D2, D2 + 3, D2 + 7, D2 + 12, D2 + 13]       # D minor with the b9 (Eb): stark, unresolved
    pad = sum(A.saw(f_of(m), n, det=0.004, voices=3) for m in ch) / len(ch)
    pad = A.lp(pad, 900) * A.adsr(n, 0.02, 0.4, 0.7, 2.5)
    B.place('chord', pad, T.T_WIN, 0.55)
    for m in ch[:4]:
        B.place('chord', v_piano(m, 0.7), T.T_WIN, 0.35)
    B.place('fx', v_turn(), T.T_WIN, 0.7)


def thuds(B):
    th = A.thud()
    for t0, k, nch in T.LANDINGS:
        B.place('fx', th, t0, 0.42 if k == 0 else 0.3)


BUS_GAIN = {'drone': 0.55, 'heart': 0.8, 'tick': 0.5, 'p0': 0.55, 'p1': 0.3, 'p2': 0.55, 'p3': 0.42, 'fx': 0.6,
            'tom': 0.5, 'drain': 0.33, 'chord': 0.6}
REVERB = {'p1': 0.6, 'p3': 0.4, 'fx': 0.35, 'chord': 0.7, 'drain': 0.3, 'tick': 0.15}
MUSIC = ['drone', 'heart', 'chord']                    # "music" bed (the sand voices must sit on top of it)
SAND_BUSES = ['tick', 'p0', 'p1', 'p2', 'p3', 'drain', 'tom']
TARGET_DB = -11.5


def master(B):
    dry = sum(b * BUS_GAIN.get(k, 1.0) for k, b in B.b.items())
    send = sum(B.b[k] * v * BUS_GAIN.get(k, 1.0) for k, v in REVERB.items() if k in B.b)
    wet = A.reverb(np.concatenate([send, send]), dur=2.6, decay=0.7)[NS:]
    x = dry + wet * 0.24
    win = T.T_WIN or 52.0
    pts = [(0, -6.5), (3, -5.8), (9, -4.8), (T.T_HOLE - 0.1, -3.2), (T.T_HOLE, -1.2), (29, -1.0), (34, 1.2), (44, 2.2), (win, 2.8),
           (T.T_ROLL0, 2.6), (59.6, 2.0), (DUR, -6.5)]
    g = np.interp(tt(NS), [p[0] for p in pts], [p[1] for p in pts])
    x = x * (10 ** (g / 20))[:, None]
    ra, rb = int(22 * SR), int(win * SR)
    ref = np.sqrt((x[ra:rb] ** 2).mean())
    thr = ref * 10 ** (-5 / 20)
    xl = np.concatenate([x[-SR:], x])
    a = math.exp(-1.0 / (0.08 * SR))
    e = np.sqrt(lfilter([1 - a], [1, -a], (xl ** 2).mean(1)).clip(1e-12))[SR:]
    x = x * np.where(e > thr, (thr / e) ** 0.25, 1.0)[:, None]
    ref = np.sqrt((x[ra:rb] ** 2).mean())
    x = x * (10 ** (TARGET_DB / 20) / ref)
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
    act, land, moves = slot_activity()
    drone(B)
    heartbeat(B)
    ticks(B, land)
    parts(B, act)
    turns(B)
    toms(B, moves)
    drain_tones(B)
    knockouts(B)
    final_chord(B)
    thuds(B)
    B.fold()
    stems = {k: v.copy() for k, v in B.b.items()}
    out = master(B)
    path = os.path.join(HERE, 'audio.wav')
    A.write(path, out)
    win = T.T_WIN or 52.0
    spans = [(0, 3), (3, 9), (9, 20.3), (20.3, 22.9), (22.9, 34), (34, 44), (44, win), (win, 57.2), (57.2, DUR)]
    print('section levels (dBFS rms / peak):')
    for a, b, rms, pk in A.section_levels(path, spans):
        print(f'  {a:5.1f}-{b:5.1f}s  rms {rms:6.1f}  peak {pk:5.1f}')
    # the sand's voices over the music bed, in the tick band and overall
    from scipy.signal import butter, sosfilt
    sand = sum(stems[k] * BUS_GAIN[k] for k in SAND_BUSES if k in stems).mean(1)
    bed = sum(stems[k] * BUS_GAIN[k] for k in MUSIC if k in stems).mean(1)
    sos = butter(4, [3000, 7000], 'bp', fs=SR, output='sos')
    sb, bb = sosfilt(sos, sand), sosfilt(sos, bed)
    lv = lambda x: 20 * np.log10(np.sqrt((x ** 2).mean()) + 1e-12)
    for a, b in spans:
        i, j = int(a * SR), int(b * SR)
        print(f'  sand over music {a:5.1f}-{b:5.1f}s: band 3-7 kHz {lv(sb[i:j]) - lv(bb[i:j]):+5.1f} dB, full {lv(sand[i:j]) - lv(bed[i:j]):+5.1f} dB')


if __name__ == '__main__':
    main()
