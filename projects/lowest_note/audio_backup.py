"""The Lowest Note in the Universe: soundtrack. Everything is synthesised; the melody is original.

The science IS the instrument: every pressure-wave crest the picture launches (timeline.LAUNCH) is a
sound. From 29 s each crest is a kick (55 -> 109 -> 219 BPM), the kicks blur into a flutter (7.3 ->
14.6 Hz), and at octave -4 the same pulse train runs at 29.1 Hz and is heard as a pitch, B-flat0.
B-flat1..B-flat4 then stack on top. The climax beat is the hole's note 8 octaves down (109.3 BPM).

    python audio.py            -> audio.wav, prints section levels and science-vs-music band levels"""
import math
import os
import sys
import numpy as np

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
from reelkit import audio as A
import timeline as T

SR = A.SR
RNG = np.random.default_rng(7)


# ------------------------------------------------------------------ voices
def crest_kick(dec=0.32, f0=170.0, f1=None, click=1.0):
    """one wave crest: pitch-swept body tuned to B-flat1 + a short click so phones hear it"""
    f1 = f1 or T.freq(-3)                       # 58.27 Hz
    n = int(max(0.12, dec * 3.2) * SR)
    t = A.tt(n)
    f = f1 + (f0 - f1) * np.exp(-t / 0.035)
    body = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / dec)
    knock = np.sin(2 * np.pi * 2.0 * np.cumsum(f) / SR) * np.exp(-t / min(dec, 0.06)) * 0.35
    ck = A.hp(RNG.normal(0, 1, n), 1800) * np.exp(-t / 0.004) * 0.8 * click
    thock = (np.sin(2 * np.pi * T.BB4 * t) + 0.4 * np.sin(2 * np.pi * 2 * T.BB4 * t)) * np.exp(-t / 0.035) * 0.32 * click
    return body + knock + ck + thock


def bl_pulse(f, n, duty, fmax=5000.0, t0=0.0):
    """band-limited pulse wave (additive); t0 = absolute start time keeps the phase continuous"""
    t = A.tt(n) + t0
    out = np.zeros(n)
    nh = max(1, int(fmax / f))
    for h in range(1, nh + 1):
        out += math.sin(math.pi * h * duty) / h * np.cos(2 * np.pi * h * f * t)
    return out / (np.abs(out).max() + 1e-9)


def env_pts(n, pts, t0):
    """piecewise-linear envelope from absolute (time, value) points"""
    tt = A.tt(n) + t0
    return np.interp(tt, [p[0] for p in pts], [p[1] for p in pts])


def lead(m, dur):
    n = int((dur + 0.6) * SR)
    t = A.tt(n)
    f = A.mtof(m) * (1 + 0.004 * np.sin(2 * np.pi * 5.5 * t) * np.clip((t - 0.15) / 0.3, 0, 1))
    ph = 2 * np.pi * np.cumsum(f) / SR
    x = np.sin(ph) + 0.45 * np.sin(2 * ph) * np.exp(-t / 0.25) + 0.18 * np.sin(3 * ph) * np.exp(-t / 0.12)
    e = np.clip(t / 0.006, 0, 1) * np.where(t < dur, np.exp(-t / max(0.6, dur * 0.7)), np.exp(-dur / max(0.6, dur * 0.7)) * np.exp(-(t - dur) / 0.12))
    return x * e


def shimmer(ms, dur, att=1.2):
    n = int(dur * SR)
    t = A.tt(n)
    x = sum(np.sin(2 * np.pi * A.mtof(m) * (1 + 0.002 * k) * t + k) * (0.6 + 0.4 * np.sin(2 * np.pi * (0.7 + 0.23 * k) * t)) for k, m in enumerate(ms))
    return x / len(ms) * A.adsr(n, att, 0.3, 0.85, min(1.5, dur * 0.4))


def tick(f=2400.0, dec=0.006):
    n = int(0.04 * SR)
    t = A.tt(n)
    return (np.sin(2 * np.pi * f * t) * 0.7 + A.hp(RNG.normal(0, 1, n), 3000) * 0.3) * np.exp(-t / dec)


def pop():
    n = int(0.25 * SR)
    t = A.tt(n)
    f = 200 + 700 * np.exp(-t / 0.03)
    return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.07) + A.bp(RNG.normal(0, 1, n), 900, 3500) * np.exp(-t / 0.01) * 0.6


def hum(m, dur):
    """a hummed 'mmm': soft buzz through a closed-mouth formant"""
    n = int(dur * SR)
    t = A.tt(n)
    f = A.mtof(m) * (1 + 0.005 * np.sin(2 * np.pi * 4.8 * t))
    ph = 2 * np.pi * np.cumsum(f) / SR
    x = sum(np.sin(h * ph) / h ** 1.3 for h in range(1, 14))
    return A.lp(A.bp(x, 150, 2200), 900) * 1.6


