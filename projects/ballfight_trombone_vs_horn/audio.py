"""#34 BALLFIGHT #8: Trombone vs French Horn - soundtrack, v5 (mood: cinematic brass trailer), all synthesised, original.
v5 (HORN v3): ECHO CALL is a delay effect: every call (one per beat) is a round, mellow horn note of timeline.HORN_CALL,
and every echo ghost repeats the call it trails a beat later (HORN RIP: an 8th), quieter and darker each time, panned
toward the ghost; landed blasts are brighter stabs up the chord. HORN RIP rips up the harmonic series of the bar's root;
FINAL FLICK is a horn fanfare over a slide-whistle wind-up and a spring "boing" as the glove pops out, then a THWACK
(snap + thud + braam) and a heavy rope hit. THE HUNT's sounds are kept in audio_v4_backup.py.
v3: bar 0 = the VS title card (a unison band hit per word: TROMBONE's slide blast, VS's thunder crack, FRENCH HORN's
hunting call; a pickup as the doors open), bar 1 = the count-in, FIGHT! on bar 2; the doors' whoosh + the riser ramp
into the title slam on sample 0. v2 kept as audio_v2_backup.py.
v2 (BALLFIGHT v2 redesign): the HORN hunts: every horn call is a note of its theme (a full call per hound released, a
short echo while the pack is full), every bite on the 8th grid a horn stab + jaws, a knocked-out hound a falling yelp,
TALLY-HO's charge a gallop of taiko under the fanfare. v1 (arrows) is kept as audio_v1_backup.py.

The two fighters ARE the brass section; the bed is strings, braams and taiko with no brass of its own. Both parts come
from the honest run, in each fighter's own Pixel Pitch voice, registers kept apart (trombone low, horn mid):
  TROMBONE  every thrust of the slide is a stab of the low-brass ostinato (timeline.TROMBONE_LINE, one note per beat,
            gliding up into the note like a real slide); a landed lance hit is a punchy accent with a big glissando,
            climbing the chord as the reach grows; POWER SLIDE's 8th thrusts double the riff; SAD TROMBONE is the
            plunger "wah wah wah waaah"; PEDAL TONE holds the lowest note over the hovering bell, then the stomp.
  HORN      every volley is a note of the noble hunting-call theme (timeline.HORN_LINE, every 2 beats; horn fifths
            when it leads); landed arrows are short calls up the chord; STOPPED HORN is the brassy, buzzing stopped
            tone; HORN RIP rips up the scale with its 7 arrows; TALLY-HO is the full hunting fanfare.
As HP drops a fighter's part thins and dirties (high-passed, saturated, notes dropped); the leader gets a doubling.
Bed: D minor at 121.97 BPM (31 bars = 61 s): low-string 16th ostinato, braams every 4 bars, taiko pattern growing
through the fight, high string tremolo in the back half. The count-in bar (taiko + braam stab + brass chop per count,
taiko pickup) drops on FIGHT!; at the K.O. the bed tape-stops with the hit-stop, the horn's tubing clatters, the crowd
roars, the trombone's stinger lands, then its solo (slide glissandi between notes) over a lighter bed; bar 30 builds
(taiko roll, riser, braam swell on the dominant) into the "3" on sample 0. Every hit is snapped to the nearest 16th.
Tails past the loop fold onto the start.

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
KT, KH = NAMES.index('TROMBONE'), NAMES.index('HORN')
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
    """a trombone note gliding up `gl` semitones into the pitch (the slide)"""
    x = note('TROMBONE', m, round(dur, 3), cap or round(dur + 0.3, 2), round(vel, 1))
    return gliss(x, gl, gl_t) if gl else x


def horn(m, dur=0.3, vel=0.8, cap=None):
    return note('HORN', m, round(dur, 3), cap or round(dur + 0.35, 2), round(vel, 1))


def hp_at(k, t):
    return RD.hp_at(k, t)


def colour(x, k, t, drive_k=1.0):
    """the fighter's tone at time t: thinner + dirtier as its HP drops; returns (signal, gain, lead)"""
    hp = hp_at(k, t) / 100.0
    lead = (hp_at(k, t) - hp_at(1 - k, t)) / 100.0
    cut = 70 + 420 * (1 - hp) ** 1.6
    y = A.hp(x, cut)
    drive = 1 + 4.5 * drive_k * (1 - hp) ** 2
    y = np.tanh(drive * y * 1.2) / np.tanh(drive * 1.2) * (1.0 / (1 + 0.15 * (drive - 1)))
    g = 10 ** (float(np.clip(6.0 * lead, -4.0, 3.5)) / 20)
    return y, g, lead


