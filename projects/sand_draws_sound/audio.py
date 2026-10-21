"""Soundtrack for Sand That Draws Sound.
- the speaker tone (exactly the frequency the sand is reacting to)
- sand rattle driven by how much sand is moving in the simulation
- desert ambience: wind, night insects, brazier fire, dawn birds
- Grieg's Morning Mood (1875, public domain), arranged here."""
import math
import numpy as np
from scipy.signal import butter, sosfilt, fftconvolve, lfilter
from scipy.io import wavfile
import timeline as T

SR = 48000
DUR = T.DUR
NS = int(SR * DUR)
D = np.load('drive.npz')
ACT = D['activity']
VFR = D['V']
rng = np.random.default_rng(12)


def tt(n):
    return np.arange(n) / SR


def buf():
    return np.zeros((NS + SR * 5, 2))


def place(bus, sig, t, gain=1.0, pan=0.0):
    i0 = int(round(t * SR))
    if sig.ndim == 1:
        a = (pan + 1) * math.pi / 4
        sig = np.stack([sig * math.cos(a), sig * math.sin(a)], 1)
    s0 = max(0, -i0); i0 = max(0, i0)
    n = min(len(sig) - s0, len(bus) - i0)
    if n > 0:
        bus[i0:i0 + n] += sig[s0:s0 + n] * gain


def sos(kind, f, order=2):
    return butter(order, f, kind, fs=SR, output='sos')


def hp(x, f, o=2): return sosfilt(sos('hp', f, o), x, axis=0)
def lp(x, f, o=2): return sosfilt(sos('lp', f, o), x, axis=0)
def bp(x, lo, hi, o=2): return sosfilt(sos('bp', [lo, hi], o), x, axis=0)


def env_adsr(n, a=0.01, r=0.05, sustain=1.0):
    e = np.ones(n) * sustain
    na, nr = int(a * SR), int(r * SR)
    if na:
        e[:na] = np.linspace(0, sustain, na)
    if nr and nr < n:
        e[-nr:] *= np.linspace(1, 0, nr)
    return e


