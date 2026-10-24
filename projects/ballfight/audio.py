"""#27 BALLFIGHT #1 - soundtrack (mood A, drift phonk), all synthesised, original.

The fight is the duet: every landed hit in the honest run plays the attacker's own Pixel Pitch voice, snapped to the
nearest 16th (<= 58 ms):
  GUITAR  neck hit = a power-chord stab on the bar's root; its spinning neck chugs a palm-muted note every half turn
          (SHRED doubles the spin, so the chugs double to 8ths); a parried key rings a high harmonic
  PIANO   each key that lands plays one note, climbing the scale by its place in the volley (keys landing in the same 16th
          stack into a chord); every volley launches with a soft arpeggio of as many notes as it has keys, so you hear
          the volley grow
You hear who is winning: the leader's part is louder and gets an octave double; a fighter's part thins (high-pass) and
distorts as its own HP drops. Abilities have their own sounds (POWER CHORD stab + sub boom, SUSTAIN shimmer chord,
SHRED run, GLISSANDO = the arc's 15 keys as a real glissando, FEEDBACK screech, GRAND SLAM low cluster + boom).
Bed: E minor drift phonk at 129.84 BPM (33 bars = 61 s): pitched 808 cowbell riff, sliding distorted 808, hard kick,
clap, 16th hats; the count-in bar (guitar chord + piano octave + 808 on 3, 2, 1) drops on FIGHT!; at the K.O. the bed
tape-stops with the hit-stop, the crowd roars, the winner's stinger lands, then its solo over a lighter groove; bar 32
builds (roll, riser, 808 slide) into the "3" on sample 0. Tails past the loop fold onto the start.

    python audio.py            -> audio.wav (whole AAC frames) + stems in check/ for the band check"""
import math
import os
import sys
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', '..'))
sys.path.insert(0, HERE)
sys.path.append(os.path.join(HERE, '..', 'which_instrument_wins'))
from reelkit import audio as A
from reelkit.ui import impact_landings
import timeline as T
import render as RD
import pp_bridge as B

SR = A.SR
RUN, EV = RD.RUN, RD.EV
NAMES, WIN, LOSE, T_KO = RD.NAMES, RD.WIN, RD.LOSE, RD.T_KO
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames (2860 = 61.013 s): loops sample-perfectly
S16 = T.S16


def q16(t):
    return round(t / S16) * S16


# ---------------------------------------------------------------- instrument voices (cached, level-matched)
_VC, _GAIN = {}, {}


def _fit(x, cap):
    n = int(cap * SR)
    x = np.asarray(x, float)[:n].copy()
    if len(x) < n:
        x = np.pad(x, (0, n - len(x)))
    fd = int(0.05 * SR)
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


def hp_at(k, t):
    return RD.hp_at(k, t)


def colour(x, k, t, full=True):
    """the fighter's tone at time t: thinner + dirtier as its HP drops; returns (signal, gain)"""
    hp = hp_at(k, t) / 100.0
    lead = (hp_at(k, t) - hp_at(1 - k, t)) / 100.0
    cut = 90 + 650 * (1 - hp) ** 1.6
    y = A.hp(x, cut)
    drive = 1 + 5.5 * (1 - hp) ** 2
    y = np.tanh(drive * y * 1.2) / np.tanh(drive * 1.2) * (1.0 / (1 + 0.15 * (drive - 1)))
    g = 10 ** (float(np.clip(7.0 * lead, -4.5, 4.0)) / 20)
    return y, g, lead


SCALE_PC = [4, 6, 7, 9, 11, 0, 2]                       # E natural minor


def piano_note(j, t):
    """the j-th key of a volley: up the scale from E4 (over a B chord the D is raised to D#)"""
    ch = T.chord_at(t)
    pcs = [p if not (ch == 'B' and p == 2) else 3 for p in SCALE_PC]
    out, m = [], 64
    while len(out) < 24:
        if m % 12 in pcs:
            out.append(m)
        m += 1
    return out[min(len(out) - 1, j)]


def power_chord(t):
    r = {'E': 52, 'C': 48, 'B': 47}.get(T.chord_at(t), 52)
    return (r, r + 7, r + 12)


# ---------------------------------------------------------------- bed synths (original)
def cowbell(m, dur=0.16):
    n = int(dur * SR)
    f = A.mtof(m)
    x = A.square(f, n, 0.5) * 0.6 + A.square(f * 1.48, n, 0.5) * 0.4
    x = A.bp(x, 350, 5200) * A.expenv(n, 0.075, 0.001)
    return np.tanh(x * 2.2) / np.tanh(2.2)