def wah(x, opens, seed=0):
    """plunger mute: a band-pass whose centre opens (500 -> 1800 Hz) over each (t0, t1) span: the "wah" """
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


def snap(seed=0):
    """a finger snap / the glove's flick: a dry click with a little low body"""
    n = int(0.06 * SR)
    t = A.tt(n)
    x = A.bp(np.random.default_rng(seed).normal(0, 1, n), 900, 4500) * np.exp(-t / 0.008)
    return x + 0.5 * np.sin(2 * np.pi * 180 * t) * np.exp(-t / 0.015)


def slide_whistle(dur, f0=520.0, f1=2100.0):
    """FINAL FLICK's wind-up: a slide whistle rising over the beat (cartoon tension)"""
    n = int(dur * SR)
    t = A.tt(n)
    f = f0 * (f1 / f0) ** (t / dur) * (1 + 0.012 * np.sin(2 * np.pi * 6 * t))
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) + 0.15 * A.bp(np.random.default_rng(5).normal(0, 1, n), 1500, 6000)
    return x * np.clip(t / 0.04, 0, 1) * np.clip((dur - t) / 0.02, 0, 1)


def boing(seed=0, f0=190.0, dur=0.45):
    """a spring: the glove bursting out of the bell (a wobbling, falling twang)"""
    n = int(dur * SR)
    t = A.tt(n)
    f = f0 * (1 + 0.35 * np.exp(-t / 0.12) * np.sin(2 * np.pi * 18 * t)) * (1 - 0.25 * t / dur)
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) + 0.3 * np.sin(2 * np.pi * np.cumsum(f * 2.01) / SR)
    return x * np.exp(-t / 0.16) + 0.25 * A.bp(np.random.default_rng(seed).normal(0, 1, n), 2000, 7000) * np.exp(-t / 0.01)


def stopped(x):
    """stopped horn: the hand closes the bell: thin, buzzing, brassy (cuivre)"""
    y = A.hp(x, 500)
    y = np.tanh(3.5 * y / (np.abs(y).max() + 1e-9)) * 0.5
    return y + 0.6 * A.bp(y, 1800, 4200)


# ---------------------------------------------------------------- bed synths (original)
def strings_low(m, dur, vel=1.0):
    """a spiccato cello/bass note: detuned saws, short bow envelope, dark"""
    n = int((dur + 0.08) * SR)
    x = A.saw(A.mtof(m), n, 0.006, 3)
    x = A.lp(x, 700 + 500 * vel) * A.adsr(n, 0.004, 0.06, 0.35, 0.05)
    return x * vel


def tremolo(ms, dur, att=0.4):
    """high string tremolo pad (violins above the horn's register)"""
    n = int(dur * SR)
    t = A.tt(n)
    x = np.zeros(n)
    for m in ms:
        x += A.saw(A.mtof(m), n, 0.004, 3)
    x /= max(1, len(ms))
    x *= 0.75 + 0.25 * np.sign(np.sin(2 * np.pi * 15.0 * t))      # bowed tremolo
    return A.lp(A.hp(x, 900), 5200) * A.adsr(n, att, 0.2, 0.9, min(0.3, dur * 0.3))


