"""Rock Paper Scissors Orchestra soundtrack: mood A, an ominous cinematic orchestra in D minor at 110.16 BPM (28 bars in
61 s, so the pulse is in phase at the loop point). Everything is synthesised and original, and the chase drives it:
every sound below comes from the cached honest run (sim.npz) through timeline.py.

    the sections       every catch steps the HUNTER's part, quantised forward to the grid:
                       ROCK = timpani (+ a taiko on every 4th step), 8ths; PAPER = strings, 16ths (pizzicato that turns
                       to tremolo as the section grows); SCISSORS = brass stabs, 8ths. Each section's volume follows its
                       headcount; a wiped-out section stops dead. The doomed team (it can't catch anything any more)
                       stutters on 8ths, fewer and fewer as its pieces are eaten.
    the hook           the burst at frame 0 = a big timpani hit; ROCK. PAPER. SCISSORS. each lands with a timpani hit
                       plus its own section (taiko / pizzicato chord / brass stab)
    the bed            a low cello + bass drone from frame 0 (Dm | Bb | Gm | A), a heartbeat timpani from bar 2,
                       a staccato low-string ostinato (8ths, then 16ths as the chase heats up), dark string pads
    the story          wipe-out: a big hit and half a bar of held breath; the twist's last line: a low brass sting,
                       then a ticking pulse and rising tremolo strings through the mop-up; the last catch: a full
                       orchestra hit; the winner's dark fanfare (led by the winner's section); 57-61 s a timpani roll
                       and string swell into the burst at frame 0
    caption thuds      reelkit.audio.thud() on every impact caption landing

    python audio.py    # -> audio.wav, prints section levels and the sections-over-bed band check"""
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
import rps

SR = A.SR
DUR = T.DUR
NS = int(DUR * SR)
BEAT, S16, BAR = T.BEAT, T.S16, T.BAR
E8 = BEAT / 2
RNG = np.random.default_rng(22)
D3 = 50

# chords per bar (root midi near D2..A2, quality): hook drone, the cycle, the theme, the swell
DM, BB, GM, AM = (38, 'm'), (34, 'M'), (43, 'm'), (45, 'M')
CHORDS = [DM, DM] + [DM, BB, GM, AM] * 5 + [DM, BB, GM, AM] + [DM, AM]
assert len(CHORDS) == 28


def chord_at(t):
    return CHORDS[int((t % DUR) / BAR) % 28]


def tones(ch):
    r, q = ch
    return [r, r + (3 if q == 'm' else 4), r + 7]


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
        if gain <= 0:
            return
        b = self.get(k)
        i0 = int(round((t % DUR) * SR))
        a = (pan + 1) * math.pi / 4
        st = np.stack([sig * math.cos(a), sig * math.sin(a)], 1) * gain
        n = min(len(st), len(b) - i0)
        b[i0:i0 + n] += st[:n]

    def fold(self):
        for k, b in self.b.items():
            b[:len(b) - NS] += b[NS:]
            self.b[k] = b[:NS].copy()


# ------------------------------------------------------------------ instruments (cached)
_C = {}


def cached(key, fn):
    if key not in _C:
        _C[key] = fn()
    return _C[key]


def timpani(m, dur=2.4):
    def mk():
        n = int(dur * SR)
        t = tt(n)
        f0 = f_of(m)
        glide = 1 + 0.025 * np.exp(-t / 0.03)
        out = np.zeros(n)
        for r, a, d in [(1.0, 1.0, 1.5), (1.504, 0.55, 0.9), (1.742, 0.25, 0.6), (2.0, 0.32, 0.55), (2.44, 0.16, 0.35), (2.9, 0.1, 0.25)]:
            out += a * np.sin(2 * np.pi * np.cumsum(f0 * r * glide) / SR) * np.exp(-t / d)
        mallet = A.lp(RNG.normal(0, 1, n), 1400) * np.exp(-t / 0.012) * 0.8
        x = out + mallet
        return x * np.clip(t / 0.002, 0, 1) / 1.6
    return cached(('timp', m, dur), mk)


def taiko():
    def mk():
        n = int(0.9 * SR)
        t = tt(n)
        f = 52 + 90 * np.exp(-t / 0.05)
        body = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.32)
        slap = A.bp(np.random.default_rng(5).normal(0, 1, n), 250, 1600) * np.exp(-t / 0.03) * 0.7
        return (body + slap) * np.clip(t / 0.001, 0, 1)
    return cached('taiko', mk)


