"""#29 BALLFIGHT #3: Trumpet vs Sax - soundtrack (noir jazz), all synthesised, original.

The two horns are the leads, both played by the honest run (Pixel Pitch voices):
  TRUMPET  every charge is a muted note swelling open over its beat; every blast is a bright stab on the beat after it:
           a landed blast climbs one step up the chord (so the trumpet's line rises as its beam grows), a miss falls off.
           DOUBLE TONGUE doubles the stabs (ta-ka), FANFARE is a rising call, HIGH C a screaming shake on top.
  SAX      every smoke-ring volley is a growled bluesy lick on the beat, one note per ring (up to four, the lower octave
           joins at five): the more rings, the longer the lick. Rings that land are breathy puffs; SMOKE SCREEN is a
           subtone held under it, BEBOP RUN a 32nd-note run, HONK a huge low honk on the slam.
  Volleys fire on beats 1 and 3 and the blasts on 2 and 4, so the fight plays itself as call and response. The leader's
  horn gets fuller (octave / third doubles) and louder; the loser's thins and dirties as its HP drops.
Bed: C minor at 133.77 BPM (34 bars = 61 s), i - bVI7 - iv - V7: walking upright bass, brushes (swirl, taps on 2 and 4,
swung ride) that switch to sticks for the final third, muted piano comping (Charleston, rootless voicings), a dark pad.
The count-in bar (brush taps + a G7 horn stab per count, a snare pickup) drops on FIGHT!; at the K.O. the bed
tape-stops with the hit-stop, the sax falls apart, the crowd roars, the trumpet's stinger lands, then its solo over a
lighter band; bar 33 builds (roll, riser, bass walk Ab-C-Eb-Ab) into the "3" on sample 0. Every hit is snapped to the
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
KT, KS = NAMES.index('TRUMPET'), NAMES.index('SAX')
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames: loops sample-perfectly
S16 = T.S16
SWING = 0.6                                             # light swing at this tempo: the off-beat 8th at 60 % of the beat


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


def tpt(m, dur=0.16, vel=0.9, cap=None):
    return note('TRUMPET', int(np.clip(m, 58, 86)), round(dur, 3), round(cap or dur + 0.22, 2), round(vel, 1))


def sax(m, dur=0.16, vel=0.85, cap=None):
    return note('SAX', int(np.clip(m, 54, 80)), round(dur, 3), round(cap or dur + 0.25, 2), round(vel, 1))


def growl(x, depth=0.35, rate=31.0, drive=1.6):
    """a sax growl: flutter in the amplitude + a little grit"""
    t = A.tt(len(x))
    y = x * (1 + depth * np.sin(2 * np.pi * rate * t))
    return np.tanh(drive * y) / np.tanh(drive) * 0.8


def breath(dur=0.08, lo=500, hi=2600, seed=2):
    n = int(dur * SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    return A.bp(x, lo, hi) * A.expenv(n, dur * 0.35, 0.006)


def swell(x, dur):
    """a muted note opening up over `dur`: a crescendo + the harmon mute easing off (lowpass -> open)"""
    n = len(x)
    t = A.tt(n)
    k = np.clip(t / max(1e-3, dur), 0, 1)
    dark = A.lp(x, 1100)
    y = dark * (1 - k) + x * k
    return y * (0.18 + 0.82 * k ** 1.6)


def hp_at(k, t):
    return RD.hp_at(k, t)


def colour(x, k, t, drive_k=1.0):
    """the fighter's tone at time t: thinner + dirtier as its HP drops; returns (signal, gain, lead)"""
    hp = hp_at(k, t) / 100.0
    lead = (hp_at(k, t) - hp_at(1 - k, t)) / 100.0
    cut = 90 + 520 * (1 - hp) ** 1.6
    y = A.hp(x, cut)
    drive = 1 + 4.0 * drive_k * (1 - hp) ** 2
    y = np.tanh(drive * y * 1.2) / np.tanh(drive * 1.2) * (1.0 / (1 + 0.15 * (drive - 1)))
    g = 10 ** (float(np.clip(6.0 * lead, -4.0, 3.5)) / 20)
    return y, g, lead


