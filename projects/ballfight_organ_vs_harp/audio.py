"""#35 BALLFIGHT #9: Organ vs Harp - soundtrack (gothic horror), all synthesised, original.

The bed has no organ and no harp of its own: the two fighters are the leads, both from the honest run:
  ORGAN  every pipe row is an organ chord stab on the beat (the chord of the bar, short, never held, so the bed doesn't
         turn to mush), in the Pixel Pitch organ's own reed voice; a row that lands adds the pedal and a high accent;
         CATHEDRAL is a held full-organ chord with a bell, PULL ALL STOPS a swell, TOCCATA a falling 16th run over the
         telegraph and the lowest, fullest chord on the stomp. As its HP drops the stabs lose voices and get dirtier.
  HARP   every string it lays is a rising plucked arpeggio over the next two beats; every crossing rings that string
         (a chord tone up high, by the string's id); PLUCK is a rising 7-note fan, ANGEL WINGS a shimmer, 47 STRINGS a
         huge glissando down. As its HP drops the arpeggios thin out and buzz. The leader gets an octave double.
Bed: D minor at 125.90 BPM (32 bars = 61 s), slow burn into a pounding back half: a choir "aah" pad, a tolling bell, a
low drone, a ticking clock, a heartbeat kick and a low-string ostinato (bars 1-12); toms and a fuller ostinato (13-20);
four-on-the-floor, timpani and staccato choir hits (21-). The count-in bar (bell + organ stab + harp pluck per count, on
the dominant) drops on FIGHT!; at the K.O. the bed tape-stops with the hit-stop, the loser shatters (strings snapping or
pipes clattering), the crowd roars, the winner's stinger lands, then its solo over a lighter bed; bar 31 builds
(timpani roll, riser, choir swell, bell on A) into the "3" on sample 0. Every hit is snapped to the nearest 16th.

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
KO_, KH = NAMES.index('ORGAN'), NAMES.index('HARP')
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames (2860 = 61.013 s): loops sample-perfectly
S16 = T.S16
BAR_KO = int(T_KO // T.BAR)


def q16(t):
    return round(t / S16) * S16


def h01(*xs):
    """deterministic hash in [0, 1) (thinning decisions are part of the score, not random each run)"""
    v = 0
    for x in xs:
        v = (v * 1000003 + int(x * 1000) + 7) % 2147483647
    return ((v * 48271) % 2147483647) / 2147483647.0


# ---------------------------------------------------------------- the fighters' Pixel Pitch voices (cached, level-matched)
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


def organ_chord(ms, dur=0.17, vel=0.85, cap=None):
    cap = cap or round(dur + 0.22, 2)
    return sum(note('ORGAN', m, dur, cap, vel) for m in ms) / max(1.0, len(ms) ** 0.6)


def harp(m, vel=0.85, cap=1.0):
    return note('HARP', m, 0.3, cap, vel)


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


VOICING = {'Dm': (57, 62, 65), 'Bb': (58, 62, 65), 'Gm': (55, 58, 62), 'A': (57, 61, 64)}
PEDAL = {'Dm': 38, 'Bb': 34, 'Gm': 43, 'A': 33}


# ---------------------------------------------------------------- bed synths (original)
def choir(ms, dur, att=0.45, rel=0.6, vowel=(700, 1150, 2650), vel=1.0, seed=0):
    """a choir 'aah': detuned saw sections through three vowel formants, a slow vibrato"""
    n = int((dur + rel) * SR)
    t = A.tt(n)
    x = np.zeros(n)
    rng = np.random.default_rng(seed)
    for m in ms:
        f = A.mtof(m)
        for d in (-0.006, -0.002, 0.003, 0.007):
            vib = 1 + 0.004 * np.sin(2 * np.pi * (5.0 + rng.uniform(-0.4, 0.4)) * t + rng.uniform(0, 6))
            ph = np.cumsum(f * (1 + d) * vib) / SR
            x += 2 * (ph % 1.0) - 1
    x /= 4 * max(1, len(ms))
    f1, f2, f3 = vowel
    y = A.bp(x, f1 * 0.8, f1 * 1.2) * 1.0 + A.bp(x, f2 * 0.88, f2 * 1.12) * 0.55 + A.bp(x, f3 * 0.93, f3 * 1.07) * 0.22
    y += A.lp(x, 450) * 0.35
    env = np.clip(t / att, 0, 1) ** 1.5 * np.exp(-np.maximum(t - dur, 0.0) / (rel * 0.4))
    return y * env * vel


def bell(m, dur=4.0, vel=1.0):
    """a tolling church bell: hum, prime, minor-third tierce, quint, nominal + upper partials, each its own decay"""
    n = int(dur * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = np.zeros(n)
    for r, a, dec in ((0.5, 0.6, 2.6), (1.0, 0.8, 1.9), (1.19, 0.55, 1.5), (1.5, 0.35, 1.2), (2.0, 0.6, 1.0),
                      (2.52, 0.28, 0.7), (3.0, 0.2, 0.55), (4.07, 0.14, 0.4), (5.2, 0.08, 0.25)):
        x += a * np.sin(2 * np.pi * f * r * t + r) * np.exp(-t / dec) * (1 + 0.15 * np.sin(2 * np.pi * 1.3 * r * t))
    k = int(0.006 * SR)
    x[:k] += np.random.default_rng(int(m)).normal(0, 0.8, k) * np.linspace(1, 0, k)
    return x * vel * 0.5


def tick(vel=1.0, hi=True):
    """the clock: a dry woody tick (tock a little lower)"""
    n = int(0.06 * SR)
    t = A.tt(n)
    f = 2400 if hi else 1700
    x = np.sin(2 * np.pi * f * t) * np.exp(-t / 0.008) + 0.4 * A.bp(np.random.default_rng(2).normal(0, 1, n), 1500, 5000) * np.exp(-t / 0.004)
    return x * vel


def heart(vel=1.0):
    """a heartbeat kick: soft, deep, short"""
    return A.lp(A.kick(0.6, 0.25, 90, 42), 400) * vel


def timp(m, vel=1.0, dur=1.4):
    """a timpani hit: a tuned drum (pitch drops a touch after the stroke) + the felt mallet"""
    n = int(dur * SR)
    t = A.tt(n)
    f = A.mtof(m) * (1 + 0.06 * np.exp(-t / 0.05))
    ph = 2 * np.pi * np.cumsum(f) / SR
    x = np.sin(ph) + 0.5 * np.sin(1.5 * ph) * np.exp(-t / 0.3) + 0.3 * np.sin(1.99 * ph) * np.exp(-t / 0.2)
    x = x * np.exp(-t / 0.55) + 0.4 * A.lp(np.random.default_rng(int(m)).normal(0, 1, n), 900) * np.exp(-t / 0.02)
    return np.tanh(1.3 * x) * vel


def cello(m, dur=0.16, vel=1.0):
    """low strings, staccato: three detuned saws, bowed attack, dark"""
    n = int((dur + 0.08) * SR)
    x = A.saw(A.mtof(m), n, det=0.004, voices=3)
    x = A.lp(x, 1500) * A.adsr(n, 0.008, 0.06, 0.6, 0.05)
    return x * vel


def drone(m, dur, vel=1.0):
    n = int(dur * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = np.sin(2 * np.pi * f * t) + 0.4 * A.lp(A.saw(f, n, det=0.003, voices=2), 300)
    env = np.clip(t / 0.4, 0, 1) * np.clip((dur - t) / 0.4, 0, 1)
    return x * env * (1 + 0.1 * np.sin(2 * np.pi * 0.25 * t)) * vel


def roll(dur, m=38, f0=0.25, step=None):
    """a timpani roll, swelling"""
    step = step or S16 / 2
    n = int(dur * SR)
    out = np.zeros(n + SR * 2)
    t = 0.0
    j = 0
    while t < dur - 1e-4:
        a = int(t * SR)
        s = timp(m, 0.6, 0.5) * (f0 + (1 - f0) * (t / dur) ** 1.5) * (0.85 + 0.15 * (j % 2))
        out[a:a + len(s)] += s[:len(out) - a]
        t += step
        j += 1
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


def snap(m, vel=1.0):
    """a harp string snapping: a sharp pluck with a twang and a whip of noise"""
    n = int(0.5 * SR)
    t = A.tt(n)
    f = A.mtof(m) * (1 + 0.25 * np.exp(-t / 0.03))
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.09) + 0.5 * A.bp(np.random.default_rng(int(m)).normal(0, 1, n), 2000, 9000) * np.exp(-t / 0.015)
    return np.tanh(2 * x) * vel


def clank(seed, vel=1.0):
    """a tin pipe clattering on the canvas"""
    n = int(0.4 * SR)
    t = A.tt(n)
    rng = np.random.default_rng(seed)
    x = np.zeros(n)
    for r in (1.0, 2.76, 5.4, 8.9):
        x += np.sin(2 * np.pi * 620 * r * (1 + rng.uniform(-0.05, 0.05)) * t) * np.exp(-t * (8 + 4 * r))
    return x * vel * 0.5


# ---------------------------------------------------------------- one bar of the bed
CELLO_PAT = (0, 0, 7, 0, 8, 0, 7, 12)            # 8ths, from the bar's root (a minor-sixth sting in it)


def bed(mix, b, sec, light=False):
    t0 = b * T.BAR
    ch = T.CHORDS[b]
    vo = VOICING[ch]
    root = PEDAL[ch]
    # choir pad on the chord (fuller from the climb)
    ms = [vo[0] - 12, vo[0], vo[1], vo[2]] if sec != 'burn' else [vo[0] - 12, vo[0], vo[2]]
    mix.place('choir', choir(ms, T.BAR, att=0.5 if sec == 'burn' else 0.3, rel=0.5, seed=b), t0,
              {'burn': 0.10, 'climb': 0.13, 'pound': 0.15}[sec] * (0.8 if light else 1.0))
    mix.place('drone', drone(root - 12 if root > 30 else root, T.BAR + 0.3), t0, 0.10)
    # the bell: every bar in the burn, on 1 and 3 later
    bell_m = 50 if ch != 'A' else 45
    mix.place('bell', bell(bell_m, 3.5), t0, 0.16)
    if sec != 'burn' and not light:
        mix.place('bell', bell(bell_m + 12, 2.0), t0 + 2 * T.BEAT, 0.07)
    if light:
        return
    # the clock ticks the 8ths (tick-tock), quieter once the drums pound
    for e8 in range(8):
        mix.place('perc', tick(1.0 if e8 % 2 == 0 else 0.7, e8 % 2 == 0), t0 + e8 * T.BEAT / 2,
                  {'burn': 0.05, 'climb': 0.045, 'pound': 0.035}[sec], pan=0.35 if e8 % 2 == 0 else -0.35)
    # low strings: the root pulsing on the 8ths in the burn, the full ostinato from the climb
    for e8 in range(8):
        if sec == 'burn':
            m, v = root, (0.9 if e8 % 2 == 0 else 0.6)
        else:
            m, v = root + CELLO_PAT[e8], (1.0 if e8 in (0, 3, 6) else 0.7)
        mix.place('strings', cello(m, 0.15), t0 + e8 * T.BEAT / 2, (0.045 if sec == 'burn' else 0.07) * v)
    # drums
    if sec == 'burn':                                       # a heartbeat: lub-dub on 1, again on 3
        for bt in (0, 2):
            mix.place('drums', heart(1.0), t0 + bt * T.BEAT, 0.55)
            mix.place('drums', heart(0.7), t0 + bt * T.BEAT + S16 * 1.5, 0.40)
    elif sec == 'climb':
        mix.place('drums', A.kick(0.9, 0.3), t0, 0.50)
        mix.place('drums', A.kick(0.8, 0.3), t0 + 2.5 * T.BEAT, 0.40)
        for bt in (1, 3):
            mix.place('drums', A.thud(0.35, 150, 70, seed=b * 4 + bt), t0 + bt * T.BEAT, 0.30)
        mix.place('drums', timp(38 if ch != 'A' else 45, 0.8), t0, 0.22)
    else:                                                   # pound: four on the floor, timpani, staccato choir hits
        for bt in range(4):
            mix.place('drums', A.kick(1.0, 0.32), t0 + bt * T.BEAT, 0.55)
            mix.place('choir', choir([vo[1] + 12, vo[2] + 12], 0.09, att=0.01, rel=0.08, vowel=(800, 1250, 2700),
                                     seed=b * 4 + bt), t0 + (bt + 0.5) * T.BEAT, 0.07)
        mix.place('drums', timp(PEDAL[ch] + 12 if PEDAL[ch] < 40 else PEDAL[ch], 1.0), t0, 0.30)
        mix.place('drums', timp(PEDAL[ch] + 7 + (12 if PEDAL[ch] < 40 else 0), 0.7), t0 + 2 * T.BEAT, 0.20)
        for st in (14, 15):
            mix.place('drums', A.thud(0.25, 180, 90, seed=b * 9 + st), t0 + st * S16, 0.22)
        for bt in (1, 3):
            mix.place('drums', A.snare(0.12), t0 + bt * T.BEAT, 0.10)


def main():
    mix = A.Mix(T.DUR + 4.0)
    # ------------------------------------------------ v3: the VS title card (bar 0, on the tonic): one band hit per word
    t1, t2, t3 = 0.004, T.BEAT, 2 * T.BEAT                 # ORGAN 4 ms in (no click at the seam), VS, HARP
    for tw in (t1, t2, t3):                                # the band on every word: timpani + boom + bell + a choir "hah"
        mix.place('drums', timp(38, 1.0, 1.2), tw, 0.42)
        mix.place('fx', A.thud(0.9, 120, 32, seed=int(tw * 50) + 3), tw, 0.50)
        mix.place('bell', bell(50, 2.2), tw, 0.16)
        mix.place('choir', choir([50, 57, 62, 65], 0.30, att=0.008, rel=0.25, vowel=(800, 1250, 2700), seed=int(tw * 9)),
                  tw, 0.16)
    # ORGAN: the full organ at once, pedal to mixture, the lowest D under it
    mix.place('organ', organ_chord([26, 38, 50, 57, 62, 65, 69, 74], 0.5, 1.0, cap=1.2), t1, 0.40)
    # VS: a thunder crack where it splits the divider
    nn = int(1.4 * SR)
    tt = A.tt(nn)
    crack = A.bp(np.random.default_rng(91).normal(0, 1, nn), 1500, 9000) * np.exp(-tt / 0.035)
    rumble = A.lp(np.random.default_rng(92).normal(0, 1, nn), 160) * np.clip(tt / 0.02, 0, 1) * np.exp(-tt / 0.5) * 3.0
    mix.place('fx', crack * 0.9 + rumble, t2, 0.40)
    mix.place('drums', A.crash(1.6), t2, 0.10)
    # HARP: a sweep up the D-minor strings into a big rolled chord
    sweep = T.scale_notes(50, 86, 'Dm')
    for j, m in enumerate(sweep):
        mix.place('harp', harp(m, 0.75, 0.8), t3 - 0.12 + 0.12 * j / len(sweep), 0.12, pan=0.5 * math.sin(j * 0.6))
    for j, m in enumerate((50, 57, 62, 65, 69, 74, 77)):
        mix.place('harp', harp(m, 1.0, 1.4), t3 + j * 0.012, 0.22, pan=-0.3 + 0.1 * j)
    # beat 4: the hold + a timpani pickup as the doors open; a choir pad under the whole card (never quiet)
    mix.place('choir', choir([50, 57, 62, 65], 3 * T.BEAT, att=0.05, rel=0.3), 0.0, 0.10)
    mix.place('drone', drone(26, T.BAR + 0.2), 0.0, 0.10)
    for st, v in ((12, 0.5), (13, 0.62), (14, 0.78), (15, 0.95)):
        mix.place('drums', timp(45, v, 0.5), st * S16, 0.30)
    mix.place('fx', A.whoosh(T.T_DOORS[1] - T.T_DOORS[0] + 0.1, 300, 5000), T.T_DOORS[0] - 0.05, 0.16)
    # ------------------------------------------------ count-in (bar 1, on the dominant): bell + organ stab + harp pluck
    for j, (tc, txt) in enumerate(T.COUNT[:3]):
        mix.place('bell', bell(45 + 12 * (j == 2), 2.0), tc, 0.20)
        mix.place('organ', organ_chord(VOICING['A'], 0.18, 0.9), tc, 0.30)
        mix.place('harp', harp(69 + 4 * j, 0.9), tc, 0.30)
        mix.place('drums', timp(45, 0.9), tc, 0.30)
    mix.place('choir', choir([45, 57, 61, 64], T.BAR, att=0.2, rel=0.05), T.BAR, 0.09)
    mix.place('drums', roll(T.BEAT, 45, 0.3), T.BAR + 3 * T.BEAT, 0.30)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), T.BAR + 3 * T.BEAT, 0.14)
    mix.place('drums', A.crash(2.0), T.T_FIGHT, 0.16)
    mix.place('drums', A.kick(1.0, 0.4), T.T_FIGHT, 0.6)
    mix.place('bell', bell(38, 5.0), T.T_FIGHT, 0.24)
    # ------------------------------------------------ the bed: FIGHT! (bar 2) .. the K.O.
    for b in range(int(round(T.T_FIGHT / T.BAR)), BAR_KO + 1):
        bed(mix, b, T.section_at(b * T.BAR))
    b_pound = next(b0 for b0, nm in T.SECTION if nm == 'pound')
    mix.place('drums', A.crash(2.5), b_pound * T.BAR, 0.18)            # the pound arrives
    mix.place('fx', A.whoosh(3.6, 300, 6000), b_pound * T.BAR - 3.6, 0.10)
    for k in ('choir', 'drone', 'bell', 'perc', 'strings', 'drums'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ the fighters, from the run
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        tq = q16(t)
        ch = T.chord_at(tq)
        if kind == 'line':                                  # ORGAN: a pipe row = a chord stab
            hp = hp_at(KO_, t) / 100.0
            vo = list(VOICING[ch])
            if hp < 0.5:
                vo = vo[:2] if h01(t, 1) < 0.6 else vo[1:]
            if hp < 0.25:
                vo = vo[:1]
            sig, g, lead = colour(organ_chord(vo, 0.17, 0.85), KO_, t, 0.8)
            mix.place('organ', sig, tq, 0.34 * g, pan=-0.15)
            if lead > 0.04:                                 # the leader: the pedal under it
                mix.place('organ', colour(organ_chord([PEDAL[ch] + 12], 0.2, 0.7), KO_, t)[0], tq, 0.16 * g, pan=-0.1)
        elif kind == 'string':                              # HARP: a new string = a rising arpeggio over two beats
            hp = hp_at(KH, t) / 100.0
            notes = T.chord_notes(ch, 50, 81)
            arp = [notes[0], notes[2], notes[3], notes[5]] if len(notes) > 5 else notes[:4]
            if hp < 0.6:
                arp = arp[::2]
            if hp < 0.3:
                arp = arp[:1]
            lead = (hp_at(KH, t) - hp_at(KO_, t)) / 100.0
            for j, m in enumerate(arp):
                tj = tq + j * 2 * S16 * (4 // max(1, len(arp)) if len(arp) < 4 else 1)
                sig, g, _ = colour(harp(m, 0.8 + 0.05 * j), KH, t, 1.2)
                mix.place('harp', sig, tj, 0.30 * g, pan=0.2)
                if lead > 0.04 and j == len(arp) - 1:
                    mix.place('harp', colour(harp(m + 12, 0.7), KH, t)[0], tj + S16, 0.16 * g, pan=0.25)
        elif kind == 'hit':
            att, how, sub = e[2], e[7], e[8]
            pan = float(np.clip(e[5] / 3.5, -0.6, 0.6))
            if how == 'pipe':                               # a row lands: the pedal + a high accent
                sig, g, _ = colour(organ_chord([PEDAL[ch], VOICING[ch][2] + 12], 0.22, 1.0), KO_, t, 0.6)
                mix.place('organ', sig, tq + S16 if q16(t) < t else tq, 0.30 * g, pan=pan)
                mix.place('fx', A.thud(0.3, 130, 55, seed=int(t * 7)), tq, 0.14)
            elif how == 'string':                           # a crossing rings the string
                hi = T.chord_notes(ch, 67, 88)
                m = hi[int(sub) % len(hi)]
                sig, g, _ = colour(harp(m, 1.0, 1.2), KH, t, 1.0)
                mix.place('harp', sig, tq, 0.34 * g, pan=pan)
                mix.place('harp', snap(m + 12, 0.25), tq, 0.05, pan=pan)
            elif how == 'key':                              # a PLUCK dart lands
                m = T.chord_notes(ch, 69, 93)[int(sub) % 5]
                mix.place('harp', colour(harp(m, 0.9, 0.8), KH, t)[0], tq, 0.26, pan=pan)
            elif how == 'wave':
                mix.place('fx', A.thud(0.45, 110, 40), tq, 0.36)
        elif kind == 'block':                               # a hit lands on a shield: a glassy chime
            mix.place('fx', A.chime(98, 0.5) + A.chime(105, 0.5) * 0.5, tq, 0.07)
        elif kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.05 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            mix.place('fx', A.chime(91, 0.25), t, 0.05)
        elif kind == 'ability':
            who, name = e[2], e[3]
            if name == 'CATHEDRAL':                         # a held full-organ chord ringing under the dome + the bell
                vo = VOICING[ch]
                mix.place('organ', organ_chord([PEDAL[ch], vo[0] - 12, vo[0], vo[1], vo[2], vo[2] + 12], 2 * T.BEAT, 0.8,
                                               cap=2 * T.BEAT + 0.6), tq, 0.20)
                mix.place('bell', bell(50, 3.0), tq, 0.18)
            elif name == 'PULL ALL STOPS':                  # a swell: the chord climbing through the stops
                for j, m in enumerate((PEDAL[ch], VOICING[ch][0], VOICING[ch][1], VOICING[ch][2], VOICING[ch][2] + 12)):
                    mix.place('organ', organ_chord([m], 2 * T.BEAT - j * S16, 0.75, cap=2 * T.BEAT + 0.3),
                              tq + j * S16, 0.12 + 0.03 * j)
            elif name == 'TOCCATA':                         # a falling 16th run over the telegraph (the stomp follows)
                run_ = [81, 79, 77, 76, 74, 73, 70, 69]
                for j, m in enumerate(run_):
                    mix.place('organ', organ_chord([m, m - 12], 0.11, 0.9), tq + j * S16, 0.22)
            elif name == 'PLUCK':                           # the fan of darts: a rising 7-note pluck
                ms = T.chord_notes(ch, 62, 93)
                for j in range(7):
                    mix.place('harp', colour(harp(ms[j % len(ms)], 0.85), KH, t)[0], q16(t + j * T.BEAT / 6), 0.24,
                              pan=-0.3 + 0.1 * j)
            elif name == 'ANGEL WINGS':                     # a shimmer: harmonics high up + a choir swell
                ms = T.chord_notes(ch, 74, 98)
                for j in range(8):
                    mix.place('harp', harp(ms[(j * 2) % len(ms)], 0.6, 1.4), tq + j * S16 / 2, 0.13, pan=0.4 * math.sin(j))
                mix.place('choir', choir([74, 77, 81], 2 * T.BEAT, att=0.15, rel=0.6, vowel=(450, 950, 2500)), tq, 0.10)
            elif name == '47 STRINGS':                      # every string at once: a huge glissando down
                ms = [m for m in T.scale_notes(43, 96, ch)][::-1]
                span = 1.2
                for j, m in enumerate(ms):
                    mix.place('harp', np.pad(snap(m, 0.6) * 0.5, (0, int(0.4 * SR))) + harp(m, 0.8, 0.9) * 0.6, tq + span * j / len(ms), 0.16,
                              pan=0.6 * math.sin(j * 0.7))
                mix.place('fx', A.thud(0.6, 100, 35), tq, 0.25)
        elif kind == 'slam':                                # TOCCATA's stomp: the lowest, fullest chord
            ts = e[0]
            vo = VOICING[T.chord_at(ts)]
            mix.place('organ', organ_chord([26, 38, vo[0] - 12, vo[0], vo[1], vo[2], vo[2] + 12], 0.6, 1.0, cap=1.4), ts, 0.34)
            mix.place('fx', A.thud(0.7, 90, 30), ts, 0.55)
            mix.place('drums', timp(38, 1.0), ts, 0.4)
            mix.place('bell', bell(38, 3.0), ts, 0.2)
    # ------------------------------------------------ the K.O. moment
    mix.place('drums', A.crash(2.5), T_KO, 0.26)
    mix.place('drums', timp(38, 1.0, 2.0), T_KO, 0.5)
    mix.place('fx', A.thud(0.9, 120, 30), T_KO, 0.7)
    loser = NAMES[LOSE]
    if loser == 'HARP':
        for j, m in enumerate((86, 81, 77, 74, 69, 62, 57)):              # the strings snapping, top to bottom
            mix.place('harp', snap(m, 1.0), RD.T_HS + 0.07 * j, 0.20, pan=(j - 3) / 6)
    else:
        for j in range(7):                                                # the pipes clattering down
            mix.place('fx', clank(j, 1.0 - 0.08 * j), RD.T_HS + 0.09 * j, 0.20, pan=(j - 3) / 6)
            mix.place('organ', organ_chord([62 - 3 * j], 0.08, 0.6), RD.T_HS + 0.09 * j, 0.10)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.16)
    # the winner's stinger on "(WINNER) WINS"
    tw = q16(T_KO + 1.3)
    if NAMES[WIN] == 'ORGAN':
        mix.place('solo', organ_chord([38, 50, 57, 62, 65, 69, 74], 0.9, 1.0, cap=1.6), tw, 0.30)
    else:
        for j, m in enumerate((62, 69, 74, 77, 81, 86)):
            mix.place('solo', harp(m, 1.0, 1.4), tw + j * S16 / 2, 0.30)
    mix.place('bell', bell(50, 4.0), tw, 0.22)
    solo0 = RD.T_SOLO
    mix.place('fx', A.whoosh(max(0.3, solo0 - tw - 0.1), 300, 5000), tw + 0.1, 0.10)
    # ------------------------------------------------ victory: the lighter bed + the winner's solo
    for b in range(int(round(solo0 / T.BAR)), T.N_BARS - 1):
        bed(mix, b, 'climb', light=True)
    if NAMES[WIN] == 'ORGAN':
        for bt, m, d in T.SOLO['ORGAN']:
            t0 = solo0 + bt * T.BEAT
            if t0 >= T.T_REASSEMBLE:
                break
            dur = min(d * T.BEAT * 0.92, T.T_REASSEMBLE - t0)
            if isinstance(m, tuple):
                kind_, r = m
                ms = [r, r + 3, r + 6, r + 9] if kind_ == 'dim' else [r - 12, r, r + 3, r + 7]
            else:
                ms = [m, m - 12]
            mix.place('solo', organ_chord(ms, dur, 0.9, cap=round(dur + 0.3, 2)), t0, 0.34)
    else:
        for bt, m, d in T.SOLO['HARP']:
            t0 = solo0 + bt * T.BEAT
            if t0 >= T.T_REASSEMBLE:
                break
            if isinstance(m, tuple):                        # a glissando up to the top note
                ms = T.scale_notes(62, m[1])
                for j, mm in enumerate(ms):
                    mix.place('solo', harp(mm, 0.7, 0.8), t0 + d * T.BEAT * j / len(ms), 0.16)
            else:
                mix.place('solo', harp(m, 0.95, 1.4), t0, 0.40)
    # the loser reassembles: its pieces fly home as a rising run, a glass "pop" when it's whole
    run_ = [50, 53, 57, 62, 65, 69, 74, 77]
    for j, m in enumerate(run_):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run_))
        if loser == 'HARP':
            mix.place('harp', harp(m + 12, 0.7, 0.6), tj, 0.20)
        else:
            mix.place('organ', organ_chord([m], 0.1, 0.7), tj, 0.16)
    mix.place('fx', A.chime(95, 0.5) + A.chime(102, 0.5) * 0.5, T.T_GLIDE, 0.12)
    # ------------------------------------------------ bar 31: build into the title slam on sample 0
    t31 = (T.N_BARS - 1) * T.BAR
    mix.place('drums', roll(T.DUR - t31 + 0.03, 45, 0.15), t31, 0.32)
    mix.place('fx', riser(T.DUR - t31), t31, 0.20)
    mix.place('choir', choir([45, 57, 61, 64, 69], T.DUR - t31 + 0.02, att=1.2, rel=0.001, vowel=(700, 1150, 2650)), t31, 0.14)
    mix.place('bell', bell(45, 2.0), t31, 0.2)
    for bt in range(4):
        mix.place('strings', cello(33 + (0, 4, 7, 12)[bt], 0.2), t31 + bt * T.BEAT, 0.08)
        mix.place('drums', A.kick(0.9, 0.3), t31 + bt * T.BEAT, 0.45)
    nc = int((T.T_CLOSE + 0.012) * SR)                              # v3: the doors sliding shut into the title slam
    tc_ = A.tt(nc)
    doors = A.bp(np.random.default_rng(93).normal(0, 1, nc), 400, 5000) * (tc_ / T.T_CLOSE) ** 2
    mix.place('fx', doors, T.DUR - T.T_CLOSE, 0.55)
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


BUS_GAIN = {'organ': 2.0, 'harp': 0.85, 'solo': 2.4, 'fx': 1.5, 'choir': 1.6, 'drone': 1.0, 'bell': 1.0, 'perc': 1.0,
            'strings': 1.2, 'drums': 1.6}
# section automation (dB), applied AFTER the compressor so the limiter can't flatten the build: the count-in at full,
# the title card + count-in at full, the burn pulled back, rising through the climb and the pound, the victory loud
GAIN_PTS = [(0, 0.0), (3.5, 0.0), (4.1, -1.8), (20, -1.4), (26.7, -0.6), (41.9, 1.0), (44, 2.0), (50.8, 3.0), (52.3, 2.6),
            (53.4, 3.0), (58.0, 3.0), (60.0, 1.6), (61.02, 0.6)]


def master(mix, thr=0.22, ceiling=0.89):
    """Mix.master with the overflow past the loop (tails, reverb) folded onto the start; NA samples (whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('organ', 0.55), ('harp', 0.5), ('solo', 0.6), ('fx', 0.3), ('choir', 0.6),
                                                ('bell', 0.7), ('strings', 0.3), ('drums', 0.12)))
    wet = A.reverb(send, dur=3.2, decay=0.7) * 0.24
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
