"""#40 BALLFIGHT #14: Bari Sax vs Bass Clarinet - soundtrack (mood A: industrial construction site), all synthesised,
original.

The bed is a dark, driving construction site with NO bass instrument of its own: a tight distorted kick, an anvil
backbeat (an anvil ring over a short snare), metal ticks on the 8ths (16ths as it thickens), a struck-pipe ostinato tuned
to the chord tones up high (G4-G5), a distorted grinder chugging power chords on syncopated 16ths (high-passed: it never
touches the low end), sheet-metal crashes on the phrase, and as fills hydraulic hisses and air-wrench bursts. G minor
at 125.90 BPM (32 bars = 61 s), Gm Eb Cm D. Both fighters are low reeds, played from the honest run in their own Pixel
Pitch voices (#39's bari, #38's bass clarinet):
  BARI SAX       the bass line. Every swing is one huge honk on the bar's root, swelling to the bottom of the arc and
                 bent like a doppler pass (#39 v2); SKANK's off-beat swings honk up the octave. While it rolls between
                 swings it pumps the root and fifth on the beats (timeline.BARI_PUMP: an engine idling). The crane's
                 winch whirrs as the cable drops, the hook clanks home; a smash = a heavy steel thud + grit. GROWL a
                 gravelly growl; CABLE SNAP: the winch strains, the cable parts with a twang, a falling whistle, the crash.
  BASS CLARINET  the growling counter-line an octave up (#38's idiom, raised an octave): the peg going in plays the
                 bar's root (G3 etc.) with a growl, every 8th of a whirl plays timeline.COUNTER_8THS, a smash pops a slap
                 tongue, it growls on as it rolls; MULTIPHONICS adds the twelfth (two notes at once); SLAP TONGUE the big
                 slap; LIGHTS OUT a low growl swelling in the dark, the eruption a huge slap over a thunder thud.
As HP drops a fighter's part thins and dirties; the leader's part is fuller and louder.
Consistent loudness from frame 0 (project rule, Oct 26): the full groove plays under the title card; it builds with
parts and density (16th ticks, the third kick, grinder fills, more sheet metal), never with level. One master gain + a
compressor and a soft limiter: no section automation. Bar 0 is the VS title card (a bari honk, the thunder crack, a
bass-clarinet growl + slap, each over a unison machine hit; an air wrench as the doors open), bar 1 the count-in on the
dominant (D), the drop on FIGHT!. At the K.O. the bed tape-stops, the bass clarinet falls apart, the winner's stinger
and solo play over lighter machinery; the last bars build into the title slam on sample 0. Every hit is snapped to the
nearest 16th; tails past the loop fold onto the start.

    python audio.py            -> audio.wav (whole AAC frames) + stems in check/ for the band check"""
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
KX, KC = NAMES.index('BARI SAX'), NAMES.index('BASS CLARINET')
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames: loops sample-perfectly
S16 = T.S16
BXM = B.module('barisax_lowpoly')                       # #39's bari (read-only)
BCM = B.module('bassclarinet_lowpoly')                  # #38's bass clarinet (read-only)


def q16(t):
    return round(t / S16) * S16


M64 = (1 << 64) - 1


def h01(*xs):
    """deterministic hash in [0, 1) (splitmix64: neighbouring slots get unrelated values, #37's fix)"""
    z = 0x9E3779B97F4A7C15
    for x in xs:
        z = (z ^ (int(round(x * 1000)) & M64)) * 0xBF58476D1CE4E5B9 & M64
        z = (z + 0x9E3779B97F4A7C15) & M64
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & M64
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & M64
        z ^= z >> 31
    return z / float(1 << 64)


def thin(slot, key, hp, prev_dropped):
    """drop this note? (the hurt fighter's line thins: up to 30 % of its notes at low HP, never two in a row)"""
    return not prev_dropped and h01(slot, key) < 0.30 * (1 - hp) ** 1.6


# ---------------------------------------------------------------- the two fighters' voices (cached, level-matched)
_VC = {}


