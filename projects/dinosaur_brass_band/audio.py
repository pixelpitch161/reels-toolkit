"""Dinosaur Brass Band soundtrack. Everything synthesised; the chorale is original.

The science sound IS the band: every honk is a closed-tube resonator voice (odd harmonics strongest,
a nasal crumhorn-like colour) whose pitch comes from that animal's crest air length (timeline.VOICES:
longer tube = lower note). Underneath, quietly: timpani on the chord roots, a soft pad, footsteps,
scanner ticks, test tones, bottle blows and a floodplain ambience (wind, river, insects).

    python audio.py        -> audio.wav, prints section levels and the honk-vs-music band check"""
import math
import os
import sys
import numpy as np

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
from reelkit import audio as A
import timeline as T

SR = A.SR
rng = np.random.default_rng(8)


def ss_arr(a, b, x):
    u = np.clip((x - a) / (b - a), 0, 1)
    return u * u * (3 - 2 * u)


# ------------------------------------------------------------------ the crest voice
def honk(midi, dur, style='call', scale=1.0, seed=0):
    """closed-tube resonator honk. style: call | rumble | hold | song | tone | hum"""
    f0 = A.mtof(midi)
    rel = 0.22 if style != 'hum' else 0.6
    n = int((dur + rel) * SR)
    t = np.arange(n) / SR
    att = {'call': 0.09, 'rumble': 0.14, 'hold': 0.12, 'song': 0.06, 'tone': 0.03, 'hum': 0.5}[style]
    if f0 < 60:
        att *= 1.4
    # pitch: a lip-buzz scoop up into the note, gentle vibrato, slight droop at the end of calls
    cents = -110 * np.exp(-t / 0.05) if style not in ('tone', 'hum') else np.zeros(n)
    vib_d = {'call': 9, 'rumble': 28, 'hold': 10, 'song': 7, 'tone': 3, 'hum': 4}[style]
    vib = vib_d * np.sin(2 * np.pi * (5.1 + 0.4 * (seed % 3)) * t) * ss_arr(0.15, 0.5, t)
    droop = -45 * ss_arr(dur - 0.25, dur + rel, t) if style in ('call', 'rumble') else 0
    f = f0 * 2 ** ((cents + vib + droop) / 1200)
    ph = 2 * np.pi * np.cumsum(f) / SR
    # brightness envelope (brass "blat" on the attack), amplitude envelope
    env = np.clip(t / att, 0, 1) ** 1.5
    env *= np.where(t > dur, np.clip(1 - (t - dur) / rel, 0, 1) ** 1.6, 1.0)
    if style in ('call', 'rumble'):
        env *= 1 - 0.18 * ss_arr(0.3, dur, t)
    bright = np.clip(t / (att * 1.6), 0, 1) * (0.75 + 0.25 * np.exp(-t / 0.4))
    if style in ('tone', 'hum'):
        bright *= 0.45
    p = 2.5 - 1.35 * bright                                  # spectral tilt exponent over time
    K = int(min(80, 5200 / f0))
    out = np.zeros(n)
    F1, F2 = 520 + 260 * (1 - scale), 1450 + 500 * (1 - scale)      # fixed resonances of the passages
    for k in range(1, K + 1):
        fk = k * f0
        odd = 1.0 if k % 2 else 0.42                         # closed tube: odd harmonics dominate
        form = 1 + 1.3 * math.exp(-((fk - F1) / (F1 * 0.55)) ** 2) + 0.7 * math.exp(-((fk - F2) / (F2 * 0.4)) ** 2)
        lowcut = min(1.0, (fk / 90.0) ** 1.2)                # keep sub-bass in check
        out += odd * form * lowcut * k ** (-p) * np.sin(k * ph + 0.3 * k)
    if style == 'rumble' or (f0 < 60 and style == 'call'):
        out *= 1 + 0.22 * np.sin(2 * np.pi * 27 * t) * ss_arr(0.2, 0.6, t)       # throaty rumble
    breath = A.bp(rng.normal(0, 1, n), 700, 3000) * (0.05 + 0.12 * np.exp(-t / 0.08))
    out = out / (np.abs(out).max() + 1e-9) + breath * (0.6 if style != 'tone' else 0.25)
    return out * env


