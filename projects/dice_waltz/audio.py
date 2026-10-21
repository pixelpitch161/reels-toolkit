"""Mozart's Dice Waltz - soundtrack. Everything is synthesised; the music is the 1792 waltz table
(public domain), with every bar chosen by the dice rolls in timeline.py.

    python audio.py        -> audio.wav, prints section levels and the dice-clack band check

The 'science sound' is the dice: every clack is a roll that picks the next bar. The clacks sit on
top of the music in the 2.5-6 kHz band (checked at the end)."""
import math
import os
import sys
import numpy as np

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
from reelkit import audio as A
import timeline as T
import waltz_table as WT

SR = A.SR
RNG = np.random.default_rng(1792)
_cache = {}


def tt(n):
    return np.arange(n) / SR


# ---------------------------------------------------------------- instruments
def fortepiano(m, dur, vel=0.9):
    """Viennese fortepiano: slightly inharmonic partials, two detuned strings, hammer thump,
    quick leather dampers (short sustain, bright attack)"""
    key = ('fp', m, round(dur, 3), round(vel, 2))
    if key in _cache:
        return _cache[key]
    f = A.mtof(m)
    rel = 0.09
    tail = min(2.2, 0.35 + 1.3 * (261.6 / f) ** 0.35)
    n = int((dur + rel + 0.05) * SR)
    t = tt(n)
    B = 0.00025 * (f / 261.6) ** 0.5
    tau0 = 1.5 * (261.6 / f) ** 0.45
    out = np.zeros(n)
    K = int(min(18, 9000 / f))
    for k in range(1, K + 1):
        fk = k * f * math.sqrt(1 + B * k * k)
        amp = (1.0 / k ** 1.15) * math.exp(-k * (0.16 - 0.08 * vel))
        tau = tau0 / (1 + 0.45 * (k - 1))
        ph = RNG.uniform(0, 6.283)
        env = np.exp(-t / tau)
        out += amp * env * (np.sin(2 * np.pi * fk * 1.0004 * t + ph) + np.sin(2 * np.pi * fk * 0.9996 * t + ph * 1.3)) * 0.5
    thump = A.lp(np.random.default_rng(m).normal(0, 1, n), 1800) * np.exp(-t / 0.007) * 0.18 * vel
    out = out + thump
    damp = np.where(t < dur, 1.0, np.exp(-(t - dur) / rel * 3))
    out *= damp * np.clip(t / 0.002, 0, 1)
    out = A.lp(out, 6500) * vel
    _cache[key] = out
    return out


def music_box(m, dur=0.0, vel=0.8):
    key = ('mb', m, round(vel, 2))
    if key in _cache:
        return _cache[key]
    f = A.mtof(m)
    n = int(1.6 * SR)
    t = tt(n)
    out = (np.sin(2 * np.pi * f * t) * np.exp(-t / 0.9) + 0.32 * np.sin(2 * np.pi * f * 2.76 * t) * np.exp(-t / 0.22)
           + 0.12 * np.sin(2 * np.pi * f * 5.4 * t) * np.exp(-t / 0.07))
    out *= np.clip(t / 0.0015, 0, 1) * vel
    _cache[key] = out
    return out


def pizz(m, dur=0.0, vel=0.8):
    key = ('pz', m, round(vel, 2))
    if key in _cache:
        return _cache[key]
    f = A.mtof(m)
    n = int(0.9 * SR)
    t = tt(n)
    out = np.zeros(n)
    for k in range(1, int(min(14, 6000 / f)) + 1):
        out += (1.0 / k) * np.sin(2 * np.pi * f * k * t + k) * np.exp(-t / (0.32 / (1 + 0.55 * (k - 1))))
    out = A.lp(out, 3200) * np.clip(t / 0.003, 0, 1) * vel
    _cache[key] = out
    return out


