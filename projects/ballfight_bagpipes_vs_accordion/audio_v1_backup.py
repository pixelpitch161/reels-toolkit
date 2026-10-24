"""v1 BACKUP (drone aura vs bellows rings, seed 1581; the roster has since moved to v2, so this no longer reproduces v1). #30 BALLFIGHT #4: Bagpipes vs Accordion - soundtrack (dark sea shanty), all synthesised, original.

The bed has no lead: the bagpipes and the accordion are the band's two leads, both played by the honest run:
  BAGPIPES   the drone (A2 + two A3 drones, in the Pixel Pitch Highland pipe's own timbre) runs under the whole video,
             kept low, and swells every time the aura lands a tick; each landed tick plays the chanter's shanty line at
             that 8th (16ths under DOUBLE DRONE), the first tick of a contact with the high-G grace squeal. WAR CRY is a
             skirl up to high A, DOUBLE DRONE adds the tenor fifth, BAG BURST deflates the bag in one wheezing slide.
             As its HP drops the chanter thins and dirties and the drones go sour.
  ACCORDION  every stroke is a chord in the Pixel Pitch accordion's reeds on the bar's chord: a push is the "oom" (a
             bass button + the chord, with the bellows' wheeze), a pull the "pah" (the chord swelling in, an inhale);
             landed strokes hit harder, POLKA turns it into an every-beat oom-pah, SQUEEZEBOX folds shut with a squeal,
             FULL BELLOWS inhales for two beats and blasts every reed at once. As its HP drops the chords lose notes and
             wheeze.
Bed: A dorian at 121.97 BPM (31 bars = 61 s): foot stomps on 1 and 3, gang claps on 2 and 4, a low male-choir hum
pad, a bass voice on the stomps, a frame drum joining for the back half. The count-in bar (stomp + clap + a chanter
blast + an accordion chord per count, on the dominant E) drops on FIGHT!; at the K.O. the bed tape-stops with the
hit-stop, the loser's instrument dies (the bag deflates / the bellows tear), the crowd roars, the winner's stinger
lands, then its solo over a lighter bed; bar 30 builds (stomp roll, riser, choir climb) into the "3" on sample 0.
The drone loops sample-perfectly (whole cycles in the file). Every hit is snapped to the nearest 16th.

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
import render_v1_backup as RD
import pp_bridge as B

SR = A.SR
RUN, EV = RD.RUN, RD.EV
NAMES, WIN, LOSE, T_KO = RD.NAMES, RD.WIN, RD.LOSE, RD.T_KO
KP, KA = NAMES.index('BAGPIPES'), NAMES.index('ACCORDION')
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames (2860 = 61.013 s): loops sample-perfectly
S16 = T.S16
BAG = B.module('bagpipes60_lowpoly')
ACC = B.module('accordion60_lowpoly')
HL = BAG.TONES['ghb']                                    # the Great Highland pipe's chanter tone
GOLDEN = ACC.TONES['golden']                             # three reeds + the low octave: the full accordion


def q16(t):
    return round(t / S16) * S16


def h01(*xs):
    """deterministic hash in [0, 1) (thinning decisions are part of the score, not random each run)"""
    v = 0
    for x in xs:
        v = (v * 1000003 + int(x * 1000) + 7) % 2147483647
    return ((v * 48271) % 2147483647) / 2147483647.0


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


def _fit(x, cap):
    n = int(cap * SR)
    x = np.asarray(x, float)[:n].copy()
    if len(x) < n:
        x = np.pad(x, (0, n - len(x)))
    fd = min(len(x) // 3, int(0.03 * SR))
    x[-fd:] *= np.linspace(1, 0, fd)
    return x


def _norm(x, target=0.16):
    return x * min(6.0, target / (np.sqrt((A.hp(x, 120) ** 2).mean()) + 1e-6))


# ---------------------------------------------------------------- the two leads in their Pixel Pitch voices (cached)
_VC = {}
_G = {}


def chanter(m, dur=0.2, vel=0.85, grace=False):
    """one chanter note (the Highland pipe's own synth), optionally with the high-G grace squeal in front"""
    key = ('ch', int(m), round(dur, 3), round(vel, 2), grace)
    if key not in _VC:
        x = BAG.chanter(BAG.midi_hz(m), dur, vel, HL, BAG.midi_hz(79) if grace else None, seed=int(m) * 7)
        x = _fit(B._resample(np.asarray(x, float)), dur + 0.12)
        if 'ch' not in _G:
            ref = B._resample(np.asarray(BAG.chanter(BAG.midi_hz(72), 0.2, 0.85, HL, None, 1), float))
            _G['ch'] = min(6.0, 0.16 / (np.sqrt((A.hp(ref, 120) ** 2).mean()) + 1e-6))
        _VC[key] = x * _G['ch']
    return _VC[key]


def reeds(ms, dur=0.2, vel=0.85, swell=False, wheeze=0.0, seed=0):
    """an accordion chord in the Pixel Pitch accordion's reeds ('golden': three reeds + the low octave); swell = the
    pull's inhale (the chord fades in); wheeze = bellows air noise on top"""
    key = ('acc', tuple(int(m) for m in ms), round(dur, 3), round(vel, 2), swell, round(wheeze, 2), seed)
    if key not in _VC:
        x = 0
        for m in ms:
            y = ACC.reed(ACC.midi_hz(m), dur, vel, seed=int(m) * 3 + seed, **GOLDEN)
            y = B._resample(np.asarray(y, float))
            x = x + np.pad(y, (0, max(0, int((dur + 0.3) * SR) - len(y))))[:int((dur + 0.3) * SR)]
        x = x / math.sqrt(len(ms))
        if 'acc' not in _G:
            ref = B._resample(np.asarray(ACC.reed(ACC.midi_hz(64), 0.2, 0.85, seed=1, **GOLDEN), float))
            _G['acc'] = min(6.0, 0.16 / (np.sqrt((A.hp(ref, 120) ** 2).mean()) + 1e-6))
        x = x * _G['acc']
        n = len(x)
        t = A.tt(n)
        if swell:
            x = x * np.clip(t / max(0.05, dur * 0.85), 0, 1) ** 1.6
        if wheeze > 0:
            air = A.bp(np.random.default_rng(seed + 17).normal(0, 1, n), 900, 5200)
            env = np.clip(t / max(0.04, dur * 0.8), 0, 1) ** 1.4 if swell else np.exp(-t / 0.07)
            x = x + wheeze * 0.35 * air * env * (t < dur + 0.08)
        _VC[key] = _fit(x, dur + 0.25)
    return _VC[key]


# ---------------------------------------------------------------- the drone: whole cycles in the file (seamless loop)
def drone_layer(ms_amps, n=NA):
    """the Highland drones as wavetables at frequencies nudged (< 0.01 Hz) to whole cycles in n samples, so the drone
    at the file's last sample flows into its first; ms_amps: [(midi, amp, cents)]"""
    dc = BAG.DRONES['ghb']
    L = n / SR
    x = np.zeros(n)
    t = np.arange(n) / SR
    for j, (m, a, cents) in enumerate(ms_amps):
        f = BAG.midi_hz(m) * 2 ** (cents / 1200)
        fq = round(f * L) / L
        tb = BAG.wt(f, dc['tilt'], dc['form'], dc['fbw'], dc['fgain'], dc['odd'])
        ph = (fq * t + 0.137 * (j + 1)) % 1.0
        x += a * tb[(ph * 2048).astype(int) & 2047]
    return A.lp(x, dc['lpf']) * 0.3


# ---------------------------------------------------------------- bed synths (original)
def stomp(vel=1.0, seed=0):
    """a boot on a wooden deck: a low thump + the board's knock"""
    n = int(0.35 * SR)
    t = A.tt(n)
    f = 52 + 70 * np.exp(-t / 0.03)
    body = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.13)
    knock = A.bp(np.random.default_rng(seed).normal(0, 1, n), 180, 1400) * np.exp(-t / 0.022)
    return (np.tanh(1.8 * body) * 0.9 + 0.55 * knock) * vel


