"""#28 BALLFIGHT #2: Drums vs Violin - soundtrack (mood B, dark Balkan fire), all synthesised, original.

The bed has NO drums of its own: the drum fighter is the kit and the violin is the lead, both from the honest run:
  DRUMS   every pulse is a drum: kick on beats 1 and 3, snare on 2 and 4 (the pulses fire on the beat grid, so the fight
          IS the groove); DRUM ROLL's 16th pulses are a snare roll; every landed pulse adds a tom fill; the leader adds
          off-beat hats. As its HP drops the beat thins: fills go (< 45 HP), then the snare (< 25 HP: only beat 4 is
          left, < 12 HP none), leaving a lonely kick.
  VIOLIN  the bow's turns play the melody: every quarter turn plays the line's note at that moment (snapped to the
          16th), so the slow early bow plays a few long strokes and the sped-up bow plays the tune as frantic
          spiccato; every landed bow hit is an accent climbing the scale; PIZZICATO plucks, TREMOLO shivers, SCREECH is
          the beginner's squeal. As its HP drops the line gets scratchy and sparse; the leader gets a double-stop.
Bed: D harmonic minor at 141.64 BPM (36 bars = 61 s): upright-bass oom-pah, musette accordion on the off-beats and a
pad, a cimbalom shimmer building through the back half. The count-in bar (sticks + a violin chop + the bass on the
dominant, a snare pickup) drops on FIGHT!; at the K.O. the bed tape-stops with the hit-stop, strings snap, the crowd
roars, the winner's stinger lands, then its solo over a lighter oom-pah; bar 35 builds (roll, riser, bass climb) into
the "3" on sample 0. Every hit is snapped to the nearest 16th. Tails past the loop fold onto the start.

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
KD, KV = NAMES.index('DRUMS'), NAMES.index('VIOLIN')
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames (2860 = 61.013 s): loops sample-perfectly
S16 = T.S16
PIECE = {'K': 36, 'S': 38, 'T1': 48, 'T2': 45, 'T3': 41, 'C': 49}


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


def drum(piece, vel=0.9):
    """a kit piece in the drum fighter's own Pixel Pitch voice"""
    m = PIECE[piece]
    cap = 1.8 if piece == 'C' else 0.45
    return note('DRUMS', m, 0.2, cap, round(0.5 + 0.5 * min(1.0, vel), 1)) * (0.55 + 0.45 * vel)


def bow_note(m, n16=2, vel=0.8):
    """a bowed violin note held for n16 sixteenths (the bow decides the length)"""
    dur = max(0.07, n16 * S16 * 0.92)
    return note('VIOLIN', m, round(dur, 3), round(dur + 0.25, 2), vel)


def hp_at(k, t):
    return RD.hp_at(k, t)


def colour(x, k, t, drive_k=1.0):
    """the fighter's tone at time t: thinner + dirtier as its HP drops; returns (signal, gain, lead)"""
    hp = hp_at(k, t) / 100.0
    lead = (hp_at(k, t) - hp_at(1 - k, t)) / 100.0
    cut = 90 + 520 * (1 - hp) ** 1.6
    y = A.hp(x, cut)
    drive = 1 + 4.5 * drive_k * (1 - hp) ** 2
    y = np.tanh(drive * y * 1.2) / np.tanh(drive * 1.2) * (1.0 / (1 + 0.15 * (drive - 1)))
    g = 10 ** (float(np.clip(6.0 * lead, -4.0, 3.5)) / 20)
    return y, g, lead


def scratch(dur, seed=5):
    """bow scratch: rosin noise, band-limited, gritty"""
    n = int(dur * SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    x = A.bp(x, 1800, 7000) * (1 + 0.8 * np.sign(np.sin(2 * np.pi * 37 * A.tt(n))))
    return x * A.expenv(n, dur * 0.4, 0.004)


def pizz(m, vel=0.8):
    """pizzicato: a plucked string with a little wooden body"""
    n = int(0.5 * SR)
    f = A.mtof(m)
    t = A.tt(n)
    x = (np.sin(2 * np.pi * f * t) + 0.5 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.05)
         + 0.25 * np.sin(6 * np.pi * f * t) * np.exp(-t / 0.03)) * np.exp(-t / 0.16)
    x += 0.15 * A.bp(np.random.default_rng(m).normal(0, 1, n), 800, 3000) * np.exp(-t / 0.01)
    return A.lp(x, 4500) * vel


