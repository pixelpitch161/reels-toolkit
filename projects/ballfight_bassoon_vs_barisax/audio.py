"""#39 BALLFIGHT #13: Bassoon vs Bari Sax - soundtrack (mood A: dark ska-punk), all synthesised, original (plus Grieg's
public-domain Mountain King theme for MOUNTAIN KING).

The bed is fast, dark ska-punk with NO horns of its own: driving punk drums (kick, snare on 2 and 4, 8th hats, crashes;
double-time snare and ride as it thickens), an upstroke guitar skank on every off-beat (short, bright, choked chord
stabs) and an organ bubble (the off-beat chord plus its 16th bounce). E minor at 169.18 BPM (43 bars = 61 s). Both
fighters are low reeds, played from the honest run in their own voices (the new Pixel Pitch bassoon and bari):
  BASSOON   sneaky staccato up high: one dot per hop landing (timeline.BASSOON_8THS, E4-D#5; STACCATO's 16th
            landings take the passing note), a missed stomp ghosted, a backstab accented with a grace up the chord;
            BOCAL HOOK a comic blat sliding up + the crook's clank; MOUNTAIN KING = Grieg's theme in E minor, one note
            per hop, accelerating with the hops.
  BARI SAX  v2 (WRECKING BALL, Oct 26): the bass line rides the swings. Every swing is one huge honk on the bass root
            of the bar it bottoms out in (SKANK's swings: up the octave, on the off-beats), swelling to the bottom and
            bent like a doppler pass (sharp coming in, sliding flat as it goes by), over a rush of air; the crane's
            winch whirrs as the cable drops and the hook clanks home; a smash = a heavy steel-and-wood impact.
            GROWL a gravelly growl (unchanged). CABLE SNAP: the winch strains up, the cable parts with a twang, a
            falling whistle, then the crash (thunder, a band hit and the biggest honk). v1's honk-per-bar + ringing
            ropes live in v1/audio.py.
As HP drops a fighter's part thins and dirties; the leader's part is fuller and louder. At 169 BPM every hit note stays
short so the mix doesn't smear.
Consistent loudness from frame 0 (project rule, Oct 26): the band is at full level on the title card; it builds with
parts and density (open hats, ride, double-time snare, organ bubble, the ropes' bass chord), never with level. One
master gain + a compressor and a soft limiter: no section automation. Bar 0 is the VS title card (a bassoon blat, the
thunder crack, a bari honk, each over a unison band hit), bar 1 the count-in on the dominant (B), the drop on FIGHT!.
At the K.O. the band tape-stops, the loser falls apart, the winner's stinger and solo play over a lighter skank; the
last bars build into the title slam on sample 0. Every hit is snapped to the nearest 16th; tails past the loop fold
onto the start.

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
KB, KX = NAMES.index('BASSOON'), NAMES.index('BARI SAX')
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames: loops sample-perfectly
S16 = T.S16
BNM = B.module('bassoon_lowpoly')                       # the new bassoon's double reed (read-only)
BXM = B.module('barisax_lowpoly')                       # the new bari's honk


def q16(t):
    return round(t / S16) * S16


M64 = (1 << 64) - 1


def h01(*xs):
    """deterministic hash in [0, 1): a splitmix64 mixer (neighbouring slots get unrelated values; #37's fix)"""
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


def bsn(m, dur, vel=0.85, growl=0.0):
    """one bassoon note (staccato when short), dur s"""
    key = ('b', round(m, 3), round(dur, 3), round(vel, 2), round(growl, 2))
    if key not in _VC:
        x = BNM.bassoon_reed(BNM._midi_hz(m), dur, vel, growl=growl)
        _VC[key] = _norm(_fit(B._resample(np.asarray(x, float)), dur + 0.09))
    return _VC[key]


def bari(m, dur, vel=0.9, honk=0.0):
    """one bari note; honk 0..1 = the hard-blown, growled honk"""
    key = ('x', round(m, 3), round(dur, 3), round(vel, 2), round(honk, 2))
    if key not in _VC:
        x = BXM.bari_tone(BXM._midi_hz(m), dur, vel, honk=honk)
        _VC[key] = _norm(_fit(B._resample(np.asarray(x, float)), dur + 0.12))
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
    and swelling to the pass (a time warp + an envelope)"""
    n = len(x)
    t = A.tt(n)
    z = (t - t_bot) / width
    off = np.where(z < 0, up, down) * -np.tanh(z)
    rate = 2 ** (off / 12)
    pos = np.cumsum(rate) - rate[0]
    y = np.interp(pos, np.arange(n), x, right=0.0)
    return y * (0.5 + 0.5 * np.exp(-((t - t_bot) / (2.2 * width)) ** 2))


def winch(dur, seed=0, rise=1.0):
    """the crane's winch: a geared whirr (a buzzy tone sweeping up `rise` octaves) + chain rattle"""
    n = int(dur * SR)
    t = A.tt(n)
    f = 110 * 2 ** (rise * t / dur)
    ph = 2 * np.pi * np.cumsum(f) / SR
    buzz = np.sign(np.sin(ph)) * 0.3 + np.sin(2 * ph) * 0.2
    r = np.random.default_rng(seed)
    rattle = A.bp(r.normal(0, 1, n), 2500, 8000) * (0.5 + 0.5 * np.sign(np.sin(2 * np.pi * 34 * t)))
    env = np.clip(t / 0.03, 0, 1) * np.clip((dur - t) / 0.03, 0, 1)
    return A.bp(buzz, 150, 3000) * env * 0.7 + rattle * env * 0.25


def hp_at(k, t):
    return RD.hp_at(k, t)


def colour(x, k, t, drive_k=1.0):
    """the fighter's tone at time t: thinner + dirtier as its HP drops; returns (signal, gain, lead)"""
    hp = hp_at(k, t) / 100.0
    lead = (hp_at(k, t) - hp_at(1 - k, t)) / 100.0
    cut = 40 + 200 * (1 - hp) ** 1.6
    y = A.hp(x, cut)
    drive = 1 + 4.0 * drive_k * (1 - hp) ** 2
    y = np.tanh(drive * y * 1.2) / np.tanh(drive * 1.2) * (1.0 / (1 + 0.15 * (drive - 1)))
    g = 10 ** (float(np.clip(6.0 * lead, -4.0, 3.5)) / 20)
    return y, g, lead


# ---------------------------------------------------------------- the band (original): drums, guitar skank, organ
_BC = {}


def rope_pluck(m, dur=1.0, vel=1.0, seed=0):
    """a ring rope as a bass string: a plucked low tone (strong fundamental + a few harmonics decaying faster), a
    fibrous buzz on the attack, decaying like the rope's ring"""
    key = ('r', m, round(dur, 2), seed)
    if key not in _BC:
        n = int(dur * SR)
        t = A.tt(n)
        f = A.mtof(m)
        x = np.zeros(n)
        for k in range(1, 9):
            x += (1.0 / k ** 1.1) * np.sin(2 * np.pi * f * k * t + 0.3 * k) * np.exp(-t * (1.1 + 1.4 * k) / max(0.3, dur))
        buzz = A.bp(np.random.default_rng(seed + m).normal(0, 1, n), 300, 2500) * np.exp(-t / 0.025) * 0.4
        x = np.tanh(1.6 * (x + buzz)) * np.clip(t / 0.002, 0, 1)
        _BC[key] = x
    return _BC[key] * vel


def skank(ch, vel=1.0, seed=0):
    """the guitar's upstroke: a choked high triad (a few ms strum, bright, a pick scratch), ~70 ms long"""
    key = ('g', ch, seed)
    if key not in _BC:
        n = int(0.11 * SR)
        t = A.tt(n)
        x = np.zeros(n)
        tones = T.chord_tones(64, 79, ch)[-3:]
        for j, m in enumerate(reversed(tones)):                  # an upstroke: high string first
            d = int(j * 0.004 * SR)
            f = A.mtof(m)
            tt_ = t[:n - d]
            y = sum((1 / k) * np.sin(2 * np.pi * f * k * tt_ + k) for k in range(1, 7)) * np.exp(-tt_ / 0.035)
            x[d:] += y
        scratch = A.bp(np.random.default_rng(seed + 9).normal(0, 1, n), 2000, 7000) * np.exp(-t / 0.006) * 0.5
        x = A.hp(np.tanh(1.8 * (x / 3 + scratch)), 700)
        _BC[key] = x
    return _BC[key] * vel


def organ(ms, dur, vel=1.0):
    """a ska organ stab: drawbar harmonics (8' 4' 2 2/3' 2'), a key click, a little leslie wobble"""
    key = ('o', tuple(ms), round(dur, 3))
    if key not in _BC:
        n = int((dur + 0.02) * SR)
        t = A.tt(n)
        x = np.zeros(n)
        for m in ms:
            f = A.mtof(m)
            for h, a in ((1, 1.0), (2, 0.7), (3, 0.45), (4, 0.35), (6, 0.15)):
                x += a * np.sin(2 * np.pi * f * h * t * (1 + 0.002 * np.sin(2 * np.pi * 6.3 * t)))
        x = x / (len(ms) * 2.6)
        click = A.bp(np.random.default_rng(len(ms)).normal(0, 1, n), 1500, 6000) * np.exp(-t / 0.003) * 0.4
        env = np.clip(t / 0.003, 0, 1) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.01))
        _BC[key] = A.hp((x + click) * env, 280)                    # a thin, mid bubble: the low end is the bari's
    return _BC[key] * vel