def gang_clap(vel=1.0, seed=0):
    """a few hands clapping a hair apart"""
    out = np.zeros(int(0.32 * SR))
    for j, d in enumerate((0.0, 0.006, 0.013, 0.019)):
        c = A.clap()
        i = int(d * SR)
        out[i:i + len(c)] += c[:len(out) - i] * (1.0 - 0.15 * j)
    return A.bp(out, 600, 7000) * vel * 0.6


def frame_drum(low=True, vel=1.0):
    """a bodhran-ish frame drum: pitched skin thump (low) or a rim tap"""
    n = int(0.3 * SR)
    t = A.tt(n)
    if low:
        f = 82 + 40 * np.exp(-t / 0.02)
        x = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.09)
        x += 0.25 * A.bp(np.random.default_rng(5).normal(0, 1, n), 300, 2000) * np.exp(-t / 0.01)
    else:
        x = A.bp(np.random.default_rng(6).normal(0, 1, n), 1200, 5000) * np.exp(-t / 0.018) * 0.7
        x += 0.3 * np.sin(2 * np.pi * 410 * t) * np.exp(-t / 0.03)
    return x * vel


def choir(ms, dur, att=0.35, rel=0.4, vowel='m'):
    """a low male choir: three detuned saws per note with a slow vibrato through 'mm' / 'oh' formants"""
    n = int((dur + rel) * SR)
    t = A.tt(n)
    x = np.zeros(n)
    for m in ms:
        f = A.mtof(m)
        for d, ph0 in ((-0.004, 0.1), (0.0, 0.5), (0.005, 0.8)):
            fr = f * (1 + d) * (1 + 0.004 * np.sin(2 * np.pi * (4.6 + 3 * d * 100) * t + ph0 * 6))
            ph = (np.cumsum(fr) / SR + ph0) % 1.0
            x += 2 * ph - 1
    x /= 3 * max(1, len(ms))
    if vowel == 'm':
        y = A.bp(x, 180, 420) * 1.0 + A.bp(x, 700, 1100) * 0.25
    else:
        y = A.bp(x, 300, 650) * 1.0 + A.bp(x, 750, 1250) * 0.45
    env = np.clip(t / att, 0, 1) * np.clip((dur + rel - t) / rel, 0, 1)
    return A.lp(y, 1600) * env


