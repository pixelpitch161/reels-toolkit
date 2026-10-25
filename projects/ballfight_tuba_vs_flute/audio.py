"""#32 BALLFIGHT #6: Tuba vs Flute - soundtrack (villain march), all synthesised, original.

The march has no bass of its own and no high lead: the two fighters ARE the bottom and the top, both from the honest run:
  TUBA    every wave is the bass: a main wave (on the bar line) plays the chord root as a long "oom", OOMPAH's extra waves
          (beat 3) play the fifth, so the haste is literally an oom-pah; a landed wave adds a chromatic walk-up into
          the next bar and a deep boom; the leader gets a sub-octave under it. As its HP drops the tuba thins and rasps
          (walk-ups gone below 45 HP, the fifth below 25 HP, a lonely, buzzing root).
  FLUTE   every volley is a run: n darts = n notes rushing up the chord (32nds) on the beat, so the growing volley is a
          growing flourish; FLUTTER TONGUE flutter-tongues them, TRILL is a trill across the arc, PICCOLO a piercing
          shriek; every landed dart is a short high "tu". As its HP drops the runs get breathy and sparse.
Bed: C minor at 125.90 BPM (32 bars = 61 s): a snare ostinato and a heavy bass drum on 1 and 3, low brass "pah" chords
on 2 and 4, a slow horn swell, and a sinister music-box counter-line (the villain's lullaby) from bar 3, timpani and
crashes building through the back half. The count-in bar (a tuba "oom" + a flute chirp + the snare per count, a snare
pickup) drops on FIGHT!; at the K.O. the bed tape-stops with the hit-stop, the loser falls apart, the crowd roars, the
winner's stinger lands, then its solo over a lighter march; bar 31 builds (roll, riser, the bass climbing) into the "3"
on sample 0. Every hit is snapped to the nearest 16th. Tails past the loop fold onto the start.

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
KT, KF = NAMES.index('TUBA'), NAMES.index('FLUTE')
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
        ref = x if name == 'TUBA' else A.hp(x, 120)
        _GAIN[name] = min(4.0, 0.16 / (np.sqrt((ref ** 2).mean()) + 1e-6))
    return _GAIN[name]


def note(name, midi, dur=0.18, cap=0.6, vel=0.85):
    key = (name, int(midi), round(dur, 3), cap, round(vel, 2))
    if key not in _VC:
        _VC[key] = _fit(B.voice(name, int(midi), vel, dur), cap) * inst_gain(name)
    return _VC[key]


def tuba(m, beats=1.0, vel=0.85):
    """a tuba note held for `beats` (its Pixel Pitch voice, stretched with a soft sustain under it)"""
    dur = beats * T.BEAT * 0.92
    x = note('TUBA', m, round(min(dur, 0.4), 3), round(dur + 0.2, 2), round(vel, 1)).copy()
    if dur > 0.36:                                         # hold: a buzzing sustain where the sampled note decays
        n = len(x)
        tt = A.tt(n)
        f = A.mtof(m)
        ph = (f * tt) % 1.0
        sus = A.lp(0.6 * (2 * ph - 1) + 0.4 * np.sin(2 * np.pi * f * tt), 700) * 0.22 * vel
        env = np.clip((tt - 0.12) / 0.15, 0, 1) * np.clip((dur - tt) / 0.06, 0, 1)
        x += sus * env * inst_gain('TUBA') * 0.5
    return x


def flute(m, dur=0.1, vel=0.8):
    return note('FLUTE', m, round(max(0.06, dur), 3), round(max(0.06, dur) + 0.18, 2), round(vel, 1))


def hp_at(k, t):
    return RD.hp_at(k, t)


def colour(x, k, t, drive_k=1.0, low=False):
    """the fighter's tone at time t: thinner + dirtier as its HP drops; returns (signal, gain, lead). The tuba (`low`)
    keeps its fundamental: it gets raspy (drive) and a little thinner, not high-passed away"""
    hp = hp_at(k, t) / 100.0
    lead = (hp_at(k, t) - hp_at(1 - k, t)) / 100.0
    cut = (30 + 90 * (1 - hp) ** 1.6) if low else (90 + 520 * (1 - hp) ** 1.6)
    y = A.hp(x, cut)
    drive = 1 + 4.5 * drive_k * (1 - hp) ** 2
    y = np.tanh(drive * y * 1.2) / np.tanh(drive * 1.2) * (1.0 / (1 + 0.15 * (drive - 1)))
    g = 10 ** (float(np.clip(6.0 * lead, -4.0, 3.5)) / 20)
    return y, g, lead


def breath(dur, seed=5):
    """flute breath: airy band noise"""
    n = int(dur * SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    return A.bp(x, 1500, 7000) * A.expenv(n, dur * 0.5, 0.01)


def flutter(x, rate=24.0):
    """flutter-tongue: a fast growl on the tone"""
    tt = A.tt(len(x))
    return x * (0.55 + 0.45 * np.sign(np.sin(2 * np.pi * rate * tt)))


def shriek(dur=1.6, f0=2093.0):
    """PICCOLO: a piercing, wobbling high whistle with a hard edge"""
    n = int(dur * SR)
    tt = A.tt(n)
    f = f0 * (1 + 0.012 * np.sin(2 * np.pi * 7.5 * tt)) * (1 + 0.06 * np.clip(tt / 0.08, 0, 1))
    ph = 2 * np.pi * np.cumsum(f) / SR
    x = np.sin(ph) + 0.3 * np.sin(2 * ph) + 0.12 * np.sin(3 * ph)
    x = np.tanh(1.8 * x) * np.clip(tt / 0.02, 0, 1) * np.exp(-tt / (dur * 0.6))
    return x + 0.25 * breath(dur, 21)


def sub_drone(dur, f0=30.0, f1=48.0):
    """SUBSONIC's telegraph: a sub-bass growl climbing toward the slam"""
    n = int(dur * SR)
    tt = A.tt(n)
    u = tt / dur
    f = f0 + (f1 - f0) * u ** 1.5
    ph = 2 * np.pi * np.cumsum(f) / SR
    x = np.sin(ph) + 0.4 * np.sin(2 * ph) * (0.5 + 0.5 * np.sin(2 * np.pi * 9 * tt))
    return np.tanh(2.0 * x) * np.clip(tt / 0.1, 0, 1) * (0.4 + 0.6 * u)