def harp(m, dur=0.0, vel=0.8):
    key = ('hp', m, round(vel, 2))
    if key in _cache:
        return _cache[key]
    f = A.mtof(m)
    n = int(1.4 * SR)
    t = tt(n)
    out = np.zeros(n)
    for k in range(1, int(min(10, 7000 / f)) + 1):
        out += (1.0 / k ** 1.6) * np.sin(2 * np.pi * f * k * t) * np.exp(-t / (0.9 / (1 + 0.4 * (k - 1))))
    out *= np.clip(t / 0.002, 0, 1) * vel
    _cache[key] = out
    return out


def celesta(m, dur=0.0, vel=0.8):
    key = ('ce', m, round(vel, 2))
    if key in _cache:
        return _cache[key]
    f = A.mtof(m)
    n = int(1.1 * SR)
    t = tt(n)
    out = np.sin(2 * np.pi * f * t) * np.exp(-t / 0.55) + 0.25 * np.sin(2 * np.pi * f * 4.0 * t) * np.exp(-t / 0.09)
    out *= np.clip(t / 0.002, 0, 1) * vel
    _cache[key] = out
    return out


def flute(m, dur, vel=0.8):
    key = ('fl', m, round(dur, 3), round(vel, 2))
    if key in _cache:
        return _cache[key]
    f = A.mtof(m)
    n = int((dur + 0.12) * SR)
    t = tt(n)
    vib = 1 + 0.004 * np.sin(2 * np.pi * 5.2 * t) * np.clip((t - 0.15) / 0.2, 0, 1)
    ph = 2 * np.pi * f * np.cumsum(vib) / SR
    out = np.sin(ph) + 0.22 * np.sin(2 * ph) + 0.06 * np.sin(3 * ph)
    breath = A.bp(np.random.default_rng(m + 7).normal(0, 1, n), f * 0.8, min(f * 4, 9000)) * 0.08
    env = A.adsr(n, 0.04, 0.06, 0.82, 0.1)
    out = (out + breath) * env * vel
    _cache[key] = out
    return out


def clarinet(m, dur, vel=0.8):
    key = ('cl', m, round(dur, 3), round(vel, 2))
    if key in _cache:
        return _cache[key]
    f = A.mtof(m)
    n = int((dur + 0.1) * SR)
    t = tt(n)
    out = sum(a * np.sin(2 * np.pi * f * k * t) for k, a in ((1, 1.0), (3, 0.42), (5, 0.2), (7, 0.1)))
    out = A.lp(out, 3000) * A.adsr(n, 0.03, 0.08, 0.78, 0.08) * vel
    _cache[key] = out
    return out


# cup -> (instrument, transpose, gain, pan, play left hand too?)
VOICE = {
    'A': (fortepiano, 0, 1.0, 0.0, True),
    'B': (music_box, 12, 0.5, -0.35, False),
    'C': (pizz, -12, 0.55, 0.35, False),
    'D': (flute, 0, 0.32, -0.15, False),
    'E': (celesta, 12, 0.3, 0.5, False),
    'F': (harp, 0, 0.36, -0.55, False),
    'G': (clarinet, -12, 0.24, 0.2, False),
    'H': (music_box, 12, 0.14, -0.6, False),
    'I': (celesta, 12, 0.1, 0.65, False),
    'J': (music_box, 0, 0.1, 0.0, False),
}


def place_bar(mix, bus, bar, t0, inst=fortepiano, tr=0, gain=1.0, pan=0.0, lh=True, second=False, vel=0.9):
    rh, lhn = WT.bar_notes(bar, second)
    for o, d, p in rh:
        v = vel * (1.0 if o == 0 else 0.84)
        tj = t0 + o * T.S16 + RNG.normal(0, 0.004)
        mix.place(bus, inst(p + tr, d * T.S16, v), tj, gain, pan)
    if lh:
        for o, d, p in lhn:
            v = vel * (0.8 if o == 0 else 0.62)
            tj = t0 + o * T.S16 + RNG.normal(0, 0.004)
            mix.place(bus, inst(p + tr, d * T.S16, v), tj, gain * 0.85, pan - 0.15)