def pizz(m, bright=1.0):
    def mk():
        n = int(0.7 * SR)
        t = tt(n)
        f = f_of(m)
        x = np.zeros(n)
        for h in range(1, 9):
            x += (1.0 / h) * np.sin(2 * np.pi * f * h * t + h * 0.7) * np.exp(-t * (5.5 + 2.2 * h * bright))
        thump = A.lp(np.random.default_rng(int(m)).normal(0, 1, n), 900) * np.exp(-t / 0.006) * 0.25
        return (x + thump) * np.clip(t / 0.0015, 0, 1) * 0.7
    return cached(('pizz', m, bright), mk)


def strings(ms, dur, att=0.12, rel=0.25, trem=0.0, cut=2600, vib=0.004, seed=0):
    """bowed string section: detuned saw ensemble, vibrato, optional bowed tremolo (0..1)"""
    n = int(dur * SR)
    t = tt(n)
    r = np.random.default_rng(seed)
    out = np.zeros(n)
    for m in ms:
        for v in range(4):
            f = f_of(m) * (1 + 0.004 * (v - 1.5))
            ph = 2 * np.pi * np.cumsum(f * (1 + vib * np.sin(2 * np.pi * (5.2 + 0.4 * v) * t + r.uniform(0, 6)))) / SR
            out += (2 * ((ph / (2 * np.pi) + r.uniform()) % 1.0) - 1)
    out /= 4 * max(1, len(ms))
    out = A.lp(A.hp(out, 70), cut)
    env = np.minimum(1, t / max(att, 1e-3)) * np.minimum(1, (dur - t) / max(rel, 1e-3)).clip(0, 1)
    if trem > 0:
        tr = np.abs(np.sin(np.pi * 6.6 * t)) ** 0.6
        env = env * (1 - trem + trem * tr)
    return out * env


def brass(ms, dur=0.42, sting=False):
    def mk():
        n = int(dur * SR)
        t = tt(n)
        x = np.zeros(n)
        for m in ms:
            f = f_of(m)
            for v in range(2):
                x += 2 * ((t * f * (1 + 0.003 * (v - 0.5)) + 0.3 * v) % 1.0) - 1
        x /= 2 * len(ms)
        # filter envelope: the brassy "blat" (blocks of 128 samples with a moving one-pole lowpass, run twice)
        if sting:
            fc = 260 + 1500 * (1 - np.exp(-t / 0.35)) * np.exp(-np.maximum(t - 0.6, 0) / 1.4)
        else:
            fc = 380 + 3200 * np.exp(-t / 0.06) + 500 * np.exp(-t / 0.3)
        y = np.zeros(n)
        for _ in range(2):
            z, src = 0.0, (x if _ == 0 else y.copy())
            for i0 in range(0, n, 128):
                a = math.exp(-2 * math.pi * fc[i0] / SR)
                seg, zi = lfilter([1 - a], [1, -a], src[i0:i0 + 128], zi=[z])
                y[i0:i0 + 128] = seg
                z = zi[0]
        if sting:
            env = np.minimum(1, t / 0.05) * np.exp(-np.maximum(t - 0.4, 0) / 0.9)
        else:
            env = np.minimum(1, t / 0.012) * np.exp(-t / 0.16) * 0.8 + np.minimum(1, t / 0.012) * 0.2 * np.exp(-t / 0.5)
        env *= np.minimum(1, (dur - t) / 0.03).clip(0, 1)
        return y * env * 2.2
    return cached(('brass', tuple(ms), dur, sting), mk)


def tick():
    def mk():
        n = int(0.06 * SR)
        t = tt(n)
        x = A.bp(np.random.default_rng(9).normal(0, 1, n), 1700, 2600, 2) * np.exp(-t / 0.008)
        return x + np.sin(2 * np.pi * 2100 * t) * np.exp(-t / 0.01) * 0.4
    return cached('tick', mk)


def boom():
    def mk():
        n = int(2.5 * SR)
        t = tt(n)
        f = 34 + 70 * np.exp(-t / 0.08)
        return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.9) + A.lp(RNG.normal(0, 1, n), 400) * np.exp(-t / 0.15) * 0.5
    return cached('boom', mk)


# ------------------------------------------------------------------ headcount -> section gain
FR = T.FPS
CNT = T.COUNT
_sm = uniform_filter1d(CNT, size=int(0.5 * FR), axis=0, mode='nearest')


def sec_gain(q, t):
    if T.out_now(q, t):
        return 0.0
    c = _sm[T.frame_of(t), q]
    return float(np.clip((c / 40.0) ** 0.55, 0.0, 1.5))


