"""Prime-Number Cicadas soundtrack (all synthesised, all original).

The science is the rhythm: 1 year = 1 sixteenth at 120 BPM. Brood XIX calls every 13 sixteenths,
Brood XIII every 17, over an original A-major groove (A | E | F#m | D, 2 s bars, bar lines on odd
seconds so 2024 lands on a downbeat). The calls are synthesised "Pharaoh" calls: a near-pure tone
pulsed by the tymbals (~150 Hz AM) with a falling slur at the end. XIX (13-year, M. tredecim sings
slightly lower) = C#6 voice, XIII = E6 voice; both sit in 1.0-1.4 kHz, which is carved out of the
music bus so the calls ride on top. Each hit also plucks a chord tone (XIX low, XIII high), so the
13:17 polyrhythm is heard in the harmony too. The ghost 12:18 broods (36 years per second) become a
3-against-2 that meets every second.

    python audio.py        -> audio.wav (+ section levels and the call-band check)"""
import os
import sys
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit import audio as A
import timeline as T

SR = A.SR
F_XIX, F_XIII = A.mtof(85), A.mtof(88)        # C#6 1108.7 Hz, E6 1318.5 Hz
CHORDS = [[45, 57, 61, 64], [40, 56, 59, 64], [42, 57, 61, 66], [38, 57, 62, 66]]   # A, E/G#, F#m, D (bass + triad)


def chord_at(t):
    k = int(np.floor((t - T.GRID0) / T.BAR)) % 4
    return CHORDS[k]


def ss(a, b, x):
    x = np.clip((x - a) / (b - a), 0, 1)
    return x * x * (3 - 2 * x)


# ------------------------------------------------------------------ voices
def pharaoh(f0, dur, drop=0.17, pulse=148.0, seed=0, swell=0.0):
    """periodical cicada 'Pharaoh' call: near-pure tone, tymbal AM, falling slur at the end"""
    r = np.random.default_rng(seed)
    n = int(dur * SR)
    t = A.tt(n)
    u = t / dur
    f = f0 * (1 - drop * ss(0.6, 1.0, u)) * (1 + 0.004 * np.sin(2 * np.pi * 5.5 * t + r.uniform(0, 6)))
    ph = 2 * np.pi * np.cumsum(f) / SR
    car = np.sin(ph) + 0.18 * np.sin(2 * ph + 0.4) + 0.06 * np.sin(3 * ph + 1.1)
    pr = pulse * (1 + 0.03 * r.normal())
    am = 0.45 + 0.55 * (0.5 + 0.5 * np.sin(2 * np.pi * pr * t)) ** 2
    env = np.clip(t / 0.05, 0, 1) * (1 - ss(0.72, 1.0, u)) * (1 + swell * ss(0.0, 0.6, u))
    return car * am * env


def brood_hit(f0, dur, seed, voices=3, spread=0.012):
    """one emergence = a little brood: a few detuned, slightly staggered calls"""
    r = np.random.default_rng(seed)
    n = int((dur + 0.05) * SR)
    out = np.zeros(n)
    for v in range(voices):
        d = int(r.uniform(0, 0.03) * SR)
        x = pharaoh(f0 * (1 + r.uniform(-spread, spread)), dur, seed=seed * 7 + v)
        out[d:d + len(x)] += x[:n - d] / voices ** 0.6
    return out


def chorus(f0, dur, n_calls, seed, spread=0.035, call=(0.8, 1.6)):
    """a whole brood singing: many overlapping calls (a dense, swelling buzz)"""
    r = np.random.default_rng(seed)
    n = int(dur * SR)
    out = np.zeros(n)
    for k in range(n_calls):
        cd = r.uniform(*call)
        t0 = r.uniform(0, max(0.01, dur - cd))
        x = pharaoh(f0 * (1 + r.uniform(-spread, spread)), cd, drop=r.uniform(0.12, 0.2), pulse=r.uniform(135, 165), seed=seed + k)
        i0 = int(t0 * SR)
        out[i0:i0 + len(x)] += x[:n - i0] * r.uniform(0.5, 1.0)
    return out / np.sqrt(max(1, n_calls / 4))


def marimba(m, dur=0.6, bright=1.0):
    n = int(dur * SR)
    t = A.tt(n)
    f = A.mtof(m)
    return (np.sin(2 * np.pi * f * t) * np.exp(-t / (dur * 0.4))
            + 0.3 * bright * np.sin(2 * np.pi * f * 3.93 * t) * np.exp(-t / 0.05)) * np.clip(t / 0.002, 0, 1)