# ---------------------------------------------------------------- dice + foley
def click(rng, bright=1.0, wood=1.0):
    n = int(0.045 * SR)
    t = tt(n)
    noise = A.bp(rng.normal(0, 1, n), 1800 * wood, min(9000, 7000 * wood)) * np.exp(-t / (0.0028 + 0.002 * (1 - bright)))
    tone = np.sin(2 * np.pi * rng.uniform(2300, 3900) * wood * t) * np.exp(-t / 0.011) * 0.5
    tone2 = np.sin(2 * np.pi * rng.uniform(1100, 1700) * wood * t) * np.exp(-t / 0.018) * 0.25
    return (noise * 0.9 + tone + tone2) * bright


def landing(mix, t0, strength=1.0, seed=0, wood=1.0, bus='dice', pan=0.0):
    """two dice: first contact, two bounces, a little tumble"""
    rng = np.random.default_rng(seed)
    for die in range(2):
        dt = die * rng.uniform(0.012, 0.035)
        for k, (off, a) in enumerate(((0.0, 1.0), (0.085, 0.55), (0.16, 0.32), (0.22, 0.18), (0.27, 0.1))):
            mix.place(bus, click(rng, a, wood), t0 + dt + off + rng.uniform(-0.008, 0.008), strength * 0.9 * a ** 0.3, pan + rng.uniform(-0.15, 0.15))


def rattle(mix, t0, dur, strength=0.5, seed=0, bus='dice'):
    """dice shaken inside a leather cup (muffled)"""
    rng = np.random.default_rng(seed)
    nclk = int(dur * 34)
    for k in range(nclk):
        tk = t0 + rng.uniform(0, dur)
        c = A.lp(click(rng, rng.uniform(0.3, 0.8), 0.7), 3500)
        mix.place(bus, c, tk, strength * 0.6, rng.uniform(-0.2, 0.2))


def whoosh_s(dur, lo, hi, rise=True, seed=6):
    return A.whoosh(dur, lo, hi, rise, seed)


def paper(dur, seed=3):
    n = int(dur * SR)
    t = tt(n)
    x = A.bp(np.random.default_rng(seed).normal(0, 1, n), 900, 6000)
    am = 0.5 + 0.5 * np.abs(np.sin(2 * np.pi * (3 + 9 * np.random.default_rng(seed + 1).random()) * t + np.sin(t * 17)))
    return x * am * np.sin(np.linspace(0, np.pi, n)) ** 0.6


