"""Soundtrack: every cricket chirp lands on the beat and IS the hi-hat.
The song's tempo is the chirp rate (Dolbear: 4 x (T - 40) per minute),
morphing genres as the night warms, then tape-stopping when it cools."""
import math
import numpy as np
from scipy.signal import butter, sosfilt, fftconvolve, lfilter
from scipy.io import wavfile
import timeline as T

SR = 48000
NS = int(SR * T.DUR)
rng = np.random.default_rng(31)


def tt(n):
    return np.arange(n) / SR


def buf():
    return np.zeros((NS + SR * 6, 2))


def place(bus, sig, t, gain=1.0, pan=0.0):
    i0 = int(round(t * SR))
    if sig.ndim == 1:
        a = (pan + 1) * math.pi / 4
        sig = np.stack([sig * math.cos(a), sig * math.sin(a)], 1)
    s0 = max(0, -i0); i0 = max(0, i0)
    n = min(len(sig) - s0, len(bus) - i0)
    if n > 0:
        bus[i0:i0 + n] += sig[s0:s0 + n] * gain


def sos(kind, f, o=2):
    return butter(o, f, kind, fs=SR, output='sos')


def hp(x, f, o=2): return sosfilt(sos('hp', f, o), x, axis=0)
def lp(x, f, o=2): return sosfilt(sos('lp', f, o), x, axis=0)
def bp(x, lo, hi, o=2): return sosfilt(sos('bp', [lo, hi], o), x, axis=0)


def mtof(m):
    return 440.0 * 2 ** ((m - 69) / 12)


# ------------------------------------------------------------ beat grid (chirps)
BEATS = list(T.BEAT_T)
_last = BEATS[-1]
for k in range(1, 17):                     # continue past the cold front for the tape stop
    BEATS.append(_last + k * 0.3)
BEATS = np.array(BEATS)


def bt(bar, pos):
    return float(np.interp(4 * bar + pos, np.arange(len(BEATS)), BEATS))


def blen(bar, pos, dur):
    return bt(bar, pos + dur) - bt(bar, pos)


# ------------------------------------------------------------ the chirp
def chirp(T_f, gain=1.0, pitch=1.0, short=1.0):
    """snowy tree cricket chirp: a burst of pulses on a ~3 kHz carrier"""
    car = (2550 + 18 * (T_f - 55)) * pitch
    npulse = 5
    prate = 48 + 0.9 * (T_f - 55)
    dur = npulse / prate * short
    n = int((dur + 0.02) * SR)
    t = tt(n)
    ph = (t * prate) % 1.0
    pulse = np.where(t < dur, np.sin(np.pi * np.clip(ph / 0.7, 0, 1)) ** 2 * (ph < 0.7), 0)
    env = np.clip(t / 0.004, 0, 1) * np.clip((dur + 0.015 - t) / 0.015, 0, 1)
    s = (np.sin(2 * np.pi * car * t) + 0.18 * np.sin(4 * np.pi * car * t + 1)) * pulse * env
    return s * gain


# ------------------------------------------------------------ drums & synths
def kick(punch=1.0, dec=0.32):
    n = int(0.5 * SR); t = tt(n)
    f = 46 + 110 * punch * np.exp(-t / 0.035)
    s = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / dec)
    s += hp(rng.normal(0, 1, n), 2500) * np.exp(-t / 0.004) * 0.3
    return np.tanh(1.6 * s) / np.tanh(1.6)


def snare(dec=0.14):
    n = int(0.35 * SR); t = tt(n)
    s = bp(rng.normal(0, 1, n), 1200, 8000) * np.exp(-t / dec) + 0.6 * np.sin(2 * np.pi * 190 * t) * np.exp(-t / 0.05)
    return s / np.abs(s).max()


def clap():
    n = int(0.35 * SR); t = tt(n)
    e = np.zeros(n)
    for d in (0.0, 0.011, 0.022):
        i = int(d * SR); e[i:] += np.exp(-t[:n - i] / (0.008 if d < 0.02 else 0.13))
    s = bp(rng.normal(0, 1, n), 1000, 5000) * e
    return s / np.abs(s).max()


def ohat():
    n = int(0.25 * SR); t = tt(n)
    s = hp(rng.normal(0, 1, n), 7000, 4) * np.exp(-t / 0.09)
    return s / np.abs(s).max()