def mtof(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def frame_curve(arr):
    return np.interp(tt(NS), np.arange(len(arr)) / T.FPS, arr)


# ------------------------------------------------------------ the speaker tone
def tone_track():
    """Continuous drive tone following the sim's script, phase-continuous."""
    f = np.zeros(NS)
    amp = np.zeros(NS)
    t = tt(NS)
    for nm, t0, t1 in T.HOOK:
        i0, i1 = int(t0 * SR), int(t1 * SR)
        f[i0:i1] = T.res_hz(nm)
        e = np.ones(i1 - i0)
        e[:int(0.03 * SR)] = np.linspace(0, 1, int(0.03 * SR))
        e[-int(0.25 * SR):] *= np.linspace(1, 0, int(0.25 * SR)) if nm == 'H2' else 1
        amp[i0:i1] = np.maximum(amp[i0:i1], e)
    # slow-mo: low throb in time with the waving plate
    i0, i1 = int(T.T_SLOW0 * SR), int(T.T_SLOW1 * SR)
    # sweep
    i0, i1 = int(T.T_SWEEP0 * SR), int(T.T_SWEEP1 * SR)
    fs = np.array([T.sweep_freq(x) for x in t[i0:i1:48]])
    f[i0:i1] = np.interp(t[i0:i1], t[i0:i1:48], fs)
    amp[i0:i1] = np.clip((t[i0:i1] - T.T_SWEEP0) / 0.3, 0, 1) * np.clip((T.T_SWEEP1 - t[i0:i1]) / 0.08, 0, 1)
    # sweep loudness: louder on resonances (the plate rings along)
    Vc = frame_curve(VFR)
    amp[i0:i1] *= 0.55 + 0.45 * np.clip((Vc[i0:i1] - 0.28) / 0.72, 0, 1)
    ph = 2 * np.pi * np.cumsum(f) / SR
    # equal-loudness taming of the high tones
    tame = np.where(f > 400, (np.maximum(f, 1) / 400) ** -0.42, 1.0)
    sig = (np.sin(ph) + 0.18 * np.sin(2 * ph) + 0.06 * np.sin(3 * ph)) * amp * tame
    return sig


def ding(f0, dur=1.6):
    """plate ringing: a few inharmonic partials"""
    n = int(dur * SR)
    tq = tt(n)
    s = np.zeros(n)
    for k, (r, a, d) in enumerate([(1.0, 1.0, 0.9), (2.32, 0.45, 0.45), (4.25, 0.22, 0.25), (6.6, 0.1, 0.12)]):
        fr = f0 * r
        if fr > 15000:
            continue
        s += a * np.sin(2 * np.pi * fr * tq + k) * np.exp(-tq / d)
    s[:48] *= np.linspace(0, 1, 48)
    return s


def blip(f0):
    n = int(0.12 * SR)
    tq = tt(n)
    return np.sin(2 * np.pi * f0 * tq) * np.exp(-tq / 0.03) + 0.4 * np.sin(2 * np.pi * 2 * f0 * tq) * np.exp(-tq / 0.015)


# ------------------------------------------------------------ sand rattle
def sand_rattle():
    act = frame_curve(ACT)
    # Poisson clicks, rate follows the activity
    rate = np.clip(act, 0, None) * 9000
    p = rate / SR
    clicks = (rng.uniform(0, 1, NS) < p).astype(float) * rng.uniform(0.3, 1.0, NS)
    L = bp(clicks, 2200, 9000)
    R = bp(np.roll(clicks, 37) * rng.uniform(0.6, 1.0, NS), 2200, 9000)
    hiss = bp(rng.normal(0, 1, NS), 3000, 10000) * act * 0.05
    return np.stack([L + hiss, R + hiss], 1)


# ------------------------------------------------------------ ambience
def ambience():
    t = tt(NS)
    out = np.zeros((NS, 2))
    # wind: lowpassed noise with slow gusts
    gust = 0.55 + 0.45 * np.sin(2 * np.pi * 0.07 * t) * np.sin(2 * np.pi * 0.031 * t + 1)
    for ch in range(2):
        w = lp(rng.normal(0, 1, NS), 500, 2) + 0.3 * bp(rng.normal(0, 1, NS), 600, 1800)
        out[:, ch] += w * gust * 0.11
    # night insects (fade out by dawn)
    night = 1 - np.clip((t - 22) / 14, 0, 1)
    for k, (fc, rate, pan) in enumerate([(4300, 18, -0.6), (5100, 23, 0.5), (3800, 15, 0.1)]):
        car = np.sin(2 * np.pi * fc * t)
        chirp = (np.sin(2 * np.pi * rate * t) > 0.3).astype(float)
        groups = (np.sin(2 * np.pi * (0.55 + 0.1 * k) * t + k) > -0.2).astype(float)
        sig = lp(car * chirp * groups, 9000) * night * 0.018
        a = (pan + 1) * math.pi / 4
        out[:, 0] += sig * math.cos(a); out[:, 1] += sig * math.sin(a)
    # brazier fire: crackle + low roar
    fire_k = np.where(t < 38, 1.0, np.clip(1 - (t - 38) / 10 * 0.6, 0.4, 1))
    for pan in (-0.55, 0.55):
        roar = lp(rng.normal(0, 1, NS), 300) * 0.06
        pops = (rng.uniform(0, 1, NS) < 18 / SR).astype(float) * rng.uniform(0.2, 1, NS)
        crack = bp(lfilter([1], [1, -0.97], pops), 900, 6000) * 0.35
        sig = (roar + crack) * fire_k
        a = (pan + 1) * math.pi / 4
        out[:, 0] += sig * math.cos(a); out[:, 1] += sig * math.sin(a)
    # dawn birds
    for k in range(26):
        tb = 43.8 + rng.uniform(0, 15)
        f0 = rng.uniform(2600, 4200)
        n = int(rng.uniform(0.06, 0.14) * SR)
        tq = tt(n)
        fr = f0 * (1 + 0.35 * np.sin(np.pi * tq / tq[-1]) * rng.choice([-1, 1]))
        s = np.sin(2 * np.pi * np.cumsum(fr) / SR) * np.sin(np.pi * tq / tq[-1]) ** 2
        for rep in range(rng.integers(1, 4)):
            place(out, s, tb + rep * 0.13, 0.05, rng.uniform(-0.7, 0.7))
    return out


# ------------------------------------------------------------ instruments
def flute(m, dur, vel=1.0):
    n = int((dur + 0.15) * SR)
    tq = tt(n)
    f = mtof(m) * (1 + 0.004 * np.sin(2 * np.pi * 5.2 * tq) * np.clip((tq - 0.15) / 0.2, 0, 1))
    ph = 2 * np.pi * np.cumsum(f) / SR
    s = np.sin(ph) + 0.16 * np.sin(2 * ph) + 0.05 * np.sin(3 * ph)
    breath = bp(rng.normal(0, 1, n), mtof(m) * 1.5, min(12000, mtof(m) * 4)) * 0.05
    e = np.ones(n)
    a = int(0.035 * SR); r = int(0.12 * SR); nd = int(dur * SR)
    e[:a] = np.linspace(0, 1, a)
    e[nd:] = np.linspace(1, 0, n - nd) if n > nd else 1
    return (s + breath) * e * vel


def pluck(m, dur=1.4):
    n = int(dur * SR)
    tq = tt(n)
    f = mtof(m)
    s = sum(np.sin(2 * np.pi * f * h * tq) * np.exp(-tq * (2.2 + h * 1.6)) / h for h in (1, 2, 3, 4))
    s[:40] *= np.linspace(0, 1, 40)
    return s


def pad(ms, dur, bright=1200, att=0.35):
    n = int((dur + 0.6) * SR)
    tq = tt(n)
    s = np.zeros(n)
    for m in ms:
        for d in (-0.004, 0.0, 0.0045):
            ph = (mtof(m) * (1 + d) * tq + rng.uniform()) % 1.0
            s += 2 * ph - 1
    s = lp(s / (3 * len(ms)), bright, 2)
    e = np.clip(tq / att, 0, 1) * np.clip((dur + 0.6 - tq) / 0.6, 0, 1)
    return s * e


def horn(m, dur):
    n = int((dur + 0.3) * SR)
    tq = tt(n)
    ph = (mtof(m) * tq) % 1.0
    s = lp(2 * ph - 1, 900, 2) + 0.5 * np.sin(2 * np.pi * mtof(m) * tq)
    e = np.clip(tq / 0.12, 0, 1) * np.clip((dur + 0.3 - tq) / 0.3, 0, 1)
    return s * e


def bass(m, dur):
    n = int((dur + 0.1) * SR)
    tq = tt(n)
    f = mtof(m)
    s = np.sin(2 * np.pi * f * tq) + 0.3 * (2 * np.abs(((f * tq) % 1) - 0.5) * 2 - 1)
    return s * np.clip(tq / 0.02, 0, 1) * np.exp(-tq / (dur * 1.5)) * np.clip((dur + 0.1 - tq) / 0.1, 0, 1)


def kick():
    n = int(0.45 * SR)
    tq = tt(n)
    f = 45 + 90 * np.exp(-tq / 0.04)
    return np.tanh(1.5 * np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-tq / 0.25))


