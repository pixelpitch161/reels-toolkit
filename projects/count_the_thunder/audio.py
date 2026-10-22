"""Count the Thunder v2: soundtrack. Everything is synthesised; the music is original (D minor, 120 BPM).

The science IS the arrangement (all times from timeline.STRIKES):
  - every thunder lands at flash + delay, and its low-pass follows the distance
    (fc = 180 Hz + 6 kHz * e^(-km/0.5)): far = low rolling rumble, near = sharp clap.
  - 5-30 s: many far strikes, so their rumbles overlap and land out of step with the flashes (the smear);
    the tagged strikes are counted on the beat: one pitched tick per second (each octave = 5 s = 1 mile).
  - 30-50 s: a bright snare on every flash (the downbeat); the kick IS the thunder, late by the real
    delay (2 s = a bar, 1, 1/2, 1/4, 1/8 s) until it fuses with the snare at 50 s.
  - 54-60 s: the second cell: thunders 3 s, 2 s, 1 s late (one hit per second), riser into frame 0.

Never quiet: rain + wind + a low-passed groove from frame 0; the bed's low-pass opens as the storm nears
(fully open at 32 s). Cold open = climax (full groove). 3-5 s = tape rewind of the cold open.

    python audio.py      -> audio.wav (+ stems/ for the band checks), prints levels"""
import math
import os
import sys
import numpy as np
from scipy.signal import butter, sosfilt

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
from reelkit import audio as A
import timeline as T

SR = A.SR
NS = int(T.DUR * SR)
E = T.BEAT / 2                                       # an eighth = 0.25 s


def env_curve(fn, n, t0=0.0, step=0.01):
    """sample a timeline function into a per-sample curve"""
    ts = np.arange(t0, t0 + n / SR + step, step)
    v = np.array([fn(x) for x in ts])
    return np.interp(A.tt(n) + t0, ts, v)


def smooth_noise(n, rate, rng):
    k = max(4, int(n / SR * rate) + 4)
    pts = rng.uniform(0, 1, k)
    return np.interp(np.linspace(0, k - 1, n), np.arange(k), pts)


