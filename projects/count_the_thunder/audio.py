"""Count the Thunder: soundtrack. Everything is synthesised; the music is original (D minor).

The science IS the arrangement (all times from timeline.STRIKES):
  - first half: a soft "start" click at each flash, then one pitched tick per counted second (the
    count climbs a D-minor pentatonic), and the thunder lands exactly at flash + delay. The thunder's
    low-pass opens with nearness (fc = 180 Hz + 6 kHz * e^(-km/0.5)): far = low rumble, near = clap.
  - back half (120 BPM): a bright snare on every flash (the downbeat); the kick IS the thunder,
    late by the real delay (2 s = a bar, then 1, 1/2, 1/4, 1/8 s) until it fuses with the snare at 50 s.
  - rain is the shaker; the groove fades in at 30 s and out after the climax; one distant rumble as the
    storm leaves, then back to the calm opening (loop).

    python audio.py      -> audio.wav (+ stems/ for the band checks), prints levels"""
import math
import os
import sys
import numpy as np

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
from reelkit import audio as A
import timeline as T

SR = A.SR
NS = int(T.DUR * SR)


def env_curve(fn, n, t0=0.0, step=0.01):
    """sample a timeline function (0..1) into a per-sample envelope"""
    ts = np.arange(t0, t0 + n / SR + step, step)
    v = np.array([fn(x) for x in ts])
    return np.interp(A.tt(n) + t0, ts, v)


def smooth_noise(n, rate, rng):
    """slow random 0..1 curve (rolls of the rumble)"""
    k = max(4, int(n / SR * rate) + 4)
    pts = rng.uniform(0, 1, k)
    x = np.interp(np.linspace(0, k - 1, n), np.arange(k), pts)
    return x


# ------------------------------------------------------------------ the science voices
def fc_for(km):
    return 180.0 + 6000.0 * math.exp(-km / 0.5)


def thunder(km, seed, groove=False, big=False):
    """one thunder: crack (near only) + boom + rolling rumble + a mid 'roll' layer phones can hear.
    groove=True keeps the onset tight (it is the kick) and the tail shorter."""
    rng = np.random.default_rng(seed)
    near = math.exp(-km / 0.3)
    fc = fc_for(km)
    dur = (4.0 if big else 1.9) if groove else 5.2 - 1.4 * near
    n = int(dur * SR)
    t = A.tt(n)
    att = 0.004 if groove else 0.006 + 0.16 * min(1.0, km / 2.5)
    out = np.zeros(n)
    # crack: an N-wave + tearing burst, only when close
    if near > 0.02:
        tn = 0.0025 + 0.002 * km
        nn = int(tn * SR)
        nw = np.zeros(n)
        nw[:nn] = np.linspace(1, -1, nn)
        burst = A.hp(rng.normal(0, 1, n), 1400) * np.exp(-t / 0.07)
        crackle = np.zeros(n)
        for k in range(12):
            i = int(rng.uniform(0, 0.18) * SR)
            crackle[i:i + 60] += rng.choice([-1, 1]) * np.exp(-np.arange(60) / 10.0)
        out += (nw * 1.2 + burst * 0.8 + A.hp(crackle, 800) * 0.6) * near
    # boom: low-passed noise burst with the distance filter
    noise = rng.normal(0, 1, n)
    boom = A.lp(noise, min(fc, 9000), 4) * np.clip(t / att, 0, 1) * np.exp(-t / (0.35 + 0.5 * min(1, km)))
    # rumble: rolling, low, long
    roll = smooth_noise(n, 3.5, rng) ** 2
    tail = np.exp(-t / (dur * 0.35)) * np.clip(t / (att + 0.08), 0, 1)
    rum = A.lp(rng.normal(0, 1, n), min(fc, 420), 4) * roll * tail
    # phone layer: the roll in 160-600 Hz so small speakers still hear the rumble
    mid = A.bp(rng.normal(0, 1, n), 150, min(700, fc * 2.2)) * roll * tail
    out += boom * (1.0 + 0.6 * near) + rum * 1.6 + mid * 0.7
    if groove:
        # the kick body: pitched sweep + knock + click, tight and on time
        f = 46 + (130 - 46) * np.exp(-t / 0.035)
        ph = 2 * np.pi * np.cumsum(f) / SR
        body = np.sin(ph) * np.exp(-t / 0.3) + 0.35 * np.sin(2 * ph) * np.exp(-t / 0.06)
        click = A.hp(rng.normal(0, 1, n), 2000) * np.exp(-t / 0.004)
        out += body * 1.6 + click * 0.5
    sub = np.sin(2 * np.pi * 42 * t) * np.exp(-t / 0.5) * np.clip(t / (att + 0.01), 0, 1)
    out += sub * (0.4 + 0.6 * near)
    out /= np.abs(out).max() + 1e-9
    return out * (0.42 + 0.58 * math.exp(-km / 1.0))