def bass_voice(m, dur=0.35, vel=1.0):
    """the bass singer's 'dum' on the stomps"""
    n = int((dur + 0.1) * SR)
    t = A.tt(n)
    f = A.mtof(m)
    ph = (f * t) % 1.0
    x = (2 * ph - 1) * 0.6 + np.sin(2 * np.pi * f * t)
    y = A.bp(x, 60, 380) + 0.3 * A.bp(x, 500, 900)
    env = np.clip(t / 0.02, 0, 1) * np.exp(-np.maximum(0, t - 0.05) / max(0.12, dur * 0.6))
    return np.tanh(1.4 * y * env) * vel


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


def deflate(dur=1.6, f0=440.0, f1=150.0, seed=21):
    """the bag giving out: the chanter's reed sliding down and choking, with the air hissing out"""
    n = int(dur * SR)
    t = A.tt(n)
    u = t / dur
    f = f0 * (f1 / f0) ** (u ** 0.7) * (1 + 0.03 * np.sin(2 * np.pi * 7 * t))
    tb = BAG.wt(f0, HL['tilt'], HL['form'], HL['fbw'], HL['fgain'], HL['odd'])
    ph = np.cumsum(f) / SR
    x = tb[(ph * 2048).astype(int) & 2047] * (1 - u) ** 1.2
    air = A.bp(np.random.default_rng(seed).normal(0, 1, n), 1500, 7000) * np.exp(-u * 2.5) * 0.5
    return A.lp(x, 4500) + air