def tok(f, dur=0.12):
    n = int(dur * SR)
    t = A.tt(n)
    return np.sin(2 * np.pi * f * t) * np.exp(-t / 0.03) + 0.2 * A.bp(np.random.default_rng(int(f)).normal(0, 1, n), f * 0.8, f * 1.6) * np.exp(-t / 0.01)


def bird_chirp(seed):
    r = np.random.default_rng(seed)
    parts = []
    for k in range(r.integers(2, 5)):
        n = int(r.uniform(0.05, 0.09) * SR)
        t = A.tt(n)
        f = np.linspace(r.uniform(2800, 3600), r.uniform(4000, 5200), n)
        parts.append(np.sin(2 * np.pi * np.cumsum(f) / SR) * np.sin(np.linspace(0, np.pi, n)) ** 2)
        parts.append(np.zeros(int(r.uniform(0.04, 0.09) * SR)))
    return np.concatenate(parts)


def scratch(seed, dur=0.06):
    n = int(dur * SR)
    return A.bp(np.random.default_rng(seed).normal(0, 1, n), 300, 1800) * np.exp(-A.tt(n) / 0.015)


def music_pad(ms, dur, cut=900):
    return A.pad(ms, dur, cut=cut, att=0.25)


# ------------------------------------------------------------------ arrangement
def main():
    mix = A.Mix(T.DUR)
    N = mix.NS
    rng = np.random.default_rng(12)
    tt = A.tt(N)
    # -- ambience: soft wind, golden-hour birds, soil rumble underground (start == end for the loop)
    wind = A.lp(rng.normal(0, 1, N), 420) * (0.6 + 0.4 * np.sin(2 * np.pi * tt / 20) ** 2)
    mix.place('amb', wind, 0.0, 0.10)
    under = np.clip(1 - ss(4.3, 5.2, tt) + ss(55.6, 58.0, tt), 0, 1) + 0.35 * np.clip(ss(8.6, 9.6, tt) - ss(15.6, 16.4, tt), 0, 1)
    rumble = A.lp(np.random.default_rng(3).normal(0, 1, N), 90, 4) * 3.0 * under
    mix.place('amb', rumble, 0.0, 0.22)
    for k, tb in enumerate(np.arange(1.7, 59.0, 2.9) + rng.uniform(-0.6, 0.6, 20)):
        if T.T_2024 - 0.5 < tb < T.T_END:
            continue
        mix.place('amb', A.hp(bird_chirp(40 + k), 2000), float(tb), 0.03, rng.uniform(-0.7, 0.7))
    # digging scratches while nymphs climb / burrow (very quiet)
    for a, b in [(T.T_CLIMB0, T.T_POP0), T.T_BURROW0, T.T_BURROW1]:
        x = a
        k = 0
        while x < b:
            mix.place('amb', scratch(100 + k + int(a * 10)), x, 0.035, rng.uniform(-0.5, 0.5))
            x += rng.uniform(0.08, 0.2)
            k += 1
    # -- hook: one distant buzz, then the 2024 double chorus
    mix.place('cic', A.lp(pharaoh(F_XIX, 1.5, seed=1), 2600), 1.0, 0.16, -0.15)
    for s, f0, pan in (('L', F_XIX, -0.45), ('R', F_XIII, 0.45)):
        c = chorus(f0, 4.0, 26, seed=50 if s == 'L' else 60)
        env = np.clip(A.tt(len(c)) / 0.5, 0, 1) * (1 - ss(2.6, 4.0, A.tt(len(c))))
        mix.place('cic', c * env, T.T_POP0 - 0.05, 0.42, pan)
    mix.place('fx', A.whoosh(1.0, 200, 4000), T.T_POP0 - 0.85, 0.16)
    mix.place('fx', A.kick(punch=1.0, dec=0.5, f0=110, f1=38), T.T_POP0, 0.35)
    # -- warm pad bed from the reveal on (kept below 900 Hz)
    t = T.GRID0 + 3 * T.BAR      # 7.0
    while t < T.T_GROOVE:
        lvl = 0.055 if t < 16 else 0.07
        mix.place('music', music_pad(chord_at(t)[1:], T.BAR + 0.3, cut=750), t, lvl)
        mix.place('music', A.bass(chord_at(t)[0] + 12, T.BAR * 0.9) * 0.6, t, 0.06 if t >= 12 else 0.0)
        t += T.BAR
    # -- 8-12 s: the rings complete: low XIX call, high XIII call
    mix.place('cic', brood_hit(F_XIX, 0.9, 5), T.RING_DONE['L'], 0.34, -0.45)
    mix.place('cic', brood_hit(F_XIII, 0.9, 6), T.RING_DONE['R'], 0.34, 0.45)
    for tr in np.arange(T.RING_T0, T.RING_DONE['R'], 1 / T.RING_RATE):
        mix.place('fx', tok(1760, 0.05), float(tr), 0.025)
    # -- 12 s: ping for "both prime"
    mix.place('fx', A.chime(81, 1.8), T.T_PING, 0.13, -0.3)
    mix.place('fx', A.chime(88, 1.8), T.T_PING + 0.22, 0.11, 0.3)
    # -- 16-20 s: ghost 12/18 broods at 36 years/s = a 3-against-2 that meets every second
    for tg in T.G12:
        mix.place('fx', A.lp(tok(620), 1800), tg, 0.2, -0.5)
    for tg in T.G18:
        mix.place('fx', A.lp(tok(930), 2200), tg, 0.18, 0.5)
    for tm in T.G_MEET:
        mix.place('fx', A.kick(punch=0.8, dec=0.2, f0=120, f1=50), tm, 0.22)
        mix.place('fx', A.lp(A.crash(0.5), 3000), tm, 0.05)
    # -- 20-23: riser + fill into the groove
    mix.place('fx', A.whoosh(2.6, 150, 3500), T.T_GROOVE - 2.6, 0.14)
    for k in range(8):
        mix.place('drums', A.snare(0.08), T.T_GROOVE - 1.0 + k * T.S16 * 1.0, 0.05 + 0.025 * k)
    # -- the groove (bars at odd seconds), 23 -> 55
    bar_t = T.T_GROOVE
    b = 0
    while bar_t < T.T_END + 1.0 - 1e-6:
        ch = chord_at(bar_t)
        climax = bar_t >= T.T_2024
        build = T.T_BUILD <= bar_t < T.T_2024
        mix.place('music', music_pad(ch[1:], T.BAR + 0.25, cut=820 if not climax else 880), bar_t, 0.075 if not climax else 0.1)
        for q in range(4):
            tq = bar_t + q * T.BEAT
            four = b >= 4 or climax
            last_beat = build and bar_t + q * T.BEAT >= T.T_2024 - 0.5
            if (q % 2 == 0 or four) and not last_beat:
                mix.place('drums', A.kick(punch=1.0, dec=0.28, f0=140, f1=48), tq, 0.5)
                mix.place('drums', tok(2400, 0.02), tq, 0.03)          # click so the kick survives phone speakers
            if q % 2 == 1 and b >= 2 and not last_beat:
                mix.place('drums', A.clap(), tq, 0.22)
                mix.place('drums', A.snare(0.12), tq, 0.1)
            # bass: root in eighths with an octave hop
            for e8 in range(2):
                m = ch[0] + (12 if (q * 2 + e8) % 4 == 3 else 0)
                mix.place('music', A.bass(m, T.BEAT / 2 * 0.85), tq + e8 * T.BEAT / 2, 0.15 if not last_beat else 0.0)
        # sixteenth ticks = the years (from bar 2 on): accent every beat
        if b >= 1:
            for s16 in range(16):
                ts = bar_t + s16 * T.S16
                acc = 1.0 if s16 % 4 == 0 else 0.55
                mix.place('drums', A.hat(0.02), ts, 0.07 * acc * (1.25 if climax else 1.0), 0.15)
        # arpeggio from bar 9 (39 s): chord tones in 16ths, octave 4 (under 700 Hz)
        if b >= 8:
            tones = [ch[1], ch[2], ch[3], ch[2] + 12 if ch[2] + 12 < 78 else ch[1] + 12]
            for s16 in range(16):
                m = tones[s16 % 4]
                mix.place('music', A.lp(A.tri(A.mtof(m), int(0.11 * SR)) * A.expenv(int(0.11 * SR), 0.05), 1600),
                          bar_t + s16 * T.S16, 0.035 if not climax else 0.045, 0.25 if s16 % 2 else -0.25)
        if build:
            for s16 in range(16):
                ts = bar_t + s16 * T.S16
                dens = (ts - T.T_BUILD) / (T.T_2024 - T.T_BUILD)
                if s16 % 2 == 0 or dens > 0.5:
                    mix.place('drums', A.snare(0.07), ts, 0.03 + 0.09 * dens)
        bar_t += T.BAR
        b += 1
    mix.place('fx', A.whoosh(3.5, 200, 6000), T.T_2024 - 3.5, 0.2)
    mix.place('drums', A.crash(2.5), T.T_2024, 0.32)
    mix.place('drums', A.kick(punch=1.2, dec=0.5, f0=150, f1=40), T.T_2024, 0.4)
    mix.place('music', music_pad([57, 61, 64, 71], 3.0, cut=900), T.T_END + 1.0, 0.09)      # final chord, decays
    mix.place('music', A.bass(45, 2.0), T.T_END + 1.0, 0.16)
    # -- the polyrhythm: every emergence in the race = calls + a chord-tone pluck
    for e in T.EMERGE:
        if e['kind'] == 'hook':
            continue
        side = e['side']
        f0 = F_XIX if side == 'L' else F_XIII
        pan = -0.45 if side == 'L' else 0.45
        ch = chord_at(e['t'] + 1e-3)
        if e['kind'] == 'climax':
            continue
        dur = 0.62 if e['kind'] == 'race' else 0.9
        mix.place('cic', brood_hit(f0, dur, 200 + e['id'], voices=4 if e['kind'] == 'joint' else 3), e['t'], 0.4, pan)
        m = ch[0] + 24 if side == 'L' else ch[3]
        mix.place('music', marimba(m, 0.7), e['t'], 0.2, pan * 0.6)
    # -- 2024: both choruses together on the downbeat, then the music thins to one buzz and soil
    for s, f0, pan in (('L', F_XIX, -0.4), ('R', F_XIII, 0.4)):
        c = chorus(f0, 4.6, 40, seed=70 if s == 'L' else 80)
        tc = A.tt(len(c))
        env = np.clip(tc / 0.12, 0, 1) * (1 + 0.25 * ss(0.0, 1.5, tc)) * (1 - ss(3.2, 4.6, tc))
        mix.place('cic', c * env, T.T_2024, 0.5, pan)
        mix.place('cic', brood_hit(f0, 1.0, 300 if s == 'L' else 301, voices=5), T.T_2024, 0.35, pan)
    mix.place('cic', A.lp(pharaoh(F_XIII, 1.6, seed=9), 2400), 56.4, 0.13, 0.2)     # one buzz left...
    # -- carve the call band out of the music + drums so the cicadas ride on top
    for bus in ('music', 'drums'):
        x = mix.buses[bus]
        mix.buses[bus] = x - 0.62 * A.bp(x, 880, 1750, 2)
    out = mix.master(gains={'cic': 1.0, 'music': 0.95, 'drums': 0.85, 'fx': 1.0, 'amb': 1.0},
                     reverb_buses={'music': 0.8, 'cic': 0.25, 'fx': 0.4},
                     gain_db_pts=[(0, 3.5), (3.8, 3.5), (4.4, 1.5), (8.0, 3.5), (22.5, 3.5), (23.2, 0.5), (46.5, 0.5), (51.0, 1.5), (54.0, 1.5), (56.5, 2.5), (60, 3.5)])
    A.write(os.path.join(HERE, 'audio.wav'), out)
    spans = [(0, 4), (4, 8), (8, 16), (16, 20), (20, 23.4), (23.4, 35), (35, 47), (47, 51), (51, 54), (54, 60)]
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')
    check_band(mix)


