"""#42 BALLFIGHT #16: Viola vs Violin - soundtrack (mood A: orchestral drill), all synthesised, original.

A UK drill beat (a sliding 808, skittering hi-hat rolls in 16th triplets and 32nds, the half-time snare / clap on beat
3, rim clicks) under a dark staccato string bed of CELLOS + BASSES only (the Pixel Pitch cello's voice, the basses a
bowed saw an octave down): no violin or viola in the bed. The two fighters carry the music, both from the honest run,
each in its own Pixel Pitch voice:
  VIOLIN  the melody, as in #28: every quarter turn of its bow plays the line's note at that moment (timeline.melody_at,
          snapped to the 16th), so the slow early bow plays a few long strokes and the fast late bow (TREMOLO) frantic
          spiccato; landed bow hits are accents climbing the scale, PIZZICATO plucks, SCREECH the beginner's squeal.
  VIOLA   the inner harmony, a fifth (or so) under the line (timeline.viola_inner), in its new darker voice: the
          counting rests are a soft muted pluck on 1-2-3; every whole note is one long sustained viola note (a whole
          note: a bar) with a heavy bow attack; DOUBLE STOP plays two strings at once; a crush lands as a low string
          crunch + thud. MELODY AT LAST: the arrangement flips for its 2 bars: the viola sings the lead
          (timeline.VIOLA_LEAD, fitted to the chords), the violin drops to soft harmony under it. VIOLA JOKE: a rimshot
          (ba-dum-tss), the whole crowd bursts out laughing, then the biggest note: the open C + G, fortissimo.
As HP drops a fighter's part thins (a splitmix hash, at most 30 %, never two in a row, #37's fix) and dirties; the
leader's part is fuller and louder.
C minor at 141.64 BPM (36 bars = 61 s). Bar 0 is the VS title card (a unison band hit per word: VIOLA's dark chord, VS's
thunder crack, VIOLIN's flourish; a hat roll as the doors open), bar 1 the count-in on G, the drop on FIGHT!. At the
K.O. the beat tape-stops, the loser falls apart, the crowd roars, the winner's stinger lands and its solo plays over a
lighter drill beat; the last bar builds (roll, riser, the cellos climbing G) into the title slam on sample 0. Every hit is
snapped to the nearest 16th. Tails past the loop fold onto the start.
Loudness (project rule, Oct 26): ONE master gain + a compressor + a soft limiter, no section automation; the full drill
beat plays the title card, the build comes from the arrangement.

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
KA, KV = NAMES.index('VIOLA'), NAMES.index('VIOLIN')
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames: loops sample-perfectly
S16 = T.S16


def q16(t):
    return round(t / S16) * S16


def h01(*xs):
    """deterministic splitmix hash in [0, 1) (#37's fix)"""
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


def bow_note(m, n16=2, vel=0.8):
    """a bowed violin note held for n16 sixteenths (the bow decides the length) (#28)"""
    dur = max(0.07, n16 * S16 * 0.92)
    return note('VIOLIN', m, round(dur, 3), round(dur + 0.25, 2), round(vel, 1))


def viola(m, dur, vel=0.85, accent=0.0):
    """a bowed viola note (the new voice) with an optional heavy bow attack (rosin crunch)"""
    x = note('VIOLA', m, round(dur, 3), round(dur + 0.3, 2), round(vel, 1))
    if accent > 0:
        s = scratch(0.07, int(m * 7))[:len(x)]
        x = x.copy()
        x[:len(s)] += accent * 0.25 * s
    return x


def hp_at(k, t):
    return RD.hp_at(k, t)


def colour(x, k, t, drive_k=1.0):
    """the fighter's tone at time t: thinner + dirtier as its HP drops; returns (signal, gain, lead)"""
    hp = hp_at(k, t) / 100.0
    lead = (hp_at(k, t) - hp_at(1 - k, t)) / 100.0
    cut = 70 + 300 * (1 - hp) ** 1.6 if k == KA else 90 + 520 * (1 - hp) ** 1.6   # the viola keeps its low strings
    y = A.hp(x, cut)
    drive = 1 + 4.5 * drive_k * (1 - hp) ** 2
    y = np.tanh(drive * y * 1.2) / np.tanh(drive * 1.2) * (1.0 / (1 + 0.15 * (drive - 1)))
    g = 10 ** (float(np.clip(6.0 * lead, -4.0, 3.5)) / 20)
    return y, g, lead


def scratch(dur, seed=5):
    """bow scratch: rosin noise, band-limited, gritty (#28)"""
    n = int(dur * SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    x = A.bp(x, 1800, 7000) * (1 + 0.8 * np.sign(np.sin(2 * np.pi * 37 * A.tt(n))))
    return x * A.expenv(n, dur * 0.4, 0.004)


def pizz(m, vel=0.8, muted=False):
    """pizzicato: a plucked string with a little wooden body (#28); `muted` = palm-muted, short and dull (the rests)"""
    n = int((0.25 if muted else 0.5) * SR)
    f = A.mtof(m)
    t = A.tt(n)
    dec = 0.05 if muted else 0.16
    x = (np.sin(2 * np.pi * f * t) + 0.5 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.05)
         + 0.25 * np.sin(6 * np.pi * f * t) * np.exp(-t / 0.03)) * np.exp(-t / dec)
    x += 0.15 * A.bp(np.random.default_rng(m).normal(0, 1, n), 800, 3000) * np.exp(-t / 0.01)
    return A.lp(x, 1600 if muted else 4500) * vel


def squeal(dur=1.8):
    """SCREECH: the beginner-violin squeal: a wobbling, scratchy whistle sliding up (#28)"""
    n = int(dur * SR)
    t = A.tt(n)
    f = 2300 + 500 * np.sin(2 * np.pi * 6.5 * t) + 900 * t + 300 * np.sign(np.sin(2 * np.pi * 3.1 * t))
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) + 0.35 * np.sin(2 * np.pi * np.cumsum(f * 2.02) / SR)
    x = np.tanh(2.8 * x) * np.clip(t / 0.03, 0, 1) * np.exp(-t / (dur * 0.55))
    return A.bp(x, 700, 8000) + 0.5 * scratch(dur, 13)


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