def sticks(vel=1.0):
    """the drummer clicking the sticks together for the count"""
    n = int(0.09 * SR)
    t = A.tt(n)
    x = A.bp(np.random.default_rng(9).normal(0, 1, n), 1800, 6500) * np.exp(-t / 0.012)
    x += 0.6 * np.sin(2 * np.pi * 2350 * t) * np.exp(-t / 0.02) + 0.3 * np.sin(2 * np.pi * 3900 * t) * np.exp(-t / 0.01)
    return x * vel


# ---------------------------------------------------------------- bed synths (original)
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


def reed(ms, dur, att=0.01, rel=0.05, cut=2600, musette=0.006):
    """accordion reeds: pulse + saw per note, three reeds detuned (musette), a little bellows tremolo"""
    n = int(dur * SR)
    t = A.tt(n)
    x = np.zeros(n)
    for m in ms:
        f = A.mtof(m)
        for d in (-musette, 0.0, musette):
            ph = (f * (1 + d) * t) % 1.0
            x += 0.6 * np.where(ph < 0.32, 1.0, -0.45) + 0.4 * (2 * ph - 1)
    x /= 3 * max(1, len(ms))
    x *= 1 + 0.06 * np.sin(2 * np.pi * 5.3 * t)
    return A.lp(x, cut) * A.adsr(n, att, 0.08, 0.8, min(rel, dur * 0.4))


def cimbalom(m, vel=1.0):
    """hammered strings: bright attack, two slightly detuned strings, ringing"""
    n = int(0.7 * SR)
    f = A.mtof(m)
    t = A.tt(n)
    x = (np.sin(2 * np.pi * f * t) + np.sin(2 * np.pi * f * 1.004 * t)) * 0.5
    x += 0.4 * np.sin(2 * np.pi * 2 * f * t) * np.exp(-t / 0.08) + 0.2 * np.sin(2 * np.pi * 3.01 * f * t) * np.exp(-t / 0.05)
    return x * np.exp(-t / 0.22) * vel


def roll(dur, f0=0.3, step=None, piece='S'):
    step = step or S16 / 2
    n = int(dur * SR)
    out = np.zeros(n + SR)
    t = 0.0
    while t < dur - 1e-4:
        a = int(t * SR)
        d_ = drum(piece, round(0.4 + 0.6 * (t / dur), 1))
        sn = A.snare(0.05)
        s = d_ * 0.5
        s[:len(sn)] += sn[:len(s)] * 0.5
        s *= (f0 + (1 - f0) * (t / dur) ** 1.5)
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


def squeal(dur=1.8):
    """the beginner-violin squeal: a wobbling, scratchy whistle sliding up, with the bow grinding underneath"""
    n = int(dur * SR)
    t = A.tt(n)
    f = 2300 + 500 * np.sin(2 * np.pi * 6.5 * t) + 900 * t + 300 * np.sign(np.sin(2 * np.pi * 3.1 * t))
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) + 0.35 * np.sin(2 * np.pi * np.cumsum(f * 2.02) / SR)
    x = np.tanh(2.8 * x) * np.clip(t / 0.03, 0, 1) * np.exp(-t / (dur * 0.55))
    return A.bp(x, 700, 8000) + 0.5 * scratch(dur, 13)


BASS = {'Dm': (38, 33), 'Gm': (43, 38), 'A': (33, 40)}          # (root on 1, fifth on 3)
VOICING = {'Dm': (62, 65, 69), 'Gm': (55, 58, 62), 'A': (57, 61, 64, 67)}
CIMB = {'Dm': (74, 77, 81, 86), 'Gm': (74, 79, 82, 86), 'A': (73, 76, 79, 81)}


