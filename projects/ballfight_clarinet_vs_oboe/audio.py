"""#37 BALLFIGHT #11: Clarinet vs Oboe - soundtrack (mood A: Baroque storm), all synthesised, original.

The bed is a Baroque storm with NO woodwinds of its own: harpsichord continuo (repeated 8th-note bass, chords, 16th
arpeggios later), a racing minor string ostinato (violins in 16ths, cellos and basses driving 8ths, violas trembling in
the back half), timpani and thunder. D minor at 141.64 BPM (36 bars = 61 s). Both fighters are woodwinds, played from
the honest run in their own voices (the Pixel Pitch clarinet's reed synth, the new oboe double reed):
  CLARINET  the lower, liquid line: every jump plays the chalumeau line (timeline.CLARINET_LINE, D3-E4; CLARION's 8th
            jumps add the passing notes), a landed cut adds the TWELFTH above as a grace (the register key); SQUEAK is
            the infamous squeal; RHAPSODY five rising notes gliding up through the registers.
  OBOE      the higher, nasal line: every bar its tuning A (A4) sounds with the beam; while the enemy is OUT OF TUNE each
            sting is one of its running 8ths (timeline.OBOE_8THS); ENDLESS NOTE holds the A for a bar; DOUBLE REED is
            a reed crow + a scissor snip; ILL WIND a sour gust and a bent, beating A.
OUT OF TUNE is a game rule and the audio follows it: while the clarinet is detuned its notes play flat by up to a
quarter-tone (depth x 0.5 semitone) with a beating second reed: sour, not broken. As HP drops a fighter's part thins and
dirties; the leader's part is fuller and louder.
Consistent loudness from frame 0 (project rule, Oct 26): the storm is at full level on the title card; it builds with
parts and density (violin II, violas, 16th arpeggios, timpani on 3), never with level. One master gain + a compressor
and a soft limiter: no section automation. Bar 0 is the VS title card (a clarinet glissando blast, the thunder crack,
the oboe's held A, each over a tutti hit), bar 1 the count-in on the dominant, the drop on FIGHT!. At the K.O. the bed
tape-stops into a held tutti, the loser falls apart, the winner's stinger and solo play over a lighter storm; the last
bars build into the title slam on sample 0. Every hit is snapped to the nearest 16th; tails past the loop fold onto
the start.

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
KC, KO_ = NAMES.index('CLARINET'), NAMES.index('OBOE')
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024          # whole AAC frames: loops sample-perfectly
S16 = T.S16
CLM = B.module('clarinet60_lowpoly')                    # the Pixel Pitch clarinet's reed synth (read-only)
OBM = B.module('oboe_lowpoly')                          # the new oboe's double reed


def q16(t):
    return round(t / S16) * S16


def h01(*xs):
    v = 0
    for x in xs:
        v = (v * 1000003 + int(x * 1000) + 7) % 2147483647
    return ((v * 48271) % 2147483647) / 2147483647.0


# ---------------------------------------------------------------- the two fighters' voices (cached, level-matched)
_VC = {}


def _fit(x, cap):
    n = int(cap * SR)
    x = np.asarray(x, float)[:n].copy()
    if len(x) < n:
        x = np.pad(x, (0, n - len(x)))
    fd = min(len(x) // 3, int(0.03 * SR))
    x[-fd:] *= np.linspace(1, 0, fd)
    return x


def _norm(x):
    r = np.sqrt((x[:int(0.25 * SR)] ** 2).mean()) + 1e-9
    return x * (0.16 / r)


def clar(m, dur, vel=0.85, tone='boehm'):
    """one clarinet note (fractional midi = detuned), dur s, from the series' reed synth"""
    key = ('c', round(m, 3), round(dur, 3), round(vel, 2), tone)
    if key not in _VC:
        x = CLM.reed(CLM.midi_hz(m), dur, vel, **CLM.TONES[tone])
        _VC[key] = _norm(_fit(B._resample(np.asarray(x, float)), dur + 0.14))
    return _VC[key]