def bottle(midi, dur):
    """blow across a bottle: a breathy near-sine with a noisy edge"""
    f = A.mtof(midi)
    n = int((dur + 0.3) * SR)
    t = np.arange(n) / SR
    env = np.clip(t / 0.12, 0, 1) * np.where(t > dur, np.clip(1 - (t - dur) / 0.3, 0, 1), 1.0)
    fl = f * (1 + 0.004 * np.sin(2 * np.pi * 4.5 * t))
    ph = 2 * np.pi * np.cumsum(fl) / SR
    tone = np.sin(ph) + 0.12 * np.sin(2 * ph) + 0.05 * np.sin(3 * ph)
    air = A.bp(rng.normal(0, 1, n), f * 0.9, min(f * 1.15, 20000)) * 3.5 + A.bp(rng.normal(0, 1, n), 1500, 6000) * 0.25
    amp = env * (0.85 + 0.15 * np.sin(2 * np.pi * 6 * t))
    return (tone * 0.8 + air * 0.35) * amp


def timp(midi, dec=0.9, gain=1.0):
    n = int(1.6 * SR)
    t = np.arange(n) / SR
    f = A.mtof(midi) * (1 + 0.06 * np.exp(-t / 0.03))
    ph = 2 * np.pi * np.cumsum(f) / SR
    body = np.sin(ph) + 0.5 * np.sin(1.5 * ph) * np.exp(-t / 0.25) + 0.3 * np.sin(2 * ph) * np.exp(-t / 0.4)
    hit = A.lp(rng.normal(0, 1, n), 1200) * np.exp(-t / 0.015)
    return (body * np.exp(-t / dec) + hit * 0.4) * gain


def thud(scale):
    n = int(0.35 * SR)
    t = np.arange(n) / SR
    f = 50 + 40 * np.exp(-t / 0.03)
    s = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.09)
    grit = A.bp(rng.normal(0, 1, n), 200, 1200) * np.exp(-t / 0.03) * 0.25
    return (s + grit) * scale


def tick(f=2600):
    n = int(0.03 * SR)
    t = np.arange(n) / SR
    return np.sin(2 * np.pi * f * t) * np.exp(-t / 0.008)