def shaker():
    n = int(0.06 * SR); t = tt(n)
    return hp(rng.normal(0, 1, n), 6000) * np.exp(-t / 0.015)


def crash(dur=2.0):
    n = int(dur * SR); t = tt(n)
    return hp(rng.normal(0, 1, n), 4000) * np.exp(-t / (dur / 3))


def saw(f, n, det=0.0, voices=1):
    t = tt(n)
    out = np.zeros(n)
    for v in range(voices):
        d = (v - (voices - 1) / 2) * det
        out += 2 * ((f * (1 + d) * t + rng.uniform()) % 1.0) - 1
    return out / voices


def square(f, n, duty=0.5):
    t = tt(n)
    return np.where((f * t) % 1.0 < duty, 1.0, -1.0)


def adsr(n, a=0.005, d=0.1, s=0.6, r=0.05):
    e = np.ones(n) * s
    na, nd, nr = int(a * SR), int(d * SR), int(r * SR)
    na = min(na, n)
    e[:na] = np.linspace(0, 1, na)
    nd2 = min(nd, n - na)
    e[na:na + nd2] = np.linspace(1, s, nd2)
    if 0 < nr < n:
        e[-nr:] *= np.linspace(1, 0, nr)
    return e


def rhodes(m, dur, vel=1.0):
    n = int((dur + 0.4) * SR); t = tt(n); f = mtof(m)
    mod = np.sin(2 * np.pi * f * 14 * t) * np.exp(-t / 0.05) * 1.2
    s = np.sin(2 * np.pi * f * t + mod) + 0.25 * np.sin(4 * np.pi * f * t)
    e = np.exp(-t / 1.2) * np.clip((dur + 0.4 - t) / 0.4, 0, 1)
    return s * e * vel


def pluck(m, dur):
    n = int((dur + 0.2) * SR); t = tt(n); f = mtof(m)
    s = sum(np.sin(2 * np.pi * f * h * t) * np.exp(-t * (6 + 4 * h)) / h for h in (1, 2, 3, 4))
    return s * np.clip(t / 0.002, 0, 1)


def supersaw(m, dur, cut=3500):
    n = int((dur + 0.15) * SR)
    s = lp(saw(mtof(m), n, 0.012, 5), cut)
    return s * adsr(n, 0.01, 0.1, 0.8, 0.15)


