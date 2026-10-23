"""Battle of the Bands soundtrack. Every band note comes from music.notes() (the same list that drives the
cone pumps and the sparkles), i.e. from the moves of the cached battle:

    SAND = drums   BALL = lead   RIPPLE = bass   VINE = keys

Each band's volume follows its share of the stage (smoothed); a knocked-out band powers down (pitch dive +
low-pass) and drops out; the others swell so the mix never thins. Around the band: the boxing bell at 20 s,
the crowd (murmur, "ooh" on knockouts, cheer on the win), the win hit, the crack + falling tiles at 58 s and a
riser into the opening burst. All synthesised; the arrangement is original. Buses are folded into one periodic
60 s buffer with circular reverb so the loop seam is clean.

    python audio.py        # -> audio.wav + section levels + band margins + quietest window"""
import math
import os
import sys
import numpy as np
from scipy.signal import lfilter, sosfilt, butter
from scipy.ndimage import minimum_filter1d, uniform_filter1d

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit import audio as A
import timeline as T
import music as MU

SR = A.SR
NS = int(T.DUR * SR)
RNG = np.random.default_rng(7)
TARGET_DB = -11.5           # RMS of the solo section after mastering


def tt(n):
    return np.arange(n) / SR


def env(n, dec, att=0.002):
    t = tt(n)
    return np.clip(t / att, 0, 1) * np.exp(-t / dec)


def f_of(m):
    return 440.0 * 2 ** ((m - 69) / 12)


# ------------------------------------------------------------------ voices
def v_kick(vel):
    n = int(0.45 * SR)
    t = tt(n)
    f = 50 + 120 * np.exp(-t / 0.035)
    body = np.sin(2 * math.pi * np.cumsum(f) / SR) * np.exp(-t / 0.24)
    thock = np.sin(2 * math.pi * 180 * t) * np.exp(-t / 0.045) * 0.5
    click = A.hp(RNG.normal(0, 1, n), 3000) * np.exp(-t / 0.004) * 0.4
    return (body + thock + click) * vel


def v_snare(vel, ghost=False):
    n = int(0.3 * SR)
    t = tt(n)
    noise = A.bp(RNG.normal(0, 1, n), 1800, 9000) * np.exp(-t / (0.07 if ghost else 0.13))
    tone = (np.sin(2 * math.pi * 200 * t) + 0.5 * np.sin(2 * math.pi * 330 * t)) * np.exp(-t / 0.05)
    return (noise * 0.8 + tone * 0.6) * vel * (0.35 if ghost else 1.0)


def v_hat(vel, open_=False):
    n = int((0.32 if open_ else 0.07) * SR)
    t = tt(n)
    x = A.hp(RNG.normal(0, 1, n), 7500) * np.exp(-t / (0.11 if open_ else 0.022))
    return x * vel * 0.55


def v_tom(m, vel):
    n = int(0.4 * SR)
    t = tt(n)
    f = f_of(m) * (1 + 0.5 * np.exp(-t / 0.03))
    body = np.sin(2 * math.pi * np.cumsum(f) / SR) * np.exp(-t / 0.2)
    skin = A.bp(RNG.normal(0, 1, n), 400, 3000) * np.exp(-t / 0.02) * 0.3
    return (body + skin) * vel


def v_steel(m, dur, vel):
    """the drummer's tuned voice (solo melody): bright pitched drum, steel-pan like"""
    n = int(max(dur, 0.35) * SR)
    t = tt(n)
    f = f_of(m)
    x = sum(a * np.sin(2 * math.pi * f * r * t + r) * np.exp(-t / d) for r, a, d in
            ((1, 1.0, 0.45), (2, 0.5, 0.25), (3.02, 0.22, 0.12), (4.1, 0.12, 0.06)))
    x += A.bp(RNG.normal(0, 1, n), 1500, 6000) * np.exp(-t / 0.008) * 0.25
    return x * np.clip(t / 0.002, 0, 1) * vel