# ---------------------------------------------------------------- bed synths (original)
def brass(ms, dur, att=0.012, cut=1500, bite=1.0):
    """low brass section: saws per note, a bright 'bwah' filter bite at the attack, slightly detuned"""
    n = int(dur * SR)
    tt = A.tt(n)
    x = np.zeros(n)
    for j, m in enumerate(ms):
        f = A.mtof(m)
        for d in (-0.004, 0.004):
            ph = (f * (1 + d) * tt + 0.37 * j) % 1.0
            x += 2 * ph - 1
    x /= 2 * max(1, len(ms))
    env = A.adsr(n, att, 0.08, 0.7, min(0.06, dur * 0.3))
    k = np.exp(-tt / 0.07)
    y = A.lp(x, cut * 0.45) * (1 - k * bite) + A.lp(x, cut * 1.6) * k * bite
    return y * env


def horn_pad(ms, dur):
    n = int(dur * SR)
    tt = A.tt(n)
    x = np.zeros(n)
    for m in ms:
        f = A.mtof(m)
        x += np.sin(2 * np.pi * f * tt) + 0.35 * np.sin(4 * np.pi * f * tt) + 0.15 * np.sin(6 * np.pi * f * tt)
    x /= max(1, len(ms))
    return A.lp(x, 1300) * A.adsr(n, min(0.5, dur * 0.3), 0.2, 0.85, min(0.4, dur * 0.3)) * (1 + 0.05 * np.sin(2 * np.pi * 4.8 * tt))


def music_box(m, vel=1.0):
    """a music-box tine: bright sine + an inharmonic partial, short ring, a little detune (out-of-tune, sinister)"""
    n = int(0.9 * SR)
    tt = A.tt(n)
    f = A.mtof(m) * (1 - 0.004)
    x = np.sin(2 * np.pi * f * tt) * np.exp(-tt / 0.32) + 0.35 * np.sin(2 * np.pi * f * 2.76 * tt) * np.exp(-tt / 0.06)
    x += 0.15 * np.sin(2 * np.pi * f * 5.4 * tt) * np.exp(-tt / 0.02)
    return x * vel