def team_count(q, t):
    return CNT[T.frame_of(t), q]


# ------------------------------------------------------------------ the sections, stepped by catches
def triggers(grid):
    """{team: sorted list of grid slots (quantised forward) with at least one catch by that team}"""
    out = {q: set() for q in range(3)}
    for t, h, c, nt, ot in T.CATCH:
        k = int(math.ceil(t / grid - 1e-6))
        out[int(nt)].add(k)
    return {q: sorted(s) for q, s in out.items()}


PAN = {0: -0.15, 1: -0.45, 2: 0.45}


def section_rock(B, slots):
    """ROCK: timpani on the chord root / fifth, a taiko on every 4th step"""
    for i, k in enumerate(slots):
        t = k * E8
        if t >= DUR:
            continue
        g = sec_gain(0, t)
        ch = chord_at(t)
        m = ch[0] if i % 2 == 0 else ch[0] + 7
        while m > 45:
            m -= 12
        acc = 1.0 if (k % 2 == 0) else 0.7
        B.place('rock', timpani(m, 1.6), t, 0.55 * g * acc, PAN[0])
        if i % 4 == 0:
            B.place('rock', taiko(), t, 0.5 * g, PAN[0] + 0.1)


PAPER_PAT = [0, 2, 1, 2, 0, 3, 2, 1]          # chord-tone indices (3 = octave root)


def section_paper(B, slots):
    """PAPER: pizzicato arpeggios on 16ths; as the section grows, tremolo strings swell under them"""
    for i, k in enumerate(slots):
        t = k * S16
        if t >= DUR:
            continue
        g = sec_gain(1, t)
        ch = chord_at(t)
        ts = tones(ch) + [ch[0] + 12]
        m = ts[PAPER_PAT[i % 8]] + 24
        acc = 1.0 if k % 4 == 0 else 0.75
        B.place('paper', pizz(m), t, 0.5 * g * acc, PAN[1] + 0.1 * ((i % 3) - 1))
    # tremolo bed: per beat, scaled by how big the section is (pizz turns into tremolo as it grows)
    nb = int(DUR / BEAT)
    for b in range(nb):
        t = b * BEAT
        c = team_count(1, t)
        if T.out_now(1, t) or c < 30:
            continue
        k = min(1.0, (c - 30) / 50)
        ch = chord_at(t)
        ms = [tones(ch)[1] + 24, tones(ch)[2] + 24]
        B.place('paper', strings(ms, BEAT + 0.05, att=0.03, rel=0.05, trem=0.85, cut=3200, seed=b), t, 0.22 * k, PAN[1])


SCI_PAT = [2, 2, 1, 2, 3, 2, 1, 0]


def section_scissors(B, slots):
    """SCISSORS: brass stabs (three-note chords, the top voice walking a riff)"""
    for i, k in enumerate(slots):
        t = k * E8
        if t >= DUR:
            continue
        g = sec_gain(2, t)
        ch = chord_at(t)
        ts = tones(ch)
        top = (ts + [ch[0] + 12])[SCI_PAT[i % 8]] + 12
        ms = [ts[0] + 12, ts[1] + 12, top + 12 if top < ts[1] + 12 else top]
        acc = 1.0 if k % 2 == 0 else 0.72
        B.place('sci', brass(tuple(ms), 0.36), t, 0.42 * g * acc, PAN[2])


def doomed_stutter(B):
    """the doomed team can't catch anything after the wipe-out: its section stutters on 8ths, fewer and fewer"""
    doomed = T.KILLER
    c0 = max(1.0, team_count(doomed, T.T1 + 0.05))
    k0 = int(math.ceil(T.T1 / E8)) + 2
    for k in range(k0, int(T.T_WIN / E8)):
        t = k * E8
        c = team_count(doomed, t)
        if c <= 0:
            break
        p = 0.25 + 0.6 * (c / c0)
        if RNG.random() > p:
            continue
        g = 0.55 * max(0.25, (c / c0) ** 0.5)
        ch = chord_at(t)
        if doomed == 0:
            m = ch[0] if k % 2 == 0 else ch[0] + 7
            while m > 45:
                m -= 12
            B.place('rock', timpani(m, 1.2), t, 0.5 * g, PAN[0])
        elif doomed == 1:
            B.place('paper', pizz(tones(ch)[k % 3] + 24), t, 0.5 * g, PAN[1])
        else:
            ts = tones(ch)
            B.place('sci', brass((ts[0] + 12, ts[1] + 12, ts[2] + 12), 0.3), t, 0.4 * g, PAN[2])


