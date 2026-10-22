"""Marble Waterfall Organ soundtrack. Every marble hit in marbles.npz (the same list that drives the
picture) is played by the voice of the element it struck: drum skins, glass discs, tuned pipe
"bloops", glass bowls, marimba bars, the spiral xylophone, cymbal discs and golden bell bars.
Open gates = the spray: random contacts = a chaotic wash of random (D-major) notes. Shut gates =
routed marbles on the 120 BPM grid = the song. Around it only a looping drone/pad, a riser into
the drop, gate clacks and the lever. All synthesised; arrangement original.

    python audio.py        # -> audio.wav + section levels, marble-vs-bed band margins, quietest window"""
import math
import os
import sys
import numpy as np
from scipy.signal import butter, sosfilt, lfilter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit import audio as A
import timeline as T
import machine as M
import sim as SIM

SR = A.SR
NS = int(T.DUR * SR)
TAIL = 6 * SR
RNG = np.random.default_rng(18)
# home pitches for spray hits on the spiral bars (D-major pentatonic, high at the top of the ramp)
SPIRAL_HOME = [86, 83, 81, 78, 76, 74, 71, 69, 66, 64, 62, 59, 57, 54, 52, 50]


def tt(n):
    return np.arange(n) / SR


def env(n, dec, att=0.002):
    t = tt(n)
    return np.clip(t / att, 0, 1) * np.exp(-t / dec)


def partials(f, specs, dur, att=0.0015):
    """sum of sines: specs = [(ratio, amp, decay)]"""
    n = int(dur * SR)
    t = tt(n)
    out = np.zeros(n)
    for r, a, d in specs:
        if f * r < SR / 2 - 500:
            out += a * np.sin(2 * math.pi * f * r * t + r) * np.exp(-t / d)
    return out * np.clip(t / att, 0, 1)


# ------------------------------------------------------------------ voices (one per element kind)
def v_kick():
    n = int(0.5 * SR)
    t = tt(n)
    f = 48 + 120 * np.exp(-t / 0.035)
    body = np.sin(2 * math.pi * np.cumsum(f) / SR) * np.exp(-t / 0.26)
    thock = np.sin(2 * math.pi * 196 * t) * np.exp(-t / 0.05) * 0.45                    # phone-audible skin
    click = A.hp(RNG.normal(0, 1, n), 2500) * np.exp(-t / 0.004) * 0.35
    return body + thock + click


def v_tom(p):
    n = int(0.45 * SR)
    t = tt(n)
    f0 = A.mtof(p)
    f = f0 * (1 + 0.35 * np.exp(-t / 0.03))
    return np.sin(2 * math.pi * np.cumsum(f) / SR) * np.exp(-t / 0.2) + A.bp(RNG.normal(0, 1, n), 800, 3000) * np.exp(-t / 0.02) * 0.25


def v_snare():
    n = int(0.32 * SR)
    t = tt(n)
    wires = A.bp(RNG.normal(0, 1, n), 1800, 9000) * np.exp(-t / 0.11) * 0.75
    body = np.sin(2 * math.pi * 210 * t) * np.exp(-t / 0.045) * 0.6 + np.sin(2 * math.pi * 330 * t) * np.exp(-t / 0.03) * 0.25
    tick = A.hp(RNG.normal(0, 1, n), 4000) * np.exp(-t / 0.003) * 0.4
    return wires + body + tick


def v_hat(k):
    """glass discs: a glassy cluster + a little air"""
    dec = (0.05, 0.11, 0.035)[k % 3]
    base = (5240, 4660, 6120)[k % 3]
    g = partials(base, [(1, 1.0, dec), (1.413, 0.7, dec * 0.8), (1.83, 0.5, dec * 0.6), (2.31, 0.3, dec * 0.5)], 0.3)
    n = len(g)
    air = A.hp(RNG.normal(0, 1, n), 7000) * np.exp(-tt(n) / (dec * 0.6)) * 0.35
    return (g * 0.5 + air)


def v_pipe(p):
    """tuned pipe 'bloop': pitch scoop + strong harmonics (audible on phones) + a hollow formant"""
    f0 = A.mtof(p)
    n = int(0.55 * SR)
    t = tt(n)
    f = f0 * (1 + 0.22 * np.exp(-t / 0.025))
    ph = 2 * math.pi * np.cumsum(f) / SR
    x = np.sin(ph) + 0.55 * np.sin(2 * ph) + 0.32 * np.sin(3 * ph) + 0.16 * np.sin(4 * ph)
    e = np.clip(t / 0.004, 0, 1) * np.exp(-t / 0.3)
    x = x * e
    return A.lp(x, 1400) + A.bp(x, 500, 1100) * 0.4