# ---------------------------------------------------------------- the band (original)
def upright(m, dur=0.5, vel=1.0):
    """upright bass pluck: round fundamental, woody thump, quick decay"""
    n = int((dur + 0.1) * SR)
    f = A.mtof(m)
    t = A.tt(n)
    x = np.sin(2 * np.pi * f * t) + 0.35 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.12) + 0.12 * np.sin(6 * np.pi * f * t) * np.exp(-t / 0.06)
    x *= np.exp(-t / max(0.18, dur * 0.7)) * np.clip(t / 0.003, 0, 1)
    x += 0.25 * A.bp(np.random.default_rng(int(m)).normal(0, 1, n), 300, 1600) * np.exp(-t / 0.008)
    fd = int(0.05 * SR)
    x[-fd:] *= np.linspace(1, 0, fd)
    return A.lp(np.tanh(1.6 * x) / np.tanh(1.6), 1500) * vel


_RIDE = {}


def ride(vel=1.0, bell=False):
    key = (round(vel, 1), bell)
    if key not in _RIDE:
        n = int(1.1 * SR)
        t = A.tt(n)
        parts = (520, 1173, 2406, 3605, 5110, 6893) if not bell else (1220, 2950, 4420)
        x = sum(np.sin(2 * np.pi * f * t + i) * (0.9 ** i) for i, f in enumerate(parts)) / len(parts)
        x = x * np.exp(-t / (0.55 if not bell else 0.35))
        x += 0.5 * A.bp(np.random.default_rng(31).normal(0, 1, n), 5000, 14000) * np.exp(-t / 0.18)
        x += 0.6 * A.bp(np.random.default_rng(32).normal(0, 1, n), 3000, 9000) * np.exp(-t / 0.006)
        _RIDE[key] = x * vel
    return _RIDE[key]


def brush_tap(vel=1.0, seed=0):
    n = int(0.22 * SR)
    t = A.tt(n)
    x = A.bp(np.random.default_rng(40 + seed).normal(0, 1, n), 1600, 7500) * np.clip(t / 0.006, 0, 1) * np.exp(-t / 0.07)
    return x * vel


def swirl(dur, seed=0):
    """the brush circling the snare head: soft noise sweeping once per beat"""
    n = int(dur * SR)
    t = A.tt(n)
    x = A.bp(np.random.default_rng(50 + seed).normal(0, 1, n), 2500, 9000)
    ph = (t / T.BEAT) % 1.0
    return x * (0.35 + 0.65 * np.sin(np.pi * ph) ** 2)


def stick_snare(vel=1.0):
    return A.snare(0.09) * vel


def piano_chord(ms, dur=0.22, vel=0.7):
    x = sum(note('PIANO', m, round(dur, 3), round(dur + 0.25, 2), round(vel, 1)) for m in ms) / max(1, len(ms)) * 1.6
    return A.lp(x, 2400)


def dark_pad(ms, dur, cut=900):
    n = int(dur * SR)
    t = A.tt(n)
    x = np.zeros(n)
    for m in ms:
        f = A.mtof(m)
        for d in (-0.004, 0.004):
            ph = (f * (1 + d) * t) % 1.0
            x += 2 * ph - 1
    x /= 2 * max(1, len(ms))
    return A.lp(x, cut) * A.adsr(n, 0.3, 0.2, 0.85, 0.3)


def roll(dur, f0=0.3, step=None):
    step = step or S16 / 2
    n = int(dur * SR)
    out = np.zeros(n + SR)
    t = 0.0
    while t < dur - 1e-4:
        a = int(t * SR)
        s = brush_tap(1.0, int(t * 100) % 7) * 0.6
        s = s * (f0 + (1 - f0) * (t / dur) ** 1.5)
        out[a:a + len(s)] += s[:len(out) - a]
        t += step
    return out[:n]


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