def oboe(m, dur, vel=0.85):
    key = ('o', round(m, 3), round(dur, 3), round(vel, 2))
    if key not in _VC:
        x = OBM.double_reed(OBM._midi_hz(m), dur, vel)
        _VC[key] = _norm(_fit(B._resample(np.asarray(x, float)), dur + 0.12)) * 0.85
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
    """bend the pitch by `semis` between t0 and t1 s into the note (ILL WIND's sag, the K.O. fall)"""
    n = len(x)
    t = A.tt(n)
    off = semis * np.clip((t - t0) / max(1e-3, t1 - t0), 0, 1)
    rate = 2 ** (off / 12)
    pos = np.cumsum(rate) - rate[0]
    return np.interp(pos, np.arange(n), x, right=0.0)


def hp_at(k, t):
    return RD.hp_at(k, t)


def detune(k, t):
    """the detune depth on fighter k at t (0..1): the recorded status, not the picture's fade"""
    if not (T.T_FIGHT <= t < T_KO):
        return 0.0
    left, dep = RUN['DET'][RD.sim_i(t), k]
    return float(dep) if left > 0 else 0.0


def colour(x, k, t, drive_k=1.0):
    """the fighter's tone at time t: thinner + dirtier as its HP drops; returns (signal, gain, lead)"""
    hp = hp_at(k, t) / 100.0
    lead = (hp_at(k, t) - hp_at(1 - k, t)) / 100.0
    cut = 60 + 260 * (1 - hp) ** 1.6
    y = A.hp(x, cut)
    drive = 1 + 4.0 * drive_k * (1 - hp) ** 2
    y = np.tanh(drive * y * 1.2) / np.tanh(drive * 1.2) * (1.0 / (1 + 0.15 * (drive - 1)))
    g = 10 ** (float(np.clip(6.0 * lead, -4.0, 3.5)) / 20)
    return y, g, lead


def clar_note(m, dur, vel, t):
    """the clarinet as the run has it at t: flat by up to a quarter-tone while OUT OF TUNE, with a beating second
    reed (sour), thinner as it's hurt"""
    dep = detune(KC, t)
    x = clar(m - 0.5 * dep, dur, vel)
    if dep > 0.25:
        x = x + 0.32 * dep * clar(m - 0.5 * dep - 0.22, dur, vel * 0.9)
    return colour(x, KC, t)


def oboe_note(m, dur, vel, t):
    return colour(oboe(m, dur, vel), KO_, t)


# ---------------------------------------------------------------- the storm (original): harpsichord, strings, timpani
_BC = {}


def harpsi(m, dur=0.7, vel=1.0):
    """a harpsichord string: bright plucked additive (plucked near the end), the 4' octave, a jack click"""
    key = ('h', m, round(dur, 2))
    if key not in _BC:
        n = int(dur * SR)
        t = A.tt(n)
        f = A.mtof(m)
        x = np.zeros(n)
        for k in range(1, 32):
            fk = f * k
            if fk > 12000:
                break
            a = abs(math.sin(math.pi * k * 0.13)) / k ** 0.75
            x += a * np.sin(2 * np.pi * fk * t + k) * np.exp(-t * (1.2 + 0.35 * k) * (f / 220) ** 0.35)
        f2 = 2 * f
        for k in range(1, 12):
            if f2 * k > 12000:
                break
            x += 0.32 * abs(math.sin(math.pi * k * 0.2)) / k * np.sin(2 * np.pi * f2 * k * t) * np.exp(-t * (1.6 + 0.4 * k))
        click = A.bp(np.random.default_rng(m).normal(0, 1, n), 2500, 9000) * np.exp(-t / 0.004) * 0.6
        _BC[key] = (x / 4 + click) * np.clip(t / 0.0015, 0, 1)
    return _BC[key] * vel