def v_bowl(p, short=1.0):
    f = A.mtof(p)
    x = partials(f, [(1, 1.0, 1.7 * short), (1.0025, 0.6, 1.5 * short), (2.0, 0.12, 0.7 * short), (2.76, 0.28, 0.55 * short),
                     (5.4, 0.1, 0.25 * short)], 2.6 * short + 0.2, att=0.003)
    return x


def v_bar(p, short=1.0):
    f = A.mtof(p)
    return partials(f, [(1, 1.0, 0.42 * short), (4.0, 0.33, 0.08), (9.2, 0.1, 0.025)], 0.9 * short + 0.1)


def v_xylo(p, short=1.0):
    f = A.mtof(p)
    return partials(f, [(1, 1.0, 0.17 * short), (3.0, 0.42, 0.06), (6.27, 0.16, 0.03)], 0.45)


def v_crash():
    n = int(2.2 * SR)
    t = tt(n)
    noise = A.hp(RNG.normal(0, 1, n), 3500) * np.exp(-t / 0.75) * 0.55
    metal = partials(410, [(1, 0.2, 0.9), (2.32, 0.18, 0.7), (3.71, 0.15, 0.6), (5.03, 0.12, 0.5), (7.44, 0.1, 0.4)], 2.2)
    return noise + metal


def v_ride():
    n = int(1.0 * SR)
    t = tt(n)
    ping = partials(2960, [(1, 0.5, 0.35), (1.47, 0.35, 0.28), (2.1, 0.2, 0.2)], 1.0)
    wash = A.hp(RNG.normal(0, 1, n), 5000) * np.exp(-t / 0.3) * 0.25
    return ping + wash


def v_bell(p, short=1.0):
    """melody: golden bell bars, glock-vibe hybrid, bright enough to lead"""
    f = A.mtof(p)
    x = partials(f, [(1, 1.0, 0.95 * short), (2.0, 0.32, 0.45 * short), (3.0, 0.14, 0.25), (4.0, 0.22, 0.06), (5.4, 0.1, 0.04)],
                 1.6 * short + 0.2, att=0.001)
    return x


_CACHE = {}


def voice(kind, p, spray, el_idx):
    key = (kind, p, spray, el_idx if kind == 'hat' else 0)
    if key in _CACHE:
        return _CACHE[key]
    sh = 0.55 if spray else 1.0
    if kind == 'kick':
        v = v_kick()
    elif kind == 'tom':
        v = v_tom(p)
    elif kind == 'snare':
        v = v_snare()
    elif kind == 'hat':
        v = v_hat(el_idx)
    elif kind == 'pipe':
        v = v_pipe(p)
    elif kind == 'bowl':
        v = v_bowl(p, sh)
    elif kind == 'bar':
        v = v_bar(p, sh)
    elif kind == 'xylo':
        v = v_xylo(p, sh)
    elif kind == 'crash':
        v = v_crash() * (0.5 if spray else 1.0)
    elif kind == 'ride':
        v = v_ride()
    else:
        v = v_bell(p, sh)
    _CACHE[key] = v
    return v


# per-kind bus gains (routed) -- melody leads, bowls sit under
GAIN = {'kick': 0.95, 'tom': 0.55, 'snare': 1.3, 'hat': 0.8, 'pipe': 0.56, 'bowl': 0.14, 'bar': 0.22, 'xylo': 0.34,
        'crash': 0.55, 'ride': 0.32, 'bell': 0.6}
SPRAY_GAIN = {k: v * 0.68 for k, v in {'tom': 0.4, 'snare': 0.25, 'hat': 0.25, 'pipe': 0.45, 'bowl': 0.14, 'bar': 0.22,
                                       'xylo': 0.2, 'crash': 0.07, 'ride': 0.08, 'bell': 0.2}.items()}


class Bus:
    def __init__(self):
        self.b = {}

    def get(self, name):
        if name not in self.b:
            self.b[name] = np.zeros((NS + TAIL, 2))
        return self.b[name]

    def place(self, name, sig, t, gain=1.0, pan=0.0):
        b = self.get(name)
        t = t % T.DUR
        i0 = int(round(t * SR))
        if sig.ndim == 1:
            a = (pan + 1) * math.pi / 4
            sig = np.stack([sig * math.cos(a), sig * math.sin(a)], 1)
        n = min(len(sig), len(b) - i0)
        b[i0:i0 + n] += sig[:n] * gain