def bed(mix, b, intensity=1.0, light=False):
    """one bar of the drum-less bed (bar index b): oom-pah bass + accordion, the pad, the cimbalom shimmer"""
    t0 = b * T.BAR
    ch = T.CHORDS[b]
    nxt = T.CHORDS[(b + 1) % T.N_BARS]
    r, f5 = BASS[ch]
    # oom: bass on 1 and 3 (a chromatic pickup into the next bar later in the fight)
    mix.place('bass', upright(r, 0.42), t0, 0.62)
    mix.place('bass', upright(f5, 0.36, 0.9), t0 + 2 * T.BEAT, 0.52)
    if intensity > 1.25 and not light:
        tgt = BASS[nxt][0]
        mix.place('bass', upright(tgt - 1, 0.18, 0.7), t0 + 3.5 * T.BEAT, 0.40)
    # pah: accordion stabs on 2 and 4 (8th-note "pah-pah" when the fire builds)
    vo = VOICING[ch]
    for bt in (1, 3):
        mix.place('accordion', reed(vo, 0.20), t0 + bt * T.BEAT, 0.20 if light else 0.24)
        if intensity > 1.35 and not light:
            mix.place('accordion', reed(vo, 0.12), t0 + (bt + 0.5) * T.BEAT, 0.13)
    # the bellows pad under it (from bar 3)
    if b >= 3 or light:
        mix.place('pad', reed([vo[0] - 12, vo[0], vo[2]], T.BAR, att=0.25, rel=0.3, cut=1400), t0, 0.05)
    # cimbalom shimmer: 16th arpeggios, building through the back half
    if intensity > 1.2 and not light:
        k = min(1.0, (intensity - 1.2) / 0.3)
        tones = CIMB[ch]
        for st in range(16):
            if st % 2 and k < 0.6:
                continue
            m = tones[(st * 3 + b) % len(tones)]
            mix.place('cimbalom', cimbalom(m, 0.7 + 0.3 * (st % 4 == 0)), t0 + st * S16, 0.05 * k, pan=0.3 * math.sin(st))


# ---------------------------------------------------------------- the violin's bow: quarter turns -> the melody
def bow_triggers():
    """times the violin's bow passes a quarter turn during the fight, from the recorded angle (30 fps, interpolated)"""
    th = RUN['TH'][:, KV]
    d = (np.diff(th) + np.pi) % (2 * np.pi) - np.pi
    phi = np.concatenate([[0.0], np.cumsum(np.abs(d))])
    tt = T.T_FIGHT + np.arange(len(phi)) / T.FPS
    q = math.pi / 2
    out = []
    k = 1
    for i in range(1, len(phi)):
        while phi[i] >= k * q:
            u = (k * q - phi[i - 1]) / max(1e-9, phi[i] - phi[i - 1])
            out.append(tt[i - 1] + u / T.FPS)
            k += 1
    return [t for t in out if t < T_KO]