def sqlead(m, dur, arp=None):
    n = int((dur + 0.05) * SR); t = tt(n)
    if arp:
        step = int(0.035 * SR)
        f = np.array([mtof(m + arp[(i // step) % len(arp)]) for i in range(n)])
        ph = np.cumsum(f) / SR
        s = np.where(ph % 1.0 < 0.25, 1.0, -1.0)
    else:
        f = mtof(m) * (1 + 0.006 * np.sin(2 * np.pi * 6 * t) * np.clip((t - 0.1) / 0.1, 0, 1))
        ph = np.cumsum(f) / SR
        s = np.where(ph % 1.0 < 0.5, 1.0, -1.0)
    return lp(s, 7000) * adsr(n, 0.003, 0.05, 0.75, 0.04)


def bass(m, dur, kind='sine'):
    n = int((dur + 0.05) * SR); t = tt(n); f = mtof(m)
    if kind == 'sine':
        s = np.sin(2 * np.pi * f * t) + 0.25 * np.sin(4 * np.pi * f * t)
    elif kind == '808':
        s = np.tanh(1.5 * np.sin(2 * np.pi * f * t)) * np.exp(-t / max(0.2, dur))
    elif kind == 'slap':
        s = lp(saw(f, n), 1800 * np.exp(-0) if True else 0) * np.exp(-t / 0.18) + 0.5 * np.sin(2 * np.pi * f * t)
    elif kind == 'reese':
        s = lp(saw(f, n, 0.02, 3), 700) * 1.3
    elif kind == 'pulse':
        s = lp(square(f, n, 0.5), 2200) * 0.6
    return s * adsr(n, 0.004, 0.05, 0.85, 0.03)


def pad(ms, dur, cut=1600, att=0.3):
    n = int((dur + 0.5) * SR)
    s = np.zeros(n)
    for m in ms:
        s += saw(mtof(m), n, 0.006, 3)
    s = lp(s / len(ms), cut)
    t = tt(n)
    return s * np.clip(t / att, 0, 1) * np.clip((dur + 0.5 - t) / 0.5, 0, 1)


def whoosh(dur, lo=300, hi=5000):
    n = int(dur * SR)
    x = rng.normal(0, 1, n)
    out = np.zeros(n); zi = None
    for k in range(0, n, 1024):
        fc = lo * (hi / lo) ** (k / n)
        so = sos('bp', [fc * 0.7, min(fc * 1.4, SR / 2 - 100)])
        if zi is None:
            zi = np.zeros((so.shape[0], 2))
        y, zi = sosfilt(so, x[k:k + 1024], zi=zi)
        out[k:k + 1024] = y
    return out / (np.abs(out).max() + 1e-9) * np.sin(np.linspace(0, np.pi, n)) ** 2


def chime(m, dur=1.2):
    n = int(dur * SR); t = tt(n); f = mtof(m)
    return (np.sin(2 * np.pi * f * t) + 0.4 * np.sin(2 * np.pi * f * 2.76 * t) * np.exp(-t / 0.2)) * np.exp(-t / 0.5)


def reverb(x, dur=2.2, decay=0.55):
    r = np.random.default_rng(3)
    n = int(dur * SR); t = tt(n)
    ir = np.stack([r.normal(0, 1, n), r.normal(0, 1, n)], 1) * np.exp(-t / decay)[:, None]
    ir = lp(ir, 6000); ir[:int(0.015 * SR)] = 0
    ir /= np.sqrt((ir ** 2).sum(0)).max()
    return np.stack([fftconvolve(x[:, 0], ir[:, 0])[:len(x)], fftconvolve(x[:, 1], ir[:, 1])[:len(x)]], 1)


# ------------------------------------------------------------ the song
CHORDS = [  # bass root, voicing (lo-fi sevenths)
    (45, [57, 60, 64, 67]),   # Am7
    (41, [53, 57, 60, 64]),   # Fmaj7
    (48, [55, 60, 64, 71]),   # Cmaj7
    (43, [55, 59, 62, 65]),   # G7
]
MELODY = [
    [(0, 76, 1), (1, 74, 0.5), (1.5, 72, 0.5), (2, 69, 1), (3, 72, 1)],
    [(0, 77, 1), (1, 76, 0.5), (1.5, 72, 0.5), (2, 69, 2)],
    [(0, 79, 1), (1, 76, 0.5), (1.5, 72, 0.5), (2, 76, 1), (3, 79, 1)],
    [(0, 74, 1), (1, 71, 0.5), (1.5, 67, 0.5), (2, 71, 1), (3, 74, 1)],
]


def song(mus, drums, nbars_total):
    for b in range(nbars_total):
        g = T.BAR_GENRE[min(b, T.NBARS - 1)]
        climax = b >= 20
        root, voic = CHORDS[b % 4]
        t0 = bt(b, 0)
        L = blen(b, 0, 4)
        mel = MELODY[b % 4]
        first = b == 0 or (b < T.NBARS and T.BAR_GENRE[b] != T.BAR_GENRE[b - 1]) or b == 20
        if first and b > 0:
            place(drums, crash(2.0), t0, 0.16)
            place(mus, whoosh(0.6, 3000, 400), t0 - 0.05, 0.05)
        if g == 0:                       # LO-FI
            for m in voic:
                place(mus, rhodes(m, L * 0.95), t0, 0.07, -0.2 + 0.13 * voic.index(m))
            place(mus, bass(root - 12, blen(b, 0, 1.6)), t0, 0.32)
            place(mus, bass(root - 12, blen(b, 2.5, 1.2)), bt(b, 2.5), 0.26)
            place(drums, kick(0.8, 0.25), t0, 0.6)
            place(drums, kick(0.8, 0.25), bt(b, 2.5), 0.45)
            for p in (1, 3):
                place(drums, snare(0.12), bt(b, p), 0.22)
            if b >= 1:
                for pos, m, d in mel:
                    place(mus, rhodes(m + 12, blen(b, pos, d)) * 0.8, bt(b, pos), 0.07, 0.15)
        elif g == 1:                     # HIP-HOP
            for m in voic:
                place(mus, rhodes(m, blen(b, 0, 1.5)), t0, 0.06)
                place(mus, rhodes(m, blen(b, 2, 1.5)), bt(b, 2), 0.05)
            place(mus, bass(root - 12, blen(b, 0, 1.4), '808'), t0, 0.5)
            place(mus, bass(root - 12, blen(b, 2.5, 1.2), '808'), bt(b, 2.5), 0.45)
            for p in (0, 0.75, 2.5):
                place(drums, kick(1.0, 0.3), bt(b, p), 0.7)
            for p in (1, 3):
                place(drums, snare(0.15), bt(b, p), 0.35)
                place(drums, clap(), bt(b, p), 0.15)
            for pos, m, d in mel:
                place(mus, sqlead(m, blen(b, pos, d) * 0.9) * 0.6, bt(b, pos), 0.05, 0.1)
        elif g == 2:                     # FUNK
            for p in (0, 1.5, 2.5):
                place(drums, kick(1.0, 0.28), bt(b, p), 0.7)
            for p in (1, 3):
                place(drums, snare(0.13), bt(b, p), 0.42)
            for k in range(16):
                place(drums, shaker(), bt(b, k * 0.25), 0.05 + 0.04 * (k % 2), 0.4)
            pat = [0, 12, 0, 0, 10, 12, 0, 7]
            for k, o in enumerate(pat):
                place(mus, bass(root - 12 + o, blen(b, k * 0.5, 0.4), 'slap'), bt(b, k * 0.5), 0.3)
            for p in (0.5, 1.5, 2.5, 3.5):
                for m in voic[1:]:
                    place(mus, pluck(m + 12, blen(b, p, 0.3)), bt(b, p), 0.045, 0.3)
            for pos, m, d in mel:
                place(mus, supersaw(m, blen(b, pos, d) * 0.85, 2400) * 0.7, bt(b, pos), 0.07, -0.1)
        elif g == 3:                     # HOUSE
            for p in range(4):
                place(drums, kick(1.0, 0.3), bt(b, p), 0.85)
                place(drums, ohat(), bt(b, p + 0.5), 0.12, 0.3)
            for p in (1, 3):
                place(drums, clap(), bt(b, p), 0.4)
            for p in range(4):
                place(mus, bass(root - 12, blen(b, p + 0.5, 0.4), 'pulse'), bt(b, p + 0.5), 0.5)
            for p in (0.75, 2.25, 3.5):
                for m in voic:
                    place(mus, pluck(m + 12, blen(b, p, 0.4)), bt(b, p), 0.06, -0.25)
            for pos, m, d in mel:
                place(mus, rhodes(m + 12, blen(b, pos, d)), bt(b, pos), 0.09, 0.15)
        elif g == 4:                     # TRANCE
            for p in range(4):
                place(drums, kick(1.1, 0.3), bt(b, p), 0.85)
                for q in (0.25, 0.5, 0.75):
                    place(mus, bass(root - 12, blen(b, p + q, 0.22), 'pulse'), bt(b, p + q), 0.42)
            for p in (1, 3):
                place(drums, clap(), bt(b, p), 0.38)
            for k in range(16):
                place(mus, supersaw(voic[k % 4] + 12, blen(b, k * 0.25, 0.2), 4000), bt(b, k * 0.25), 0.03 + 0.02 * (k % 4 == 0), (k % 2) * 0.6 - 0.3)
            for pos, m, d in mel:
                place(mus, supersaw(m + 12, blen(b, pos, d) * 0.9, 5000), bt(b, pos), 0.08)
        elif g == 5:                     # DRUM & BASS
            for p in (0, 2.5):
                place(drums, kick(1.2, 0.25), bt(b, p), 0.85)
            for p in (1, 3):
                place(drums, snare(0.16), bt(b, p), 0.6)
            for p in (1.75, 3.5):
                place(drums, snare(0.06), bt(b, p), 0.18)
            place(mus, bass(root - 12, L * 0.95, 'reese'), t0, 0.5)
            place(mus, pad(voic, L * 0.95, 1200), t0, 0.08)
            for pos, m, d in mel:
                place(mus, pluck(m + 12, blen(b, pos, d)), bt(b, pos), 0.12, 0.2)
                place(mus, pluck(m + 24, blen(b, pos, d)), bt(b, pos) + blen(b, pos, 0.5), 0.04, -0.4)
        if g == 6 or climax:             # CHIPTUNE (+ everything at max tempo)
            for p in range(4):
                place(drums, kick(1.2, 0.22), bt(b, p), 0.85)
            for p in (1, 3):
                place(drums, snare(0.1), bt(b, p), 0.5)
            for k in range(8):
                place(mus, bass(root - 12 + (12 if k % 2 else 0), blen(b, k * 0.5, 0.45), 'pulse'), bt(b, k * 0.5), 0.45)
            arp = [0, 3, 7, 12] if b % 4 in (0,) else ([0, 4, 7, 12] if b % 4 in (1, 2) else [0, 4, 7, 10])
            for k in range(4):
                place(mus, sqlead(voic[0] + 12, blen(b, k, 0.95), arp) * 0.7, bt(b, k), 0.04, 0.4)
            for pos, m, d in mel:
                place(mus, sqlead(m + 12, blen(b, pos, d) * 0.92), bt(b, pos), 0.09, -0.1)
            if climax:
                place(drums, crash(1.2), t0, 0.12)
                for pos, m, d in mel:
                    place(mus, supersaw(m, blen(b, pos, d) * 0.9, 5000), bt(b, pos), 0.06, 0.2)


def tape_stop(x, t0, length=1.5):
    """speed falls 1 -> 0 from t0; output silent afterwards"""
    i0 = int(t0 * SR)
    n = int(length * SR)
    spd = (1 - np.linspace(0, 1, n)) ** 1.7
    pos = np.cumsum(spd)
    out = x.copy()
    src = x[i0:i0 + int(pos[-1]) + 4]
    for ch in range(2):
        out[i0:i0 + n, ch] = np.interp(pos, np.arange(len(src)), src[:, ch]) * np.linspace(1, 0.2, n)
    out[i0 + n:] = 0
    return out


def main():
    chirps = buf(); mus = buf(); drums = buf(); fx = buf(); amb = buf()
    # ---------------- chirps: hero + chorus (the hi-hat)
    sync_start, sync_end = T.BAR_T[7] - 0.5, T.BAR_T[8] + 0.5
    for tc, kind in T.CHIRPS:
        if tc >= T.DUR:
            continue
        Tf = T.temp(tc)
        if kind == 'ff':
            place(chirps, chirp(55, pitch=1.06, short=0.7), tc, 0.45)
            continue
        hero_g = 0.5 if kind in ('real', 'cool') else 0.55
        place(chirps, chirp(Tf), tc, hero_g)
        if kind == 'beat' or kind == 'cool':
            spread = 0.11 * (1 - min(1, max(0, (tc - sync_start) / (sync_end - sync_start))))
            bl = 1.0 / T.chirp_rate(Tf)
            for k in range(6):
                jit = rng.normal(0, spread) * bl + rng.normal(0, 0.003)
                place(chirps, chirp(Tf, pitch=1 + rng.uniform(-0.04, 0.04)), tc + jit, 0.12, rng.uniform(-0.8, 0.8))
    # ---------------- hook + count + reveal
    place(mus, pad([57, 64, 67, 71, 74], 3.9, 900, 0.8), 0.0, 0.06)
    place(fx, whoosh(3.75, 600, 4000) * 0.6, T.T_FF0, 0.05)
    # clock ticks during the count (each sped-up second)
    for k in range(16):
        n = int(0.02 * SR)
        place(fx, np.sin(2 * np.pi * 1800 * tt(n)) * np.exp(-tt(n) / 0.004), T.T_FF0 + k * 0.25, 0.10, 0.3)
    place(fx, chime(84), T.T_FF1, 0.14)
    place(fx, chime(91), T.T_FF1 + 0.12, 0.12)
    place(fx, whoosh(2.5, 3000, 300), T.T_MUSIC - 0.1, 0.10)
    # ---------------- the song
    song(mus, drums, T.NBARS + 2)
    # milestones: a bubbly blip every +5 F
    for ms, m in zip(range(60, 91, 5), [72, 74, 76, 79, 81, 84, 88]):
        for f in range(int(T.T_RISE0 * 30), int(T.T_RISE1 * 30) + 2):
            if T.temp(f / 30) >= ms:
                place(fx, chime(m, 0.6) * 0.7, f / 30, 0.08, -0.6)
                break
    # ---------------- cold front: tape stop + wind
    mixm = mus + drums
    mixm = tape_stop(mixm, T.T_COOL0, 1.5)
    place(fx, whoosh(2.0, 2500, 200), T.T_COOL0, 0.12)
    # outro: gentle lo-fi chords on the slow chirps
    for k, tc in enumerate([c[0] for c in T.CHIRPS if c[1] == 'cool' and c[0] > T.T_COOL1 - 0.1][:6]):
        root, voic = CHORDS[k % 4]
        for m in voic:
            place(mixm, rhodes(m, 0.95), tc, 0.035)
    place(fx, chime(81, 1.6), 57.0, 0.08)
    # ---------------- ambience: other insects, frogs, wind in the grass
    t = tt(NS)
    wind = lp(rng.normal(0, 1, (NS, 2)), 420) * (0.6 + 0.4 * np.sin(2 * np.pi * 0.06 * t))[:, None]
    amb[:NS] += wind * 0.06
    trill = bp(rng.normal(0, 1, NS), 4200, 6200) * (0.55 + 0.45 * np.sin(2 * np.pi * 28 * t)) * 0.03
    amb[:NS, 0] += trill; amb[:NS, 1] += np.roll(trill, 900)
    for k in range(18):
        tf = rng.uniform(0.5, 59)
        n = int(0.16 * SR); tq = tt(n)
        f0 = rng.uniform(150, 260)
        croak = np.sin(2 * np.pi * f0 * tq * (1 + 0.3 * tq)) * (np.sin(2 * np.pi * 30 * tq) > 0) * np.sin(np.pi * tq / tq[-1])
        place(amb, lp(croak, 1200), tf, 0.05, rng.uniform(-0.7, -0.2))

    # ---------------- mix
    rv = reverb(mixm * 0.7 + fx * 0.5 + chirps * 0.6, 2.0, 0.5)
    mix = (chirps + mixm + fx + amb + rv * 0.16)[:NS]
    for nm, bus in (('chirps', chirps), ('music', mixm), ('fx', fx), ('amb', amb)):
        lv = []
        for a, b in ((0, 3.9), (8.05, 12), (20, 24), (30, 34), (40, 44), (48, 51), (55, 59)):
            seg = bus[int(a * SR):int(b * SR)]
            lv.append(f"{20 * np.log10(np.sqrt((seg ** 2).mean()) + 1e-9):6.1f}")
        print(f"{nm:7s}", ' '.join(lv))
    pts = [(0, 11), (3.9, 11), (4.0, 9), (7.6, 9), (8.05, 1), (T.T_COOL0, 0), (T.T_COOL0 + 1.0, 4), (54.5, 8), (60, 10)]
    gdb = np.interp(tt(NS), [p[0] for p in pts], [p[1] for p in pts])
    mix *= (10 ** (gdb / 20))[:, None]
    env = np.sqrt(lfilter([0.002], [1, -0.998], (mix ** 2).mean(1)))
    thr = 0.15
    mix *= np.where(env > thr, (thr / np.maximum(env, 1e-9)) ** 0.45, 1.0)[:, None]
    mix *= 0.89 / np.abs(mix).max()
    mix = np.tanh(mix * 1.1) / np.tanh(1.1)
    mix *= 0.89 / np.abs(mix).max()
    fd = int(0.004 * SR)
    mix[:fd] *= np.linspace(0, 1, fd)[:, None]; mix[-fd:] *= np.linspace(1, 0, fd)[:, None]
    wavfile.write('audio.wav', SR, (mix * 32767).astype(np.int16))
    secs = [(0, 3.9), (3.9, 8.05)] + [(T.BAR_T[i], T.BAR_T[i + 1]) for i in (0, 3, 5, 7, 10, 13, 16)] + \
           [(T.BAR_T[20], T.T_COOL0), (T.T_COOL0, T.T_COOL1), (T.T_COOL1, 60)]
    for a, b in secs:
        seg = mix[int(a * SR):int(b * SR)]
        print(f'{a:5.1f}-{b:5.1f}s rms {20 * np.log10(np.sqrt((seg ** 2).mean()) + 1e-9):6.1f} peak {20 * np.log10(np.abs(seg).max() + 1e-9):5.1f}')


if __name__ == '__main__':
    main()
