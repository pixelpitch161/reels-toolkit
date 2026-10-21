"""The Perfect Loop - soundtrack. Everything is synthesised and original.

    python audio.py        -> audio.wav, prints section levels and the pluck-band check

The science sound is the plucks: every lantern plucks its own note (3 octaves of C major
pentatonic, longest = lowest) each time it is back at its release side, at exactly the trigger
times in timeline.py. So the patterns ARE the rhythm: 2 groups = eighths, 3 = triplets ...
From 35 s an original bass + pad change chord with each pattern, and a soft groove locks to the
middle pendulum (n = 58 -> 58 BPM, one pluck per beat). The build lands on the near-unison
strum at 58.97 s; the exact unison at 60.0 s is frame 0 of the loop."""
import math
import os
import sys
import numpy as np
from scipy.io import wavfile

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
from reelkit import audio as A
import timeline as T

SR = A.SR
_cache = {}


def tt(n):
    return np.arange(n) / SR


# ---------------------------------------------------------------- instruments
def lantern_pluck(m):
    """glassy mallet pluck: a few partials (one slightly inharmonic) + a tiny strike click.
    Short decay, because 15 voices make ~14.5 plucks per second."""
    if m in _cache:
        return _cache[m]
    f = A.mtof(m)
    dec = 0.4 * (261.6 / f) ** 0.3
    n = int(min(1.2, dec * 4) * SR)
    t = tt(n)
    out = np.zeros(n)
    for ratio, amp, dk in ((1.0, 1.0, 1.0), (2.0, 0.45, 0.42), (3.0, 0.2, 0.28), (4.18, 0.13, 0.16)):
        if f * ratio < 9000:
            out += amp * np.sin(2 * np.pi * f * ratio * t) * np.exp(-t / (dec * dk))
    click = A.hp(np.random.default_rng(m).normal(0, 1, n), 2500) * np.exp(-t / 0.0018) * 0.12
    out = (out + click) * np.clip(t / 0.0015, 0, 1)
    out = A.lp(out, 7500)
    _cache[m] = out
    return out


def bass_note(m, dur, vel=1.0):
    n = int((dur + 0.08) * SR)
    t = tt(n)
    f = A.mtof(m)
    x = np.sin(2 * np.pi * f * t) + 0.35 * np.sin(4 * np.pi * f * t) + 0.12 * np.sin(6 * np.pi * f * t)
    x = np.tanh(1.4 * x) * A.adsr(n, 0.006, 0.15, 0.7, 0.07)
    return x * vel


def soft_kick():
    return A.kick(punch=0.9, dec=0.28, f0=120, f1=44)


def tick():
    """woody clock-tick for off-beats (kept out of the pluck band)"""
    n = int(0.06 * SR)
    t = tt(n)
    return A.hp(np.random.default_rng(9).normal(0, 1, n), 6500) * np.exp(-t / 0.012) * 0.6


def sub_thump(dur=0.9):
    n = int(dur * SR)
    t = tt(n)
    f = 60 + 40 * np.exp(-t / 0.06)
    return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.3)


def shimmer(m, dur=2.5):
    n = int(dur * SR)
    t = tt(n)
    f = A.mtof(m)
    x = sum(np.sin(2 * np.pi * f * r * t + r) * a for r, a in ((1, 1), (2.01, 0.4), (3.0, 0.2)))
    return x * A.expenv(n, dur * 0.35, attack=0.02)


# ---------------------------------------------------------------- the score
def plucks(mix):
    """every pluck from the physics; simultaneous plucks are scaled so chords don't spike"""
    times = np.array([e[0] for e in T.EVENTS])
    for tp, k in T.EVENTS:
        dens = int(((times > tp - 0.025) & (times < tp + 0.025)).sum())
        g = 0.34 / math.sqrt(dens)
        vel = 0.92 + 0.08 * math.sin(tp * 7.3 + k)
        mix.place('pluck', lantern_pluck(T.NOTES[k]), tp, g * vel, pan=-0.55 + 1.1 * k / (T.K - 1))