# ---------------------------------------------------------------- VIOLA JOKE: the rimshot and the crowd's laugh
def addp(*xs):
    """sum signals of different lengths (zero-padded)"""
    n = max(len(x) for x in xs)
    return sum(np.pad(np.asarray(x, float), (0, n - len(x))) for x in xs)


def tom(f0=200.0, dec=0.18, vel=1.0):
    n = int(0.45 * SR)
    t = A.tt(n)
    f = f0 * (1 + 0.6 * np.exp(-t / 0.03))
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / dec)
    x += 0.3 * A.bp(np.random.default_rng(int(f0)).normal(0, 1, n), 300, 3000) * np.exp(-t / 0.02)
    return x * vel


def rimshot():
    """ba-dum-tss: two toms (high, low + the kick) and a ride / crash 'tss'"""
    n = int(1.6 * SR)
    out = np.zeros(n)

    def put(x, t, g):
        a = int(t * SR)
        b = min(n, a + len(x))
        out[a:b] += x[:b - a] * g
    put(addp(A.snare(0.08) * 0.6, tom(230, 0.14)), 0.0, 0.9)
    put(addp(tom(150, 0.22), A.kick(0.8, 0.25, 120, 50)[:int(0.4 * SR)] * 0.7), 1.5 * S16, 1.0)
    nt = int(1.2 * SR)
    tt_ = A.tt(nt)
    tss = A.hp(np.random.default_rng(77).normal(0, 1, nt), 5500) * np.exp(-tt_ / 0.35)
    tss += 0.12 * sum(np.sin(2 * np.pi * f * tt_) for f in (5100, 6650, 8230)) * np.exp(-tt_ / 0.5)
    put(tss, 3 * S16, 0.55)
    return out


def ha(f0, seed, dur=0.13, vowel=((800, 1.0), (1200, 0.6), (2600, 0.25))):
    """one 'ha': a breathy voiced burst (a pulse train through the vowel's formants) with an aspirated onset"""
    n = int(dur * SR)
    t = A.tt(n)
    rng = np.random.default_rng(seed)
    f = f0 * (1 + 0.06 * np.exp(-t / 0.04)) * (1 - 0.08 * t / dur)
    ph = np.cumsum(f) / SR
    pulse = (2 * (ph % 1.0) - 1) ** 3
    noise = rng.normal(0, 1, n)
    src = pulse * np.clip((t - 0.015) / 0.01, 0, 1) + 0.35 * noise
    x = sum(g * A.bp(src, fc * 0.82, fc * 1.18) for fc, g in vowel)
    env = np.clip(t / 0.012, 0, 1) * np.exp(-np.maximum(t - 0.03, 0) / (dur * 0.45))
    return x * env