def v_lead(m, dur, vel):
    n = int((dur + 0.12) * SR)
    t = tt(n)
    f = f_of(m) * (1 + 0.006 * np.sin(2 * math.pi * 5.5 * t) * np.clip((t - 0.12) / 0.2, 0, 1))
    ph = np.cumsum(f) / SR
    x = 0.55 * (2 * (ph % 1) - 1) + 0.45 * np.where((ph * 1.003) % 1 < 0.42, 1.0, -1.0)
    x = A.lp(x, 3800)
    e = A.adsr(n, 0.006, 0.12, 0.7, 0.1)
    return x * e * vel


def v_bass(m, dur, vel):
    n = int((dur + 0.06) * SR)
    t = tt(n)
    f = f_of(m)
    ph = np.cumsum(np.full(n, f)) / SR
    saw = 2 * (ph % 1) - 1
    sub = np.sin(2 * math.pi * ph)
    # filter envelope: bright attack (phone-audible harmonics), then darker
    k = np.exp(-t / 0.07)
    y = A.lp(saw, 2600) * k + A.lp(saw, 320) * (1 - k)
    x = 0.65 * y + 0.7 * sub
    return x * A.adsr(n, 0.004, 0.08, 0.75, 0.05) * vel


def v_keys(m, dur, vel):
    n = int(max(0.32, dur + 0.25) * SR)
    t = tt(n)
    f = f_of(m)
    mod = np.sin(2 * math.pi * f * 2 * t) * 1.6 * np.exp(-t / 0.08)
    x = np.sin(2 * math.pi * f * t + mod) * np.exp(-t / 0.28) + 0.25 * np.sin(2 * math.pi * f * 4 * t) * np.exp(-t / 0.05)
    return x * np.clip(t / 0.002, 0, 1) * vel


_CACHE = {}


def voice(q, kind, m, dur, vel):
    key = (q, kind, m, round(dur, 3), round(vel, 2))
    if key in _CACHE:
        return _CACHE[key]
    if kind == 'kick':
        x = v_kick(vel)
    elif kind in ('snare', 'ghost'):
        x = v_snare(vel, kind == 'ghost')
    elif kind in ('hat', 'ohat'):
        x = v_hat(vel, kind == 'ohat')
    elif kind == 'tom':
        x = v_tom(m, vel)
    elif kind == 'solo':
        x = {0: v_steel, 1: v_lead, 2: v_bass, 3: v_keys}[q](m, dur, vel)
    elif kind == 'lead':
        x = v_lead(m, dur, vel)
    elif kind == 'bass':
        x = v_bass(m, dur, vel)
    else:
        x = v_keys(m, dur, vel)
    if len(_CACHE) < 6000:
        _CACHE[key] = x
    return x


PAN = [-0.12, 0.22, 0.0, -0.3]
BUS_GAIN = {'sand': 0.75, 'ball': 0.62, 'ripple': 1.15, 'vine': 0.36, 'fx': 0.8, 'crowd': 0.55}
BUSN = ['sand', 'ball', 'ripple', 'vine']


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


# ------------------------------------------------------------------ the band
def band(B):
    notes = MU.notes()
    for n in notes:
        q = int(n['team'])
        g = MU.gains(min(n['t'], 59.99))[q] if n['layer'] != 'solo' else 1.0
        if n['layer'] == 'grid' and T.T_WIN is not None and T.T_WIN <= n['t'] < T.T_RESTART and q == T.WINNER:
            g = 1.0
        if n['layer'] == 'free':
            g = 0.55
        x = voice(q, n['kind'], int(n['midi']), float(n['dur']), float(n['vel']))
        extra = 0.75 if n['layer'] == 'solo' else 1.0
        B.place(BUSN[q], x, float(n['t']), g * extra, PAN[q])


def seg_tape_stop(x, t0, length):
    """pitch-down-to-silence over [t0, t0 + length] on a stereo buffer, leaving the rest untouched"""
    i0, n = int(t0 * SR), int(length * SR)
    seg = x[i0:i0 + 2 * n].copy()
    rate = np.linspace(1, 0, n) ** 1.3
    pos = np.cumsum(rate)
    out = np.stack([np.interp(pos, np.arange(len(seg)), seg[:, c]) for c in range(2)], 1)
    x[i0:i0 + n] = out * np.linspace(1, 0, n)[:, None] ** 0.6