def strng(m, dur, vel=1.0, cut=2400, vib=0.0, att=0.008):
    """a string section note: 3 detuned saws (an ensemble), dark low-pass, bow attack; vibrato on long notes"""
    key = ('s', m, round(dur, 3), cut, round(vib, 3), round(att, 3))
    if key not in _BC:
        n = int((dur + 0.08) * SR)
        t = A.tt(n)
        f = A.mtof(m)
        x = np.zeros(n)
        for v, dt in enumerate((-0.006, 0.0, 0.007)):
            fr = f * (1 + dt) * (1 + vib * np.sin(2 * np.pi * (5.2 + 0.4 * v) * t + v))
            ph = np.cumsum(fr) / SR
            x += 2 * ((ph + 0.3 * v) % 1.0) - 1
        x = A.lp(A.hp(x / 3, 90), cut)
        env = np.clip(t / att, 0, 1) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.03))
        _BC[key] = x * env
    return _BC[key] * vel


def timp(m, vel=1.0):
    key = ('t', m)
    if key not in _BC:
        n = int(1.1 * SR)
        t = A.tt(n)
        f = A.mtof(m)
        fr = f * (1 + 0.04 * np.exp(-t / 0.05))
        ph = 2 * np.pi * np.cumsum(fr) / SR
        x = (np.sin(ph) + 0.5 * np.sin(1.5 * ph) * np.exp(-t / 0.3) + 0.25 * np.sin(2 * ph) * np.exp(-t / 0.2))
        x = x * np.exp(-t / 0.55) * np.clip(t / 0.003, 0, 1)
        mallet = A.lp(np.random.default_rng(m + 5).normal(0, 1, n), 1200) * np.exp(-t / 0.012) * 0.8
        _BC[key] = x + mallet
    return _BC[key] * vel


def thunder(dur=1.6, seed=0, crack=True):
    n = int(dur * SR)
    t = A.tt(n)
    r = np.random.default_rng(seed)
    rumble = A.lp(r.normal(0, 1, n), 140) * 3.0 * np.clip(t / 0.03, 0, 1) * np.exp(-t / (dur * 0.35))
    rumble *= 0.7 + 0.3 * np.sin(2 * np.pi * 3.1 * t + seed)
    x = rumble
    if crack:
        x = x + A.bp(r.normal(0, 1, n), 1500, 9000) * np.exp(-t / 0.03) * 0.9
    return x