def shaker():
    n = int(0.08 * SR)
    tq = tt(n)
    return hp(rng.normal(0, 1, n), 6000) * np.exp(-tq / 0.02)


def timp(m=40, dur=1.2):
    n = int(dur * SR)
    tq = tt(n)
    f = mtof(m) * (1 + 0.06 * np.exp(-tq / 0.05))
    return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-tq / 0.5) + lp(rng.normal(0, 1, n), 400) * np.exp(-tq / 0.04) * 0.4


def crash(dur=2.5):
    n = int(dur * SR)
    tq = tt(n)
    return hp(rng.normal(0, 1, n), 4000) * np.exp(-tq / (dur / 3))


def whoosh(dur, lo=300, hi=3000):
    n = int(dur * SR)
    x = rng.normal(0, 1, n)
    out = np.zeros(n)
    zi = None
    for k in range(0, n, 1024):
        u = k / n
        fc = lo * (hi / lo) ** u
        so = sos('bp', [fc * 0.7, min(fc * 1.4, SR / 2 - 100)])
        if zi is None:
            zi = np.zeros((so.shape[0], 2))
        y, zi = sosfilt(so, x[k:k + 1024], zi=zi)
        out[k:k + 1024] = y
    return out / (np.abs(out).max() + 1e-9) * np.sin(np.linspace(0, np.pi, n)) ** 2