def b808(m0, m1, dur, slide=0.06):
    n = int((dur + 0.05) * SR)
    t = A.tt(n)
    ts = max(0.0, dur - slide)
    f = A.mtof(m0) * np.where(t < ts, 1.0, (A.mtof(m1) / A.mtof(m0)) ** np.clip((t - ts) / max(slide, 1e-3), 0, 1))
    ph = 2 * np.pi * np.cumsum(f) / SR
    x = np.sin(ph) + 0.12 * np.sin(2 * ph)
    env = np.clip(t / 0.003, 0, 1) * np.exp(-t / max(0.35, dur * 0.9))
    env[-int(0.05 * SR):] *= np.linspace(1, 0, int(0.05 * SR))
    x = np.tanh(2.4 * x * env) / np.tanh(2.4)
    return A.lp(x, 1400)


def kick(punch=1.0):
    x = A.kick(punch, 0.22, 170, 48)
    return np.tanh(x * 1.6) / np.tanh(1.6)


def roll(dur, f0=0.3, step=None):
    step = step or S16 / 2
    n = int(dur * SR)
    out = np.zeros(n + SR)
    t = 0.0
    while t < dur - 1e-4:
        a = int(t * SR)
        s = A.snare(0.05) * (f0 + (1 - f0) * (t / dur) ** 1.5)
        out[a:a + len(s)] += s
        t += step
    return out[:n]


def riser(dur, over=0.012, seed=11):
    """a noise + saw riser that builds right up to (and `over` s past) its end: no fade, so the loop seam stays loud"""
    n = int((dur + over) * SR)
    t = A.tt(n)
    u = np.clip(t / dur, 0, 1.05)
    x = np.random.default_rng(seed).normal(0, 1, n)
    f = 300 * (20 ** u)
    sweep = A.saw(1, n) * 0
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


def screech(dur=1.6):
    n = int(dur * SR)
    t = A.tt(n)
    f = 1900 + 900 * np.sin(2 * np.pi * 7 * t) * np.exp(-t / 0.8) + 700 * t
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) + 0.4 * np.sin(2 * np.pi * np.cumsum(f * 1.5) / SR)
    x = np.tanh(3.5 * x) * np.clip(t / 0.05, 0, 1) * np.exp(-t / (dur * 0.5))
    return A.bp(x, 600, 7000)


def shimmer(ms, dur):
    n = int(dur * SR)
    t = A.tt(n)
    x = sum(np.sin(2 * np.pi * A.mtof(m) * t) * (0.7 + 0.3 * np.sin(2 * np.pi * (5 + j) * t)) for j, m in enumerate(ms)) / len(ms)
    return x * A.adsr(n, 0.25, 0.3, 0.7, 0.6)


CB = {0: [76, None, 76, 79, None, 76, 77, None, 76, None, 83, None, 81, 79, 77, None],
      1: [76, None, 76, 79, None, 76, 77, None, 84, None, 83, None, 79, 77, 76, 74]}
BASS_ROOT = {'E': 28, 'C': 36, 'B': 35}


def bed(mix, b, light=False, intensity=1.0):
    """one bar of the groove (bar index b)"""
    t0 = b * T.BAR
    ch = T.CHORDS[b]
    nxt = T.CHORDS[(b + 1) % T.N_BARS]
    # drums: kick on 1, the "and" of 2, 3; clap on 2 and 4; 16th hats (open on the offbeat 8ths later in the fight)
    for st in (0, 6, 8, 11 if b % 2 else 14):
        mix.place('drums', kick(), t0 + st * S16, 0.62 if st in (0, 8) else 0.42)
    for st in (4, 12):
        mix.place('drums', A.clap(), t0 + st * S16, 0.42)
        mix.place('drums', A.snare(0.09), t0 + st * S16, 0.16)
    for st in range(16):
        g = 0.07 if st % 2 else 0.10
        if not light and intensity > 1.2 and st % 4 == 2:
            mix.place('drums', A.hat(open_=True), t0 + st * S16, 0.05 * intensity)
        mix.place('drums', A.hat(0.025), t0 + st * S16, g * (0.7 if light else 1.0), pan=0.25 if st % 2 else -0.15)
    # 808: root held, slides to the next bar's root on the last 16th
    r0, r1 = BASS_ROOT[ch], BASS_ROOT[nxt]
    mix.place('bass', b808(r0, r0 + 12 if b % 4 == 3 else r0, 6 * S16, 0.08), t0, 0.55)
    mix.place('bass', b808(r0, r1, 10 * S16, 0.12), t0 + 6 * S16, 0.48)
    # cowbell riff (2 bars), an octave layer later in the fight
    pat = CB[b % 2]
    for st, m in enumerate(pat):
        if m is None:
            continue
        if ch == 'B' and m == 74:
            m = 75
        mix.place('cowbell', cowbell(m), t0 + st * S16, (0.10 if light else 0.15), pan=0.15)
        if intensity > 1.35 and not light:
            mix.place('cowbell', cowbell(m + 12, 0.1), t0 + st * S16, 0.05, pan=-0.3)
    # dark pad under it (from bar 5)
    if b >= 5 or light:
        root = {'E': 52, 'C': 48, 'B': 47}[ch]
        tones = [root, root + (3 if ch == 'E' else 4), root + 7]
        mix.place('pad', A.pad(tones, T.BAR, cut=900, att=0.4), t0, 0.07)


