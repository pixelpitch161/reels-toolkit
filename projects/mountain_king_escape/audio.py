"""The Mountain King Escape soundtrack: our own synthesised arrangement of Grieg's public-domain theme
(see score.py for the transcription source). Everything is generated here; no samples, no recordings.

Every melody note = one bounce of the ball (timeline.NOTES), played by the 'bounce' bus: a crisp crystal tink
on the ring (pitched two octaves up, always on top) + the section's melody instruments, which climb:
pizzicato -> + bassoon -> + strings (theme up a fifth) -> octave doubling -> + brass -> full orchestra ->
prestissimo -> coda (two stabs with Grieg's G# A A# grace notes, the run, the final chord).
Under it: Grieg's bass (pizzicato quarters, later the running bass in octaves), off-beat chords, timpani on
downbeats and ring breaks, cymbal + glass shatter on every break, snare rolls in the prestissimo, a sting
(the grace-note figure) on every near-miss. Loop tail: final chord ring-out, whoosh up the shaft, the giant
ring's BONG, a reverse swell while the shards re-form, twelve falling pings as the ball drops through the
aligned gaps, then the low pizzicato restart that frame 0 continues.

    python audio.py   # -> audio.wav + levels, bounce-vs-music band margins, quietest window"""
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
import score as SC
import geom as G
import shards as SH

SR = A.SR
NS = int(T.DUR * SR)
TAIL = 6 * SR
RNG = np.random.default_rng(1875)


def tt(n):
    return np.arange(n) / SR