def crowd_laugh(dur=2.1, n_voices=34, seed=55):
    """the whole crowd bursts out laughing: many voices, each a run of 'ha's (faster and higher at first, then
    trailing off), random onsets, pitches and pans; returns stereo (n, 2)"""
    n = int(dur * SR)
    out = np.zeros((n, 2))
    rng = np.random.default_rng(seed)
    for v in range(n_voices):
        f0 = rng.uniform(140, 330) if v % 3 else rng.uniform(260, 420)      # men, women, a few shrieks
        t = rng.uniform(0.0, 0.28)
        rate = rng.uniform(0.15, 0.21)
        pan = rng.uniform(-0.8, 0.8)
        amp = rng.uniform(0.5, 1.0)
        j = 0
        while t < dur - 0.2:
            x = ha(f0 * (1 - 0.03 * j), int(seed * 1000 + v * 37 + j), rng.uniform(0.10, 0.15))
            g = amp * math.exp(-t / (dur * 0.7)) * (1.0 if j < 6 else 0.7)
            a = int(t * SR)
            b = min(n, a + len(x))
            out[a:b, 0] += x[:b - a] * g * (1 - pan) * 0.5
            out[a:b, 1] += x[:b - a] * g * (1 + pan) * 0.5
            t += rate * (1 + 0.04 * j) + rng.uniform(-0.02, 0.02)
            j += 1
    return out / math.sqrt(n_voices) * 2.2


# ---------------------------------------------------------------- the drill bed (original): 808, drums, cellos + basses
def sub808(m0, m1, dur, slide=0.0, vel=1.0):
    """the 808: a sine sub with a punchy pitch drop on the attack, saturated, gliding to m1 over the last `slide` s"""
    n = int((dur + 0.05) * SR)
    t = A.tt(n)
    f0, f1 = A.mtof(m0), A.mtof(m1)
    u = np.clip((t - (dur - slide)) / max(slide, 1e-3), 0, 1) if slide > 0 else np.zeros(n)
    f = f0 * (f1 / f0) ** (u * u * (3 - 2 * u)) * (1 + 0.5 * np.exp(-t / 0.02))
    x = np.sin(2 * np.pi * np.cumsum(f) / SR)
    x = np.tanh(2.2 * x) / np.tanh(2.2)
    env = np.clip(t / 0.004, 0, 1) * np.clip((dur + 0.05 - t) / 0.05, 0, 1) * (0.75 + 0.25 * np.exp(-t / 0.15))
    return A.lp(x, 900) * env * vel


def hat(vel=1.0, open_=False, seed=0):
    return A.hat(0.12 if open_ else 0.03, open_) * vel


def clap(vel=1.0):
    c, s_ = A.clap(), A.snare(0.16)
    n = max(len(c), len(s_))
    return (np.pad(c, (0, n - len(c))) * 0.8 + np.pad(s_, (0, n - len(s_))) * 0.6) * vel


def rim(vel=1.0, seed=0):
    n = int(0.06 * SR)
    t = A.tt(n)
    x = np.sin(2 * np.pi * 1700 * t) * np.exp(-t / 0.008) + 0.5 * A.bp(np.random.default_rng(seed).normal(0, 1, n), 2500, 7000) * np.exp(-t / 0.006)
    return x * vel


def cello(m, dur, vel=0.85):
    return note('CELLO', m, round(dur, 3), round(dur + 0.2, 2), round(vel, 1))


def contrabass(m, dur, vel=0.9):
    """the basses: a bowed saw an octave under the cellos, low-passed (the bed's floor between 808 hits)"""
    n = int((dur + 0.08) * SR)
    f = A.mtof(m)
    x = A.saw(f, n, det=0.004, voices=2)
    t = A.tt(n)
    env = np.clip(t / 0.02, 0, 1) * np.clip((dur + 0.08 - t) / 0.08, 0, 1)
    return A.lp(x, 380) * env * vel * 0.6


# 808 rhythm per bar (16th, length in 16ths, degree, slide into the next): drill's syncopated pattern
P808 = [(0, 6, 0, False), (6, 4, 0, True), (10, 4, 12, False), (14, 2, 7, True)]
# hi-hats: 16th-grid straight 8ths with rolls (triplet 16ths / 32nds) at fixed spots in the bar
HAT_ROLLS = {3: 'trip', 11: '32', 15: 'trip'}
# the cellos' staccato stabs (16ths): a dark ostinato on chord tones in C2-G3
CELLO_PAT = [(0, 0), (3, 1), (6, 2), (8, 0), (10, 1), (11, 2), (14, 1)]