def _fit(x, cap):
    n = int(cap * SR)
    x = np.asarray(x, float)[:n].copy()
    if len(x) < n:
        x = np.pad(x, (0, n - len(x)))
    fd = min(len(x) // 3, int(0.02 * SR))
    x[-fd:] *= np.linspace(1, 0, fd)
    return x


def _norm(x, ref=0.16):
    r = np.sqrt((x[:int(0.12 * SR)] ** 2).mean()) + 1e-9
    return x * (ref / r)


def bari(m, dur, vel=0.9, honk=0.0):
    """one bari note; honk 0..1 = the hard-blown, growled honk"""
    key = ('x', round(m, 3), round(dur, 3), round(vel, 2), round(honk, 2))
    if key not in _VC:
        x = BXM.bari_tone(BXM._midi_hz(m), dur, vel, honk=honk)
        _VC[key] = _norm(_fit(B._resample(np.asarray(x, float)), dur + 0.12))
    return _VC[key]


def bcl(m, dur, vel=0.85, growl=0.0):
    """one bass clarinet note (the chalumeau reed)"""
    key = ('b', round(m, 3), round(dur, 3), round(vel, 2), round(growl, 2))
    if key not in _VC:
        x = BCM.bass_reed(BCM._midi_hz(m), dur, vel, growl=growl)
        _VC[key] = _norm(_fit(B._resample(np.asarray(x, float)), dur + 0.16))
    return _VC[key]


def slap(m, vel=1.0):
    key = ('s', round(m, 2), round(vel, 2))
    if key not in _VC:
        x = BCM.slap(BCM._midi_hz(m), vel)
        _VC[key] = B._resample(np.asarray(x, float)) * 0.9
    return _VC[key]


def gliss(x, semis, dur):
    """slide into the note: start `semis` below (negative: above) and glide to pitch over `dur` s (a time warp)"""
    n = len(x)
    t = A.tt(n)
    off = -semis * (1 - np.clip(t / max(dur, 1e-3), 0, 1)) ** 1.5
    rate = 2 ** (off / 12)
    pos = np.cumsum(rate) - rate[0]
    return np.interp(pos, np.arange(n), x, right=0.0)


def bend(x, semis, t0, t1):
    n = len(x)
    t = A.tt(n)
    off = semis * np.clip((t - t0) / max(1e-3, t1 - t0), 0, 1)
    rate = 2 ** (off / 12)
    pos = np.cumsum(rate) - rate[0]
    return np.interp(pos, np.arange(n), x, right=0.0)


def doppler(x, t_bot, up=1.6, down=2.6, width=0.10):
    """a pass: sharp (+up semitones) coming in, through the pitch at t_bot (s into x), sliding flat (-down) going away,
    swelling to the pass (#39 v2's swing honk)"""
    n = len(x)
    t = A.tt(n)
    z = (t - t_bot) / width
    off = np.where(z < 0, up, down) * -np.tanh(z)
    rate = 2 ** (off / 12)
    pos = np.cumsum(rate) - rate[0]
    y = np.interp(pos, np.arange(n), x, right=0.0)
    return y * (0.5 + 0.5 * np.exp(-((t - t_bot) / (2.2 * width)) ** 2))


def hp_at(k, t):
    return RD.hp_at(k, t)


def colour(x, k, t, drive_k=1.0):
    """the fighter's tone at time t: thinner + dirtier as its HP drops; returns (signal, gain, lead). The bari is the
    bass line, so its thinning high-pass stops at ~110 Hz (it keeps its fundamental; the rest of the rule holds)"""
    hp = hp_at(k, t) / 100.0
    lead = (hp_at(k, t) - hp_at(1 - k, t)) / 100.0
    cut = 40 + (70 if k == KX else 200) * (1 - hp) ** 1.6
    y = A.hp(x, cut)
    drive = 1 + 4.0 * drive_k * (1 - hp) ** 2
    y = np.tanh(drive * y * 1.2) / np.tanh(drive * 1.2) * (1.0 / (1 + 0.15 * (drive - 1)))
    g = 10 ** (float(np.clip(6.0 * lead, -4.0, 3.5)) / 20)
    return y, g, lead


# ---------------------------------------------------------------- the construction site (original)
_BC = {}


def _modes(f0, modes, dur, click=0.3, seed=0, click_band=(2500, 10000)):
    """a struck metal object: damped partials (ratio, amp, decay s) over f0 + a strike click"""
    n = int(dur * SR)
    t = A.tt(n)
    x = np.zeros(n)
    r = np.random.default_rng(seed)
    for ratio, a, d in modes:
        x += a * np.sin(2 * np.pi * f0 * ratio * t + r.uniform(0, 2 * np.pi)) * np.exp(-t / d)
    x += click * A.bp(r.normal(0, 1, n), *click_band) * np.exp(-t / 0.004)
    return x * np.clip(t / 0.0008, 0, 1)


def anvil(seed=0):
    """an anvil strike: a bright inharmonic ring (~1.2 kHz) and a hard click"""
    key = ('anvil', seed)
    if key not in _BC:
        x = _modes(1180 * (1 + 0.01 * seed), ((1.0, 1.0, 0.22), (2.71, 0.6, 0.10), (5.18, 0.35, 0.05), (8.3, 0.2, 0.025)),
                   0.4, click=0.6, seed=seed)
        _BC[key] = A.hp(x, 700) * 0.55
    return _BC[key]


SNARE = A.hp(A.snare(0.10), 220)


def backbeat(seed=0):
    """the industrial snare: a short snare body + the anvil ring"""
    key = ('bb', seed % 4)
    if key not in _BC:
        a = anvil(seed % 4)
        x = np.zeros(max(len(SNARE), len(a)))
        x[:len(SNARE)] += SNARE * 0.8
        x[:len(a)] += a
        _BC[key] = x
    return _BC[key]


def pipe(m, seed=0):
    """a struck steel pipe tuned to midi m: free-bar partials (1, 2.76, 5.40), a slow beat between two near modes"""
    key = ('pipe', m, seed % 3)
    if key not in _BC:
        f = A.mtof(m)
        x = _modes(f, ((1.0, 1.0, 0.45), (1.004, 0.5, 0.45), (2.756, 0.45, 0.16), (5.404, 0.22, 0.06)), 0.7, click=0.25,
                   seed=seed % 3 + 20)
        _BC[key] = A.hp(x, 420) * 0.6
    return _BC[key]


def tick(seed=0, open_=False):
    """a metal tick on the 8ths (a chain link / rivet): a short bright noise + a tiny high ring"""
    key = ('tick', seed % 3, open_)
    if key not in _BC:
        h = A.hat(0.06 if open_ else 0.022, open_=open_)
        n = len(h)
        t = A.tt(n)
        _BC[key] = h + 0.12 * np.sin(2 * np.pi * (6100 + 300 * (seed % 3)) * t) * np.exp(-t / 0.02)
    return _BC[key]


KICK = A.hp(np.tanh(2.2 * A.hp(A.kick(1.0, 0.08, 170, 64)[:int(0.17 * SR)], 60)) * 0.75, 70)   # tight, distorted,
                                                                  # out of the bari's way (the bass is the bari's)


def grind(ch, dur=0.09, seed=0):
    """the grinder: a distorted power chord (root, fifth, octave) up in the 4th / 5th octave, gated short, high-passed
    at 480 Hz so the low end stays the fighters'"""
    key = ('gr', ch, round(dur, 3), seed % 2)
    if key not in _BC:
        root = 62 + (T.CHORD_ROOT[ch] - 62) % 12                     # D4..C#5
        n = int((dur + 0.02) * SR)
        t = A.tt(n)
        x = np.zeros(n)
        for m in (root, root + 7, root + 12):
            f = A.mtof(m)
            for dt in (-0.004, 0.004):
                ph = (f * (1 + dt) * t + 0.3 * seed) % 1.0
                x += 2 * ph - 1
        x = np.tanh(5.0 * x / 6)
        env = np.clip(t / 0.003, 0, 1) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.008))
        _BC[key] = A.lp(A.hp(x * env, 480), 4200) * 0.5
    return _BC[key]