def bass_drum(vel=1.0):
    return A.kick(0.6, 0.45, 110, 40) * vel


def march_snare(vel=1.0):
    n = int(0.22 * SR)
    tt = A.tt(n)
    x = A.snare(0.11)[:n]
    x = np.pad(x, (0, max(0, n - len(x))))
    rattle = A.bp(np.random.default_rng(31).normal(0, 1, n), 3000, 9000) * np.exp(-tt / 0.07)
    return (x + 0.35 * rattle) * vel


def timpani(m, vel=1.0):
    n = int(1.2 * SR)
    tt = A.tt(n)
    f = A.mtof(m)
    x = np.sin(2 * np.pi * f * tt * (1 + 0.02 * np.exp(-tt / 0.05))) + 0.4 * np.sin(2 * np.pi * f * 1.5 * tt) * np.exp(-tt / 0.2)
    x *= np.exp(-tt / 0.5)
    x += 0.3 * A.bp(np.random.default_rng(int(m)).normal(0, 1, n), 100, 900) * np.exp(-tt / 0.02)
    return x * vel


def roll(dur, f0=0.3, step=None):
    step = step or S16 / 2
    n = int(dur * SR)
    out = np.zeros(n + SR)
    t = 0.0
    while t < dur - 1e-4:
        a = int(t * SR)
        s = march_snare(f0 + (1 - f0) * (t / dur) ** 1.5) * 0.7
        out[a:a + len(s)] += s[:len(out) - a]
        t += step
    return out[:n]


def riser(dur, over=0.012, seed=11):
    """a noise + saw riser that builds right up to (and `over` s past) its end: no fade, so the loop seam stays loud"""
    n = int((dur + over) * SR)
    tt = A.tt(n)
    u = np.clip(tt / dur, 0, 1.05)
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