def check_band(mix):
    """calls vs music in the call band (980-1450 Hz), on the separate buses, at the race hits"""
    def band(x, a, b):
        y = A.bp(x.mean(1), 980, 1450, 4)
        return 20 * np.log10(np.sqrt((y[int(a * SR):int(b * SR)] ** 2).mean()) + 1e-12)
    cic = mix.buses['cic']
    mus = mix.buses['music'] + mix.buses['drums']
    diffs = []
    for e in T.EMERGE:
        if e['kind'] in ('race', 'joint'):
            diffs.append(band(cic, e['t'] + 0.05, e['t'] + 0.45) - band(mus, e['t'] + 0.05, e['t'] + 0.45))
    print(f'call band 980-1450 Hz, calls over music at race hits: mean {np.mean(diffs):+.1f} dB, min {np.min(diffs):+.1f} dB')
    on = A.band_level(os.path.join(HERE, 'audio.wav'), 980, 1450, [(e['t'] + 0.05, e['t'] + 0.4) for e in T.EMERGE if e['kind'] == 'race'])
    off = A.band_level(os.path.join(HERE, 'audio.wav'), 980, 1450, [(e['t'] - 0.35, e['t'] - 0.05) for e in T.EMERGE if e['kind'] == 'race'])
    d = [a[2] - b[2] for a, b in zip(on, off)]
    print(f'in the final mix, call band during a hit vs just before it: mean {np.mean(d):+.1f} dB, min {np.min(d):+.1f} dB')
    print('climax 51-54 call band vs music:', round(band(cic, 51.1, 53.9) - band(mus, 51.1, 53.9), 1), 'dB')


if __name__ == '__main__':
    main()