def bed(mix, b, density=1.0, light=False):
    """one bar of the drill beat (bar index b): 808 + kick, the half-time clap on beat 3, rim clicks, skittering hats,
    staccato cellos (+ the basses' long notes in the back half), a dark choir-ish pad on the phrase lines"""
    t0 = b * T.BAR
    ch = T.CHORDS[b]
    nxt = T.CHORDS[(b + 1) % T.N_BARS]
    root, nroot = T.CHORD_ROOT[ch], T.CHORD_ROOT[nxt]
    for s16, ln, deg, sl in P808:
        m0 = root + deg
        m1 = nroot if (sl and s16 >= 14) else (root + (12 if s16 == 6 else 0)) if sl else m0
        mix.place('bass808', sub808(m0, m1, ln * S16, slide=(0.08 if sl else 0.0), vel=1.0 if s16 == 0 else 0.8),
                  t0 + s16 * S16, 0.62)
    mix.place('drums', A.kick(0.9, 0.18, 140, 55)[:int(0.3 * SR)], t0, 0.48)
    mix.place('drums', A.kick(0.6, 0.14, 130, 55)[:int(0.25 * SR)], t0 + 10 * S16, 0.30)
    mix.place('drums', clap(1.0), t0 + 8 * S16, 0.34)                         # the half-time snare on beat 3
    if density > 1.2 and not light:
        mix.place('drums', clap(0.5), t0 + 15 * S16, 0.12)                     # a ghost into the next bar
    for s16 in range(0, 16, 2):
        if s16 + 1 in HAT_ROLLS and density > 0.9 and not light:
            continue
        mix.place('hats', hat(0.8 if s16 % 4 == 0 else 0.6), t0 + s16 * S16, 0.09, pan=0.25)
    for s16, kind in HAT_ROLLS.items():
        if light or density < 0.9:
            break
        if kind == 'trip':
            for j in range(3):
                mix.place('hats', hat(0.5 + 0.15 * j), t0 + (s16 - 1) * S16 + j * (2 * S16 / 3), 0.08, pan=0.25)
        else:
            for j in range(4):
                mix.place('hats', hat(0.45 + 0.1 * j), t0 + (s16 - 1) * S16 + j * (S16 / 2), 0.07, pan=0.25)
    mix.place('drums', rim(0.8, b), t0 + 6 * S16, 0.06, pan=-0.3)
    mix.place('drums', rim(0.6, b + 50), t0 + 13 * S16, 0.05, pan=-0.3)
    # strings: cellos (staccato ostinato), basses (held) in the denser stretches
    tones = T.chord_tones(36, 55, ch)
    for s16, j in CELLO_PAT:
        if light and s16 not in (0, 8):
            continue
        m = tones[(j + (1 if s16 >= 8 else 0)) % len(tones)]
        mix.place('cellos', cello(m, 0.09 if s16 % 4 else 0.13, 0.85), t0 + s16 * S16, 0.17, pan=-0.2)
        if density > 1.25:                                                     # the octave doubles in the back half
            mix.place('cellos', cello(m + 12, 0.08, 0.7), t0 + s16 * S16, 0.09, pan=0.2)
    if density > 1.1 or light:
        mix.place('basses', contrabass(root + 12, T.BAR - 0.06, 0.8), t0, 0.12)