def ride(seed=0):
    key = ('ride', seed)
    if key not in _BC:
        n = int(0.5 * SR)
        t = A.tt(n)
        x = A.hp(np.random.default_rng(seed + 31).normal(0, 1, n), 5000) * np.exp(-t / 0.18) * 0.35
        x += 0.12 * np.sin(2 * np.pi * 3100 * t) * np.exp(-t / 0.3)
        _BC[key] = x
    return _BC[key]


KICK = A.hp(A.kick(1.0, 0.10, 170, 58)[:int(0.2 * SR)], 55)       # a tight punk kick, out of the honk's way
_cl = A.clap()
SNARE = A.snare(0.11)
SNARE[:len(_cl)] += _cl * 0.35
HAT = A.hat(0.03)
OHAT = A.hat(open_=True)


def tom(m):
    key = ('tom', m)
    if key not in _BC:
        n = int(0.3 * SR)
        t = A.tt(n)
        f = A.mtof(m) * (1 + 0.3 * np.exp(-t / 0.03))
        _BC[key] = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.12)
    return _BC[key]


def thunder(dur=1.6, seed=0, crack=True):
    n = int(dur * SR)
    t = A.tt(n)
    r = np.random.default_rng(seed)
    rumble = A.lp(r.normal(0, 1, n), 140) * 3.0 * np.clip(t / 0.03, 0, 1) * np.exp(-t / (dur * 0.35))
    x = rumble
    if crack:
        x = x + A.bp(r.normal(0, 1, n), 1500, 9000) * np.exp(-t / 0.03) * 0.9
    return x


