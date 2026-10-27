"""The BALLFIGHT soundtrack for any episode config, all synthesised: the mood's bed (kit/moods/<MOOD>.py, tempo and
chords in) under the two fighters' parts (each fighter's Sound in kit/fighters/<name>.py, played from the honest run),
in the order every episode since #36 builds it:

    bar 0  the VS title card: the bed's hit under each word, A's title blast, VS (the bed's crack), B's blast, the doors
    bar 1  the count-in (the bed + each fighter's count notes), FIGHT!
    fight  the bed's bars to the K.O. (it tape-stops there), each fighter's part, the shared event sounds (ropes, clashes)
    K.O.   the bed's hit, the loser falls apart, the crowd; the winner's stinger, the bed until the solo, the solo over
           the bed's victory bars, the loser's pieces fly home, the glass pop
    end    the bed's build (+ each fighter's role in it) into the title slam on sample 0, the doors shutting
Consistent loudness from frame 0 (project rule, Oct 26): ONE master gain + a compressor + a soft limiter, no section
automation; the build comes from the arrangement. Every hit snaps to the nearest 16th; tails past the loop fold onto the
start; the length is whole AAC frames, so the file loops sample-perfectly.

    from ballfight.kit import music; music.main()      -> <episode>/audio.wav + check/stem_<bus>.wav"""
import importlib
import math
import os
import sys

import numpy as np

from .episode import current
from . import picture as P
from . import fighters as FIGHTER_MODULES
from reelkit import audio as A
from reelkit.ui import impact_landings
import pp_bridge as B

EP = current()
T = EP.T
SR = A.SR
RUN, EV = P.RUN, P.EV
NAMES, WIN, LOSE, T_KO = P.NAMES, P.WIN, P.LOSE, P.T_KO
CFG = P.CFG
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames (2860 = 61.013 s): loops sample-perfectly
BUS = {'solo': (1.5, 0.4), 'fx': (1.2, 0.25)}           # (gain, reverb send) of the shared buses


def q16(t):
    return round(t / T.S16) * T.S16


def hp_at(k, t):
    return P.hp_at(k, t)