def main():
    mix = A.Mix(T.DUR + 4.0)
    # ------------------------------------------------ bar 0: the VS title card, one unison band hit per word
    t1, t2, t3 = 0.004, T.BEAT, 2 * T.BEAT                 # VIOLA 4 ms in (no click at the seam), VS, VIOLIN
    bed(mix, 0, density=1.0)                               # the drill beat is full from frame 0 (levels rule)
    for j, tw_ in enumerate((t1, t2, t3)):
        mix.place('drums', A.kick(1.0, 0.25, 150, 48)[:int(0.4 * SR)], tw_, 0.50)
        mix.place('drums', A.crash(1.0), tw_, 0.05)
        mix.place('cellos', addp(cello(36, 0.3, 1.0), cello(43, 0.3, 1.0) * 0.8), tw_, 0.24)
        mix.place('fx', A.thud(0.8, 120, 34, seed=int(tw_ * 50) + 3), tw_, 0.26)
    for m, g in ((48, 0.42), (55, 0.36), (63, 0.30)):     # VIOLA: a dark chord on the C string (C3 G3 Eb4)
        mix.place('viola', viola(m, 0.8, 1.0, accent=0.5), t1, g, pan=-0.2)
    nn = int(1.4 * SR)                                     # VS: a thunder crack where it splits the divider
    tt_ = A.tt(nn)
    crack = A.bp(np.random.default_rng(91).normal(0, 1, nn), 1500, 9000) * np.exp(-tt_ / 0.035)
    rumble = A.lp(np.random.default_rng(92).normal(0, 1, nn), 160) * np.clip(tt_ / 0.02, 0, 1) * np.exp(-tt_ / 0.5) * 3.0
    mix.place('fx', crack * 0.9 + rumble, t2, 0.32)
    for j, m in enumerate((67, 70, 72, 75, 77, 79, 84)):  # VIOLIN: a flourish up to a high C
        mix.place('violin', bow_note(m, 1 if j < 6 else 5, 0.95), t3 + j * S16 / 2, 0.36 if j == 6 else 0.28, pan=0.2)
    mix.place('fx', A.whoosh(T.T_DOORS[1] - T.T_DOORS[0] + 0.1, 300, 5000), T.T_DOORS[0] - 0.05, 0.12)
    # ------------------------------------------------ bar 1: the count-in on the dominant (G)
    bed(mix, 1, density=1.0)
    for j, (tc, txt) in enumerate(T.COUNT[:3]):
        mix.place('drums', A.kick(0.9, 0.2, 150, 50)[:int(0.35 * SR)], tc, 0.40)
        mix.place('cellos', cello((43, 47, 50)[j], 0.18, 1.0), tc, 0.26)
        mix.place('viola', pizz((55, 59, 62)[j], 0.9), tc, 0.22, pan=-0.2)          # the viola counts it in
        mix.place('violin', pizz((79, 83, 86)[j], 0.9), tc + S16 / 2, 0.18, pan=0.2)
    t4 = T.BAR + 3 * T.BEAT                                          # beat 4: the pickup into the drop
    for j in range(8):
        mix.place('hats', hat(0.5 + 0.06 * j), t4 + j * S16 / 2, 0.10, pan=0.25)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), t4, 0.12)
    mix.place('violin', bow_note(79, 3, 0.9), t4 + 2 * S16, 0.22, pan=0.2)
    mix.place('drums', A.crash(1.8), T.T_FIGHT, 0.10)               # the drop
    mix.place('fx', A.thud(0.7, 110, 34), T.T_FIGHT, 0.30)
    # ------------------------------------------------ the drill beat: bar 2 (FIGHT!) .. the K.O.
    bar_ko = int(T_KO // T.BAR)
    for b in range(2, bar_ko + 1):
        bed(mix, b, density=1.0 + 0.6 * (b / bar_ko))
    for b in range(2, bar_ko + 1, 8):                                  # every 8-bar phrase: a crash on its first beat
        mix.place('drums', A.crash(1.6), b * T.BAR, 0.07)
    t18 = 18 * T.BAR
    mix.place('fx', riser(T.BAR, over=0.0, seed=12) * 0.6, t18 - T.BAR, 0.07)
    mix.place('drums', A.crash(2.0), t18, 0.10)
    for k in ('drums', 'hats', 'bass808', 'cellos', 'basses'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ MELODY AT LAST windows (the arrangement flips)
    mel_win = []
    for t0, k, until in RD.MELODIES:
        s = math.ceil((t0 + 1e-6) / T.BEAT) * T.BEAT                   # from the beat after it fires (the engine's)
        mel_win.append((s, min(until, T_KO)))

    def in_melody(t):
        return any(a <= t < b for a, b in mel_win)
    # ------------------------------------------------ VIOLIN: the bow's turns play the line (#28)
    th = RUN['TH'][:, KV]
    d = (np.diff(th) + np.pi) % (2 * np.pi) - np.pi
    phi = np.concatenate([[0.0], np.cumsum(np.abs(d))])
    tt = T.T_FIGHT + np.arange(len(phi)) / T.FPS
    trig, kq = [], 1
    for i in range(1, len(phi)):
        while phi[i] >= kq * math.pi / 2:
            u = (kq * math.pi / 2 - phi[i - 1]) / max(1e-9, phi[i] - phi[i - 1])
            trig.append(tt[i - 1] + u / T.FPS)
            kq += 1
    trig = sorted(set(round(q16(t) / S16) for t in trig if t < T_KO))
    thin_v = Thinner(3)
    for j, slot in enumerate(trig):
        tq = slot * S16
        nxt = trig[j + 1] * S16 if j + 1 < len(trig) else tq + 4 * S16
        n16 = int(np.clip(round((nxt - tq) / S16), 1, 8))
        hp = hp_at(KV, tq) / 100.0
        if not thin_v.keep(slot, hp):
            continue
        m = T.melody_at(tq)
        soft = in_melody(tq)
        if soft:                                                       # the viola sings: the violin harmonises softly
            m = min(T.chord_tones(64, 79, T.chord_at(tq)), key=lambda x: abs(x - (m - 3)))
        vel = 0.62 + 0.25 * min(1.0, 2.0 / n16)
        sig, g, lead = colour(bow_note(m, n16, vel), KV, tq, 1.0)
        mix.place('violin', sig, tq, (0.14 if soft else 0.34) * g, pan=0.2)
        if (1 - hp) > 0.25:
            mix.place('violin', scratch(min(0.25, n16 * S16), slot), tq, 0.09 * (1 - hp) ** 1.3, pan=0.2)
        if lead > 0.04 and not soft:                                   # the leader: a double-stop a sixth below
            under = [x for x in T.chord_tones(m - 10, m - 7, T.chord_at(tq))]
            if under:
                mix.place('violin', colour(bow_note(under[0], n16, 0.6), KV, tq)[0], tq, 0.14 * g, pan=0.2)
    climb = [m for m in range(74, 99) if (m - T.KEY_ROOT) % 12 in T.SCALE]
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'hit' and e[2] == KV:
            tq = q16(t)
            how, sub = e[7], e[8]
            pan = float(np.clip(e[5] / 3.5, -0.6, 0.6))
            if how == 'blade':                                        # accents climbing the scale, one per landed hit
                m = climb[int(sub) % len(climb)]
                sig, g, _ = colour(bow_note(m, 1, 1.0), KV, t)
                mix.place('violin', sig, tq, 0.30 * g, pan=pan)
                mix.place('violin', scratch(0.05, int(t * 11)), tq, 0.05, pan=pan)
            elif how == 'key':                                        # a pizzicato dart lands
                ms = T.chord_tones(69, 93, T.chord_at(tq))
                mix.place('violin', colour(pizz(ms[int(sub) * 2 % len(ms)], 0.9), KV, t)[0], tq, 0.30, pan=pan)
            elif how == 'wave':
                mix.place('fx', A.thud(0.45, 110, 40), tq, 0.30)
        elif kind == 'crushed' and e[2] == KA:                        # a dart flattened by a rolling note
            mix.place('violin', A.hp(pizz(81, 0.5), 1500), q16(t), 0.06)
        elif kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.05 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            mix.place('fx', A.chime(91, 0.25), t, 0.04)
        elif kind == 'ability' and e[2] == KV:
            name = e[3]
            tq = q16(t)
            if name == 'PIZZICATO':
                n = RD.CFG[KV]['abilities']['unlock1']['n']
                ms = T.chord_tones(69, 93, T.chord_at(tq))
                for j in range(n):                                    # the darts launch on the 16ths as a rising pluck
                    mix.place('violin', colour(pizz(ms[j * 2 % len(ms)], 0.8), KV, t)[0], tq + j * S16, 0.24, pan=0.2)
            elif name == 'TREMOLO':
                dur = RD.CFG[KV]['abilities']['unlock2']['dur']
                k = 0
                while k * S16 / 2 < dur:                              # a shiver of 32nds on the line under the frantic bow
                    tk = tq + k * S16 / 2
                    mix.place('violin', colour(bow_note(T.melody_at(tk) + 12, 1, 0.5), KV, t)[0], tk,
                              0.07 * (0.5 + 0.5 * k * S16 / 2 / dur), pan=0.2)
                    k += 1
            elif name == 'SCREECH':
                mix.place('violin', squeal(1.9), tq, 0.14, pan=0.2)
                mix.place('violin', colour(bow_note(96, 6, 1.0), KV, t, 2.0)[0], tq, 0.16, pan=0.2)
    # ------------------------------------------------ VIOLA: the rests, the whole notes, the crushes
    thin_a = Thinner(9)
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'rest' and e[2] == KA:                             # the counting: a soft muted pluck on 1-2-3
            tq = q16(t)
            m = T.viola_inner(tq) - (12 if e[3] == 1 else 0)
            mix.place('viola', pizz(max(48, m), 0.7, muted=True), tq, 0.22 + 0.03 * e[3], pan=-0.2)
        elif kind == 'note' and e[2] == KA:
            nkind, t_land, t1 = e[8], e[9], e[10]
            ts = q16(t_land)
            if nkind != 2 and in_melody(ts):
                continue                                              # the lead line plays these (below)
            slot = round(ts / S16)
            if nkind == 0 and not thin_a.keep(slot, hp_at(KA, ts) / 100.0):
                continue
            dur = max(0.3, min(T.BAR, t1 - t_land) * 0.96)
            m = T.viola_inner(ts)
            if nkind == 2:                                            # VIOLA JOKE: the open C + G, fortissimo
                for mm, g in ((48, 0.50), (55, 0.42), (60, 0.26)):
                    mix.place('viola', viola(mm, dur, 1.0, accent=0.8), ts, g, pan=-0.15)
                continue
            sig, g, lead = colour(viola(m, dur, 0.9, accent=0.45), KA, ts, 0.8)
            gain = 0.52 if nkind == 0 else 0.38
            mix.place('viola', sig, ts, gain * g, pan=-0.2)
            if nkind == 1:                                            # DOUBLE STOP: a second string a sixth above
                m2 = min(T.chord_tones(m + 7, m + 10, T.chord_at(ts)) or [m + 7])
                mix.place('viola', colour(viola(m2, dur, 0.85, accent=0.3), KA, ts)[0], ts, 0.26 * g, pan=-0.25)
            elif lead > 0.04:                                         # the leader: doubled an octave down (the C string)
                mix.place('viola', colour(viola(max(48, m - 12), dur, 0.7), KA, ts)[0], ts, 0.16 * g, pan=-0.2)
        elif kind == 'crush' and e[2] == KA:                          # a whole note crushes: a low string crunch + thud
            tq = q16(t)
            big = any(N['kind'] == 2 for nid, N in RD.NOTES.items() if nid == e[3])
            mix.place('fx', A.thud(0.5 if big else 0.32, 110, 36, seed=int(t * 13)), tq, 0.42 if big else 0.26)
            mix.place('cellos', cello(36, 0.12, 1.0), tq, 0.20 if big else 0.12)
            mix.place('viola', scratch(0.12, int(t * 17)), tq, 0.10 if big else 0.06, pan=-0.2)
            if big:
                mix.place('drums', A.crash(2.0), tq, 0.12)
                mix.place('bass808', sub808(24, 24, 0.9), tq, 0.70)
        elif kind == 'ability' and e[2] == KA:
            name = e[3]
            tq = q16(t)
            if name == 'DOUBLE STOP':                                 # two strings dug in at once
                mix.place('viola', scratch(0.1, int(t * 29)), tq, 0.10, pan=-0.2)
            elif name == 'VIOLA JOKE':                                # ba-dum-tss + the crowd laughing
                mix.place('fx', rimshot(), tq, 0.42)
                lg = crowd_laugh(2.1)
                mix.place('laugh', lg[:, 0], tq + 3 * S16 + 0.05, 0.52, pan=-0.6)
                mix.place('laugh', lg[:, 1], tq + 3 * S16 + 0.05, 0.52, pan=0.6)
                mix.place('fx', A.whoosh(T.BEAT, 200, 3000, rise=False), tq + 0.1, 0.10)    # the note falling
    # MELODY AT LAST: the viola sings the lead for its 2 bars
    for a, b in mel_win:
        prev = None
        for bt, m, dl in T.VIOLA_LEAD:
            ts = a + bt * T.BEAT
            if ts >= b - 0.02:
                break
            mm = T.fit(m, T.chord_at(ts + 0.01), 52, 76)
            x = viola(mm, dl * T.BEAT * 0.97, 0.95, accent=0.3 if prev is None or abs(mm - prev) > 2 else 0.1)
            sig, g, _ = colour(x, KA, ts, 0.5)
            mix.place('viola', sig, ts, 0.50 * g, pan=-0.1)
            mix.place('viola', colour(viola(max(48, mm - 12), dl * T.BEAT * 0.97, 0.7), KA, ts)[0], ts, 0.14, pan=-0.2)
            prev = mm
        mix.place('fx', A.whoosh(0.5, 300, 4000), a - 0.25, 0.06)
    # ------------------------------------------------ the K.O. moment
    mix.place('drums', A.crash(2.4), T_KO, 0.14)
    mix.place('fx', A.thud(0.9, 120, 30), T_KO, 0.55)
    loser = NAMES[LOSE]
    if loser == 'VIOLIN':                                             # the violin falls apart: a scratchy run down
        for j, m in enumerate((88, 84, 79, 75, 72, 67)):
            mix.place('violin', bow_note(m, 1, 0.8), RD.T_HS + 0.1 * j, 0.20, pan=(j - 2.5) / 6)
            mix.place('violin', A.hp(scratch(0.06, 40 + j), 3000), RD.T_HS + 0.1 * j, 0.08, pan=(j - 2.5) / 6)
    else:                                                             # the viola falls apart: a groaning slide down
        for j, m in enumerate((67, 63, 60, 55, 51, 48)):
            mix.place('viola', viola(m, 0.12, 0.8), RD.T_HS + 0.1 * j, 0.24, pan=(j - 2.5) / 6)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.14)
    tw = q16(T_KO + 1.3)                                              # the winner's stinger on "(WINNER) WINS"
    if NAMES[WIN] == 'VIOLA':
        for m, g in ((48, 0.44), (55, 0.36), (63, 0.30), (67, 0.26)):
            mix.place('solo', viola(m, 1.1, 1.0, accent=0.6), tw, g)
    else:
        for j, m in enumerate((72, 75, 79, 84)):
            mix.place('solo', bow_note(m, 1 if j < 3 else 8, 1.0), tw + j * S16 / 2, 0.30)
    mix.place('cellos', addp(cello(36, 0.5, 1.0), cello(43, 0.5, 1.0) * 0.8), tw, 0.24)
    mix.place('bass808', sub808(36, 36, 0.8), tw, 0.6)
    mix.place('drums', A.crash(1.8), tw, 0.09)
    solo0 = RD.T_SOLO
    mix.place('fx', A.whoosh(max(0.2, solo0 - tw - 0.1), 300, 5000), tw + 0.1, 0.08)
    # ------------------------------------------------ victory: a lighter drill beat + the winner's solo
    b_solo = int(round(solo0 / T.BAR))
    for b in range(b_solo, 34):
        bed(mix, b, density=1.0, light=True)
    for bt, m, d in T.SOLO[NAMES[WIN]]:
        ts = solo0 + bt * T.BEAT
        if ts > T.T_REASSEMBLE - 0.05:
            break
        if NAMES[WIN] == 'VIOLA':
            mix.place('solo', viola(m, d * T.BEAT * 0.95, 0.95, accent=0.3), ts, 0.52)
        else:
            mix.place('solo', bow_note(m, max(1, int(round(d * 4))), 0.9), ts, 0.46)
    # the loser reassembles: its pieces fly home as a rising run, a glass "pop" when it's whole
    for j in range(6):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / 6)
        if loser == 'VIOLIN':
            mix.place('violin', pizz((67, 72, 75, 79, 84, 87)[j], 0.7), tj, 0.18, pan=0.2)
        else:
            mix.place('viola', pizz((48, 55, 60, 63, 67, 72)[j], 0.7), tj, 0.20, pan=-0.2)
    mix.place('fx', addp(A.chime(95, 0.5), A.chime(100, 0.5) * 0.5), T.T_GLIDE, 0.10)
    # ------------------------------------------------ bars 34-35: the beat comes back and builds into the slam on sample 0
    bed(mix, 34, density=1.3)
    t35 = 35 * T.BAR
    bed(mix, 35, density=1.5)
    for bt in range(4):                                              # the cellos climbing G into the C on frame 0
        mix.place('cellos', cello((43, 47, 50, 55)[bt], 0.3, 1.0), t35 + bt * T.BEAT, 0.26)
        mix.place('viola', viola((55, 59, 62, 67)[bt], 0.3, 0.9), t35 + bt * T.BEAT, 0.20, pan=-0.2)
        mix.place('violin', bow_note((79, 83, 86, 91)[bt], 2, 0.9), t35 + bt * T.BEAT + 2 * S16, 0.16, pan=0.2)
    for j in range(16):                                              # a hat roll speeding into the seam
        mix.place('hats', hat(0.4 + 0.035 * j), t35 + 2 * T.BEAT + j * S16 / 2, 0.10, pan=0.25)
    mix.place('fx', riser(T.DUR - t35), t35, 0.16)
    nc = int((T.T_CLOSE + 0.012) * SR)                              # the doors sliding shut into the title slam
    tcl = A.tt(nc)
    doors = A.bp(np.random.default_rng(93).normal(0, 1, nc), 400, 5000) * (tcl / T.T_CLOSE) ** 2
    mix.place('fx', doors, T.DUR - T.T_CLOSE, 0.36)
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


BUS_GAIN = {'violin': 1.6, 'viola': 1.7, 'solo': 1.5, 'fx': 1.2, 'drums': 1.0, 'hats': 1.0, 'bass808': 1.0,
            'cellos': 1.1, 'basses': 1.0, 'laugh': 1.0}


def master(mix, thr=0.15, ceiling=0.89):
    """ONE master gain (no section automation: consistent loudness from frame 0) + a compressor and a soft limiter;
    the overflow past the loop (tails, reverb) folds onto the start; NA samples (whole AAC frames) (#40)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('violin', 0.4), ('viola', 0.4), ('solo', 0.45), ('fx', 0.2),
                                                ('cellos', 0.35), ('laugh', 0.3), ('drums', 0.1)))
    wet = A.reverb(send, dur=2.2, decay=0.55) * 0.20                 # a concert-hall arena
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