# ------------------------------------------------------------------ the score
def main():
    mix = A.Mix(T.DUR)
    n_all = int(T.DUR * SR)

    # air: a faint, steady space hiss (identical at both ends so the loop is seamless)
    air = A.lp(RNG.normal(0, 1, n_all), 520) * 0.5 + A.bp(RNG.normal(0, 1, n_all), 2500, 7000) * 0.05
    mix.place('amb', np.stack([air, np.roll(air, 4800)], 1), 0.0, 0.11)

    # 4.6 s jets fire + soft swell, cut to near-silence on "SPACE IS SILENT...", shimmer on "EXCEPT HERE"
    mix.place('fx', A.whoosh(1.3, 150, 2600), T.T_JETS - 0.9, 0.22)
    mix.place('fx', crest_kick(0.6, 110, 40, click=0.2), T.T_JETS, 0.22)
    sw = A.pad([46, 53, 58, 62, 65], 3.6, cut=1100, att=2.2)
    sw *= np.clip((7.95 - (T.T_JETS + A.tt(len(sw)))) / 0.25, 0, 1)
    mix.place('music', sw, T.T_JETS, 0.16)
    mix.place('music', shimmer([82, 86, 89, 94], 4.2, att=1.0), 9.2, 0.07)
    # 11-12.5 the bubble stretches, 12.5 it pops: the first piston wave
    n = int(1.5 * SR)
    tt = A.tt(n)
    stretch = np.sin(2 * np.pi * np.cumsum(80 + 120 * (tt / 1.5) ** 2 + 6 * np.sin(2 * np.pi * 7 * tt)) / SR) * (tt / 1.5) ** 1.5
    mix.place('sci', stretch, T.T_POP - 1.5, 0.08)
    mix.place('sci', crest_kick(0.9, 130, 38, click=0.6), T.T_POP, 0.75)
    mix.place('fx', pop(), T.T_POP, 0.3)
    # 14 s HUD slides in
    mix.place('fx', A.whoosh(0.45, 600, 5000), T.T_PANEL_IN - 0.15, 0.1)
    mix.place('fx', A.chime(82, 0.8), T.T_PANEL_IN + 0.3, 0.06)

    # 17-29 s the counter climbs: ticks per octave, chimes at the milestones, a riser, accelerating thumps
    for st, k in T.STEPS:
        if 16.5 < st <= T.T9:
            mix.place('fx', tick(1300 * 2 ** ((k + 57) / 40)), st, 0.05)
        if T.T_REWIND0 <= st <= T.T_REWIND1:
            mix.place('fx', tick(1300 * 2 ** ((k + 57) / 40), 0.004), st, 0.03)
    for tm, m in ((18.7, 82), (21.0, 84), (23.6, 86), (26.8, 89)):
        mix.place('fx', A.chime(m, 1.4), tm, 0.06)
    riser = A.whoosh(12.0, 250, 6000) * np.linspace(0.2, 1, int(12 * SR)) ** 2
    mix.place('fx', riser, 17.0, 0.16)
    for k, ta in enumerate(T.ACCEL_THUMPS):
        if ta > T.T_POP:
            mix.place('sci', crest_kick(0.45, 140, 45, click=0.4), ta, 0.22 + 0.05 * k)
    for bar, (root, ch) in enumerate(T.SWELL_CHORDS * 2):
        tb = 17.0 + bar * 3.0
        if tb < T.T9:
            mix.place('music', A.pad(ch, 3.2, cut=900, att=0.9), tb, 0.06 + 0.012 * bar)

    # 29-38 s every crest is a kick; they double, then blur into a flutter
    for tl, a, g in T.LAUNCH:
        if T.T9 - 0.01 <= tl < T.T4 - 1e-6:
            k = T.octave(tl + 1e-4)
            dec = {-9: 0.38, -8: 0.28, -7: 0.17, -6: 0.085, -5: 0.045}[k]
            gain = {-9: 0.95, -8: 0.85, -7: 0.75, -6: 0.62, -5: 0.55}[k]
            mix.place('sci', crest_kick(dec, 170 if k >= -7 else 140, click=1.0 if k >= -7 else 0.6), tl, gain)
    for bar, (root, ch) in enumerate(T.SWELL_CHORDS[:3]):
        tb = T.T9 + bar * 3.0
        mix.place('music', A.pad(ch, 3.2, cut=1000, att=0.3), tb, 0.08)
    mix.place('fx', A.whoosh(3.0, 400, 7000), T.T6, 0.12)

    # 38-60 s the pulse train is a pitch: B-flat0, then the octaves stack (B-flat1..B-flat4)
    t_start = T.T4
    t_end = T.DUR
    n = int((t_end - t_start) * SR)
    duck = np.ones(n)
    tt = A.tt(n) + t_start
    for j in range(17):
        tb = T.T_CLIMAX + j * T.BEAT
        ph = tt - tb
        duck *= 1 - 0.32 * np.where(ph >= 0, np.exp(-np.clip(ph, 0, None) / 0.12), 0.0)
    layers = [  # (octave k, onset, duty, fmax, gain, fade-out points)
        (-4, T.T4, 0.12, 3500, 0.42, [(T.T_LAND + 0.5, 1), (55.5, 0)]),
        (-3, T.T3, 0.25, 4000, 0.30, [(T.T_LAND + 0.5, 1), (55.8, 0)]),
        (-2, T.T2, 0.35, 4500, 0.26, [(T.T_LAND + 0.5, 1), (56.0, 0)]),
        (-1, T.T1, 0.42, 5000, 0.22, [(T.T_LAND + 0.5, 1), (56.3, 0)]),
        (0, T.T0, 0.48, 6000, 0.16, [(T.T_LAND + 0.2, 1), (55.0, 0)]),
    ]
    for k, on, duty, fmax, gain, outro in layers:
        f = T.freq(k)
        m = int((t_end - on) * SR)
        x = bl_pulse(f, m, duty, fmax, t0=on)
        e = env_pts(m, [(on, 0.0), (on + (0.03 if k == -4 else 0.35), 1.0)] + outro, on)
        i0 = int((on - t_start) * SR)
        mix.place('sci', x * e * duck[i0:i0 + m], on, gain)
        if k > -4:
            mix.place('fx', A.chime(70 + 12 * k + 12, 1.2), on, 0.04)
    mix.place('music', A.pad([58, 62, 65, 70], T.T0 - T.T4 + 0.5, cut=900, att=1.5), T.T4, 0.06)

    # 44-52.8 s climax: kick on every crest, the original melody over the B-flat stack
    for j in range(17):
        tb = T.T_CLIMAX + j * T.BEAT
        mix.place('sci', crest_kick(0.3 if j < 16 else 0.6, 175), tb, 0.8 if j < 16 else 1.0)
        if j < 16:
            if j % 2 == 1:
                mix.place('drums', A.clap(), tb, 0.22)
            mix.place('drums', A.hat(), tb + T.BEAT / 2, 0.08)
            mix.place('drums', A.hat(0.02), tb, 0.04)
    mix.place('drums', A.crash(2.5), T.T_CLIMAX, 0.12)
    mix.place('drums', A.crash(3.5), T.T_LAND, 0.14)
    for bar, (root, ch) in enumerate(T.CLIMAX_CHORDS):
        tb = T.T_CLIMAX + bar * 4 * T.BEAT
        mix.place('music', A.pad(ch, 4 * T.BEAT + 0.1, cut=1300, att=0.05), tb, 0.08)
    for b, m, d in T.MELODY:
        tb = T.T_CLIMAX + b * T.BEAT
        dur = d * T.BEAT
        mix.place('music', lead(m, dur * 0.95), tb, 0.19, pan=-0.08)
        mix.place('music', lead(m + 12, dur * 0.9), tb, 0.04, pan=0.15)

    # 52.8-60 s decays to one hum; the counter rewinds; the loop ring; back to the air
    hm = hum(58, 6.8)
    hm *= env_pts(len(hm), [(T.T_LAND, 0), (T.T_LAND + 0.9, 1), (56.5, 0.9), (59.3, 0)], T.T_LAND)
    mix.place('sci', hm, T.T_LAND, 0.22)
    mix.place('fx', A.whoosh(3.6, 4000, 200, rise=False), T.T_REWIND0, 0.08)
    mix.place('sci', crest_kick(0.7, 110, 40, click=0.15), T.T_LOOP_RING, 0.16)

    out = mix.master(reverb_buses={'music': 0.9, 'fx': 0.6, 'sci': 0.18, 'drums': 0.25},
                     gains={'sci': 1.0, 'music': 1.0},
                     gain_db_pts=[(0, -3), (8, -3), (12, 0), (60, 0)])
    A.write('audio.wav', out)

    # ---------------- checks
    secs = [(0, 4), (4, 8), (8, 11), (11, 14), (14, 17), (17, 29), (29, 35), (35, 38), (38, 44), (44, 52.8), (52.8, 60)]
    print('section levels (dBFS):')
    for a, b, rms, pk in A.section_levels('audio.wav', secs):
        print(f'  {a:5.1f}-{b:5.1f}s  rms {rms:6.1f}  peak {pk:5.1f}')
    # science vs music, per band, from the stems before mastering
    stems = {k: v[:n_all].mean(1) for k, v in mix.buses.items()}
    from scipy.signal import butter, sosfilt

    def lvl(x, lo, hi, a, b):
        y = sosfilt(butter(4, [lo, hi], 'bp', fs=SR, output='sos'), x)
        seg = y[int(a * SR):int(b * SR)]
        return 20 * np.log10(np.sqrt((seg ** 2).mean()) + 1e-12)
    music = stems['music'] + stems.get('drums', 0)
    print('science vs music+drums (stems, dB):')
    for a, b, lo, hi, name in ((29, 35, 40, 160, 'kick body'), (29, 35, 1500, 5000, 'kick click'), (38, 44, 25, 600, 'B-flat stack'),
                               (44, 52.8, 25, 600, 'stack in climax'), (44, 52.8, 600, 3000, 'mids in climax')):
        s, mu = lvl(stems['sci'], lo, hi, a, b), lvl(music, lo, hi, a, b)
        print(f'  {name:16s} {lo:5d}-{hi:<5d}Hz  science {s:6.1f}  music {mu:6.1f}  -> {s - mu:+5.1f} dB')


if __name__ == '__main__':
    main()