def varlp(x, fc_curve, block=480, order=4):
    """time-varying Butterworth low-pass (cutoff per 10 ms block, filter state carried across blocks)"""
    y = np.zeros_like(x)
    zi = None
    cache = {}
    for i0 in range(0, len(x), block):
        fc = int(np.clip(fc_curve[min(i0 + block // 2, len(x) - 1)], 40, SR / 2 - 200))
        if fc not in cache:
            cache[fc] = butter(order, fc, 'lp', fs=SR, output='sos')
        sos = cache[fc]
        seg = x[i0:i0 + block]
        if zi is None:
            zi = np.zeros((sos.shape[0], 2, seg.shape[1]))
        y[i0:i0 + block], zi = sosfilt(sos, seg, axis=0, zi=zi)
    return y


# ------------------------------------------------------------------ the science voices
def fc_for(km):
    return 180.0 + 6000.0 * math.exp(-km / 0.5)


def thunder(km, seed, groove=False, big=False, bright=0.0):
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
    if near > 0.02:                                   # crack: N-wave + tearing burst, only when close
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
    noise = rng.normal(0, 1, n)
    boom = A.lp(noise, min(fc, 9000), 4) * np.clip(t / att, 0, 1) * np.exp(-t / (0.35 + 0.5 * min(1, km)))
    roll = smooth_noise(n, 3.5, rng) ** 2
    tail = np.exp(-t / (dur * 0.35)) * np.clip(t / (att + 0.08), 0, 1)
    rum = A.lp(rng.normal(0, 1, n), min(fc, 420), 4) * roll * tail
    mid = A.bp(rng.normal(0, 1, n), 150, min(700, fc * 2.2) + 500 * bright) * roll * tail
    out += boom * (1.0 + 0.6 * near) + rum * 1.6 + mid * (0.7 + bright)
    if groove:                                        # the kick body: pitched sweep + knock + click
        f = 46 + (130 - 46) * np.exp(-t / 0.035)
        ph = 2 * np.pi * np.cumsum(f) / SR
        body = np.sin(ph) * np.exp(-t / 0.3) + 0.35 * np.sin(2 * ph) * np.exp(-t / 0.06)
        click = A.hp(rng.normal(0, 1, n), 2000) * np.exp(-t / 0.004)
        out += body * 1.6 + click * 0.5
    sub = np.sin(2 * np.pi * 42 * t) * np.exp(-t / 0.5) * np.clip(t / (att + 0.01), 0, 1)
    out += sub * (0.4 + 0.6 * near)
    out /= np.abs(out).max() + 1e-9
    return out * (0.42 + 0.58 * math.exp(-km / 1.0))


PENTA = [0, 3, 5, 7, 10]                             # D minor pentatonic


def count_tick(n):
    """woodblock tick for count n: climbs the pentatonic, one octave per 5 s (= 1 mile)"""
    m = 74 + PENTA[(n - 1) % 5] + 12 * ((n - 1) // 5)
    f = A.mtof(m)
    k = int(0.16 * SR)
    t = A.tt(k)
    x = np.sin(2 * np.pi * f * t) * np.exp(-t / 0.05) + 0.6 * np.sin(2 * np.pi * f * 2.76 * t) * np.exp(-t / 0.025)
    x += A.hp(np.random.default_rng(n).normal(0, 1, k), 3000) * np.exp(-t / 0.003) * 0.5
    return x * (0.7 if n > 10 else 1.0)


def start_click():
    """the stopwatch button at a counted flash (light itself is silent)"""
    n = int(0.05 * SR)
    t = A.tt(n)
    return (np.sin(2 * np.pi * 3200 * t) * 0.6 + A.hp(np.random.default_rng(9).normal(0, 1, n), 4000) * 0.4) * np.exp(-t / 0.008)


# ------------------------------------------------------------------ music voices
def lead(m, dur):
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
    """the flash on the downbeat: snare + clap + a high sizzle (leaves the low band to the thunder-kick)"""
    s = A.snare(0.16)
    c = A.clap()
    n = max(len(s), len(c))
    out = np.zeros(n)
    out[:len(s)] += s
    out[:len(c)] += c * 0.8
    t = A.tt(n)
    out += A.hp(np.random.default_rng(12).normal(0, 1, n), 7000) * np.exp(-t / 0.05) * 0.35
    return A.hp(out, 280, 2)


def tom(m, dec=0.22):
    n = int(0.5 * SR)
    t = A.tt(n)
    f0 = A.mtof(m)
    f = f0 * (1 + 0.6 * np.exp(-t / 0.03))
    return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / dec) + 0.2 * A.bp(np.random.default_rng(int(m)).normal(0, 1, n), 200, 3000) * np.exp(-t / 0.02)


def scratch(src, dur=0.32, depth=0.09, seed=0):
    """a DJ 'wika': read a short source back and forth with a swung speed profile"""
    n = int(dur * SR)
    u = np.linspace(0, 1, n)
    pos = depth * SR * (1 - np.cos(2 * np.pi * u)) / 2 * (1 + 0.25 * np.sin(6 * np.pi * u))
    y = np.interp(pos, np.arange(len(src)), src)
    return A.bp(y, 300, 6000) * np.sin(np.pi * u) ** 0.5


# chords (root, triad) - D minor
CH = {'Dm': (50, [62, 65, 69]), 'Bb': (46, [62, 65, 70]), 'F': (53, [60, 65, 69]), 'C': (48, [60, 64, 67]),
      'Gm': (43, [62, 67, 70]), 'A': (45, [61, 64, 69])}
PROG = ['Dm', 'Bb', 'F', 'C']
DARK = ['Dm', 'Bb', 'Gm', 'A']
LEAD = {  # (offset in 8ths, midi, length in 8ths) per chord
    'Dm': [(0, 69, 2), (2, 72, 1), (3, 74, 2), (5, 72, 1), (6, 69, 1), (7, 67, 1)],
    'Bb': [(0, 65, 2), (2, 67, 1), (3, 69, 2), (5, 65, 1), (6, 62, 2)],
    'F': [(0, 72, 2), (2, 69, 1), (3, 72, 2), (5, 74, 1), (6, 77, 1), (7, 76, 1)],
    'C': [(0, 74, 2), (2, 72, 1), (3, 67, 2), (5, 64, 1), (6, 72, 2)],
    'A': [(0, 69, 2), (2, 73, 2), (4, 76, 2), (6, 73, 1), (7, 76, 1)],
}
BASS_PAT = [(0, 0, 1.6), (1.5, 0, 0.4), (2, 12, 0.8), (3, 0, 0.8), (4.5, 0, 0.4), (5, 7, 0.8), (6, 0, 0.8), (7, 12, 0.6)]


def section(tb):
    if tb < T.T_CO:
        return 'cold'
    if tb < T.T_FILL:
        return 'chaos'
    if tb < T.T_FILL + 1.0:
        return 'fill'
    if tb < 32.0:
        return 'open'
    if tb < T.T_CLIMAX:
        return 'lock'
    if tb < 52.0:
        return 'climax'
    return 'post'


def chord_for(tb):
    s = section(tb)
    if s == 'cold':
        return PROG[int(round(tb / 2)) % 4]
    if s == 'chaos':
        return DARK[int(round((tb - 5) / 2)) % 4]
    if s == 'fill':
        return 'A'
    if tb >= 58.0 - 1e-6:
        return 'A'                                   # V: pulls into the cold open's Dm at the loop
    return PROG[int(round((tb - 34) / 2)) % 4]


def main():
    mix = A.Mix(T.DUR)
    rng = np.random.default_rng(3)
    flash_db = {round(T.STRIKES[i][0], 3) for i in T.SEEN if T.KIND[i] in ('count', 'lock', 'rod', 'cell2') and T.STRIKES[i][0] >= 30 or T.KIND[i] == 'rod'}

    # ---------------- the science: thunder (time + filter from the physics), count ticks
    for i in range(T.N_STRIKES):
        tf, d, az, kind = T.STRIKES[i]
        km = T.true_km(i)
        tt = T.thunder_t(i)
        if kind in ('lock', 'rod', 'cell2') or (kind == 'count' and tf >= 30):
            th = thunder(km, 400 + i, groove=True, big=(kind == 'rod'))
            mix.place('thunder_k', th, tt, 1.6 if kind == 'rod' else 1.2)            # the thunder-kick
        elif kind == 'count':
            # the counted far ones: clearly on top of the smear (a little brighter, louder)
            th = thunder(km, 400 + i, bright=0.6)
            mix.place('thunder_k', th, tt, 1.5 if i == T.I_TAG else 1.25, pan=-0.1)
        else:
            th = thunder(km, 400 + i)
            pan = float(np.clip((az or 0) / 22.0, -0.7, 0.7))
            mix.place('thunder', th, tt, 1.0 * (0.75 + 0.25 * math.exp(-km / 3.0)), pan=pan)
    for i in T.COUNTED:
        mix.place('count', start_click(), T.STRIKES[i][0], 0.3)
    TICK_T = {round(tt, 3) for tt, k in T.count_ticks()}
    for tt, k in T.count_ticks():
        mix.place('count', count_tick(k), tt, 0.42 if tt < 30 else 0.75)       # the groove is busier from 30 s

    # ---------------- ambience: rain (heavy in the cold open and from ~30 s), wind
    n = NS
    rain_env = env_curve(T.rain, n)
    rainn = A.bp(rng.normal(0, 1, n), 1200, 9000) * 0.5 + A.lp(rng.normal(0, 1, n), 900) * 0.25
    drops = np.zeros(n)
    for k in range(int(60 * 110)):
        i = int(rng.uniform(0, n - 400))
        drops[i:i + 200] += rng.uniform(0.3, 1.0) * np.exp(-np.arange(200) / 25.0) * rng.choice([-1, 1])
    rainn += A.hp(drops, 2500) * 0.3
    rainn -= A.bp(rainn, 1500, 4000) * 0.45                       # leave room for the count ticks
    mix.place('amb', rainn * (0.35 + 0.65 * rain_env) * 0.17, 0.0, 1.0)
    wind_env = env_curve(T.wind, n)
    wv = A.lp(rng.normal(0, 1, n), 380) * (0.5 + 0.5 * smooth_noise(n, 0.4, rng))
    mix.place('amb', wv * (0.3 + 0.7 * wind_env) * 0.2, 0.0, 1.0)

    # ---------------- the groove (bed buses are low-passed by timeline.bed_cutoff afterwards)
    for tb, blen in T.bars():
        sec = section(tb)
        name = chord_for(tb)
        root, chord = CH[name]
        n8 = int(round(blen / E))
        full = sec in ('cold', 'lock', 'climax', 'post')
        lvl = {'cold': 1.0, 'chaos': 0.9, 'fill': 0.9, 'open': 0.95, 'lock': 0.95, 'climax': 1.1, 'post': 1.0}[sec]
        if sec == 'lock':
            lvl = 0.85 + 0.25 * (tb - 32) / 18.0                  # builds into the climax
        # flash on the downbeat = bright snare (only the on-grid flashes)
        if round(tb, 3) in flash_db:
            # a count tick on the same downbeat keeps the 1.5-4 kHz band: the snare steps back there
            mix.place('drums', bright_snare(), tb, (0.3 if round(tb, 3) in TICK_T else 0.95) * lvl)
        if sec in ('cold', 'climax'):
            mix.place('drums', A.crash(1.3), tb, 0.2)
        # chaos: a muffled four-on-the-floor heartbeat (fades out as the thunder takes the kick over)
        if sec in ('chaos', 'fill', 'open'):
            for b4 in range(int(round(blen / T.BEAT))):
                tk = tb + b4 * T.BEAT
                g = 1.0 - T.ss(30.0, 32.5, tk)
                if g > 0.01:
                    mix.place('bed_kick', A.kick(1.0, 0.3), tk, 0.2 * g)
        if sec in ('cold', 'climax'):
            mix.place('bed_kick', A.kick(0.9, 0.28), tb + 2 * T.BEAT, 0.3)        # extra drive on beat 3
        # hats / shaker / backbeat
        for e8 in range(n8):
            te = tb + e8 * E
            if full or (sec == 'open' and te >= 30.0):
                mix.place('drums', A.hat(0.022), te, 0.42 * lvl * (1.2 if e8 % 2 else 0.8))
                if e8 == 7:
                    mix.place('drums', A.hat(open_=True), te, 0.1 * lvl)
                if e8 in (2, 6):
                    mix.place('drums', A.clap(), te, 0.3 * lvl)
            elif sec == 'chaos' and e8 % 2 == 1:
                mix.place('drums', A.hat(0.02), te, 0.10)          # (muffled by the bed filter)
        for s16 in range(n8 * 2):
            ts = tb + s16 * E / 2
            acc = 1.0 if s16 % 4 == 2 else 0.55
            if full or sec == 'open':
                mix.place('drums', shaker(acc, s16), ts, 0.12 * lvl * (0.5 + 0.5 * T.rain(ts)))
            else:
                mix.place('amb', shaker(acc, s16), ts, 0.16 * (0.6 + 0.4 * T.rain(ts)))   # rain-shaker, unfiltered
        # bass
        for pos, step, ln in BASS_PAT:
            if pos < n8:
                mix.place('bass', synth_bass(root - 12 + step, ln * E), tb + pos * E, (0.14 if sec == 'chaos' else 0.17) * lvl)
        # stabs + pad
        for pos in (1, 5):
            if pos < n8:
                mix.place('music', stab(chord), tb + pos * E, 0.15 * lvl)
        mix.place('music', A.pad([root] + chord, blen + 0.3, cut=1200, att=0.25), tb, 0.3 * lvl)
        # lead: cold open, then from 36 s on (an octave up from 44 s and in the climax)
        if sec in ('cold', 'climax', 'post') or (sec == 'lock' and tb >= 36.0):
            up = 12 if (tb >= 44.0 or sec in ('cold', 'climax')) else 0
            for pos, m, ln in LEAD[name]:
                if pos < n8:
                    mix.place('lead', lead(m + up, ln * E * 0.92), tb + pos * E, 0.12 * lvl, pan=0.15)
        # the 2/4 fill into the even grid ("IT'S COMING")
        if sec == 'fill':
            for k, m in enumerate([50, 50, 47, 47, 45, 45, 43, 43]):
                mix.place('fill', tom(m), tb + k * E / 2, 0.16 + 0.03 * k)
            for k in range(8):
                mix.place('fill', A.snare(0.08), tb + k * E / 2, 0.08 + 0.04 * k)

    # ---------------- the riser into the loop (57-60): noise sweep, pitch riser, snare roll
    nr = int(3.0 * SR)
    tr = A.tt(nr)
    sweep = np.zeros(nr)
    xr = rng.normal(0, 1, nr)
    nb = 30
    for k in range(nb):
        u = (k + 0.5) / nb
        fc = 400 * (9000 / 400) ** u
        a0, a1 = int(k * nr / nb), int((k + 1) * nr / nb)
        sweep[a0:a1] = A.bp(xr[max(0, a0 - 2048):a1], fc * 0.8, min(fc * 1.25, SR / 2 - 100))[-(a1 - a0):]
    mix.place('fx', sweep * (tr / 3.0) ** 2, 57.0, 0.17)
    f = A.mtof(38) * 2 ** (2 * (tr / 3.0) ** 1.5)
    riser = A.lp(np.sign(np.sin(2 * np.pi * np.cumsum(f) / SR)), 3000) * (tr / 3.0) ** 2
    mix.place('fx', riser, 57.0, 0.05)
    t = 58.0
    while t < 60.0 - 1e-6:
        step = E if t < 59.0 else (E / 2 if t < 59.5 else E / 4)
        mix.place('fill', A.snare(0.06), t, 0.12 + 0.16 * (t - 58.0) / 2.0)
        t += step

    # ---------------- bed filter: muffled at the horizon, opening as the storm nears
    fcc = env_curve(T.bed_cutoff, mix.NS + SR * 6, step=0.02)
    for name in ('drums', 'bed_kick', 'bass', 'music', 'lead'):
        b = mix.bus(name)
        b[:] = varlp(b, fcc)

    # ---------------- duck the music under the counted / lock-in thunders (the science on top)
    kb = mix.bus('thunder_k')
    env = np.sqrt(A.lp((kb[:, 0] ** 2 + kb[:, 1] ** 2) * 0.5, 8.0, 1).clip(0, None))
    env = env / (env.max() + 1e-9)
    for name, depth in (('music', 0.55), ('bass', 0.85), ('lead', 0.4), ('bed_kick', 0.7)):
        b = mix.bus(name)
        b *= (1 - depth * np.clip(env * 5.0, 0, 1))[:, None]
    # the chaos smear sits under the counted rumbles too
    th = mix.bus('thunder')
    th *= (1 - 0.6 * np.clip(env * 4.0, 0, 1))[:, None]
    # sidechain pump on every beat (0.5 s grid runs through the whole track): pads, rain and the smear breathe
    nbus = mix.NS + SR * 6
    tq = A.tt(nbus)
    ph = np.mod(tq, T.BEAT)
    pump = np.exp(-ph / 0.11) * np.clip(ph / 0.004, 0, 1)
    pump[(tq >= T.T_CO) & (tq < T.T_REW)] = 0.0
    for name, depth in (('music', 0.65), ('amb', 0.32), ('thunder', 0.3), ('lead', 0.15)):
        b = mix.bus(name)
        b *= (1 - depth * pump)[:, None]
    # carve the music under the count ticks band
    for name in ('music', 'lead'):
        b = mix.bus(name)
        b -= A.bp(b, 1800, 4200) * 0.45

    # ---------------- 3-5 s: tape rewind of the cold open (replaces everything in that window)
    i3, i5 = int(T.T_CO * SR), int(T.T_REW * SR)
    pre = sum(b for b in mix.buses.values())[:i3]                  # the cold open as heard
    nrw = i5 - i3
    u = np.linspace(0, 1, nrw)
    speed = np.where(u < 0.86, 1.0 + 8.0 * (u / 0.86) ** 1.6, 9.0 * (1 - (u - 0.86) / 0.14) ** 1.5)
    pos = i3 - 1 - np.cumsum(speed)
    pos = np.mod(pos, i3)                                          # wraps around the 3 s (tape loop)
    rew = np.stack([np.interp(pos, np.arange(i3), pre[:, c]) for c in range(2)], 1)
    rew = A.hp(A.lp(rew, 7000, 2), 120, 2) * (0.85 + 0.15 * u)[:, None]
    for name in list(mix.buses):
        mix.buses[name][i3:i5] *= np.linspace(1, 0, nrw)[:, None] ** 8 if name != 'fx' else 1.0
    mix.place('fx', rew, T.T_CO, 0.9)
    clap_src = pre[:int(0.4 * SR)].mean(1)
    mix.place('fx', scratch(clap_src, 0.3, 0.08, 1), T.T_CO, 0.9)
    mix.place('fx', scratch(clap_src[::-1].copy(), 0.26, 0.06, 2), T.T_CO + 0.3, 0.6)
    rc = A.crash(1.7)[::-1]                                        # reverse cymbal swelling into 5 s
    mix.place('fx', A.lp(rc, 9000), T.T_REW - len(rc) / SR - 0.06, 0.55)
    mix.place('fx', A.whoosh(1.4, 250, 5000, rise=False, seed=8), T.T_CO + 0.15, 0.35)
    # the drop back in at 5 s: low boom + tape spin-up on the first bed beat
    nb_ = int(0.9 * SR)
    tb_ = A.tt(nb_)
    boom = np.sin(2 * np.pi * np.cumsum(38 + 60 * np.exp(-tb_ / 0.08)) / SR) * np.exp(-tb_ / 0.35)
    mix.place('fx', boom, T.T_REW, 0.6)

    if os.environ.get('BUS_LEVELS'):
        spans = [(0, 3), (5, 15), (15, 28), (32, 42), (42, 50), (50, 52), (52, 57), (57, 60)]
        print('bus        ' + ' '.join(f'{a:>2}-{b:<3}' for a, b in spans))
        for k, b in sorted(mix.buses.items()):
            row = [20 * np.log10(np.sqrt((b[int(a * SR):int(c * SR)] ** 2).mean()) + 1e-12) for a, c in spans]
            print(f'{k:10s} ' + ' '.join(f'{v:6.1f}' for v in row))

    os.makedirs('stems', exist_ok=True)
    science = mix.bus('thunder')[:NS] + mix.bus('thunder_k')[:NS] + mix.bus('count')[:NS]
    other = sum(mix.bus(k)[:NS] for k in mix.buses if k not in ('thunder', 'thunder_k', 'count'))
    out = master(mix, GAINS, REVERB, GAIN_PTS)
    A.write('audio.wav', out)
    sc = 0.89 / (np.abs(science + other).max() + 1e-9)
    A.write('stems/science.wav', science * sc)
    A.write('stems/music.wav', other * sc)
    A.write('stems/thunder.wav', (mix.bus('thunder')[:NS] + mix.bus('thunder_k')[:NS]) * sc)
    A.write('stems/thunder_k.wav', mix.bus('thunder_k')[:NS] * sc)
    A.write('stems/count.wav', mix.bus('count')[:NS] * sc)
    for a, b, rms, pk in A.section_levels('audio.wav', [(0, 3), (3, 5), (5, 10), (10, 15), (15, 20), (20, 25), (25, 30), (30, 35),
                                                         (35, 40), (40, 45), (45, 50), (50, 52), (52, 57), (57, 60)]):
        print(f'{a:4.0f}-{b:4.0f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


def master(mix, gains, reverb_buses, gain_pts, ceiling=0.89):
    """sum + reverb + section gain + slow glue compressor + drive into a 3 ms lookahead limiter
    (the big claps no longer set the level of everything else) + a touch of soft clip"""
    from scipy.ndimage import minimum_filter1d, uniform_filter1d
    from scipy.signal import lfilter
    dry = sum(b * gains.get(k, 1.0) for k, b in mix.buses.items())
    send = sum(mix.buses[k] * v for k, v in reverb_buses.items() if k in mix.buses)
    x = (dry + A.reverb(send) * 0.22)[:NS]
    g = np.interp(A.tt(NS), [p[0] for p in gain_pts], [p[1] for p in gain_pts])
    x = x * (10 ** (g / 20))[:, None]
    # glue: RMS ~80 ms, 2:1 above thr
    a = math.exp(-1.0 / (0.08 * SR))
    env = np.sqrt(lfilter([1 - a], [1, -a], (x ** 2).mean(1)).clip(1e-12))
    gc = np.where(env > COMP_THR, (COMP_THR / env) ** 0.5, 1.0)
    x = x * gc[:, None]
    x = x * 10 ** (DRIVE_DB / 20)
    # lookahead peak limiter
    lim = 0.84
    pk = np.abs(x).max(1)
    gl = np.minimum(1.0, lim / np.maximum(pk, 1e-9))
    L = int(0.003 * SR)
    gl = minimum_filter1d(gl, 2 * L + 1)
    gl = uniform_filter1d(gl, 2 * L + 1)
    # release: recover at most ~ 1/60 ms per step on a 1 ms grid
    hop = SR // 1000
    gb = gl[:len(gl) // hop * hop].reshape(-1, hop).min(1)
    rel = math.exp(-1.0 / 60.0)
    y = np.empty_like(gb)
    cur = 1.0
    for i, v in enumerate(gb):
        cur = v if v < cur else min(v, 1 - (1 - cur) * rel)
        y[i] = cur
    gs = np.interp(np.arange(len(gl)), np.arange(len(y)) * hop + hop // 2, y)
    x = x * np.minimum(gl, gs)[:, None]
    x = np.tanh(x * 1.05) / np.tanh(1.05)
    x *= ceiling / (np.abs(x).max() + 1e-9)
    fd = int(0.004 * SR)
    x[:fd] *= np.linspace(0, 1, fd)[:, None]
    x[-fd:] *= np.linspace(1, 0, fd)[:, None]
    return x


GAINS = {}
REVERB = {'music': 0.9, 'lead': 0.5, 'thunder': 0.35, 'thunder_k': 0.3, 'drums': 0.07, 'count': 0.1}
COMP_THR = 0.12
DRIVE_DB = 5.0
GAIN_PTS = [(0, 0.0), (3.0, 0.0), (4.9, -2.0), (5.0, -5.8), (20, -5.4), (25, -5.0), (27, -5.4), (30, -3.6), (33, -2.8), (48, -0.6), (50, 0.3),
            (52, 0.3), (53, -1.5), (57, -1.5), (60, 0.0)]

if __name__ == '__main__':
    main()