def music(mix):
    B = T.BEAT
    # pad + bass from 35 s: chords change with each pattern
    for i, (b, name, pad, root) in enumerate(T.CHORDS):
        b_next = T.CHORDS[i + 1][0] if i + 1 < len(T.CHORDS) else 58
        t0, dur = T.beat(b), (b_next - b) * B
        att = 1.2 if i == 0 else 0.25
        mix.place('music', A.pad(pad, dur + 0.4, cut=1100, att=att), t0, 0.9 if i else 0.7)
        sub = 1 if b < 48 else 2                                  # eighths in the build
        nb = int(round((b_next - b) * sub))
        for j in range(nb):
            tb = t0 + j * B / sub
            m = root if (j % 4) != 2 else root + 7
            if b >= 52 and j % 2:
                m = root + 12
            mix.place('bass', bass_note(m, B / sub * 0.85), tb, 0.42 if j % sub == 0 else 0.3)
    # groove: soft kick on every beat (= the middle pendulum's pluck) from the C chord before 40 s
    for b in range(38, 58):
        tb = T.beat(b)
        mix.place('drums', soft_kick(), tb, 0.42 if b % 2 == 0 else 0.3)
        if b >= 43:
            mix.place('drums', tick(), tb + B / 2, 0.14)
        if b >= 48 and b % 2 == 1:
            mix.place('drums', A.snare(0.12), tb, 0.16)
    # build: snare roll 8ths -> 16ths -> 32nds into the climax strum, plus a riser
    t_c = T.T_CLIMAX
    for b in range(52, 57):
        div = 2 if b < 54 else (4 if b < 56 else 8)
        for j in range(div):
            tb = T.beat(b) + j * B / div
            mix.place('drums', A.snare(0.08), tb, 0.05 + 0.1 * (tb - T.beat(52)) / (t_c - T.beat(52)))
    mix.place('fx', A.whoosh(4.6, 400, 6000), t_c - 4.6, 0.16)
    mix.place('drums', A.crash(3.0), t_c, 0.32)
    mix.place('drums', soft_kick(), t_c, 0.6)
    mix.place('fx', sub_thump(1.2), t_c, 0.5)
    mix.place('music', A.pad([48, 55, 60, 64, 67], 1.1, cut=1200, att=0.05), t_c, 0.5)   # brief C colour on the strum
    # into the loop: a very soft air swell; the unison chord at 0.0 s is frame 0
    mix.place('fx', A.whoosh(0.8, 800, 5000), 59.2, 0.05)


def accents(mix):
    """hook and the group milestones"""
    mix.place('fx', sub_thump(1.2), 0.0, 0.45)
    mix.place('fx', shimmer(84, 3.0), 0.0, 0.07, pan=0.2)
    for tm, g in T.GROUPS:
        mix.place('fx', A.whoosh(0.9, 500, 4500, seed=int(tm)), tm - 0.9, 0.05)
        if tm < T.T_BAND:
            mix.place('fx', sub_thump(0.8), tm, 0.22)


def ambience(mix):
    n = int(T.DUR * SR)
    room = A.lp(np.random.default_rng(1).normal(0, 1, n), 380) * 0.05
    mix.place('amb', room, 0.0, 0.5)


def main():
    mix = A.Mix(T.DUR)
    plucks(mix)
    music(mix)
    accents(mix)
    ambience(mix)
    gains = {'pluck': 1.0, 'music': 1.0, 'bass': 0.9, 'drums': 1.4, 'fx': 1.0, 'amb': 1.0}
    # carve the music bus under the plucks (300-3000 Hz) so the science sound stays on top
    for k in ('music', 'drums', 'fx'):
        b = mix.buses[k]
        mid = A.bp(b, 300, 3000)
        mix.buses[k] = b - 0.35 * mid
    out = mix.master(gains=gains, reverb_buses={'pluck': 0.55, 'music': 1.0, 'fx': 0.4},
                     gain_db_pts=[(0, 0), (34, 0), (36, 1.0), (52, 1.5), (59, 2.0), (60, 1.0)])
    A.write('audio.wav', out)
    for a, b, rms, pk in A.section_levels('audio.wav', [(0, 4), (4, 22), (22, 35), (35, 50), (50, 59), (59, 60)]):
        print(f'{a:4.0f}-{b:4.0f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')
    # band check on the dry stems (same bus gains): plucks alone vs everything else
    from scipy.signal import butter, sosfilt
    sos = butter(4, [300, 3000], 'bp', fs=SR, output='sos')

    def stem(keep):
        return sum(mix.buses[k][:mix.NS] * gains.get(k, 1.0) for k in keep if k in mix.buses).mean(1)
    yp = sosfilt(sos, stem(['pluck']))
    yr = sosfilt(sos, stem(['music', 'bass', 'drums', 'fx', 'amb']))
    lvl = lambda y, a, b: 20 * np.log10(np.sqrt((y[int(a * SR):int(b * SR)] ** 2).mean()) + 1e-12)
    print('pluck band 300-3000 Hz, plucks vs the rest of the mix:')
    for a, b in [(4, 22), (22, 35), (35, 45), (45, 52), (52, 58.5)]:
        print(f'  {a:4.0f}-{b:4.0f}s  margin {lvl(yp, a, b) - lvl(yr, a, b):5.1f} dB')


if __name__ == '__main__':
    main()