def mtof(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def partials(f, specs, dur, att=0.0015):
    n = int(dur * SR)
    t = tt(n)
    out = np.zeros(n)
    for r, a, d in specs:
        if f * r < SR / 2 - 800:
            out += a * np.sin(2 * math.pi * f * r * t + r) * np.exp(-t / d)
    return out * np.clip(t / att, 0, 1)


def harm_wave(f, n, amps, ph_jit=0.0, fm=None):
    """additive periodic wave with harmonic amplitudes amps[k-1] (band-limited)"""
    t = tt(n)
    ph = 2 * math.pi * (np.cumsum(fm) / SR if fm is not None else f * t)
    out = np.zeros(n)
    for k, a in enumerate(amps, 1):
        if a and f * k < 9000:
            out += a * np.sin(k * ph + ph_jit * k)
    return out


def env_ad(n, att, rel, hold):
    e = np.ones(n)
    na = max(1, int(att * SR))
    e[:na] = np.linspace(0, 1, na)
    nh = int(hold * SR)
    nr = max(1, int(rel * SR))
    if nh + nr < n:
        e[nh:nh + nr] *= np.linspace(1, 0, nr)
        e[nh + nr:] = 0
    return e


# ------------------------------------------------------------------ instruments
_C = {}


def cached(key, fn):
    if key not in _C:
        _C[key] = fn()
    return _C[key]


def pizz(p, length=0.6):
    def make():
        f = mtof(p)
        n = int(length * SR)
        t = tt(n)
        tau0 = 0.42 * (220 / f) ** 0.35
        x = np.zeros(n)
        for k in range(1, 13):
            if f * k > 9000:
                break
            a = abs(math.sin(math.pi * k * 0.18)) / k ** 1.05
            x += a * np.sin(2 * math.pi * f * k * t * (1 + 0.0004 * k * k)) * np.exp(-t / (tau0 / (1 + 0.55 * (k - 1))))
        body = A.bp(x, 180, 520) * 0.35
        snap = A.hp(RNG.normal(0, 1, n), 2200) * np.exp(-t / 0.005) * 0.25
        return (x + body + snap) * np.clip(t / 0.0015, 0, 1)
    return cached(('pizz', p, length), make)


def bassoon(p, hold):
    def make():
        f = mtof(p)
        n = int((hold + 0.08) * SR)
        amps = [abs(math.sin(math.pi * k * 0.22)) / k ** 0.75 for k in range(1, 16)]
        x = harm_wave(f, n, amps)
        x = A.lp(x, 2000) + A.bp(x, 420, 620) * 0.8 + A.bp(x, 1050, 1350) * 0.5
        return x * env_ad(n, 0.014, 0.05, hold)
    return cached(('bsn', p, round(hold, 2)), make)


def strings(p, hold, bright=1.0):
    def make():
        f = mtof(p)
        n = int((hold + 0.1) * SR)
        x = np.zeros(n)
        for det in (-0.004, 0.0, 0.0045):
            amps = [1 / k for k in range(1, 24)]
            x += harm_wave(f * (1 + det), n, amps, ph_jit=det * 300)
        x = A.lp(x / 3, 3800 * bright) + A.bp(RNG.normal(0, 1, n), 2500, 6000) * 0.03
        e = env_ad(n, 0.006, 0.07, hold) * (0.65 + 0.35 * np.exp(-tt(n) / 0.05))
        return x * e
    return cached(('str', p, round(hold, 2), bright), make)


def brass(p, hold):
    def make():
        f = mtof(p)
        n = int((hold + 0.1) * SR)
        t = tt(n)
        fm = f * (1 - 0.012 * np.exp(-t / 0.025))
        fc = 500 + 2600 * (1 - np.exp(-t / 0.03)) - 900 * (1 - np.exp(-t / 0.15))
        x = np.zeros(n)
        ph = 2 * math.pi * np.cumsum(fm) / SR
        for k in range(1, 26):
            if f * k > 9000:
                break
            x += np.sin(k * ph) / k * np.exp(-(f * k) / fc)
        return x * env_ad(n, 0.012, 0.06, hold) * 1.6
    return cached(('brs', p, round(hold, 2)), make)


def flute(p, hold):
    def make():
        f = mtof(p)
        n = int((hold + 0.08) * SR)
        t = tt(n)
        x = np.sin(2 * math.pi * f * t) + 0.25 * np.sin(4 * math.pi * f * t) + 0.08 * np.sin(6 * math.pi * f * t)
        x += A.bp(RNG.normal(0, 1, n), f * 0.8, min(f * 2.5, 15000)) * 0.08
        return x * env_ad(n, 0.02, 0.05, hold)
    return cached(('fl', p, round(hold, 2)), make)


def tink(p, k_ring):
    """the ball on the crystal ring: pitched glassy tink (two octaves up) + a tick"""
    def make():
        f = mtof(p + 24)
        x = partials(f, [(1, 1.0, 0.06), (2.76, 0.35, 0.025), (5.4, 0.15, 0.012)], 0.18, att=0.0008)
        n = len(x)
        tick = A.hp(RNG.normal(0, 1, n), 3500) * np.exp(-tt(n) / 0.0025) * 0.45
        return x + tick
    return cached(('tink', p), make)


def timpani(p, strength=1.0):
    def make():
        f = mtof(p)
        n = int(1.4 * SR)
        t = tt(n)
        glide = 1 + 0.025 * np.exp(-t / 0.05)
        x = np.zeros(n)
        for r, a, d in ((1, 1.0, 0.9), (1.5, 0.5, 0.6), (1.98, 0.32, 0.45), (2.44, 0.22, 0.35), (2.94, 0.12, 0.25)):
            x += a * np.sin(2 * math.pi * np.cumsum(f * r * glide) / SR) * np.exp(-t / d)
        thump = A.lp(RNG.normal(0, 1, n), 600) * np.exp(-t / 0.018) * 0.9
        return (x + thump) * np.clip(t / 0.002, 0, 1)
    return cached(('timp', p), make) * strength


def bass_drum():
    def make():
        n = int(0.9 * SR)
        t = tt(n)
        f = 42 + 50 * np.exp(-t / 0.05)
        return np.sin(2 * math.pi * np.cumsum(f) / SR) * np.exp(-t / 0.45) + A.lp(RNG.normal(0, 1, n), 300) * np.exp(-t / 0.03) * 0.6
    return cached(('bd',), make)


def crash(dur=2.4, seed=5):
    def make():
        n = int(dur * SR)
        t = tt(n)
        r = np.random.default_rng(seed)
        noise = A.hp(r.normal(0, 1, n), 3200) * np.exp(-t / (dur * 0.33)) * 0.6
        metal = partials(415, [(1, 0.2, 0.9), (2.32, 0.18, 0.7), (3.71, 0.15, 0.6), (5.03, 0.12, 0.5), (7.44, 0.1, 0.4)], dur)
        return noise + metal
    return cached(('crash', dur, seed), make)


def shatter(k):
    def make():
        r = np.random.default_rng(100 + k)
        n = int(0.9 * SR)
        t = tt(n)
        x = np.zeros(n)
        for i in range(40):
            f = r.uniform(1800, 9500)
            t0 = r.uniform(0, 0.09) * (1 + 0.5 * r.uniform())
            d = r.uniform(0.02, 0.16)
            i0 = int(t0 * SR)
            seg = np.sin(2 * math.pi * f * t[:n - i0]) * np.exp(-t[:n - i0] / d) * r.uniform(0.3, 1.0)
            x[i0:] += seg
        noise = A.hp(r.normal(0, 1, n), 1500) * np.exp(-t / 0.18) * 1.6
        crack = A.bp(r.normal(0, 1, n), 250, 900) * np.exp(-t / 0.03) * 1.8
        return x * 0.22 + noise * 0.5 + crack * 0.5
    return cached(('shat', k), make)


def bell(p, dur=0.5):
    def make():
        f = mtof(p)
        return partials(f, [(1, 1.0, dur * 0.5), (2.0, 0.3, dur * 0.3), (3.0, 0.15, dur * 0.2), (4.2, 0.12, 0.05)], dur, att=0.001)
    return cached(('bell', p, dur), make)


def gong():
    n = int(3.6 * SR)
    t = tt(n)
    f = mtof(35)
    x = np.zeros(n)
    for r, a, d in ((1, 1.0, 2.4), (2.08, 0.6, 1.8), (2.93, 0.45, 1.4), (3.41, 0.4, 1.2), (4.72, 0.3, 0.9), (5.91, 0.22, 0.7), (8.3, 0.12, 0.4)):
        x += a * np.sin(2 * math.pi * f * r * t * (1 + 0.004 * np.sin(2 * math.pi * 0.7 * t))) * np.exp(-t / d)
    hit = A.lp(RNG.normal(0, 1, n), 1200) * np.exp(-t / 0.04)
    return x * np.clip(t / 0.003, 0, 1) + hit


# ------------------------------------------------------------------ bus
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


# ------------------------------------------------------------------ arrangement
SEC_GAIN = [0.85, 0.9, 0.95, 1.0, 1.05, 1.12, 1.2, 1.25]


def melody(B):
    """one entry per bounce: the melody voices of the current section + the crystal tink"""
    for i, n in enumerate(T.NOTES):
        t0, p, sec = n['m'], n['pitch'], n['sec']
        dur = n['dur']
        long_ = n['kind'] != 'bounce' or n['beats'] >= 1.0
        hold = min(dur * (0.85 if long_ else 0.55), 0.9 if long_ else 0.16)
        g = SEC_GAIN[sec]
        pan = 0.25 * math.cos(n['b'] * 0.9)
        if n['stab']:
            stab(B, t0, sec)
            continue
        if n['kind'] == 'final':
            continue                                    # the final chord is placed in coda_final()
        if sec <= 3:
            B.place('bounce', pizz(p + 12), t0, 0.55 * g, pan)
            B.place('bounce', pizz(p), t0, 0.30 * g, -pan)
        if sec >= 1:
            B.place('bounce', bassoon(p, hold), t0, (0.28 if sec < 4 else 0.22) * g, -0.2)
        if sec >= 2:
            B.place('bounce', strings(p + 12, hold), t0, 0.30 * g, 0.15)
        if sec >= 3:
            B.place('bounce', strings(p + 24, hold, 1.1), t0, 0.22 * g, -0.15)
        if sec >= 4:
            B.place('bounce', brass(p + 12, hold), t0, 0.28 * g, 0.05)
            B.place('bounce', brass(p, hold), t0, 0.20 * g, -0.05)
        if sec >= 5:
            B.place('bounce', flute(p + 24, hold), t0, 0.14 * g, 0.3)
            B.place('bounce', strings(p, hold), t0, 0.18 * g, 0.0)
        B.place('tink', tink(p, n['ring']), t0, 0.42 * (1.0 + 0.1 * sec), pan * 1.4)
        if n.get('near'):
            sting(B, t0, p)


def stab(B, t0, sec):
    """coda stab: grace notes G# A A# -> B over the diminished chord, tutti, timpani + bass drum + crash"""
    g32 = 0.035
    for j, gp in enumerate(SC.GRACE):
        B.place('bounce', flute(gp + 24, 0.03), t0 - (3 - j) * g32, 0.25)
        B.place('bounce', strings(gp + 12, 0.03), t0 - (3 - j) * g32, 0.22)
    for p in SC.STAB_CHORD:
        B.place('chords', brass(p + 12, 0.32), t0, 0.16)
        B.place('chords', strings(p + 12, 0.32), t0, 0.14)
    B.place('bounce', brass(59 + 12, 0.32), t0, 0.32)
    B.place('bounce', strings(59 + 24, 0.32), t0, 0.26)
    B.place('tink', tink(59, 12), t0, 0.6)
    B.place('perc', timpani(35), t0, 0.9)
    B.place('perc', bass_drum(), t0, 0.8)
    B.place('perc', crash(1.6, 9), t0, 0.35)


def sting(B, t0, p):
    """near-miss: Grieg's grace-note figure (G# A A# -> B) high on a bell + a zing"""
    t1 = t0 + 0.02
    for j, gp in enumerate((80, 81, 82, 83)):
        B.place('fx', bell(gp, 0.4 if j == 3 else 0.12), t1 + j * 0.032, 0.28 if j == 3 else 0.18, 0.4)
    n = int(0.14 * SR)
    t = tt(n)
    f = 1200 * 2 ** (1.4 * t / 0.14)
    z = np.sin(2 * math.pi * np.cumsum(f) / SR) * np.sin(np.pi * t / 0.14) ** 2
    B.place('fx', z, t1, 0.05, -0.3)


def accompaniment(B):
    for pi, (m0, b0, ph, sec) in enumerate(T.PHRASE_START):
        if ph == 'coda':
            continue
        nb = SC.phrase_len(ph) // 8
        g = SEC_GAIN[sec]
        for bar in range(nb):
            bb = b0 + 4 * bar
            # bass
            if sec <= 3:
                for q, p in enumerate(SC.BASS[ph][bar]):
                    tq = float(T.m_of_beat(bb + q))
                    B.place('bass', pizz(p, 0.5), tq, 0.55 * g, -0.1)
                    if sec >= 2:
                        B.place('bass', pizz(p - 12, 0.5), tq, 0.35 * g, 0.1)
            else:
                for e8, p in enumerate(SC.RUN[ph][bar]):
                    te = float(T.m_of_beat(bb + e8 / 2))
                    hold = min(0.12, float(T.m_of_beat(bb + e8 / 2 + 0.5)) - te) * 0.8
                    B.place('bass', strings(p - 12, hold, 0.8), te, 0.30 * g, -0.1)
                    B.place('bass', strings(p - 24, hold, 0.6), te, 0.28 * g, 0.1)
            # off-beat chords (beats 2 and 4) from the strings section on
            if sec >= 2:
                for q in (1, 3):
                    tq = float(T.m_of_beat(bb + q))
                    hold = min(0.14, float(T.m_of_beat(bb + q + 0.5)) - tq)
                    chord = SC.HARM[ph][bar * 2 + (q // 2)]
                    for p in chord:
                        B.place('chords', strings(p, hold, 0.7), tq, 0.10 * g, 0.2)
                        if sec >= 4:
                            B.place('chords', brass(p, hold), tq, 0.075 * g, -0.2)
            # timpani on the downbeat (tonic / dominant)
            if sec >= 3:
                root = SC.BASS[ph if ph in SC.BASS else 'A'][bar][0]
                tp = root - 12 if root - 12 >= 33 else root
                tq = float(T.m_of_beat(bb))
                B.place('perc', timpani(tp), tq, 0.45 * g)
                if sec >= 5:
                    B.place('perc', timpani(tp + 7 if tp + 7 < 47 else tp - 5), float(T.m_of_beat(bb + 2)), 0.3 * g)
                    B.place('perc', bass_drum(), tq, 0.45 * g)
            # snare: tutti off-beats, prestissimo rolls (32nds, crescendo per bar)
            if sec == 5:
                for q in (1, 3):
                    B.place('perc', A.snare(0.09), float(T.m_of_beat(bb + q)), 0.16)
            if sec == 6:
                for s32 in range(32):
                    ts = float(T.m_of_beat(bb + s32 / 8))
                    B.place('perc', A.snare(0.05), ts, 0.05 + 0.13 * (s32 / 31) ** 1.5, 0.1 * math.sin(s32))
    # coda run: running bass + timpani roll under it
    m0, b0, ph, sec = T.PHRASE_START[-1]
    for e8 in range(12):
        te = float(T.m_of_beat(b0 + 8 + e8 / 2))
        p = [47, 45, 43, 42, 47, 45, 43, 42, 46, 42, 46, 49][e8]
        B.place('bass', strings(p - 12, 0.1, 0.8), te, 0.32, -0.1)
        B.place('bass', strings(p - 24, 0.1, 0.6), te, 0.3, 0.1)
    for s16 in range(24):
        ts = float(T.m_of_beat(b0 + 8 + s16 / 4))
        B.place('perc', timpani(35, 0.5), ts, 0.12 + 0.2 * s16 / 23)


def breaks(B):
    """every ring break: glass shatter + timpani + cymbal (crash from the octave section on)"""
    for k, mb in T.RING_BREAK.items():
        sec = T.section_at(mb)
        B.place('fx', shatter(k), mb, 0.55, 0.15 * (-1) ** k)
        if k == T.NRINGS:
            continue
        B.place('perc', timpani(35 if k % 2 else 42), mb, 0.75 + 0.03 * k)
        if sec >= 3:
            B.place('perc', crash(2.2, k), mb, 0.36)
        else:
            B.place('perc', crash(1.4, k), mb, 0.22)
        if sec >= 5:
            B.place('perc', bass_drum(), mb, 0.6)


def coda_final(B):
    tf = T.T_FINAL
    for p in SC.FINAL_CHORD:
        B.place('chords', brass(p + 12, 1.4) if p > 40 else strings(p, 1.6, 0.7), tf, 0.17)
        B.place('chords', strings(p + 12, 1.6), tf, 0.14)
    B.place('bounce', brass(71, 1.2), tf, 0.4)
    B.place('bounce', strings(83, 1.2), tf, 0.3)
    B.place('tink', tink(59, 12), tf, 0.7)
    B.place('perc', crash(3.2, 21), tf, 0.6)
    B.place('perc', bass_drum(), tf, 1.0)
    B.place('perc', timpani(35), tf, 1.0)
    for j in range(14):                                       # timpani roll dying away
        B.place('perc', timpani(35, 0.6), tf + 0.12 + j * 0.065, 0.35 * (1 - j / 14) ** 1.5)


def tail_fx(B):
    tf = T.T_FINAL
    B.place('fx', A.whoosh(1.4, 300, 6500, rise=True, seed=31), tf + 0.05, 0.3, 0.0)
    B.place('fx', A.whoosh(1.2, 5000, 400, rise=False, seed=32), tf + 0.85, 0.14, 0.2)
    B.place('fx', gong(), T.T_BONG, 0.55, 0.0)
    B.place('fx', bell(83, 1.4), T.T_BONG, 0.15, 0.3)
    # reverse swell while the shards fly home
    r0, r1 = T.T_REFORM0, T.T_REFORM1
    n = int((r1 - r0) * SR)
    t = tt(n)
    u = t / (r1 - r0)
    sw = A.hp(RNG.normal(0, 1, n), 2500) * u ** 2.2 + A.whoosh(r1 - r0, 400, 7000, rise=True, seed=33) * u
    B.place('fx', sw * np.minimum(1, (n - np.arange(n)) / (0.02 * SR)), r0, 0.16)
    # each ring snapping whole again: a rising B-minor chime
    arp = [71, 74, 78, 83, 86, 90, 95, 98, 102, 107, 110, 114]
    for k in range(1, T.NRINGS + 1):
        B.place('fx', bell(min(arp[k - 1], 108), 0.35), SH.RING_DONE[k], 0.12, 0.3 * math.sin(k))
    # twelve pings as the ball drops through the aligned gaps (outer -> inner, falling)
    for k in range(T.NRINGS, 0, -1):
        ta = G.M_ALIGN[k] + T.DUR
        B.place('fx', bell(arp[k - 1] - 12, 0.25), ta, 0.13, 0.25 * math.sin(k * 1.7))
        B.place('fx', tink(arp[k - 1] - 36, k), ta, 0.2)
    B.place('fx', A.whoosh(0.9, 6000, 500, rise=False, seed=34), G.M_TOP + T.DUR - 0.25, 0.12, 0.0)


def camera_fx(B):
    B.place('fx', A.whoosh(2.8, 300, 4000, rise=True, seed=41), 3.0, 0.08, -0.2)
    B.place('fx', A.whoosh(1.3, 4000, 400, rise=False, seed=42), 6.9, 0.07, 0.2)
    B.place('fx', A.whoosh(1.2, 4500, 300, rise=False, seed=43), T.T_REENTER - 0.7, 0.13, 0.0)


def ambience(B):
    t = tt(NS)
    wind = A.lp(np.random.default_rng(7).normal(0, 1, NS), 350)
    wind = np.concatenate([wind[-SR:], wind])[SR:] * (0.7 + 0.3 * np.sin(2 * math.pi * t * 3 / T.DUR))
    B.get('amb')[:NS] += np.stack([wind, wind], 1) * 0.02
    r = np.random.default_rng(8)
    for k in range(70):
        td = r.uniform(0, T.DUR)
        B.place('amb', partials(r.uniform(1800, 3200), [(1, 1, 0.03), (1.6, 0.4, 0.02)], 0.1), td, 0.025, r.uniform(-0.8, 0.8))


# ------------------------------------------------------------------ master (loop-aware)
REVERB = {'bounce': 0.35, 'tink': 0.3, 'chords': 0.5, 'bass': 0.15, 'perc': 0.25, 'fx': 0.45}
BUS_GAIN = {'bounce': 1.0, 'tink': 1.0, 'chords': 1.0, 'bass': 1.0, 'perc': 1.0, 'fx': 1.0, 'amb': 1.0}
GAIN_PTS = [(0, -1.5), (3.3, -1.2), (9.6, -0.8), (21.4, -0.3), (26.6, 0.0), (36.3, 0.4), (44.9, 0.8), (52.0, 1.2),
            (53.5, 0.6), (56.9, -0.3), (57.0, -1.6), (60, -1.5)]
TARGET_DB = -11.5
REF = (44.9, 52.0)


def master(B):
    from scipy.ndimage import minimum_filter1d, uniform_filter1d
    dry = sum(b * BUS_GAIN.get(k, 1.0) for k, b in B.b.items())
    send = sum(B.b[k] * v for k, v in REVERB.items() if k in B.b)
    wet = A.reverb(np.concatenate([send, send]), dur=2.8, decay=0.75)[NS:]       # circular: the seam gets its tail
    x = dry + wet * 0.26
    g = np.interp(tt(NS), [p[0] for p in GAIN_PTS], [p[1] for p in GAIN_PTS])
    x = x * (10 ** (g / 20))[:, None]
    ref = np.sqrt((x[int(REF[0] * SR):int(REF[1] * SR)] ** 2).mean())
    thr = ref * 10 ** (-6 / 20)
    xl = np.concatenate([x[-SR:], x])
    a = math.exp(-1.0 / (0.08 * SR))
    e = np.sqrt(lfilter([1 - a], [1, -a], (xl ** 2).mean(1)).clip(1e-12))[SR:]
    x = x * np.where(e > thr, (thr / e) ** 0.33, 1.0)[:, None]
    ref = np.sqrt((x[int(REF[0] * SR):int(REF[1] * SR)] ** 2).mean())
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


def build():
    B = Bus()
    melody(B)
    accompaniment(B)
    breaks(B)
    coda_final(B)
    tail_fx(B)
    camera_fx(B)
    ambience(B)
    for k in list(B.b):                                   # periodic: fold what rings past 60 s onto the start
        b = B.b[k]
        b[:TAIL] += b[NS:NS + TAIL]
        B.b[k] = b[:NS].copy()
    for k, amt in (('chords', 0.55), ('bass', 0.4), ('perc', 0.3), ('amb', 0.5)):     # room for the tinks
        if k in B.b:
            b = B.b[k]
            B.b[k] = b - amt * A.bp(np.concatenate([b[-SR:], b]), 1800, 6000)[SR:]
    return B


def main():
    B = build()
    out = master(B)
    path = os.path.join(HERE, 'audio.wav')
    A.write(path, out)
    spans = [(a, a + 2) for a in range(0, 60, 2)]
    lv = A.section_levels(path, spans)
    print('2 s windows (rms dBFS):')
    print('  ' + ' '.join(f'{a:02d}:{r:5.1f}' for a, b, r, p in lv))
    print(f'  min {min(r for *_, r, p in lv):.1f}  max {max(r for *_, r, p in lv):.1f}  peak {max(p for *_, p in lv):.1f}')
    # bounce (melody + tinks) vs the rest of the music in 1.5-6 kHz and 250-1500 Hz
    lvl = lambda y, a, b: 20 * np.log10(np.sqrt((y[int(a * SR):int(b * SR)] ** 2).mean()) + 1e-12)
    for lo, hi in ((1500, 6000), (250, 1500)):
        sos = butter(4, [lo, hi], 'bp', fs=SR, output='sos')
        yb = sosfilt(sos, sum(B.b[k] for k in ('bounce', 'tink') if k in B.b).mean(1))
        ym = sosfilt(sos, sum(B.b[k] for k in ('chords', 'bass', 'perc', 'amb') if k in B.b).mean(1))
        print(f'bounce vs music {lo}-{hi} Hz (dB margin, 4 s windows):')
        print('  ' + ' '.join(f'{a:02d}:{lvl(yb, a, a + 4) - lvl(ym, a, a + 4):4.0f}' for a in range(0, 60, 4)))


if __name__ == '__main__':
    main()
