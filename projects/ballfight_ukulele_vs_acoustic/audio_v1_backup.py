"""#33 BALLFIGHT #7: Ukulele vs Acoustic Guitar - soundtrack (spaghetti-western standoff), all synthesised, original.

The two fighters ARE the rhythm section, straight from the honest run:
  UKULELE   every dash is a strum in the uke's own Pixel Pitch voice (reentrant GCEA shapes): down-strums on beats 1 and
            3 (its dash grid), up-strums on 2 and 4 when SPEED STRUM doubles the dashes; a landed dash adds a picked
            note climbing the scale and a percussive chunk. As its HP drops it loses strings (4 -> 3 -> 2) and gets
            dirtier; the leader adds an up-strum on the off-beat.
  ACOUSTIC  every sound-hole cone is a strum of the big open chord: a full down-strum on 1 and 3, a lighter "chick" on
            the top strings on 2 and 4, CAPO's extra cones are bright up-strums on the 8ths; a landed cone adds a body
            slap and a bass-string pop. As its HP drops it loses strings and the chicks drop out; the leader adds the
            boom-chick alternating bass under its own strums.
Bed: A minor at 129.84 BPM (33 bars = 61 s), the descending Am - G - F - E cadence: a galloping clip-clop with a frame-
drum thump, a twangy baritone guitar on the bass line (kept low and dark, under both fighters), a whistled lead playing
an original standoff theme (bars 1-8 and 17-24) with a dotted-8th echo, a wordless choir and a tolling bell from bar
9, tremolo strings and a snare ostinato for the final exchange. The count-in bar is whip cracks + both fighters'
strums on the E chord; at the K.O. the bed tape-stops with the hit-stop, the loser's strings snap, the crowd roars,
the winner's stinger lands, then its solo over a lighter gallop; bar 32 rides into the "3" on sample 0. Every
fighter hit is snapped to the nearest 16th. Tails past the loop fold onto the start.

    python audio.py            -> audio.wav (whole AAC frames) + stems in check/ for the band check"""
import math
import os
import sys
import numpy as np
from scipy.signal import lfilter

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
KU, KA = NAMES.index('UKULELE'), NAMES.index('ACOUSTIC')
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames (2860 = 61.013 s): loops sample-perfectly
S16 = T.S16
PAN = {KU: -0.25, KA: 0.25}


def q16(t):
    return round(t / S16) * S16


def h01(*xs):
    """deterministic hash in [0, 1) (thinning decisions are part of the score, not random each run)"""
    v = 0
    for x in xs:
        v = (v * 1000003 + int(x * 1000) + 7) % 2147483647
    return ((v * 48271) % 2147483647) / 2147483647.0


# ---------------------------------------------------------------- the fighters' voices (cached, level-matched)
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


def note(name, midi, dur=0.3, cap=0.9, vel=0.85):
    key = (name, int(midi), round(dur, 3), cap, round(vel, 2))
    if key not in _VC:
        _VC[key] = _fit(B.voice(name, int(midi), vel, dur), cap) * inst_gain(name)
    return _VC[key]


def strum(name, notes, vel=0.85, spread=0.011, up=False, dur=0.40, cap=1.0):
    """a strum: the chord's strings a few ms apart, low to high (down) or high to low (up)"""
    ns = list(notes)[::-1] if up else list(notes)
    d = int(spread * SR)
    out = np.zeros(int(cap * SR) + d * len(ns))
    for j, m in enumerate(ns):
        v = round(vel * (1.0 if j == 0 else 0.9), 2)
        x = note(name, m, dur, cap, v)
        out[j * d:j * d + len(x)] += x
    return out / math.sqrt(len(ns)) * 1.5


def colour(x, k, t, drive_k=1.0):
    """the fighter's tone at time t: thinner + dirtier as its HP drops; returns (signal, gain, lead)"""
    hp = RD.hp_at(k, t) / 100.0
    lead = (RD.hp_at(k, t) - RD.hp_at(1 - k, t)) / 100.0
    cut = 90 + 520 * (1 - hp) ** 1.6
    y = A.hp(x, cut)
    drive = 1 + 4.5 * drive_k * (1 - hp) ** 2
    y = np.tanh(drive * y * 1.2) / np.tanh(drive * 1.2) * (1.0 / (1 + 0.15 * (drive - 1)))
    g = 10 ** (float(np.clip(6.0 * lead, -4.0, 3.5)) / 20)
    return y, g, lead