# ------------------------------------------------------------------ mix
def main():
    mix = A.Mix(T.DUR)
    N = int(T.DUR * SR)
    tt = np.arange(N) / SR
    # ambience: wind, river, insects, distant volcano. Quiet, continuous (loops cleanly).
    wind = A.lp(rng.normal(0, 1, N), 420) * (0.7 + 0.3 * np.sin(2 * np.pi * tt / 13.0))
    river = A.bp(rng.normal(0, 1, N), 350, 1400) * (0.5 + 0.5 * np.abs(np.sin(2 * np.pi * tt * 0.7) * np.sin(2 * np.pi * tt * 1.9)))
    ins = A.bp(rng.normal(0, 1, N), 4200, 6000) * (0.5 + 0.5 * np.sin(2 * np.pi * 23 * tt) ** 8)
    volc = A.lp(rng.normal(0, 1, N), 60, 4)
    mix.place('amb', wind * 0.16 + river * 0.06 + ins * 0.012 + volc * 0.5, 0.0)

    # every honk = the crest voice at its own pitch
    for (t0, dur, k, m, st) in T.HONKS:
        v = T.VOICES[k]
        style = 'song' if st == 'song' else st
        g = {0: 0.62, 1: 0.5, 2: 0.42, 3: 0.4, 4: 0.36}[k]
        if st == 'song':
            g *= 0.85
        if st == 'hold':
            g *= 0.8
        pan = (-0.35 + 0.18 * k) if t0 > 29 else 0.0
        mix.place('honk', honk(m, dur, style, v['scale'], seed=int(t0 * 10)), t0, g, pan)
    # 4-8 s: the hollow crest resonates (a soft hum under the X-ray reveal)
    mix.place('honk', honk(31, 3.4, 'hum', 1.0), 4.5, 0.22)
    # test tones: long tube (low) then half the length (one octave up)
    for t0, dur, kind, m in T.TEST_TONES:
        mix.place('honk', honk(m, dur, 'tone', 0.7), t0, 0.32, -0.15 if kind == 'long' else 0.15)
    # bottle blows: the "try it"
    for t0, dur, m, lvl in T.BOTTLE_BLOWS:
        mix.place('fx', bottle(m, dur), t0, 0.32)

    # scanner: ticks + a whirring sweep
    for i, ts in enumerate(T.SCAN_TICKS):
        mix.place('fx', tick(2400 + 300 * (i % 3 == 0)), ts, 0.16, 0.25 * math.sin(i))
    n = int(3.8 * SR)
    ts_ = np.arange(n) / SR
    whirr = A.bp(rng.normal(0, 1, n), 900, 2600) * (0.5 + 0.5 * np.sin(2 * np.pi * 7 * ts_)) * np.sin(np.pi * ts_ / 3.8) ** 2
    mix.place('fx', whirr, T.T_CT0, 0.05)

    # footsteps from the walk cycles (same function the picture uses)
    for k, v in enumerate(T.VOICES):
        prev = None
        for t in np.arange(0, T.DUR, 0.005):
            _, _, _, off, spd = T.walk(k, t)
            if spd <= 0.05:
                prev = None
                continue
            ph = off / (1.7 * v['scale'])
            step = math.floor(ph)
            if prev is not None and step != prev:
                mix.place('fx', thud(v['scale']), t, 0.22 * min(1.0, spd * 1.5), -0.3 + 0.15 * k)
            prev = step

    # transitions
    mix.place('fx', A.whoosh(0.9, 200, 3000), 3.3, 0.12)
    mix.place('fx', A.whoosh(0.7, 400, 6000), 7.6, 0.08)
    mix.place('fx', A.whoosh(0.8, 300, 4000, rise=False), 16.2, 0.07)
    mix.place('fx', A.whoosh(2.6, 150, 5000), 27.9, 0.12)
    mix.place('fx', A.whoosh(1.2, 300, 5000), 45.4, 0.12)
    mix.place('fx', A.crash(3.0), T.T_CLIMAX, 0.16)
    mix.place('fx', A.chime(91, 1.6), T.T_CLIMAX, 0.05)

    # music bed (quiet, under the honks): pad swell for the stack, timpani + pad in the chorale
    mix.place('music', A.pad([55, 62, 67, 71], 4.2, cut=1100, att=1.5), T.T_STACK, 0.07)
    roots = {'G': 43, 'C': 48, 'D': 50, 'Em': 40, 'Am': 45}
    tones = {'G': [55, 59, 62], 'C': [55, 60, 64], 'D': [54, 57, 62], 'Em': [55, 59, 64], 'Am': [57, 60, 64]}
    chords = T.CHORDS + [(24, None)]
    for (b, c), (b2, _) in zip(chords, chords[1:]):
        t0 = T.T_SONG + b * T.BEAT
        ln = (b2 - b) * T.BEAT
        mix.place('music', A.pad(tones[c], ln + 0.3, cut=900 if b < 16 else 1400, att=0.25), t0, 0.05 if b < 16 else 0.075)
    for beat in range(24):
        tb = T.T_SONG + beat * T.BEAT
        c = [cc for bb, cc in T.CHORDS if bb <= beat][-1]
        acc = 1.0 if beat % 4 == 0 else (0.6 if beat % 2 == 0 else 0.42)
        mix.place('music', timp(roots[c], 0.7, acc), tb, 0.3 if beat < 16 else 0.38)
        if beat >= 8:
            mix.place('music', A.hat(0.03), tb + T.BEAT / 2, 0.035 if beat < 16 else 0.05)
    for i in range(8):                                       # timpani roll into the final chord
        mix.place('music', timp(43, 0.3, 0.5 + 0.06 * i), T.T_SONG + 21 * T.BEAT + i * T.BEAT / 8, 0.22)
    mix.place('music', timp(43, 1.4, 1.2), T.T_SONG + 22 * T.BEAT, 0.4)
    mix.place('music', A.pad([55, 59, 62, 67], 3.0, cut=1200, att=0.1), T.T_SONG + 22 * T.BEAT, 0.06)

    out = mix.master(reverb_buses={'honk': 0.55, 'music': 0.9, 'fx': 0.4},
                     gains={'honk': 1.0, 'music': 1.0, 'fx': 1.0, 'amb': 1.0},
                     gain_db_pts=[(0, 0), (T.T_SONG - 0.3, 0), (T.T_SONG, 1.5), (T.T_END + 0.5, 1.5), (54.0, 0), (60, 0)])
    A.write('audio.wav', out)

    # checks: section loudness, and the honk band sitting above the music bed
    for a, b, rms, pk in A.section_levels('audio.wav', [(0, 4), (4, 12), (12, 20), (20, 29), (29, 34.5), (34.5, 46.5), (46.5, 52.5), (52.5, 60)]):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')
    from scipy.signal import butter, sosfilt
    sos = butter(4, [150, 1500], 'bp', fs=SR, output='sos')
    for name in ('honk', 'music'):
        x = mix.buses[name][:N].mean(1)
        y = sosfilt(sos, x)
        for a, b in [(34.5, 40.5), (40.5, 46.5), (46.5, 52.5)]:
            seg = y[int(a * SR):int(b * SR)]
            print(f'  {name:5s} 150-1500 Hz {a:4.1f}-{b:4.1f}s: {20 * np.log10(np.sqrt((seg ** 2).mean()) + 1e-12):6.1f} dB')


if __name__ == '__main__':
    main()