def hits(B):
    D = SIM.load()
    for th, el, p, vel, src, ch in D['hits']:
        if th < 0 or th >= T.DUR:
            continue                                    # wrapped copies already sit in [0, 60)
        e = M.ELEMS[int(el)]
        kind = e.kind
        spray = src == 1
        if p < 0:
            p = SPIRAL_HOME[e.idx] if kind == 'xylo' else None
        p = None if p is None else int(p)
        v = voice(kind, p, spray, e.idx)
        pan = float(np.clip(e.pos[0] / 1.3, -0.65, 0.65))
        if spray:
            B.place('spray', v, th, SPRAY_GAIN.get(kind, 0.2) * vel * RNG.uniform(0.8, 1.1), pan)
        else:
            bus = {'kick': 'kick', 'snare': 'drums', 'hat': 'drums', 'pipe': 'bass', 'bowl': 'chords', 'bar': 'chords',
                   'xylo': 'arp', 'crash': 'cym', 'ride': 'cym', 'bell': 'lead'}[kind]
            B.place(bus, v, th, GAIN[kind] * vel, pan * 0.7)


def loop_freq(f):
    """round a frequency so it completes whole cycles in 60 s (seamless loop)"""
    return round(f * T.DUR) / T.DUR


def drone(B):
    """B/F# open-fifth drone, continuous over the loop, swelling 3-6 s, under the chords later"""
    t = tt(NS)
    x = np.zeros(NS)
    for m, a in ((35, 0.5), (47, 0.7), (54, 0.55), (59, 0.35), (66, 0.18)):
        f = loop_freq(A.mtof(m))
        for k, det in enumerate((-0.0035, 0.0, 0.0042)):
            fd = loop_freq(f * (1 + det))
            x += a * (2 * ((t * fd + 0.31 * k) % 1.0) - 1) / 3
    x = A.lp(np.concatenate([x[-SR:], x]), 900)[SR:]                 # filter with a wrapped pre-roll
    lfo = 0.85 + 0.15 * np.sin(2 * math.pi * 3 * t / T.DUR * 4)
    g = np.interp(t, [0, 3.0, 6.0, 13.5, 15.0, 56.5, 57.6, 60.0], [0.42, 0.45, 1.0, 1.0, 0.28, 0.28, 0.42, 0.42])
    sig = x * lfo * g
    B.get('pad')[:NS] += np.stack([sig, sig], 1) * 0.16


def chord_pad(B):
    voic = {0: (47, 54, 59, 62, 66), 1: (43, 50, 55, 59, 62), 2: (50, 57, 62, 66, 69), 3: (45, 52, 57, 61, 64)}
    for bar in range(7, 28):
        ci = T.chord_idx(bar)
        t0 = T.bt(bar)
        if t0 >= T.T_GAP - 0.01 and t0 < T.T_DROP:
            continue
        dur = T.BAR + 0.25
        if bar == 19:
            dur = T.T_GAP - t0
        sig = A.pad(list(voic[ci]), dur, cut=1500, att=0.08)
        B.place('pad', sig, t0, 0.17)
    B.place('pad', A.pad([47, 54, 59, 62, 66], 2.0, cut=1500, att=0.05), 56.0, 0.17)


