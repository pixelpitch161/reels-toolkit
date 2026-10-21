"""Audio building blocks (numpy/scipy, 48 kHz stereo). Every sound is synthesised; music
arrangements must be original or public domain (pre-1929 compositions are fine to arrange).

    from reelkit import audio as A
    mix = A.Mix(dur=60)
    mix.place('drums', A.kick(), t=1.0, gain=0.5)
    mix.place('fx', A.whoosh(0.6), t=4.0, gain=0.2, pan=-0.3)
    out = mix.master(reverb_buses={'music': 0.22})
    A.write('audio.wav', out)

Keep the "science" sound (the chirp, the tone, the tick) clearly on top of the music:
check it with band_level() - in the cricket video the chirps sat 10-21 dB above the music in
their 2.2-3.8 kHz band. Keep incidental textures (e.g. sand rattle) very low or out."""
import math
import numpy as np
from scipy.signal import butter, sosfilt, fftconvolve, lfilter
from scipy.io import wavfile

SR = 48000


def tt(n):
    return np.arange(n) / SR


def mtof(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def sos(kind, f, o=2):
    return butter(o, f, kind, fs=SR, output='sos')


def hp(x, f, o=2): return sosfilt(sos('hp', f, o), x, axis=0)
def lp(x, f, o=2): return sosfilt(sos('lp', f, o), x, axis=0)
def bp(x, lo, hi, o=2): return sosfilt(sos('bp', [lo, hi], o), x, axis=0)


def adsr(n, a=0.005, d=0.1, s=0.6, r=0.05):
    e = np.ones(n) * s
    na, nd, nr = int(a * SR), int(d * SR), int(r * SR)
    na = min(na, n); e[:na] = np.linspace(0, 1, na)
    nd = min(nd, n - na); e[na:na + nd] = np.linspace(1, s, nd)
    if nr > 0 and nr < n:
        e[-nr:] *= np.linspace(1, 0, nr)
    return e


def expenv(n, decay, attack=0.002):
    t = tt(n)
    return np.clip(t / max(attack, 1e-4), 0, 1) * np.exp(-t / decay)


# ------------------------------------------------------------------ oscillators
def saw(f, n, det=0.0, voices=1):
    t = tt(n)
    out = np.zeros(n)
    for v in range(voices):
        fv = f * (1 + det * (v - (voices - 1) / 2))
        out += 2 * ((t * fv + v * 0.37) % 1.0) - 1
    return out / voices


def square(f, n, duty=0.5):
    return np.where((tt(n) * f) % 1.0 < duty, 1.0, -1.0)


def tri(f, n):
    return 2 * np.abs(2 * ((tt(n) * f) % 1.0) - 1) - 1


# ------------------------------------------------------------------ drums
def kick(punch=1.0, dec=0.32, f0=150, f1=46):
    n = int(0.5 * SR)
    t = tt(n)
    f = f1 + (f0 - f1) * np.exp(-t / 0.04)
    ph = 2 * np.pi * np.cumsum(f) / SR
    return np.sin(ph) * np.exp(-t / dec) * punch + 0.3 * np.exp(-t / 0.004) * np.random.default_rng(1).normal(0, 1, n)


def snare(dec=0.14):
    n = int(0.35 * SR)
    t = tt(n)
    r = np.random.default_rng(2)
    return bp(r.normal(0, 1, n), 1500, 8000) * np.exp(-t / dec) * 0.7 + np.sin(2 * np.pi * 190 * t) * np.exp(-t / 0.05) * 0.5


def clap():
    n = int(0.3 * SR)
    t = tt(n)
    r = np.random.default_rng(3)
    env = sum(np.exp(-np.clip(t - d, 0, None) / 0.008) * (t >= d) for d in (0, 0.011, 0.022)) + np.exp(-t / 0.09) * 0.6
    return bp(r.normal(0, 1, n), 900, 5000) * env * 0.5


def hat(dec=0.035, open_=False):
    n = int((0.4 if open_ else 0.12) * SR)
    t = tt(n)
    return hp(np.random.default_rng(4).normal(0, 1, n), 7000) * np.exp(-t / (0.18 if open_ else dec)) * 0.5


def crash(dur=2.0):
    n = int(dur * SR)
    t = tt(n)
    return hp(np.random.default_rng(5).normal(0, 1, n), 4000) * np.exp(-t / (dur * 0.35)) * 0.5


def whoosh(dur, lo=300, hi=5000, rise=True, seed=6):
    n = int(dur * SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    out = np.zeros(n)
    nb = 24
    for k in range(nb):
        u = (k + 0.5) / nb
        fc = lo * (hi / lo) ** (u if rise else 1 - u)
        a, b = int(k * n / nb), int((k + 1) * n / nb)
        out[a:b] = bp(x[max(0, a - 2048):b], fc * 0.8, min(fc * 1.25, SR / 2 - 100))[-(b - a):]
    return out * np.sin(np.linspace(0, np.pi, n)) ** 2


# ------------------------------------------------------------------ tonal
def pluck(m, dur=1.2):
    n = int(dur * SR)
    f = mtof(m)
    t = tt(n)
    return (np.sin(2 * np.pi * f * t) + 0.4 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.15)) * expenv(n, dur * 0.3)


def pad(ms, dur, cut=1600, att=0.3):
    n = int(dur * SR)
    x = sum(saw(mtof(m), n, det=0.006, voices=3) for m in ms) / max(1, len(ms))
    return lp(x, cut) * adsr(n, att, 0.2, 0.8, min(0.6, dur * 0.4))


def bass(m, dur):
    n = int(dur * SR)
    t = tt(n)
    return np.sin(2 * np.pi * mtof(m) * t) * adsr(n, 0.004, 0.1, 0.8, 0.04)


def chime(m, dur=1.2):
    n = int(dur * SR)
    t = tt(n)
    f = mtof(m)
    return (np.sin(2 * np.pi * f * t) + 0.3 * np.sin(2 * np.pi * f * 2.76 * t)) * expenv(n, dur * 0.35)


# ------------------------------------------------------------------ mixing
def reverb(x, dur=2.4, decay=0.6, seed=3):
    r = np.random.default_rng(seed)
    n = int(dur * SR)
    t = tt(n)
    ir = np.stack([r.normal(0, 1, n), r.normal(0, 1, n)], 1) * np.exp(-t / decay)[:, None]
    ir = lp(ir, 5000)
    ir[:int(0.02 * SR)] = 0
    ir /= np.sqrt((ir ** 2).sum(0)).max()
    return np.stack([fftconvolve(x[:, 0], ir[:, 0])[:len(x)], fftconvolve(x[:, 1], ir[:, 1])[:len(x)]], 1)


def tape_stop(x, t0, length=1.5):
    """pitch-down-to-silence effect starting at t0 (s) on a stereo buffer"""
    i0 = int(t0 * SR)
    n = int(length * SR)
    seg = x[i0:i0 + n * 2]
    rate = np.linspace(1, 0, n) ** 1.5
    pos = np.cumsum(rate)
    out = np.stack([np.interp(pos, np.arange(len(seg)), seg[:, c]) for c in range(2)], 1)
    y = x.copy()
    y[i0:i0 + n] = out * np.linspace(1, 0, n)[:, None] ** 0.5
    y[i0 + n:] = 0
    return y


class Mix:
    """named stereo buses; place sounds at times; master to a limited, normalised stereo array"""
    def __init__(self, dur=60.0):
        self.dur = dur
        self.NS = int(SR * dur)
        self.buses = {}

    def bus(self, name):
        if name not in self.buses:
            self.buses[name] = np.zeros((self.NS + SR * 6, 2))
        return self.buses[name]

    def place(self, name, sig, t, gain=1.0, pan=0.0):
        b = self.bus(name)
        i0 = int(round(t * SR))
        if sig.ndim == 1:
            a = (pan + 1) * math.pi / 4
            sig = np.stack([sig * math.cos(a), sig * math.sin(a)], 1)
        s0 = max(0, -i0)
        i0 = max(0, i0)
        n = min(len(sig) - s0, len(b) - i0)
        if n > 0:
            b[i0:i0 + n] += sig[s0:s0 + n] * gain

    def master(self, gains=None, reverb_buses=None, gain_db_pts=None, thr=0.16, ceiling=0.89):
        """gains: {bus: linear}; reverb_buses: {bus: send}; gain_db_pts: [(t, dB)] section automation"""
        gains = gains or {}
        dry = sum(b * gains.get(k, 1.0) for k, b in self.buses.items())
        if reverb_buses:
            send = sum(self.buses[k] * v for k, v in reverb_buses.items() if k in self.buses)
            dry = dry + reverb(send) * 0.22
        mix = dry[:self.NS]
        if gain_db_pts:
            g = np.interp(tt(self.NS), [p[0] for p in gain_db_pts], [p[1] for p in gain_db_pts])
            mix = mix * (10 ** (g / 20))[:, None]
        env = np.sqrt(lfilter([0.002], [1, -0.998], (mix ** 2).mean(1)))
        gain = np.where(env > thr, (thr / np.maximum(env, 1e-9)) ** 0.45, 1.0)
        mix = mix * gain[:, None]
        mix *= ceiling / (np.abs(mix).max() + 1e-9)
        mix = np.tanh(mix * 1.1) / np.tanh(1.1)
        mix *= ceiling / (np.abs(mix).max() + 1e-9)
        fd = int(0.004 * SR)
        mix[:fd] *= np.linspace(0, 1, fd)[:, None]
        mix[-fd:] *= np.linspace(1, 0, fd)[:, None]
        return mix


def write(path, mix):
    wavfile.write(path, SR, (np.clip(mix, -1, 1) * 32767).astype(np.int16))


def band_level(path, lo, hi, spans):
    """RMS dB in a frequency band over time spans [(a, b)] - to check a sound sits where it should"""
    sr, x = wavfile.read(path)
    x = x.astype(float).mean(1) / 32768
    y = sosfilt(butter(4, [lo, hi], 'bp', fs=sr, output='sos'), x)
    return [(a, b, 20 * np.log10(np.sqrt((y[int(a * sr):int(b * sr)] ** 2).mean()) + 1e-12)) for a, b in spans]


def section_levels(path, spans):
    sr, x = wavfile.read(path)
    x = x.astype(float) / 32768
    out = []
    for a, b in spans:
        seg = x[int(a * sr):int(b * sr)]
        out.append((a, b, 20 * np.log10(np.sqrt((seg ** 2).mean()) + 1e-12), 20 * np.log10(np.abs(seg).max() + 1e-12)))
    return out
