"""The Perfect Loop v2 (100 pendulums) - soundtrack. Everything is synthesised and original.

    python audio.py        -> audio.wav, prints section levels and the pluck-band check

The science sound is the plucks: every lantern plucks its own note (4 octaves of C major pentatonic,
five neighbouring pendulums per note, longest = lowest) each time it is back at its release side, at
exactly the trigger times in timeline.py: 10 050 plucks, ~167 per second, a glassy wash. Plucks are
short and individually quiet, so whenever pendulums align their plucks land together and the wash
punches through by itself: the 2-team moment is a 50-voice chord, the line-up at 52 s a 100-voice strum.
In the last seconds before 52 s every pendulum's last few plucks fall in rolled sweeps from low to high
that tighten into the strum: the physics writes its own riser.

The arrangement locks to the physics: 120 BPM = pendulum n = 120 (one pluck per beat), every group
milestone is on that grid, and the hi-hat subdivision equals the current group count (6 groups =
sextuplets, 5 = quintuplets, 4 = sixteenths, 3 = triplets, 2 = eighths, 8 = 32nds). Bass + pad enter
at 27 s, the chords change with each pattern, and the level builds ~8 dB into the 52 s drop."""
import math
import os
import sys
import numpy as np

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
from reelkit import audio as A
import timeline as T

SR = A.SR
_cache = {}


def tt(n):
    return np.arange(n) / SR


# ---------------------------------------------------------------- instruments
def lantern_pluck(k):
    """glassy mallet pluck for pendulum k: a few partials (one slightly inharmonic) + a tiny strike
    click. Very short, because 100 voices make ~167 plucks per second."""
    if k in _cache:
        return _cache[k]
    m = T.NOTES[k] + 0.06 * math.sin(k * 2.3)              # +-6 cents spread between the five voices of a note
    f = A.mtof(m)
    dec = 0.2 * (261.6 / f) ** 0.35
    n = int(min(0.8, dec * 4.5) * SR)
    t = tt(n)
    out = np.zeros(n)
    for ratio, amp, dk in ((1.0, 1.0, 1.0), (2.0, 0.42, 0.45), (3.0, 0.18, 0.3), (4.2, 0.12, 0.18)):
        if f * ratio < 9000:
            out += amp * np.sin(2 * np.pi * f * ratio * t + k) * np.exp(-t / (dec * dk))
    click = A.hp(np.random.default_rng(k).normal(0, 1, n), 2500) * np.exp(-t / 0.0015) * 0.1
    out = (out + click) * np.clip(t / 0.0012, 0, 1)
    out = A.lp(out, 8000)
    _cache[k] = out
    return out


def bass_note(m, dur, vel=1.0, cut=None):
    n = int((dur + 0.08) * SR)
    t = tt(n)
    f = A.mtof(m)
    x = np.sin(2 * np.pi * f * t) + 0.35 * np.sin(4 * np.pi * f * t) + 0.12 * np.sin(6 * np.pi * f * t)
    x = np.tanh(1.4 * x) * A.adsr(n, 0.006, 0.15, 0.7, 0.07)
    if cut:
        x = A.lp(x, cut)
    return x * vel


def kick(cut=None):
    x = A.kick(punch=1.0, dec=0.26, f0=130, f1=45)
    return A.lp(x, cut) if cut else x


def sub_thump(dur=0.9):
    n = int(dur * SR)
    t = tt(n)
    f = 55 + 45 * np.exp(-t / 0.05)
    return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.32)


def hat(acc=False):
    return A.hat(0.022 if not acc else 0.035) * (1.0 if acc else 0.62)


def clap():
    return A.hp(A.clap(), 2800)


def bell(m, dur=0.5):
    """call-and-answer bell, kept above the pluck band"""
    n = int(dur * SR)
    t = tt(n)
    f = A.mtof(m)
    x = np.sin(2 * np.pi * f * t) + 0.4 * np.sin(2 * np.pi * f * 2.76 * t) * np.exp(-t / 0.05)
    return A.hp(x * A.expenv(n, 0.12, attack=0.002), 2500)


# ---------------------------------------------------------------- the score
def place_loop(mix, bus, sig, t, gain=1.0, pan=0.0):
    """place a sound; whatever rings past 60 s wraps to the start, so the loop seam is seamless"""
    mix.place(bus, sig, t, gain, pan)
    if t + len(sig) / SR > T.DUR:
        mix.place(bus, sig, t - T.DUR, gain, pan)


def plucks(mix):
    """every pluck from the physics. Clusters are only gently tamed (dens^-0.3), so alignment punches."""
    times = np.array([e[0] for e in T.EVENTS])
    for tp, k in T.EVENTS:
        dens = int(np.searchsorted(times, tp + 0.012) - np.searchsorted(times, tp - 0.012))
        g = 0.105 / dens ** 0.3
        vel = 0.9 + 0.1 * math.sin(tp * 7.3 + k)
        place_loop(mix, 'pluck', lantern_pluck(k), tp, g * vel, pan=-0.55 + 1.1 * k / (T.K - 1))


