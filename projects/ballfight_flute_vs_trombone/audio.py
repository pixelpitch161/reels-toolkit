"""#41 BALLFIGHT #15: Flute vs Trombone - soundtrack (mood A: sinister big top), all synthesised, original.

A dark circus: the bed has NO brass and NO flute of its own. It is a band organ / calliope (breathy, slightly out-of-
tune steam pipes) chunking the minor chords on the off-beats, a concert bass drum on the "oom" (beats 1 + 3), a choked
cymbal on 2 + 4, snare rolls into the sections, a glockenspiel glint; the back half (bar 18 on) adds the march snare's
8ths, a tambourine and a sustained band-organ chord, crashes on the phrase lines. The two fighters carry the music,
both from the honest run, each in its own Pixel Pitch voice (as in #34 / #32):
  TROMBONE  the bass and the smears: every slide thrust (on the beat) is the circus bass (root on 1 + 3, the fifth on
            2 + 4, timeline.bass_note) with a smear up into it; POWER SLIDE's 8ths walk (timeline.walk_note); a landed
            lance is a big smear up the chord; SAD TROMBONE is the clown's plunger "wah wah wah waaah" down to the tonic;
            PEDAL TONE holds the pedal E under the hovering bell, then the stomp. The leader doubles itself an octave up.
  FLUTE     the high melody: while its cobras are up, the sideshow line (timeline.MEL_8THS, E5-F#6, one note per 8th);
            FLUTTER TONGUE flutter-tongues it and adds 16th grace notes; SNAKE DANCE slithers (a chromatic lower-neighbour
            after each note); a hiss-click on every strike, a high accent on every bite; PICCOLO a piercing shriek. The
            leader doubles itself an octave down.
  #41's swat (the lance knocks a cobra flat): a leathery slap + a startled hiss.
As HP drops a fighter's part thins (notes dropped: a splitmix hash, at most 30 %, never two in a row, #37's fix) and
dirties; the leader's part is fuller and louder.
E minor (harmonic) at 133.77 BPM (34 bars = 61 s). Bar 0 is the VS title card (a unison band hit per word: FLUTE's
flurry, VS's thunder crack, TROMBONE's smear; a snare pickup as the doors open), bar 1 the count-in on B7, the band on
FIGHT!. At the K.O. the band tape-stops, the loser falls apart, the crowd roars, the winner's stinger lands and its solo
plays over a lighter big top; the last bar builds (snare roll, riser, the trombone climbing B7) into the title slam on
sample 0. Every hit is snapped to the nearest 16th. Tails past the loop fold onto the start.
Loudness (project rule, Oct 26): ONE master gain + a compressor + a soft limiter, no section automation; the full band
plays the title card, the build comes from the arrangement.

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
KF, KT = NAMES.index('FLUTE'), NAMES.index('TROMBONE')
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames (2860 = 61.013 s): loops sample-perfectly
S16 = T.S16


def q16(t):
    return round(t / S16) * S16


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
    """drops notes as a fighter's HP falls: p = min(0.30, 0.55 (1 - hp)^1.6), never two drops in a row"""

    def __init__(self, seed):
        self.seed, self.last = seed, False

    def keep(self, slot, hp01):
        p = min(0.30, 0.55 * (1 - hp01) ** 1.6)
        drop = (not self.last) and h01(slot, self.seed) < p
        self.last = drop
        return not drop


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


def flute(m, dur=0.1, vel=0.8):
    return note('FLUTE', m, round(max(0.06, dur), 3), round(max(0.06, dur) + 0.18, 2), round(vel, 1))