def braam(root, dur=1.6, vel=1.0, swell=0.06):
    """the trailer braam: a low detuned saw stack (root, octave, fifth) with a filter swell and grit"""
    n = int(dur * SR)
    t = A.tt(n)
    x = np.zeros(n)
    for m, g in ((root, 1.0), (root + 12, 0.7), (root + 19, 0.35)):
        x += g * A.saw(A.mtof(m), n, 0.01, 4)
    env = np.clip(t / swell, 0, 1) * np.exp(-np.maximum(t - swell, 0) / (dur * 0.45))
    nb = 16
    out = np.zeros(n)
    for k in range(nb):
        a, b = int(k * n / nb), int((k + 1) * n / nb)
        u = (a + b) / 2 / n
        fc = 200 + 1500 * math.exp(-u * 4) * vel
        out[a:b] = A.lp(x[max(0, a - 4096):b], fc)[-(b - a):]
    out = np.tanh(2.2 * out) / np.tanh(2.2)
    return out * env * vel


def taiko(vel=1.0, big=False, seed=0):
    """a big low drum: pitched body sweeping down + a skin slap"""
    dur = 0.9 if big else 0.55
    n = int(dur * SR)
    t = A.tt(n)
    f = 52 + (130 if big else 160) * np.exp(-t / 0.045)
    body = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / (0.32 if big else 0.2))
    slap = A.bp(np.random.default_rng(seed).normal(0, 1, n), 180, 1400) * np.exp(-t / 0.018)
    return (body + 0.45 * slap) * vel


def tick(vel=1.0, seed=1):
    """trailer ticking: a dry wooden click"""
    n = int(0.05 * SR)
    t = A.tt(n)
    return A.bp(np.random.default_rng(seed).normal(0, 1, n), 2500, 7000) * np.exp(-t / 0.006) * vel


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