def fx(B):
    # gate clacks (on the shut downbeat), gate-8 clunks, the snap, the lever
    for g, ts in enumerate(T.GATE_SHUT):
        n = int(0.25 * SR)
        t = tt(n)
        clack = A.bp(RNG.normal(0, 1, n), 1500, 6000) * np.exp(-t / 0.012) + partials(1250 + 90 * g, [(1, 0.5, 0.06), (2.7, 0.25, 0.04)], 0.25)
        B.place('fx', clack, ts - 0.004, 0.32, 0.0)
    for tt_ in T.T_TRY:
        n = int(0.4 * SR)
        t = tt(n)
        clunk = A.lp(RNG.normal(0, 1, n), 900) * np.exp(-t / 0.03) * 0.8 + partials(170, [(1, 1, 0.12), (2.4, 0.4, 0.08)], 0.4)
        B.place('fx', clunk, tt_, 0.5, 0.0)
    # riser 36 -> 39.5 (noise sweep + rising tone), cut for the half-beat gap
    dur = T.T_GAP - T.T_RISER
    n = int(dur * SR)
    t = tt(n)
    u = t / dur
    nz = A.whoosh(dur, 300, 9000, rise=True) * (0.2 + 0.8 * u ** 2)
    f = 220 * 2 ** (2.5 * u ** 1.3)
    tone = np.sin(2 * math.pi * np.cumsum(f) / SR) * u ** 2 * 0.25
    r = (nz + tone) * np.minimum(1, (dur - t) / 0.01)
    B.place('fx', r, T.T_RISER, 0.42, 0.0)
    # drop: sub boom + air burst
    n = int(1.6 * SR)
    t = tt(n)
    boom = np.sin(2 * math.pi * np.cumsum(38 + 60 * np.exp(-t / 0.08)) / SR) * np.exp(-t / 0.55)
    B.place('fx', boom, T.T_DROP, 0.75, 0.0)
    B.place('fx', A.whoosh(1.2, 4000, 300, rise=False, seed=9), T.T_DROP, 0.18, 0.0)
    # camera whooshes
    B.place('fx', A.whoosh(2.4, 300, 5000, rise=True, seed=11), T.T_PULL - 0.3, 0.12, -0.2)
    B.place('fx', A.whoosh(1.4, 5000, 400, rise=False, seed=12), T.T_WIDE - 0.6, 0.1, 0.2)
    # lever: clank, spring, the spill whoosh rising back into the opening chaos
    n = int(0.6 * SR)
    t = tt(n)
    clank = partials(310, [(1, 1, 0.18), (2.63, 0.6, 0.12), (4.1, 0.35, 0.08)], 0.6) + A.bp(RNG.normal(0, 1, n), 800, 5000) * np.exp(-t / 0.02)
    B.place('fx', clank, T.T_LEVER, 0.55, 0.3)
    spring = partials(620, [(1, 0.6, 0.25)], 0.5) * (1 + 0.5 * np.sin(2 * math.pi * 9 * tt(int(0.5 * SR))))
    B.place('fx', spring, T.T_LEVER + 0.45, 0.25, 0.3)
    B.place('fx', A.crash(2.0), T.T_LEVER, 0.22, 0.0)
    B.place('fx', A.whoosh(2.9, 300, 6000, rise=True, seed=13), T.T_LEVER + 0.1, 0.15, -0.1)


def ambience(B):
    """very quiet: the lift's mechanical whir and the basin clatter (incidental textures stay low)"""
    t = tt(NS)
    ph = np.array([T.lift_turns(x) for x in np.arange(0, T.DUR, 0.01)])
    ph = np.interp(t, np.arange(0, T.DUR, 0.01), ph)
    whir = np.sin(2 * math.pi * ph * 18) * 0.5 + np.sin(2 * math.pi * ph * 36) * 0.25
    whir = whir * (0.5 + 0.5 * np.sin(2 * math.pi * ph) ** 2)
    B.get('amb')[:NS] += np.stack([whir, whir], 1) * 0.006
    D = SIM.load()
    for row in D['lands']:
        if 0 <= row[0] < T.DUR:
            n = int(0.03 * SR)
            c = A.bp(RNG.normal(0, 1, n), 2500, 7000) * np.exp(-tt(n) / 0.006)
            B.place('amb', c, row[0], 0.025 * RNG.uniform(0.5, 1.0), float(np.clip(row[1] / 1.6, -0.7, 0.7)))


# ------------------------------------------------------------------ master (loop-aware)
REVERB = {'spray': 0.5, 'lead': 0.45, 'chords': 0.55, 'arp': 0.4, 'bass': 0.15, 'drums': 0.12, 'kick': 0.05, 'cym': 0.25,
          'pad': 0.8, 'fx': 0.35}
BUS_GAIN = {'spray': 1.0, 'kick': 1.0, 'drums': 1.0, 'bass': 1.0, 'chords': 1.0, 'arp': 1.0, 'cym': 1.0, 'lead': 1.0,
            'pad': 1.0, 'fx': 1.0, 'amb': 1.0}
GAIN_PTS = [(0, 0.0), (5.9, 0.0), (6.0, -0.6), (14, -0.5), (24, 0.0), (34, 0.3), (39.45, 1.5), (40.0, 1.5), (56.9, 1.5),
            (57.6, 0.0), (60, 0.0)]
TARGET_DB = -10.5