def roar(dur, seed=3):
    n = int(dur * SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    x = A.bp(x, 300, 2600) * 0.5 + A.bp(x, 900, 5000) * 0.25
    tt = A.tt(n)
    env = np.clip(tt / 0.25, 0, 1) * np.exp(-np.maximum(tt - 0.6, 0) / (dur * 0.45))
    return x * env


VOICING = {'Cm': (55, 60, 63), 'Ab': (56, 60, 63), 'Db': (56, 59, 61, 65), 'G': (55, 59, 62, 65)}
SNARE16 = {0: 1.0, 3: 0.45, 4: 0.75, 6: 0.5, 8: 0.9, 11: 0.45, 12: 0.7, 13: 0.4, 14: 0.5, 15: 0.62}


def bed(mix, b, intensity=1.0, light=False):
    """one bar of the march (bar index b): snare ostinato + bass drum, brass 'pah' on 2 and 4, horn swell, music box"""
    t0 = b * T.BAR
    ch = T.CHORDS[b]
    for st, v in SNARE16.items():
        if light and st % 4:
            continue
        mix.place('drums', march_snare(v), t0 + st * S16, (0.20 if light else 0.24) * (0.8 + 0.2 * intensity), pan=-0.1)
    for bt in (0, 2):
        mix.place('drums', bass_drum(1.0 if bt == 0 else 0.8), t0 + bt * T.BEAT, 0.21 if not light else 0.18)
    vo = VOICING[ch]
    for bt in (1, 3):
        mix.place('brass', brass(vo, 0.22), t0 + bt * T.BEAT, 0.20 if light else 0.24)
        if intensity > 1.35 and not light:
            mix.place('brass', brass(vo, 0.12, bite=0.6), t0 + (bt + 0.5) * T.BEAT, 0.12)
    if b >= 3 or light:
        mix.place('pad', horn_pad([vo[0], vo[1], vo[2]], T.BAR), t0, 0.07)            # no low note: the tuba owns the bass
    if b >= 4 and not light:                                   # the villain's lullaby on top
        lb = (b - 2) % 4
        for j in range(0, 8, 2):                               # v2: quarters only, softer (the charmer line leads)
            m = T.MUSIC_BOX[lb * 8 + j]
            if m is not None:
                mix.place('musicbox', music_box(m + 12, 0.9), t0 + j * 2 * S16, 0.06, pan=0.35)
    if intensity > 1.3 and not light:                          # timpani on the downbeat + a crash every 4 bars
        mix.place('drums', timpani(T.CHORD_ROOT[ch] - 12 + 24, 0.9), t0, 0.16 * min(1.0, (intensity - 1.3) / 0.2))
        if b % 4 == 1:
            mix.place('drums', A.crash(1.6), t0, 0.07)


# ---------------------------------------------------------------- the fight's own parts
def main():
    mix = A.Mix(T.DUR + 4.0)
    # ------------------------------------------------ v3 title card (bar 0, the villain's Cm): one unison band hit per word
    t1, t2, t3 = 0.004, T.BEAT, 2 * T.BEAT                 # TUBA 4 ms in (no click at the seam), VS, FLUTE
    for tw_ in (t1, t2, t3):                               # the whole band on every word: bass drum, snare, brass, timpani
        mix.place('drums', bass_drum(1.0), tw_, 0.55)
        mix.place('drums', march_snare(1.0), tw_, 0.30)
        mix.place('brass', brass((48, 55, 60, 63), 0.42, bite=1.0), tw_, 0.30)
        mix.place('drums', timpani(36, 1.0), tw_, 0.30)
        mix.place('fx', A.thud(0.9, 120, 32, seed=int(tw_ * 50) + 3), tw_, 0.45)
    # TUBA: the tuba's blast, low C with the low G under it
    mix.place('tuba', tuba(36, 1.6, 1.0), t1, 0.70)
    mix.place('tuba', tuba(31, 1.6, 1.0), t1, 0.45)
    # VS: a thunder crack where it splits the divider
    nn = int(1.4 * SR)
    tt0 = A.tt(nn)
    crack = A.bp(np.random.default_rng(91).normal(0, 1, nn), 1500, 9000) * np.exp(-tt0 / 0.035)
    rumble = A.lp(np.random.default_rng(92).normal(0, 1, nn), 160) * np.clip(tt0 / 0.02, 0, 1) * np.exp(-tt0 / 0.5) * 3.0
    mix.place('fx', crack * 0.9 + rumble, t2, 0.42)
    mix.place('drums', A.crash(1.6), t2, 0.10)
    # FLUTE: a shrieking flurry up to a flutter-tongued high G
    for j, m in enumerate((79, 84, 87, 91)):
        mix.place('flute', flute(m, 0.06, 1.0), t3 + j * S16 / 2, 0.32, pan=0.15)
    mix.place('flute', flutter(flute(91, 0.5, 1.0), 26), t3 + 2 * S16, 0.36, pan=0.15)
    mix.place('flute', breath(0.4, 81), t3, 0.10, pan=0.15)
    # beat 4: the hold, a snare pickup as the doors open, a horn pad under the whole card (never quiet)
    mix.place('pad', horn_pad([48, 55, 60, 63], 3 * T.BEAT), 0.0, 0.10)
    for st, v in ((12, 0.45), (13, 0.6), (14, 0.75), (15, 0.9)):
        mix.place('drums', march_snare(v), st * S16, 0.30)
    mix.place('fx', A.whoosh(T.T_DOORS[1] - T.T_DOORS[0] + 0.1, 300, 5000), T.T_DOORS[0] - 0.05, 0.16)
    # ------------------------------------------------ count-in (bar 1, on the dominant G): tuba oom + flute chirp + snare
    for j, (tc, txt) in enumerate(T.COUNT[:3]):
        mix.place('tuba', tuba(43, 0.9, 0.9), tc, 0.55)
        mix.place('flute', flute(79 + (0, 4, 7)[j], 0.12, 0.9), tc, 0.30, pan=0.2)
        mix.place('drums', march_snare(1.0), tc, 0.28)
        mix.place('brass', brass(VOICING['G'], 0.16), tc, 0.20)
    for st, v in ((12, 0.45), (13, 0.6), (14, 0.75), (15, 0.9)):   # the snare pickup into FIGHT!
        mix.place('drums', march_snare(v), T.BAR + st * S16, 0.30)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), T.BAR + 3 * T.BEAT, 0.14)
    mix.place('drums', A.crash(2.0), T.T_FIGHT, 0.14)
    mix.place('drums', bass_drum(1.0), T.T_FIGHT, 0.5)
    # ------------------------------------------------ the bed: bars 2 (FIGHT!) .. the K.O.
    bar_ko = int(T_KO // T.BAR)
    for b in range(2, bar_ko + 1):
        bed(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
    mix.place('fx', A.whoosh(3.6, 300, 6000), 44.0 - 3.6, 0.10)
    for k in ('drums', 'brass', 'pad', 'musicbox'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ TUBA (v2, OOM-PAH SWALLOW): oom = the low inhale note, pah = the blast
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'oom' and e[2] == KT:
            tq = q16(t)
            ch = T.chord_at(tq + 0.01)
            beats = max(1.0, (q16(e[3]) - tq) / T.BEAT + 0.4)
            sig, g, lead = colour(tuba(T.CHORD_ROOT[ch], beats, 0.9), KT, t, 0.8, low=True)
            mix.place('tuba', sig, tq, 0.62 * g, pan=-0.12)
            n = int((e[3] - t + 0.1) * SR)                        # the inhale: air sucked in, rising into the bell
            if n > 400:
                tt_ = A.tt(n)
                air = A.bp(np.random.default_rng(int(t * 10)).normal(0, 1, n), 300, 2400) * (tt_ / tt_[-1]) ** 2
                mix.place('fx', air, tq, 0.05, pan=-0.1)
            if lead > 0.04:                                       # the leader: a sub-octave under the oom
                n = int(beats * T.BEAT * 0.9 * SR)
                tt_ = A.tt(n)
                sub = np.sin(2 * np.pi * A.mtof(T.CHORD_ROOT[ch] - 12) * tt_) * A.adsr(n, 0.02, 0.2, 0.7, 0.1)
                mix.place('tuba', sub, tq, 0.22 * min(1.0, lead * 4))
        elif kind == 'pah' and e[2] == KT:
            tq = q16(t)
            hp = hp_at(KT, t)
            ch = T.chord_at(tq + 0.01)
            caught = bool(e[3])
            if hp > 25 or caught:                                 # pah: the fifth as a short blast (gone below 25 HP)
                sig, g, _ = colour(tuba(T.CHORD_FIFTH[ch], 0.6, 0.95 if caught else 0.75), KT, t, 0.8, low=True)
                mix.place('tuba', sig, tq, (0.62 if caught else 0.45) * g, pan=-0.12)
            if caught:                                            # the spit: a brass blast + the cannonball's whoosh
                mix.place('brass', brass((43, 50, 55, 59), 0.3, bite=1.0), tq, 0.30)
                mix.place('fx', A.whoosh(0.6, 500, 7000), tq, 0.16)
        elif kind == 'gulp' and e[2] == KT:                       # gulp: a low bubble pop + a growl from the bell
            tq = q16(t)
            n = int(0.22 * SR)
            tt_ = A.tt(n)
            pop = np.sin(2 * np.pi * np.cumsum(180 * np.exp(-tt_ / 0.06) + 50) / SR) * np.exp(-tt_ / 0.08)
            mix.place('fx', pop, tq, 0.32)
            mix.place('tuba', colour(tuba(31, 0.5, 0.9), KT, t, 1.2, low=True)[0], tq, 0.30)
        elif kind == 'hit' and e[2] == KT and e[7] == 'cannon':   # into the ropes
            tq = q16(t)
            mix.place('fx', A.thud(0.5, 120, 40, seed=int(t * 7)), tq, 0.42)
            mix.place('drums', A.crash(1.2), tq, 0.08)
            mix.place('drums', bass_drum(1.0), tq, 0.35)
    # ------------------------------------------------ FLUTE (v2, SNAKE CHARMER): the charmer line while its cobras are up
    rise = RD.CFG[KF]['weapon']['rise'] * T.BEAT
    t_up = min([b['t_drop'] for b in RD.BASKETS.values()] + [99.0]) + rise
    dances = RD.DANCES
    k0, k1 = int(math.ceil(t_up / S16)), int(T_KO / S16)
    for slot in range(k0, k1 + 1):
        tq = slot * S16
        cn = T.charmer_note(slot)
        hasted = float(RUN['HA'][RD.sim_i(tq), KF]) > 0
        if cn is None and not (hasted and T.charmer_note(slot - 1) is not None):
            continue
        if cn is None:                                            # FLUTTER TONGUE: an extra grace note between
            m0, _ = T.charmer_note(slot - 1)
            cn = (m0 + 1 if (m0 + 1) % 12 in [(T.KEY_ROOT + q) % 12 for q in T.SCALE] else m0 + 2, 1)
        m, n16 = cn
        ch = T.chord_at(tq + 0.01)
        if ch == 'Db' and m % 12 == 2:
            m -= 1
        if any(a_ <= tq < b_ for a_, b_ in dances) and m + 12 <= 96:
            m += 12                                               # SNAKE DANCE: the line climbs an octave
        hp = hp_at(KF, tq) / 100.0
        if h01(slot, 5) < 0.55 * (1 - hp) ** 1.6:                # sparse when hurt
            continue
        x = flute(m, min(0.5, n16 * S16 * 0.9), 0.8)
        if hasted:
            x = flutter(x)
        sig, g, lead = colour(x, KF, tq, 0.9)
        mix.place('flute', sig, tq, 0.24 * g, pan=0.22)
        if (1 - hp) > 0.3 and n16 >= 2:
            mix.place('flute', breath(0.1, slot), tq, 0.08 * (1 - hp), pan=0.22)
        if lead > 0.04 and n16 >= 2:                              # the leader: a thin octave double
            mix.place('flute', colour(flute(m - 12, min(0.5, n16 * S16 * 0.9), 0.6), KF, tq)[0], tq, 0.08 * g, pan=0.25)
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'strike' and e[2] == KF:                       # a hiss-click on every strike
            tq = q16(t)
            n = int(0.09 * SR)
            tt_ = A.tt(n)
            hiss = A.bp(np.random.default_rng(int(t * 50)).normal(0, 1, n), 3500, 10000) * np.exp(-tt_ / 0.03)
            click = np.sin(2 * np.pi * 2800 * tt_) * np.exp(-tt_ / 0.004)
            mix.place('flute', hiss * 0.8 + click * 0.5, tq, 0.10, pan=float(np.clip(math.cos(e[4]) * 0.4, -0.5, 0.5)))
        elif kind == 'hit' and e[2] == KF and e[7] == 'fang':     # a bite lands: a high accent
            tq = q16(t)
            m = T.chord_notes(84, 96, T.chord_at(tq + 0.01))[int(e[8]) % 3]
            sig, g, _ = colour(flute(m, 0.07, 1.0), KF, t, 0.9)
            mix.place('flute', sig, tq, 0.16 * g, pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'basket' and e[2] == KF:                     # a basket lands: a soft wicker thump
            mix.place('fx', A.thud(0.18, 220, 120, seed=int(t * 9)), t + 0.05, 0.08)
        elif kind == 'hit' and e[2] == KF and e[7] == 'wave':
            mix.place('fx', A.thud(0.45, 140, 60), q16(t), 0.30)
        elif kind == 'block':                                     # a fang pings off the brass dome
            mix.place('fx', A.chime(96, 0.4) * 0.6 + A.chime(91, 0.4) * 0.4, q16(t), 0.06)
        elif kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.05 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            mix.place('fx', A.chime(91, 0.25), t, 0.05)
        elif kind == 'ability':
            who, name = e[2], e[3]
            tq = q16(t)
            if name == 'OOMPAH':
                mix.place('brass', brass((48, 55, 60, 63), 0.5), tq, 0.22)
            elif name == 'BRASS WALL':
                dur = RD.CFG[who]['abilities']['unlock2']['dur_beats'] * T.BEAT
                mix.place('brass', brass((43, 50, 55, 59, 62), dur, att=0.15, cut=900, bite=0.3), tq, 0.20)
                mix.place('drums', A.crash(1.2), tq, 0.08)
            elif name == 'FLUTTER TONGUE':
                mix.place('flute', flutter(flute(91, 0.5, 0.9), 26), tq, 0.20, pan=0.22)
            elif name == 'SNAKE DANCE':                           # a rattle of hisses as the cobras leave the baskets
                for j in range(4):
                    n = int(0.12 * SR)
                    tt_ = A.tt(n)
                    hs = A.bp(np.random.default_rng(70 + j).normal(0, 1, n), 4000, 11000) * np.exp(-tt_ / 0.05)
                    mix.place('flute', hs, tq + j * S16, 0.09, pan=0.3 * (j % 2 * 2 - 1))
            elif name == 'PICCOLO':
                mix.place('flute', shriek(1.7), tq, 0.15, pan=0.2)
                mix.place('flute', colour(flute(93, 0.9, 1.0), KF, t, 2.0)[0], tq, 0.18)
            elif name == 'SUBSONIC':
                tele = RD.CFG[who]['abilities']['low']['tele'] * T.BEAT
                mix.place('tuba', sub_drone(tele + 0.05), tq, 0.40)
        elif kind == 'slam':                                      # SUBSONIC lands: a ring-shaking low note
            ts = e[0]
            mix.place('tuba', tuba(29, 2.5, 1.0), ts, 0.70)
            mix.place('tuba', sub_drone(1.2, 26, 30), ts, 0.30)
            mix.place('fx', A.thud(0.9, 90, 28), ts, 0.65)
            mix.place('drums', bass_drum(1.0), ts, 0.6)
    # ------------------------------------------------ the K.O. moment
    mix.place('drums', A.crash(2.4), T_KO, 0.16)
    mix.place('tuba', tuba(31, 2.5, 1.0), T_KO, 0.5)
    mix.place('fx', A.thud(0.9, 120, 30), T_KO, 0.7)
    loser = NAMES[LOSE]
    if loser == 'FLUTE':
        for j, m in enumerate((93, 89, 84, 80, 77, 74)):                  # the flute falls apart: a broken run down
            mix.place('flute', flute(m, 0.08, 0.8), RD.T_HS + 0.09 * j, 0.20, pan=(j - 2.5) / 6)
            mix.place('flute', breath(0.08, 40 + j), RD.T_HS + 0.09 * j, 0.06)
        mix.place('fx', A.chime(103, 0.6) + A.chime(98, 0.6) * 0.5, RD.T_HS, 0.08)
    else:
        for j, m in enumerate((43, 41, 38, 36, 34, 31)):                  # the tuba collapses: a slide down + clanks
            mix.place('tuba', tuba(m, 0.4, 0.8), RD.T_HS + 0.1 * j, 0.40)
            mix.place('fx', A.chime(70 + j * 3, 0.3), RD.T_HS + 0.1 * j, 0.04)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.16)
    # the winner's stinger on "(WINNER) WINS"
    tw = q16(T_KO + 1.3)
    if NAMES[WIN] == 'TUBA':
        mix.place('solo', tuba(36, 2.0, 1.0), tw, 0.6)
        mix.place('solo', tuba(43, 2.0, 0.9), tw, 0.35)
    else:
        for j, m in enumerate((84, 87, 91, 96)):
            mix.place('solo', flute(m, 0.12 if j < 3 else 0.8, 1.0), tw + j * S16 / 2, 0.32)
    mix.place('brass', brass((48, 55, 60, 63, 67), 1.2), tw, 0.24)
    mix.place('drums', bass_drum(1.0), tw, 0.5)
    mix.place('drums', A.crash(2.0), tw, 0.12)
    solo0 = RD.T_SOLO
    mix.place('fx', A.whoosh(max(0.3, solo0 - tw - 0.1), 300, 5000), tw + 0.1, 0.10)
    # ------------------------------------------------ victory: a lighter march + the winner's solo
    b_solo = int(round(solo0 / T.BAR))
    for b in range(b_solo, 31):
        bed(mix, b, light=b < 30)                          # bar 30 back to the full march under the reassembly
    wname = NAMES[WIN]
    for bt, m, d in T.SOLO[wname]:
        ts = solo0 + bt * T.BEAT
        if ts > T.T_REASSEMBLE + 0.2:
            break
        if wname == 'TUBA':
            mix.place('solo', tuba(m, d, 0.9), ts, 0.62)
        else:
            mix.place('solo', flute(m, d * T.BEAT * 0.9, 0.9), ts, 0.62)
    # the loser reassembles: its pieces fly home as a rising run, a glass "pop" when it's whole
    run = [60, 63, 67, 72, 75, 79, 84, 87]
    for j, m in enumerate(run):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
        if loser == 'FLUTE':
            mix.place('flute', flute(m + 12, 0.08, 0.7), tj, 0.16)
        else:
            mix.place('tuba', tuba(m - 24, 0.4, 0.7), tj, 0.32)
    mix.place('fx', A.chime(95, 0.5) + A.chime(102, 0.5) * 0.5, T.T_GLIDE, 0.12)
    # ------------------------------------------------ bar 31: build into the "3" on sample 0
    t31 = 31 * T.BAR
    mix.place('drums', roll(T.DUR - t31 + 0.03, 0.15, S16 / 2), t31, 0.30)
    mix.place('fx', riser(T.DUR - t31), t31, 0.22)
    nc = int((T.T_CLOSE + 0.012) * SR)                              # v3: the doors sliding shut into the title slam
    tc_ = A.tt(nc)
    doors = A.bp(np.random.default_rng(93).normal(0, 1, nc), 400, 5000) * (tc_ / T.T_CLOSE) ** 2
    mix.place('fx', doors, T.DUR - T.T_CLOSE, 0.55)
    for bt in range(4):
        mix.place('tuba', tuba((31, 35, 38, 41)[bt], 0.9, 0.85), t31 + bt * T.BEAT, 0.50)
        mix.place('brass', brass(VOICING['G'], 0.18), t31 + bt * T.BEAT + T.BEAT / 2, 0.18)
    mix.place('pad', horn_pad([43, 55, 59, 62, 65], T.DUR - t31 + 0.02), t31, 0.08)
    for st in (0, 8, 12, 14):
        mix.place('drums', bass_drum(0.9), t31 + st * S16, 0.42)
    # caption landings: soft thuds
    for tl, k, nch in impact_landings(RD.CAPTIONS):
        mix.place('fx', A.thud(), tl, 0.08 if k == 0 else 0.06)
    out, stems = master(mix)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    for k, s in stems.items():
        A.write(os.path.join(HERE, 'check', f'stem_{k}.wav'), s)
    spans = [(0, T.BAR), (T.BAR, 8), (8, 20), (20, 32), (32, 44), (44, T_KO), (T_KO, solo0), (solo0, 58), (58, 61)]
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


BUS_GAIN = {'drums': 0.9, 'tuba': 1.9, 'flute': 1.8, 'solo': 1.6, 'fx': 1.5, 'brass': 3.0, 'pad': 2.0, 'musicbox': 1.2}
# section automation (dB), applied AFTER the compressor so the limiter can't flatten the build: the count-in at full,
# the early fight pulled back, rising through the final exchange, the victory section loud, the end close to the start
GAIN_PTS = [(0, 0.6), (1.9, 0.3), (3.6, 0.0), (4.1, -1.9), (20, -1.7), (30, -0.6), (40, 1.3), (44, 2.3), (50.8, 3.3), (52.3, 2.8),
            (53.4, 3.3), (58.0, 3.3), (60.0, 1.6), (61.02, 0.6)]


def master(mix, thr=0.22, ceiling=0.89):
    """Mix.master with the overflow past the loop (tails, reverb) folded onto the start; NA samples (whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('flute', 0.5), ('solo', 0.5), ('fx', 0.4), ('brass', 0.4),
                                                ('pad', 0.5), ('musicbox', 0.7), ('drums', 0.15), ('tuba', 0.1)))
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