# ------------------------------------------------------------------ the bed
def drone(B):
    for b in range(28):
        t = b * BAR
        ch = CHORDS[b]
        r = ch[0]
        if b == 26:                                    # the swell bars hold the tonic under the roll
            r = 38
        B.place('bed', strings([r, r + 12], BAR + 0.3, att=0.25, rel=0.3, cut=900, seed=100 + b), t, 0.5)
        B.place('bed', A.lp(A.bass(r - 12, BAR + 0.2), 300), t, 0.28)
        if b >= 4:                                     # dark mid pad from bar 4
            k = min(1.0, (b - 3) / 6)
            ms = [tones(ch)[1] + 12, tones(ch)[2] + 12]
            B.place('pad', strings(ms, BAR + 0.4, att=0.6, rel=0.5, cut=1500, seed=200 + b), t, 0.18 * k)


def heartbeat(B):
    """timpani heartbeat on beats 1 and 3 from bar 2 to the win"""
    for b in range(2, 22):
        for beat, gain in ((0, 0.42), (2, 0.3)):
            t = b * BAR + beat * BEAT
            if t >= T.T_WIN - 0.05:
                break
            if T.T1 <= t < T.T1 + 1.2:                # held breath after the wipe-out
                continue
            B.place('beat', timpani(38, 1.4), t, gain, 0.0)


def ostinato(B):
    """staccato low strings on the chord root: 8ths from bar 4, 16ths from bar 8 (the pulse tightens)"""
    t = 4 * BAR
    while t < T.T_WIN - 0.05:
        bar = int(t / BAR)
        step = E8 if bar < 8 else S16
        if not (T.T1 <= t < T.T1 + 1.2):
            ch = chord_at(t)
            m = ch[0] + 12
            k = int(round(t / S16))
            acc = 1.0 if k % 4 == 0 else 0.6
            g = 0.16 * min(1.0, 0.5 + (bar - 4) / 10) * acc
            B.place('ost', strings([m], step * 0.75, att=0.006, rel=0.04, cut=1800, vib=0.0, seed=k % 16), t, g, -0.1)
        t += step


# ------------------------------------------------------------------ story moments
def hook(B):
    # the burst at frame 0, then one hit per word
    B.place('fx', timpani(38, 2.6), 0.0, 0.9)
    B.place('fx', boom(), 0.0, 0.35)
    B.place('fx', strings([38, 50], 2.4, att=0.02, rel=0.6, cut=900, seed=7), 0.0, 0.35)
    for q, tl in enumerate(T.HOOK_LAND):
        B.place('fx', timpani([38, 41, 45][q], 1.8), tl, 0.6)
        if q == 0:
            B.place('rock', taiko(), tl, 0.6, PAN[0])
        elif q == 1:
            for j, m in enumerate([62, 65, 69]):
                B.place('paper', pizz(m), tl + 0.012 * j, 0.45, PAN[1])
        else:
            B.place('sci', brass((62, 65, 69), 0.5), tl, 0.5, PAN[2])


def wipeout(B):
    t = T.T1
    B.place('fx', boom(), t, 0.6)
    B.place('fx', timpani(38, 2.4), t, 0.8)
    B.place('fx', A.crash(2.5), t, 0.14, 0.2)


def twist(B):
    t = T.T_TWIST_LAST
    B.place('fx', brass((26, 33, 38, 39), 2.6, sting=True), t, 0.85)           # low brass sting (D, A, D, Eb)
    B.place('fx', timpani(38, 2.4), t, 0.5)
    k0 = int(math.ceil((t + 1.2) / E8))
    k1 = int(T.T_WIN / E8)
    for k in range(k0, k1):                                                    # the ticking pulse
        tk = k * E8
        B.place('tick', tick(), tk, 0.35 if k % 2 == 0 else 0.22, 0.25)
    # rising tremolo strings through the mop-up, a semitone per bar
    tb = t + 1.0
    m = 62
    while tb < T.T_WIN - 0.1:
        d = min(BAR, T.T_WIN - tb)
        k = (tb - t) / max(1.0, T.T_WIN - t)
        B.place('rise', strings([m, m + 7], d + 0.05, att=0.15, rel=0.08, trem=0.9, cut=3400, seed=int(m)), tb, 0.12 + 0.18 * k, -0.3)
        tb += BAR
        m += 1