def uke_strings(hp):
    return 4 if hp > 50 else (3 if hp > 25 else 2)


def ac_strings(hp):
    return 6 if hp > 60 else (5 if hp > 35 else (4 if hp > 15 else 3))


# ---------------------------------------------------------------- bed synths (original)
_KS = {}


def ks(m, dur, decay=0.996, bright=0.6, seed=0):
    """Karplus-Strong string (comb filter via lfilter), cached"""
    key = (m, round(dur, 3), decay, bright, seed)
    if key not in _KS:
        f = A.mtof(m)
        N = int(round(SR / f))
        n = int(dur * SR)
        exc = np.random.default_rng(seed + int(m)).uniform(-1, 1, N)
        exc = bright * exc + (1 - bright) * np.convolve(exc, np.ones(6) / 6, 'same')
        x = np.zeros(n)
        x[:N] = exc
        a = np.zeros(N + 2)
        a[0], a[N], a[N + 1] = 1.0, -decay * 0.5, -decay * 0.5
        _KS[key] = lfilter([1.0], a, x)
    return _KS[key]


def twang(m, dur=0.6, vel=1.0):
    """baritone guitar: a bright pluck through a little tremolo and a spring tank, kept dark (lowpassed)"""
    x = ks(m, dur + 0.3, 0.997, 0.85)
    t = A.tt(len(x))
    x = x * (1 + 0.22 * np.sin(2 * np.pi * 6.2 * t))
    x = np.tanh(1.8 * x) / np.tanh(1.8)
    y = x.copy()
    for dl, g in ((0.037, 0.32), (0.053, 0.24), (0.071, 0.18)):          # the spring: short metallic echoes
        d = int(dl * SR)
        a = np.zeros(d + 1)
        a[0], a[d] = 1.0, -g
        y += 0.5 * lfilter([1.0], a, x) - 0.5 * x
    y = A.bp(y, 70, 1900)
    fd = int(0.06 * SR)
    y[-fd:] *= np.linspace(1, 0, fd)
    return y * vel


def hoof(vel=1.0, hi=False, seed=0):
    """a coconut-shell clip-clop: two hollow tones and a click"""
    n = int(0.09 * SR)
    t = A.tt(n)
    f = 1080 if hi else 880
    x = np.sin(2 * np.pi * f * t) * np.exp(-t / 0.018) + 0.4 * np.sin(2 * np.pi * f * 1.9 * t) * np.exp(-t / 0.008)
    x += 0.3 * A.bp(np.random.default_rng(seed).normal(0, 1, n), 1800, 6000) * np.exp(-t / 0.003)
    return x * vel


def thump(vel=1.0):
    """a low frame drum under the gallop"""
    return A.kick(punch=0.6, dec=0.22, f0=118, f1=56) * vel


def whip(vel=1.0, seed=2):
    """a whip crack: a short swish, then the crack at +0.10 s (place it 0.10 s early)"""
    n = int(0.22 * SR)
    t = A.tt(n)
    x = np.random.default_rng(seed).normal(0, 1, n)
    sw = A.bp(x, 900, 5000) * np.clip(t / 0.10, 0, 1) ** 2 * (t < 0.10) * 0.35
    tc = np.maximum(t - 0.10, 0)
    crack = (A.hp(x, 2500) * np.exp(-tc / 0.006) + 0.6 * np.sin(2 * np.pi * np.cumsum(4200 * np.exp(-tc / 0.006) + 1100) / SR)
             * np.exp(-tc / 0.012)) * (t >= 0.10)
    return (sw + crack) * vel


WHIP_PRE = 0.10