def music(mix):
    B = T.BEAT
    # ---- hook + milestone hits before the band: kick, sub and a bass hit on the chord root
    for tm in [0.0, 2.0, 4.0, 7.0, 12.0, 22.0]:
        root = T.chord_at(tm)[3]
        mix.place('drums', kick(), tm, 0.55 if tm == 0 else 0.42)
        mix.place('fx', sub_thump(1.0), tm, 0.5 if tm == 0 else 0.34)
        mix.place('bass', bass_note(root, 1.4, cut=900), tm, 0.5 if tm == 0 else 0.36)
    mix.place('drums', A.crash(2.0), 0.0, 0.12)
    # ---- 0-7: kick on each bar downbeat (the hits above cover 0 / 2 / 4)
    mix.place('drums', kick(), 6.0, 0.36)
    # ---- 7-22: filtered groove under the wash (4-on-the-floor + root bass, low-passed)
    for b in range(14, 44):
        tb = b * B
        if tb in (7.0, 12.0):
            continue
        cut = 260 if tb < 17 else 170
        mix.place('drums', kick(cut), tb, 0.5 if tb < 17 else 0.44)
    for i, (t0, name, pad, root) in enumerate(T.CHORDS):
        if not (7.0 <= t0 < 22.0):
            continue
        t1 = T.CHORDS[i + 1][0]
        nb = int(round((t1 - t0) / B))
        for j in range(nb):
            mix.place('bass', bass_note(root if j % 4 != 2 else root + 7, B * 0.8, cut=320), t0 + j * B, 0.34)
    # ---- 17-22 chaos: a slow noise swell keeps it moving
    mix.place('fx', A.whoosh(4.8, 300, 3500, seed=17), 17.2, 0.06)
    # ---- 22-27 halftime: kick on 1, clap on 3; call and answer in eighths (left / right)
    for bar in range(11, 14):
        t0 = bar * T.BAR
        if t0 != 22.0:
            mix.place('drums', kick(), t0, 0.46)
        mix.place('drums', clap(), t0 + 2 * B, 0.3)
    call = [81, 84, 81, 88]
    for j in range(20):                                  # 22.0 .. 27.0 in eighths
        tb = 22.0 + j * B / 2
        side = -0.6 if j % 2 == 0 else 0.6
        m = call[(j // 2) % 4] + (0 if j % 2 == 0 else -5)
        mix.place('hats', bell(m), tb, 0.12, pan=side)
    # ---- 27-60: bass + pad, chords change with each pattern
    band = [c for c in T.CHORDS if c[0] >= T.T_BAND]
    for i, (t0, name, pad, root) in enumerate(band):
        t1 = band[i + 1][0] if i + 1 < len(band) else 60.0
        dur = t1 - t0
        att = 1.0 if t0 == T.T_BAND else 0.08
        place_loop(mix, 'music', A.pad([m - 12 if m > 57 else m for m in pad], dur + 0.35, cut=700 if t0 < 52 else 800, att=att), t0, 1.0)
        sub = 1 if t0 < 37 else 2                         # eighth-note bass from 37 s
        nb = int(round(dur / B * sub))
        for j in range(nb):
            tb = t0 + j * B / sub
            if 50.0 <= tb < 52.0:                         # the bass drops out for the last bar of the build
                continue
            m = root if (j % 4) != 2 else root + 7
            if t0 >= 52 and j % 2:
                m = root + 12
            mix.place('bass', bass_note(m, B / sub * 0.85), tb, 0.42 if j % sub == 0 else 0.3)
    # ---- kick: 4-on-the-floor from 27, eighths 44.5-50, out for 50-52, back on the drop
    for b in range(54, 120):
        tb = b * B
        if 50.0 <= tb < 52.0:
            continue
        mix.place('drums', kick(), tb, 0.55)
        if 44.5 <= tb < 50.0:
            mix.place('drums', kick(), tb + B / 2, 0.38)
    # ---- clap on 2 and 4 from 32 s
    for b in range(64, 120):
        tb = b * B
        if b % 2 == 1 and not (50.0 <= tb < 52.0):
            mix.place('drums', clap(), tb, 0.3)
    # ---- hats: the subdivision is the current group count
    for b in range(0, 120):
        tb = b * B
        s = T.subdiv_at(tb + 1e-6)
        if s == 0 or (22.0 <= tb < 27.0):
            continue
        g0 = 0.05 if tb < 7 else (0.065 if tb < 27 else 0.08)
        if tb >= 44.5:
            g0 = 0.09
        for j in range(s):
            mix.place('hats', hat(j == 0), tb + j * B / s, g0 * (1.25 if j == 0 else 1.0), pan=0.25 * math.sin(j * 1.7 + b))
    # ---- build into the line-up: snare roll 8ths -> 16ths -> 32nds and a riser
    tb, tl = 48.0, T.T_LINE
    while tb < tl - 1e-6:
        div = 2 if tb < 49.0 else (4 if tb < 50.5 else 8)
        mix.place('drums', A.hp(A.snare(0.07), 1800), tb, 0.06 + 0.16 * (tb - 48.0) / (tl - 48.0))
        tb += B / div
    mix.place('fx', A.whoosh(4.0, 300, 7000, seed=48), tl - 4.0, 0.16)
    # ---- the drop at 52 s: crash, kick, sub (the 100-voice strum is in the pluck bus)
    mix.place('drums', A.crash(3.0), tl, 0.36)
    mix.place('fx', sub_thump(1.4), tl, 0.62)
    mix.place('drums', kick(), tl, 0.72)
    # ---- 58-60: a soft reverse swell back into the opening (the bass hit at 0.0 lands the loop)
    mix.place('fx', A.whoosh(1.9, 900, 5000, seed=58), 58.1, 0.07)


def ambience(mix):
    n = int(T.DUR * SR)
    room = A.lp(np.random.default_rng(1).normal(0, 1, n), 380) * 0.04
    mix.place('amb', room, 0.0, 0.5)


GAINS = {'pluck': 1.0, 'music': 0.5, 'bass': 0.9, 'drums': 1.3, 'hats': 1.0, 'fx': 1.0, 'amb': 1.0}
# section automation: the build (+8 dB from the opening to the 52 s strum), a dip back into the loop
GAIN_PTS = [(0, -2.0), (6.8, -2.0), (7.2, -1.5), (21.8, -0.8), (22.0, 0.2), (27.0, 1.2), (32, 2.0), (44, 3.2),
            (51.9, 5.3), (52.0, 6.4), (54.0, 5.2), (57.5, 3.2), (59.4, 0.4), (60, -2.0)]


def limit(x, drive_db=4.5, ceil=0.89, win=0.024):
    """lookahead peak limiter: the in-phase 100-voice strum would otherwise set the normalisation and
    leave the whole mix quiet. A min-filtered, smoothed gain keeps every peak under the ceiling."""
    from scipy.ndimage import minimum_filter1d, uniform_filter1d
    y = x * 10 ** (drive_db / 20)
    need = np.minimum(1.0, ceil / np.maximum(np.abs(y).max(1), 1e-9))
    n = int(win * SR)
    g = uniform_filter1d(minimum_filter1d(need, 2 * n + 1), n | 1)
    y = y * np.minimum(g, need)[:, None]
    return np.clip(y, -ceil, ceil)


def main():
    mix = A.Mix(T.DUR)
    plucks(mix)
    music(mix)
    ambience(mix)
    # carve the music buses under the plucks (300-3000 Hz) so the science sound stays on top
    for k, amt in (('music', 0.88), ('drums', 0.75), ('fx', 0.75), ('bass', 0.7)):
        b = mix.buses[k]
        mix.buses[k] = b - amt * A.bp(b, 300, 3000)
    out = mix.master(gains=GAINS, reverb_buses={'pluck': 0.5, 'music': 1.0, 'fx': 0.4, 'hats': 0.3}, gain_db_pts=GAIN_PTS)
    out = limit(out)
    A.write('audio.wav', out)
    spans = [(a, a + 2) for a in range(0, 60, 2)]
    lv = A.section_levels('audio.wav', spans)
    print('2 s windows (rms dBFS):')
    print('  ' + ' '.join(f'{a:02d}:{rms:5.1f}' for a, b, rms, pk in lv))
    print(f'  opening 0-6 s {np.mean([r for a, b, r, p in lv[:3]]):.1f}  strum 52-54 s {lv[26][2]:.1f}  '
          f'min {min(r for a, b, r, p in lv):.1f}  max {max(r for a, b, r, p in lv):.1f}')
    pk = A.section_levels('audio.wav', [(51.95, 52.25), (21.95, 22.25), (0, 0.3)])
    print('  peak at the strum %.1f, 2 teams %.1f, frame 0 %.1f dBFS' % (pk[0][3], pk[1][3], pk[2][3]))
    # band check on the dry stems (same bus gains): plucks alone vs everything else
    from scipy.signal import butter, sosfilt
    sos = butter(4, [300, 3000], 'bp', fs=SR, output='sos')

    def stem(keep):
        return sum(mix.buses[k][:mix.NS] * GAINS.get(k, 1.0) for k in keep if k in mix.buses).mean(1)
    yp = sosfilt(sos, stem(['pluck']))
    yr = sosfilt(sos, stem(['music', 'bass', 'drums', 'hats', 'fx', 'amb']))
    lvl = lambda y, a, b: 20 * np.log10(np.sqrt((y[int(a * SR):int(b * SR)] ** 2).mean()) + 1e-12)
    print('pluck band 300-3000 Hz, plucks vs the rest of the mix:')
    print('  ' + '  '.join(f'{a}-{b}s {lvl(yp, a, b) - lvl(yr, a, b):5.1f} dB' for a, b in
                         [(0, 7), (7, 17), (17, 22), (22, 27), (27, 32), (32, 42), (42, 48), (48, 52), (52, 58)]))


if __name__ == '__main__':
    main()