def clank(m, seed=0):
    """a key or a ferrule hitting the canvas: a short metallic ring"""
    n = int(0.35 * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = sum(np.sin(2 * np.pi * f * r * t) * np.exp(-t / d) for r, d in ((1.0, 0.10), (2.76, 0.05), (5.4, 0.025)))
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


def scissor(seed=0):
    """DOUBLE REED's snip: two bright metallic scrapes a few ms apart"""
    n = int(0.12 * SR)
    t = A.tt(n)
    r = np.random.default_rng(seed)
    a = A.bp(r.normal(0, 1, n), 3000, 9000) * np.exp(-t / 0.012)
    b = np.roll(A.bp(r.normal(0, 1, n), 2500, 7000) * np.exp(-t / 0.018), int(0.028 * SR))
    return a + b * 0.9


# the storm's voicings (D minor)
HARP_RH = {'Dm': (62, 65, 69, 74), 'Gm': (62, 67, 70, 74), 'C': (64, 67, 72, 76), 'F': (65, 69, 72, 77),
           'Bb': (65, 70, 74, 77), 'A': (61, 64, 69, 73)}
VLN = {'Dm': (74, 69, 77, 69), 'Gm': (74, 70, 79, 70), 'C': (76, 72, 79, 72), 'F': (77, 72, 81, 72),
       'Bb': (77, 74, 82, 74), 'A': (76, 73, 81, 73)}   # violin I: a 16th cell per beat (top, mid, peak, mid)


def storm_bar(mix, b, dense=0.0, light=False):
    """one bar of the storm (bar index b). dense 0..1: the arrangement thickens (violin II, violas, 16th arpeggios,
    timpani on 3), never the level"""
    t0 = b * T.BAR
    ch = T.CHORDS[b % T.N_BARS]
    root = T.CHORD_ROOT[ch]
    lo = root - 12 if root >= 43 else root                              # basses: D1-A1 region (+ the octave)
    for e8 in range(8):                                                  # continuo + cellos / basses: driving 8ths
        te = t0 + e8 * T.BEAT / 2
        mix.place('harpsi', harpsi(lo + (12 if e8 % 2 else 0), 0.5), te, 0.16, pan=-0.15)
        if not light:
            mix.place('strings', strng(lo, 0.17, 1.0, 900), te, 0.10, pan=0.1)
            mix.place('strings', strng(lo + 12, 0.17, 0.8, 1200), te, 0.08, pan=0.1)
    for bt in (0, 2):                                                    # the harpsichord's right hand: chords
        for j, m in enumerate(HARP_RH[ch]):
            mix.place('harpsi', harpsi(m + 12, 0.6), t0 + bt * T.BEAT + j * 0.008, 0.07, pan=0.2)
    cell = VLN[ch]
    for st in range(16):                                                 # violin I: the racing 16ths
        if light and st % 2:
            continue
        m = cell[st % 4] - (12 if st // 4 in (1, 3) and st % 4 == 2 else 0)
        acc = 1.0 if st % 4 == 0 else 0.72
        mix.place('strings', strng(m, 0.075, acc, 2600), t0 + st * S16, 0.075, pan=-0.3)
        if dense > 0.35 and not light:                                   # violin II: a third / sixth under
            m2 = max([x for x in T.chord_tones(m - 9, m - 3, ch)] or [m - 12])
            mix.place('strings', strng(m2, 0.075, acc * 0.9, 2300), t0 + st * S16, 0.055, pan=0.35)
    if dense > 0.6 and not light:                                        # violas: a trembling chord (32nds)
        tones = T.chord_tones(57, 69, ch)[:2]
        for s32 in range(32):
            for m in tones:
                mix.place('strings', strng(m, 0.045, 0.6 + 0.4 * (s32 % 2 == 0), 1800), t0 + s32 * S16 / 2, 0.028)
    if dense > 0.5 and not light:                                        # 16th arpeggios on the harpsichord
        arp = HARP_RH[ch]
        for st in range(16):
            mix.place('harpsi', harpsi(arp[st % 4] + 12 * (st // 8 % 2), 0.3), t0 + st * S16, 0.045, pan=0.3)
    mix.place('drums', timp(root - 12 if root > 40 else root), t0, 0.40 if not light else 0.3)
    if dense > 0.45 or light:
        mix.place('drums', timp(root - 5 if root - 5 >= 33 else root + 7), t0 + 2 * T.BEAT, 0.30)
    mix.place('drums', A.hp(A.kick(1.0, 0.14, 150, 50)[:int(0.25 * SR)], 45), t0, 0.22)           # modern punch under
    mix.place('drums', A.hp(A.kick(1.0, 0.14, 150, 50)[:int(0.25 * SR)], 45), t0 + 2 * T.BEAT, 0.18)
    if not light:
        for bt in (1, 3):                                                # a snare-ish string chop on 2 and 4
            mix.place('strings', A.hp(strng(T.chord_tones(62, 74, ch)[0], 0.06, 1.0, 3500), 200), t0 + bt * T.BEAT, 0.07)


def tutti(mix, t, ch, g=1.0, dur=0.25):
    """a storm hit: strings + harpsichord + timpani + a low thunder thud on the chord"""
    for m in T.chord_tones(50, 77, ch):
        mix.place('strings', strng(m, dur, 1.0, 3000, att=0.004), t, 0.06 * g)
    for m in HARP_RH[ch]:
        mix.place('harpsi', harpsi(m, 0.8), t, 0.08 * g)
        mix.place('harpsi', harpsi(m + 12, 0.8), t, 0.05 * g)
    root = T.CHORD_ROOT[ch]
    mix.place('drums', timp(root - 12 if root > 40 else root), t, 0.6 * g)
    mix.place('fx', A.thud(0.8, 110, 32, seed=int(t * 50) + 3), t, 0.35 * g)
    mix.place('drums', A.crash(1.2), t, 0.05 * g)


def main():
    mix = A.Mix(T.DUR + 4.0)
    bar_ko = int(T_KO // T.BAR)
    # ------------------------------------------------ bar 0: the VS title card over the storm at full level
    t1, t2, t3 = 0.004, T.BEAT, 2 * T.BEAT                 # CLARINET 4 ms in (no click at the seam), VS, OBOE
    storm_bar(mix, 0, dense=0.7)
    tutti(mix, t1, 'Dm', 0.8)
    tutti(mix, t2, 'Dm', 0.75)
    tutti(mix, t3, 'Dm', 0.75)
    for j, m in enumerate((50, 53, 57, 62, 65, 69, 74)):       # CLARINET: a glissando blast up through the registers
        x = clar(m, 0.10 if j < 6 else 0.5, 1.0)
        mix.place('clarinet', gliss(x, 2.0, 0.04), t1 + j * 0.032, 0.42)
    mix.place('fx', thunder(1.4, 91), t2, 0.55)              # VS: the thunder crack where it splits the divider
    a_hold = 1.6 * T.BEAT
    mix.place('oboe', oboe(69, a_hold, 1.0), t3, 0.52)       # OBOE: its A, held (the tuning note)
    mix.place('oboe', oboe(81, a_hold, 0.9), t3, 0.30)
    for st in range(12, 16):                                 # beat 4: a string run up as the doors open
        mix.place('strings', strng((69, 72, 74, 77)[st - 12], 0.08, 1.0, 3000), st * S16, 0.12)
    mix.place('fx', A.whoosh(T.T_DOORS[1] - T.T_DOORS[0] + 0.1, 300, 5000), T.T_DOORS[0] - 0.05, 0.14)
    # ------------------------------------------------ bar 1: the count-in on the dominant (A)
    storm_bar(mix, 1, dense=0.4)
    for j, (tc, txt) in enumerate(T.COUNT[:3]):
        tutti(mix, tc, 'A', 0.8)
    for st in range(12, 16):                                 # the run into FIGHT!
        mix.place('strings', strng((69, 73, 76, 81)[st - 12], 0.08, 1.0, 3200), T.BAR + st * S16, 0.12)
    mix.place('fx', A.whoosh(T.BEAT, 400, 6000), T.BAR + 3 * T.BEAT, 0.12)
    tutti(mix, T.T_FIGHT, 'Dm', 1.1)                         # the drop
    mix.place('fx', thunder(1.8, 7), T.T_FIGHT, 0.40)
    # ------------------------------------------------ the storm: bar 2 (FIGHT!) .. the K.O. (thickens as it goes)
    for b in range(2, bar_ko + 1):
        storm_bar(mix, b, dense=min(1.0, (b - 2) / 16.0))
    for t_, who, name in RD.STORY['caps']:                   # thunder under every captioned ability
        mix.place('fx', thunder(1.6, int(t_ * 10), crack=False), q16(t_), 0.35)
    for k in ('strings', 'harpsi', 'drums'):
        mix.buses[k] = A.tape_stop(mix.buses[k], T_KO, 0.42)
    # ------------------------------------------------ the fighters, from the run
    jumps = [e for e in EV if e[1] == 'jump' and e[2] == KC and e[0] <= T_KO + 0.01]
    jt = [e[0] for e in jumps]
    rh_n = 0
    last_slot = None
    for j, e in enumerate(jumps):
        t, hit, kind = e[0], e[7], e[8]
        tq = q16(t)
        slot = round(tq / S16)
        if slot == last_slot:
            continue
        last_slot = slot
        gap = (jt[j + 1] - t) if j + 1 < len(jt) else T.BEAT
        if kind == 1:                                        # RHAPSODY: five rising notes gliding up the registers
            m = (50, 57, 62, 69, 74)[min(rh_n, 4)]
            rh_n += 1
            sig, g, _ = clar_note(m, 0.22, 1.0, t)
            mix.place('clarinet', gliss(sig, 5.0, 0.07), tq, 0.46 * g, pan=-0.2)
            mix.place('fx', A.whoosh(0.18, 600, 7000), tq - 0.05, 0.06)
            continue
        hp = hp_at(KC, t) / 100.0
        if h01(slot, 3) < 0.45 * (1 - hp) ** 1.6:            # sparse when hurt
            continue
        on_beat = slot % 4 == 0
        m = T.clarinet_at(tq)
        if not on_beat:                                      # CLARION's 8ths: a passing note toward the next beat
            nxt = T.clarinet_at(tq + T.BEAT / 2)
            pas = [x for x in T.scale_notes(min(m, nxt), max(m, nxt), T.chord_at(tq)) if x not in (m, nxt)]
            m = pas[len(pas) // 2] if pas else nxt
        dur = min(0.40, max(0.12, gap * 0.92))
        sig, g, lead = clar_note(m, dur, 0.85 if on_beat else 0.7, t)
        mix.place('clarinet', sig, tq, (0.40 if on_beat else 0.30) * g, pan=-0.22)
        if lead > 0.04 and on_beat:                          # the leader: a third under (fuller)
            under = T.chord_tones(m - 5, m - 3, T.chord_at(tq))
            if under:
                mix.place('clarinet', clar_note(under[-1], dur, 0.6, t)[0], tq, 0.16 * g, pan=-0.3)
        if hit:                                              # a landed cut: the twelfth above (the register key)
            mix.place('clarinet', clar_note(m + 19, 0.09, 0.85, t)[0], tq + 0.03, 0.26 * g, pan=-0.1)
            mix.place('fx', A.whoosh(0.10, 1500, 9000), tq - 0.02, 0.05)
    for e in EV:
        t, kind = e[0], e[1]
        if t > T_KO + 0.01:
            break
        if kind == 'tune' and e[2] == KO_:                   # the tuning A with every beam
            tq = q16(t)
            sig, g, lead = oboe_note(69, 0.75 * T.BEAT, 0.95, t)
            mix.place('oboe', sig, tq, 0.48 * g, pan=0.22)
            if lead > 0.04:
                mix.place('oboe', oboe_note(81, 0.75 * T.BEAT, 0.6, t)[0], tq, 0.14 * g, pan=0.25)
        elif kind == 'hit' and e[2] == KO_ and e[7] == 'tune':    # it lands: the A rings out against a sour twin
            tq = q16(t)
            mix.place('oboe', oboe_note(81, 0.25, 0.9, t)[0], tq + S16, 0.22, pan=0.2)
            sour = clar(69 - 0.5, 0.3, 0.7) * 0.5
            mix.place('clarinet', colour(sour, KC, t)[0], tq + S16, 0.18, pan=-0.15)
        elif kind == 'hit' and e[2] == KO_ and e[7] == 'sting':   # each sting: one of the oboe's running 8ths
            tq = q16(t)
            hp = hp_at(KO_, t) / 100.0
            if h01(round(tq / S16), 11) < 0.4 * (1 - hp) ** 1.6:
                continue
            m = T.oboe_8th_at(tq)
            sig, g, _ = oboe_note(m, 0.13, 0.78, t)
            mix.place('oboe', sig, tq, 0.30 * g, pan=0.25)
        elif kind == 'endless' and e[2] == KO_:              # ENDLESS NOTE: the A held for a bar (circular breathing)
            tq = q16(t)
            dur = e[3] - t
            sig, g, _ = oboe_note(69, dur, 1.0, t)
            mix.place('oboe', sig * np.linspace(0.85, 1.1, len(sig)), tq, 0.46 * g, pan=0.2)
            mix.place('oboe', oboe_note(57, dur, 0.7, t)[0], tq, 0.14 * g, pan=0.2)
        elif kind == 'snip_open' and e[2] == KO_:            # DOUBLE REED: a reed crow as the blades open
            tq = q16(t)
            x = oboe(86, 0.12, 1.0)
            mix.place('oboe', gliss(x, 3.0, 0.06), tq, 0.22, pan=0.25)
        elif kind == 'snip' and e[2] == KO_:
            mix.place('fx', scissor(int(t * 10)), q16(t), 0.30 if e[3] else 0.16, pan=0.15)
            if e[3]:
                mix.place('fx', A.thud(0.3, 160, 60, seed=int(t * 7)), q16(t), 0.25)
        elif kind == 'stun':                                 # the stunned enemy: a dizzy "boing"
            n = int(0.4 * SR)
            tt_ = A.tt(n)
            boing = np.sin(2 * np.pi * np.cumsum(420 + 140 * np.sin(2 * np.pi * 9 * tt_)) / SR) * np.exp(-tt_ / 0.15)
            mix.place('fx', boing, q16(t) + 0.03, 0.08)
        elif kind == 'rope':
            mix.place('fx', A.thud(0.22, 95, 55, seed=int(t * 10)), t, 0.05 * min(1.5, e[7] / 2.6),
                      pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
        elif kind == 'clash':
            mix.place('fx', A.thud(0.15, 200, 90, seed=int(t * 3)), t, 0.06)
        elif kind == 'ability':
            who, name = e[2], e[3]
            tq = q16(t)
            if name == 'SQUEAK':                             # the infamous squeak: a shrill, wobbling squeal
                x = clar(93.0, 0.22, 1.0, 'goodman')
                x = np.tanh(3.0 * gliss(x, -1.5, 0.05) / (np.abs(x).max() + 1e-9)) * 0.5
                mix.place('clarinet', x, tq, 0.34, pan=-0.2)
            elif name == 'CLARION':                          # a rip up a twelfth into the clarion register
                for j, m in enumerate((57, 62, 69, 76)):
                    mix.place('clarinet', clar_note(m, 0.09, 0.9, t)[0], tq + j * 0.035, 0.30, pan=-0.2)
            elif name == 'ILL WIND':                         # a sour gust: a bent, beating A over a whoosh
                dur = 2 * T.BEAT
                x = oboe(81, dur, 1.0)
                y = bend(x, -1.5, 0.05, dur)
                y2 = bend(oboe(80.5, dur, 0.8), -1.8, 0.05, dur)
                mix.place('oboe', colour(y + 0.6 * y2, KO_, t)[0], tq, 0.36, pan=0.2)
                mix.place('fx', A.whoosh(1.2, 200, 3000, rise=False), tq, 0.22)
                mix.place('fx', thunder(1.4, 33, crack=False), tq, 0.30)
            elif name == 'RHAPSODY':
                mix.place('drums', A.crash(1.6), tq, 0.08)
    # ------------------------------------------------ the K.O. moment
    tutti(mix, T_KO, 'Dm', 1.0, 0.5)
    mix.place('fx', thunder(2.4, 51), T_KO, 0.55)
    loser = NAMES[LOSE]
    if loser == 'OBOE':                                      # the oboe falls apart: a falling, sour A, keys clattering
        x = oboe(69, 1.0, 1.0)
        mix.place('oboe', bend(x, -8.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x)), RD.T_HS, 0.40)
    else:
        x = clar(62, 1.0, 1.0)
        mix.place('clarinet', bend(x, -8.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x)), RD.T_HS, 0.40)
    for j, m in enumerate((86, 81, 91, 77, 84)):
        mix.place('fx', clank(m, 20 + j), RD.T_HS + 0.08 * j, 0.07, pan=(j - 2) / 5)
    mix.place('fx', roar(5.5), T_KO + 0.2, 0.22)
    tw = q16(T_KO + 1.3)                                     # the winner's stinger on "(WINNER) WINS"
    tutti(mix, tw, 'Dm', 1.0, 0.6)
    if NAMES[WIN] == 'CLARINET':
        for j, m in enumerate((50, 53, 57, 62)):
            mix.place('solo', clar(m, 0.9 - 0.06 * j, 1.0), tw + j * 0.03, 0.30)
    else:
        mix.place('solo', oboe(69, 0.9, 1.0), tw, 0.45)
        mix.place('solo', oboe(74, 0.9, 0.9), tw, 0.30)
    solo0 = RD.T_SOLO
    for b in range(bar_ko + 1, int(round(solo0 / T.BAR))):  # the storm holds its breath, but never goes quiet:
        tq = b * T.BAR                                       # trembling strings + timpani until the solo
        for m in T.chord_tones(50, 69, 'Dm'):
            for s32 in range(32):
                mix.place('strings', strng(m, 0.045, 0.8, 2200), tq + s32 * S16 / 2, 0.035)
        mix.place('drums', timp(38), tq, 0.4)
        mix.place('drums', timp(45), tq + 2 * T.BEAT, 0.3)
    # ------------------------------------------------ victory: a lighter storm + the winner's solo
    b_solo = int(round(solo0 / T.BAR))
    for b in range(b_solo, 34):
        storm_bar(mix, b, dense=0.5, light=True)
        for m in T.chord_tones(50, 69, T.CHORDS[b]):         # a held string chord under the solo
            mix.place('strings', strng(m, T.BAR, 0.7, 1800, vib=0.004, att=0.08), b * T.BAR, 0.08)
    for bt, m, d in T.SOLO[NAMES[WIN]]:
        ts = solo0 + bt * T.BEAT
        if ts > T.T_REASSEMBLE - 0.05:
            break
        if NAMES[WIN] == 'CLARINET':
            mix.place('solo', gliss(clar(m, d * T.BEAT * 0.95, 0.95, 'boehm'), 1.0, 0.03), ts, 0.50)
        else:
            mix.place('solo', oboe(m, d * T.BEAT * 0.95, 0.95), ts, 0.52)
    # the loser reassembles: its pieces fly home as a rising run, a glass "pop" when it's whole
    run = [62, 65, 69, 74, 77, 81]
    for j, m in enumerate(run):
        tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
        if loser == 'OBOE':
            mix.place('oboe', oboe(m, 0.12, 0.7), tj, 0.20)
        else:
            mix.place('clarinet', clar(m - 12, 0.12, 0.7), tj, 0.22)
    mix.place('fx', A.chime(95, 0.5) + A.chime(102, 0.5) * 0.5, T.T_GLIDE, 0.12)
    # ------------------------------------------------ bars 34-35: the storm returns, building into the title slam
    for b in (34, 35):
        storm_bar(mix, b, dense=1.0)
    t35 = 35 * T.BAR
    for bt in range(4):
        mix.place('drums', timp(45), t35 + bt * T.BEAT, 0.36)
        mix.place('drums', timp(45), t35 + (bt + 0.5) * T.BEAT, 0.24)
    st = 0.0
    while t35 + 2 * T.BEAT + st < T.DUR - 1e-4:               # a timpani roll into the seam
        mix.place('drums', timp(45, 0.6), t35 + 2 * T.BEAT + st, 0.20)
        st += S16
    mix.place('fx', riser(T.DUR - t35), t35, 0.20)
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


BUS_GAIN = {'clarinet': 1.5, 'oboe': 1.3, 'solo': 1.5, 'fx': 1.2, 'drums': 1.0, 'strings': 1.6, 'harpsi': 1.3}


def master(mix, thr=0.20, ceiling=0.89):
    """ONE master gain (no section automation: consistent loudness from frame 0) + a compressor and a soft limiter;
    the overflow past the loop (tails, reverb) folds onto the start; NA samples (whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * w for k, w in (('clarinet', 0.35), ('oboe', 0.3), ('solo', 0.4), ('fx', 0.25),
                                                ('strings', 0.45), ('harpsi', 0.3), ('drums', 0.15)))
    wet = A.reverb(send, dur=2.2, decay=0.5) * 0.22
    full = sum(buses.values()) + wet

    def fold(x):
        y = x[:NS].copy()
        tail = x[NS:]
        y[:len(tail)] += tail[:NS]
        return y
    m = fold(full)
    from scipy.signal import lfilter
    env = np.sqrt(lfilter([0.002], [1, -0.998], (np.concatenate([m[-SR:], m]) ** 2).mean(1)))[SR:]
    gain = np.where(env > thr, (thr / np.maximum(env, 1e-9)) ** 0.5, 1.0)[:, None]
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