def roar(dur, seed=3):
    n = int(dur * SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    x = A.bp(x, 300, 2600) * 0.5 + A.bp(x, 900, 5000) * 0.25
    t = A.tt(n)
    env = np.clip(t / 0.25, 0, 1) * np.exp(-np.maximum(t - 0.6, 0) / (dur * 0.45))
    return x * env


def key_clicks(n=6, seed=8):
    """sax pads clacking: short woody clicks"""
    out = np.zeros(int(0.5 * SR))
    rng = np.random.default_rng(seed)
    for j in range(n):
        a = int(rng.uniform(0, 0.4) * SR)
        m = int(0.02 * SR)
        c = A.bp(rng.normal(0, 1, m), 900, 4000) * np.exp(-A.tt(m) / 0.004)
        out[a:a + m] += c
    return out


# ---------------------------------------------------------------- harmony
BASS_ROOT = {'Cm': 36, 'Ab7': 32, 'Fm': 41, 'G7': 43}
CHORD_TONES_LO = {'Cm': (36, 39, 43, 46), 'Ab7': (32, 36, 39, 42), 'Fm': (41, 44, 36, 39), 'G7': (43, 47, 38, 41)}
VOICE = {'Cm': (63, 67, 70, 74), 'Ab7': (60, 63, 66, 70), 'Fm': (56, 60, 63, 67), 'G7': (65, 68, 71, 75)}
PAD = {'Cm': (48, 55, 63), 'Ab7': (44, 51, 60), 'Fm': (41, 48, 56), 'G7': (43, 50, 59)}


def walk(b):
    """four walking quarter notes for bar b: the root, two chord tones, a chromatic approach to the next root"""
    ch, nxt = T.CHORDS[b % T.N_BARS], T.CHORDS[(b + 1) % T.N_BARS]
    r = BASS_ROOT[ch]
    tones = CHORD_TONES_LO[ch]
    up = h01(b, 1) < 0.5
    m2 = tones[1] if up else tones[3] - 12 if tones[3] - 12 >= 28 else tones[3]
    m3 = tones[2]
    tgt = BASS_ROOT[nxt]
    m4 = tgt + (1 if h01(b, 2) < 0.5 else -1)
    return [r, m2, m3, m4]


def bed(mix, b, intensity=1.0, light=False, sticks=False):
    """one bar of the band (bar index b)"""
    t0 = b * T.BAR
    ch = T.CHORDS[b % T.N_BARS]
    for j, m in enumerate(walk(b)):
        mix.place('bass', upright(m, 0.40, 0.95 if j else 1.0), t0 + j * T.BEAT, 0.60)
    # drums
    if not sticks:
        mix.place('drums', swirl(T.BAR, b), t0, 0.030)
        for bt in (1, 3):
            mix.place('drums', brush_tap(0.9, b + bt), t0 + bt * T.BEAT, 0.20)
        for bt, v in ((0, 0.8), (1, 0.7), (1 + SWING, 0.45), (2, 0.8), (3, 0.7), (3 + SWING, 0.45)):
            mix.place('drums', ride(v), t0 + bt * T.BEAT, 0.07, pan=0.35)
    else:
        for bt, v in ((0, 1.0), (1, 0.85), (1 + SWING, 0.6), (2, 1.0), (3, 0.85), (3 + SWING, 0.6)):
            mix.place('drums', ride(v), t0 + bt * T.BEAT, 0.10, pan=0.35)
        for bt in range(4):
            mix.place('drums', A.kick(0.5, 0.2), t0 + bt * T.BEAT, 0.10)          # feathered kick
        for bt in (1, 3):
            mix.place('drums', A.hat(0.03), t0 + bt * T.BEAT, 0.10, pan=-0.3)     # hi-hat foot
        for bt in (1 + SWING, 2 + SWING, 3 + SWING):                              # snare comping
            if h01(b, bt) < 0.45:
                mix.place('drums', stick_snare(0.5), t0 + bt * T.BEAT, 0.10, pan=-0.1)
        if b % 4 == 0:
            mix.place('drums', A.crash(1.6), t0, 0.07, pan=0.4)
    # piano comping (Charleston: 1 and the & of 2; more stabs as it builds)
    vo = VOICE[ch]
    hits = [(0, 0.26), (1 + SWING, 0.18)]
    if intensity > 1.3 and not light:
        hits.append((3 + SWING, 0.14))
    for bt, d in hits:
        mix.place('piano', piano_chord(vo, d, 0.6), t0 + bt * T.BEAT, 0.20 if light else 0.24, pan=-0.2)
    if b >= 2 or light:
        mix.place('pad', dark_pad(PAD[ch], T.BAR + 0.05), t0, 0.05)


# ---------------------------------------------------------------- the trumpet: charges + blasts
def climb_pitch(t, n):
    ms = T.scale_notes(67, 86, T.chord_at(t))
    return ms[n % min(8, len(ms))]


def trumpet_part(mix):
    landed = {}
    for e in EV:
        if e[1] == 'hit' and e[2] == KT and e[7] == 'beam':
            landed[round(e[0], 3)] = True
    hit_t = np.array(sorted(landed)) if landed else np.zeros(0)
    n_landed = 0
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'charge' and e[2] == KT:
            tq, tc = q16(t), e[4]
            m = climb_pitch(tq, n_landed) - 5
            m = max(60, m)
            x = swell(tpt(m, tc * 0.95, 0.6, tc + 0.1), tc)
            sig, g, lead = colour(x, KT, t, 0.6)
            mix.place('trumpet', sig, tq, 0.13 * g, pan=-0.25)
        elif kind == 'beam' and e[2] == KT:
            tq = q16(t)
            hit = len(hit_t) and np.any((hit_t >= t - 1e-6) & (hit_t <= t + 0.16))
            m = climb_pitch(tq, n_landed)
            hp = hp_at(KT, t) / 100.0
            if hp < 0.5 and h01(t, 4) < 0.5 * (1 - hp) ** 1.5:          # thinning when hurt: some stabs crack
                mix.place('trumpet', A.hp(breath(0.06, 1500, 6000, int(t * 10)), 900), tq, 0.08, pan=-0.25)
                continue
            if hit:
                n_landed += 1
                x = tpt(m, 0.14, 1.0)
                sig, g, lead = colour(x, KT, t, 0.8)
                mix.place('trumpet', sig, tq, 0.40 * g, pan=-0.25)
                if lead > 0.04:                                          # the leader: the octave below joins
                    mix.place('trumpet', colour(tpt(m - 12, 0.14, 0.8), KT, t)[0], tq, 0.16 * g, pan=-0.25)
                mix.place('fx', A.thud(0.25, 140, 60, seed=int(t * 9)), tq, 0.10, pan=0.2)
            else:                                                        # a miss: the stab falls off
                sig, g, _ = colour(tpt(m, 0.10, 0.85), KT, t, 0.8)
                mix.place('trumpet', sig, tq, 0.26 * g, pan=-0.25)
                ms = T.scale_notes(m - 7, m - 1, T.chord_at(tq))
                for j, mm in enumerate(reversed(ms[-2:])):
                    mix.place('trumpet', colour(tpt(mm, 0.05, 0.6), KT, t)[0], tq + (j + 1) * S16 / 2 + S16 / 2, 0.12 * g,
                              pan=-0.25)
            if float(RUN['HA'][RD.sim_i(t), KT]) > 0:                      # DOUBLE TONGUE: ta-ka
                mix.place('trumpet', colour(tpt(m, 0.07, 0.8), KT, t)[0], tq + S16, 0.24, pan=-0.25)


# ---------------------------------------------------------------- the sax: volleys of smoke rings
BLUES_SET = set((T.KEY_ROOT + p) % 12 for p in T.BLUES)
BLUES = [m for m in range(55, 80) if m % 12 in BLUES_SET]


def sax_lick(v, n, t):
    """n notes (<= 4) from the blues scale on the 16ths, starting near a chord tone, alternately falling / rising"""
    ch = T.chord_at(t)
    start_pool = [m for m in T.scale_notes(62, 74, ch)]
    s = start_pool[int(h01(v, 7) * len(start_pool))]
    i0 = min(range(len(BLUES)), key=lambda i: abs(BLUES[i] - s))
    step = -1 if v % 2 == 0 else 1
    out = []
    for j in range(min(4, n)):
        i = int(np.clip(i0 + step * j, 0, len(BLUES) - 1))
        out.append(BLUES[i])
    return out


def sax_part(mix):
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'fire' and e[2] == KS:
            n, v = e[3], e[4]
            tq = q16(t)
            hp = hp_at(KS, t) / 100.0
            lick = sax_lick(v, n, tq)
            for j, m in enumerate(lick):
                if j and h01(v, j) < 0.6 * (1 - hp) ** 1.6:                # sparse when hurt
                    continue
                d = 0.10 if j < len(lick) - 1 else 0.22
                x = growl(sax(m, d, 0.85), 0.25 + 0.25 * (1 - hp))
                if j == 0:                                              # a bluesy scoop into the first note
                    mix.place('sax', colour(sax(m - 1, 0.03, 0.6), KS, t)[0], tq - S16 / 4, 0.12, pan=0.25)
                sig, g, lead = colour(x, KS, t, 1.0)
                mix.place('sax', sig, tq + j * S16, 0.30 * g, pan=0.25)
                if lead > 0.04:                                         # the leader: a third below
                    under = T.scale_notes(m - 4, m - 3, T.chord_at(tq))
                    if under:
                        mix.place('sax', colour(sax(under[0], d, 0.7), KS, t)[0], tq + j * S16, 0.12 * g, pan=0.25)
            if n >= 5:                                                  # five rings and up: the low octave growls under
                mix.place('sax', colour(growl(sax(lick[0] - 12, 0.3, 0.9), 0.4), KS, t)[0], tq, 0.18, pan=0.25)
            mix.place('sax', breath(0.07, 400, 2400, v), tq, 0.05, pan=0.25)
        elif kind == 'hit' and e[2] == KS and e[7] == 'key':            # a ring (or a bebop note) lands: a breathy puff
            tq = q16(t)
            mix.place('fx', breath(0.10, 300, 1800, int(t * 13)), tq, 0.10, pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
            mix.place('fx', A.thud(0.18, 110, 70, seed=int(t * 5)), tq, 0.05)
        elif kind == 'cleared' and e[2] == KS:                          # a ring pierced by a blast
            mix.place('fx', breath(0.06, 2500, 7000, int(t * 17)), t, 0.04)


# ---------------------------------------------------------------- abilities, ropes, clashes
def extras(mix):
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'block':
            mix.place('fx', A.lp(breath(0.25, 200, 1500, int(t * 3)), 900), q16(t), 0.12)
        elif kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.05 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            mix.place('drums', ride(0.8, bell=True), t, 0.06)
        elif kind == 'ability':
            who, name = e[2], e[3]
            tq = q16(t)
            if name == 'FANFARE':                                       # a rising call: C - Eb - G - C
                for j, m in enumerate((72, 75, 79, 84)):
                    d = 0.09 if j < 3 else 0.6
                    mix.place('trumpet', colour(tpt(m, d, 1.0), KT, t)[0], tq + j * S16, 0.36, pan=-0.25)
                mix.place('drums', A.crash(1.4), tq + 3 * S16, 0.10, pan=0.3)
                mix.place('fx', A.thud(0.4, 120, 45), tq, 0.25)
            elif name == 'DOUBLE TONGUE':                               # ta-ka-ta-ka on the fifth
                for j in range(8):
                    mix.place('trumpet', colour(tpt(79, 0.05, 0.7 + 0.3 * (j % 2 == 0)), KT, t)[0], tq + j * S16 / 2,
                              0.22, pan=-0.25)
            elif name == 'HIGH C':                                      # a screaming C6 with a shake
                dur = 1.6
                mix.place('trumpet', colour(tpt(84, dur, 1.0, dur + 0.3), KT, t, 1.5)[0], tq, 0.30, pan=-0.25)
                k = 0
                while k * S16 / 2 < dur:
                    if k % 2:
                        mix.place('trumpet', tpt(87, 0.05, 0.7), tq + k * S16 / 2, 0.10 * min(1.0, k / 6), pan=-0.25)
                    k += 1
                mix.place('fx', A.thud(0.6, 110, 38), tq + 0.3, 0.30)
            elif name == 'SMOKE SCREEN':                                # a subtone held under the smoke
                dur = RD.CFG[who]['abilities']['unlock1']['dur_beats'] * T.BEAT
                x = A.lp(sax(60, dur * 0.95, 0.5, dur + 0.2), 1400)
                n = len(x)
                air = A.bp(np.random.default_rng(6).normal(0, 1, n), 300, 2400) * A.adsr(n, 0.2, 0.2, 0.7, 0.3)
                mix.place('sax', colour(x, KS, t)[0] + 0.15 * air, tq, 0.16, pan=0.25)
            elif name == 'BEBOP RUN':                                   # the 8 notes fly out as a 32nd-note run
                run = [60, 62, 63, 64, 65, 67, 69, 70]
                for j, m in enumerate(run):
                    mix.place('sax', colour(sax(m + 12 if m + 12 <= 80 else m, 0.05, 0.85), KS, t)[0],
                              tq + j * T.BEAT / 7, 0.24, pan=0.25)
            elif name == 'HONK':                                        # the breath in, two beats before the honk
                n = int(2 * T.BEAT * SR)
                inhale = A.bp(np.random.default_rng(8).normal(0, 1, n), 500, 3000) * np.linspace(0, 1, n) ** 2
                mix.place('fx', inhale, tq, 0.06)
        elif kind == 'slam':                                            # HONK: the giant low honk on the slam
            ts = q16(e[0])
            x = growl(sax(56, 0.7, 1.0, 1.0), 0.6, 29, 3.0)
            mix.place('sax', x, ts, 0.42, pan=0.1)
            mix.place('sax', growl(sax(68, 0.6, 1.0, 0.9), 0.5, 29, 2.5), ts, 0.16, pan=0.1)
            mix.place('fx', A.thud(0.7, 90, 32), ts, 0.55)
            mix.place('drums', A.kick(1.0), ts, 0.30)
            mix.place('bass', upright(31, 0.8), ts, 0.5)


def main():
    mix = A.Mix(T.DUR + 4.0)
    # ------------------------------------------------ count-in (bar 0, on G7): brush taps + a horn stab per count
    for j, (tc, txt) in enumerate(T.COUNT[:3]):
        tc = max(tc, 0.004)                                # the "3" 4 ms into the file: no click at the seam
        mix.place('drums', brush_tap(1.0, j), tc, 0.30)
        mix.place('drums', ride(0.8), tc, 0.06, pan=0.35)
        mix.place('trumpet', tpt(71, 0.14, 0.9), tc, 0.30, pan=-0.25)
        mix.place('sax', growl(sax(65, 0.14, 0.85), 0.3), tc, 0.28, pan=0.25)
        mix.place('bass', upright(43, 0.4), tc, 0.62)
        mix.place('piano', piano_chord(VOICE['G7'], 0.14, 0.7), tc, 0.20, pan=-0.2)
    for st, v in ((12, 0.45), (13, 0.6), (14, 0.75), (15, 0.9)):   # the pickup into FIGHT!
        mix.place('drums', brush_tap(v, st), st * S16, 0.30)
        mix.place('drums', stick_snare(v * 0.6), st * S16, 0.14)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), 3 * T.BEAT, 0.14)
    mix.place('drums', A.crash(2.0), T.BAR, 0.16, pan=0.3)
    mix.place('drums', A.kick(1.0), T.BAR, 0.30)
    # ------------------------------------------------ the band: bars 1 .. the K.O.
    bar_ko = int(T_KO // T.BAR)
    for b in range(1, bar_ko + 1):
        bed(mix, b, intensity=1.0 + 0.6 * (b / bar_ko), sticks=b >= 22)
    mix.place('fx', A.whoosh(3.6, 300, 6000), 22 * T.BAR - 3.6, 0.08)
    for k in ('bass', 'drums', 'piano', 'pad'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ the horns (the fight)
    trumpet_part(mix)
    sax_part(mix)
    extras(mix)
    # ------------------------------------------------ the K.O. moment
    mix.place('drums', A.crash(2.4), T_KO, 0.22)
    mix.place('bass', upright(24, 1.4), T_KO, 0.62)
    mix.place('fx', A.thud(0.9, 120, 30), T_KO, 0.7)
    loser = NAMES[LOSE]
    if loser == 'SAX':                                     # the sax falls apart: a falling smear + its keys clattering
        for j, m in enumerate((74, 70, 67, 63, 60, 58, 56)):
            mix.place('sax', growl(sax(m, 0.06, 0.8), 0.5, 27, 2.0), RD.T_HS + 0.07 * j, 0.20 * (1 - j / 9), pan=0.2)
        mix.place('fx', key_clicks(9, 4), RD.T_HS, 0.16)
    else:                                                  # the trumpet cracks: a split note squealing down
        for j, m in enumerate((84, 80, 77, 72, 68, 65, 60)):
            mix.place('trumpet', tpt(m, 0.06, 0.8), RD.T_HS + 0.07 * j, 0.20 * (1 - j / 9), pan=-0.2)
        mix.place('fx', key_clicks(5, 5), RD.T_HS, 0.14)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.16)
    # the winner's stinger on "(WINNER) WINS"
    tw = q16(T_KO + 1.3)
    if NAMES[WIN] == 'TRUMPET':
        for m in (72, 79, 84):
            mix.place('solo', tpt(m, 0.5, 1.0, 0.9), tw, 0.20)
    else:
        for m in (60, 67, 75):
            mix.place('solo', growl(sax(m, 0.5, 1.0, 0.9), 0.3), tw, 0.20)
    mix.place('piano', piano_chord((60, 63, 67, 70, 74), 0.8, 0.8), tw, 0.24)
    mix.place('bass', upright(36, 0.9), tw, 0.6)
    mix.place('drums', A.crash(1.8), tw, 0.14, pan=0.3)
    solo0 = RD.T_SOLO
    mix.place('fx', A.whoosh(solo0 - tw - 0.1, 300, 5000), tw + 0.1, 0.10)
    # ------------------------------------------------ victory: a lighter band + the winner's solo
    b_solo = int(round(solo0 / T.BAR))
    for b in range(b_solo, 33):
        bed(mix, b, light=True)
    wname = NAMES[WIN]
    for bt, m, d in T.SOLO[wname]:
        dur = d * T.BEAT * 0.9
        x = tpt(m, dur, 0.95, dur + 0.3) if wname == 'TRUMPET' else growl(sax(m, dur, 0.9, dur + 0.3), 0.2)
        mix.place('solo', x, solo0 + bt * T.BEAT, 0.46, pan=0.0)
    # the loser reassembles: its pieces fly home as a rising run of soft staccato notes, a glass "pop" when it's whole
    run = [60, 63, 65, 67, 70, 72, 75, 79]
    for j, m in enumerate(run):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
        if loser == 'SAX':
            mix.place('sax', sax(m, 0.05, 0.6), tj, 0.12, pan=0.2)
        else:
            mix.place('trumpet', tpt(m, 0.05, 0.6), tj, 0.12, pan=-0.2)
        mix.place('fx', key_clicks(2, 20 + j), tj, 0.05)
    mix.place('fx', A.chime(95, 0.5) + A.chime(102, 0.5) * 0.5, T.T_GLIDE, 0.12)
    # ------------------------------------------------ bar 33 (Ab7): build into the "3" on sample 0
    t33 = 33 * T.BAR
    mix.place('drums', roll(T.DUR - t33 + 0.03, 0.15, S16 / 2), t33, 0.40)
    mix.place('fx', riser(T.DUR - t33), t33, 0.22)
    for bt, m in enumerate((32, 36, 39, 44)):
        mix.place('bass', upright(m, 0.36), t33 + bt * T.BEAT, 0.58)
    for bt in (0, 1 + SWING, 3 + SWING):
        mix.place('piano', piano_chord(VOICE['Ab7'], 0.18, 0.7), t33 + bt * T.BEAT, 0.22, pan=-0.2)
    mix.place('pad', dark_pad((44, 51, 60, 66), T.DUR - t33 + 0.02, 1200), t33, 0.07)
    for j, (m_t, m_s) in enumerate(((72, 63), (75, 66), (78, 68), (80, 70))):   # both horns climb into the count
        mix.place('trumpet', tpt(m_t, 0.10, 0.6 + 0.1 * j), t33 + (j + 0) * T.BEAT + T.BEAT / 2, 0.14 + 0.03 * j, pan=-0.25)
        mix.place('sax', growl(sax(m_s, 0.10, 0.6 + 0.1 * j), 0.3), t33 + (j + 0) * T.BEAT + T.BEAT / 2, 0.13 + 0.03 * j, pan=0.25)
    for st in (0, 8, 12, 14):
        mix.place('drums', A.kick(0.9), t33 + st * S16, 0.26)
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


BUS_GAIN = {'trumpet': 2.1, 'sax': 1.45, 'solo': 1.6, 'fx': 1.4, 'bass': 0.8, 'drums': 1.8, 'piano': 1.4, 'pad': 1.2}
# section automation (dB), applied AFTER the compressor so the limiter can't flatten the build: the count-in at full,
# the early fight pulled back, rising through the final exchange, the victory section loud, the end close to the start
GAIN_PTS = [(0, 0.0), (1.6, 0.0), (2.1, -1.6), (20, -1.5), (30, -0.6), (40, 1.3), (44, 2.3), (49.0, 3.3), (50.5, 2.8),
            (52.0, 3.3), (57.8, 3.3), (59.8, 1.6), (61.02, 0.6)]


def master(mix, thr=0.22, ceiling=0.89):
    """Mix.master with the overflow past the loop (tails, reverb) folded onto the start; NA samples (whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('trumpet', 0.5), ('sax', 0.55), ('solo', 0.55), ('fx', 0.4),
                                                ('piano', 0.45), ('pad', 0.5), ('drums', 0.2)))
    wet = A.reverb(send) * 0.22
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