THEME = [(62, 1.5), (57, 0.5), (62, 1), (65, 1), (69, 2), (67, 1), (65, 1), (67, 1.5), (70, 0.5), (69, 1), (67, 1), (69, 3), (73, 1)]


def win(B):
    t = math.ceil(T.T_WIN / S16) * S16
    B.place('fx', boom(), t, 0.7)
    B.place('fx', timpani(38, 2.6), t, 0.9)
    B.place('fx', A.crash(3.0), t, 0.18)
    B.place('fx', brass((38, 50, 53, 57), 1.4, sting=True), t, 0.6)
    B.place('fx', strings([50, 57, 62, 65], 1.4, att=0.01, rel=0.4, cut=3000, seed=3), t, 0.4)
    # roll into the theme
    t_theme = 22 * BAR
    tr = t + 0.4
    i = 0
    while tr < t_theme - 0.02:
        u = (tr - t) / max(0.1, t_theme - t)
        B.place('fx', timpani(38, 0.8), tr, 0.15 + 0.3 * u)
        tr += S16 / 2 if i % 2 else S16 / 2
        i += 1
    # the winner's dark fanfare (bars 22-25), led by the winner's section, horns + low strings under it
    w = T.WINNER
    tb = t_theme
    for m, d in THEME:
        dur = d * BEAT
        if w == 1:
            B.place('lead', strings([m, m + 12], dur + 0.08, att=0.05, rel=0.12, trem=0.0, cut=3600, vib=0.006, seed=m), tb, 0.42, -0.3)
        elif w == 2:
            B.place('lead', brass((m, m + 12), dur + 0.05, sting=True), tb, 0.5, 0.3)
        else:
            B.place('lead', brass((m - 12, m), dur + 0.05, sting=True), tb, 0.45, -0.1)
            B.place('rock', timpani(38 if m % 12 in (2, 9) else 45, 1.2), tb, 0.35, PAN[0])
        B.place('lead', brass((m - 12,), dur + 0.05, sting=True), tb, 0.2, 0.2)       # horns double an octave down
        tb += dur
    for b in range(22, 26):
        ch = CHORDS[b]
        ts = tones(ch)
        B.place('lead', strings([ts[0] + 12, ts[1] + 12, ts[2] + 12], BAR + 0.1, att=0.08, rel=0.2, cut=2200, seed=300 + b), b * BAR, 0.2)
        B.place('lead', brass((ts[0], ts[0] + 12), BAR * 0.95, sting=True), b * BAR, 0.25)
        for beat in (0, 2):
            B.place('beat', timpani(38 if beat == 0 else 45, 1.6), b * BAR + beat * BEAT, 0.5)
        if w == 1:                                                         # paper: pizzicato 8ths under the theme
            for j in range(8):
                B.place('paper', pizz(ts[j % 3] + 24), b * BAR + j * E8, 0.32, PAN[1])
        elif w == 2:
            for j in (0, 3, 6):
                B.place('sci', brass((ts[0] + 12, ts[1] + 12, ts[2] + 12), 0.3), b * BAR + j * E8, 0.3, PAN[2])
        else:
            for j in (0, 3, 4, 6):
                B.place('rock', taiko(), b * BAR + j * E8, 0.35, PAN[0])


def swell(B):
    """bars 26-27: the arrival on D, then a timpani roll and a rising string swell into the burst at frame 0"""
    t0 = 26 * BAR
    B.place('lead', strings([62, 74], BAR * 1.0, att=0.05, rel=0.4, cut=3600, seed=11), t0, 0.35, -0.2)
    B.place('lead', brass((50, 57, 62), BAR * 1.1, sting=True), t0, 0.35)
    B.place('beat', timpani(38, 2.0), t0, 0.6)
    tr = t0 + BEAT
    while tr < DUR - 0.01:
        u = (tr - t0) / (DUR - t0)
        B.place('roll', timpani(38 if tr < 27 * BAR else 45, 0.6), tr, 0.08 + 0.42 * u ** 1.6, 0.0)
        tr += S16 / 2
    n = int((DUR - 27 * BAR) * SR)
    sw = strings([45, 52, 57], DUR - 27 * BAR + 0.02, att=DUR - 27 * BAR, rel=0.01, trem=0.6, cut=2800, seed=12)
    B.place('roll', sw, 27 * BAR, 0.45)
    B.place('fx', A.whoosh(1.6, 200, 3000), DUR - 1.6, 0.12)


def thuds(B):
    for k, (t, line, nch) in enumerate(T.LANDINGS):
        B.place('thud', A.thud(), t, 0.5 / (1 + 0.25 * line))