def breath(dur, seed=5):
    """flute breath: airy band noise (#32)"""
    n = int(dur * SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    return A.bp(x, 1500, 7000) * A.expenv(n, dur * 0.5, 0.01)


def flutter(x, rate=24.0):
    """flutter-tongue: a fast growl on the tone (#32)"""
    tt = A.tt(len(x))
    return x * (0.55 + 0.45 * np.sign(np.sin(2 * np.pi * rate * tt)))


def shriek(dur=1.6, f0=2637.0):
    """PICCOLO: a piercing, wobbling high whistle with a hard edge (#32), on E7"""
    n = int(dur * SR)
    tt = A.tt(n)
    f = f0 * (1 + 0.012 * np.sin(2 * np.pi * 7.5 * tt)) * (1 + 0.06 * np.clip(tt / 0.08, 0, 1))
    ph = 2 * np.pi * np.cumsum(f) / SR
    x = np.sin(ph) + 0.3 * np.sin(2 * ph) + 0.12 * np.sin(3 * ph)
    x = np.tanh(1.8 * x) * np.clip(tt / 0.02, 0, 1) * np.exp(-tt / (dur * 0.6))
    return x + 0.25 * breath(dur, 21)


def hp_at(k, t):
    return RD.hp_at(k, t)


def colour(x, k, t, drive_k=1.0, low=False):
    """the fighter's tone at time t: thinner + dirtier as its HP drops; returns (signal, gain, lead). The bass trombone
    line (`low`) keeps its fundamental: it gets raspy and a little thinner, never high-passed away"""
    hp = hp_at(k, t) / 100.0
    lead = (hp_at(k, t) - hp_at(1 - k, t)) / 100.0
    cut = (30 + 70 * (1 - hp) ** 1.6) if low else (90 + 520 * (1 - hp) ** 1.6)
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


def clank(m, seed=0):
    """brass hitting the canvas / the lance banging: a short metallic ring (#34)"""
    n = int(0.4 * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = sum(np.sin(2 * np.pi * f * r * t) * np.exp(-t / d) for r, d in ((1.0, 0.12), (2.76, 0.06), (5.4, 0.03)))
    return x + 0.3 * A.bp(np.random.default_rng(seed).normal(0, 1, n), 2000, 8000) * np.exp(-t / 0.005)


def hiss(dur=0.09, seed=0, lo=3500, hi=10000, dec=0.03):
    n = int(dur * SR)
    tt_ = A.tt(n)
    return A.bp(np.random.default_rng(seed).normal(0, 1, n), lo, hi) * np.exp(-tt_ / dec)


def swat_sound(seed=0):
    """#41: the lance slaps a cobra flat: a leathery slap (a short low-mid smack) + a startled, falling hiss"""
    n = int(0.3 * SR)
    tt_ = A.tt(n)
    rng = np.random.default_rng(seed)
    smack = A.bp(rng.normal(0, 1, n), 600, 3500) * np.exp(-tt_ / 0.012)
    body = np.sin(2 * np.pi * np.cumsum(260 * np.exp(-tt_ / 0.04) + 120) / SR) * np.exp(-tt_ / 0.05)
    hs = A.bp(rng.normal(0, 1, n), 2500, 9000) * np.clip((tt_ - 0.02) / 0.02, 0, 1) * np.exp(-np.maximum(tt_ - 0.04, 0) / 0.07)
    return smack * 1.2 + body * 0.6 + hs * 0.5


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


# ---------------------------------------------------------------- the big top (original): band organ, drums, glock
def calliope(ms, dur, vel=1.0, seed=0, att=0.008, vib=0.004):
    """band organ / calliope: breathy steam pipes (sine + a little 2nd / 3rd), each pipe a hair out of tune, a fast
    shallow vibrato, chiff noise on the attack"""
    n = int((dur + 0.06) * SR)
    t = A.tt(n)
    rng = np.random.default_rng(seed)
    x = np.zeros(n)
    for j, m in enumerate(ms):
        f = A.mtof(m) * (1 + rng.uniform(-0.005, 0.005))
        fv = f * (1 + vib * np.sin(2 * np.pi * 6.3 * t + 1.7 * j))
        ph = 2 * np.pi * np.cumsum(fv) / SR
        x += np.sin(ph) + 0.28 * np.sin(2 * ph) + 0.10 * np.sin(3 * ph)
    x /= max(1, len(ms))
    chiff = A.bp(rng.normal(0, 1, n), 1200, 5000) * np.exp(-t / 0.015) * 0.25
    air = A.bp(rng.normal(0, 1, n), 800, 3000) * 0.04
    env = np.clip(t / att, 0, 1) * np.clip((dur + 0.06 - t) / 0.06, 0, 1)
    return (x + chiff + air) * env * vel


def bass_drum(vel=1.0):
    """the concert bass drum ("oom"): a soft boom, over in ~0.2 s and high-passed so the trombone's bass stays on top"""
    x = A.kick(0.7, 0.2, 105, 50)[:int(0.36 * SR)]
    return A.hp(x, 46) * vel


def choke(vel=1.0, seed=0):
    """the "pah" cymbal: a crash choked by hand after ~0.12 s"""
    n = int(0.22 * SR)
    t = A.tt(n)
    x = A.hp(np.random.default_rng(seed).normal(0, 1, n), 4200) * np.exp(-t / 0.05)
    x += 0.15 * np.sin(2 * np.pi * 4130 * t) * np.exp(-t / 0.06)
    return x * np.clip((0.2 - t) / 0.06, 0, 1) * vel


def snare(vel=1.0):
    return A.snare(0.13) * vel


def roll(dur, v0=0.4, v1=1.0, rate=None, seed=0):
    """a snare roll (32nds), crescendo from v0 to v1"""
    rate = rate or (S16 / 2)
    n = int((dur + 0.15) * SR)
    out = np.zeros(n)
    k = 0
    while k * rate < dur - 1e-6:
        v = v0 + (v1 - v0) * (k * rate / max(dur, 1e-6))
        s = snare(0.6 + 0.4 * ((k % 2) == 0)) * v
        a = int(k * rate * SR)
        b = min(n, a + len(s))
        out[a:b] += s[:b - a]
        k += 1
    return out


def tambourine(vel=1.0, seed=0):
    n = int(0.12 * SR)
    t = A.tt(n)
    x = A.bp(np.random.default_rng(seed).normal(0, 1, n), 5500, 12000) * np.exp(-t / 0.04)
    x += 0.2 * sum(np.sin(2 * np.pi * f * t) for f in (6300, 7900, 9100)) * np.exp(-t / 0.05)
    return x * vel


def glock(m, dur=0.9):
    return A.chime(m, dur)


VOICE = {'Em': (64, 67, 71), 'Am': (64, 69, 72), 'B7': (63, 66, 69, 71), 'C': (64, 67, 72)}   # E4-C5: under the flute
ORGAN = {'Em': (59, 64, 67), 'Am': (60, 64, 69), 'B7': (59, 63, 69), 'C': (60, 64, 67)}       # the back half's held chord
COUNTER = {'Em': 71, 'Am': 72, 'B7': 71, 'C': 72}                 # unused high voice reserved for the flute (kept clear)


def bed(mix, b, intensity=1.0, light=False):
    """one bar of the big top (bar index b): bass drum on 1 + 3, choked cymbal on 2 + 4, the calliope's off-beat chords;
    the back half (`intensity` > 1.25) adds the march snare 8ths, tambourine 16ths, the held band-organ chord and a
    glockenspiel glint"""
    t0 = b * T.BAR
    ch = T.CHORDS[b]
    big = intensity > 1.25 and not light
    for bt in range(4):
        tb = t0 + bt * T.BEAT
        if bt in (0, 2):
            mix.place('drums', bass_drum(1.0), tb, 0.62 if not light else 0.50)
        else:
            mix.place('drums', choke(0.9, 10 * b + bt), tb, 0.16)
            mix.place('drums', snare(0.8), tb, 0.10 if not big else 0.16)
        mix.place('calliope', calliope(VOICE[ch], 0.13, 1.0, seed=b * 4 + bt), tb + 0.5 * T.BEAT, 0.20,
                  pan=0.15 * (1 if bt % 2 else -1))
        if big:
            for half in (0.0, 0.5):                              # the march snare's 8ths, accent on the beat
                if not (half == 0.0 and bt in (1, 3)):
                    mix.place('drums', snare(0.55 + 0.25 * (half == 0.0)), tb + half * T.BEAT, 0.07)
            for q in range(4):
                mix.place('drums', tambourine(0.6 + 0.4 * (q == 2), 40 + q + bt), tb + q * S16, 0.05, pan=0.3)
    if big:
        mix.place('calliope', calliope(ORGAN[ch], T.BAR - 0.05, 0.8, seed=200 + b, att=0.06, vib=0.006), t0, 0.08)
        if b % 2 == 0:
            mix.place('glock', glock(88 + (0, 5, 7, 4)[(b // 2) % 4] + 12, 0.8), t0, 0.035)
    elif light:
        mix.place('calliope', calliope(ORGAN[ch], T.BAR - 0.05, 0.8, seed=300 + b, att=0.08, vib=0.006), t0, 0.07)


def main():
    mix = A.Mix(T.DUR + 4.0)
    # ------------------------------------------------ bar 0: the VS title card, one unison band hit per word
    t1, t2, t3 = 0.004, T.BEAT, 2 * T.BEAT                 # FLUTE 4 ms in (no click at the seam), VS, TROMBONE
    for j, tw_ in enumerate((t1, t2, t3)):
        mix.place('drums', bass_drum(1.0), tw_, 0.70)
        mix.place('drums', A.crash(1.0), tw_, 0.06)
        mix.place('calliope', calliope(VOICE['Em'], 0.30, 1.0, seed=700 + j), tw_, 0.22)
        mix.place('fx', A.thud(0.8, 120, 34, seed=int(tw_ * 50) + 3), tw_, 0.30)
    for j, m in enumerate((76, 79, 83, 84, 86, 87, 88)):  # FLUTE: a shrieking flurry up to a flutter-tongued high E
        mix.place('flute', flute(m, 0.06, 1.0), t1 + j * S16 / 2, 0.30, pan=-0.15)
    mix.place('flute', flutter(flute(88, 0.5, 1.0), 26), t1 + 3.5 * S16, 0.32, pan=-0.15)
    mix.place('flute', breath(0.4, 81), t1, 0.10, pan=-0.15)
    nn = int(1.4 * SR)                                     # VS: a thunder crack where it splits the divider
    tt_ = A.tt(nn)
    crack = A.bp(np.random.default_rng(91).normal(0, 1, nn), 1500, 9000) * np.exp(-tt_ / 0.035)
    rumble = A.lp(np.random.default_rng(92).normal(0, 1, nn), 160) * np.clip(tt_ / 0.02, 0, 1) * np.exp(-tt_ / 0.5) * 3.0
    mix.place('fx', crack * 0.9 + rumble, t2, 0.36)
    mix.place('trombone', bone(40, 0.7, 1.0, 0, cap=1.0), t3, 0.42)        # TROMBONE: the low E + a big smear up to E3
    mix.place('trombone', bone(52, 0.6, 1.0, 12.0, 0.26, cap=0.95), t3, 0.34)
    mix.place('calliope', calliope(ORGAN['Em'], 3 * T.BEAT, 0.8, seed=710, att=0.03), 0.0, 0.08)   # never quiet
    mix.place('drums', roll(T.BEAT * 0.95, 0.3, 0.9), 3 * T.BEAT, 0.20)    # beat 4: a snare roll as the doors open
    mix.place('fx', A.whoosh(T.T_DOORS[1] - T.T_DOORS[0] + 0.1, 300, 5000), T.T_DOORS[0] - 0.05, 0.14)
    # ------------------------------------------------ bar 1: the count-in on the dominant (B7)
    for j, (tc, txt) in enumerate(T.COUNT[:3]):
        mix.place('drums', bass_drum(1.0), tc, 0.62)
        mix.place('drums', choke(1.0, 50 + j), tc, 0.14)
        mix.place('calliope', calliope(VOICE['B7'], 0.18, 1.0, seed=720 + j), tc, 0.20)
        mix.place('trombone', bone((47, 42, 47)[j], 0.25, 0.95, 3.0, 0.05), tc, 0.40)
        mix.place('flute', flute((83, 87, 90)[j], 0.12, 0.9), tc, 0.26, pan=-0.2)
    mix.place('calliope', calliope(ORGAN['B7'], T.BAR, 0.7, seed=730, att=0.2) * np.linspace(0.6, 1.0, int((T.BAR + 0.06) * SR)),
              T.BAR, 0.08)
    for bt in range(4):                                              # the calliope chunks the off-beats of the count too
        mix.place('calliope', calliope(VOICE['B7'], 0.13, 1.0, seed=740 + bt), T.BAR + (bt + 0.5) * T.BEAT, 0.20)
    t4 = T.BAR + 3 * T.BEAT                                          # beat 4: the pickup - a band hit + a smear up
    mix.place('drums', bass_drum(1.0), t4, 0.55)
    mix.place('calliope', calliope(VOICE['B7'], 0.18, 1.0, seed=745), t4, 0.18)
    mix.place('trombone', bone(47, 0.4, 1.0, 7.0, 0.3), t4, 0.36)
    mix.place('flute', flute(87, 0.25, 0.9), t4, 0.20, pan=-0.2)
    mix.place('drums', roll(T.BEAT, 0.5, 1.0), t4, 0.30)              # the roll into FIGHT!
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), T.BAR + 3 * T.BEAT, 0.12)
    mix.place('drums', A.crash(1.8), T.T_FIGHT, 0.12)               # the band comes in
    mix.place('fx', A.thud(0.7, 110, 34), T.T_FIGHT, 0.34)
    # ------------------------------------------------ the big top: bar 2 (FIGHT!) .. the K.O.; a roll into bar 18
    bar_ko = int(T_KO // T.BAR)
    for b in range(2, bar_ko + 1):
        bed(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
    for b in range(2, bar_ko + 1, 8):                                  # every 8-bar phrase: a crash on its first beat
        mix.place('drums', A.crash(1.6), b * T.BAR, 0.08)
    t18 = 18 * T.BAR
    mix.place('drums', roll(2 * T.BEAT, 0.3, 1.0), t18 - 2 * T.BEAT, 0.20)
    mix.place('fx', riser(T.BAR, over=0.0, seed=12) * 0.6, t18 - T.BAR, 0.08)
    mix.place('drums', A.crash(2.0), t18, 0.12)
    for k in ('drums', 'calliope', 'glock'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ TROMBONE: the bass line (a smear into every thrust)
    thin_t = Thinner(7)
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
            if not thin_t.keep(slot, hp):
                continue
            hasted = float(RUN['HA'][RD.sim_i(t), KT]) > 0
            on_beat = slot % 4 == 0
            m = T.walk_note(tq) if hasted else T.bass_note(tq)
            dur = 0.17 if hasted else 0.30
            sig, g, lead = colour(bone(m, dur, 0.85 if on_beat else 0.7, 3.0 if on_beat else 1.5, 0.05), KT, t, 0.8,
                                  low=True)
            mix.place('trombone', sig, tq, (0.44 if on_beat else 0.34) * g, pan=0.15)
            if lead > 0.04 and on_beat:                               # the leader: doubled an octave up, fuller
                mix.place('trombone', colour(bone(m + 12, dur, 0.6, 2.0, 0.05), KT, t)[0], tq, 0.14 * g, pan=0.2)
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'hit' and e[2] == KT and e[7] == 'blade':          # a landed lance: a big smear up the chord
            tq = q16(t)
            ct = T.chord_tones(52, 62, T.chord_at(tq + 0.01))
            m = ct[int(e[8]) % len(ct)]
            sig, g, _ = colour(bone(m, 0.22, 1.0, 7.0, 0.11), KT, t)
            mix.place('trombone', sig, tq, 0.26 * g, pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
            mix.place('fx', A.thud(0.22, 150, 70, seed=int(t * 7)), tq, 0.08)
        elif kind == 'hit' and e[2] == KT and e[7] == 'shockwave':
            mix.place('fx', A.thud(0.4, 120, 45), q16(t), 0.22)
        elif kind == 'swat':                                          # #41: the lance slaps a cobra flat
            tq = q16(t)
            mix.place('fx', swat_sound(int(t * 31)), tq, 0.16, pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
            mix.place('flute', hiss(0.16, int(t * 17), 3000, 9000, 0.05), tq + 0.02, 0.05, pan=-0.2)
        elif kind == 'block':
            mix.place('fx', clank(84, int(t * 5)), q16(t), 0.06)
        elif kind == 'parry':
            mix.place('fx', clank(88, int(t * 9)), t, 0.04)
        elif kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.05 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            mix.place('fx', clank(79, int(t * 3)), t, 0.04)
        elif kind == 'ability' and e[2] == KT:
            name = e[3]
            tq = q16(t)
            if name == 'SAD TROMBONE':                                # the clown: wah wah wah waaah, down to the tonic
                for j, m in enumerate((55, 54, 53, 52)):
                    d = 0.5 * T.BEAT if j < 3 else 1.6 * T.BEAT
                    x = bone(m, d, 0.9, 1.0 if j else 0.0, 0.04, cap=round(d + 0.35, 2))
                    if j == 3:
                        x = vibrato(x, 0.5, 5.0, 0.2)
                    x = wah(x, [(0.0, min(0.18, d * 0.6))])
                    mix.place('trombone', colour(x, KT, t)[0], tq + j * 0.5 * T.BEAT, 0.42, pan=0.1)
                mix.place('fx', A.whoosh(0.6, 200, 1500, rise=False), tq, 0.08)
            elif name == 'POWER SLIDE':                               # a long rip up two octaves into the walk
                mix.place('trombone', colour(bone(64, 0.45, 1.0, 24.0, 0.32), KT, t)[0], tq, 0.30)
            elif name == 'PEDAL TONE':                                # the pedal E under the hovering bell
                mix.place('trombone', bone(28, 2 * T.BEAT, 1.0, 2.0, 0.1, cap=round(2 * T.BEAT + 0.3, 2)), tq, 0.40)
                mix.place('trombone', bone(40, 2 * T.BEAT, 0.9, 0.0, cap=round(2 * T.BEAT + 0.3, 2)), tq, 0.30)
                mix.place('fx', A.whoosh(2 * T.BEAT, 150, 1800), tq, 0.10)
        elif kind == 'slam' and e[2] == KT:                           # PEDAL TONE: the bell stomps
            ts = e[0]
            mix.place('drums', bass_drum(1.0), ts, 0.8)
            mix.place('fx', A.thud(0.8, 90, 30), ts, 0.5)
            mix.place('fx', clank(64, 9), ts, 0.10)
            mix.place('trombone', bone(40, 0.5, 1.0, 0), ts, 0.36)
            mix.place('drums', A.crash(1.6), ts, 0.10)
    # ------------------------------------------------ FLUTE: the high melody while its cobras are up
    rise = RD.CFG[KF]['weapon']['rise'] * T.BEAT
    t_up = min([b['t_drop'] for b in RD.BASKETS.values()] + [99.0]) + rise
    thin_f = Thinner(5)
    k0, k1 = int(math.ceil(t_up / (2 * S16))), int(T_KO / (2 * S16))
    for s8 in range(k0, k1 + 1):
        tq = s8 * 2 * S16
        m = T.mel_at(tq)
        hp = hp_at(KF, tq) / 100.0
        if not thin_f.keep(s8, hp):
            continue
        hasted = float(RUN['HA'][RD.sim_i(tq), KF]) > 0
        dancing = any(a_ <= tq < b_ for a_, b_ in RD.DANCES)
        dur = 0.2 if not (hasted or dancing) else 0.11
        x = flute(m, dur, 0.8)
        if hasted:
            x = flutter(x)
        sig, g, lead = colour(x, KF, tq, 0.9)
        mix.place('flute', sig, tq, 0.26 * g, pan=-0.2)
        if hasted or dancing:                                         # the 16th between: a grace note
            m2 = m - 1 if dancing else T.mel_at(tq + 2 * S16)        # SNAKE DANCE slithers a semitone down
            if m2 != m or dancing:
                x2 = flute(m2, 0.1, 0.7)
                if hasted:
                    x2 = flutter(x2)
                mix.place('flute', colour(x2, KF, tq)[0], tq + S16, 0.20 * g, pan=-0.2)
        if (1 - hp) > 0.3:
            mix.place('flute', breath(0.1, s8), tq, 0.07 * (1 - hp), pan=-0.2)
        if lead > 0.04:                                               # the leader: a thin octave double underneath
            mix.place('flute', colour(flute(m - 12, dur, 0.6), KF, tq)[0], tq, 0.08 * g, pan=-0.25)
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'strike' and e[2] == KF:                           # a hiss-click on every strike (#32)
            tq = q16(t)
            n = int(0.09 * SR)
            tt_ = A.tt(n)
            click = np.sin(2 * np.pi * 2800 * tt_) * np.exp(-tt_ / 0.004)
            mix.place('fx', hiss(0.09, int(t * 50)) * 0.8 + click * 0.5, tq, 0.08,
                      pan=float(np.clip(math.cos(e[4]) * 0.4, -0.5, 0.5)))
        elif kind == 'hit' and e[2] == KF and e[7] == 'fang':         # a bite lands: a high accent
            tq = q16(t)
            ct = T.chord_tones(84, 93, T.chord_at(tq + 0.01))
            m = ct[int(e[8]) % len(ct)] if ct else 88
            sig, g, _ = colour(flute(m, 0.07, 1.0), KF, t, 0.9)
            mix.place('flute', sig, tq, 0.16 * g, pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'basket' and e[2] == KF:                         # a basket lands: a soft wicker thump
            mix.place('fx', A.thud(0.18, 220, 120, seed=int(t * 9)), t + 0.05, 0.07)
        elif kind == 'hit' and e[2] == KF and e[7] == 'wave':
            mix.place('fx', A.thud(0.45, 140, 60), q16(t), 0.26)
        elif kind == 'ability' and e[2] == KF:
            name = e[3]
            tq = q16(t)
            if name == 'FLUTTER TONGUE':
                mix.place('flute', flutter(flute(88, 0.5, 0.9), 26), tq, 0.20, pan=-0.2)
            elif name == 'SNAKE DANCE':                               # a rattle of hisses as the cobras leave the baskets
                for j in range(4):
                    mix.place('flute', hiss(0.12, 70 + j, 4000, 11000, 0.05), tq + j * S16, 0.09, pan=0.3 * (j % 2 * 2 - 1))
            elif name == 'PICCOLO':                                   # the highest note: a piercing shriek on E7
                mix.place('flute', shriek(1.7), tq, 0.13, pan=-0.2)
                mix.place('flute', colour(flute(93, 0.9, 1.0), KF, t, 2.0)[0], tq, 0.16)
    # ------------------------------------------------ the K.O. moment
    mix.place('drums', A.crash(2.4), T_KO, 0.16)
    mix.place('fx', A.thud(0.9, 120, 30), T_KO, 0.6)
    loser = NAMES[LOSE]
    if loser == 'FLUTE':                                              # the flute falls apart: a broken run down (#32)
        for j, m in enumerate((93, 89, 84, 80, 77, 74)):
            mix.place('flute', flute(m, 0.08, 0.8), RD.T_HS + 0.09 * j, 0.20, pan=(j - 2.5) / 6)
            mix.place('flute', breath(0.08, 40 + j), RD.T_HS + 0.09 * j, 0.06)
    else:                                                             # the slide falls apart: a falling wail, clattering tubing
        x = bone(52, 0.9, 1.0, 0, cap=1.3)
        mix.place('trombone', gliss(x, -10, 0.9), RD.T_HS, 0.26)
        for j, m in enumerate((80, 75, 85, 71)):
            mix.place('fx', clank(m, 30 + j), RD.T_HS + 0.1 * j, 0.08, pan=(j - 1.5) / 4)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.15)
    tw = q16(T_KO + 1.3)                                              # the winner's stinger on "(WINNER) WINS"
    if NAMES[WIN] == 'TROMBONE':
        mix.place('solo', bone(40, 1.0, 1.0, 0, cap=1.5), tw, 0.40)
        for m, g in ((52, 0.30), (55, 0.24), (59, 0.20)):
            mix.place('solo', bone(m, 1.0, 1.0, 5.0, 0.12, cap=1.5), tw, g)
    else:
        for j, m in enumerate((76, 79, 83, 88)):
            mix.place('solo', flute(m, 0.12 if j < 3 else 0.8, 1.0), tw + j * S16 / 2, 0.30)
    mix.place('calliope', calliope(VOICE['Em'], 0.35, 1.0, seed=800), tw, 0.22)
    mix.place('drums', bass_drum(1.0), tw, 0.65)
    mix.place('drums', A.crash(1.8), tw, 0.10)
    solo0 = RD.T_SOLO
    mix.place('fx', A.whoosh(max(0.2, solo0 - tw - 0.1), 300, 5000), tw + 0.1, 0.08)
    # ------------------------------------------------ victory: a lighter big top + the winner's solo
    b_solo = int(round(solo0 / T.BAR))
    for b in range(b_solo, 33):                                       # the band plays on under the solo (no march snare)
        bed(mix, b, intensity=1.1)
        mix.place('calliope', calliope(ORGAN[T.CHORDS[b]], T.BAR - 0.05, 0.8, seed=300 + b, att=0.08, vib=0.006),
                  b * T.BAR, 0.07)
    t_end_solo = solo0 + max(bt + d for bt, m, d in T.SOLO[NAMES[WIN]]) * T.BEAT
    if NAMES[WIN] == 'TROMBONE':                                      # then the champion goes back to the bass line
        for j in range(int(math.ceil(t_end_solo / T.BEAT)), int(33 * T.BAR / T.BEAT + 0.5)):
            tb = j * T.BEAT
            mix.place('solo', bone(T.bass_note(tb), 0.3, 0.85, 3.0, 0.05), tb, 0.40)
    prev = None
    for bt, m, d in T.SOLO[NAMES[WIN]]:
        ts = solo0 + bt * T.BEAT
        if ts > T.T_REASSEMBLE - 0.05:                                 # a late K.O. leaves less room: stop at the reassembly
            break
        if NAMES[WIN] == 'TROMBONE':                                  # the slide glides from the last note to this one
            gl = float(np.clip(m - prev, -9, 9)) if prev is not None else 5.0
            mix.place('solo', bone(m, d * T.BEAT * 0.95, 0.95, gl, 0.08, cap=round(d * T.BEAT + 0.35, 2)), ts, 0.50)
        else:
            mix.place('solo', flute(m, d * T.BEAT * 0.9, 0.9), ts, 0.50)
        prev = m
    # the loser reassembles: its pieces fly home as a rising run, a glass "pop" when it's whole
    run = [76, 79, 83, 84, 88, 91] if loser == 'FLUTE' else [40, 45, 47, 52, 55, 59]
    for j, m in enumerate(run):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
        if loser == 'FLUTE':
            mix.place('flute', flute(m, 0.08, 0.7), tj, 0.16)
        else:
            mix.place('trombone', bone(m, 0.12, 0.7), tj, 0.20)
    mix.place('fx', A.chime(95, 0.5) + A.chime(100, 0.5) * 0.5, T.T_GLIDE, 0.10)
    # ------------------------------------------------ the last bar (33): build into the title slam on sample 0
    t33 = 33 * T.BAR
    mix.place('drums', roll(T.DUR - t33 + 0.01, 0.4, 1.0), t33, 0.20)
    for bt in range(4):
        mix.place('drums', bass_drum(0.95), t33 + bt * T.BEAT, 0.56)
        mix.place('trombone', bone((47, 51, 54, 57)[bt], 0.35, 0.9, 3.0, 0.06), t33 + bt * T.BEAT, 0.42)  # climbing B7
        mix.place('calliope', calliope(VOICE['B7'], 0.13, 1.0, seed=900 + bt), t33 + (bt + 0.5) * T.BEAT, 0.20)
    mix.place('fx', riser(T.DUR - t33), t33, 0.18)
    sw = calliope(ORGAN['B7'], T.DUR - t33 + 0.012, 0.9, seed=950, att=T.DUR - t33, vib=0.008)   # swelling to the seam
    mix.place('calliope', sw, t33, 0.10)
    nc = int((T.T_CLOSE + 0.012) * SR)                              # the doors sliding shut into the title slam
    tcl = A.tt(nc)
    doors = A.bp(np.random.default_rng(93).normal(0, 1, nc), 400, 5000) * (tcl / T.T_CLOSE) ** 2
    mix.place('fx', doors, T.DUR - T.T_CLOSE, 0.40)
    for tl, k, nch in impact_landings(RD.CAPTIONS):                 # caption landings: soft thuds
        mix.place('fx', A.thud(), tl, 0.07 if k == 0 else 0.05)
    out, stems = master(mix)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    for k, s in stems.items():
        A.write(os.path.join(HERE, 'check', f'stem_{k}.wav'), s)
    spans = [(0, T.BAR), (T.BAR, T.T_FIGHT), (T.T_FIGHT, 10), (10, 20), (20, 32), (32, 44), (44, T_KO), (T_KO, solo0),
             (solo0, T.T_REASSEMBLE), (T.T_REASSEMBLE, 61)]
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


BUS_GAIN = {'trombone': 1.8, 'flute': 1.7, 'solo': 1.5, 'fx': 1.2, 'drums': 1.0, 'calliope': 1.2, 'glock': 1.0}


def master(mix, thr=0.15, ceiling=0.89):
    """ONE master gain (no section automation: consistent loudness from frame 0) + a compressor and a soft limiter;
    the overflow past the loop (tails, reverb) folds onto the start; NA samples (whole AAC frames) (#40)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('trombone', 0.2), ('flute', 0.45), ('solo', 0.4), ('fx', 0.25),
                                                ('calliope', 0.45), ('glock', 0.5), ('drums', 0.15)))
    wet = A.reverb(send, dur=2.0, decay=0.55) * 0.22                 # a big tent
    full = sum(buses.values()) + wet

    def fold(x):
        y = x[:NS].copy()
        tail = x[NS:]
        y[:len(tail)] += tail[:NS]
        return y
    m = fold(full)
    from scipy.signal import lfilter
    env = np.sqrt(lfilter([0.002], [1, -0.998], (np.concatenate([m[-SR:], m]) ** 2).mean(1)))[SR:]
    gain = np.where(env > thr, (thr / np.maximum(env, 1e-9)) ** 0.62, 1.0)[:, None]
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
