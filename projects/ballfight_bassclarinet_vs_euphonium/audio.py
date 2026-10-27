"""#38 BALLFIGHT #12: Bass Clarinet vs Euphonium - soundtrack (mood A: dark drumline), all synthesised, original.

The bed is a minor marching-band battle cadence with NO low brass or woodwinds of its own: a snare line (accents,
taps, flams, diddle rolls), quads / tenors (offbeat answers, sweeps later), five pitched bass drums (unison hits and
split runs on the chord roots), a cymbal line (crashes on the phrase, choked crunches later), and the front-ensemble
pit for the harmony: a high marimba ostinato and timpani. C minor at 125.90 BPM (32 bars = 61 s). Both fighters are
played from the honest run in their own new Pixel Pitch voices:
  BASS CLARINET  the rumbling bottom: the peg going in plays the bar's root with a growl; every 8th of a whirl plays
                 the chalumeau rumble (timeline.BASS_8THS); a smash pops a slap tongue on that pitch; MULTIPHONICS adds
                 the twelfth above every rumble note (two notes at once); SLAP TONGUE is the big slap; LIGHTS OUT a low
                 growl swelling through the dark bar, the eruption a huge slap over a thunder thud.
  EUPHONIUM      the warm tenor melody: every salvo plays the melody note of its beat (timeline.MELODY; MARCHING ORDERS
                 = a salvo every beat = the whole melody), a landed salvo a fat chord tone a sixth under; NOT A TUBA a
                 falling run, one note per shell; IDENTITY CRISIS a puzzled rising bend, then the copied move in the
                 euphonium's voice (a brass slap; MULTIPHONICS: its salvo notes doubled a twelfth up).
As HP drops a fighter's part thins and dirties; the leader's part is fuller and louder.
Consistent loudness from frame 0 (project rule, Oct 26): the drumline is at full level on the title card; it builds
with density (taps -> flams and rolls, tenor sweeps, bass-drum splits, cymbal crunches), never with level. One master
gain + a compressor and a soft limiter: no section automation. Bar 0 is the VS title card (a bass-clarinet growl +
slap, the thunder crack, a euphonium blast, each over a unison drumline hit; a snare roll as the doors open), bar 1
the snare line's count-in, the drop on FIGHT!. At the K.O. the bed tape-stops into a held hit, the loser falls apart,
the winner's stinger and solo play over a lighter cadence; the last bars build into the title slam on sample 0.
Every hit is snapped to the nearest 16th; tails past the loop fold onto the start.

    python audio.py            -> audio.wav (whole AAC frames) + stems in check/ for the band check"""
import math
import os
import sys
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', '..'))
sys.path.insert(0, HERE)
sys.path.append(os.path.join(HERE, '..', 'ballfight'))
sys.path.append(os.path.join(HERE, '..', 'which_instrument_wins'))
from reelkit import audio as A
from reelkit.ui import impact_landings
import timeline as T
import render as RD
import pp_bridge as B

SR = A.SR
RUN, EV = RD.RUN, RD.EV
NAMES, WIN, LOSE, T_KO = RD.NAMES, RD.WIN, RD.LOSE, RD.T_KO
KB, KE = NAMES.index('BASS CLARINET'), NAMES.index('EUPHONIUM')
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames: loops sample-perfectly
S16 = T.S16
BCM = B.module('bassclarinet_lowpoly')                  # the new Pixel Pitch bass clarinet (read-only)
EUM = B.module('euphonium_lowpoly')                     # the new Pixel Pitch euphonium


def q16(t):
    return round(t / S16) * S16


M64 = (1 << 64) - 1


def h01(*xs):
    """deterministic hash in [0, 1) (splitmix64: neighbouring slots get unrelated values, #37's fix)"""
    z = 0x9E3779B97F4A7C15
    for x in xs:
        z = (z ^ (int(round(x * 1000)) & M64)) * 0xBF58476D1CE4E5B9 & M64
        z = (z + 0x9E3779B97F4A7C15) & M64
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & M64
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & M64
        z ^= z >> 31
    return z / float(1 << 64)


def thin(slot, key, hp, prev_dropped):
    """drop this note? (the hurt fighter's line thins: up to 30 % of its notes at low HP, never two in a row)"""
    return not prev_dropped and h01(slot, key) < 0.30 * (1 - hp) ** 1.6


# ---------------------------------------------------------------- the two fighters' voices (cached, level-matched)
_VC = {}