# ------------------------------------------------------------------ mix
BUS_GAIN = {'bed': 0.55, 'pad': 0.6, 'beat': 0.7, 'ost': 0.6, 'rock': 0.85, 'paper': 0.9, 'sci': 0.75, 'fx': 0.8, 'tick': 0.6,
            'rise': 0.7, 'lead': 0.8, 'roll': 0.7, 'thud': 0.55}
REVERB = {'bed': 0.3, 'pad': 0.6, 'paper': 0.35, 'sci': 0.3, 'rock': 0.25, 'fx': 0.45, 'rise': 0.5, 'lead': 0.5, 'roll': 0.3, 'beat': 0.2}
BED = ['bed', 'pad', 'beat', 'ost']
SECTIONS = ['rock', 'paper', 'sci']
TARGET_DB = -12.5


def master(B):
    dry = sum(b * BUS_GAIN.get(k, 1.0) for k, b in B.b.items())
    send = sum(B.b[k] * v * BUS_GAIN.get(k, 1.0) for k, v in REVERB.items() if k in B.b)
    wet = A.reverb(np.concatenate([send, send]), dur=2.8, decay=0.85)[NS:]
    x = dry + wet * 0.3
    ra, rb = int(10 * SR), int(T.T_WIN * SR)
    ref = np.sqrt((x[ra:rb] ** 2).mean())
    thr = ref * 10 ** (-5 / 20)
    xl = np.concatenate([x[-SR:], x])
    a = math.exp(-1.0 / (0.08 * SR))
    e = np.sqrt(lfilter([1 - a], [1, -a], (xl ** 2).mean(1)).clip(1e-12))[SR:]
    x = x * np.where(e > thr, (thr / e) ** 0.3, 1.0)[:, None]
    ref = np.sqrt((x[ra:rb] ** 2).mean())
    x = x * (10 ** (TARGET_DB / 20) / ref)
    # section automation after the compressor, so the build survives: a loud hook, sparse-but-tense middle, the climb
    pts = [(0, -1.5), (2.6, -1.5), (3.6, -8.0), (10, -7.5), (20, -5.5), (T.T1, -2.0), (T.T_WIN - 0.2, 0.0), (T.T_WIN, 1.5), (57.0, 1.5),
           (60.6, 0.5), (DUR, -1.0)]
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
    drone(B)
    heartbeat(B)
    ostinato(B)
    hook(B)
    tr8 = triggers(E8)
    tr16 = triggers(S16)
    section_rock(B, tr8[0])
    section_paper(B, tr16[1])
    section_scissors(B, tr8[2])
    doomed_stutter(B)
    wipeout(B)
    twist(B)
    win(B)
    swell(B)
    thuds(B)
    B.fold()
    stems = {k: v.copy() for k, v in B.b.items()}
    out = master(B)
    path = os.path.join(HERE, 'audio.wav')
    A.write(path, out)
    spans = [(0, 3), (3, 10), (10, 20), (20, T.T1), (T.T1, T.T_WIN), (T.T_WIN, 57.2), (57.2, DUR)]
    print('section levels (dBFS rms / peak):')
    for a_, b_, rms, pk in A.section_levels(path, spans):
        print(f'  {a_:5.1f}-{b_:5.1f}s  rms {rms:6.1f}  peak {pk:5.1f}')
    from scipy.signal import butter, sosfilt
    sec = sum(stems[k] * BUS_GAIN[k] for k in SECTIONS if k in stems).mean(1)
    bed = sum(stems[k] * BUS_GAIN[k] for k in BED if k in stems).mean(1)
    lv = lambda x: 20 * np.log10(np.sqrt((x ** 2).mean()) + 1e-12)
    for lo, hi in ((1000, 5000), (60, 16000)):
        sos = butter(4, [lo, hi], 'bp', fs=SR, output='sos')
        sb, bb = sosfilt(sos, sec), sosfilt(sos, bed)
        row = []
        for a_, b_ in spans[1:5]:
            i, j = int(a_ * SR), int(b_ * SR)
            row.append(f'{a_:4.0f}-{b_:4.0f}s {lv(sb[i:j]) - lv(bb[i:j]):+5.1f}')
        print(f'  sections over bed, {lo}-{hi} Hz: ' + '  '.join(row))
    print('triggers per team (8ths/16ths):', {rps.NAMES[q]: (len(tr8[q]), len(tr16[q])) for q in range(3)})


if __name__ == '__main__':
    main()