def clank(m, seed=0):
    """a piece of brass tubing hitting the canvas: a short metallic ring"""
    n = int(0.4 * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = sum(np.sin(2 * np.pi * f * r * t) * np.exp(-t / d) for r, d in ((1.0, 0.12), (2.76, 0.06), (5.4, 0.03)))
    return x + 0.3 * A.bp(np.random.default_rng(seed).normal(0, 1, n), 2000, 8000) * np.exp(-t / 0.005)


ROOT = {'Dm': 38, 'Bb': 34, 'Gm': 43, 'A': 33}
TREM = {'Dm': (74, 77, 81), 'Bb': (74, 77, 82), 'Gm': (74, 79, 82), 'A': (73, 76, 81)}
OST = (0, 0, 12, 0, 0, 12, 0, 7, 0, 0, 12, 0, 7, 0, 12, 10)       # the 16th ostinato's intervals over the bar's root


def bed(mix, b, intensity=1.0, light=False):
    """one bar of the trailer bed (bar index b): low-string ostinato, taiko, braam, high tremolo, ticking"""
    t0 = b * T.BAR
    ch = T.CHORDS[b]
    r = ROOT[ch] + 12 if ROOT[ch] < 36 else ROOT[ch]
    for st, iv in enumerate(OST):                                     # low strings: spiccato 16ths
        if light and st % 2:
            continue
        acc = 1.0 if st % 4 == 0 else 0.7
        mix.place('strings', strings_low(r + iv, S16 * 0.9, acc), t0 + st * S16, 0.11 * (0.8 if light else 1.0))
    # taiko: the trailer pattern, denser as the fight builds
    pat = [(0, 1.0, True), (2, 0.85, False)]
    if intensity > 1.15:
        pat += [(1.5, 0.6, False), (3.5, 0.7, False)]
    if intensity > 1.4 and not light:
        pat += [(2.75, 0.5, False), (3.0, 0.65, False), (3.25, 0.55, False)]
    for bt, v, big in pat:
        mix.place('taiko', taiko(v, big, seed=int(bt * 4) + b), t0 + bt * T.BEAT, 0.42 if not light else 0.30)
    if (b - 1) % 4 == 0 and not light:                                # braam on every loop's downbeat
        mix.place('braam', braam(ROOT[ch] - 12 if ROOT[ch] > 30 else ROOT[ch], 1.7, 0.8 + 0.2 * min(1.0, intensity - 1)),
                  t0, 0.20)
    if intensity > 1.3 or light:                                      # high string tremolo: the back half's tension
        k = 1.0 if light else min(1.0, (intensity - 1.3) / 0.3)
        mix.place('tremolo', tremolo(TREM[ch], T.BAR + 0.05, 0.3), t0, 0.05 * k)
    if intensity > 1.2 and not light:                                 # ticking 8ths
        for st in range(0, 16, 2):
            mix.place('fx', tick(0.6 + 0.4 * (st % 4 == 0), st), t0 + st * S16, 0.04)


def main():
    mix = A.Mix(T.DUR + 4.0)
    # ------------------------------------------------ v3 title card (bar 0): one unison band hit per word
    t1, t2, t3 = 0.004, T.BEAT, 2 * T.BEAT                 # TROMBONE 4 ms in (no click at the seam), VS, FRENCH HORN
    for tw_ in (t1, t2, t3):                               # the band on every word: taiko, a braam stab, a boom, low strings
        mix.place('taiko', taiko(1.0, True, int(tw_ * 100) + 7), tw_, 0.55)
        mix.place('braam', braam(26, 0.55, 1.0, 0.008), tw_, 0.22)
        mix.place('fx', A.thud(0.9, 120, 32, seed=int(tw_ * 50) + 3), tw_, 0.45)
        mix.place('strings', strings_low(38, 0.4, 1.0), tw_, 0.18)
    # TROMBONE: a slide blast: a low brass chord gliding up a fifth into D (the slide shooting out)
    for m, g in ((38, 0.30), (45, 0.26), (50, 0.24), (57, 0.16)):
        mix.place('trombone', bone(m, 0.55, 1.0, 7.0, 0.16, cap=0.95), t1, g)
    # VS: a thunder crack where it splits the divider
    nn = int(1.4 * SR)
    tt_ = A.tt(nn)
    crack = A.bp(np.random.default_rng(91).normal(0, 1, nn), 1500, 9000) * np.exp(-tt_ / 0.035)
    rumble = A.lp(np.random.default_rng(92).normal(0, 1, nn), 160) * np.clip(tt_ / 0.02, 0, 1) * np.exp(-tt_ / 0.5) * 3.0
    mix.place('fx', crack * 0.9 + rumble, t2, 0.40)
    mix.place('fx', A.crash(1.6), t2, 0.10)
    # FRENCH HORN: a hunting call: two horns in fifths, a leap up a fifth (D4 -> A4)
    for dt_, m, d in ((0.0, 62, 0.14), (0.15, 69, 0.42)):
        mix.place('horn', horn(m, d, 1.0), t3 + dt_, 0.32)
        mix.place('horn', horn(m - 7 if m - 7 >= 55 else m - 5, d, 0.85), t3 + dt_, 0.18)
    # beat 4: the hold + a taiko pickup fill as the doors open; a string tremolo under the whole card (never quiet)
    mix.place('tremolo', tremolo((74, 77, 81), 3 * T.BEAT + 0.1, 0.05), 0.0, 0.07)
    for st, v in ((12, 0.5), (13, 0.62), (14, 0.74), (15, 0.86)):
        mix.place('taiko', taiko(v, False, 40 + st), st * S16, 0.36)
    mix.place('fx', A.whoosh(T.T_DOORS[1] - T.T_DOORS[0] + 0.1, 300, 5000), T.T_DOORS[0] - 0.05, 0.16)
    # ------------------------------------------------ count-in (bar 1, on the dominant): taiko + braam stab + brass chop
    for j, (tc, txt) in enumerate(T.COUNT[:3]):
        mix.place('taiko', taiko(1.0, True, 50 + j), tc, 0.5)
        mix.place('braam', braam(33, 0.42, 0.9, 0.01), tc, 0.18)
        mix.place('trombone', bone(45, 0.2, 1.0, 3.0, 0.05), tc, 0.30)
        mix.place('trombone', bone(52, 0.2, 0.9, 3.0, 0.05), tc, 0.18)
        mix.place('horn', horn(69, 0.22, 1.0), tc, 0.22)
        mix.place('horn', horn(73, 0.22, 0.9), tc, 0.14)
        mix.place('strings', strings_low(45, 0.3), tc, 0.14)
    for st, v in ((12, 0.5), (13, 0.65), (14, 0.8), (15, 0.95)):      # the taiko pickup into FIGHT!
        mix.place('taiko', taiko(v, False, 60 + st), T.BAR + st * S16, 0.40)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), T.BAR + 3 * T.BEAT, 0.14)
    mix.place('braam', braam(26, 2.2, 1.0), T.T_FIGHT, 0.30)
    mix.place('taiko', taiko(1.0, True, 70), T.T_FIGHT, 0.55)
    mix.place('fx', A.crash(1.6), T.T_FIGHT, 0.10)
    # ------------------------------------------------ the bed: bars 2 (FIGHT!) .. the K.O.
    bar_ko = int(T_KO // T.BAR)
    for b in range(2, bar_ko + 1):
        bed(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
    mix.place('fx', A.whoosh(3.6, 300, 6000), 44.0 - 3.6, 0.10)
    for k in ('strings', 'taiko', 'braam', 'tremolo'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ TROMBONE: the thrusts are the ostinato
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
            m = T.trombone_at(tq)
            on_beat = slot % 4 == 0
            sig, g, lead = colour(bone(m, 0.15 if on_beat else 0.1, 0.8 if on_beat else 0.6), KT, t, 0.8)
            mix.place('trombone', sig, tq, (0.30 if on_beat else 0.20) * g, pan=-0.2)
            if lead > 0.04:                                           # the leader: an octave below
                mix.place('trombone', colour(bone(m - 12 if m - 12 >= 38 else m, 0.15, 0.7, 0), KT, t)[0], tq,
                          0.14 * g, pan=-0.2)
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'hit' and e[2] == KT and e[7] == 'blade':          # a landed lance: punchy accent, big glissando
            tq = q16(t)
            ct = T.chord_tones(53, 67, T.chord_at(tq))
            m = ct[int(e[8]) % len(ct)]
            sig, g, _ = colour(bone(m, 0.18, 1.0, 5.0, 0.08), KT, t)
            mix.place('trombone', sig, tq, 0.30 * g, pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
            mix.place('fx', A.thud(0.25, 140, 60, seed=int(t * 7)), tq, 0.10)
        elif kind == 'hit' and e[2] == KT and e[7] == 'shockwave':
            mix.place('fx', A.thud(0.4, 120, 45), q16(t), 0.25)
    # ------------------------------------------------ HORN (v5 ECHO CALL): the calls + their delayed repeats
    call_note = {}                                                    # call id -> (time, midi): the echoes repeat it
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'echo' and e[2] == KH:
            j, call = e[3], e[4]
            tq = q16(t)
            pan = float(np.clip(e[5] / 3.5, -0.6, 0.6))
            if j == 0:                                                # the call: a round, mellow horn note on the beat
                m = T.call_at(tq)
                call_note[call] = (tq, m)
                hp = hp_at(KH, t) / 100.0
                if h01(round(tq / S16), 3) < 0.5 * (1 - hp) ** 1.6:   # sparse when hurt
                    continue
                sig, g, lead = colour(horn(m, 0.55 * T.BEAT, 0.8), KH, t)
                mix.place('horn', sig, tq, 0.24 * g, pan=pan)
                if lead > 0.04:                                       # the leader: a horn a sixth below (the call's own harmony)
                    under = T.chord_tones(m - 9, m - 3, T.chord_at(tq))
                    if under:
                        mix.place('horn', colour(horn(under[-1], 0.55 * T.BEAT, 0.65), KH, t)[0], tq, 0.10 * g, pan=pan)
            elif call in call_note:                                   # an echo: the same note, quieter + darker each time
                m = call_note[call][1]
                x = horn(m, 0.4 * T.BEAT, 0.6)
                x = A.lp(x, 2600 / 1.55 ** j)
                mix.place('horn', colour(x, KH, t, 0.5)[0], tq, 0.26 * 0.56 ** j, pan=pan)
        elif kind == 'hit' and e[2] == KH and e[7] == 'echo':         # a blast lands: a brighter stab up the chord
            tq = q16(t)
            ct = T.chord_tones(65, 79, T.chord_at(tq))
            m = ct[(int(e[8]) + int(h01(t, 5) * 3)) % len(ct)]
            pan = float(np.clip(e[5] / 3.5, -0.6, 0.6))
            mix.place('horn', colour(horn(m, 0.1, 0.9), KH, t)[0], tq, 0.12 if e[8] == 0 else 0.08, pan=pan)
            mix.place('fx', A.thud(0.2, 160, 70, seed=int(t * 11)), tq, 0.06 if e[8] == 0 else 0.04, pan=pan)
        elif kind == 'echo_grow':                                     # a new ghost peels off: a soft copper shimmer
            mix.place('fx', A.chime(T.call_at(q16(t)) + 24, 0.6), q16(t), 0.025)
        elif kind == 'flick_wind':                                    # FINAL FLICK: the glove bursts out, winds up
            tq = q16(t)
            mix.place('fx', boing(int(t * 7)), tq, 0.16)
            mix.place('fx', slide_whistle(e[3] - tq), tq, 0.035)
            mix.place('fx', A.whoosh(e[3] - tq, 300, 4000), tq, 0.08)
        elif kind == 'flick':                                         # ... and flicks: THWACK
            ts = e[0]
            mix.place('fx', snap(int(ts * 13)), ts, 0.30)
            mix.place('fx', A.whoosh(0.25, 800, 7000, rise=False), ts - 0.02, 0.14)
            if e[3]:
                mix.place('fx', A.thud(0.7, 140, 34), ts, 0.6)
                mix.place('taiko', taiko(1.0, True, 81), ts, 0.55)
                mix.place('braam', braam(26, 0.9, 1.0, 0.005), ts, 0.22)
        elif kind == 'hit' and e[2] == KH and e[7] == 'flick_rope':   # flung into the ropes: the heavy one
            ts = e[0]
            mix.place('fx', A.thud(0.6, 85, 36), ts, 0.55, pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
            mix.place('taiko', taiko(1.0, True, 83), ts, 0.45)
            mix.place('fx', clank(70, 12), ts, 0.08)
        elif kind == 'block':                                         # the lance bangs off the copper hand
            mix.place('fx', clank(84, int(t * 5)), q16(t), 0.05)
            mix.place('fx', A.thud(0.2, 160, 80), q16(t), 0.05)
        elif kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.05 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            mix.place('fx', clank(79, int(t * 3)), t, 0.04)
        elif kind == 'ability':
            who, name = e[2], e[3]
            tq = q16(t)
            if name == 'SAD TROMBONE':                                # wah wah wah waaah
                ms = (57, 56, 55, 54)
                for j, m in enumerate(ms):
                    d = 0.5 * T.BEAT if j < 3 else 1.6 * T.BEAT
                    x = bone(m, d, 0.9, 1.0 if j else 0.0, 0.04, cap=round(d + 0.35, 2))
                    if j == 3:
                        x = vibrato(x, 0.5, 5.0, 0.2)
                    x = wah(x, [(0.0, min(0.18, d * 0.6))])
                    mix.place('trombone', colour(x, KT, t)[0], tq + j * 0.5 * T.BEAT, 0.40, pan=-0.15)
                mix.place('fx', A.whoosh(0.6, 200, 1500, rise=False), tq, 0.10)
            elif name == 'POWER SLIDE':                               # a long rip up an octave into the riff
                m = T.trombone_at(tq) + 12 if T.trombone_at(tq) + 12 <= 65 else T.trombone_at(tq)
                mix.place('trombone', colour(bone(m, 0.4, 1.0, 12.0, 0.28), KT, t)[0], tq, 0.30)
            elif name == 'PEDAL TONE':                                # the lowest note under the hovering bell
                mix.place('trombone', bone(34, 2 * T.BEAT, 1.0, 2.0, 0.1, cap=round(2 * T.BEAT + 0.3, 2)), tq, 0.45)
                mix.place('fx', A.whoosh(2 * T.BEAT, 150, 1800), tq, 0.10)
            elif name == 'STOPPED HORN':
                m = T.horn_at(tq)
                mix.place('horn', stopped(horn(m, T.BEAT * 1.5, 1.0)), tq, 0.18)
                dur = RD.CFG[who]['abilities']['unlock1']['dur_beats'] * T.BEAT
                n = int(dur * SR)
                buzz = A.bp(np.random.default_rng(4).normal(0, 1, n), 3000, 9000) * A.adsr(n, 0.05, 0.2, 0.5, 0.4)
                mix.place('fx', buzz, tq, 0.02)
            elif name == 'HORN RIP':                                  # a rip up the harmonic series of the bar's root
                root = T.CHORD_ROOT[T.chord_at(tq)]
                ms = [root + iv for iv in (24, 28, 31, 34, 36)]       # partials 4 5 6 7 8
                while max(ms) > 77:
                    ms = [m - 12 for m in ms]
                for j, m in enumerate(ms):
                    last = j == len(ms) - 1
                    x = horn(m, 0.6 if last else 0.07, 0.95)
                    x = gliss(vibrato(x, 0.4, 5.5, 0.15) if last else x, 2.0, 0.03)
                    mix.place('horn', colour(x, KH, t)[0], tq + j * S16 / 2, 0.26 if last else 0.17)
                mix.place('fx', A.whoosh(0.35, 500, 6000), tq, 0.06)
            elif name == 'FINAL FLICK':                               # a horn fanfare over the wind-up
                for bt, m, d in ((0, 62, .25), (.25, 69, .25), (.5, 74, .5)):
                    mix.place('horn', horn(m, d * T.BEAT, 1.0), tq + bt * T.BEAT, 0.28)
                    mix.place('horn', horn(m - 5 if m - 5 >= 57 else m, d * T.BEAT, 0.8), tq + bt * T.BEAT, 0.13)
        elif kind == 'slam':                                          # PEDAL TONE: the bell stomps
            ts = e[0]
            mix.place('taiko', taiko(1.0, True, 90), ts, 0.7)
            mix.place('braam', braam(26, 1.2, 1.0, 0.005), ts, 0.30)
            mix.place('fx', A.thud(0.8, 90, 30), ts, 0.6)
            mix.place('fx', clank(62, 9), ts, 0.10)
            mix.place('trombone', bone(38, 0.4, 1.0, 0), ts, 0.30)
    # ------------------------------------------------ the K.O. moment
    mix.place('taiko', taiko(1.0, True, 99), T_KO, 0.6)
    mix.place('braam', braam(26, 1.6, 1.0, 0.005), T_KO, 0.28)
    mix.place('fx', A.thud(0.9, 120, 30), T_KO, 0.7)
    loser = NAMES[LOSE]
    if loser == 'HORN':                                               # the horn falls apart: a falling wail, clattering tubing
        x = horn(69, 0.9, 1.0, 1.3)
        mix.place('horn', gliss(x, -12, 0.9) * np.linspace(1, 0.3, len(x)), RD.T_HS, 0.22)
        for j, m in enumerate((86, 81, 91, 77, 84, 89)):
            mix.place('fx', clank(m, 20 + j), RD.T_HS + 0.09 * j + 0.04 * h01(j, 4), 0.08, pan=(j - 2.5) / 6)
    else:
        x = bone(57, 0.9, 1.0, 0, cap=1.3)
        mix.place('trombone', gliss(x, -10, 0.9), RD.T_HS, 0.22)
        for j, m in enumerate((80, 75, 85, 71)):
            mix.place('fx', clank(m, 30 + j), RD.T_HS + 0.1 * j, 0.08, pan=(j - 1.5) / 4)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.16)
    # the winner's stinger on "(WINNER) WINS"
    tw = q16(T_KO + 1.3)
    if NAMES[WIN] == 'TROMBONE':
        for m, g in ((38, 0.30), (45, 0.26), (50, 0.24), (57, 0.16)):
            mix.place('solo', bone(m, 1.0, 1.0, 3.0, 0.1, cap=1.5), tw, g)
    else:
        for m, g in ((62, 0.26), (69, 0.22), (74, 0.2)):
            mix.place('solo', horn(m, 1.0, 1.0, 1.5), tw, g)
    mix.place('braam', braam(26, 1.4, 0.8), tw, 0.20)
    mix.place('taiko', taiko(1.0, True, 101), tw, 0.45)
    solo0 = RD.T_SOLO
    mix.place('fx', A.whoosh(solo0 - tw - 0.1, 300, 5000), tw + 0.1, 0.10)
    # ------------------------------------------------ victory: lighter bed + the winner's solo
    b_solo = int(round(solo0 / T.BAR))
    for b in range(b_solo, 30):
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
            mix.place('solo', horn(m, d * T.BEAT * 0.95, 0.95), ts, 0.44)
        prev = m
    # the loser reassembles: its pieces fly home as a rising call, a glass "pop" when it's whole
    run = [62, 65, 69, 70, 74, 77] if loser == 'HORN' else [45, 50, 53, 57, 60, 62]
    for j, m in enumerate(run):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
        if loser == 'HORN':
            mix.place('horn', horn(m, 0.12, 0.7), tj, 0.14)
        else:
            mix.place('trombone', bone(m, 0.12, 0.7), tj, 0.16)
    mix.place('fx', A.chime(95, 0.5) + A.chime(102, 0.5) * 0.5, T.T_GLIDE, 0.12)
    # ------------------------------------------------ bar 30: build into the title slam on sample 0
    t30 = 30 * T.BAR
    st = 0.0
    while t30 + st < T.DUR - 1e-4:                                    # taiko roll: 8ths, then 16ths for the last beat
        v = 0.45 + 0.55 * st / (T.DUR - t30)
        mix.place('taiko', taiko(v, False, 200 + int(st * 10)), t30 + st, 0.36)
        st += T.BEAT / 2 if st < 3 * T.BEAT - 1e-6 else S16
    mix.place('fx', riser(T.DUR - t30), t30, 0.22)
    for bt in range(4):
        mix.place('strings', strings_low((45, 49, 52, 55)[bt], T.BEAT * 0.9, 1.0), t30 + bt * T.BEAT, 0.16)
    sw = braam(33, T.DUR - t30 + 0.012, 0.9, T.DUR - t30)             # a braam swelling right up to the seam
    mix.place('braam', sw * np.linspace(0.3, 1.0, len(sw)), t30, 0.16)
    mix.place('tremolo', tremolo((73, 76, 81), T.DUR - t30 + 0.012, 1.2), t30, 0.06)
    nc = int((T.T_CLOSE + 0.012) * SR)                              # v3: the doors sliding shut into the title slam
    tcl = A.tt(nc)
    doors = A.bp(np.random.default_rng(93).normal(0, 1, nc), 400, 5000) * (tcl / T.T_CLOSE) ** 2
    mix.place('fx', doors, T.DUR - T.T_CLOSE, 0.45)
    # caption landings: soft thuds
    for tl, k, nch in impact_landings(RD.CAPTIONS):
        mix.place('fx', A.thud(), tl, 0.08 if k == 0 else 0.06)
    out, stems = master(mix)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    for k, s in stems.items():
        A.write(os.path.join(HERE, 'check', f'stem_{k}.wav'), s)
    spans = [(0, T.BAR), (T.BAR, T.T_FIGHT), (T.T_FIGHT, 10), (10, 20), (20, 32), (32, 44), (44, T_KO), (T_KO, solo0),
             (solo0, 58), (58, 61)]          # v3: the title card, the count-in, then the fight
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


BUS_GAIN = {'trombone': 1.5, 'horn': 1.25, 'solo': 1.6, 'fx': 1.4, 'strings': 0.85, 'taiko': 1.2, 'braam': 1.0,
            'tremolo': 1.0}
# section automation (dB), applied AFTER the compressor so the limiter can't flatten the build: the count-in at full,
# the early fight pulled back, rising through the final exchange, the victory section loud, the end close to the start
GAIN_PTS = [(0, 0.0), (3.8, 0.0), (4.4, -1.6), (20, -1.5), (30, -0.6), (40, 1.3), (44, 2.3), (50.0, 3.3), (51.5, 2.8),
            (52.6, 3.3), (58.0, 3.3), (60.0, 1.6), (61.02, 0.6)]


def master(mix, thr=0.22, ceiling=0.89):
    """Mix.master with the overflow past the loop (tails, reverb) folded onto the start; NA samples (whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('trombone', 0.35), ('horn', 0.55), ('solo', 0.5), ('fx', 0.3),
                                                ('strings', 0.3), ('tremolo', 0.6), ('braam', 0.3), ('taiko', 0.25)))
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