def main():
    mix = A.Mix(T.DUR + 4.0)
    # ------------------------------------------------ count-in (bar 0): 3, 2, 1 = guitar chord + piano octave + 808
    for j, (tc, txt) in enumerate(T.COUNT[:3]):
        tc = max(tc, 0.004)                                # the "3" 4 ms into the file: no click at the seam
        for m in power_chord(tc):
            mix.place('guitar', note('GUITAR', m, 0.3, 0.7), tc, 0.30)
        for m in (52, 64):
            mix.place('piano', note('PIANO', m, 0.4, 0.9), tc, 0.34)
        mix.place('bass', b808(28, 28, 0.42), tc, 0.6)
        mix.place('drums', kick(1.1), tc, 0.6)
    mix.place('drums', roll(T.BEAT, 0.25, S16 / 2), 3 * T.BEAT, 0.28)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), 3 * T.BEAT, 0.18)
    mix.place('fx', A.crash(1.6), T.BAR, 0.32)
    # ------------------------------------------------ the fight's groove: bars 1 .. the K.O.
    bar_ko = int(T_KO // T.BAR)
    for b in range(1, bar_ko + 1):
        inten = 1.0 + 0.5 * (b / bar_ko)
        bed(mix, b, intensity=inten)
    # risers into the final exchange
    mix.place('fx', A.whoosh(3.6, 300, 6000), 44.0 - 3.6, 0.10)
    mix.place('drums', roll(2 * T.BAR, 0.1, S16), (bar_ko - 2) * T.BAR, 0.10)
    # tape-stop the bed with the K.O. hit-stop
    for k in ('drums', 'bass', 'cowbell', 'pad'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ the fighters
    k_g = 0 if NAMES[0] == 'GUITAR' else 1
    k_p = 1 - k_g
    piano_slots = {}
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'hit':
            att, vic, d, x, y, how, sub = e[2:9]
            if how == 'blade':
                tq = q16(t)
                sig = sum(note('GUITAR', m, 0.22, 0.6, 0.95) for m in power_chord(tq))
                sig, g, lead = colour(sig, att, t)
                pan = float(np.clip(x / 3.5, -0.6, 0.6))
                mix.place('guitar', sig, tq, 0.42 * g, pan=pan)
                if lead > 0.04:
                    mix.place('guitar', colour(note('GUITAR', power_chord(tq)[0] + 12, 0.2, 0.5), att, t)[0], tq, 0.18 * g, pan=-pan)
                mix.place('fx', A.thud(0.25, 140, 60), tq, 0.16)
            elif how == 'key':
                tq = q16(t)
                piano_slots.setdefault((att, round(tq / S16)), []).append((t, piano_note(int(sub), tq), x))
            elif how in ('shockwave', 'wave', 'slam'):
                mix.place('fx', A.thud(0.45, 110, 40), q16(t), 0.42)
        elif kind == 'parry':
            tq = q16(t)
            mix.place('fx', A.chime(100 + (int(e[6]) % 3) * 2, 0.12), tq, 0.035, pan=float(np.clip(e[4] / 3.5, -0.6, 0.6)))
        elif kind == 'block':
            mix.place('fx', A.chime(96, 0.6) + A.chime(103, 0.6) * 0.6, q16(t), 0.10)
        elif kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.07 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            mix.place('fx', A.chime(91, 0.25), t, 0.06)
        elif kind == 'fire':
            who, n = e[2], e[3]
            tq = q16(t)
            for j in range(n):                                  # the volley launches as a soft upward arpeggio
                m = piano_note(j, tq) - 12
                sig, g, _ = colour(note('PIANO', m, 0.15, 0.45, 0.5), who, t)
                mix.place('piano', sig, tq + j * min(S16 / 2, T.BEAT / max(n, 1) / 2), 0.07 * g)
        elif kind == 'ability':
            who, name = e[2], e[3]
            tq = q16(t)
            if name == 'POWER CHORD':
                sig = sum(note('GUITAR', m, 0.6, 1.2, 1.0) for m in power_chord(tq) + (power_chord(tq)[0] + 19,))
                mix.place('guitar', colour(sig, who, t)[0], tq, 0.42)
                mix.place('bass', b808(28, 24, 0.5, 0.3), tq, 0.35)
            elif name == 'SUSTAIN':
                ms = [64, 67, 71, 76]
                mix.place('piano', sum(note('PIANO', m, 1.5, 2.0, 0.6) for m in ms), tq, 0.16)
                mix.place('fx', shimmer([88, 91, 95], 2 * T.BAR), tq, 0.035)
            elif name == 'SHRED':
                run = [64, 66, 67, 69, 71, 72, 74, 76, 79, 76, 74, 72, 71, 69, 67, 66]
                for j, m in enumerate(run):
                    mix.place('guitar', colour(note('GUITAR', m, 0.1, 0.25, 0.9), who, t)[0], tq + j * S16, 0.2)
            elif name == 'GLISSANDO':
                n = 15
                for j in range(n):
                    tj = t + T.BEAT * j / (n - 1)
                    mix.place('piano', colour(note('PIANO', piano_note(j, tj), 0.12, 0.5, 0.75), who, t)[0], tj, 0.17)
            elif name == 'FEEDBACK':
                mix.place('fx', screech(1.8), tq, 0.10)
                sig = sum(note('GUITAR', m, 1.2, 1.8, 1.0) for m in power_chord(tq))
                mix.place('guitar', colour(sig, who, t)[0], tq, 0.30)
            elif name == 'GRAND SLAM':
                mix.place('piano', sum(note('PIANO', m, 0.8, 1.5, 1.0) for m in (28, 29, 33, 35)), tq, 0.22)
        elif kind == 'slam':
            mix.place('fx', A.thud(0.6, 90, 32), e[0], 0.55)
            mix.place('piano', sum(note('PIANO', m, 0.6, 1.4, 1.0) for m in (28, 31, 35, 40)), q16(e[0]), 0.28)
            mix.place('fx', A.crash(1.2), e[0], 0.12)
    # piano key hits: one note per key, keys in the same 16th stack into a chord
    for (att, slot), lst in piano_slots.items():
        tq = slot * S16
        ms = sorted(set(m for _, m, _ in lst))[:5]
        x = float(np.mean([v for _, _, v in lst]))
        sig = sum(note('PIANO', m, 0.16, 0.55, 0.9) for m in ms) / max(1, len(ms)) ** 0.4
        sig, g, lead = colour(sig, att, lst[0][0])
        mix.place('piano', sig, tq, 0.36 * g, pan=float(np.clip(x / 3.5, -0.6, 0.6)))
        if lead > 0.04:
            mix.place('piano', note('PIANO', ms[-1] + 12, 0.12, 0.4, 0.7), tq, 0.10 * g)
    # the guitar's spinning neck: a palm-muted chug every half turn (double with SHRED)
    for b in range(1, int(T_KO / T.BAR) + 1):
        for st in range(16):
            tq = b * T.BAR + st * S16
            if tq >= T_KO:
                break
            i = RD.sim_i(tq)
            haste = RUN['HA'][i, k_g] > 0
            if (st % 4 == 0) or (haste and st % 2 == 0):
                m = power_chord(tq)[0] - 12 if power_chord(tq)[0] - 12 >= 40 else power_chord(tq)[0]
                sig = A.lp(note('GUITAR', m, 0.07, 0.16, 0.6), 1800)
                sig, g, _ = colour(sig, k_g, tq)
                mix.place('guitar', sig, tq, (0.14 if st % 4 == 0 else 0.09) * g, pan=-0.2)
    # ------------------------------------------------ the K.O. moment
    mix.place('fx', A.crash(2.4), T_KO, 0.42)
    mix.place('bass', b808(31, 19, 1.4, 1.2), T_KO, 0.6)
    mix.place('fx', A.thud(0.9, 120, 30), T_KO, 0.7)
    loser = NAMES[LOSE]
    if loser == 'GUITAR':
        for j, m in enumerate((64, 63, 59, 58, 52, 47)):                  # strings snapping, falling apart
            mix.place('guitar', A.hp(note('GUITAR', m, 0.25, 0.6, 0.9), 200), RD.T_HS + 0.09 * j, 0.16, pan=(j - 2.5) / 6)
    else:
        for j, m in enumerate((84, 79, 76, 71, 67, 64, 60, 55)):
            mix.place('piano', note('PIANO', m, 0.2, 0.6, 0.8), RD.T_HS + 0.07 * j, 0.14, pan=(j - 3.5) / 8)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.16)
    # the winner's stinger on "(WINNER) WINS"
    tw = q16(T_KO + 1.3)
    if NAMES[WIN] == 'PIANO':
        mix.place('solo', sum(note('PIANO', m, 1.6, 2.4, 0.95) for m in (40, 52, 59, 64, 67, 71)), tw, 0.22)
    else:
        mix.place('solo', sum(note('GUITAR', m, 1.6, 2.4, 1.0) for m in (52, 59, 64, 67)), tw, 0.24)
    mix.place('fx', A.crash(1.8), tw, 0.18)
    solo0 = RD.T_SOLO
    mix.place('fx', A.whoosh(solo0 - tw - 0.1, 300, 5000), tw + 0.1, 0.10)
    # ------------------------------------------------ victory: lighter groove + the winner's solo
    b_solo = int(round(solo0 / T.BAR))
    for b in range(b_solo, 32):
        bed(mix, b, light=True)
    wname = NAMES[WIN]
    lo, hi = B.entrant(wname)[3]
    for bt, m, d in T.SOLO[wname]:
        dur = d * T.BEAT * 0.95
        x = _fit(B.voice(wname, m, 0.9, dur), dur + 0.6) * inst_gain(wname)
        mix.place('solo', x, solo0 + bt * T.BEAT, 0.46)
    # the loser reassembles: its pieces fly home as a rising run, a glass "pop" when it's whole
    run = [52, 55, 59, 62, 64, 67, 71, 74] if loser == 'GUITAR' else [60, 64, 67, 71, 72, 76, 79, 84]
    for j, m in enumerate(run):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
        mix.place('guitar' if loser == 'GUITAR' else 'piano', note(loser, m, 0.12, 0.4, 0.7), tj, 0.12)
    mix.place('fx', A.chime(95, 0.5) + A.chime(102, 0.5) * 0.5, T.T_GLIDE, 0.12)
    # ------------------------------------------------ bar 32: build into the "3" on sample 0
    t32 = 32 * T.BAR
    mix.place('drums', roll(T.DUR - t32 + 0.03, 0.15, S16 / 2), t32, 0.30)
    mix.place('fx', riser(T.DUR - t32), t32, 0.22)
    mix.place('bass', b808(28, 40, T.DUR - t32 - 0.02, 0.9), t32, 0.40)
    for st in (0, 4, 8, 10, 12, 14):
        mix.place('drums', kick(), t32 + st * S16, 0.5)
    # caption landings: soft thuds
    for tl, k, nch in impact_landings(RD.CAPTIONS):
        mix.place('fx', A.thud(), tl, 0.08 if k == 0 else 0.06)
    out, stems = master(mix)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    for k, s in stems.items():
        A.write(os.path.join(HERE, 'check', f'stem_{k}.wav'), s)
    spans = [(0, 1.85), (1.85, 8), (8, 20), (20, 32), (32, 44), (44, T_KO), (T_KO, solo0), (solo0, 58), (58, 61)]
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


BUS_GAIN = {'guitar': 2.2, 'piano': 2.0, 'solo': 2.0, 'fx': 1.5, 'bass': 0.62, 'drums': 0.75, 'cowbell': 0.72, 'pad': 0.8}
# section automation (dB), applied AFTER the compressor so the limiter can't flatten the build (#22 / #24 lesson): the
# count-in hits at full, the early fight pulled back, rising through the final exchange, the victory section loud, and
# the end close to the start so the seam has no level step
GAIN_PTS = [(0, 0.0), (1.7, 0.0), (2.2, -2.9), (20, -2.7), (30, -1.4), (40, 0.0), (44, 1.0), (51.2, 2.0), (52.5, 1.5),
            (53.6, 2.5), (58.0, 2.5), (60.0, 1.5), (61.02, 1.0)]


def master(mix, thr=0.22, ceiling=0.89):
    """Mix.master with the overflow past the loop (tails, reverb) folded onto the start; NA samples (whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('piano', 0.6), ('guitar', 0.35), ('solo', 0.7), ('fx', 0.4), ('cowbell', 0.3), ('pad', 0.5)))
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
    s1 = ceiling / (np.abs(m).max() + 1e-9)
    m = np.tanh(m * s1 * 1.1) / np.tanh(1.1)
    s2 = ceiling / (np.abs(m).max() + 1e-9)
    m *= s2
    stems = {}
    for k in buses:
        stems[k] = fold(buses[k]) * gain * s1 * s2
    return m, stems


if __name__ == '__main__':
    main()