def clank(m, seed=0):
    """a key, a ferrule or the crook hitting something: a short metallic ring"""
    n = int(0.3 * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = sum(np.sin(2 * np.pi * f * r * t) * np.exp(-t / d) for r, d in ((1.0, 0.09), (2.76, 0.045), (5.4, 0.02)))
    return x + 0.3 * A.bp(np.random.default_rng(seed).normal(0, 1, n), 2000, 8000) * np.exp(-t / 0.005)


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


def grit(dur=0.5, seed=0):
    """GROWL's gravel: a rough, low, crackling noise burst"""
    n = int(dur * SR)
    t = A.tt(n)
    r = np.random.default_rng(seed)
    x = A.bp(r.normal(0, 1, n), 120, 900) * (0.6 + 0.4 * np.sign(np.sin(2 * np.pi * 31 * t)))
    return x * np.clip(t / 0.01, 0, 1) * np.exp(-t / (dur * 0.4))


ORG_V = {'Em': (52, 55, 59), 'C': (52, 55, 60), 'D': (54, 57, 62), 'B': (54, 59, 63)}    # the organ's mid voicings


def band_bar(mix, b, dense=0.0, light=False, honk_down=False, duck=()):
    """one bar of the band (bar index b). dense 0..1: the arrangement thickens (open hats, organ 16th bounce, ride,
    double-time snare, crash every 2 bars), never the level. honk_down: the bari honks on this bar's downbeat, so the
    kick steps back there (the honk IS the downbeat); v2: duck = the beats where a swing honk bottoms out (same)"""
    t0 = b * T.BAR
    ch = T.CHORDS[b % T.N_BARS]
    for bt in range(4):
        tb = t0 + bt * T.BEAT
        if bt in (0, 2) or (dense > 0.75 and not light):                 # kick 1 + 3 (every beat later: punk drive)
            mix.place('drums', KICK, tb, (0.18 if ((bt == 0 and honk_down) or bt in duck) else 0.42) if bt in (0, 2)
                      else 0.20)
        if bt in (1, 3):
            mix.place('drums', SNARE, tb, 0.36)
        elif dense > 0.55 and not light:                                  # double-time: a lighter snare on 1 + 3 '&'
            mix.place('drums', SNARE, tb + T.BEAT / 2, 0.11)
        for e in (0, 1):                                                  # hats on the 8ths, open on the '&' later
            te = tb + e * T.BEAT / 2
            if e == 1 and dense > 0.3:
                mix.place('drums', OHAT, te, 0.10, pan=0.25)
            else:
                mix.place('drums', HAT, te, 0.13, pan=0.25)
        if dense > 0.45 and not light:
            mix.place('drums', ride(bt), tb, 0.08, pan=-0.3)
        # the guitar skank: every off-beat
        mix.place('guitar', skank(ch, seed=b * 4 + bt), tb + T.BEAT / 2, 0.30 if not light else 0.24, pan=-0.35)
        # the organ bubble: the off-beat chord + its 16th bounce (the bounce comes in as it thickens)
        mix.place('organ', organ(ORG_V[ch], 0.07), tb + T.BEAT / 2, 0.20, pan=0.35)
        mix.place('organ', organ(ORG_V[ch], 0.05), tb + 3 * S16, 0.11, pan=0.35)
    if (b % 2 == 0 and dense > 0.6) or b % 8 == 2:
        mix.place('drums', A.crash(1.4), t0, 0.07)
    if b % 4 == 1 and not light:                                         # a tom fill into every 4th bar
        for j, m in enumerate((50, 47, 43)):
            mix.place('drums', tom(m), t0 + 3 * T.BEAT + j * S16 + S16, 0.22)


def band_hit(mix, t, ch, g=1.0):
    """a unison band hit: kick + crash + a long skank + organ + a bass thud on the chord's root"""
    mix.place('drums', KICK, t, 0.5 * g)
    mix.place('drums', A.crash(1.6), t, 0.10 * g)
    mix.place('drums', SNARE, t, 0.25 * g)
    mix.place('guitar', skank(ch, 1.0, seed=int(t * 50)), t, 0.36 * g, pan=-0.3)
    mix.place('organ', organ(ORG_V[ch], 0.3), t, 0.26 * g, pan=0.3)
    mix.place('ropes', rope_pluck(T.ROPE_NOTES[ch][0], 1.0, seed=int(t * 9)), t, 0.40 * g)
    mix.place('fx', A.thud(0.6, 110, 36, seed=int(t * 50) + 3), t, 0.15 * g)


def main():
    mix = A.Mix(T.DUR + 4.0)
    bar_ko = int(T_KO // T.BAR)
    # ------------------------------------------------ bar 0: the VS title card over the band at full level
    t1, t2, t3 = 0.004, T.BEAT, 2 * T.BEAT                 # BASSOON 4 ms in (no click at the seam), VS, BARI SAX
    band_bar(mix, 0, dense=0.7)
    band_hit(mix, t1, 'Em', 0.85)
    band_hit(mix, t2, 'Em', 0.8)
    band_hit(mix, t3, 'Em', 0.8)
    x = bsn(40, 0.35, 1.0, growl=0.25)                     # BASSOON: a comic low blat sliding up
    mix.place('bassoon', gliss(x, -3.0, 0.08), t1, 0.48)
    mix.place('bassoon', bsn(52, 0.12, 0.9), t1 + 0.20, 0.30)
    mix.place('fx', thunder(1.4, 91), t2, 0.55)              # VS: the thunder crack where it splits the divider
    mix.place('bari', bari(40, 0.5, 1.0, honk=1.0), t3, 0.55)  # BARI SAX: a huge honk on the root
    mix.place('bari', bari(52, 0.4, 0.8, honk=0.6), t3 + 0.02, 0.20)
    for j in range(4):                                       # beat 4: a snare pickup as the doors open
        mix.place('drums', SNARE, 3 * T.BEAT + j * S16, 0.18 + 0.05 * j)
    mix.place('fx', A.whoosh(T.T_DOORS[1] - T.T_DOORS[0] + 0.1, 300, 5000), T.T_DOORS[0] - 0.05, 0.14)
    # ------------------------------------------------ bar 1: the count-in on the dominant (B)
    band_bar(mix, 1, dense=0.4)
    for tc, txt in T.COUNT[:3]:
        band_hit(mix, tc, 'B', 0.75)
    for j in range(4):
        mix.place('drums', tom((50, 47, 45, 43)[j]), T.BAR + 3 * T.BEAT + j * S16, 0.26)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), T.BAR + 3 * T.BEAT, 0.12)
    band_hit(mix, T.T_FIGHT, 'Em', 1.1)                      # the drop
    mix.place('fx', thunder(1.6, 7, crack=False), T.T_FIGHT, 0.35)
    # ------------------------------------------------ the band: bar 2 (FIGHT!) .. the K.O. (thickens as it goes)
    honk_bars = {int(round(e[0] / T.BAR)) for e in EV if e[1] == 'honk' and e[5] == 0 and e[0] <= T_KO}
    duck = {}                                                # v2: the beats where a swing honk bottoms out
    for e in EV:
        if e[1] == 'swing' and e[2] == KX and e[13] == 0 and e[0] <= T_KO:
            tb = q16((e[11] + e[12]) / 2)
            duck.setdefault(int(tb // T.BAR), set()).add(int(round((tb % T.BAR) / T.BEAT)) % 4)
    for b in range(2, bar_ko + 1):
        band_bar(mix, b, dense=min(1.0, (b - 2) / 22.0), honk_down=b in honk_bars, duck=duck.get(b, ()))
    for t_, who, name in RD.STORY['caps']:                   # a band hit under every captioned ability
        band_hit(mix, q16(t_), T.chord_at(t_), 0.55)
    for k in ('drums', 'guitar', 'organ'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ the BASSOON, from the run: a staccato dot per hop landing
    dropped = False
    last_slot = None
    last_played = -9.0
    mk_i = 0
    mk_hops = [e for e in EV if e[1] == 'hop' and e[2] == KB and e[8] == 2]
    for e in EV:
        t = e[0]
        if t > T_KO + 0.01:
            break
        if e[1] != 'stomp' or e[2] != KB:
            continue
        hit, back, kind = e[5], e[6], e[8]
        tq = q16(t)
        slot = round(tq / S16)
        if kind == 2:                                        # MOUNTAIN KING: Grieg's theme, one note per hop
            m = T.MK_THEME[min(mk_i, len(T.MK_THEME) - 1)]
            nxt = mk_hops[mk_i + 1][0] if mk_i + 1 < len(mk_hops) else t + T.BEAT / 2
            mk_i += 1
            dur = float(np.clip((nxt - t) * 0.7, 0.06, 0.2))
            sig, g, _ = colour(bsn(m, dur, 0.95), KB, t, 0.5)
            mix.place('bassoon', sig, t, 0.50 * g, pan=-0.2)
            mix.place('bassoon', colour(bsn(m - 12, dur, 0.7), KB, t, 0.5)[0], t, 0.18 * g, pan=-0.2)
            continue
        if slot == last_slot:
            continue
        last_slot = slot
        hp = hp_at(KB, t) / 100.0
        dropped = thin(slot, 3, hp, dropped) and tq - last_played < 0.5   # sparser when hurt (never two in a row,
        if dropped:                                          # and never when the line is sparse already)
            continue
        last_played = tq
        m = T.bassoon_at(tq)
        if slot % 2 == 1:                                    # STACCATO's 16th landings: a passing note
            nxt = T.bassoon_at(tq + S16)
            pas = [x for x in T.scale_notes(min(m, nxt), max(m, nxt), T.chord_at(tq)) if x not in (m, nxt)]
            m = pas[len(pas) // 2] if pas else nxt
        vel = 0.92 if back else (0.8 if hit else 0.6)
        sig, g, lead = colour(bsn(m, 0.085, vel), KB, t)
        mix.place('bassoon', sig, tq, (0.46 if hit else 0.30) * g, pan=-0.22)
        if lead > 0.05 and hit:                              # the leader: a sixth under (fuller)
            under = T.chord_tones(m - 9, m - 5, T.chord_at(tq))
            if under:
                mix.place('bassoon', colour(bsn(under[0], 0.085, 0.6), KB, t)[0], tq, 0.16 * g, pan=-0.3)
        if back:                                             # a backstab: a grace up the chord, the stab
            up_ = [x for x in T.chord_tones(m + 3, m + 9, T.chord_at(tq))]
            if up_:
                mix.place('bassoon', colour(bsn(up_[0], 0.06, 0.9), KB, t)[0], tq + 0.035, 0.26 * g, pan=-0.15)
            mix.place('fx', A.whoosh(0.07, 2000, 9000), tq - 0.03, 0.05)
    # ------------------------------------------------ the BARI SAX + its ropes, from the run
    x_dropped = False
    last_rattle = {}
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'honk' and e[2] == KX:                    # the honk: the bass root (SKANK's on the off-beats)
            tq = q16(t)
            ch = T.chord_at(tq + 0.01)
            hp = hp_at(KX, t) / 100.0
            if e[5] == 1:
                x_dropped = thin(round(tq / S16), 11, hp, x_dropped)
                if x_dropped:
                    continue
            m = T.HONK_NOTE[ch] + (12 if e[5] == 1 else 0)
            dur = 0.30 if e[5] == 0 else 0.14
            sig, g, lead = colour(bari(m, dur, 0.95, honk=0.8 if e[5] == 0 else 0.5), KX, t, 0.6)
            mix.place('bari', sig, tq, (0.95 if e[5] == 0 else 0.62) * g, pan=0.2)
            if lead > 0.05 and e[5] == 0:                    # the leader: the octave honked on top (fuller)
                mix.place('bari', colour(bari(m + 12, dur, 0.7, honk=0.4), KX, t)[0], tq, 0.18 * g, pan=0.25)
        elif kind == 'rattle' and e[2] == KX:                # the ropes it rattles: plucked bass strings
            tq = q16(t)
            ch = T.chord_at(tq + 0.01)
            sides = list(e[3:])
            gn = 1.0 / math.sqrt(len(sides))                 # more ropes = a fuller chord, not a louder one
            for sd in sides:
                if tq - last_rattle.get(sd, -9) < 0.05:
                    continue
                last_rattle[sd] = tq
                m = T.ROPE_NOTES[ch][sd]
                mix.place('ropes', rope_pluck(m, 1.3, 1.0, seed=sd), tq + 0.006 * sd, (0.34 - 0.03 * sd) * gn,
                          pan=(-0.4, 0.4, -0.15, 0.15)[sd])
        elif kind == 'sling':                                # slung off a ringing rope: a slap + a growled honk
            tq = q16(t)
            a = e[4]
            mix.place('ropes', rope_pluck(T.ROPE_NOTES[T.chord_at(tq)][0], 0.6, 1.0, seed=7), tq, 0.35 * a)
            mix.place('fx', A.hp(A.thud(0.25, 180, 70, seed=int(t * 10)), 150), tq, 0.2)
            sig, g, _ = colour(bari(T.HONK_NOTE[T.chord_at(tq)] + 7, 0.16, 1.0, honk=1.0), KX, t, 0.6)
            mix.place('bari', gliss(sig, -2.0, 0.05), tq + S16, 0.34 * g, pan=0.2)
        elif kind == 'cable' and e[2] == KX:                 # v2: the cable drops (the winch), the hook clanks home
            tq = q16(t)
            mix.place('fx', winch(e[3] - t, int(t * 10), rise=-0.8), tq, 0.10, pan=0.15)
            mix.place('fx', clank(79, int(t * 13)), q16(e[3]), 0.12, pan=0.15)
        elif kind == 'swing' and e[2] == KX:                 # v2: the swing honk: the bass on the swings
            ts0, ts1, sk = e[11], e[12], e[13]
            tb = q16((ts0 + ts1) / 2)                        # the bottom: on the beat (SKANK's: the off-beat)
            ch = T.chord_at(tb + 0.01)
            hp = hp_at(KX, t) / 100.0
            if sk:
                x_dropped = thin(round(tb / S16), 11, hp, x_dropped)
                if x_dropped:
                    continue
            m = T.HONK_NOTE[ch] + (12 if sk else 0)
            t_on = q16(ts0 - S16)                            # it honks as it's let go, swelling to the bottom
            dur = ts1 - t_on + 0.06
            sig, g, lead = colour(bari(m, dur, 0.95, honk=0.85 if not sk else 0.55), KX, t, 0.6)
            sig = doppler(sig, tb - t_on, up=1.6 if not sk else 1.0, down=2.6 if not sk else 1.6, width=0.09)
            mix.place('bari', sig, t_on, (1.15 if not sk else 0.80) * g, pan=0.2)
            if lead > 0.05 and not sk:                       # the leader: the octave honked on top (fuller)
                up8 = doppler(colour(bari(m + 12, dur, 0.7, honk=0.4), KX, t)[0], tb - t_on, 1.6, 2.6, 0.09)
                mix.place('bari', up8, t_on, 0.20 * g, pan=0.25)
            mix.place('fx', A.whoosh(ts1 - ts0, 250, 3500), ts0, 0.10 if not sk else 0.07, pan=0.1)
        elif kind == 'smash' and e[2] == KX:                 # v2: the wrecking ball connects
            tq = q16(t)
            sp = e[5]
            mix.place('fx', A.thud(0.45, 120, 38, seed=int(t * 10)), tq, 0.22 + 0.22 * sp)
            mix.place('fx', clank(67, int(t * 11)), tq, 0.10 + 0.08 * sp, pan=0.1)
            mix.place('fx', grit(0.25, int(t * 7)), tq, 0.12 + 0.12 * sp)
            if sp > 0.6:
                mix.place('drums', A.crash(1.0), tq, 0.06)
        elif kind == 'snap_aim' and e[2] == KX:              # v2: CABLE SNAP: the winch strains it up high
            mix.place('fx', winch(e[5] - t, 41, rise=1.3), q16(t), 0.16, pan=0.1)
        elif kind == 'cable_snap' and e[2] == KX:            # the cable parts: a twang, the whip, a falling whistle
            tq = q16(t)
            mix.place('fx', clank(98, 5) + clank(86, 6) * 0.6, tq, 0.22)
            mix.place('fx', A.whoosh(0.25, 3000, 9000), tq, 0.12)
            fall = [q for q in EV if q[1] == 'crater' and q[2] == KX and q[0] >= t]
            tf_ = (fall[0][0] - t) if fall else 0.4
            nf = int(tf_ * SR)
            tt_ = A.tt(nf)
            whistle = np.sin(2 * np.pi * np.cumsum(1800 * 2 ** (-1.6 * tt_ / tf_)) / SR) * (tt_ / tf_) ** 1.5 * 0.5
            mix.place('fx', whistle, tq, 0.10)
        elif kind == 'crater' and e[2] == KX:                # the crash
            tq = q16(t)
            mix.place('fx', thunder(2.0, 77), tq, 0.55)
            mix.place('fx', A.thud(0.7, 100, 32, seed=17), tq, 0.40)
            band_hit(mix, tq, T.chord_at(tq + 0.01), 0.9)
            sig, g, _ = colour(bari(T.HONK_NOTE[T.chord_at(tq + 0.01)], 0.7, 1.0, honk=1.0), KX, t, 0.5)
            mix.place('bari', bend(sig, -1.5, 0.3, 0.7), tq, 0.70 * g, pan=0.0)
            mix.place('bari', colour(bari(T.HONK_NOTE[T.chord_at(tq + 0.01)] + 12, 0.5, 0.7, honk=0.5), KX, t)[0], tq,
                      0.18 * g)
        elif kind == 'hit' and e[2] == KX and e[7] == 'rattle':   # the shove lands: a low thump
            mix.place('fx', A.hp(A.thud(0.3, 140, 45, seed=int(t * 10)), 150), q16(t), 0.12)
        elif kind == 'hook_open' and e[2] == KB:             # BOCAL HOOK: a comic blat sliding up as it swings out
            tq = q16(t)
            sig, g, _ = colour(bsn(46, 0.16, 1.0, growl=0.3), KB, t)
            mix.place('bassoon', gliss(sig, 4.0, 0.12), tq, 0.40 * g, pan=-0.2)
        elif kind == 'hook' and e[2] == KB:
            tq = q16(t)
            mix.place('fx', clank(91, int(t * 7)), tq, 0.16 if e[3] else 0.08, pan=0.1)
            if e[3]:
                mix.place('fx', A.whoosh(0.3, 800, 6000), tq, 0.10)
        elif kind == 'rope' and e[2] == KB:
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.05 * min(1.5, e[7] / 2.6))
        elif kind == 'clash':
            mix.place('fx', A.thud(0.15, 200, 90, seed=int(t * 3)), t, 0.06)
        elif kind == 'ability':
            who, name = e[2], e[3]
            tq = q16(t)
            if name == 'STACCATO':                           # a quick staccato run up
                for j, m in enumerate((64, 67, 71, 76)):
                    mix.place('bassoon', colour(bsn(m, 0.06, 0.9), KB, t)[0], tq + j * S16, 0.30, pan=-0.2)
            elif name == 'GROWL':                            # a gravelly growl + grit
                sig, g, _ = colour(bari(T.HONK_NOTE[T.chord_at(tq)], 0.6, 1.0, honk=1.0), KX, t, 0.8)
                mix.place('bari', bend(sig, -1.0, 0.1, 0.6), tq, 0.50 * g, pan=0.2)
                mix.place('fx', grit(0.6, int(t * 10)), tq, 0.30)
            elif name == 'SKANK':
                mix.place('drums', A.crash(1.2), tq, 0.08)
            elif name == 'LOW A':                            # the lowest note, held, and the whole ring rattles
                dur = 2 * T.BAR * 0.9
                sig, g, _ = colour(bari(36, dur, 1.0, honk=1.0), KX, t, 0.5)
                mix.place('bari', sig, tq, 0.55 * g, pan=0.0)
                mix.place('bari', colour(bari(48, dur, 0.7, honk=0.5), KX, t)[0], tq, 0.16 * g)
                mix.place('fx', thunder(2.6, 33, crack=False), tq, 0.45)
            elif name == 'MOUNTAIN KING':
                mix.place('drums', A.crash(1.6), tq, 0.08)
    # ------------------------------------------------ the K.O. moment
    band_hit(mix, T_KO, 'Em', 1.0)
    mix.place('fx', thunder(2.4, 51), T_KO, 0.55)
    loser = NAMES[LOSE]
    if loser == 'BASSOON':                                   # the bassoon falls apart: a falling blat, keys clattering
        x = bsn(52, 1.0, 1.0, growl=0.4)
        mix.place('bassoon', bend(x, -9.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x)), RD.T_HS, 0.42)
    else:
        x = bari(40, 1.0, 1.0, honk=0.8)
        mix.place('bari', bend(x, -7.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x)), RD.T_HS, 0.42)
    for j, m in enumerate((86, 81, 91, 77, 84)):
        mix.place('fx', clank(m, 20 + j), RD.T_HS + 0.08 * j, 0.07, pan=(j - 2) / 5)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.22)
    tw = q16(T_KO + 1.3)                                     # the winner's stinger on "(WINNER) WINS"
    band_hit(mix, tw, 'Em', 1.0)
    if NAMES[WIN] == 'BARI SAX':
        mix.place('solo', bari(40, 0.8, 1.0, honk=1.0), tw, 0.50)
        mix.place('solo', bari(52, 0.8, 0.8, honk=0.6), tw + 0.02, 0.22)
    else:
        for j, m in enumerate((64, 67, 71, 76)):
            mix.place('solo', bsn(m, 0.08, 1.0), tw + j * S16, 0.42)
    solo0 = RD.T_SOLO
    for b in range(bar_ko + 1, int(round(solo0 / T.BAR))):  # the band holds its breath, but never goes quiet:
        tq = b * T.BAR                                       # a skank on every off-beat + kick, until the solo
        for bt in range(4):
            mix.place('guitar', skank('Em', seed=b * 4 + bt), tq + bt * T.BEAT + T.BEAT / 2, 0.30, pan=-0.3)
            mix.place('organ', organ(ORG_V['Em'], 0.07), tq + bt * T.BEAT + T.BEAT / 2, 0.20, pan=0.3)
            mix.place('drums', HAT, tq + bt * T.BEAT, 0.13)
        mix.place('drums', KICK, tq, 0.42)
        mix.place('drums', KICK, tq + 2 * T.BEAT, 0.36)
        mix.place('ropes', rope_pluck(40, 1.3, seed=1), tq, 0.30)
    # ------------------------------------------------ victory: a lighter skank + the winner's solo
    b_solo = int(round(solo0 / T.BAR))
    for b in range(b_solo, 41):
        band_bar(mix, b, dense=0.5)
        ch = T.CHORDS[b]
        for bt in (0, 2):                                    # the ropes keep the bass going under the solo
            mix.place('ropes', rope_pluck(T.ROPE_NOTES[ch][0], 0.7, seed=bt), b * T.BAR + bt * T.BEAT, 0.34)
            mix.place('ropes', rope_pluck(T.ROPE_NOTES[ch][1], 0.5, seed=bt + 2), b * T.BAR + (bt + 1) * T.BEAT, 0.22)
    for bt, m, d in T.SOLO[NAMES[WIN]]:
        ts = solo0 + bt * T.BEAT
        if ts > T.T_REASSEMBLE - 0.05:
            break
        if NAMES[WIN] == 'BARI SAX':
            mix.place('solo', bari(m, d * T.BEAT * 0.9, 0.95, honk=0.7), ts, 0.55)
        else:
            mix.place('solo', bsn(m, max(0.07, d * T.BEAT * 0.6), 0.95), ts, 0.52)
    # the loser reassembles: its pieces fly home as a rising run, a glass "pop" when it's whole
    run = [64, 67, 71, 74, 76, 79]
    for j, m in enumerate(run):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
        if loser == 'BASSOON':
            mix.place('bassoon', bsn(m, 0.08, 0.7), tj, 0.24)
        else:
            mix.place('bari', bari(m - 24, 0.10, 0.7), tj, 0.24)
    mix.place('fx', A.chime(95, 0.5) + A.chime(102, 0.5) * 0.5, T.T_GLIDE, 0.12)
    # ------------------------------------------------ bars 41-42: the band returns, building into the title slam
    for b in (41, 42):
        band_bar(mix, b, dense=1.0)
        ch = T.CHORDS[b]
        for bt in range(4):
            mix.place('ropes', rope_pluck(T.ROPE_NOTES[ch][bt % 2], 0.5, seed=bt), b * T.BAR + bt * T.BEAT, 0.26)
    t42 = 42 * T.BAR
    st = 0.0
    while t42 + 2 * T.BEAT + st < T.DUR - 1e-4:              # a snare roll into the seam
        mix.place('drums', SNARE, t42 + 2 * T.BEAT + st, 0.12 + 0.12 * st / (2 * T.BEAT))
        st += S16
    mix.place('fx', riser(T.DUR - t42), t42, 0.20)
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
             (solo0, 57.5), (57.5, 61)]
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


BUS_GAIN = {'bassoon': 1.5, 'bari': 1.3, 'ropes': 1.0, 'solo': 1.5, 'fx': 1.2, 'drums': 1.0, 'guitar': 1.0, 'organ': 1.0}


def master(mix, thr=0.15, ceiling=0.89):
    """ONE master gain (no section automation: consistent loudness from frame 0) + a compressor and a soft limiter;
    the overflow past the loop (tails, reverb) folds onto the start; NA samples (whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('bassoon', 0.25), ('bari', 0.2), ('solo', 0.35), ('fx', 0.25),
                                                ('guitar', 0.3), ('organ', 0.3), ('drums', 0.12), ('ropes', 0.1)))
    wet = A.reverb(send, dur=1.6, decay=0.45) * 0.18
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