def tear(dur=1.2, seed=23):
    """the bellows tearing: a ripping burst of air and every reed sagging out of tune"""
    n = int(dur * SR)
    t = A.tt(n)
    rip = A.bp(np.random.default_rng(seed).normal(0, 1, n), 700, 6000) * np.exp(-t / 0.18)
    x = 0
    for m in (57, 60, 64, 69):
        f = A.mtof(m) * (1 - 0.18 * (t / dur) ** 0.8)
        ph = (np.cumsum(f) / SR) % 1.0
        x = x + (np.where(ph < 0.3, 1.0, -0.43) + 0.4 * (2 * ph - 1)) * 0.25
    return A.lp(x, 2600) * np.exp(-t / (dur * 0.4)) + rip


# ---------------------------------------------------------------- harmony
BASS = {'Am': (45, 40), 'G': (43, 38), 'Em': (40, 47), 'E': (40, 47)}          # (root on 1, fifth on 3)
OOM = {'Am': 45, 'G': 43, 'Em': 40, 'E': 40}                                    # the accordion's bass button
VOICING = {'Am': (57, 60, 64, 69), 'G': (55, 59, 62, 67), 'Em': (55, 59, 64, 67), 'E': (56, 59, 62, 64)}
CHOIR = {'Am': (33, 45, 52, 57, 60), 'G': (31, 43, 50, 55, 59), 'Em': (28, 40, 47, 52, 55), 'E': (28, 40, 47, 52, 56)}


def acc_chord(t, hp, lead, high=False):
    """the accordion's chord at t, thinned as its HP drops (fewer reeds), with a top note added when it leads"""
    ch = T.chord_at(t)
    vo = list(VOICING[ch])
    if high:
        vo = [m + 12 for m in vo[:3]]
    keep = 4 if hp > 0.5 else (3 if hp > 0.25 else 2)
    vo = vo[:keep]
    if lead > 0.04:
        vo = vo + [vo[0] + 12]
    return tuple(vo)


def bed(mix, b, intensity=1.0, light=False):
    """one bar of the shanty bed: stomps on 1 and 3, gang claps on 2 and 4, the choir hum, the bass singer, the frame
    drum in the back half"""
    t0 = b * T.BAR
    ch = T.CHORDS[b]
    r, f5 = BASS[ch]
    for bt in (0, 2):
        mix.place('stomp', stomp(1.0 if bt == 0 else 0.85, b * 4 + bt), t0 + bt * T.BEAT, 0.50 if not light else 0.36)
        mix.place('choir', bass_voice(r if bt == 0 else f5, 0.34, 1.0), t0 + bt * T.BEAT, 0.22 if not light else 0.16)
    for bt in (1, 3):
        mix.place('stomp', gang_clap(0.9 if bt == 1 else 1.0, b * 4 + bt), t0 + bt * T.BEAT, 0.34 if not light else 0.24,
                  pan=0.15 * (1 if bt == 1 else -1))
    if intensity > 1.15 and not light:                               # a stomp pickup on the "and" of 4
        mix.place('stomp', stomp(0.6, b * 4 + 7), t0 + 3.5 * T.BEAT, 0.26)
    mix.place('choir', choir(CHOIR[ch], T.BAR, att=0.3, rel=0.35, vowel='m' if intensity < 1.3 else 'o'), t0,
              0.10 if not light else 0.08)
    if intensity > 1.25 and not light:                               # frame drum: driving 8ths in the back half
        k = min(1.0, (intensity - 1.25) / 0.25)
        for st in range(0, 16, 2):
            low = st % 4 == 0
            mix.place('drum', frame_drum(low, 0.8 + 0.2 * (st % 8 == 0)), t0 + st * S16, (0.20 if low else 0.10) * k)