def power_down(B):
    """knocked out: the band's sound dives in pitch and closes up over 0.9 s; crowd 'ooh', boom"""
    for t0, q in T.OUTS:
        bar = int(t0 // T.BAR)
        root = MU.chord(bar)[0]
        m = {0: 45, 1: root + 12, 2: root - 24, 3: root}[q]
        n = int(1.1 * SR)
        tq = tt(n)
        f = f_of(m) * 2 ** (-2.5 * (tq / 1.1) ** 1.4)
        ph = np.cumsum(f) / SR
        if q == 0:
            x = np.sin(2 * math.pi * ph) * 0.8 + A.bp(RNG.normal(0, 1, n), 300, 3000) * 0.3 * ((tq * 14) % 1 < 0.3)
        elif q == 2:
            x = 0.7 * (2 * (ph % 1) - 1) + 0.6 * np.sin(2 * math.pi * ph)
        else:
            x = 0.6 * (2 * (ph % 1) - 1) + 0.4 * np.sin(2 * math.pi * 2 * ph)
        x = A.lp(x, 2500) * (1 - tq / 1.1) ** 1.2
        B.place(BUSN[q], x, t0, 0.9, PAN[q])
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
    f = 1180
    return sum(a * np.sin(2 * math.pi * f * r * t) * np.exp(-t / d) for r, a, d in
               ((1, 1.0, 0.6), (2.41, 0.6, 0.35), (3.88, 0.35, 0.2), (5.4, 0.2, 0.12), (0.5, 0.25, 0.8)))


def crowd_swell(dur, kind='cheer', seed=0):
    r = np.random.default_rng(11 + seed)
    n = int(dur * SR)
    t = tt(n)
    x = r.normal(0, 1, n)
    if kind == 'ooh':
        fc = 900 * np.exp(-t / 1.0) + 350
        e = np.sin(np.pi * np.clip(t / dur, 0, 1)) ** 1.5
    else:
        fc = 1400 + 300 * np.sin(2 * math.pi * 3.1 * t)
        e = np.clip(t / 0.25, 0, 1) * np.exp(-np.clip(t - 0.6, 0, None) / (dur * 0.45))
    y = np.zeros(n)
    for lo, hi, g in ((0.5, 1.4, 1.0), (1.8, 2.9, 0.5)):
        seg = A.bp(x, 300, 3500)
        y += seg * g
    # many voices: amplitude flutter
    flutter = 1 + 0.3 * np.sin(2 * math.pi * 7 * t + r.uniform(0, 6)) * np.sin(2 * math.pi * 2.3 * t)
    return A.lp(y, 2400) * e * flutter * 0.5


def crowd_bed(B):
    r = np.random.default_rng(3)
    x = r.normal(0, 1, NS)
    y = A.bp(x, 250, 2200)
    y = y * (1 + 0.25 * np.sin(2 * math.pi * tt(NS) / 3.0) * np.sin(2 * math.pi * tt(NS) / 7.0))
    B.place('crowd', y * 0.06, 0.0, 1.0)


def fx(B):
    # lock-in riser 2.5 -> 4.0 and a hit when the parts land on the grid
    B.place('fx', A.whoosh(1.5, 400, 7000), MU.LOCK1 - 1.5, 0.35)
    B.place('fx', v_kick(1.0), MU.LOCK1, 0.5)
    # the bell (two dings) + crowd at 20 s
    B.place('fx', bell(), T.T_BELL, 0.45)
    B.place('fx', bell(), T.T_BELL + 0.22, 0.38)
    B.place('fx', boom(), T.T_BELL, 0.6)
    B.place('fx', A.whoosh(1.0, 300, 6000), T.T_BELL - 1.0, 0.3)
    B.place('crowd', crowd_swell(2.5, 'cheer', 1), T.T_BELL + 0.05, 0.9)
    # the final duel: a riser from the second knockout into the win
    if len(T.OUTS) >= 2 and T.T_WIN is not None:
        t2 = T.OUTS[1][0] + 2.0
        dur = T.T_WIN - t2
        if dur > 1.0:
            n = int(dur * SR)
            ri = A.whoosh(dur, 300, 6000) * np.linspace(0.2, 1.0, n) ** 2
            B.place('fx', ri, t2, 0.55)
    # the win: big hit, cheer, fireworks pops
    if T.T_WIN is not None:
        n = int(2.4 * SR)
        tq = tt(n)
        hit = A.hp(RNG.normal(0, 1, n), 2500) * np.exp(-tq / 0.6) * 0.6 + boom(2.4)[:n]
        B.place('fx', hit, T.T_WIN, 0.75)
        B.place('crowd', crowd_swell(5.0, 'cheer', 2), T.T_WIN + 0.05, 1.3)
        for k, dt in enumerate([0.35, 1.4, 2.6, 3.9, 5.2, 6.4]):
            m = int(0.5 * SR)
            pop = A.bp(RNG.normal(0, 1, m), 800, 6000) * np.exp(-tt(m) / 0.05) + A.hp(RNG.normal(0, 1, m), 5000) * np.exp(-tt(m) / 0.25) * 0.3
            B.place('fx', pop, T.T_WIN + dt, 0.3, pan=RNG.uniform(-0.6, 0.6))
    # the crack at 58 s, falling tiles, riser into the opening burst (wraps over the seam)
    m = int(0.6 * SR)
    crack = A.hp(RNG.normal(0, 1, m), 2000) * np.exp(-tt(m) / 0.03) + sum(np.sin(2 * math.pi * f * tt(m)) * np.exp(-tt(m) / 0.15) * 0.2 for f in (2300, 3700, 5100))
    B.place('fx', crack, T.T_CRACK, 0.55)
    for k in range(70):
        tk = T.T_CRACK + 0.12 + RNG.uniform(0, 0.5)
        mm = int(0.06 * SR)
        tick = np.sin(2 * math.pi * RNG.uniform(1800, 4200) * tt(mm)) * np.exp(-tt(mm) / 0.01)
        B.place('fx', tick, tk, 0.12, pan=RNG.uniform(-0.7, 0.7))
    B.place('fx', A.whoosh(2.1, 200, 9000) * np.linspace(0.3, 1.0, int(2.1 * SR)) ** 2, T.T_RESTART - 2.0, 0.5)
    B.place('fx', boom(0.9), T.T_RESTART, 0.45)        # wraps to 0.0: the seeds drop on the boom


REVERB = {'ball': 0.5, 'vine': 0.6, 'ripple': 0.1, 'sand': 0.15, 'fx': 0.5, 'crowd': 0.3}


def level_band(B, max_db=3.5, win=1.5):
    """slow leveler on the four band buses (circular): sections where parts stall keep their energy; the
    stutters themselves stay audible because the gain moves over ~1.5 s, not per note"""
    band = sum(B.b[k] * BUS_GAIN[k] for k in BUSN if k in B.b).mean(1)
    hop = SR // 50
    e = np.sqrt(uniform_filter1d(band ** 2, int(win * SR), mode='wrap')[::hop] + 1e-12)
    ref = np.sqrt((band[int((T.T_BELL + 2) * SR):int((T.T_WIN or 50) * SR)] ** 2).mean())
    g = np.clip(0.8 * ref / e, 1.0, 10 ** (max_db / 20))
    g = uniform_filter1d(g, 25, mode='wrap')
    gs = np.interp(np.arange(NS), np.arange(len(g)) * hop, g)
    for k in BUSN:
        if k in B.b:
            B.b[k] *= gs[:, None]


def master(B):
    dry = sum(b * BUS_GAIN.get(k, 1.0) for k, b in B.b.items())
    send = sum(B.b[k] * v * BUS_GAIN.get(k, 1.0) for k, v in REVERB.items() if k in B.b)
    wet = A.reverb(np.concatenate([send, send]), dur=2.2, decay=0.55)[NS:]
    x = dry + wet * 0.2
    # the build: fill a little under the fight, the duel and the solo on top
    pts = [(0, -0.5), (MU.LOCK0, -1.0), (MU.LOCK1, -2.0), (T.T_BELL - 0.2, -1.5), (T.T_BELL, -0.4),
           (T.OUTS[0][0] if T.OUTS else 31, 0.0),
           ((T.T_WIN or 50) - 0.1, 0.6), (T.T_WIN or 50, 1.2), (T.T_CRACK, 1.2), (60.0, -0.5)]
    g = np.interp(tt(NS), [p[0] for p in pts], [p[1] for p in pts])
    x = x * (10 ** (g / 20))[:, None]
    ref_a, ref_b = (T.T_WIN or 50) + 1.0, T.T_CRACK
    # glue compressor (80 ms RMS), wrapped around the loop
    ref = np.sqrt((x[int(ref_a * SR):int(ref_b * SR)] ** 2).mean())
    thr = ref * 10 ** (-6 / 20)
    xl = np.concatenate([x[-SR:], x])
    a = math.exp(-1.0 / (0.08 * SR))
    e = np.sqrt(lfilter([1 - a], [1, -a], (xl ** 2).mean(1)).clip(1e-12))[SR:]
    x = x * np.where(e > thr, (thr / e) ** 0.35, 1.0)[:, None]
    ref = np.sqrt((x[int(ref_a * SR):int(ref_b * SR)] ** 2).mean())
    x = x * (10 ** (TARGET_DB / 20) / ref)
    # lookahead peak limiter (3 ms), ~60 ms release, wrapped
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


def main():
    B = Buses()
    band(B)
    for k in BUSN:                                    # the crack: the winner's solo tape-stops into the new run
        seg_tape_stop(B.get(k), T.T_CRACK, T.T_RESTART - T.T_CRACK)
    for tl, q in MU.lead_changes():                      # the crowd reacts to every lead change
        B.place('crowd', crowd_swell(1.4, 'cheer', 5 + int(tl)), tl, 0.5)
    power_down(B)
    crowd_bed(B)
    fx(B)
    B.fold()
    level_band(B)
    stems = {k: v.copy() for k, v in B.b.items()}
    out = master(B)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    # report
    print('section levels (dBFS rms / peak):')
    spans = [(0, 2.4), (2.4, 9.4), (9.4, 15.4), (15.4, 21.4), (21.4, 23.4), (23.4, 31.1), (31.1, 42.5), (42.5, 51.6),
             (51.6, 59.4), (59.4, 60)]
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'  {a:4.1f}-{b:4.1f}s  rms {rms:6.1f}  peak {pk:5.1f}')
    x = out.mean(1)
    w = int(0.5 * SR)
    r = np.array([np.sqrt((x[i:i + w] ** 2).mean()) for i in range(0, NS - w, w)])
    print(f'quietest 0.5 s window: {20 * np.log10(r.min() + 1e-12):.1f} dBFS at {r.argmin() * 0.5:.1f} s; loudest {20 * np.log10(r.max()):.1f}')
    # band vs crowd/fx bed (the band is the "science sound": it must sit on top)
    bandsum = sum(stems[k] * BUS_GAIN[k] for k in BUSN if k in stems).mean(1)
    bed = (stems['crowd'] * BUS_GAIN['crowd'] + stems['fx'] * BUS_GAIN['fx']).mean(1)
    for a, b in [(2.4, 21.4), (23.4, 31.1), (31.1, 42.5), (42.5, 51.6), (51.6, 59.4)]:
        sa, sb = int(a * SR), int(b * SR)
        print(f'  band over crowd+fx {a:4.1f}-{b:4.1f}s: {20 * np.log10(np.sqrt((bandsum[sa:sb] ** 2).mean()) / np.sqrt((bed[sa:sb] ** 2).mean() + 1e-12)):+.1f} dB')


if __name__ == '__main__':
    main()