# ---------------------------------------------------------------- thinning (a hurt fighter's line loses notes)
def h01(*xs):
    """deterministic splitmix hash in [0, 1) (#37's fix: the old linear hash fell below the threshold on runs of
    consecutive slots and silenced whole phrases)"""
    v = 0x9E3779B97F4A7C15
    for x in xs:
        v = (v ^ (int(round(x * 1000)) & 0xFFFFFFFFFFFFFFFF)) & 0xFFFFFFFFFFFFFFFF
        v = (v + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
        z = v
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFFFFFFFFFFFFFF
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
        v = z ^ (z >> 31)
    return (v >> 11) / float(1 << 53)


class Thinner:
    """drops notes as a fighter's HP falls: p = min(0.30, 0.55 (1 - hp)^1.6), never two drops in a row (#41)"""

    def __init__(self, seed):
        self.seed, self.last = seed, False

    def keep(self, slot, hp01):
        p = min(0.30, 0.55 * (1 - hp01) ** 1.6)
        drop = (not self.last) and h01(slot, self.seed) < p
        self.last = drop
        return not drop


M64 = (1 << 64) - 1


def h01_37(*xs):
    """#37's splitmix64 mixer (the reed fighters' thinning, as #37 built it)"""
    z = 0x9E3779B97F4A7C15
    for x in xs:
        z = (z ^ (int(round(x * 1000)) & M64)) * 0xBF58476D1CE4E5B9 & M64
        z = (z + 0x9E3779B97F4A7C15) & M64
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & M64
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & M64
        z ^= z >> 31
    return z / float(1 << 64)


def thin_37(slot, key, hp, prev_dropped):
    """drop this note? (#37: up to 30 % of a hurt fighter's notes, never two in a row)"""
    return not prev_dropped and h01_37(slot, key) < 0.30 * (1 - hp) ** 1.6


# ---------------------------------------------------------------- the Pixel Pitch voices (cached, level-matched)
_VC, _GAIN = {}, {}


def fit(x, cap, fade=0.04):
    n = int(cap * SR)
    x = np.asarray(x, float)[:n].copy()
    if len(x) < n:
        x = np.pad(x, (0, n - len(x)))
    fd = min(len(x) // 3, int(fade * SR))
    x[-fd:] *= np.linspace(1, 0, fd)
    return x


def inst_gain(name):
    if name not in _GAIN:
        lo, hi = B.entrant(name)[3]
        x = B.voice(name, (lo + hi) // 2, 0.8, 0.2)[:int(0.3 * SR)]
        _GAIN[name] = min(4.0, 0.16 / (np.sqrt((A.hp(x, 120) ** 2).mean()) + 1e-6))
    return _GAIN[name]


def note(name, midi, dur=0.18, cap=0.6, vel=0.85):
    """one note in a Pixel Pitch voice (pp_bridge roster name), level-matched across instruments"""
    key = (name, int(midi), round(dur, 3), cap, round(vel, 2))
    if key not in _VC:
        _VC[key] = fit(B.voice(name, int(midi), vel, dur), cap) * inst_gain(name)
    return _VC[key]


# ---------------------------------------------------------------- shaping
def gliss(x, semis, dur):
    """slide into the note: start `semis` below (negative: above) and glide to pitch over `dur` s (a time warp)"""
    n = len(x)
    t = A.tt(n)
    off = -semis * (1 - np.clip(t / max(dur, 1e-3), 0, 1)) ** 1.5
    rate = 2 ** (off / 12)
    pos = np.cumsum(rate) - rate[0]
    return np.interp(pos, np.arange(n), x, right=0.0)


def bend(x, semis, t0, t1):
    """bend the pitch by `semis` between t0 and t1 s into the note (a sag, a falling K.O.)"""
    n = len(x)
    t = A.tt(n)
    off = semis * np.clip((t - t0) / max(1e-3, t1 - t0), 0, 1)
    rate = 2 ** (off / 12)
    pos = np.cumsum(rate) - rate[0]
    return np.interp(pos, np.arange(n), x, right=0.0)


def vibrato(x, depth=0.35, rate=5.5, delay=0.15):
    n = len(x)
    t = A.tt(n)
    off = depth * np.sin(2 * np.pi * rate * t) * np.clip((t - delay) / 0.2, 0, 1)
    r = 2 ** (off / 12)
    pos = np.cumsum(r) - r[0]
    return np.interp(pos, np.arange(n), x, right=0.0)


def wah(x, opens):
    """plunger mute: a band-pass whose centre opens (500 -> 1800 Hz) over each (t0, t1) span: the "wah" (#34)"""
    n = len(x)
    out = np.zeros(n)
    nb = 48
    t = A.tt(n)
    fc = np.full(n, 450.0)
    for t0, t1 in opens:
        u = np.clip((t - t0) / max(1e-3, t1 - t0), 0, 1)
        fc = np.maximum(fc, np.where((t >= t0) & (t < t1 + 0.25), 450 + 1400 * np.sin(np.pi * 0.5 * u) ** 2, 450))
    for k in range(nb):
        a, b = int(k * n / nb), int((k + 1) * n / nb)
        c = float(fc[(a + b) // 2])
        out[a:b] = A.bp(x[max(0, a - 4096):b], c * 0.55, min(c * 1.8, SR / 2 - 200))[-(b - a):]
    return out * 1.6


def colour(x, k, t, drive_k=1.0, cut0=90, cut_span=520, drive=4.5, low=False, low_cut=(30, 70)):
    """the fighter's tone at time t: thinner (high-passed) and dirtier (driven) as its HP drops; returns (signal, gain,
    lead): the leader plays up to +3.5 dB, the loser down to -4 dB. A bass line (`low`) keeps its fundamental"""
    hp = hp_at(k, t) / 100.0
    lead = (hp_at(k, t) - hp_at(1 - k, t)) / 100.0
    cut = (low_cut[0] + low_cut[1] * (1 - hp) ** 1.6) if low else (cut0 + cut_span * (1 - hp) ** 1.6)
    y = A.hp(x, cut)
    dr = 1 + drive * drive_k * (1 - hp) ** 2
    y = np.tanh(dr * y * 1.2) / np.tanh(dr * 1.2) * (1.0 / (1 + 0.15 * (dr - 1)))
    g = 10 ** (float(np.clip(6.0 * lead, -4.0, 3.5)) / 20)
    return y, g, lead


# ---------------------------------------------------------------- shared sound effects
def clank(m, seed=0, dur=0.4, decays=((1.0, 0.12), (2.76, 0.06), (5.4, 0.03))):
    """metal on canvas: a short metallic ring"""
    n = int(dur * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = sum(np.sin(2 * np.pi * f * r * t) * np.exp(-t / d) for r, d in decays)
    return x + 0.3 * A.bp(np.random.default_rng(seed).normal(0, 1, n), 2000, 8000) * np.exp(-t / 0.005)


def hiss(dur=0.09, seed=0, lo=3500, hi=10000, dec=0.03):
    n = int(dur * SR)
    tt_ = A.tt(n)
    return A.bp(np.random.default_rng(seed).normal(0, 1, n), lo, hi) * np.exp(-tt_ / dec)


def roar(dur, seed=3):
    n = int(dur * SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    x = A.bp(x, 300, 2600) * 0.5 + A.bp(x, 900, 5000) * 0.25
    t = A.tt(n)
    env = np.clip(t / 0.25, 0, 1) * np.exp(-np.maximum(t - 0.6, 0) / (dur * 0.45))
    return x * env


def thunder(dur=1.6, seed=0, crack=True):
    """a rolling thunder rumble (+ a crack on top) (#37)"""
    n = int(dur * SR)
    t = A.tt(n)
    r = np.random.default_rng(seed)
    rumble = A.lp(r.normal(0, 1, n), 140) * 3.0 * np.clip(t / 0.03, 0, 1) * np.exp(-t / (dur * 0.35))
    rumble *= 0.7 + 0.3 * np.sin(2 * np.pi * 3.1 * t + seed)
    x = rumble
    if crack:
        x = x + A.bp(r.normal(0, 1, n), 1500, 9000) * np.exp(-t / 0.03) * 0.9
    return x


def riser(dur, over=0.012, seed=11):
    """a noise + saw riser that builds right up to (and `over` s past) its end: no fade, so the loop seam stays loud"""
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


# ---------------------------------------------------------------- the mood and the fighters
M = sys.modules[__name__]
bed = importlib.import_module(f'{__package__}.moods.{T.MOOD}').Bed(M)
SOUNDS = [FIGHTER_MODULES.sound(NAMES[k], M, k) for k in (0, 1)]


def shared_events(mix):
    """the ring's own sounds from the run: ropes, ball clashes (the mood's), shield blocks, parries"""
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.05 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            bed.clash(mix, t)
        elif kind == 'block':
            mix.place('fx', clank(84, int(t * 5)), q16(t), 0.06)
        elif kind == 'parry':
            mix.place('fx', clank(88, int(t * 9)), t, 0.04)


def master(mix, ceiling=0.89):
    """ONE master gain (no section automation: consistent loudness from frame 0) + a compressor and a soft limiter;
    the overflow past the loop (tails, reverb) folds onto the start; NA samples (whole AAC frames)"""
    levels = {}
    for s in SOUNDS:
        levels[s.bus] = (s.bus_gain, s.send)
        levels.update(s.buses)                             # a fighter's extra buses (#42's crowd laugh)
    levels.update(BUS)
    levels.update(bed.buses)
    levels.update({k: tuple(v) for k, v in T.MIX.items()})
    buses = {k: v * levels.get(k, (1.0, 0.0))[0] for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * levels[k][1] for k in levels)
    rdur, rdec, rwet = bed.reverb
    wet = A.reverb(send, dur=rdur, decay=rdec) * rwet
    full = sum(buses.values()) + wet

    def fold(x):
        y = x[:NA].copy()
        tail = x[NA:]
        y[:len(tail)] += tail[:NA]
        return y
    m = fold(full)
    from scipy.signal import lfilter
    thr, ex = bed.comp
    env = np.sqrt(lfilter([0.002], [1, -0.998], (np.concatenate([m[-SR:], m]) ** 2).mean(1)))[SR:]
    gain = np.where(env > thr, (thr / np.maximum(env, 1e-9)) ** ex, 1.0)[:, None]
    m = m * gain
    s1 = ceiling / (np.percentile(np.abs(m), 99.97) + 1e-9)
    m = np.tanh(m * s1 * 1.1) / np.tanh(1.1)
    s2 = ceiling / (np.abs(m).max() + 1e-9)
    m *= s2
    stems = {}
    for k in buses:
        stems[k] = fold(buses[k]) * gain * s1 * s2
    return m, stems


def score():
    """the whole soundtrack as a mix (before the master)"""
    mix = A.Mix(T.DUR + 4.0)
    a, b = SOUNDS
    # ------------------------------------------------ bar 0: the VS title card, one band hit per word
    if T.CARD:                                             # (legacy episodes without the card: FIGHT_BAR = 1)
        t1, t2, t3 = 0.004, T.BEAT, 2 * T.BEAT             # A 4 ms in (no click at the seam), VS, B
        bed.title(mix, (t1, t2, t3))
        a.title_blast(mix, t1)
        bed.vs(mix, t2)
        b.title_blast(mix, t3)
        bed.title_rest(mix)
        mix.place('fx', A.whoosh(T.T_DOORS[1] - T.T_DOORS[0] + 0.1, 300, 5000), T.T_DOORS[0] - 0.05,
                  getattr(bed, 'doors_open_gain', 0.14))
    # ------------------------------------------------ bar 1: the count-in on the second chord, FIGHT!
    bed.count_in(mix)
    for s in SOUNDS:
        s.count_in(mix)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), T.BAR + 3 * T.BEAT if T.CARD else T.T_FIGHT - T.BEAT,
              getattr(bed, 'count_whoosh_gain', 0.12))
    bed.fight_start(mix)
    # ------------------------------------------------ the fight: the bed to the K.O. (tape-stopped there), the fighters
    bar_ko = int(T_KO // T.BAR)
    bed.fight(mix, bar_ko)
    for s in SOUNDS:
        s.play(mix)
    shared_events(mix)
    # ------------------------------------------------ the K.O.
    bed.ko(mix)
    for s in SOUNDS:
        s.ko(mix)
    SOUNDS[LOSE].fall_apart(mix)
    mix.place('fx', roar(5.5), T_KO + 0.2, bed.roar_gain)
    tw = q16(T_KO + 1.3)                                   # the winner's stinger on "(WINNER) WINS"
    SOUNDS[WIN].stinger(mix, tw)
    bed.stinger(mix, tw)
    t_solo = P.T_SOLO
    bed.after_ko(mix, bar_ko, tw, t_solo)
    # ------------------------------------------------ victory: the bed's lighter bars + the winner's solo
    bed.victory(mix, int(round(t_solo / T.BAR)))
    SOUNDS[WIN].solo(mix, t_solo)
    SOUNDS[LOSE].reassemble(mix)                           # the loser's pieces fly home, a glass pop when it's whole
    p_lo, p_hi, p_g = bed.pop
    mix.place('fx', A.chime(p_lo, 0.5) + A.chime(p_hi, 0.5) * 0.5, T.T_GLIDE, p_g)
    # ------------------------------------------------ the build into the title slam on sample 0
    bed.last_bars(mix)
    for s in SOUNDS:
        s.last_bar(mix)
    if T.CARD:
        nc = int((T.T_CLOSE + 0.012) * SR)                 # the doors sliding shut into the title slam
        tcl = A.tt(nc)
        doors = A.bp(np.random.default_rng(93).normal(0, 1, nc), 400, 5000) * (tcl / T.T_CLOSE) ** 2
        mix.place('fx', doors, T.DUR - T.T_CLOSE, bed.doors_gain)
    g0, g1 = bed.landing
    for tl, k, nch in impact_landings(P.CAPTIONS):        # caption landings: soft thuds
        mix.place('fx', A.thud(), tl, g0 if k == 0 else g1)
    return mix


def main(out=None, stems=True):
    out = out or EP.path('audio.wav')
    mix = score()
    m, st = master(mix)
    A.write(out, m)
    if stems:
        for k, s in st.items():
            A.write(os.path.join(EP.check, f'stem_{k}.wav'), s)
    spans = [(0, T.BAR), (T.BAR, T.T_FIGHT), (T.T_FIGHT, 10), (10, 20), (20, 32), (32, 44), (44, T_KO), (T_KO, P.T_SOLO),
             (P.T_SOLO, T.T_REASSEMBLE), (T.T_REASSEMBLE, 61)]
    for a, b, rms, pk in A.section_levels(out, spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')
    return out