def sheet(seed=0, dur=1.2):
    """a sheet-metal crash: many inharmonic modes (220 Hz-3.4 kHz) wobbling as the sheet flexes, high-passed"""
    key = ('sheet', seed, round(dur, 2))
    if key not in _BC:
        r = np.random.default_rng(seed + 50)
        modes = [(r.uniform(220, 3400) / 220, r.uniform(0.3, 1.0), r.uniform(0.15, 0.7)) for _ in range(16)]
        x = _modes(220, modes, dur, click=0.4, seed=seed + 50, click_band=(800, 9000))
        t = A.tt(len(x))
        x *= 1 + 0.35 * np.sin(2 * np.pi * 6.5 * t) * np.exp(-t / 0.5)
        x += 0.5 * A.bp(r.normal(0, 1, len(x)), 1500, 9000) * np.exp(-t / 0.25)
        _BC[key] = A.hp(x, 400, 4) * 0.16
    return _BC[key]


def hiss(dur=0.5, seed=0):
    """a hydraulic hiss: high air noise, a fast attack, the filter falling as the pressure drops (pssshh)"""
    key = ('hiss', round(dur, 2), seed)
    if key not in _BC:
        n = int(dur * SR)
        t = A.tt(n)
        x = np.random.default_rng(seed + 70).normal(0, 1, n)
        lo = A.bp(x, 1200, 6000)
        hi = A.bp(x, 4000, 11000)
        k = np.clip(t / dur, 0, 1)
        y = lo * (0.4 + 0.6 * k) + hi * (1 - k)
        _BC[key] = y * np.clip(t / 0.02, 0, 1) * np.exp(-t / (dur * 0.45)) * 0.5
    return _BC[key]


def wrench(dur=0.4, seed=0):
    """an air wrench burst: a motor whine spinning up + the ratchet's clicks at ~32 Hz"""
    key = ('wr', round(dur, 2), seed)
    if key not in _BC:
        n = int(dur * SR)
        t = A.tt(n)
        f = 260 + 520 * (1 - np.exp(-t / 0.06))
        ph = 2 * np.pi * np.cumsum(f) / SR
        whine = A.bp(np.sign(np.sin(ph)) * 0.5 + np.sin(2 * ph) * 0.3, 300, 3200)
        r = np.random.default_rng(seed + 90)
        clicks = A.bp(r.normal(0, 1, n), 1500, 7000) * (np.sin(2 * np.pi * 32 * t) > 0.85)
        env = np.clip(t / 0.01, 0, 1) * np.clip((dur - t) / 0.03, 0, 1)
        _BC[key] = (whine * 0.35 + clicks * 0.7) * env
    return _BC[key]


def winch(dur, seed=0, rise=1.0):
    """the crane's winch (#39): a geared whirr (a buzzy tone sweeping `rise` octaves) + chain rattle"""
    n = int(dur * SR)
    t = A.tt(n)
    f = 110 * 2 ** (rise * t / dur)
    ph = 2 * np.pi * np.cumsum(f) / SR
    buzz = np.sign(np.sin(ph)) * 0.3 + np.sin(2 * ph) * 0.2
    r = np.random.default_rng(seed)
    rattle = A.bp(r.normal(0, 1, n), 2500, 8000) * (0.5 + 0.5 * np.sign(np.sin(2 * np.pi * 34 * t)))
    env = np.clip(t / 0.03, 0, 1) * np.clip((dur - t) / 0.03, 0, 1)
    return A.bp(buzz, 150, 3000) * env * 0.7 + rattle * env * 0.25


def thunder(dur=1.6, seed=0, crack=True):
    n = int(dur * SR)
    t = A.tt(n)
    r = np.random.default_rng(seed)
    x = A.lp(r.normal(0, 1, n), 140) * 3.0 * np.clip(t / 0.03, 0, 1) * np.exp(-t / (dur * 0.35))
    if crack:
        x = x + A.bp(r.normal(0, 1, n), 1500, 9000) * np.exp(-t / 0.03) * 0.9
    return x


