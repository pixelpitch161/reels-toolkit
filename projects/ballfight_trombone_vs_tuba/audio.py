"""#36 BALLFIGHT #10: Trombone vs Tuba - soundtrack (mood C: brass house), all synthesised, original.

The bed is a dark minor house groove with NO bass synth and NO brass of its own: the two fighters are the low end and the
hook, both from the honest run, each in its own Pixel Pitch voice and idiom (as in #34 / #32):
  TUBA      the bass line: every "oom" (the bell inhales) is the bar's root held into the "pah", every pah the fifth as a
            short blast (OOMPAH doubles them: a real oom-pah every 2 beats); a gulp is a low bubble pop + a growl, a spit
            a fifth blast + the cannonball's whoosh, the ropes a thud + crash; the leader gets a sub-octave under its oom.
            BRASS WALL holds a buzzing root under a metallic shimmer; SUBSONIC growls up from the sub before the floor
            ripples on the lowest note.
  TROMBONE  the tenor lead: every slide thrust (on the beat; POWER SLIDE's 8ths add passing notes) plays the hook
            (timeline.TROMBONE_LINE, F3-Ab4) gliding up into the note like a real slide; a landed lance is a punchy accent
            with a big smear up the chord; SAD TROMBONE is the plunger "wah wah wah waaah"; PEDAL TONE holds the pedal
            Bb under the hovering bell, then the stomp.
As HP drops a fighter's part thins and dirties (raspy, notes dropped); the leader's part is fuller and louder.
Bed: F minor at 125.90 BPM (32 bars = 61 s): four-on-the-floor kick (short, so it never masks the tuba), claps on 2 and
4, offbeat hats, a filtered minor pad pumping under the kick, syncopated synth chord stabs (high, short, not brass). The
back half opens up (16th hats, open hats, a ride, brighter pad, more stabs) after a filter-sweep build into bar 17. Bar 0
is the VS title card (a unison house hit per word: TROMBONE's slide blast, VS's thunder crack, TUBA's blast; a clap
pickup as the doors open), bar 1 the count-in on the dominant, the drop on FIGHT!. At the K.O. the groove tape-stops,
the loser falls apart, the crowd roars, the winner's stinger lands and its solo plays over a lighter groove; bar 31
builds (clap roll, riser, the tuba climbing the dominant) into the title slam on sample 0. Every hit is snapped to the
nearest 16th. Tails past the loop fold onto the start.

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
KT, KU = NAMES.index('TROMBONE'), NAMES.index('TUBA')
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames (2860 = 61.013 s): loops sample-perfectly
S16 = T.S16


def q16(t):
    return round(t / S16) * S16


def h01(*xs):
    """deterministic hash in [0, 1) (thinning decisions are part of the score, not random each run)"""
    v = 0
    for x in xs:
        v = (v * 1000003 + int(x * 1000) + 7) % 2147483647
    return ((v * 48271) % 2147483647) / 2147483647.0


# ---------------------------------------------------------------- instrument voices (cached, level-matched)
_VC, _GAIN = {}, {}


def _fit(x, cap):
    n = int(cap * SR)
    x = np.asarray(x, float)[:n].copy()
    if len(x) < n:
        x = np.pad(x, (0, n - len(x)))
    fd = min(len(x) // 3, int(0.04 * SR))
    x[-fd:] *= np.linspace(1, 0, fd)
    return x


def inst_gain(name):
    if name not in _GAIN:
        lo, hi = B.entrant(name)[3]
        x = B.voice(name, (lo + hi) // 2, 0.8, 0.2)[:int(0.3 * SR)]
        _GAIN[name] = min(4.0, 0.16 / (np.sqrt((A.hp(x, 120) ** 2).mean()) + 1e-6))
    return _GAIN[name]


def note(name, midi, dur=0.18, cap=0.6, vel=0.85):
    key = (name, int(midi), round(dur, 3), cap, round(vel, 2))
    if key not in _VC:
        _VC[key] = _fit(B.voice(name, int(midi), vel, dur), cap) * inst_gain(name)
    return _VC[key]


def gliss(x, semis, dur):
    """slide into the note: start `semis` below (negative: above) and glide to pitch over `dur` s (a time warp)"""
    n = len(x)
    t = A.tt(n)
    off = -semis * (1 - np.clip(t / max(dur, 1e-3), 0, 1)) ** 1.5
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


def bone(m, dur=0.16, vel=0.8, gl=2.0, gl_t=0.035, cap=None):
    """a trombone note gliding up `gl` semitones into the pitch (the slide) (#34)"""
    x = note('TROMBONE', m, round(dur, 3), cap or round(dur + 0.3, 2), round(vel, 1))
    return gliss(x, gl, gl_t) if gl else x


def tuba(m, beats=1.0, vel=0.85):
    """a tuba note held for `beats` (its Pixel Pitch voice, stretched with a soft buzzing sustain under it) (#32)"""
    dur = beats * T.BEAT * 0.92
    x = note('TUBA', m, round(min(dur, 0.4), 3), round(dur + 0.2, 2), round(vel, 1)).copy()
    if dur > 0.36:
        n = len(x)
        tt = A.tt(n)
        f = A.mtof(m)
        ph = (f * tt) % 1.0
        sus = A.lp(0.6 * (2 * ph - 1) + 0.4 * np.sin(2 * np.pi * f * tt), 700) * 0.22 * vel
        env = np.clip((tt - 0.12) / 0.15, 0, 1) * np.clip((dur - tt) / 0.06, 0, 1)
        x += sus * env * inst_gain('TUBA') * 0.5
    return x


def hp_at(k, t):
    return RD.hp_at(k, t)


def colour(x, k, t, drive_k=1.0, low=False):
    """the fighter's tone at time t: thinner + dirtier as its HP drops; returns (signal, gain, lead). The tuba (`low`)
    keeps its fundamental: it gets raspy and a little thinner, not high-passed away (#32)"""
    hp = hp_at(k, t) / 100.0
    lead = (hp_at(k, t) - hp_at(1 - k, t)) / 100.0
    cut = (30 + 90 * (1 - hp) ** 1.6) if low else (70 + 420 * (1 - hp) ** 1.6)
    y = A.hp(x, cut)
    drive = 1 + 4.5 * drive_k * (1 - hp) ** 2
    y = np.tanh(drive * y * 1.2) / np.tanh(drive * 1.2) * (1.0 / (1 + 0.15 * (drive - 1)))
    g = 10 ** (float(np.clip(6.0 * lead, -4.0, 3.5)) / 20)
    return y, g, lead


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


def sub_drone(dur, f0=30.0, f1=48.0):
    """SUBSONIC's telegraph: a sub-bass growl climbing toward the slam (#32)"""
    n = int(dur * SR)
    tt = A.tt(n)
    u = tt / dur
    f = f0 + (f1 - f0) * u ** 1.5
    ph = 2 * np.pi * np.cumsum(f) / SR
    x = np.sin(ph) + 0.4 * np.sin(2 * ph) * (0.5 + 0.5 * np.sin(2 * np.pi * 9 * tt))
    return np.tanh(2.0 * x) * np.clip(tt / 0.1, 0, 1) * (0.4 + 0.6 * u)


def clank(m, seed=0):
    """a piece of brass hitting the canvas / the lance banging off brass: a short metallic ring (#34)"""
    n = int(0.4 * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = sum(np.sin(2 * np.pi * f * r * t) * np.exp(-t / d) for r, d in ((1.0, 0.12), (2.76, 0.06), (5.4, 0.03)))
    return x + 0.3 * A.bp(np.random.default_rng(seed).normal(0, 1, n), 2000, 8000) * np.exp(-t / 0.005)


def roar(dur, seed=3):
    n = int(dur * SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    x = A.bp(x, 300, 2600) * 0.5 + A.bp(x, 900, 5000) * 0.25
    t = A.tt(n)
    env = np.clip(t / 0.25, 0, 1) * np.exp(-np.maximum(t - 0.6, 0) / (dur * 0.45))
    return x * env


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


# ---------------------------------------------------------------- the house bed (original): drums, pad, stabs
def kick(vel=1.0):
    """a short, punchy house kick: the body is over in ~0.2 s so the tuba's held notes stay on top"""
    x = A.kick(1.0, 0.16, 160, 52)[:int(0.32 * SR)]
    return A.hp(x, 48) * vel


def hat(vel=1.0, open_=False, seed=4):
    n = int((0.32 if open_ else 0.09) * SR)
    t = A.tt(n)
    x = A.hp(np.random.default_rng(seed).normal(0, 1, n), 7500) * np.exp(-t / (0.12 if open_ else 0.025))
    return x * 0.5 * vel


def ride(vel=1.0, seed=8):
    n = int(0.6 * SR)
    t = A.tt(n)
    x = A.hp(np.random.default_rng(seed).normal(0, 1, n), 5200) * np.exp(-t / 0.25)
    return (x * 0.4 + 0.18 * np.sin(2 * np.pi * 3170 * t) * np.exp(-t / 0.3)) * vel


def shaker(vel=1.0, seed=9):
    n = int(0.07 * SR)
    t = A.tt(n)
    return A.bp(np.random.default_rng(seed).normal(0, 1, n), 4000, 10000) * np.clip(t / 0.01, 0, 1) * np.exp(-t / 0.02) * vel


def stab(ms, dur=0.11, cut=2600):
    """a house chord stab: detuned saws + a square an octave down, a fast filter envelope (synth, not brass)"""
    n = int((dur + 0.12) * SR)
    t = A.tt(n)
    x = sum(A.saw(A.mtof(m), n, 0.008, 2) for m in ms) / len(ms)
    x = x + 0.3 * sum(A.square(A.mtof(m - 12), n, 0.4) for m in ms[:1])
    out = np.zeros(n)
    nb = 10
    for k in range(nb):
        a, b = int(k * n / nb), int((k + 1) * n / nb)
        fc = 400 + cut * math.exp(-((a + b) / 2 / SR) / 0.05)
        out[a:b] = A.lp(x[max(0, a - 2048):b], fc)[-(b - a):]
    return out * A.adsr(n, 0.003, 0.05, 0.4, 0.08) * np.clip((dur + 0.06 - t) / 0.06, 0, 1)


def pump(n, t0=0.0, depth=0.6):
    """the sidechain pump: the pad ducks under every kick (beats from t0) and swells back"""
    t = A.tt(n) + t0
    ph = t % T.BEAT
    return 1 - depth * np.exp(-ph / 0.13)


PAD = {'Fm': (68, 72, 77, 80), 'Db': (68, 72, 77, 80), 'Bbm': (65, 70, 73, 77), 'C': (67, 72, 76, 79)}
STAB = {'Fm': (72, 77, 80), 'Db': (73, 77, 80), 'Bbm': (73, 77, 82), 'C': (72, 76, 79)}
STAB_SLOTS = (3, 6, 10, 14)                    # syncopated 16ths (the "and"s around beats 2 and 4)


def pad_bar(ch, b, dur, cut, att=0.06):
    n = int(dur * SR)
    x = A.pad(PAD[ch], dur, cut, att)
    return x[:n] * pump(len(x[:n]), b * T.BAR)


def bed(mix, b, intensity=1.0, light=False):
    """one bar of the groove (bar index b): kick, claps, hats (+ 16ths / open hats / ride in the back half), the pumping
    pad, chord stabs"""
    t0 = b * T.BAR
    ch = T.CHORDS[b]
    big = intensity > 1.3 and not light                                # the back half: the groove opens up
    for bt in range(4):
        mix.place('drums', kick(1.0), t0 + bt * T.BEAT, 0.44 if not light else 0.38)
        mix.place('drums', hat(0.9, False, 10 + bt), t0 + (bt + 0.5) * T.BEAT, 0.13)
        if bt % 2:
            mix.place('drums', A.clap(), t0 + bt * T.BEAT, 0.30)
        if big:
            mix.place('drums', hat(0.9, True, 20 + bt), t0 + (bt + 0.5) * T.BEAT, 0.07)
            mix.place('drums', ride(0.8, 30 + bt), t0 + bt * T.BEAT, 0.06)
    if intensity > 1.15 or light:                                       # 16th shaker / hats
        for st in range(16):
            if st % 2:
                mix.place('drums', shaker(0.6 + 0.4 * (st % 4 == 3), 40 + st), t0 + st * S16, 0.06 if not light else 0.04)
    cut = 900 + 1500 * min(1.0, max(0.0, (intensity - 1.0) / 0.6)) + (900 if big else 0)
    mix.place('pad', pad_bar(ch, b, T.BAR + 0.05, cut if not light else 1300), t0, 0.11)
    if not light:
        slots = STAB_SLOTS if big else STAB_SLOTS[1::2]
        for st in slots:
            mix.place('stabs', stab(STAB[ch], 0.10, 2200 if big else 1500), t0 + st * S16, 0.07,
                      pan=0.25 * (1 if st % 8 > 4 else -1))


def main():
    mix = A.Mix(T.DUR + 4.0)
    # ------------------------------------------------ bar 0: the VS title card, one unison house hit per word
    t1, t2, t3 = 0.004, T.BEAT, 2 * T.BEAT                 # TROMBONE 4 ms in (no click at the seam), VS, TUBA
    for tw_ in (t1, t2, t3):
        mix.place('drums', kick(1.0), tw_, 0.70)
        mix.place('stabs', stab(STAB['Fm'], 0.22, 3200), tw_, 0.14)
        mix.place('fx', A.thud(0.9, 120, 32, seed=int(tw_ * 50) + 3), tw_, 0.40)
        mix.place('drums', A.crash(1.2), tw_, 0.06)
    for m, g in ((48, 0.26), (53, 0.24), (56, 0.20), (60, 0.16)):     # TROMBONE: a slide blast gliding up a fifth into Fm
        mix.place('trombone', bone(m, 0.55, 1.0, 7.0, 0.16, cap=0.95), t1, g)
    nn = int(1.4 * SR)                                     # VS: a thunder crack where it splits the divider
    tt_ = A.tt(nn)
    crack = A.bp(np.random.default_rng(91).normal(0, 1, nn), 1500, 9000) * np.exp(-tt_ / 0.035)
    rumble = A.lp(np.random.default_rng(92).normal(0, 1, nn), 160) * np.clip(tt_ / 0.02, 0, 1) * np.exp(-tt_ / 0.5) * 3.0
    mix.place('fx', crack * 0.9 + rumble, t2, 0.40)
    mix.place('tuba', tuba(29, 2.0, 1.0), t3, 0.55)        # TUBA: a low blast, F1 under F2
    mix.place('tuba', tuba(41, 2.0, 1.0), t3, 0.50)
    mix.place('pad', A.pad(PAD['Fm'], 3 * T.BEAT + 0.1, 1100, 0.03), 0.0, 0.10)      # never quiet under the card
    for st, v in ((12, 0.5), (13, 0.65), (14, 0.8), (15, 0.95)):      # beat 4: a clap pickup as the doors open
        mix.place('drums', A.clap(), st * S16, 0.22 * v)
    mix.place('fx', A.whoosh(T.T_DOORS[1] - T.T_DOORS[0] + 0.1, 300, 5000), T.T_DOORS[0] - 0.05, 0.16)
    # ------------------------------------------------ bar 1: the count-in on the dominant (C)
    for j, (tc, txt) in enumerate(T.COUNT[:3]):
        mix.place('drums', kick(1.0), tc, 0.60)
        mix.place('stabs', stab(STAB['C'], 0.16, 2800), tc, 0.11)
        mix.place('trombone', bone(60, 0.2, 1.0, 3.0, 0.05), tc, 0.26)
        mix.place('trombone', bone(55, 0.2, 0.9, 3.0, 0.05), tc, 0.16)
        mix.place('tuba', tuba(36, 0.8, 0.95), tc, 0.55)
    mix.place('pad', A.pad(PAD['C'], T.BAR, 900, 0.4) * np.linspace(0.4, 1.0, int(T.BAR * SR)), T.BAR, 0.10)
    for st, v in ((12, 0.5), (13, 0.62), (14, 0.76), (15, 0.9)):      # the clap pickup into FIGHT!
        mix.place('drums', A.clap(), T.BAR + st * S16, 0.28 * v)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), T.BAR + 3 * T.BEAT, 0.14)
    mix.place('drums', A.crash(1.8), T.T_FIGHT, 0.12)               # the drop
    mix.place('fx', A.thud(0.7, 110, 34), T.T_FIGHT, 0.40)
    # ------------------------------------------------ the groove: bar 2 (FIGHT!) .. the K.O.; a sweep into bar 17
    bar_ko = int(T_KO // T.BAR)
    for b in range(2, bar_ko + 1):
        bed(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
    t17 = 17 * T.BAR
    mix.place('fx', riser(2 * T.BAR, over=0.0, seed=12) * 0.6, t17 - 2 * T.BAR, 0.10)
    mix.place('drums', A.crash(2.0), t17, 0.10)
    mix.place('fx', A.whoosh(3.6, 300, 6000), 44.0 - 3.6, 0.10)
    for k in ('drums', 'pad', 'stabs'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ TUBA: the bass line (oom = the root, pah = the fifth)
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'oom' and e[2] == KU:
            tq = q16(t)
            root = T.tuba_root(tq + 0.01)
            beats = max(1.0, (q16(e[3]) - tq) / T.BEAT + 0.4)
            sig, g, lead = colour(tuba(root, beats, 0.9), KU, t, 0.8, low=True)
            mix.place('tuba', sig, tq, 0.62 * g, pan=0.12)
            n = int((e[3] - t + 0.1) * SR)                        # the inhale: air sucked in, rising into the bell
            if n > 400:
                tt_ = A.tt(n)
                air = A.bp(np.random.default_rng(int(t * 10)).normal(0, 1, n), 300, 2400) * (tt_ / tt_[-1]) ** 2
                mix.place('fx', air, tq, 0.05, pan=0.1)
            if lead > 0.04:                                       # the leader: a sub-octave under the oom
                n = int(beats * T.BEAT * 0.9 * SR)
                tt_ = A.tt(n)
                sub = np.sin(2 * np.pi * A.mtof(root - 12) * tt_) * A.adsr(n, 0.02, 0.2, 0.7, 0.1)
                mix.place('tuba', sub, tq, 0.22 * min(1.0, lead * 4))
        elif kind == 'pah' and e[2] == KU:
            tq = q16(t)
            hp = hp_at(KU, t)
            caught = bool(e[3])
            if hp > 25 or caught:                                 # pah: the fifth as a short blast (gone below 25 HP)
                sig, g, _ = colour(tuba(T.tuba_root(tq + 0.01) + 7, 0.6, 0.95 if caught else 0.75), KU, t, 0.8, low=True)
                mix.place('tuba', sig, tq, (0.62 if caught else 0.45) * g, pan=0.12)
            if caught:                                            # the spit: an octave blast + the cannonball's whoosh
                mix.place('tuba', colour(tuba(T.tuba_root(tq + 0.01) + 12, 0.5, 1.0), KU, t, 0.8, low=True)[0], tq, 0.34)
                mix.place('fx', A.whoosh(0.6, 500, 7000), tq, 0.16)
        elif kind == 'gulp' and e[2] == KU:                       # gulp: a low bubble pop + a growl from the bell
            tq = q16(t)
            n = int(0.22 * SR)
            tt_ = A.tt(n)
            pop = np.sin(2 * np.pi * np.cumsum(180 * np.exp(-tt_ / 0.06) + 50) / SR) * np.exp(-tt_ / 0.08)
            mix.place('fx', pop, tq, 0.32)
            mix.place('tuba', colour(tuba(29, 0.5, 0.9), KU, t, 1.2, low=True)[0], tq, 0.30)
        elif kind == 'hit' and e[2] == KU and e[7] == 'cannon':   # into the ropes
            tq = q16(t)
            mix.place('fx', A.thud(0.5, 120, 40, seed=int(t * 7)), tq, 0.42)
            mix.place('drums', A.crash(1.2), tq, 0.08)
            mix.place('drums', kick(1.0), tq, 0.30)
    # ------------------------------------------------ TROMBONE: the thrusts are the tenor hook
    last_slot = None
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'thrust' and e[2] == KT:
            tq = q16(t)
            slot = round(tq / S16)
            if slot == last_slot:
                continue
            last_slot = slot
            hp = hp_at(KT, t) / 100.0
            if h01(slot, 7) < 0.5 * (1 - hp) ** 1.6:                   # sparse when hurt
                continue
            on_beat = slot % 4 == 0
            m = T.trombone_at(tq)
            if not on_beat:                                           # POWER SLIDE's 8ths: a passing note toward the next
                nxt = T.trombone_at(tq + T.BEAT / 2)
                pas = [x for x in T.scale_notes(min(m, nxt), max(m, nxt), T.chord_at(tq)) if x not in (m, nxt)]
                m = pas[len(pas) // 2] if pas else nxt
            sig, g, lead = colour(bone(m, 0.16 if on_beat else 0.1, 0.8 if on_beat else 0.6, 2.5 if on_beat else 1.0),
                                  KT, t, 0.8)
            mix.place('trombone', sig, tq, (0.30 if on_beat else 0.20) * g, pan=-0.2)
            if lead > 0.04 and on_beat:                               # the leader: a harmony a third below
                under = [x for x in T.chord_tones(m - 5, m - 3, T.chord_at(tq))]
                if under:
                    mix.place('trombone', colour(bone(under[-1], 0.15, 0.7, 2.5), KT, t)[0], tq, 0.15 * g, pan=-0.25)
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'hit' and e[2] == KT and e[7] == 'blade':          # a landed lance: punchy accent, big smear
            tq = q16(t)
            ct = T.chord_tones(60, 70, T.chord_at(tq))
            m = ct[int(e[8]) % len(ct)]
            sig, g, _ = colour(bone(m, 0.18, 1.0, 5.0, 0.08), KT, t)
            mix.place('trombone', sig, tq, 0.30 * g, pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
            mix.place('fx', A.thud(0.25, 140, 60, seed=int(t * 7)), tq, 0.10)
        elif kind == 'hit' and e[2] == KT and e[7] == 'shockwave':
            mix.place('fx', A.thud(0.4, 120, 45), q16(t), 0.25)
        elif kind == 'block':                                         # the lance bangs off the brass dome
            mix.place('fx', clank(84, int(t * 5)), q16(t), 0.06)
            mix.place('fx', A.thud(0.2, 160, 80), q16(t), 0.05)
        elif kind == 'parry':
            mix.place('fx', clank(88, int(t * 9)), t, 0.04)
        elif kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.05 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            mix.place('fx', clank(79, int(t * 3)), t, 0.04)
        elif kind == 'ability':
            who, name = e[2], e[3]
            tq = q16(t)
            if name == 'SAD TROMBONE':                                # wah wah wah waaah
                for j, m in enumerate((60, 59, 58, 57)):
                    d = 0.5 * T.BEAT if j < 3 else 1.6 * T.BEAT
                    x = bone(m, d, 0.9, 1.0 if j else 0.0, 0.04, cap=round(d + 0.35, 2))
                    if j == 3:
                        x = vibrato(x, 0.5, 5.0, 0.2)
                    x = wah(x, [(0.0, min(0.18, d * 0.6))])
                    mix.place('trombone', colour(x, KT, t)[0], tq + j * 0.5 * T.BEAT, 0.40, pan=-0.15)
                mix.place('fx', A.whoosh(0.6, 200, 1500, rise=False), tq, 0.10)
            elif name == 'POWER SLIDE':                               # a long rip up an octave into the hook
                m = T.trombone_at(tq)
                mix.place('trombone', colour(bone(m, 0.4, 1.0, 12.0, 0.28), KT, t)[0], tq, 0.30)
            elif name == 'PEDAL TONE':                                # the pedal Bb under the hovering bell
                mix.place('trombone', bone(34, 2 * T.BEAT, 1.0, 2.0, 0.1, cap=round(2 * T.BEAT + 0.3, 2)), tq, 0.45)
                mix.place('fx', A.whoosh(2 * T.BEAT, 150, 1800), tq, 0.10)
            elif name == 'OOMPAH':                                    # the tuba rips up an octave into the double time
                x = tuba(T.tuba_root(tq + 0.01) + 12, 0.8, 1.0)
                mix.place('tuba', colour(gliss(x, 12, 0.22), KU, t, 0.8, low=True)[0], tq, 0.36)
            elif name == 'BRASS WALL':                                # a buzzing root under a metallic shimmer
                dur = RD.CFG[who]['abilities']['unlock2']['dur_beats'] * T.BEAT
                mix.place('tuba', colour(tuba(T.tuba_root(tq + 0.01), dur / T.BEAT, 0.8), KU, t, 0.6, low=True)[0],
                          tq, 0.20)
                n = int(dur * SR)
                tt_ = A.tt(n)
                shim = sum(np.sin(2 * np.pi * A.mtof(m) * tt_) for m in (89, 96, 101)) / 3
                shim = shim * (0.6 + 0.4 * np.sin(2 * np.pi * 6.0 * tt_)) * A.adsr(n, 0.05, 0.3, 0.6, 0.5)
                mix.place('fx', shim, tq, 0.025)
                mix.place('drums', A.crash(1.2), tq, 0.08)
            elif name == 'SUBSONIC':
                tele = RD.CFG[who]['abilities']['low']['tele'] * T.BEAT
                mix.place('tuba', sub_drone(tele + 0.05), tq, 0.40)
        elif kind == 'slam' and e[2] == KT:                           # PEDAL TONE: the bell stomps
            ts = e[0]
            mix.place('drums', kick(1.0), ts, 0.7)
            mix.place('fx', A.thud(0.8, 90, 30), ts, 0.6)
            mix.place('fx', clank(65, 9), ts, 0.10)
            mix.place('trombone', bone(41, 0.4, 1.0, 0), ts, 0.30)
            mix.place('drums', A.crash(1.6), ts, 0.10)
        elif kind == 'slam' and e[2] == KU:                           # SUBSONIC lands: a ring-shaking low note
            ts = e[0]
            mix.place('tuba', tuba(29, 2.5, 1.0), ts, 0.70)
            mix.place('tuba', sub_drone(1.2, 26, 30), ts, 0.30)
            mix.place('fx', A.thud(0.9, 90, 28), ts, 0.65)
            mix.place('drums', kick(1.0), ts, 0.6)
    # ------------------------------------------------ the K.O. moment
    mix.place('drums', A.crash(2.4), T_KO, 0.16)
    mix.place('fx', A.thud(0.9, 120, 30), T_KO, 0.7)
    loser = NAMES[LOSE]
    if loser == 'TROMBONE':                                           # the slide falls apart: a falling wail, clattering tubing
        x = bone(60, 0.9, 1.0, 0, cap=1.3)
        mix.place('trombone', gliss(x, -10, 0.9), RD.T_HS, 0.24)
        for j, m in enumerate((80, 75, 85, 71)):
            mix.place('fx', clank(m, 30 + j), RD.T_HS + 0.1 * j, 0.08, pan=(j - 1.5) / 4)
    else:                                                             # the tuba collapses: a falling groan, valves rattling
        x = tuba(41, 2.0, 1.0)
        mix.place('tuba', gliss(x, -7, 0.9) * np.linspace(1, 0.3, len(x)), RD.T_HS, 0.40)
        for j, m in enumerate((77, 72, 82, 69, 75)):
            mix.place('fx', clank(m, 20 + j), RD.T_HS + 0.09 * j, 0.08, pan=(j - 2) / 5)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.16)
    tw = q16(T_KO + 1.3)                                              # the winner's stinger on "(WINNER) WINS"
    if NAMES[WIN] == 'TUBA':
        mix.place('solo', tuba(29, 2.0, 1.0), tw, 0.55)
        mix.place('solo', tuba(41, 2.0, 1.0), tw, 0.50)
        mix.place('solo', tuba(48, 2.0, 0.9), tw, 0.30)
    else:
        for m, g in ((53, 0.30), (56, 0.26), (60, 0.24), (65, 0.16)):
            mix.place('solo', bone(m, 1.0, 1.0, 3.0, 0.1, cap=1.5), tw, g)
    mix.place('stabs', stab(STAB['Fm'], 0.3, 3200), tw, 0.12)
    mix.place('drums', kick(1.0), tw, 0.55)
    mix.place('drums', A.crash(1.8), tw, 0.10)
    solo0 = RD.T_SOLO
    mix.place('fx', A.whoosh(solo0 - tw - 0.1, 300, 5000), tw + 0.1, 0.10)
    # ------------------------------------------------ victory: a lighter groove + the winner's solo
    b_solo = int(round(solo0 / T.BAR))
    for b in range(b_solo, 31):
        bed(mix, b, light=True)
    prev = None
    for bt, m, d in T.SOLO[NAMES[WIN]]:
        ts = solo0 + bt * T.BEAT
        if ts > T.T_REASSEMBLE - 0.05:                                 # a late K.O. leaves less room: stop at the reassembly
            break
        if NAMES[WIN] == 'TROMBONE':                                  # the slide glides from the last note to this one
            gl = float(np.clip(m - prev, -7, 7)) if prev is not None else 3.0
            mix.place('solo', bone(m, d * T.BEAT * 0.95, 0.95, gl, 0.07, cap=round(d * T.BEAT + 0.35, 2)), ts, 0.46)
        else:
            mix.place('solo', tuba(m, d * 0.95, 0.95), ts, 0.62)
        prev = m
    # the loser reassembles: its pieces fly home as a rising run, a glass "pop" when it's whole
    run = [53, 56, 60, 63, 65, 68] if loser == 'TROMBONE' else [29, 34, 36, 41, 44, 48]
    for j, m in enumerate(run):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
        if loser == 'TROMBONE':
            mix.place('trombone', bone(m, 0.12, 0.7), tj, 0.16)
        else:
            mix.place('tuba', tuba(m, 0.35, 0.7), tj, 0.32)
    mix.place('fx', A.chime(95, 0.5) + A.chime(102, 0.5) * 0.5, T.T_GLIDE, 0.12)
    # ------------------------------------------------ bar 31: build into the title slam on sample 0
    t31 = 31 * T.BAR
    st = 0.0
    while t31 + st < T.DUR - 1e-4:                                    # clap roll: 8ths, then 16ths for the last beat
        v = 0.45 + 0.55 * st / (T.DUR - t31)
        mix.place('drums', A.clap(), t31 + st, 0.26 * v)
        st += T.BEAT / 2 if st < 3 * T.BEAT - 1e-6 else S16
    for bt in range(4):
        mix.place('drums', kick(0.9), t31 + bt * T.BEAT, 0.46)
        mix.place('tuba', tuba((36, 40, 43, 46)[bt], 0.9, 0.85), t31 + bt * T.BEAT, 0.50)    # the tuba climbs the C chord
    mix.place('fx', riser(T.DUR - t31), t31, 0.22)
    sw = A.pad(PAD['C'], T.DUR - t31 + 0.012, 2400, T.DUR - t31)     # a pad swelling right up to the seam
    mix.place('pad', sw * np.linspace(0.3, 1.0, len(sw)), t31, 0.12)
    nc = int((T.T_CLOSE + 0.012) * SR)                              # the doors sliding shut into the title slam
    tcl = A.tt(nc)
    doors = A.bp(np.random.default_rng(93).normal(0, 1, nc), 400, 5000) * (tcl / T.T_CLOSE) ** 2
    mix.place('fx', doors, T.DUR - T.T_CLOSE, 0.45)
    for tl, k, nch in impact_landings(RD.CAPTIONS):                 # caption landings: soft thuds
        mix.place('fx', A.thud(), tl, 0.08 if k == 0 else 0.06)
    out, stems = master(mix)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    for k, s in stems.items():
        A.write(os.path.join(HERE, 'check', f'stem_{k}.wav'), s)
    spans = [(0, T.BAR), (T.BAR, T.T_FIGHT), (T.T_FIGHT, 10), (10, 20), (20, 32), (32, 44), (44, T_KO), (T_KO, solo0),
             (solo0, 57.5), (57.5, 61)]
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


BUS_GAIN = {'trombone': 1.4, 'tuba': 2.4, 'solo': 1.6, 'fx': 1.4, 'drums': 1.0, 'pad': 1.6, 'stabs': 1.2}
# section automation (dB), applied AFTER the compressor so the limiter can't flatten the build: the card + count-in at
# full, the early fight pulled back, rising through the back half, the victory section loud, the end close to the start
GAIN_PTS = [(0, 0.6), (1.9, 0.3), (3.6, 0.0), (4.1, -2.3), (20, -2.1), (30, -0.8), (32.4, 0.6), (40, 1.3), (44, 2.3),
            (49.0, 3.3), (50.5, 2.8), (51.5, 3.3), (58.0, 3.3), (60.0, 1.6), (61.02, 0.6)]


def master(mix, thr=0.22, ceiling=0.89):
    """Mix.master with the overflow past the loop (tails, reverb) folded onto the start; NA samples (whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('trombone', 0.35), ('solo', 0.4), ('fx', 0.3), ('stabs', 0.5),
                                                ('pad', 0.4), ('drums', 0.12), ('tuba', 0.08)))
    wet = A.reverb(send) * 0.20
    full = sum(buses.values()) + wet

    def fold(x):
        y = x[:NS].copy()
        tail = x[NS:]
        y[:len(tail)] += tail[:NS]
        return y
    m = fold(full)
    from scipy.signal import lfilter
    env = np.sqrt(lfilter([0.002], [1, -0.998], (np.concatenate([m[-SR:], m]) ** 2).mean(1)))[SR:]
    gain = np.where(env > thr, (thr / np.maximum(env, 1e-9)) ** 0.35, 1.0)
    g = np.interp(np.arange(NS) / SR, [p[0] for p in GAIN_PTS], [p[1] for p in GAIN_PTS])
    gl = (10 ** (g / 20))
    gain = (gain * gl)[:, None]
    m = m * gain
    s1 = ceiling / (np.percentile(np.abs(m), 99.97) + 1e-9)     # the rare transient above it is soft-clipped
    m = np.tanh(m * s1 * 1.1) / np.tanh(1.1)
    s2 = ceiling / (np.abs(m).max() + 1e-9)
    m *= s2
    stems = {}
    for k in buses:
        stems[k] = fold(buses[k]) * gain * s1 * s2
    return m, stems


if __name__ == '__main__':
    main()