def reverb(x, dur=2.6, decay=0.7, seed=3):
    r = np.random.default_rng(seed)
    n = int(dur * SR)
    tq = tt(n)
    ir = np.stack([r.normal(0, 1, n), r.normal(0, 1, n)], 1) * np.exp(-tq / decay)[:, None]
    ir = lp(ir, 5000)
    ir[:int(0.02 * SR)] = 0
    ir /= np.sqrt((ir ** 2).sum(0)).max()
    return np.stack([fftconvolve(x[:, 0], ir[:, 0])[:len(x)], fftconvolve(x[:, 1], ir[:, 1])[:len(x)]], 1)


# ------------------------------------------------------------ main
def main():
    tone = buf(); fx = buf(); mus = buf(); amb = buf(); sand = buf()
    tsig = tone_track()
    tone[:NS, 0] += tsig * 0.16
    tone[:NS, 1] += tsig * 0.16
    # hook: plate rings when each tone hits
    for nm, t0, t1 in T.HOOK:
        place(fx, ding(T.res_hz(nm) * 0.5, 1.8), t0, 0.07)
        place(fx, timp(40, 1.0) * 0.6, t0, 0.18)
    # sweep: a ring on every resonance, a blip as each pattern is collected
    for k, (nm, ta, tl) in enumerate(T.SWEEP):
        place(fx, ding(min(T.res_hz(nm), 1800) * (1 if T.res_hz(nm) < 1800 else 0.5), 1.4), ta, 0.06)
        place(fx, blip(mtof(76 + [0, 2, 4, 7, 9, 12, 14, 16, 19, 21, 24, 26][k])), ta + 0.25, 0.10, 0.2)
        place(fx, kick() * 0.5, ta, 0.20)
    # slow motion: a deep throb in time with the waving plate
    n = int((T.T_SLOW1 - T.T_SLOW0) * SR)
    tq = tt(n)
    throb = np.sin(2 * np.pi * 55 * tq + 3 * np.sin(2 * np.pi * 0.95 * tq)) * (0.5 + 0.5 * np.abs(np.sin(2 * np.pi * 0.95 * tq)))
    throb *= np.clip(tq / 0.5, 0, 1) * np.clip((T.T_SLOW1 - T.T_SLOW0 - tq) / 0.5, 0, 1)
    throb *= 0.6 + 0.4 * np.clip((tq - (T.T_SLOW_FLING - T.T_SLOW0)) / 0.5, 0, 1)
    place(fx, throb, T.T_SLOW0, 0.22)
    place(fx, whoosh(0.7, 2000, 300), T.T_SLOW0 - 0.3, 0.12)
    # reveal whoosh + shooting star twinkle + a soft chord
    place(fx, whoosh(1.8, 300, 2500), 4.1, 0.14)
    for k, m in enumerate([88, 92, 95, 100]):
        place(fx, pluck(m, 1.2), 5.0 + k * 0.07, 0.05, 0.4)
    place(mus, pad([52, 59, 64, 66, 71], 3.0, 900, 0.8), 4.3, 0.10)
    # sweep underscore: E drone that opens up as the patterns get finer
    sw0, sw1 = T.T_SWEEP0, T.T_SWEEP1
    dr = pad([40, 47, 52], sw1 - sw0 + 0.4, 700, 1.5)
    place(mus, dr, sw0, 0.12)
    dr2 = pad([59, 64, 66, 71], sw1 - 22.0, 1600, 2.5)
    place(mus, dr2, 22.0, 0.07)
    # scramble + riser into the song
    place(fx, whoosh(0.9, 400, 6000), T.T_SCRAMBLE1 - 0.05, 0.18)
    place(fx, timp(40, 1.4), T.T_SCRAMBLE1, 0.3)
    n = int((T.T_SONG - 33.6) * SR)
    rise = bp(rng.normal(0, 1, n), 800, 6000) * np.linspace(0, 1, n) ** 3
    place(fx, rise, 33.6, 0.05)
    place(fx, whoosh(0.9, 400, 6000), T.T_SCRAMBLE2 - 0.05, 0.14)

    # ------------- Morning Mood (Grieg, 1875), arranged
    E8 = T.EIGHTH
    HALF = 3 * E8
    chords = [  # per half-bar: (bass midi, triad)
        (40, [52, 56, 59]), (40, [52, 56, 59]), (40, [52, 56, 59]), (40, [52, 56, 59]),
        (40, [52, 56, 59]), (37, [49, 52, 56]), (40, [52, 56, 59]), (42, [54, 59, 64]),
    ]
    for rep in range(2):
        big = rep == 1
        for k, (bm, tri) in enumerate(chords):
            t0 = T.T_SONG + rep * 4 * T.BAR + k * HALF
            # harp arpeggio: 3 eighths per half bar
            arp = [tri[0] - 12, tri[1], tri[2]] if k % 2 == 0 else [tri[2] + 12, tri[1] + 12, tri[2]]
            for j, m in enumerate(arp):
                place(mus, pluck(m, 1.2), t0 + j * E8, 0.05 if not big else 0.045, -0.35 + 0.35 * j)
            place(mus, bass(bm, HALF * 0.95), t0, 0.16 if not big else 0.22)
            place(mus, pad(tri + [tri[0] + 12], HALF, 1300 if not big else 2400, 0.25), t0, 0.05 if not big else 0.08)
            if big:
                place(mus, kick() * 0.7, t0, 0.32)
                for j in range(3):
                    place(mus, shaker(), t0 + j * E8, 0.04, 0.3)
                place(mus, horn(tri[1] + 12, HALF * 0.95), t0, 0.035, -0.3)
        for t0, t1, p in T.SONG[rep * 25:(rep + 1) * 25] if False else []:
            pass
    for t0, t1, p in T.SONG:
        big = t0 >= T.T_SUNRISE
        m = T.NOTE_MIDI[p]
        place(mus, flute(m, t1 - t0), t0, 0.10 if not big else 0.11, 0.05)
        if big:
            place(mus, flute(m - 12, t1 - t0) * 0.8, t0, 0.07, -0.25)   # octave below: strings
            place(mus, pad([m - 12], t1 - t0, 2600, 0.05), t0, 0.05, 0.25)
    # into the sunrise: timpani roll + cymbal swell
    for j in range(16):
        place(mus, timp(40, 0.5), T.T_SUNRISE - 1.0 + j * 1.0 / 16, 0.05 + 0.12 * j / 15)
    sw = crash(2.0)[::-1]
    place(mus, sw, T.T_SUNRISE - 2.0, 0.10)
    place(mus, crash(3.0), T.T_SUNRISE, 0.16)
    place(mus, timp(40, 1.6), T.T_SUNRISE, 0.4)
    # final chord
    tf = T.T_FINAL
    place(mus, pad([40, 52, 56, 59, 64, 68, 71, 76], 4.2, 2400, 0.08), tf, 0.11)
    place(mus, flute(76, 4.0), tf, 0.10)
    place(mus, flute(64, 4.0) * 0.8, tf, 0.07, -0.25)
    for j, m in enumerate([40, 47, 52, 56, 59, 64, 68, 71, 76, 80, 83, 88]):
        place(mus, pluck(m, 2.5), tf + j * 0.05, 0.05, -0.5 + j / 11)
    place(mus, bass(28, 3.5), tf, 0.3)
    place(mus, kick(), tf, 0.4)
    place(mus, crash(3.5), tf, 0.16)
    place(mus, timp(40, 2.0), tf, 0.35)
    # coda: the opening motif once more, softly, as the sun is up
    for j, m in enumerate([83, 80, 78, 76]):
        place(mus, pluck(m, 2.2), 56.6 + j * 0.42, 0.06, 0.15)
    place(mus, pad([52, 59, 64, 68], 3.2, 1500, 0.6), 56.4, 0.06)
    # sand + ambience
    sand[:NS] += sand_rattle() * 0.05          # sand grains: just a faint whisper under the music
    amb[:NS] += ambience()

    # section levels (dB) for the ambience: lift it in the reveal and the outro
    pts = [(0, -6), (4.0, -6), (4.6, 2), (7.2, 0), (8, -5), (33, -5), (34, 0), (35.4, -6), (51.4, -6), (54, 0), (60, 0)]
    g = 10 ** (np.interp(tt(NS), [p[0] for p in pts], [p[1] for p in pts]) / 20)
    amb[:NS] *= g[:, None]

    dry = tone + fx + mus + sand + amb
    wet = reverb(mus + fx * 0.6 + tone * 0.4, 2.6, 0.75)
    mix = (dry + wet * 0.22)[:NS]
    pts = [(0, 3), (4.2, 3), (4.5, 7), (7.0, 7), (7.4, 2), (14.5, 1), (21.5, 1), (22.5, 3.5), (32.6, 3.5),
           (33.0, 5), (35.3, 5), (35.4, 1.5), (43.3, 1.5), (43.4, 0), (51.4, 0), (55.5, 0), (56.4, 6), (60, 6)]
    gdb = np.interp(tt(NS), [p[0] for p in pts], [p[1] for p in pts])
    mix *= (10 ** (gdb / 20))[:, None]
    # gentle bus compression, then ceiling
    env = np.sqrt(lfilter([0.002], [1, -0.998], (mix ** 2).mean(1)))
    thr = 0.16
    gain = np.where(env > thr, (thr / np.maximum(env, 1e-9)) ** 0.45, 1.0)
    mix *= gain[:, None]
    mix *= 0.89 / np.abs(mix).max()
    mix = np.tanh(mix * 1.1) / np.tanh(1.1)
    mix *= 0.89 / np.abs(mix).max()
    fd = int(0.004 * SR)
    mix[:fd] *= np.linspace(0, 1, fd)[:, None]
    mix[-fd:] *= np.linspace(1, 0, fd)[:, None]
    wavfile.write('audio.wav', SR, (mix * 32767).astype(np.int16))
    for a, b in ((0, 4.2), (4.2, 7.2), (7.2, 14.5), (14.5, 22), (22, 32.6), (32.6, 35.4), (35.4, 43.4), (43.4, 51.4), (51.4, 56), (56, 60)):
        seg = mix[int(a * SR):int(b * SR)]
        print(f'{a:5.1f}-{b:5.1f}s rms {20 * np.log10(np.sqrt((seg ** 2).mean()) + 1e-9):6.1f} dBFS peak {20 * np.log10(np.abs(seg).max() + 1e-9):5.1f}')


if __name__ == '__main__':
    main()