PENTA = [0, 3, 5, 7, 10, 12, 15, 17, 19, 22]      # D minor pentatonic steps


def count_tick(nidx):
    """woodblock tick for count number nidx (1..9): pitch climbs with the count"""
    m = 74 + PENTA[min(nidx - 1, 9)]
    f = A.mtof(m)
    n = int(0.16 * SR)
    t = A.tt(n)
    x = np.sin(2 * np.pi * f * t) * np.exp(-t / 0.05) + 0.45 * np.sin(2 * np.pi * f * 2.76 * t) * np.exp(-t / 0.02)
    x += A.hp(np.random.default_rng(nidx).normal(0, 1, n), 3000) * np.exp(-t / 0.003) * 0.4
    return x


def start_click():
    """the stopwatch button at the flash (quiet): light itself is silent"""
    n = int(0.05 * SR)
    t = A.tt(n)
    return (np.sin(2 * np.pi * 3200 * t) * 0.6 + A.hp(np.random.default_rng(9).normal(0, 1, n), 4000) * 0.4) * np.exp(-t / 0.008)


# ------------------------------------------------------------------ music voices
def lead(m, dur):
    """pulse-wave lead (chiptune flavour) with a little vibrato"""
    n = int((dur + 0.15) * SR)
    t = A.tt(n)
    f = A.mtof(m) * (1 + 0.005 * np.sin(2 * np.pi * 5.5 * t) * np.clip((t - 0.12) / 0.2, 0, 1))
    ph = np.cumsum(f) / SR
    x = np.where((ph % 1.0) < 0.3, 1.0, -1.0)
    x = A.lp(x, 2600, 2)
    e = np.clip(t / 0.008, 0, 1) * np.where(t < dur, 1.0 - 0.25 * t / max(dur, 1e-3), 0.75 * np.exp(-(t - dur) / 0.05))
    return x * e


def stab(ms, dur=0.22):
    n = int(dur * SR)
    t = A.tt(n)
    x = sum(A.saw(A.mtof(m), n, det=0.004, voices=2) for m in ms) / len(ms)
    return A.lp(x, 1800) * np.exp(-t / 0.08)