def clank(m, seed=0):
    """a key, a hook or a ferrule hitting something: a short metallic ring"""
    n = int(0.3 * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = sum(np.sin(2 * np.pi * f * r * t) * np.exp(-t / d) for r, d in ((1.0, 0.09), (2.76, 0.045), (5.4, 0.02)))
    return x + 0.3 * A.bp(np.random.default_rng(seed).normal(0, 1, n), 2000, 8000) * np.exp(-t / 0.005)


def grit(dur=0.5, seed=0):
    """GROWL's gravel / a smash's grit: a rough, low, crackling noise burst"""
    n = int(dur * SR)
    t = A.tt(n)
    r = np.random.default_rng(seed)
    x = A.bp(r.normal(0, 1, n), 120, 900) * (0.6 + 0.4 * np.sign(np.sin(2 * np.pi * 31 * t)))
    return x * np.clip(t / 0.01, 0, 1) * np.exp(-t / (dur * 0.4))


def roar(dur, seed=3):
    n = int(dur * SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    x = A.bp(x, 300, 2600) * 0.5 + A.bp(x, 900, 5000) * 0.25
    t = A.tt(n)
    return x * np.clip(t / 0.25, 0, 1) * np.exp(-np.maximum(t - 0.6, 0) / (dur * 0.45))


def riser(dur, over=0.012, seed=11):
    """noise + saw riser that builds right up to (and `over` s past) its end: no fade, the loop seam stays loud"""
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


PIPE_PAT = (0, 2, 1, 2, 0, 3, 1, 2)                       # the pipe ostinato over the bar's chord tones (8ths)
GRIND_16 = (0, 3, 6, 10, 12)                              # the grinder's chug (16ths), more as it thickens


def band_bar(mix, b, dense=0.0, light=False, duck=()):
    """one bar of the construction site (bar index b). dense 0..1: the arrangement thickens (16th ticks, a third kick,
    pipe ghosts, grinder fills, sheet metal every 2 bars), never the level. duck = the beats where a swing honk bottoms
    out: the kick steps back there (the honk IS the downbeat)"""
    t0 = b * T.BAR
    ch = T.CHORDS[b % T.N_BARS]
    tones = T.chord_tones(67, 81, ch)
    for bt in range(4):
        tb = t0 + bt * T.BEAT
        if bt in (0, 2):
            mix.place('drums', KICK, tb, 0.12 if bt in duck else 0.34)
        elif dense > 0.8 and not light:                                 # later: an anvil ghost on 2 and 4's '&'
            mix.place('drums', anvil(b + bt + 7), tb + T.BEAT / 2, 0.10, pan=-0.2)
        if bt == 2 and dense > 0.4 and not light:                       # the third kick: on the '&' of 3
            mix.place('drums', KICK, tb + T.BEAT / 2, 0.18)
        if bt in (1, 3):
            mix.place('drums', backbeat(b * 4 + bt), tb, 0.34)
        for e in range(4):                                              # metal ticks: 8ths, 16ths later
            te = tb + e * S16
            if e % 2 == 0:
                mix.place('drums', tick(b + bt + e, open_=(e == 2 and dense > 0.3)), te, 0.11, pan=0.3)
            elif dense > 0.5 and not light:
                mix.place('drums', tick(b + e), te, 0.05, pan=0.35)
    for j, ix in enumerate(PIPE_PAT):                                   # the pipe ostinato on the 8ths
        m = tones[ix % len(tones)]
        mix.place('metal', pipe(m, b + j), t0 + j * T.BEAT / 2, 0.15 if j % 2 == 0 else 0.11, pan=-0.3 + 0.08 * (j % 3))
        if dense > 0.3 and j % 2 == 1 and not light:                    # ghost pipes between, a fifth up
            mix.place('metal', pipe(min(m + 7, 86), b + j + 5), t0 + j * T.BEAT / 2 + S16, 0.05, pan=0.3)
    gpos = GRIND_16 + ((14, 15) if dense > 0.6 and not light else ())
    for p16 in gpos:                                                    # the grinder's chug
        mix.place('grind', grind(ch, 0.075, p16), t0 + p16 * S16, 0.11 if p16 else 0.13, pan=0.2)
    if b % 4 == 2 or (dense > 0.5 and b % 2 == 0 and not light):         # sheet metal on the phrase
        mix.place('metal', sheet(b % 5), t0, 0.9)
    if (b - 2) % 4 == 3 and not light:                                  # the phrase's last bar: an air wrench + a hiss
        mix.place('fx', wrench(0.38, b), t0 + 3 * T.BEAT, 0.16, pan=-0.25)
        mix.place('fx', hiss(0.5, b), t0 + 3.5 * T.BEAT, 0.12, pan=0.3)


def band_hit(mix, t, ch, g=1.0):
    """a unison machine hit: kick + sheet metal + the anvil + a pipe chord + a grinder stab + a deep thud"""
    mix.place('drums', KICK, t, 0.5 * g)
    mix.place('drums', backbeat(int(t * 10)), t, 0.30 * g)
    mix.place('metal', sheet(int(t * 7) % 5, 1.6), t, 1.1 * g)
    for j, m in enumerate(T.chord_tones(67, 79, ch)[:3]):
        mix.place('metal', pipe(m, j), t, 0.16 * g, pan=(j - 1) * 0.3)
    mix.place('grind', grind(ch, 0.28, 1), t, 0.16 * g, pan=0.2)
    mix.place('fx', A.thud(0.6, 110, 36, seed=int(t * 50) + 3), t, 0.15 * g)


# ---------------------------------------------------------------- the fighters from the run
def busy_windows():
    """the times the bari is up on the crane (a cycle's hook .. the end of its swing; CABLE SNAP's hoist .. the crash):
    it doesn't pump then"""
    out = []
    for c in RD.CYCLES:
        if c['sw'] is not None:
            out.append((c['t_hook'] - 0.02, c['end'] + 0.05))
    for t0, k, spot, ts, t1, rad in RD.SNAPS:
        out.append((t0 - 0.02, t1 + 0.3))
    return out


def bari_part(mix):
    """the bass line: swing honks, the pump between swings, the crane, the smashes, GROWL, CABLE SNAP"""
    busy = busy_windows()
    dropped = False
    swing_on = [q16(q[11]) - S16 for q in RD.SWINGS if q[2] == KX]
    tb = T.T_FIGHT
    while tb < T_KO - 0.05:                                       # the pump: the root / fifth on the beats while it rolls
        if not any(a <= tb <= b for a, b in busy) and not any(abs(tb - s) < 0.12 for s in swing_on):
            slot = round(tb / S16)
            hp = hp_at(KX, tb) / 100.0
            dropped = thin(slot, 7, hp, dropped)
            if not dropped:
                m = T.pump_at(tb)
                beat = int(round(tb / T.BEAT)) % 4
                sig, g, lead = colour(bari(m, 0.24, 0.85, honk=0.45 if beat == 0 else 0.25), KX, tb, 0.6)
                mix.place('bari', sig, tb, (0.62 if beat == 0 else 0.50) * g, pan=0.15)
                if lead > 0.05 and beat == 0:                     # the leader: the octave on top (fuller)
                    mix.place('bari', colour(bari(m + 12, 0.24, 0.6, honk=0.2), KX, tb)[0], tb, 0.14 * g, pan=0.2)
        tb += T.BEAT
    x_dropped = False
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'cable' and e[2] == KX:                      # the cable drops (the winch), the hook clanks home
            tq = q16(t)
            mix.place('fx', winch(e[3] - t, int(t * 10), rise=-0.8), tq, 0.10, pan=0.15)
            mix.place('fx', clank(79, int(t * 13)), q16(e[3]), 0.12, pan=0.15)
            mix.place('fx', hiss(0.3, int(t * 3)), q16(e[3]), 0.06, pan=0.2)     # the hydraulics take the weight
        elif kind == 'swing' and e[2] == KX:                    # the swing honk: the bass on the swings (#39 v2)
            ts0, ts1, sk = e[11], e[12], e[13]
            tbot = q16((ts0 + ts1) / 2)
            ch = T.chord_at(tbot + 0.01)
            hp = hp_at(KX, t) / 100.0
            if sk:
                x_dropped = thin(round(tbot / S16), 11, hp, x_dropped)
                if x_dropped:
                    continue
            m = T.CHORD_ROOT[ch] + (12 if sk else 0)
            t_on = q16(ts0 - S16)                               # it honks as it's let go, swelling to the bottom
            dur = ts1 - t_on + 0.06
            sig, g, lead = colour(bari(m, dur, 0.95, honk=0.85 if not sk else 0.55), KX, t, 0.6)
            sig = doppler(sig, tbot - t_on, up=1.6 if not sk else 1.0, down=2.6 if not sk else 1.6, width=0.09)
            mix.place('bari', sig, t_on, (1.15 if not sk else 0.80) * g, pan=0.15)
            if lead > 0.05 and not sk:
                up8 = doppler(colour(bari(m + 12, dur, 0.7, honk=0.4), KX, t)[0], tbot - t_on, 1.6, 2.6, 0.09)
                mix.place('bari', up8, t_on, 0.20 * g, pan=0.2)
            mix.place('fx', A.whoosh(ts1 - ts0, 250, 3500), ts0, 0.10 if not sk else 0.07, pan=0.1)
        elif kind == 'smash' and e[2] == KX:                    # the wrecking ball connects
            tq = q16(t)
            sp = e[5]
            mix.place('fx', A.thud(0.45, 120, 38, seed=int(t * 10)), tq, 0.22 + 0.22 * sp)
            mix.place('fx', clank(67, int(t * 11)), tq, 0.10 + 0.08 * sp, pan=0.1)
            mix.place('fx', grit(0.25, int(t * 7)), tq, 0.12 + 0.12 * sp)
            if sp > 0.6:
                mix.place('metal', sheet(int(t) % 5, 0.9), tq, 0.6)
        elif kind == 'snap_aim' and e[2] == KX:                 # CABLE SNAP: the winch strains it up high
            mix.place('fx', winch(e[5] - t, 41, rise=1.3), q16(t), 0.16, pan=0.1)
        elif kind == 'cable_snap' and e[2] == KX:               # the cable parts: a twang, the whip, a falling whistle
            tq = q16(t)
            mix.place('fx', clank(98, 5) + clank(86, 6) * 0.6, tq, 0.22)
            mix.place('fx', A.whoosh(0.25, 3000, 9000), tq, 0.12)
            fall = [q for q in EV if q[1] == 'crater' and q[2] == KX and q[0] >= t]
            tf_ = (fall[0][0] - t) if fall else 0.4
            nf = int(tf_ * SR)
            tt_ = A.tt(nf)
            whistle = np.sin(2 * np.pi * np.cumsum(1800 * 2 ** (-1.6 * tt_ / tf_)) / SR) * (tt_ / tf_) ** 1.5 * 0.5
            mix.place('fx', whistle, tq, 0.10)
        elif kind == 'crater' and e[2] == KX:                   # the crash
            tq = q16(t)
            ch = T.chord_at(tq + 0.01)
            mix.place('fx', thunder(2.0, 77), tq, 0.55)
            mix.place('fx', A.thud(0.7, 100, 32, seed=17), tq, 0.40)
            band_hit(mix, tq, ch, 0.9)
            sig, g, _ = colour(bari(T.CHORD_ROOT[ch], 0.7, 1.0, honk=1.0), KX, t, 0.5)
            mix.place('bari', bend(sig, -1.5, 0.3, 0.7), tq, 0.70 * g, pan=0.0)
            mix.place('bari', colour(bari(T.CHORD_ROOT[ch] + 12, 0.5, 0.7, honk=0.5), KX, t)[0], tq, 0.18 * g)
        elif kind == 'ability' and e[2] == KX:
            tq = q16(t)
            if e[3] == 'GROWL':                                 # a gravelly growl + grit
                sig, g, _ = colour(bari(T.bass_root(tq), 0.6, 1.0, honk=1.0), KX, t, 0.8)
                mix.place('bari', bend(sig, -1.0, 0.1, 0.6), tq, 0.50 * g, pan=0.15)
                mix.place('fx', grit(0.6, int(t * 10)), tq, 0.30)
            elif e[3] == 'SKANK':
                mix.place('metal', sheet(3, 1.2), tq, 0.8)


def twin_on(k, t):
    return any(kk == k and t0 <= t < until for t0, kk, until in RD.TWINS)


def bcl_part(mix):
    """the counter-line an octave up: the peg's root, 8ths through every whirl, slap pops on the smashes, abilities"""
    dropped = False
    first = min(a[0] for a in RD.ANCH if a[1] == KC)              # FIGHT!: it rolls out of its corner, growling
    sig, g, _ = colour(bcl(55, first - T.T_FIGHT - 0.05, 0.75, growl=0.2), KC, T.T_FIGHT)
    mix.place('bcl', sig, T.T_FIGHT, 0.26, pan=-0.2)
    for t0, k, c, orbit, te in RD.ANCH:
        if t0 > T_KO + 0.01:
            break
        if k != KC:
            continue
        tq = q16(t0)
        root = T.bass_root(tq) + 12                               # the peg goes in: the root an octave up, growling
        sig, g, lead = colour(bcl(root, 0.9 * T.BEAT, 0.95, growl=0.35), KC, t0)
        mix.place('bcl', sig, tq, 0.46 * g, pan=-0.2)
        mix.place('fx', clank(84, int(t0 * 10)), tq, 0.05, pan=-0.2)
        tr = q16(te)                                              # it pulls the peg and rolls on: a held growl (fifth)
        nxt = min([a[0] for a in RD.ANCH if a[1] == KC and a[0] > t0 + 0.1] + [T_KO])
        if nxt - tr > 0.3:
            sig, g, _ = colour(bcl(T.bass_root(tr) + 19, min(nxt - tr, 2.2 * T.BEAT) - 0.05, 0.7, growl=0.1), KC, tr)
            mix.place('bcl', sig, tr, 0.24 * g, pan=-0.2)
        e8 = q16(t0 + T.BEAT / 2)
        while e8 < te - 0.02 and e8 < T_KO:                       # the rumble through the whirl, on the 8ths
            slot = round(e8 / S16)
            hp = hp_at(KC, e8) / 100.0
            dropped = thin(slot, 3, hp, dropped)
            if not dropped:
                m = T.counter_at(e8)
                sig, g, lead = colour(bcl(m, 0.42 * T.BEAT, 0.8, growl=0.25), KC, e8)
                mix.place('bcl', sig, e8, 0.42 * g, pan=-0.2)
                if twin_on(KC, e8):                               # MULTIPHONICS: the twelfth above at once
                    mix.place('bcl', colour(bcl(m + 19, 0.42 * T.BEAT, 0.6), KC, e8)[0], e8, 0.16 * g, pan=-0.1)
                if lead > 0.04 and slot % 4 == 0:                 # the leader: the octave under (fuller, still above
                    mix.place('bcl', colour(bcl(m - 12, 0.42 * T.BEAT, 0.6), KC, e8)[0], e8, 0.10 * g)   # the bass)
            e8 += T.BEAT / 2
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'hit' and e[2] == KC and e[7] == 'anchor':     # a smash: a slap-tongue pop on the line's pitch
            tq = q16(t)
            mix.place('bcl', colour(slap(T.counter_at(tq) + 12, 1.0), KC, t)[0], tq, 0.40, pan=-0.15)
            mix.place('fx', A.thud(0.3, 150, 55, seed=int(t * 9)), tq, 0.18)
        elif kind == 'snip' and e[2] == KC:                       # SLAP TONGUE: the big slap
            tq = q16(t)
            mix.place('bcl', slap(T.bass_root(tq) + 24, 1.0), tq, 0.55 if e[3] else 0.32, pan=-0.15)
            if e[3]:
                mix.place('fx', A.thud(0.35, 180, 60, seed=int(t * 7)), tq, 0.30)
        elif kind == 'snip_open' and e[2] == KC:                  # the tongue sticks to the reed: a tiny click
            mix.place('bcl', slap(84, 0.3), q16(t), 0.10)
        elif kind == 'ability' and e[2] == KC and e[3] == 'MULTIPHONICS':
            tq = q16(t)                                           # a split, buzzing chord: root + twelfth + 11th
            root = T.bass_root(tq) + 12
            for m, g_ in ((root, 0.34), (root + 19, 0.20), (root + 17.5, 0.11)):
                mix.place('bcl', bcl(m, 1.5 * T.BEAT, 0.9, growl=0.2), tq, g_, pan=-0.2)
        elif kind == 'lights_out' and e[2] == KC:                 # LIGHTS OUT: a low growl swelling in the dark
            tq = q16(t)
            dur = e[3] - t
            x = bcl(46, dur, 1.0, growl=0.6)
            mix.place('bcl', x * np.linspace(0.5, 1.2, len(x)), tq, 0.40, pan=-0.2)
            mix.place('fx', A.whoosh(0.8, 200, 4000, rise=False), tq, 0.18)
        elif kind == 'erupt' and e[2] == KC:                      # ... and erupts
            tq = q16(t)
            mix.place('bcl', slap(T.bass_root(tq) + 24, 1.0), tq, 0.65, pan=-0.1)
            mix.place('fx', thunder(1.6, int(t * 10)), tq, 0.45)
            mix.place('metal', sheet(4, 1.4), tq, 1.0)


def main():
    mix = A.Mix(T.DUR + 4.0)
    bar_ko = int(T_KO // T.BAR)
    # ------------------------------------------------ bar 0: the VS title card over the full groove
    t1, t2, t3 = 0.004, T.BEAT, 2 * T.BEAT                 # BARI SAX 4 ms in (no click at the seam), VS, BASS CLARINET
    band_bar(mix, 0, dense=0.7)
    band_hit(mix, t1, 'Gm', 0.85)
    band_hit(mix, t2, 'Gm', 0.8)
    band_hit(mix, t3, 'Gm', 0.8)
    mix.place('bari', bari(43, 0.5, 1.0, honk=1.0), t1, 0.62)  # BARI SAX: a huge honk on the root
    mix.place('bari', bari(55, 0.4, 0.8, honk=0.6), t1 + 0.02, 0.20)
    mix.place('fx', thunder(1.4, 91), t2, 0.55)              # VS: the thunder crack where it splits the divider
    mix.place('bcl', bcl(55, 0.9 * T.BEAT, 1.0, growl=0.7), t3, 0.50, pan=-0.2)   # BASS CLARINET: a growl + slap
    mix.place('bcl', bcl(62, 0.9 * T.BEAT, 0.8, growl=0.4), t3, 0.22, pan=-0.2)
    mix.place('bcl', slap(67, 1.0), t3 + 0.004, 0.50, pan=-0.2)
    mix.place('fx', wrench(T.T_DOORS[1] - T.T_DOORS[0] + 0.1, 3), T.T_DOORS[0] - 0.05, 0.18)   # the doors open
    mix.place('fx', A.whoosh(T.T_DOORS[1] - T.T_DOORS[0] + 0.1, 300, 5000), T.T_DOORS[0] - 0.05, 0.12)
    # ------------------------------------------------ bar 1: the count-in on the dominant (D)
    band_bar(mix, 1, dense=0.4)
    for tc, txt in T.COUNT[:3]:
        band_hit(mix, tc, 'D', 0.75)
        mix.place('bari', bari(38, 0.22, 0.9, honk=0.5), tc, 0.40, pan=0.15)      # the bari counts on D
    for j in range(4):                                       # an anvil fill into FIGHT!
        mix.place('drums', anvil(j), T.BAR + 3 * T.BEAT + j * S16, 0.26 + 0.05 * j)
    mix.place('fx', hiss(T.BEAT, 9), T.BAR + 3 * T.BEAT, 0.12)
    band_hit(mix, T.T_FIGHT, 'Gm', 1.1)                      # the drop
    mix.place('fx', thunder(1.6, 7, crack=False), T.T_FIGHT, 0.35)
    # ------------------------------------------------ the bed: bar 2 (FIGHT!) .. the K.O. (thickens as it goes)
    duck = {}                                                # the beats where a swing honk bottoms out
    for e in EV:
        if e[1] == 'swing' and e[2] == KX and e[13] == 0 and e[0] <= T_KO:
            tb = q16((e[11] + e[12]) / 2)
            duck.setdefault(int(tb // T.BAR), set()).add(int(round((tb % T.BAR) / T.BEAT)) % 4)
    for b in range(2, bar_ko + 1):
        band_bar(mix, b, dense=min(1.0, (b - 2) / 18.0), duck=duck.get(b, ()))
    for t_, who, name in RD.STORY['caps']:                   # a machine hit under every captioned ability
        band_hit(mix, q16(t_), T.chord_at(t_), 0.55)
    for k in ('drums', 'metal', 'grind'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ the two fighters, from the run
    bari_part(mix)
    bcl_part(mix)
    last_clash = -9.0
    for e in EV:                                             # contacts: a light thud, at most every 0.15 s
        if e[0] > T_KO:
            break
        if e[1] == 'clash' and e[0] - last_clash > 0.15:
            last_clash = e[0]
            mix.place('fx', A.thud(0.15, 200, 90, seed=int(e[0] * 3)), e[0], 0.05)
    # ------------------------------------------------ the K.O. moment
    band_hit(mix, T_KO, 'Gm', 1.0)
    mix.place('fx', thunder(2.4, 51), T_KO, 0.55)
    if NAMES[LOSE] == 'BASS CLARINET':                       # the bass clarinet falls apart: a falling growl, keys
        x = bcl(55, 1.0, 1.0, growl=0.5)
        mix.place('bcl', bend(x, -9.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x)), RD.T_HS, 0.42)
    else:
        x = bari(43, 1.0, 1.0, honk=0.8)
        mix.place('bari', bend(x, -7.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x)), RD.T_HS, 0.42)
    for j, m in enumerate((86, 81, 91, 77, 84)):
        mix.place('fx', clank(m, 20 + j), RD.T_HS + 0.08 * j, 0.07, pan=(j - 2) / 5)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.22)
    tw = q16(T_KO + 1.3)                                     # the winner's stinger on "(WINNER) WINS"
    band_hit(mix, tw, 'Gm', 1.0)
    if NAMES[WIN] == 'BARI SAX':
        mix.place('solo', bari(43, 0.8, 1.0, honk=1.0), tw, 0.55)
        mix.place('solo', bari(55, 0.8, 0.8, honk=0.6), tw + 0.02, 0.22)
    else:
        for j, m in enumerate((55, 62, 67)):
            mix.place('solo', bcl(m, 0.9 - 0.06 * j, 1.0, growl=0.2), tw + j * 0.03, 0.30)
        mix.place('solo', slap(67, 1.0), tw, 0.3)
    solo0 = RD.T_SOLO
    b_solo = int(round(solo0 / T.BAR))
    for b in range(bar_ko + 1, b_solo):                      # the site holds its breath, but never goes quiet
        tq = b * T.BAR
        ch = T.CHORDS[b]
        for bt in range(4):
            mix.place('drums', tick(bt), tq + bt * T.BEAT, 0.12, pan=0.3)
            mix.place('drums', tick(bt + 1), tq + bt * T.BEAT + T.BEAT / 2, 0.09, pan=0.3)
            mix.place('grind', grind(ch, 0.075, bt), tq + bt * T.BEAT + 3 * S16, 0.10, pan=0.2)
        for j, ix in enumerate(PIPE_PAT):
            mix.place('metal', pipe(T.chord_tones(67, 81, ch)[ix % 3], j), tq + j * T.BEAT / 2, 0.13, pan=-0.2)
        mix.place('drums', KICK, tq, 0.42)
        mix.place('drums', KICK, tq + 2 * T.BEAT, 0.36)
        mix.place('drums', backbeat(b), tq + T.BEAT, 0.26)
        mix.place('drums', backbeat(b + 1), tq + 3 * T.BEAT, 0.26)
        mix.place('metal', sheet(b % 5), tq, 0.8)
        if NAMES[WIN] == 'BARI SAX':                          # the champion keeps the bass pumping
            for bt in range(4):
                mix.place('bari', bari(T.bass_root(tq) + (7 if bt == 2 else 0), 0.24, 0.85, honk=0.4 if bt == 0 else 0.3),
                          tq + bt * T.BEAT, 0.62 if bt == 0 else 0.52, pan=0.15)
    # ------------------------------------------------ victory: lighter machinery + the winner's solo
    for b in range(b_solo, 30):
        band_bar(mix, b, dense=0.85)
    for bt, m, d in T.SOLO[NAMES[WIN]]:
        ts = solo0 + bt * T.BEAT
        if ts > T.T_REASSEMBLE - 0.05:
            break
        if NAMES[WIN] == 'BARI SAX':
            mix.place('solo', bari(m, d * T.BEAT * 0.9, 0.95, honk=0.7), ts, 0.78)
        else:
            mix.place('solo', bcl(m, d * T.BEAT * 0.95, 0.95, growl=0.15), ts, 0.62)
    # the loser reassembles: its pieces fly home as a rising run, a glass "pop" when it's whole
    run = [55, 58, 62, 65, 67, 70]
    for j, m in enumerate(run):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
        if NAMES[LOSE] == 'BASS CLARINET':
            mix.place('bcl', bcl(m, 0.12, 0.7), tj, 0.24, pan=-0.2)
        else:
            mix.place('bari', bari(m - 12, 0.12, 0.7), tj, 0.24, pan=0.15)
    mix.place('fx', A.chime(95, 0.5) + A.chime(102, 0.5) * 0.5, T.T_GLIDE, 0.12)
    # ------------------------------------------------ bars 30-31: the site returns, building into the title slam
    for b in (30, 31):
        band_bar(mix, b, dense=1.0)
        for bt in range(4):                                  # the bari pumps the bass under it (Eb, D)
            tb = b * T.BAR + bt * T.BEAT
            if tb < T.DUR - T.BEAT / 2:
                mix.place('bari', bari(T.pump_at(tb), 0.24, 0.85, honk=0.5 if bt == 0 else 0.3), tb,
                          0.78 if bt == 0 else 0.62, pan=0.15)
                if bt == 0:                                  # revving in its corner: the octave on top
                    mix.place('bari', bari(T.pump_at(tb) + 12, 0.24, 0.7, honk=0.3), tb, 0.22, pan=0.2)
    band_hit(mix, T.T_GLIDE, T.chord_at(T.T_GLIDE), 0.7)     # both glide home: a machine hit
    t31 = 31 * T.BAR
    st = 0.0
    while t31 + 2 * T.BEAT + st < T.DUR - 1e-4:              # an anvil roll into the seam
        mix.place('drums', anvil(int(st / S16)), t31 + 2 * T.BEAT + st, 0.10 + 0.14 * st / (2 * T.BEAT))
        st += S16
    mix.place('fx', riser(T.DUR - t31), t31, 0.20)
    mix.place('fx', wrench(0.5, 77), T.DUR - 0.55, 0.16)
    nc = int((T.T_CLOSE + 0.012) * SR)                       # the doors sliding shut into the title slam
    tcl = A.tt(nc)
    doors = A.bp(np.random.default_rng(93).normal(0, 1, nc), 400, 5000) * (tcl / T.T_CLOSE) ** 2
    mix.place('fx', doors, T.DUR - T.T_CLOSE, 0.45)
    for tl, k, nch in impact_landings(RD.CAPTIONS):         # caption landings: soft thuds
        mix.place('fx', A.thud(), tl, 0.08 if k == 0 else 0.06)
    out, stems = master(mix)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    for k, s in stems.items():
        A.write(os.path.join(HERE, 'check', f'stem_{k}.wav'), s)
    spans = [(0, T.BAR), (T.BAR, T.T_FIGHT), (T.T_FIGHT, 10), (10, 20), (20, 32), (32, 44), (44, T_KO), (T_KO, solo0),
             (solo0, 57.2), (57.2, 61)]
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


BUS_GAIN = {'bari': 1.6, 'bcl': 1.5, 'solo': 1.5, 'fx': 1.2, 'drums': 1.0, 'metal': 1.0, 'grind': 1.0}


def master(mix, thr=0.15, ceiling=0.89):
    """ONE master gain (no section automation: consistent loudness from frame 0) + a compressor and a soft limiter;
    the overflow past the loop (tails, reverb) folds onto the start; NA samples (whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('bari', 0.15), ('bcl', 0.25), ('solo', 0.35), ('fx', 0.25),
                                                ('metal', 0.4), ('grind', 0.15), ('drums', 0.15)))
    wet = A.reverb(send, dur=1.8, decay=0.5) * 0.20               # a big shed: the clangs ring
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