def master(B):
    """sum + loop-aware reverb + section automation + gentle glue; then set the drop to TARGET_DB and
    only limit peaks (the build must survive: no final peak-normalise)"""
    from scipy.ndimage import minimum_filter1d, uniform_filter1d
    dry = sum(b * BUS_GAIN.get(k, 1.0) for k, b in B.b.items())       # buses are already periodic (folded)
    send = sum(B.b[k] * v for k, v in REVERB.items() if k in B.b)
    wet = A.reverb(np.concatenate([send, send]), dur=2.6, decay=0.7)[NS:]   # circular reverb: the seam gets its tail
    x = dry + wet * 0.24
    g = np.interp(tt(NS), [p[0] for p in GAIN_PTS], [p[1] for p in GAIN_PTS])
    x = x * (10 ** (g / 20))[:, None]
    # glue (~80 ms RMS, 1.5:1 above -18 dB of the drop level), run around the loop
    ref = np.sqrt((x[int(40 * SR):int(56 * SR)] ** 2).mean())
    thr = ref * 10 ** (-6 / 20)
    xl = np.concatenate([x[-SR:], x])
    a = math.exp(-1.0 / (0.08 * SR))
    e = np.sqrt(lfilter([1 - a], [1, -a], (xl ** 2).mean(1)).clip(1e-12))[SR:]
    x = x * np.where(e > thr, (thr / e) ** 0.33, 1.0)[:, None]
    ref = np.sqrt((x[int(40 * SR):int(56 * SR)] ** 2).mean())
    x = x * (10 ** (TARGET_DB / 20) / ref)
    # lookahead peak limiter (3 ms), release ~60 ms, wrapped
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
    B = Bus()
    hits(B)
    drone(B)
    chord_pad(B)
    fx(B)
    ambience(B)
    # make every bus periodic: fold what rings past 60 s onto the start
    for k in list(B.b):
        b = B.b[k]
        b[:TAIL] += b[NS:NS + TAIL]
        B.b[k] = b[:NS].copy()
    # carve a little space for the melody (2-5 kHz) out of the wash, pad and accompaniment (circular filter)
    for k, amt in (('spray', 0.4), ('pad', 0.5), ('drums', 0.35), ('chords', 0.4), ('arp', 0.3), ('cym', 0.3)):
        if k in B.b:
            b = B.b[k]
            B.b[k] = b - amt * A.bp(np.concatenate([b[-SR:], b]), 2000, 5000)[SR:]
    out = master(B)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    path = os.path.join(HERE, 'audio.wav')
    spans = [(a, a + 2) for a in range(0, 60, 2)]
    lv = A.section_levels(path, spans)
    print('2 s windows (rms dBFS):')
    print('  ' + ' '.join(f'{a:02d}:{r:5.1f}' for a, b, r, p in lv))
    print(f'  min {min(r for *_, r, p in lv):.1f}  max {max(r for *_, r, p in lv):.1f}  peak {max(p for *_, p in lv):.1f}')
    # marbles (every hit) vs the non-marble bed (pad, riser/fx, ambience) in 300-3000 Hz
    sos = butter(4, [300, 3000], 'bp', fs=SR, output='sos')
    mk = ['spray', 'kick', 'drums', 'bass', 'chords', 'arp', 'cym', 'lead']
    ys = sosfilt(sos, sum(B.b[k][:NS] for k in mk if k in B.b).mean(1))
    yb = sosfilt(sos, sum(B.b[k][:NS] for k in ('pad', 'fx', 'amb') if k in B.b).mean(1))
    lvl = lambda y, a, b: 20 * np.log10(np.sqrt((y[int(a * SR):int(b * SR)] ** 2).mean()) + 1e-12)
    print('marble hits vs bed, 300-3000 Hz (dB margin):')
    print('  ' + ' '.join(f'{a:02d}:{lvl(ys, a, b) - lvl(yb, a, b):4.0f}' for a, b in [(a, a + 4) for a in range(0, 60, 4)]))
    # melody presence in the full song: lead bus vs everything else, 1.5-5 kHz
    sos2 = butter(4, [1500, 5000], 'bp', fs=SR, output='sos')
    yl = sosfilt(sos2, B.b['lead'][:NS].mean(1))
    yo = sosfilt(sos2, sum(B.b[k][:NS] for k in B.b if k != 'lead').mean(1))
    print(f'melody vs rest 1.5-5 kHz, 40-56 s: {lvl(yl, 40, 56) - lvl(yo, 40, 56):.1f} dB')
    x = out.mean(1)
    w = int(0.5 * SR)
    r = np.sqrt(np.convolve(x * x, np.ones(w) / w, mode='valid'))[::SR // 20]
    print(f'quietest 0.5 s window: {20 * np.log10(r.min() + 1e-12):.1f} dBFS at {np.argmin(r) / 20:.2f} s')


if __name__ == '__main__':
    main()