def synth_bass(m, dur):
    n = int(dur * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = np.sin(2 * np.pi * f * t) + 0.5 * A.lp(A.saw(f, n), 700)
    return x * A.adsr(n, 0.004, 0.08, 0.7, 0.03)


def shaker(accent=1.0, seed=0):
    n = int(0.07 * SR)
    t = A.tt(n)
    return A.hp(np.random.default_rng(40 + seed).normal(0, 1, n), 5500) * (np.clip(t / 0.006, 0, 1) * np.exp(-t / 0.02)) * accent


def bright_snare():
    """the flash on the downbeat: snare + clap + a high sizzle"""
    s = A.snare(0.16)
    c = A.clap()
    n = max(len(s), len(c))
    out = np.zeros(n)
    out[:len(s)] += s * 1.0
    out[:len(c)] += c * 0.8
    t = A.tt(n)
    out += A.hp(np.random.default_rng(12).normal(0, 1, n), 7000) * np.exp(-t / 0.05) * 0.35
    return A.hp(out, 280, 2)                 # bright: leaves the low band to the thunder-kick


# D minor: Dm  Bb  F  C  (i VI III VII)
PROG = [(50, [62, 65, 69]), (46, [62, 65, 70]), (53, [60, 65, 69]), (48, [60, 64, 67])]
INTRO_PADS = [[50, 57, 62, 65], [46, 53, 58, 62], [43, 55, 58, 62], [45, 52, 57, 61]]   # Dm Bb Gm A
LEAD = [  # (bar offset in 8ths, midi, length in 8ths) per progression bar
    [(0, 69, 2), (2, 72, 1), (3, 74, 2), (5, 72, 1), (6, 69, 1), (7, 67, 1)],
    [(0, 65, 2), (2, 67, 1), (3, 69, 2), (5, 65, 1), (6, 62, 2)],
    [(0, 72, 2), (2, 69, 1), (3, 72, 2), (5, 74, 1), (6, 77, 1), (7, 76, 1)],
    [(0, 74, 2), (2, 72, 1), (3, 67, 2), (5, 64, 1), (6, 72, 2)],
]
BASS_PAT = [(0, 0, 1.6), (1.5, 0, 0.4), (2, 12, 0.8), (3, 0, 0.8), (4.5, 0, 0.4), (5, 7, 0.8), (6, 0, 0.8), (7, 12, 0.6)]


def main():
    mix = A.Mix(T.DUR)
    rng = np.random.default_rng(3)
    E = T.BEAT / 2                                     # an eighth note = 0.25 s

    # ---------------- the science: count ticks, flash clicks, thunder
    for i in range(T.N_STRIKES):
        tf, d = T.STRIKES[i]
        km = T.true_km(i)
        if i in T.FIRST_HALF:
            mix.place('count', start_click(), tf, 0.22)
        groove = tf >= T.T_GROOVE_IN
        big = i == T.N_STRIKES - 1
        th = thunder(km, 400 + i, groove=groove, big=big)
        mix.place('thunder', th, T.thunder_t(i), 1.3 if big else (0.8 if groove else 0.9))
    for tt, k in T.count_ticks():
        mix.place('count', count_tick(k), tt, 0.46)
    mix.place('thunder', thunder(3.0, 999), T.T_RUMBLE_END, 0.55)   # the storm leaving, behind us

    # ---------------- ambience: rain (the shaker's source), wind
    n = NS
    rain_env = env_curve(T.rain, n)
    rainn = A.bp(rng.normal(0, 1, n), 1200, 9000) * 0.5 + A.lp(rng.normal(0, 1, n), 900) * 0.25
    drops = np.zeros(n)
    for k in range(int(60 * 90)):
        i = int(rng.uniform(0, n - 400))
        drops[i:i + 200] += rng.uniform(0.3, 1.0) * np.exp(-np.arange(200) / 25.0) * rng.choice([-1, 1])
    rainn += A.hp(drops, 2500) * 0.3
    rainn -= A.bp(rainn, 1500, 4000) * 0.4                         # leave room for the count ticks
    mix.place('amb', rainn * rain_env * 0.22, 0.0, 1.0)
    wind_env = env_curve(T.wind, n)
    wv = A.lp(rng.normal(0, 1, n), 380) * (0.5 + 0.5 * smooth_noise(n, 0.4, rng))
    mix.place('amb', wv * wind_env * 0.16, 0.0, 1.0)

    # ---------------- first-half bed: soft pads (Dm Bb Gm A), quiet
    for k, ch in enumerate(INTRO_PADS):
        t0 = k * 7.5
        mix.place('music', A.pad(ch, 8.2, cut=900, att=1.2), t0, 0.075)
    # end / loop bed: Dm returns as the storm leaves and runs into the loop point
    mix.place('music', A.pad(INTRO_PADS[0], 9.0, cut=900, att=2.0), 51.5, 0.05)

    # ---------------- the groove (30 s -> 56 s), levels follow timeline.groove_level
    bars = T.downbeats()
    flash_t = {round(tf, 3) for tf, d in T.STRIKES if tf >= T.T_GROOVE_IN}
    for b, tb in enumerate(bars):
        g = T.groove_level(tb + 0.1)
        if g <= 0.01:
            continue
        root, chord = PROG[b % 4]
        if round(tb, 3) in flash_t:
            mix.place('drums', bright_snare(), tb, 0.55)
        for e8 in range(8):
            te = tb + e8 * E
            ge = T.groove_level(te)
            mix.place('drums', A.hat(0.03), te, 0.1 * ge * (1.2 if e8 % 2 else 0.8))
            if e8 == 7:
                mix.place('drums', A.hat(open_=True), te, 0.05 * ge)
        for s16 in range(16):
            ts = tb + s16 * E / 2
            mix.place('drums', shaker(1.0 if s16 % 4 == 2 else 0.55, s16), ts, 0.16 * T.rain(ts) * T.groove_level(ts))
        for pos, step, ln in BASS_PAT:
            mix.place('bass', synth_bass(root - 12 + step, ln * E), tb + pos * E, 0.15 * g)
        for pos in (1, 3, 5, 7):
            if pos in (1, 5):
                mix.place('music', stab(chord), tb + pos * E, 0.07 * g)
        mix.place('music', A.pad([root] + chord, T.BAR + 0.3, cut=1200, att=0.3), tb, 0.035 * g)
        if 42.0 - 1e-6 <= tb < 52.0:
            for pos, m, ln in LEAD[b % 4]:
                mix.place('lead', lead(m, ln * E * 0.92), tb + pos * E, 0.085 * g)
    # climax bar: the full groove under the fused flash + clap
    mix.place('drums', A.crash(2.6), T.T_CLIMAX, 0.3)

    # ---------------- duck music/bass under every thunder (the science sits on top)
    thb = mix.bus('thunder')
    env = np.sqrt(A.lp((thb[:, 0] ** 2 + thb[:, 1] ** 2) * 0.5, 8.0, 1).clip(0, None))
    env = env / (env.max() + 1e-9)
    for name, depth in (('music', 0.6), ('bass', 0.88), ('lead', 0.45)):
        b = mix.bus(name)
        b *= (1 - depth * np.clip(env * 6.0, 0, 1))[:, None]
    # carve the music bus under the count ticks band a little
    for name in ('music', 'lead'):
        b = mix.bus(name)
        b -= A.bp(b, 1800, 4200) * 0.45

    os.makedirs('stems', exist_ok=True)
    science = mix.bus('thunder')[:NS] + mix.bus('count')[:NS]
    other = sum(mix.bus(k)[:NS] for k in ('music', 'bass', 'lead', 'drums', 'amb'))
    out = mix.master(reverb_buses={'music': 0.9, 'lead': 0.5, 'thunder': 0.35, 'drums': 0.15},
                     gain_db_pts=[(0, 4), (29.5, 4), (34, 1.5), (52, 1.5), (56, 4), (60, 4)])
    A.write('audio.wav', out)
    sc = 0.89 / (np.abs(science + other).max() + 1e-9)
    A.write('stems/science.wav', science * sc)
    A.write('stems/music.wav', other * sc)
    A.write('stems/thunder.wav', mix.bus('thunder')[:NS] * sc)
    A.write('stems/count.wav', mix.bus('count')[:NS] * sc)
    for a, b, rms, pk in A.section_levels('audio.wav', [(0, 4), (4, 12), (12, 21), (21, 30), (30, 36), (36, 44), (44, 50), (50, 52), (52, 57), (57, 60)]):
        print(f'{a:4.0f}-{b:4.0f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


if __name__ == '__main__':
    main()