# ---------------------------------------------------------------- the score
def main():
    mix = A.Mix(T.DUR + 4.0)
    tw = q16(T_KO + 1.3)
    solo0 = RD.T_SOLO
    # ------------------------------------------------ count-in (bar 0, on the dominant E): stomp + clap + chanter + chord
    for j, (tc, txt) in enumerate(T.COUNT[:3]):
        tc = max(tc, 0.004)                                # the "3" 4 ms into the file: no click at the seam
        mix.place('stomp', stomp(1.0, 90 + j), tc, 0.52)
        mix.place('stomp', gang_clap(1.0, 95 + j), tc, 0.30)
        mix.place('chanter', chanter((71, 74, 76)[j], 0.30, 0.9, grace=True), tc, 0.40)
        mix.place('acc', reeds(VOICING['E'], 0.22, 0.9, wheeze=0.4, seed=j), tc, 0.38, pan=0.25)
        mix.place('choir', bass_voice(40, 0.36), tc, 0.24)
    mix.place('choir', choir(CHOIR['E'], T.BAR, att=0.2, rel=0.05, vowel='o'), 0.0, 0.10)
    for st, v in ((12, 0.5), (13, 0.65), (14, 0.8), (15, 0.95)):    # a stomp run into FIGHT!
        mix.place('stomp', stomp(v, 80 + st), st * S16, 0.40)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), 3 * T.BEAT, 0.14)
    mix.place('fx', A.thud(0.8, 110, 34), T.BAR, 0.55)
    mix.place('choir', choir([m + 12 for m in CHOIR['Am']], 0.5, att=0.01, rel=0.25, vowel='o'), T.BAR, 0.22)
    # ------------------------------------------------ the bed: bars 1 .. the K.O.
    bar_ko = int(T_KO // T.BAR)
    for b in range(1, bar_ko + 1):
        bed(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
    mix.place('fx', A.whoosh(3.6, 300, 6000), 44.0 - 3.6, 0.10)
    for k in ('stomp', 'choir', 'drum'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ BAGPIPES: the drone + the chanter on every landed tick
    n = NA
    tt_ = np.arange(n) / SR
    drone = drone_layer(((57, 1.0, -1.5), (57, 1.0, 1.5), (45, 1.1, 0.0)), n)
    tenor = drone_layer(((64, 0.8, 0.0),), n)                       # DOUBLE DRONE's fifth
    hits_p = [e for e in EV if e[1] == 'hit' and e[2] == KP and e[7] == 'aura' and e[0] <= T_KO + 0.01]
    imp = np.zeros(n)
    for e in hits_p:
        imp[min(n - 1, int(q16(e[0]) * SR))] += 1.0
    from scipy.signal import lfilter
    a_rel = math.exp(-1 / (0.40 * SR))
    swell = lfilter([1 - a_rel], [1, -a_rel], imp) * (0.40 * SR) / 6.0      # ~ number of ticks in the last 0.4 s / 6
    swell = np.clip(swell, 0, 1)
    swell = lfilter([0.02], [1, -0.98], swell)                      # a ~1 ms attack (no clicks)
    alive = np.ones(n)                                              # the drone is the bagpipes' life: it stops at a
    if LOSE == KP:                                                  # bagpipes K.O. and comes back with the reassembly
        alive = np.where(tt_ < T_KO, 1.0, 0.0) + np.clip((tt_ - T.T_GLIDE) / 0.6, 0, 1) * (tt_ >= T.T_GLIDE)
        alive = lfilter([0.002], [1, -0.998], alive)
    hp_p = np.array([hp_at(KP, x) for x in np.linspace(0, T.DUR, 400)]) / 100.0
    hp_curve = np.interp(tt_, np.linspace(0, T.DUR, 400), hp_p)
    champ = (tt_ >= T_KO) * np.clip((T.T_REV - tt_) / (T.T_REV - T.T_GLIDE), 0, 1) * (LOSE != KP)
    base = 0.11 + 0.10 * champ                                      # under the mix; a little louder for the champion's
                                                                    # lap, back to the opening level before the seam
    g = (base + 0.85 * swell) * alive
    sour = np.sin(2 * np.pi * 0.37 * tt_) * 0.5 + 0.5               # the drones go sour as the bag weakens
    drone_sig = drone * g * (1 - 0.35 * (1 - hp_curve) * sour)
    haste_p = np.interp(tt_, RD.T.T_FIGHT + np.arange(len(RUN['HA'])) / T.FPS, (RUN['HA'][:, KP] > 0).astype(float),
                        left=0, right=0)
    haste_p = lfilter([0.0005], [1, -0.9995], haste_p)
    drone_sig = drone_sig + tenor * 0.8 * haste_p * g
    mix.bus('drone')[:n] += np.stack([drone_sig * 0.95, drone_sig * 1.0], 1)
    step_ticks = {}
    for e in hits_p:
        tq = q16(e[0])
        first = bool(e[8])
        hasted = float(RUN['HA'][RD.sim_i(e[0]), KP]) > 0
        step = S16 if hasted else 2 * S16
        m = T.melody_at(tq)
        if hasted and round(tq / S16) % 2:                           # the 16ths in between: a passing note up
            ms = T.scale_notes(m + 1, m + 3)
            m = ms[0] if ms else m
        sig, gg, lead = colour(chanter(m, step * 0.9, 0.9, grace=first), KP, tq, 1.0)
        pan = float(np.clip(e[5] / 3.5, -0.5, 0.5))
        mix.place('chanter', sig, tq, 0.30 * gg * (1.25 if first else 1.0), pan=pan)
        if lead > 0.06 and not hasted:                              # the leader: the line doubled an octave down
            mix.place('chanter', colour(chanter(m - 12, step * 0.9, 0.7), KP, tq)[0], tq, 0.10 * gg, pan=pan)
        step_ticks[round(tq, 3)] = True
    # ------------------------------------------------ ACCORDION: every stroke is a chord (push = oom, pull = pah)
    for e in EV:
        if e[1] != 'bellows' or e[2] != KA or e[0] > T_KO + 0.01:
            continue
        t0 = q16(e[0])
        main_, push = bool(e[3]), bool(e[4])
        hp = hp_at(KA, t0) / 100.0
        lead = (hp_at(KA, t0) - hp_at(KP, t0)) / 100.0
        wz = 0.25 + 0.9 * (1 - hp) ** 1.3
        if push:
            vo = acc_chord(t0, hp, lead)
            sig, gg, _ = colour(reeds(vo, 0.20 if main_ else 0.13, 0.9, wheeze=wz, seed=int(t0 * 10)), KA, t0, 0.8)
            mix.place('acc', sig, t0, (0.34 if main_ else 0.24) * gg, pan=0.2)
            mix.place('acc', colour(reeds((OOM[T.chord_at(t0)] - 12, OOM[T.chord_at(t0)]), 0.24, 0.9, seed=3), KA, t0)[0],
                      t0, 0.30 * gg, pan=0.1)
        else:
            vo = acc_chord(t0, hp, lead, high=True)
            d = 0.34 if main_ else 0.20
            sig, gg, _ = colour(reeds(vo, d, 0.85, swell=True, wheeze=wz, seed=int(t0 * 10) + 1), KA, t0, 0.8)
            mix.place('acc', sig, t0, (0.28 if main_ else 0.20) * gg, pan=0.25)
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'hit' and e[2] == KA and e[7] in ('bellows', 'suction'):
            tq = q16(t)
            hp = hp_at(KA, tq) / 100.0
            lead = (hp_at(KA, tq) - hp_at(KP, tq)) / 100.0
            pan = float(np.clip(e[5] / 3.5, -0.6, 0.6))
            if e[7] == 'bellows':                                  # a landed push: the full chord slammed + the hit
                sig, gg, _ = colour(reeds(acc_chord(tq, hp, lead), 0.16, 1.0, wheeze=0.5, seed=7), KA, t)
                mix.place('acc', sig, tq, 0.30 * gg, pan=pan)
                mix.place('fx', A.thud(0.32, 140, 50, seed=int(t * 7)), tq, 0.16)
            else:                                                  # a landed suction: a gulp, the chord pulled down
                vo = [m - 12 for m in acc_chord(tq, hp, lead)[:3]]
                sig, gg, _ = colour(reeds(vo, 0.14, 1.0, wheeze=0.6, seed=8), KA, t)
                mix.place('acc', sig, tq, 0.30 * gg, pan=pan)
                mix.place('fx', A.whoosh(0.18, 3000, 400, rise=False), tq - 0.12, 0.08)
                mix.place('fx', A.thud(0.28, 120, 45, seed=int(t * 9)), tq, 0.12)
        elif kind == 'block':                                      # the drone hits the closed squeezebox: a buzz
            mix.place('fx', A.bp(chanter(T.melody_at(q16(t)) + 12, 0.06, 0.6), 1500, 6000), q16(t), 0.08)
        elif kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.06 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            mix.place('fx', A.chime(91, 0.25), t, 0.05)
        elif kind == 'hit' and e[7] in ('shockwave', 'wave'):
            mix.place('fx', A.thud(0.45, 110, 40), q16(t), 0.30)
        elif kind == 'ability':
            who, name = e[2], e[3]
            tq = q16(t)
            if name == 'WAR CRY':                                  # the skirl: a run up to high A, held, the drone roars
                for j, m in enumerate((69, 71, 72, 74, 76, 79)):
                    mix.place('chanter', colour(chanter(m, S16 * 0.9, 1.0, grace=(j == 0)), KP, t)[0], tq + j * S16 / 2, 0.26)
                mix.place('chanter', colour(chanter(81, 2 * T.BEAT, 1.0, grace=True), KP, t)[0], tq + 3 * S16, 0.34)
            elif name == 'DOUBLE DRONE':
                mix.place('fx', A.whoosh(0.6, 200, 3000), tq - 0.3, 0.08)
            elif name == 'BAG BURST':                              # the bag deflates in one blast
                mix.place('chanter', deflate(1.8, BAG.midi_hz(81), 160.0), tq, 0.34)
                mix.place('fx', A.thud(0.7, 100, 32), tq, 0.45)
                mix.place('fx', A.bp(np.random.default_rng(31).normal(0, 1, int(0.9 * SR)), 1200, 8000) *
                          np.exp(-A.tt(int(0.9 * SR)) / 0.25), tq, 0.12)
            elif name == 'POLKA':                                  # a little "hup" from the choir as the oom-pah starts
                mix.place('choir', choir((57, 60, 64), 0.18, att=0.01, rel=0.12, vowel='o'), tq, 0.16)
            elif name == 'SQUEEZEBOX':                             # folds shut: a cluster squeal cut off, a clunk
                sq = reeds((69, 70, 71, 76), 0.26, 0.9, wheeze=0.9, seed=41)
                mix.place('acc', sq, tq, 0.22, pan=0.2)
                mix.place('fx', A.thud(0.25, 180, 80), tq + 0.26, 0.20)
            elif name == 'FULL BELLOWS':                           # a two-beat inhale (the slam's telegraph)
                d = 2 * T.BEAT
                mix.place('acc', reeds((45, 57, 60, 64), d, 0.9, swell=True, wheeze=1.2, seed=43), tq, 0.30, pan=0.1)
                mix.place('fx', A.whoosh(d, 3000, 300, rise=False), tq, 0.10)
        elif kind == 'slam':                                       # the giant bellows squeezed shut: every reed at once
            ts = e[0]
            landed = bool(e[5])
            vo = (33, 45, 52, 57, 60, 64, 69, 72, 76)
            mix.place('acc', reeds(vo, 0.55, 1.0, wheeze=1.4, seed=45), ts, 0.42 if landed else 0.30)
            mix.place('fx', A.thud(0.7, 90, 30), ts, 0.55 if landed else 0.32)
            mix.place('stomp', stomp(1.0, 51), ts, 0.5)
    # ------------------------------------------------ the K.O. moment
    mix.place('fx', A.thud(0.9, 120, 30), T_KO, 0.7)
    mix.place('stomp', stomp(1.0, 61), T_KO, 0.6)
    if NAMES[LOSE] == 'BAGPIPES':
        mix.place('chanter', deflate(2.2, BAG.midi_hz(76), 120.0, 27), RD.T_HS, 0.36)
    else:
        mix.place('acc', tear(1.4), RD.T_HS, 0.40)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.16)
    # the winner's stinger on "(WINNER) WINS"
    if NAMES[WIN] == 'BAGPIPES':
        mix.place('solo', chanter(81, 1.2, 1.0, grace=True), tw, 0.36)
        mix.place('solo', chanter(69, 1.2, 0.8), tw, 0.18)
    else:
        mix.place('solo', reeds((57, 64, 69, 72, 76), 1.0, 1.0, wheeze=0.4, seed=51), tw, 0.36)
    mix.place('choir', choir(CHOIR['Am'], 1.4, att=0.05, rel=0.6, vowel='o'), tw, 0.18)
    mix.place('fx', A.whoosh(solo0 - tw - 0.1, 300, 5000), tw + 0.1, 0.10)
    # ------------------------------------------------ victory: lighter bed + the winner's solo
    b_solo = int(round(solo0 / T.BAR))
    for b in range(b_solo, 30):
        bed(mix, b, light=True)
    for bt, m, d in T.SOLO[NAMES[WIN]]:
        ts = solo0 + bt * T.BEAT
        if ts >= T.T_REASSEMBLE:
            break
        dur = d * T.BEAT * 0.95
        if NAMES[WIN] == 'BAGPIPES':
            mix.place('solo', chanter(m, dur, 0.95, grace=(d >= 0.75)), ts, 0.46)
        else:
            mix.place('solo', reeds((m, m - 12), dur, 0.95, wheeze=0.2, seed=int(bt * 4)), ts, 0.44)
    # the loser reassembles: its pieces fly home as a rising run, a glass "pop" when it's whole
    run = [57, 60, 62, 64, 67, 69, 72, 76]
    for j, m in enumerate(run):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
        if NAMES[LOSE] == 'BAGPIPES':
            mix.place('chanter', chanter(m + 12 if m < 67 else m, S16 * 1.6, 0.6), tj, 0.16)
        else:
            mix.place('acc', reeds((m,), S16 * 1.6, 0.6, seed=j), tj, 0.20)
    mix.place('fx', A.chime(95, 0.5) + A.chime(102, 0.5) * 0.5, T.T_GLIDE, 0.12)
    # ------------------------------------------------ bar 30: build into the "3" on sample 0
    t30 = 30 * T.BAR
    for st in range(16):                                           # the stomps double up: 8ths, then 16ths
        if st < 8 and st % 2:
            continue
        mix.place('stomp', stomp(0.55 + 0.45 * st / 15, 70 + st), t30 + st * S16, 0.42)
    for bt in (1, 3):
        mix.place('stomp', gang_clap(1.0, 75 + bt), t30 + bt * T.BEAT, 0.30)
    mix.place('fx', riser(T.DUR - t30), t30, 0.22)
    mix.place('choir', choir(CHOIR['E'], T.DUR - t30 + 0.02, att=0.9, rel=0.001, vowel='o'), t30, 0.13)
    for bt in range(4):
        mix.place('choir', bass_voice((40, 44, 47, 52)[bt], 0.32), t30 + bt * T.BEAT, 0.22)
        mix.place('acc', reeds(VOICING['E'], 0.16, 0.85, wheeze=0.3, seed=60 + bt), t30 + bt * T.BEAT + T.BEAT / 2, 0.22,
                  pan=0.25)
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


BUS_GAIN = {'chanter': 1.0, 'acc': 1.0, 'drone': 1.0, 'solo': 1.0, 'fx': 1.5, 'stomp': 1.1, 'choir': 2.2, 'drum': 1.3}
GAIN_PTS = [(0, 0.0), (1.55, 0.0), (2.1, -1.8), (20, -1.7), (30, -0.6), (40, 1.6), (44, 2.6), (50.4, 3.5), (52.0, 2.8),
            (53.2, 3.3), (58.0, 3.3), (60.0, 1.6), (61.02, 0.6)]


def master(mix, thr=0.22, ceiling=0.89):
    """the overflow past the loop (tails, reverb) folded onto the start; NA samples (whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('chanter', 0.45), ('solo', 0.5), ('fx', 0.4), ('acc', 0.35),
                                                ('choir', 0.6), ('stomp', 0.25), ('drone', 0.2)))
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
    gain = (gain * 10 ** (g / 20))[:, None]
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