def _fit(x, cap):
    n = int(cap * SR)
    x = np.asarray(x, float)[:n].copy()
    if len(x) < n:
        x = np.pad(x, (0, n - len(x)))
    fd = min(len(x) // 3, int(0.03 * SR))
    x[-fd:] *= np.linspace(1, 0, fd)
    return x


def _norm(x, ref=0.16):
    r = np.sqrt((x[:int(0.25 * SR)] ** 2).mean()) + 1e-9
    return x * (ref / r)


def bcl(m, dur, vel=0.85, growl=0.0):
    """one bass clarinet note (fractional midi ok), dur s, from the new series voice"""
    key = ('b', round(m, 3), round(dur, 3), round(vel, 2), round(growl, 2))
    if key not in _VC:
        x = BCM.bass_reed(BCM._midi_hz(m), dur, vel, growl=growl)
        _VC[key] = _norm(_fit(B._resample(np.asarray(x, float)), dur + 0.16))
    return _VC[key]


def slap(m, vel=1.0):
    key = ('s', round(m, 2), round(vel, 2))
    if key not in _VC:
        x = BCM.slap(BCM._midi_hz(m), vel)
        _VC[key] = B._resample(np.asarray(x, float)) * 0.9
    return _VC[key]


def eup(m, dur, vel=0.85):
    key = ('e', round(m, 3), round(dur, 3), round(vel, 2))
    if key not in _VC:
        x = EUM.euph(EUM._midi_hz(m), dur, vel)
        _VC[key] = _norm(_fit(B._resample(np.asarray(x, float)), dur + 0.16))
    return _VC[key]


def bend(x, semis, t0, t1):
    """bend the pitch by `semis` between t0 and t1 s into the note (time warp)"""
    n = len(x)
    t = A.tt(n)
    off = semis * np.clip((t - t0) / max(1e-3, t1 - t0), 0, 1)
    rate = 2 ** (off / 12)
    pos = np.cumsum(rate) - rate[0]
    return np.interp(pos, np.arange(n), x, right=0.0)


def hp_at(k, t):
    return RD.hp_at(k, t)


def colour(x, k, t, drive_k=1.0):
    """the fighter's tone at time t: thinner + dirtier as its HP drops; returns (signal, gain, lead)"""
    hp = hp_at(k, t) / 100.0
    lead = (hp_at(k, t) - hp_at(1 - k, t)) / 100.0
    cut = 40 + 200 * (1 - hp) ** 1.6
    y = A.hp(x, cut)
    drive = 1 + 4.0 * drive_k * (1 - hp) ** 2
    y = np.tanh(drive * y * 1.2) / np.tanh(drive * 1.2) * (1.0 / (1 + 0.15 * (drive - 1)))
    g = 10 ** (float(np.clip(6.0 * lead, -4.0, 3.5)) / 20)
    return y, g, lead


# ---------------------------------------------------------------- the drumline + pit (original)
_BC = {}


def msnare(kind='A'):
    """a marching snare: high-tension crack, tight wires, a little ring; kind A accent / t tap / g ghost"""
    key = ('sn', kind)
    if key not in _BC:
        n = int(0.28 * SR)
        t = A.tt(n)
        r = np.random.default_rng(5 + ord(kind))
        wires = A.bp(r.normal(0, 1, n), 1800, 9500) * np.exp(-t / (0.075 if kind == 'A' else 0.05))
        crack = A.bp(r.normal(0, 1, n), 3000, 11000) * np.exp(-t / 0.004) * (1.6 if kind == 'A' else 0.8)
        body = (0.5 * np.sin(2 * np.pi * 380 * t) + 0.6 * np.sin(2 * np.pi * 620 * t)) * np.exp(-t / 0.03)
        x = (0.8 * wires + crack + 0.7 * body) * np.clip(t / 0.0008, 0, 1)
        _BC[key] = x * {'A': 1.0, 't': 0.42, 'g': 0.20, 'f': 1.0}[kind]
    return _BC[key]


def tenor(i):
    """quads: four pitched tenor drums (high to low) - a bright stick attack, a short pitched body"""
    key = ('tn', i)
    if key not in _BC:
        f = (494.0, 415.0, 349.0, 294.0)[i]          # tuned high: the bass clarinet owns the low mids
        n = int(0.32 * SR)
        t = A.tt(n)
        fr = f * (1 + 0.12 * np.exp(-t / 0.02))
        body = np.sin(2 * np.pi * np.cumsum(fr) / SR) * np.exp(-t / 0.13)
        stick = A.bp(np.random.default_rng(31 + i).normal(0, 1, n), 1500, 7000) * np.exp(-t / 0.004)
        _BC[key] = A.hp(0.7 * body + 0.8 * stick, 260)
    return _BC[key]


BD_ROOTS = {'Cm': (60, 55, 51, 48, 43), 'Ab': (63, 60, 56, 51, 44), 'Fm': (60, 56, 53, 48, 41), 'G': (62, 59, 55, 50, 43)}


def bassdrum(m, vel=1.0):
    """a pitched marching bass drum (felt mallet): a low tuned body (the midi note's pitch) + a soft beater"""
    key = ('bd', int(m))
    if key not in _BC:
        f = A.mtof(m - 24)                                         # two octaves under the label: the drum's real note
        n = int(0.45 * SR)
        t = A.tt(n)
        fr = f * (1 + 0.25 * np.exp(-t / 0.015))
        body = np.sin(2 * np.pi * np.cumsum(fr) / SR) * np.exp(-t / 0.22)
        beat = A.lp(np.random.default_rng(int(m)).normal(0, 1, n), 500) * np.exp(-t / 0.008)
        _BC[key] = (body + 0.35 * beat) * np.clip(t / 0.002, 0, 1)
    return _BC[key] * vel


def choke(seed=0):
    n = int(0.22 * SR)
    t = A.tt(n)
    x = A.bp(np.random.default_rng(40 + seed).normal(0, 1, n), 3000, 12000)
    return x * np.exp(-t / 0.045) * np.clip(t / 0.002, 0, 1)


def marimba(m, vel=1.0):
    key = ('ma', m)
    if key not in _BC:
        n = int(0.7 * SR)
        t = A.tt(n)
        f = A.mtof(m)
        x = (np.sin(2 * np.pi * f * t) * np.exp(-t / 0.35) + 0.25 * np.sin(2 * np.pi * 3.93 * f * t) * np.exp(-t / 0.06)
             + 0.08 * np.sin(2 * np.pi * 9.2 * f * t) * np.exp(-t / 0.02))
        click = A.bp(np.random.default_rng(m).normal(0, 1, n), 1000, 4000) * np.exp(-t / 0.003) * 0.3
        _BC[key] = (x + click) * np.clip(t / 0.001, 0, 1)
    return _BC[key] * vel


def timp(m, vel=1.0):
    key = ('t', m)
    if key not in _BC:
        n = int(1.1 * SR)
        t = A.tt(n)
        f = A.mtof(m)
        fr = f * (1 + 0.04 * np.exp(-t / 0.05))
        ph = 2 * np.pi * np.cumsum(fr) / SR
        x = (np.sin(ph) + 0.3 * np.sin(1.5 * ph) * np.exp(-t / 0.2) + 0.12 * np.sin(2 * ph) * np.exp(-t / 0.15))
        x = x * np.exp(-t / 0.55) * np.clip(t / 0.003, 0, 1)
        mallet = A.lp(np.random.default_rng(m + 5).normal(0, 1, n), 700) * np.exp(-t / 0.012) * 0.5
        _BC[key] = x + mallet
    return _BC[key] * vel


def thunder(dur=1.6, seed=0, crack=True):
    n = int(dur * SR)
    t = A.tt(n)
    r = np.random.default_rng(seed)
    rumble = A.lp(r.normal(0, 1, n), 140) * 3.0 * np.clip(t / 0.03, 0, 1) * np.exp(-t / (dur * 0.35))
    rumble *= 0.7 + 0.3 * np.sin(2 * np.pi * 3.1 * t + seed)
    x = rumble
    if crack:
        x = x + A.bp(r.normal(0, 1, n), 1500, 9000) * np.exp(-t / 0.03) * 0.9
    return x


def clank(m, seed=0):
    n = int(0.35 * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = sum(np.sin(2 * np.pi * f * r * t) * np.exp(-t / d) for r, d in ((1.0, 0.10), (2.76, 0.05), (5.4, 0.025)))
    return x + 0.3 * A.bp(np.random.default_rng(seed).normal(0, 1, n), 2000, 8000) * np.exp(-t / 0.005)


def boom(seed=0, dur=0.5):
    """a shell landing: a short thud + a crunchy noise burst"""
    n = int(dur * SR)
    t = A.tt(n)
    th = A.lp(A.thud(dur, 110, 40, seed=seed)[:n], 140)
    th = np.pad(th, (0, n - len(th)))
    crunch = A.bp(np.random.default_rng(seed + 9).normal(0, 1, n), 900, 5000) * np.exp(-t / 0.05)
    return th + 0.5 * crunch


def roar(dur, seed=3):
    n = int(dur * SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    x = A.bp(x, 300, 2600) * 0.5 + A.bp(x, 900, 5000) * 0.25
    t = A.tt(n)
    return x * np.clip(t / 0.25, 0, 1) * np.exp(-np.maximum(t - 0.6, 0) / (dur * 0.45))


def riser(dur, over=0.012, seed=11):
    """noise + saw riser that builds right up to (and `over` s past) its end: no fade, the loop seam stays loud"""
    n = int((dur + over) * SR)
    t = A.tt(n)
    u = np.clip(t / dur, 0, 1.05)
    x = np.random.default_rng(seed).normal(0, 1, n)
    f = 300 * (20 ** u)
    ph = 2 * np.pi * np.cumsum(55 * 2 ** (u * 2.5)) / SR
    sweep = (2 * ((ph / (2 * np.pi)) % 1) - 1)
    out = np.zeros(n)
    nb = 20
    for k in range(nb):
        a, b = int(k * n / nb), int((k + 1) * n / nb)
        fc = float(f[min(n - 1, (a + b) // 2)])
        out[a:b] = A.bp(x[max(0, a - 2048):b], fc * 0.7, min(fc * 1.4, SR / 2 - 200))[-(b - a):]
    return (out + 0.35 * A.lp(sweep, 2500)) * u ** 2


# the cadence (16 16ths per bar). Snare: A accent, t tap, g ghost, f flam, r diddle roll, . rest
SN = {'a': 'A.tgA.ttA.tgArrt', 'b': 'f.t.A.tgf.tgA.rr', 'fill': 'A.A.AttAttArrrrr'}
SN_LIGHT = {'a': 'A.g.A.g.A.g.A.g.', 'b': 'A.g.A.g.A.g.Agrg', 'fill': 'A.g.A.g.A.gtArrr'}
TEN = {'lo': '..1...2...1...3.', 'hi': '..1.2.3.1.2.3434'}     # quads: drum numbers (1 high .. 4 low)
BASS = {'lo': 'U.......U.......', 'hi': 'U...1.2.U..34.5.'}     # bass drums: U unison, 1..5 splits (high .. low)


def cadence(mix, b, dense=0.5, light=False):
    """one bar of the drumline + pit (bar index b). dense 0..1 thickens the parts (never the level)"""
    t0 = b * T.BAR
    ch = T.CHORDS[b % T.N_BARS]
    phrase = (b - 2) % 4 if b >= 2 else b
    pk = 'fill' if phrase == 3 else ('a' if phrase % 2 == 0 else 'b')
    sn = (SN_LIGHT if (light or dense < 0.3) else SN)[pk]
    for i, c in enumerate(sn):
        ts = t0 + i * S16
        if c == '.':
            continue
        if c == 'r':                                                   # diddle: two 32nds
            for j in range(2):
                mix.place('drums', msnare('t'), ts + j * S16 / 2, 0.30 * (1.0 if j == 0 else 0.85), pan=0.05)
        elif c == 'f':                                                 # flam: grace 18 ms early
            mix.place('drums', msnare('g'), ts - 0.018, 0.30, pan=0.05)
            mix.place('drums', msnare('A'), ts, 0.30, pan=0.05)
        else:
            mix.place('drums', msnare(c), ts, 0.30, pan=0.05)
    if not light:
        ten = TEN['hi' if dense > 0.55 else 'lo']
        for i, c in enumerate(ten):
            if c != '.':
                mix.place('drums', tenor(int(c) - 1), t0 + i * S16, 0.10, pan=-0.25 + 0.15 * int(c))
    bd = BD_ROOTS[ch]
    bp = BASS['hi' if dense > 0.4 and not light else 'lo']
    for i, c in enumerate(bp):
        ts = t0 + i * S16
        if c == 'U':
            for m in bd:
                mix.place('drums', bassdrum(m), ts, 0.10)
        elif c != '.':
            mix.place('drums', bassdrum(bd[int(c) - 1]), ts, 0.22, pan=-0.3 + 0.15 * int(c))
    if phrase == 0:
        mix.place('drums', A.crash(1.4), t0, 0.07 if not light else 0.05)
    if dense > 0.65 and not light:                                     # cymbal crunches on 2 and 4
        for bt in (1, 3):
            mix.place('drums', choke(b * 4 + bt), t0 + bt * T.BEAT, 0.07)
    # the pit: a high marimba ostinato (8ths) + timpani roots
    tones = T.chord_tones(72, 91, ch)
    for e8 in range(8):
        m = tones[(e8 * 2 + (e8 // 4)) % len(tones)]
        mix.place('pit', marimba(m), t0 + e8 * T.BEAT / 2, 0.06 if e8 % 2 == 0 else 0.045, pan=0.3)
    root = T.CHORD_ROOT[ch]
    mix.place('pit', timp(root), t0, 0.30)
    if dense > 0.5 and not light:
        mix.place('pit', timp(root + 7 if root + 7 <= 50 else root - 5), t0 + 2 * T.BEAT, 0.22)


def unison(mix, t, ch, g=1.0):
    """a drumline unison hit: all bass drums + accent snare + crash + timpani + a low thud"""
    for m in BD_ROOTS[ch]:
        mix.place('drums', bassdrum(m), t, 0.14 * g)
    mix.place('drums', msnare('A'), t, 0.34 * g)
    for i in range(4):
        mix.place('drums', tenor(i), t, 0.08 * g)
    mix.place('drums', A.crash(1.4), t, 0.08 * g)
    mix.place('pit', timp(T.CHORD_ROOT[ch]), t, 0.5 * g)
    mix.place('fx', A.thud(0.7, 110, 34, seed=int(t * 50) + 3), t, 0.30 * g)


def roll(mix, t0, t1, g0=0.12, g1=0.32):
    """a snare diddle roll on 32nds from t0 to t1, crescendo"""
    st = 0.0
    n = max(1, int((t1 - t0) / (S16 / 2)))
    for j in range(n):
        u = j / max(1, n - 1)
        mix.place('drums', msnare('t'), t0 + j * S16 / 2, g0 + (g1 - g0) * u, pan=0.05)


# ---------------------------------------------------------------- the fighters from the run
def twin_on(k, t):
    return any(kk == k and t0 <= t < until for t0, kk, until in RD.TWINS)


def bass_clarinet(mix):
    """the rumble: the peg's root, 8th notes through every whirl, slap pops on the smashes, abilities"""
    dropped = False
    first = min(a[0] for a in RD.ANCH if a[1] == KB)                  # FIGHT!: it rolls out of its corner, growling
    sig, g, _ = colour(bcl(43, first - T.T_FIGHT - 0.05, 0.75, growl=0.2), KB, T.T_FIGHT)
    mix.place('bcl', sig, T.T_FIGHT, 0.30, pan=-0.2)
    for t0, k, c, orbit, te in RD.ANCH:
        if t0 > T_KO + 0.01:
            break
        tq = q16(t0)
        root = T.bass_root(tq)
        if k == KB:                                                   # the peg goes in: the root, growling
            sig, g, lead = colour(bcl(root, 0.9 * T.BEAT, 0.95, growl=0.35), KB, t0)
            mix.place('bcl', sig, tq, 0.48 * g, pan=-0.2)
            mix.place('fx', clank(84, int(t0 * 10)), tq, 0.05, pan=-0.2)
            tr = q16(te)                                              # it pulls the peg and rolls on: a held growl
            nxt = min([a[0] for a in RD.ANCH if a[1] == KB and a[0] > t0 + 0.1] + [T_KO])
            if nxt - tr > 0.3:
                sig, g, _ = colour(bcl(T.bass_root(tr) + 7 if T.bass_root(tr) < 41 else T.bass_root(tr), min(nxt - tr, 2.2 * T.BEAT) - 0.05, 0.7,
                                       growl=0.1), KB, tr)
                mix.place('bcl', sig, tr, 0.30 * g, pan=-0.2)
        else:                                                         # a copied whirl (IDENTITY CRISIS): the euphonium
            mix.place('euph', colour(eup(root + 12, 0.9 * T.BEAT, 0.9), KE, t0)[0], tq, 0.36, pan=0.2)
        e8 = q16(t0 + T.BEAT / 2)
        while e8 < te - 0.02 and e8 < T_KO:                           # the rumble through the whirl, on the 8ths
            slot = round(e8 / S16)
            if k == KB:
                hp = hp_at(KB, e8) / 100.0
                dropped = thin(slot, 3, hp, dropped)
                if not dropped:
                    m = T.bass_at(e8)
                    sig, g, lead = colour(bcl(m, 0.42 * T.BEAT, 0.8, growl=0.15), KB, e8)
                    mix.place('bcl', sig, e8, 0.44 * g, pan=-0.2)
                    if twin_on(KB, e8):                               # MULTIPHONICS: the twelfth above at once
                        mix.place('bcl', colour(bcl(m + 19, 0.42 * T.BEAT, 0.6), KB, e8)[0], e8, 0.18 * g, pan=-0.1)
                    if lead > 0.04 and slot % 4 == 0:                 # the leader: an octave under (fuller)
                        mix.place('bcl', colour(bcl(max(34, m - 12), 0.42 * T.BEAT, 0.6), KB, e8)[0], e8, 0.14 * g)
            e8 += T.BEAT / 2
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'hit' and e[2] == KB and e[7] == 'anchor':        # a smash: a slap-tongue pop on the rumble's pitch
            tq = q16(t)
            m = T.bass_at(tq)
            mix.place('bcl', colour(slap(m + 12, 1.0), KB, t)[0], tq, 0.42, pan=-0.15)
            mix.place('fx', A.thud(0.3, 150, 55, seed=int(t * 9)), tq, 0.18)
        elif kind == 'snip' and e[2] == KB:                           # SLAP TONGUE: the big slap
            tq = q16(t)
            mix.place('bcl', slap(T.bass_root(tq) + 12, 1.0), tq, 0.55 if e[3] else 0.32, pan=-0.15)
            if e[3]:
                mix.place('fx', A.thud(0.35, 180, 60, seed=int(t * 7)), tq, 0.30)
        elif kind == 'snip_open' and e[2] == KB:                      # the tongue sticks to the reed: a tiny click
            mix.place('bcl', slap(84, 0.3), q16(t), 0.10)
        elif kind == 'ability' and e[2] == KB and e[3] == 'MULTIPHONICS':
            tq = q16(t)                                               # a split, buzzing chord: root + twelfth + 11th
            root = T.bass_root(tq)
            for m, g_ in ((root, 0.36), (root + 19, 0.22), (root + 17.5, 0.12)):
                mix.place('bcl', bcl(m, 1.5 * T.BEAT, 0.9, growl=0.2), tq, g_, pan=-0.2)
        elif kind == 'lights_out' and e[2] == KB:                     # LIGHTS OUT: a low growl swelling in the dark
            tq = q16(t)
            dur = e[3] - t
            x = bcl(34, dur, 1.0, growl=0.6)
            mix.place('bcl', x * np.linspace(0.5, 1.2, len(x)), tq, 0.40, pan=-0.2)
            mix.place('fx', A.whoosh(0.8, 200, 4000, rise=False), tq, 0.18)
        elif kind == 'erupt' and e[2] == KB:                          # ... and erupts
            tq = q16(t)
            mix.place('bcl', slap(T.bass_root(tq) + 12, 1.0), tq, 0.65, pan=-0.1)
            mix.place('fx', thunder(1.6, int(t * 10)), tq, 0.45)
            mix.place('drums', A.crash(1.6), tq, 0.10)


def euphonium(mix):
    """the melody: a note per salvo, a fat chord tone on a landed salvo, the barrage's run, the identity crisis"""
    dropped = False
    lands = {}
    for t, k, x, z, hit, salvo, j, kind in RD.LANDS:
        if t > T_KO + 0.01:
            break
        q = lands.setdefault((round(q16(t) / S16), k, kind), dict(t=t, n=0, hits=0, kind=kind, salvo=salvo))
        q['n'] += 1
        q['hits'] += int(hit)
    for t0, k, n, salvo in RD.SALVOS:                                 # a salvo: the melody note of its beat
        if t0 > T_KO + 0.01:
            break
        tq = q16(t0)
        slot = round(tq / S16)
        if k == KE:
            hp = hp_at(KE, t0) / 100.0
            haste = any(e[0] <= t0 < e[0] + 8 * T.BEAT for e in EV if e[1] == 'ability' and e[3] == 'MARCHING ORDERS')
            dropped = thin(slot, 11, hp, dropped)
            m = T.melody_at(tq)
            if dropped and haste:                                     # thinned: a note of the every-beat run drops out
                continue
            dur = (0.85 if haste else 1.7) * T.BEAT
            if dropped:                                               # (on the 2-beat grid it is clipped, not dropped:
                dur = 0.5 * T.BEAT                                    # no hole while it is still firing)
            sig, g, lead = colour(eup(m, dur, 0.75 + 0.04 * min(n, 5)), KE, t0)
            mix.place('euph', sig, tq, (0.26 if dropped else 0.40) * g, pan=0.2)
            if twin_on(KE, t0):                                       # a copied MULTIPHONICS: two notes at once
                mix.place('euph', colour(eup(m + 7, dur, 0.6), KE, t0)[0], tq, 0.20 * g, pan=0.25)
            if lead > 0.04:                                           # the leader: a third under (fuller)
                under = T.chord_tones(m - 5, m - 3, T.chord_at(tq))
                if under:
                    mix.place('euph', colour(eup(under[-1], dur, 0.55), KE, t0)[0], tq, 0.15 * g, pan=0.25)
            mix.place('fx', A.thud(0.18, 220, 90, seed=int(t0 * 11)), tq, 0.06 + 0.01 * min(n, 6))   # the mortar's pop
        else:                                                         # (the bass clarinet never fires shells)
            pass
    for (slot, k, kind), q in sorted(lands.items()):
        t = q['t']
        tq = slot * S16
        g_n = min(1.0, 0.5 + 0.12 * q['n'])
        mix.place('fx', boom(int(t * 13) % 1000, 0.45), tq, (0.16 if q['hits'] else 0.09) * g_n, pan=0.1)
        if q['hits'] and kind == 'mortar':                            # landed: a fat chord tone a sixth under
            m = T.melody_at(tq)
            tones = T.chord_tones(m - 10, m - 7, T.chord_at(tq))
            if tones:
                sig, g, _ = colour(eup(tones[-1], 0.35, 0.95), KE, t)
                mix.place('euph', sig, tq, 0.30 * g, pan=0.25)
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'barrage':                                         # NOT A TUBA: a falling run, one note per landing
            tq = q16(t)
            mix.place('euph', eup(72, 1.0 * T.BEAT, 1.0), tq, 0.46, pan=0.2)
            mix.place('euph', eup(60, 1.0 * T.BEAT, 0.9), tq, 0.30, pan=0.2)
            run = [l for l in RD.LANDS if l[5] == e[4]]
            seq = (72, 70, 68, 67, 65, 63, 62, 60)
            seen = []
            for l in run:
                tl = q16(l[0])
                if tl in seen:
                    continue
                seen.append(tl)
                m = seq[min(len(seen) - 1, len(seq) - 1)]
                mix.place('euph', colour(eup(m, 0.4 * T.BEAT, 0.95), KE, l[0])[0], tl, 0.34, pan=0.2)
            mix.place('fx', thunder(1.4, 44, crack=False), tq, 0.30)
        elif kind == 'copy' and e[2] == KE:                           # IDENTITY CRISIS: "wait... who am I?"
            tq = q16(t)
            x = eup(62, 0.8 * T.BEAT, 0.9)
            mix.place('euph', bend(x, 5.0, 0.08, 0.8 * T.BEAT), tq, 0.40, pan=0.2)
            mix.place('bcl', bcl(43, 0.3, 0.6, growl=0.3), tq + T.BEAT * 0.5, 0.14, pan=0.25)   # a ghost of the enemy
        elif kind == 'snip' and e[2] == KE:                           # a copied SLAP TONGUE: a brass slap
            tq = q16(t)
            mix.place('euph', eup(T.melody_at(tq) - 12, 0.12, 1.0), tq, 0.46, pan=0.2)
            mix.place('fx', slap(72, 0.8) * 0.6, tq, 0.25)
            if e[3]:
                mix.place('fx', A.thud(0.35, 180, 60, seed=int(t * 7)), tq, 0.26)
        elif kind == 'ability' and e[2] == KE and e[3] == 'MARCHING ORDERS':    # a fanfare triplet + a roll
            tq = q16(t)
            for j, m in enumerate((67, 67, 72)):
                mix.place('euph', eup(m, 0.13 if j < 2 else 0.5, 1.0), tq + j * T.BEAT / 3, 0.40, pan=0.2)
            roll(mix, tq, tq + T.BEAT, 0.10, 0.26)
        elif kind == 'lights_out' and e[2] == KE:                     # a copied LIGHTS OUT
            mix.place('euph', eup(46, e[3] - t, 0.9), q16(t), 0.34, pan=0.2)
        elif kind == 'erupt' and e[2] == KE:
            mix.place('euph', eup(58, 0.4, 1.0), q16(t), 0.5, pan=0.2)
            mix.place('fx', thunder(1.4, int(t * 10)), q16(t), 0.4)


def main():
    mix = A.Mix(T.DUR + 4.0)
    bar_ko = int(T_KO // T.BAR)
    # ------------------------------------------------ bar 0: the VS title card over the drumline at full level
    t1, t2, t3 = 0.004, T.BEAT, 2 * T.BEAT                 # BASS CLARINET 4 ms in (no click at the seam), VS, EUPHONIUM
    cadence(mix, 0, dense=0.5)
    unison(mix, t1, 'Cm', 0.75)
    unison(mix, t2, 'Cm', 0.7)
    unison(mix, t3, 'Cm', 0.72)
    mix.place('bcl', bcl(36, 0.9 * T.BEAT, 1.0, growl=0.7), t1, 0.50, pan=-0.2)    # BASS CLARINET: a growl + slap
    mix.place('bcl', bcl(43, 0.9 * T.BEAT, 0.8, growl=0.4), t1, 0.25, pan=-0.2)
    mix.place('bcl', slap(48, 1.0), t1 + 0.004, 0.50, pan=-0.2)
    mix.place('fx', thunder(1.4, 91), t2, 0.55)                   # VS: the thunder crack where it splits the divider
    for m, g_ in ((60, 0.50), (67, 0.36), (48, 0.30)):            # EUPHONIUM: a blast, a fat fifth
        mix.place('euph', eup(m, 1.5 * T.BEAT, 1.0), t3, g_, pan=0.2)
    roll(mix, 3 * T.BEAT, T.BAR - 0.02, 0.10, 0.30)               # beat 4: a snare roll as the doors open
    mix.place('fx', A.whoosh(T.T_DOORS[1] - T.T_DOORS[0] + 0.1, 300, 5000), T.T_DOORS[0] - 0.05, 0.14)
    # ------------------------------------------------ bar 1: the snare line's count-in on the dominant (G)
    cadence(mix, 1, dense=0.3, light=True)
    for j, (tc, txt) in enumerate(T.COUNT[:3]):
        unison(mix, tc, 'G', 0.75)
    roll(mix, T.BAR + 3 * T.BEAT, T.T_FIGHT - 0.02, 0.12, 0.34)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), T.BAR + 3 * T.BEAT, 0.12)
    unison(mix, T.T_FIGHT, 'Cm', 1.1)                             # the drop
    mix.place('fx', thunder(1.8, 7), T.T_FIGHT, 0.36)
    # ------------------------------------------------ the cadence: bar 2 (FIGHT!) .. the K.O. (thickens as it goes)
    for b in range(2, bar_ko + 1):
        cadence(mix, b, dense=min(1.0, 0.45 + (b - 2) / 24.0))
    for t_, who, name in RD.STORY['caps']:                       # a unison hit under every captioned ability
        unison(mix, q16(t_), T.chord_at(t_), 0.6)
    for k in ('drums', 'pit'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ the fighters, from the run
    bass_clarinet(mix)
    euphonium(mix)
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.05 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            mix.place('fx', A.thud(0.15, 200, 90, seed=int(t * 3)), t, 0.06)
    # ------------------------------------------------ the K.O. moment
    unison(mix, T_KO, T.chord_at(T_KO), 1.0)
    mix.place('fx', thunder(2.4, 51), T_KO, 0.50)
    loser = NAMES[LOSE]
    if loser == 'EUPHONIUM':                                      # the euphonium falls apart: a falling blast, valves
        x = eup(58, 1.0, 1.0)
        mix.place('euph', bend(x, -9.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x)), RD.T_HS, 0.42)
    else:
        x = bcl(43, 1.0, 1.0, growl=0.5)
        mix.place('bcl', bend(x, -7.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x)), RD.T_HS, 0.42)
    for j, m in enumerate((86, 81, 91, 77, 84)):
        mix.place('fx', clank(m, 20 + j), RD.T_HS + 0.08 * j, 0.07, pan=(j - 2) / 5)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.22)
    tw = q16(T_KO + 1.3)                                          # the winner's stinger on "(WINNER) WINS"
    unison(mix, tw, T.chord_at(tw), 0.9)
    if NAMES[WIN] == 'BASS CLARINET':
        for j, m in enumerate((41, 48, 53, 56)):
            mix.place('solo', bcl(m, 0.9 - 0.06 * j, 1.0, growl=0.2), tw + j * 0.03, 0.30)
        mix.place('solo', slap(53, 1.0), tw, 0.3)
    else:
        mix.place('solo', eup(65, 0.9, 1.0), tw, 0.45)
        mix.place('solo', eup(72, 0.9, 0.9), tw, 0.30)
    solo0 = RD.T_SOLO
    for b in range(bar_ko + 1, int(round(solo0 / T.BAR))):        # the drumline holds its breath, never quiet
        cadence(mix, b, dense=0.6)
    # ------------------------------------------------ victory: the cadence under the winner's solo (no tenor sweeps)
    b_solo = int(round(solo0 / T.BAR))
    for b in range(b_solo, 30):
        cadence(mix, b, dense=0.85)
    for bt, m, d in T.SOLO[NAMES[WIN]]:
        ts = solo0 + bt * T.BEAT
        if ts > T.T_REASSEMBLE - 0.05:
            break
        if NAMES[WIN] == 'BASS CLARINET':
            mix.place('solo', bcl(m, d * T.BEAT * 0.95, 0.95, growl=0.15), ts, 0.62)
            if d <= 0.26:
                mix.place('solo', slap(m + 12, 0.8), ts, 0.25)
        else:
            mix.place('solo', eup(m, d * T.BEAT * 0.95, 0.95), ts, 0.52)
    # the loser reassembles: its pieces fly home as a rising run, a glass "pop" when it's whole
    run = [55, 58, 60, 63, 67, 72]
    for j, m in enumerate(run):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
        if loser == 'EUPHONIUM':
            mix.place('euph', eup(m, 0.14, 0.7), tj, 0.22)
        else:
            mix.place('bcl', bcl(m - 12, 0.14, 0.7), tj, 0.24)
    mix.place('fx', A.chime(95, 0.5) + A.chime(102, 0.5) * 0.5, T.T_GLIDE, 0.12)
    # ------------------------------------------------ bars 30-31: the drumline returns, building into the title slam
    for b in (30, 31):
        cadence(mix, b, dense=1.0)
    t30 = 30 * T.BAR                                              # both fighters back in their corners, revving: the
    e8 = t30 + T.BEAT * 0.5                                       # rumble + the melody build into the frame-0 slam
    while e8 < T.DUR - T.BEAT * 0.4:
        mix.place('bcl', bcl(T.bass_at(e8), 0.42 * T.BEAT, 0.8, growl=0.15), q16(e8), 0.40, pan=-0.2)
        e8 += T.BEAT / 2
    for j in range(1, 8):
        tb = t30 + j * T.BEAT
        mix.place('euph', eup(T.melody_at(tb), 0.9 * T.BEAT, 0.85), tb, 0.36, pan=0.2)
    t31 = 31 * T.BAR
    for bt in range(4):
        for m in BD_ROOTS['G']:
            mix.place('drums', bassdrum(m), t31 + bt * T.BEAT, 0.09)
    roll(mix, t31 + 2 * T.BEAT, T.DUR - 1e-3, 0.16, 0.48)          # a snare roll into the seam
    mix.place('fx', riser(T.DUR - t31), t31, 0.30)
    nc = int((T.T_CLOSE + 0.012) * SR)                            # the doors sliding shut into the title slam
    tcl = A.tt(nc)
    doors = A.bp(np.random.default_rng(93).normal(0, 1, nc), 400, 5000) * (tcl / T.T_CLOSE) ** 2
    mix.place('fx', doors, T.DUR - T.T_CLOSE, 0.60)
    for tl, k, nch in impact_landings(RD.CAPTIONS):               # caption landings: soft thuds
        mix.place('fx', A.thud(), tl, 0.08 if k == 0 else 0.06)
    out, stems = master(mix)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    for k, s in stems.items():
        A.write(os.path.join(HERE, 'check', f'stem_{k}.wav'), s)
    spans = [(0, T.BAR), (T.BAR, T.T_FIGHT), (T.T_FIGHT, 10), (10, 20), (20, 32), (32, 44), (44, T_KO), (T_KO, solo0),
             (solo0, 57.2), (57.2, 61)]
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


BUS_GAIN = {'bcl': 2.0, 'euph': 1.3, 'solo': 1.5, 'fx': 1.0, 'drums': 1.0, 'pit': 0.9}


def master(mix, thr=0.20, ceiling=0.89):
    """ONE master gain (no section automation: consistent loudness from frame 0) + a compressor and a soft limiter;
    the overflow past the loop (tails, reverb) folds onto the start; NA samples (whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('bcl', 0.25), ('euph', 0.35), ('solo', 0.4), ('fx', 0.2),
                                                ('drums', 0.18), ('pit', 0.3)))
    wet = A.reverb(send, dur=2.0, decay=0.5) * 0.22
    full = sum(buses.values()) + wet

    def fold(x):
        y = x[:NS].copy()
        tail = x[NS:]
        y[:len(tail)] += tail[:NS]
        return y
    m = fold(full)
    from scipy.signal import lfilter
    env = np.sqrt(lfilter([0.002], [1, -0.998], (np.concatenate([m[-SR:], m]) ** 2).mean(1)))[SR:]
    gain = np.where(env > thr, (thr / np.maximum(env, 1e-9)) ** 0.5, 1.0)[:, None]
    m = m * gain
    s1 = ceiling / (np.percentile(np.abs(m), 99.97) + 1e-9)
    m = np.tanh(m * s1 * 1.1) / np.tanh(1.1)
    s2 = ceiling / (np.abs(m).max() + 1e-9)
    m *= s2
    stems = {}
    for k in buses:
        stems[k] = fold(buses[k]) * gain * s1 * s2
    return m, stems


if __name__ == '__main__':
    main()