def main():
    mix = A.Mix(T.DUR + 4.0)
    # ------------------------------------------------ count-in (bar 0, on the dominant): sticks + violin chop + bass
    for j, (tc, txt) in enumerate(T.COUNT[:3]):
        tc = max(tc, 0.004)                                # the "3" 4 ms into the file: no click at the seam
        mix.place('kit', sticks(), tc, 0.30)
        for m in (69, 73, 76):
            mix.place('violin', bow_note(m, 1, 1.0), tc, 0.20)
        mix.place('violin', scratch(0.08, j), tc, 0.05)
        mix.place('bass', upright(33, 0.4), tc, 0.62)
        mix.place('accordion', reed(VOICING['A'], 0.16), tc, 0.20)
    for st, v in ((12, 0.45), (13, 0.6), (14, 0.75), (15, 0.9)):   # the drummer's pickup into FIGHT!
        mix.place('kit', drum('S', v), 0 * T.BAR + st * S16, 0.32)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), 3 * T.BEAT, 0.14)
    mix.place('kit', drum('C', 1.0), T.BAR, 0.40)
    # ------------------------------------------------ the bed: bars 1 .. the K.O.
    bar_ko = int(T_KO // T.BAR)
    for b in range(1, bar_ko + 1):
        bed(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
    mix.place('fx', A.whoosh(3.6, 300, 6000), 44.0 - 3.6, 0.10)
    for k in ('bass', 'accordion', 'pad', 'cimbalom'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ DRUMS: the pulses are the kit
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'pulse' and e[2] == KD:
            tq = q16(t)
            hp = hp_at(KD, t)
            main_ = bool(e[3])
            pos = int(round(tq / T.BEAT)) % 4
            pan = -0.15
            if not main_:                                  # DRUM ROLL: 16th pulses = a snare roll
                sig, g, _ = colour(drum('S', 0.5), KD, t, 0.6)
                mix.place('kit', sig, tq, 0.22 * g, pan=pan)
                continue
            lead = (hp - hp_at(KV, t)) / 100.0
            if pos in (0, 2):
                sig, g, _ = colour(drum('K', 1.0), KD, t, 0.4)
                mix.place('kit', sig, tq, 0.52 * g)
            else:
                snare_ok = hp > 25 or (hp > 12 and pos == 3)
                if snare_ok:
                    sig, g, _ = colour(drum('S', 0.9), KD, t, 0.6)
                    mix.place('kit', sig, tq, 0.40 * g, pan=pan)
                elif hp > 12:
                    mix.place('kit', A.hp(drum('S', 0.3), 900), tq, 0.06, pan=pan)    # a ghost of the snare
            if lead > 0.04:                                # the leader: off-beat hats
                mix.place('kit', A.hat(0.03), tq + T.BEAT / 2, 0.10, pan=0.3)
                if pos == 3:
                    mix.place('kit', A.hat(open_=True), tq + T.BEAT / 2, 0.05, pan=0.3)
        elif kind == 'hit' and e[2] == KD and e[7] == 'pulse':
            tq = q16(t)
            hp = hp_at(KD, t)
            if hp > 45:
                fill = ('T1', 'T2', 'T3')
            elif hp > 25:
                fill = ('T3',)
            else:
                fill = ()
            for j, pc in enumerate(fill):
                sig, g, _ = colour(drum(pc, 0.85 - 0.1 * j), KD, t, 0.5)
                mix.place('kit', sig, tq + (j + 1) * S16, 0.30 * g, pan=-0.3 + 0.3 * j)
            mix.place('fx', A.thud(0.3, 130, 55, seed=int(t * 7)), tq, 0.12)
    # ------------------------------------------------ VIOLIN: the bow's turns play the line
    trig = sorted(set(round(q16(t) / S16) for t in bow_triggers()))
    for j, slot in enumerate(trig):
        tq = slot * S16
        nxt = trig[j + 1] * S16 if j + 1 < len(trig) else tq + 4 * S16
        n16 = int(np.clip(round((nxt - tq) / S16), 1, 8))
        hp = hp_at(KV, tq) / 100.0
        if h01(slot, 3) < 0.55 * (1 - hp) ** 1.6:          # sparse when hurt
            continue
        m = T.melody_at(tq)
        vel = 0.62 + 0.25 * min(1.0, 2.0 / n16)
        sig, g, lead = colour(bow_note(m, n16, round(vel, 1)), KV, tq, 1.0)
        pan = 0.2
        mix.place('violin', sig, tq, 0.34 * g, pan=pan)
        if (1 - hp) > 0.25:
            mix.place('violin', scratch(min(0.25, n16 * S16), slot), tq, 0.09 * (1 - hp) ** 1.3, pan=pan)
        if lead > 0.04:                                    # the leader: a double-stop a sixth below
            under = [x for x in T.scale_notes(m - 10, m - 8, T.chord_at(tq))]
            if under:
                mix.place('violin', colour(bow_note(under[0], n16, 0.6), KV, tq)[0], tq, 0.16 * g, pan=pan)
    climb = T.scale_notes(74, 98)
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'hit' and e[2] == KV:
            tq = q16(t)
            how, sub = e[7], e[8]
            pan = float(np.clip(e[5] / 3.5, -0.6, 0.6))
            if how == 'blade':                             # accents climbing the scale, one step per landed hit
                m = climb[int(sub) % len(climb)]
                sig, g, _ = colour(bow_note(m, 1, 1.0), KV, t)
                mix.place('violin', sig, tq, 0.36 * g, pan=pan)
                mix.place('violin', scratch(0.05, int(t * 11)), tq, 0.05, pan=pan)
            elif how == 'key':                             # a pizzicato dart lands
                m = T.scale_notes(69, 93, T.chord_at(tq))[int(sub) * 2 % 9]
                mix.place('violin', colour(pizz(m, 0.9), KV, t)[0], tq, 0.34, pan=pan)
            elif how == 'wave':
                mix.place('fx', A.thud(0.45, 110, 40), tq, 0.36)
        elif kind == 'cleared' and e[2] == KV:             # a dart swatted by a drum pulse
            mix.place('violin', A.hp(pizz(81, 0.5), 1500), q16(t), 0.06)
        elif kind == 'block':                              # the bow (or a dart) bounces off the cymbal
            mix.place('fx', A.chime(98, 0.5) + A.chime(105, 0.5) * 0.5, q16(t), 0.07)
        elif kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.06 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            mix.place('fx', A.chime(91, 0.25), t, 0.05)
        elif kind == 'ability':
            who, name = e[2], e[3]
            tq = q16(t)
            if name == 'DRUM ROLL':
                mix.place('kit', drum('C', 0.9), tq + 4 * T.BEAT, 0.28)
                mix.place('kit', drum('K', 1.0), tq + 4 * T.BEAT, 0.4)
            elif name == 'CRASH CYMBAL':
                mix.place('kit', drum('C', 1.0), tq, 0.42)
                dur = RD.CFG[who]['abilities']['unlock2']['dur_beats'] * T.BEAT
                n = int(dur * SR)
                wash = A.bp(np.random.default_rng(4).normal(0, 1, n), 4000, 12000) * A.adsr(n, 0.05, 0.2, 0.6, 0.4)
                mix.place('fx', wash, tq, 0.03)
            elif name == 'DOUBLE KICK':
                mix.place('fx', A.whoosh(2 * T.BEAT, 200, 2500), tq, 0.10)
            elif name == 'PIZZICATO':
                n = RD.CFG[who]['abilities']['unlock1']['n']
                ms = T.scale_notes(69, 93, T.chord_at(tq))
                for j in range(n):                         # the darts launch on the 16ths as a rising pluck
                    mix.place('violin', colour(pizz(ms[j * 2 % len(ms)], 0.8), KV, t)[0], tq + j * S16, 0.26)
            elif name == 'TREMOLO':
                dur = RD.CFG[who]['abilities']['unlock2']['dur']
                k = 0
                while k * S16 / 2 < dur:                   # a shiver of 32nds on the line under the frantic bow
                    tk = tq + k * S16 / 2
                    mix.place('violin', colour(bow_note(T.melody_at(tk) + 12, 1, 0.5), KV, t)[0], tk,
                              0.07 * (0.5 + 0.5 * k * S16 / 2 / dur))
                    k += 1
            elif name == 'SCREECH':
                mix.place('violin', squeal(1.9), tq, 0.16)
                mix.place('violin', colour(bow_note(97, 6, 1.0), KV, t, 2.0)[0], tq, 0.18)
        elif kind == 'slam':                               # DOUBLE KICK: the tap an 8th early, then the stomp
            ts = e[0]
            mix.place('kit', drum('K', 0.75), q16(ts - T.BEAT / 2), 0.40)
            mix.place('kit', drum('K', 1.0), ts, 0.62)
            mix.place('bass', upright(26, 0.8), ts, 0.5)
            mix.place('fx', A.thud(0.6, 90, 32), ts, 0.55)
            mix.place('kit', drum('C', 0.8), ts, 0.16)
    # ------------------------------------------------ the K.O. moment
    mix.place('kit', drum('C', 1.0), T_KO, 0.45)
    mix.place('bass', upright(26, 1.4), T_KO, 0.62)
    mix.place('fx', A.thud(0.9, 120, 30), T_KO, 0.7)
    loser = NAMES[LOSE]
    if loser == 'VIOLIN':
        for j, m in enumerate((88, 81, 74, 67)):                          # the four strings snapping, E A D G
            mix.place('violin', pizz(m, 1.0), RD.T_HS + 0.11 * j, 0.24, pan=(j - 1.5) / 4)
            mix.place('violin', A.hp(scratch(0.06, 40 + j), 3000), RD.T_HS + 0.11 * j, 0.10, pan=(j - 1.5) / 4)
    else:
        for j, pc in enumerate(('C', 'T1', 'T2', 'T3', 'S', 'K')):         # the kit collapsing
            mix.place('kit', drum(pc, 0.8), RD.T_HS + 0.08 * j, 0.26, pan=(j - 2.5) / 6)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.16)
    # the winner's stinger on "(WINNER) WINS"
    tw = q16(T_KO + 1.3)
    if NAMES[WIN] == 'DRUMS':
        for pc, v in (('C', 1.0), ('K', 1.0), ('T3', 0.9)):
            mix.place('solo', drum(pc, v), tw, 0.40)
        mix.place('solo', drum('T3', 0.8), tw + S16, 0.26)
    else:
        mix.place('solo', sum(bow_note(m, 6, 1.0) for m in (62, 69, 77)), tw, 0.30)
    mix.place('accordion', reed((50, 57, 62, 65, 69), 1.2, rel=0.5), tw, 0.20)
    mix.place('bass', upright(38, 0.9), tw, 0.6)
    solo0 = RD.T_SOLO
    mix.place('fx', A.whoosh(solo0 - tw - 0.1, 300, 5000), tw + 0.1, 0.10)
    # ------------------------------------------------ victory: lighter oom-pah + the winner's solo
    b_solo = int(round(solo0 / T.BAR))
    for b in range(b_solo, 35):
        bed(mix, b, light=True)
    wname = NAMES[WIN]
    if wname == 'DRUMS':
        for bt, pc, vel in T.SOLO['DRUMS']:
            mix.place('solo', drum(pc, vel), solo0 + bt * T.BEAT, 0.48 if pc != 'C' else 0.36,
                      pan={'T1': -0.25, 'T2': 0.0, 'T3': 0.25}.get(pc, 0.0))
    else:
        for bt, m, d in T.SOLO['VIOLIN']:
            n16 = max(1, int(round(d * 4)))
            mix.place('solo', bow_note(m, n16, 0.9), solo0 + bt * T.BEAT, 0.48)
    # the loser reassembles: its pieces fly home as a rising run, a glass "pop" when it's whole
    run = [62, 65, 69, 73, 74, 77, 81, 86]
    for j, m in enumerate(run):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
        if loser == 'VIOLIN':
            mix.place('violin', pizz(m, 0.8), tj, 0.18)
        else:
            mix.place('kit', drum(('T3', 'T2', 'T1', 'S')[j % 4], 0.5 + 0.06 * j), tj, 0.18)
    mix.place('fx', A.chime(95, 0.5) + A.chime(102, 0.5) * 0.5, T.T_GLIDE, 0.12)
    # ------------------------------------------------ bar 35: build into the "3" on sample 0
    t35 = 35 * T.BAR
    mix.place('kit', roll(T.DUR - t35 + 0.03, 0.15, S16 / 2), t35, 0.30)
    mix.place('fx', riser(T.DUR - t35), t35, 0.22)
    for bt in range(4):
        mix.place('bass', upright((33, 37, 40, 43)[bt], 0.36), t35 + bt * T.BEAT, 0.55)
        mix.place('accordion', reed(VOICING['A'], 0.18), t35 + bt * T.BEAT + T.BEAT / 2, 0.18)
    mix.place('pad', reed([45, 57, 61, 64, 67], T.DUR - t35 + 0.02, att=0.6, rel=0.001, cut=1800), t35, 0.07)
    for st in (0, 8, 12, 14):
        mix.place('kit', drum('K', 0.9), t35 + st * S16, 0.42)
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


BUS_GAIN = {'kit': 2.6, 'violin': 0.75, 'solo': 2.0, 'fx': 1.5, 'bass': 0.8, 'accordion': 2.0, 'pad': 1.2,
            'cimbalom': 1.4}
# section automation (dB), applied AFTER the compressor so the limiter can't flatten the build: the count-in at full,
# the early fight pulled back, rising through the final exchange, the victory section loud, the end close to the start
GAIN_PTS = [(0, 0.0), (1.55, 0.0), (2.1, -1.6), (20, -1.5), (30, -0.6), (40, 1.3), (44, 2.3), (50.8, 3.3), (52.3, 2.8),
            (53.4, 3.3), (58.0, 3.3), (60.0, 1.6), (61.02, 0.6)]


def master(mix, thr=0.22, ceiling=0.89):
    """Mix.master with the overflow past the loop (tails, reverb) folded onto the start; NA samples (whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('violin', 0.55), ('solo', 0.5), ('fx', 0.4), ('accordion', 0.4),
                                                ('pad', 0.5), ('cimbalom', 0.6), ('kit', 0.15)))
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