def chime_up(m0, n_notes, step=0.07, dur=0.9):
    out = np.zeros(int((n_notes * step + dur) * SR))
    scale = [0, 2, 4, 7, 9, 12, 14, 16, 19, 21, 24]
    for k in range(n_notes):
        s = music_box(m0 + scale[k % len(scale)] + 12 * (k // len(scale)), vel=0.7)
        i = int(k * step * SR)
        out[i:i + len(s)] += s[:len(out) - i]
    return out


def timp(m=36, dec=0.5):
    n = int(1.2 * SR)
    t = tt(n)
    f = A.mtof(m)
    return (np.sin(2 * np.pi * f * t) + 0.5 * np.sin(2 * np.pi * f * 1.5 * t) * np.exp(-t / 0.15)) * np.exp(-t / dec) * np.clip(t / 0.004, 0, 1) \
        + A.lp(np.random.default_rng(2).normal(0, 1, n), 600) * np.exp(-t / 0.02) * 0.3


# ---------------------------------------------------------------- arrangement
def main():
    mix = A.Mix(T.DUR)
    n = int(T.DUR * SR)
    r0 = np.random.default_rng(1)
    # room tone + candle crackle (very quiet)
    room = A.lp(r0.normal(0, 1, n), 300) * 0.05
    mix.place('amb', room, 0.0, 0.5)
    crk = np.zeros(n)
    for k in range(260):
        i = int(r0.uniform(0, n - 2000))
        crk[i:i + 400] += A.hp(r0.normal(0, 1, 400), 2500) * np.exp(-np.arange(400) / 60.0) * r0.uniform(0.2, 1)
    mix.place('amb', crk, 0.0, 0.05)

    # ---- dice: every roll (the science sound)
    for r in T.ROLLS:
        wood = 1.0 if r.where == 'tray' else 0.75
        st = 1.0 if r.cup == 'A' else (0.8 if r.cup == 'B' else 0.38)
        pan = 0.0 if r.cup == 'A' else (-0.3 if r.cup == 'B' else float(np.clip(np.sin(T.CUPS.index(r.cup) * 1.3), -0.7, 0.7)))
        if r.cup in 'AB':
            rattle(mix, r.t_throw - 0.34, 0.3, 0.45 * st, seed=r.seed)
        landing(mix, r.t_land, st, seed=r.seed, wood=wood, pan=pan)
        if r.cup == 'A' and r.t_land < 30:
            mix.place('fx', whoosh_s(0.35, 600, 3000, seed=r.seed), r.t_throw - 0.05, 0.05)
    for (tr, s, x, z, yaw, seed) in T.RAIN:
        landing(mix, tr, 0.22, seed=seed % 10000, wood=0.75, pan=float(np.clip(x / 1.5, -0.8, 0.8)))
    # the loop: dice hop into the cup, the cup is shaken, the throw that lands at 0.25 s
    rattle(mix, T.T_SETTLE - 0.3, 0.35, 0.5, seed=91)
    rattle(mix, T.T_SHAKE, T.T_LOOP_THROW - T.T_SHAKE - 0.05, 0.55, seed=92)
    mix.place('fx', whoosh_s(0.25, 600, 3000, seed=4), T.T_LOOP_THROW - 0.05, 0.05)

    # ---- waltz #1: two solo bars, then the whole waltz as the dice compose it
    for k, tp in T.SOLO_PLAYS:
        place_bar(mix, 'music', T.W1_BARS[k], tp, vel=0.95)
    for k in range(16):
        place_bar(mix, 'music', T.W1_BARS[k], T.w1_play(k), second=(k == 7), vel=0.92 if k % 4 else 1.0)
    # reprise during the time-lapse: bars 1-4 on the music box, far away
    for k in range(4):
        place_bar(mix, 'music', T.W1_BARS[k], T.T_YEARS + 1.0 + k * T.BAR, inst=music_box, tr=12, gain=0.32, lh=False, vel=0.8)
    # ---- back half: every cup's waltz on its own timbre, stacked
    for c in T.CUPS:
        inst, tr, g, pan, lh = VOICE[c]
        for (k, s, bar, tp) in T.W2[c]:
            fade = 1.0
            place_bar(mix, 'music', bar, tp, inst=inst, tr=tr, gain=g * fade, pan=pan, lh=lh, second=(k == 7), vel=0.9)
    # weight under the climax: low pizzicato doubles the lead's bass + soft timpani downbeats
    for (k, s, bar, tp) in T.W2['A']:
        if 9 <= k <= 14:
            _, lhn = WT.bar_notes(bar)
            for o, d, p in lhn:
                if o == 0:
                    mix.place('music', pizz(p - 12, vel=0.9), tp, 0.5, 0.1)
            mix.place('drums', timp(36 if WT.bar_notes(bar)[1][0][2] % 12 == 0 else 31, 0.45), tp, 0.38)
    mix.place('fx', A.crash(3.0), T.T_CLIMAX - 0.02, 0.07)
    mix.place('fx', whoosh_s(1.4, 300, 6000, seed=8), T.T_CLIMAX - 1.4, 0.12)

    # ---- foley + transitions
    mix.place('fx', paper(T.T_UNFURL[1] - T.T_UNFURL[0], 3), T.T_UNFURL[0], 0.08)
    mix.place('fx', paper(T.T_REROLL[1] - T.T_REROLL[0], 5), T.T_REROLL[0], 0.06)
    mix.place('fx', whoosh_s(0.6, 900, 5000, seed=9), 9.2, 0.07)                 # the demo bar flies to the staff
    mix.place('fx', chime_up(72, 6, 0.06), 9.65, 0.12)
    mix.place('fx', chime_up(67, 8, 0.05), T.T_HIST + 0.05, 0.1)                 # histogram lights up
    mix.place('fx', whoosh_s(0.5, 5000, 400, rise=False, seed=10), T.T_SNAP - 0.05, 0.12)   # the number snaps down
    mix.place('drums', A.kick(0.9, 0.3, 120, 40), T.T_SNAP + 0.2, 0.35)
    # calendar pages flipping faster and faster + a time-lapse riser
    tp_, k = T.T_YEARS + 0.4, 0
    while tp_ < T.T_TWO - 0.3:
        mix.place('fx', paper(0.07, 20 + k), tp_, 0.05 + 0.03 * (tp_ - T.T_YEARS) / 7)
        tp_ += max(0.05, 0.4 * 0.86 ** k)
        k += 1
    mix.place('fx', whoosh_s(1.8, 250, 5000, seed=11), T.T_TWO - 1.9, 0.12)
    # outro: the staff empties right to left, one descending music-box note per bar
    for j in range(16):
        tj = T.T_CLEAR[0] + (T.T_CLEAR[1] - T.T_CLEAR[0]) * j / 16
        mix.place('music', music_box(96 - [0, 3, 5, 7, 10][j % 5] - 12 * (j // 5), vel=0.6), tj, 0.16, 0.4 - 0.05 * j)

    # carve a pocket for the dice: about -8 dB at 2.5-6 kHz on the music bus
    mb = mix.buses['music']
    mix.buses['music'] = mb - 0.6 * A.bp(mb, 2500, 6000, 2)
    out = mix.master(gains={'dice': 1.45, 'music': 0.9, 'fx': 1.0, 'drums': 0.8, 'amb': 1.0},
                     reverb_buses={'music': 1.0, 'fx': 0.4, 'dice': 0.12},
                     gain_db_pts=[(0, 0), (34.5, 0), (35.5, 0.5), (45.5, 1.0), (52, 1.0), (54, 0), (60, 0)])
    A.write('audio.wav', out)
    spans = [(0, 4), (4, 9), (9, 28), (28, 35), (35, 46), (46, 52), (52, 60)]
    for a, b, rms, pk in A.section_levels('audio.wav', spans):
        print(f'{a:4.0f}-{b:4.0f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')
    check_dice_band()


def check_dice_band():
    """the clacks must sit on top of the music in their band: compare 2.5-6 kHz energy right at the
    lead landings with the same band 0.3-0.6 s later (music only)"""
    lands = [r.t_land for r in T.LEAD if r.t_land > 9 and r.t_land < 34]
    on = A.band_level('audio.wav', 2500, 6000, [(t, t + 0.08) for t in lands])
    off = A.band_level('audio.wav', 2500, 6000, [(t + 0.45, t + 0.7) for t in lands])
    diff = [a[2] - b[2] for a, b in zip(on, off)]
    print(f'dice band 2.5-6 kHz, clack vs music right after (front half): mean {np.mean(diff):+.1f} dB, min {np.min(diff):+.1f} dB')
    lands2 = sorted(r.t_land for r in T.ROLLS if r.cup == 'A' and r.t_land > 35)
    on = A.band_level('audio.wav', 2500, 6000, [(t, t + 0.08) for t in lands2])
    off = A.band_level('audio.wav', 2500, 6000, [(t + 0.5, t + 0.62) for t in lands2])
    diff = [a[2] - b[2] for a, b in zip(on, off)]
    print(f'dice band, back half (stacked waltzes): mean {np.mean(diff):+.1f} dB, min {np.min(diff):+.1f} dB')


if __name__ == '__main__':
    main()