def bell(m=45, vel=1.0, dur=4.0):
    """a tolling church bell: inharmonic partials (hum, prime, tierce, quint, nominal ...) with their own decays"""
    n = int(dur * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = np.zeros(n)
    for r, a, d in ((0.5, 0.5, 3.0), (1.0, 1.0, 2.2), (1.19, 0.55, 1.6), (1.5, 0.3, 1.1), (2.0, 0.75, 1.3),
                    (2.61, 0.3, 0.7), (3.0, 0.22, 0.6), (4.12, 0.18, 0.4)):
        x += a * np.sin(2 * np.pi * f * r * t + r) * np.exp(-t / d)
    x += 0.4 * A.bp(np.random.default_rng(3).normal(0, 1, n), 800, 4000) * np.exp(-t / 0.01)
    return x * vel * 0.5


def choir(ms, dur, att=0.45, rel=0.5):
    """a wordless 'aah' choir: detuned saws through three vowel formants"""
    n = int(dur * SR)
    x = sum(A.saw(A.mtof(m), n, det=0.008, voices=3) for m in ms) / max(1, len(ms))
    y = A.bp(x, 620, 900) + 0.5 * A.bp(x, 1050, 1350) + 0.18 * A.bp(x, 2400, 2900)
    t = A.tt(n)
    y *= 1 + 0.04 * np.sin(2 * np.pi * 5.0 * t)
    return y * A.adsr(n, att, 0.2, 0.85, min(rel, dur * 0.4))


def tremolo_strings(ms, dur, k0=0.4, k1=1.0):
    """tremolo strings for the final exchange: a saw chord shivering on the 32nds, swelling from k0 to k1"""
    n = int(dur * SR)
    t = A.tt(n)
    x = sum(A.saw(A.mtof(m), n, det=0.005, voices=2) for m in ms) / max(1, len(ms))
    trem = 0.55 + 0.45 * np.abs(np.sin(np.pi * t / (S16 / 2)))
    sw = k0 + (k1 - k0) * np.clip(t / dur, 0, 1)
    return A.lp(x * trem * sw, 2800) * A.adsr(n, 0.08, 0.1, 1.0, 0.05)


def whistle(m, dur, vel=1.0, m_prev=None):
    """a whistled note: a near-pure tone with breath, a glide in from the previous note, vibrato that blooms late"""
    n = int((dur + 0.12) * SR)
    t = A.tt(n)
    f1 = A.mtof(m)
    f0 = A.mtof(m_prev) if m_prev is not None else f1 * 0.985
    f = f1 + (f0 - f1) * np.exp(-t / 0.035)
    f *= 1 + 0.0075 * np.clip((t - 0.16) / 0.3, 0, 1) * np.sin(2 * np.pi * 5.6 * t)
    ph = 2 * np.pi * np.cumsum(f) / SR
    x = np.sin(ph) + 0.05 * np.sin(2 * ph) + 0.015 * np.sin(3 * ph)
    br = np.random.default_rng(int(m)).normal(0, 1, n)
    x += 0.10 * A.bp(br, f1 * 0.85, f1 * 1.2) + 0.012 * A.bp(br, 4000, 9000)
    env = np.clip(t / 0.035, 0, 1) * np.clip((dur + 0.08 - t) / 0.08, 0, 1)
    return A.lp(x * env, 7000) * vel


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


def scrape(dur, seed=21):
    """a pick dragged down the wound strings: a falling, ribbed noise band, getting louder"""
    n = int(dur * SR)
    t = A.tt(n)
    x = np.random.default_rng(seed).normal(0, 1, n) * (1 + 0.8 * np.sign(np.sin(2 * np.pi * 70 * t)))
    out = np.zeros(n)
    nb = 16
    for k in range(nb):
        a, b = int(k * n / nb), int((k + 1) * n / nb)
        fc = 3200 * (0.25 ** ((k + 0.5) / nb))
        out[a:b] = A.bp(x[max(0, a - 2048):b], fc * 0.7, fc * 1.4)[-(b - a):]
    return out * (t / dur) ** 1.5


BASS = {'Am': (45, 52), 'G': (43, 50), 'F': (41, 48), 'E': (40, 47)}       # (root on 1, fifth on 3) for the twang
CHOIR = {'Am': (57, 60, 64), 'G': (55, 59, 62), 'F': (57, 60, 65), 'E': (56, 59, 64)}
STRINGS = {'Am': (57, 64, 69, 72), 'G': (55, 62, 67, 71), 'F': (53, 60, 65, 69), 'E': (52, 59, 64, 68)}


def bed(mix, b, light=False):
    """one bar of the bed (bar index b): gallop, twang bass, choir, bell, strings / snare by section"""
    t0 = b * T.BAR
    ch = T.CHORDS[b]
    nxt = T.CHORDS[(b + 1) % T.N_BARS]
    sec = 0 if b <= 8 else (1 if b <= 16 else (2 if b <= 24 else 3))
    if light:
        sec = 0
    # gallop: DUM . da-da on every beat (the hooves), a frame-drum thump on 1 and 3 (all four from bar 9)
    for bt in range(4):
        tb = t0 + bt * T.BEAT
        if bt in (0, 2) or sec >= 1:
            mix.place('perc', thump(1.0 if bt in (0, 2) else 0.6), tb, 0.36)
        mix.place('perc', hoof(1.0, bt % 2 == 0, b * 4 + bt), tb, 0.24, pan=-0.2)
        mix.place('perc', hoof(0.55, False, b * 4 + bt + 50), tb + 2 * S16, 0.24, pan=0.2)
        mix.place('perc', hoof(0.8, True, b * 4 + bt + 90), tb + 3 * S16, 0.24, pan=0.0)
        if sec == 3 and not light:                           # the final exchange: 16th hooves all the way
            mix.place('perc', hoof(0.45, False, b * 4 + bt + 130), tb + S16, 0.13, pan=-0.1)
    # twang bass: root on 1, fifth on 3, a walk into the next chord on the and of 4
    r, f5 = BASS[ch]
    mix.place('twang', twang(r, 1.4 * T.BEAT), t0, 0.42)
    mix.place('twang', twang(f5, 1.2 * T.BEAT, 0.85), t0 + 2 * T.BEAT, 0.36)
    if sec >= 1 and not light:
        tgt = BASS[nxt][0]
        app = tgt + 1 if tgt < r else tgt - 1
        mix.place('twang', twang(app, 0.4 * T.BEAT, 0.7), t0 + 3.5 * T.BEAT, 0.30)
    # choir from bar 9 (louder from 17), the bell tolls every 4 bars from bar 17
    if sec >= 1 or light:
        mix.place('choir', choir(CHOIR[ch], T.BAR + 0.15), t0, 0.40 if sec == 1 or light else 0.55)
    if sec >= 2 and not light and (b - 17) % 4 == 0:
        mix.place('bell', bell(45, 1.0), t0, 0.30)
    # the final exchange: tremolo strings + a snare ostinato
    if sec == 3 and not light:
        k = min(1.0, (b - 24) / 4)
        mix.place('strings', tremolo_strings(STRINGS[ch], T.BAR, 0.45 + 0.4 * k, 0.55 + 0.45 * k), t0, 0.22)
        for st in (0, 3, 6, 8, 11, 14):
            mix.place('perc', A.hp(A.snare(0.06), 400), t0 + st * S16, 0.05 + 0.03 * k)


def whistle_phrase(mix, b0, octave=0, gain=0.075):
    """the theme over 8 bars from bar b0 (with the dotted-8th echo on the other side)"""
    prev = None
    for bar, beat, m, d in T.WHISTLE:
        t = (b0 + bar) * T.BAR + beat * T.BEAT
        x = whistle(m + 12 * octave, d * T.BEAT * 0.96, 1.0, prev)
        mix.place('whistle', x, t, gain, pan=0.08)
        mix.place('whistle', A.lp(x, 3500), t + 0.75 * T.BEAT, gain * 0.24, pan=-0.35)
        prev = m + 12 * octave


# ---------------------------------------------------------------- the fighters' parts
def uke_part(mix):
    climb = T.scale_notes(72, 93)
    n_landed = 0
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'dash' and e[2] == KU:
            tq = q16(t)
            hp = RD.hp_at(KU, t)
            ch = T.chord_at(tq)
            vo = T.UKE_CHORD[ch][:uke_strings(hp)]
            main_ = bool(e[3])
            sig = strum('UKULELE', vo, 0.9 if main_ else 0.7, 0.010 if main_ else 0.008, up=not main_)
            y, g, lead = colour(sig, KU, t, 0.8)
            mix.place('uke', y, tq, (0.36 if main_ else 0.26) * g, pan=PAN[KU])
            if main_ and lead > 0.04:                          # the leader: an up-strum on the off-beat
                y2 = colour(strum('UKULELE', vo, 0.6, 0.008, up=True), KU, t, 0.8)[0]
                mix.place('uke', y2, tq + 2 * S16, 0.18 * g, pan=PAN[KU])
        elif kind == 'hit' and e[2] == KU and e[7] == 'dash':
            tq = q16(t)
            m = climb[n_landed % len(climb)]
            n_landed += 1
            y, g, _ = colour(note('UKULELE', m, 0.25, 0.6, 1.0), KU, t, 0.6)
            mix.place('uke', y, tq, 0.30 * g, pan=PAN[KU])
            mix.place('fx', A.bp(np.random.default_rng(int(t * 50)).normal(0, 1, int(0.05 * SR)), 900, 4500)
                      * np.exp(-A.tt(int(0.05 * SR)) / 0.01), tq, 0.10, pan=PAN[KU])
            mix.place('fx', A.thud(0.3, 140, 60, seed=int(t * 7)), tq, 0.12)


def ac_part(mix):
    n_landed = 0
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'cone' and e[2] == KA:
            tq = q16(t)
            hp = RD.hp_at(KA, t)
            ch = T.chord_at(tq)
            full = T.AC_CHORD[ch]
            vo = full[:max(3, ac_strings(hp) - (6 - len(full)))]              # broken strings go from the top
            main_ = bool(e[3])
            pos = int(round(tq / T.BEAT)) % 4
            if not main_:                                       # CAPO's 8ths: bright up-strums on the top strings
                y, g, _ = colour(A.hp(strum('ACOUSTIC', full[-4:], 0.6, 0.007, up=True, dur=0.2), 350), KA, t, 0.6)
                mix.place('acoustic', y, tq, 0.22 * g, pan=PAN[KA])
                continue
            lead = (hp - RD.hp_at(KU, t)) / 100.0
            if pos in (0, 2):
                y, g, _ = colour(strum('ACOUSTIC', vo, 0.92, 0.013), KA, t, 0.7)
                mix.place('acoustic', y, tq, 0.36 * g, pan=PAN[KA])
                if lead > 0.04:                                 # the leader: boom-chick alternating bass
                    r, f5 = BASS[ch]
                    mix.place('acoustic', colour(note('ACOUSTIC', r if pos == 0 else f5, 0.4, 0.7, 1.0), KA, t)[0],
                              tq, 0.26 * g, pan=PAN[KA])
            else:
                if h01(tq, 7) < 0.6 * (1 - hp / 100.0) ** 1.6:   # the chicks drop out as it gets hurt
                    continue
                top = vo[-min(4, len(vo)):]
                y, g, _ = colour(strum('ACOUSTIC', top, 0.72, 0.008, dur=0.18, cap=0.6), KA, t, 0.7)
                mix.place('acoustic', y, tq, 0.26 * g, pan=PAN[KA])
        elif kind == 'hit' and e[2] == KA and e[7] == 'cone':
            tq = q16(t)
            n_landed += 1
            r = BASS[T.chord_at(tq)][0]
            mix.place('acoustic', colour(note('ACOUSTIC', r - 12 if r - 12 >= 40 else r, 0.3, 0.6, 1.0), KA, t)[0],
                      tq, 0.18, pan=PAN[KA])
            mix.place('fx', A.thud(0.25, 180, 80, seed=int(t * 9)), tq, 0.12, pan=PAN[KA])     # a slap on the body
        elif kind == 'hit' and e[2] == KA and e[7] == 'key':   # a FINGERPICK dart lands: a harmonic ping
            tq = q16(t)
            m = T.scale_notes(76, 93, T.chord_at(tq))[int(e[8]) % 6]
            mix.place('acoustic', A.chime(m, 0.6), tq, 0.10, pan=PAN[KA])


def abilities(mix):
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.05 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            mix.place('fx', hoof(0.6, True, int(t * 13)), t, 0.06)
        elif kind == 'slam':                                   # BIG STRUM: the giant downstroke
            ts = e[0]
            mix.place('acoustic', strum('ACOUSTIC', T.AC_CHORD[T.chord_at(ts)], 1.0, 0.016, dur=0.9, cap=1.8), ts, 0.62,
                      pan=PAN[KA])
            mix.place('acoustic', note('ACOUSTIC', 40, 0.9, 1.6, 1.0), ts, 0.4, pan=PAN[KA])
            mix.place('fx', A.thud(0.7, 90, 32), ts, 0.55)
            mix.place('fx', whip(1.0, 7), ts - WHIP_PRE, 0.30)
        elif kind == 'ability':
            who, name = e[2], e[3]
            tq = q16(t)
            if name == 'CHUNK':                                # the percussive chunk strum
                vo = T.UKE_CHORD[T.chord_at(tq)]
                mix.place('uke', A.lp(strum('UKULELE', vo, 1.0, 0.004, dur=0.05, cap=0.12), 3000), tq, 0.5, pan=PAN[KU])
                n = int(0.07 * SR)
                mix.place('fx', A.bp(np.random.default_rng(5).normal(0, 1, n), 600, 3500) * np.exp(-A.tt(n) / 0.012), tq, 0.22)
                mix.place('fx', A.thud(0.4, 130, 45), tq, 0.3)
            elif name == 'SPEED STRUM':                        # a quick tremolo-strum flourish to start it
                vo = T.UKE_CHORD[T.chord_at(tq)]
                for j in range(4):
                    mix.place('uke', strum('UKULELE', vo, 0.7, 0.005, up=bool(j % 2), dur=0.12, cap=0.4),
                              tq + j * S16 / 2, 0.22, pan=PAN[KU])
            elif name == 'UKE-NADO':                           # a whirl of strums spiralling up, with the wind
                for j in range(16):
                    ch = T.chord_at(tq + j * S16 / 2)
                    vo = tuple(m + 12 * (j // 8) for m in T.UKE_CHORD[ch])
                    mix.place('uke', strum('UKULELE', vo, 0.6 + 0.025 * j, 0.004, up=bool(j % 2), dur=0.1, cap=0.35),
                              tq + j * S16 / 2, 0.20, pan=0.6 * math.sin(j * 1.3))
                mix.place('fx', A.whoosh(2 * T.BEAT, 300, 6000), tq, 0.18)
            elif name == 'CAPO':                               # the clamp: a metal click and a ring
                n = int(0.05 * SR)
                mix.place('fx', A.hp(np.random.default_rng(8).normal(0, 1, n), 3000) * np.exp(-A.tt(n) / 0.004), tq, 0.18)
                mix.place('fx', A.chime(96, 0.4), tq, 0.06)
            elif name == 'FINGERPICK':                         # the darts leave as a picked arpeggio
                ab = RD.CFG[who]['abilities']['unlock2']
                n = ab['n']
                ms = [m for m in T.AC_CHORD[T.chord_at(tq)][2:]] + [T.AC_CHORD[T.chord_at(tq)][-1] + 12]
                for j in range(n):
                    te = t + ab['beats'] * T.BEAT * j / max(1, n - 1)
                    mix.place('acoustic', colour(note('ACOUSTIC', ms[j % len(ms)], 0.3, 0.7, 0.85), KA, te)[0], q16(te),
                              0.30, pan=PAN[KA] + 0.1 * (j - n / 2) / n)
            elif name == 'BIG STRUM':                          # the pick winds up across the strings
                mix.place('fx', scrape(2 * T.BEAT), tq, 0.12)


def main():
    mix = A.Mix(T.DUR + 4.0)
    # ------------------------------------------------ count-in (bar 0, on E): whip cracks + both fighters' strums
    for j, (tc, txt) in enumerate(T.COUNT[:3]):
        tc = max(tc, 0.004)                                # the "3" 4 ms into the file: no click at the seam
        mix.place('fx', whip(1.0, j), tc - WHIP_PRE, 0.30)
        mix.place('uke', strum('UKULELE', T.UKE_CHORD['E'], 0.9), tc, 0.30, pan=PAN[KU])
        mix.place('acoustic', strum('ACOUSTIC', T.AC_CHORD['E'], 0.9, 0.013), tc, 0.30, pan=PAN[KA])
        mix.place('twang', twang(40, 0.9 * T.BEAT), tc, 0.42)
        mix.place('perc', thump(1.0), tc, 0.36)
    for st, v in ((12, 0.5), (13, 0.65), (14, 0.8), (15, 1.0)):   # the gallop's pickup into FIGHT!
        mix.place('perc', hoof(v, st % 2 == 0, st), st * S16, 0.18)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), 3 * T.BEAT, 0.14)
    mix.place('bell', bell(45, 1.0), T.BAR, 0.34)
    mix.place('fx', whip(1.0, 9), T.BAR - WHIP_PRE, 0.30)
    # ------------------------------------------------ the bed: bars 1 .. the K.O.
    bar_ko = int(T_KO // T.BAR)
    for b in range(1, bar_ko + 1):
        bed(mix, b)
    whistle_phrase(mix, 1)
    whistle_phrase(mix, 17)
    mix.place('fx', A.whoosh(3.6, 300, 6000), 44.0 - 3.6, 0.10)
    for k in ('perc', 'twang', 'choir', 'whistle', 'strings', 'bell'):
        if k in mix.buses:
            mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ the fighters + their abilities
    uke_part(mix)
    ac_part(mix)
    abilities(mix)
    # ------------------------------------------------ the K.O. moment
    mix.place('bell', bell(40, 1.0, 5.0), T_KO, 0.40)
    mix.place('fx', whip(1.0, 13), T_KO - WHIP_PRE, 0.34)
    mix.place('fx', A.thud(0.9, 120, 30), T_KO, 0.7)
    loser = NAMES[LOSE]
    snap = (64, 59, 55, 50, 45, 40) if loser == 'ACOUSTIC' else (69, 64, 60, 67)
    for j, m in enumerate(snap):                           # the loser's strings snapping, top to bottom
        nm = loser
        x = note(nm, m, 0.2, 0.5, 1.0)
        mix.place('solo', A.hp(x, 200), RD.T_HS + 0.09 * j, 0.30, pan=(j - len(snap) / 2) / 6)
        n = int(0.04 * SR)
        mix.place('fx', A.hp(np.random.default_rng(40 + j).normal(0, 1, n), 3000) * np.exp(-A.tt(n) / 0.006),
                  RD.T_HS + 0.09 * j, 0.10)
    n = int(0.3 * SR)                                      # the body cracking
    mix.place('fx', A.bp(np.random.default_rng(77).normal(0, 1, n), 150, 1200) * np.exp(-A.tt(n) / 0.05), RD.T_HS, 0.35)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.16)
    # the winner's stinger on "(WINNER) WINS"
    tw = q16(T_KO + 1.3)
    wname = NAMES[WIN]
    if wname == 'UKULELE':
        mix.place('solo', strum('UKULELE', (69, 72, 76, 81), 1.0, 0.012, dur=0.9, cap=1.4), tw, 0.46)
    else:
        mix.place('solo', strum('ACOUSTIC', (45, 52, 57, 60, 64, 69), 1.0, 0.014, dur=0.9, cap=1.8), tw, 0.46)
    mix.place('bell', bell(45, 0.8), tw, 0.26)
    mix.place('twang', twang(45, 1.0), tw, 0.45)
    solo0 = RD.T_SOLO
    mix.place('fx', A.whoosh(solo0 - tw - 0.1, 300, 5000), tw + 0.1, 0.10)
    # ------------------------------------------------ victory: a lighter gallop + the winner's solo
    b_solo = int(round(solo0 / T.BAR))
    for b in range(b_solo, 32):
        bed(mix, b, light=True)
    for bt, m, d in T.SOLO[wname]:
        ts = solo0 + bt * T.BEAT
        if isinstance(m, tuple):
            mix.place('solo', strum(wname, m, 0.9, 0.010, up=(bt % 1) > 0.1, dur=max(0.12, d * T.BEAT), cap=1.0), ts, 0.40)
        else:
            mix.place('solo', note(wname, m, max(0.1, d * T.BEAT * 0.9), 0.8, 0.9), ts, 0.48)
    # the loser reassembles: its pieces fly home as a rising run, a glass "pop" when it's whole
    run = [52, 57, 60, 64, 69, 72, 76, 81] if loser == 'ACOUSTIC' else [60, 64, 67, 69, 72, 76, 79, 81]
    for j, m in enumerate(run):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
        mix.place('solo', note(loser, m, 0.2, 0.5, 0.7), tj, 0.22)
    mix.place('fx', A.chime(95, 0.5) + A.chime(102, 0.5) * 0.5, T.T_GLIDE, 0.12)
    # ------------------------------------------------ bar 32 (E): ride into the "3" on sample 0
    t32 = 32 * T.BAR
    tail = T.DUR - t32
    k = 0
    while k * S16 / 2 < tail:                              # the hooves break into a full gallop (32nds) and swell
        u = k * S16 / 2 / tail
        mix.place('perc', hoof(0.4 + 0.6 * u, k % 2 == 0, 300 + k), t32 + k * S16 / 2, 0.15 + 0.05 * u)
        k += 1
    for bt in range(4):
        mix.place('perc', thump(0.8 + 0.2 * bt / 3), t32 + bt * T.BEAT, 0.36)
        mix.place('twang', twang((40, 44, 47, 52)[bt], 0.9 * T.BEAT), t32 + bt * T.BEAT, 0.40)
    mix.place('fx', riser(tail), t32, 0.22)
    mix.place('choir', choir((56, 59, 64, 68), tail + 0.02, att=0.8, rel=0.001), t32, 0.5)
    mix.place('whistle', whistle(76, tail - 0.08, 1.0, 71), t32 + 0.05, 0.07)
    # caption landings: soft thuds
    for tl, k_, nch in impact_landings(RD.CAPTIONS):
        mix.place('fx', A.thud(), tl, 0.08 if k_ == 0 else 0.06)
    out, stems = master(mix)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    for k_, s in stems.items():
        A.write(os.path.join(HERE, 'check', f'stem_{k_}.wav'), s)
    spans = [(0, T.BAR), (T.BAR, 8), (8, 20), (20, 32), (32, 44), (44, T_KO), (T_KO, solo0), (solo0, 58), (58, 61)]
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


BUS_GAIN = {'uke': 1.45, 'acoustic': 1.35, 'solo': 1.2, 'fx': 1.3, 'perc': 1.0, 'twang': 0.55, 'whistle': 1.0,
            'choir': 1.0, 'bell': 0.9, 'strings': 1.0}
# section automation (dB), applied AFTER the compressor so the limiter can't flatten the build: the count-in at full,
# the early fight pulled back, rising through the final exchange, the victory section loud, the end close to the start
GAIN_PTS = [(0, 0.0), (1.55, 0.0), (2.1, -1.6), (20, -1.5), (30, -0.6), (40, 1.3), (44, 2.3), (50.8, 3.3), (52.3, 2.8),
            (53.4, 3.3), (58.0, 3.3), (60.0, 1.6), (61.02, 0.6)]


def master(mix, thr=0.22, ceiling=0.89):
    """Mix.master with the overflow past the loop (tails, reverb) folded onto the start; NA samples (whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('whistle', 0.9), ('solo', 0.45), ('fx', 0.3), ('uke', 0.25),
                                                ('acoustic', 0.25), ('choir', 0.6), ('bell', 0.5), ('twang', 0.3),
                                                ('strings', 0.4), ('perc', 0.15)))
    wet = A.reverb(send, 2.8, 0.8) * 0.22
    full = sum(buses.values()) + wet

    def fold(x):
        y = x[:NS].copy()
        tail = x[NS:]
        y[:len(tail)] += tail[:NS]
        return y
    m = fold(full)
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
