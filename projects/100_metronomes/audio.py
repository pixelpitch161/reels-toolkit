"""Soundtrack: every metronome tick comes from the simulation, the music
layers lock onto the crowd's beat as it forms, and the drop is an original
120 BPM chiptune piece in A minor."""
import math
import numpy as np
from scipy.signal import lfilter, butter, sosfilt, fftconvolve
from scipy.io import wavfile
from sim import FPS, N, HOLDOUT, T_UNLOCK, COLS
import render as RD

SR = 48000
DUR = 60.0
NS = int(SR * DUR)
TL = np.load('timeline.npz')
R_FR = TL['R']
COUNT = TL['count']
T_DROP = float(TL['t_drop'])
T_HOLD_IN = float(TL['t_hold_in'])
BEAT = 0.5
T_END = T_DROP + 32 * BEAT
rng = np.random.default_rng(5)


def R_at(t):
    f = min(len(R_FR) - 1, max(0, int(t * FPS)))
    return float(R_FR[f])


def mtof(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def buf():
    return np.zeros((NS + SR * 4, 2))


def place(bus, sig, t, gain=1.0, pan=0.0):
    """Add mono or stereo signal at time t with equal-power pan."""
    i0 = int(round(t * SR))
    if i0 >= len(bus) or i0 + len(sig) <= 0:
        return
    if sig.ndim == 1:
        a = (pan + 1) * math.pi / 4
        st = np.stack([sig * math.cos(a), sig * math.sin(a)], 1)
    else:
        st = sig
    s0 = max(0, -i0)
    i0c = max(0, i0)
    n = min(len(st) - s0, len(bus) - i0c)
    bus[i0c:i0c + n] += st[s0:s0 + n] * gain


def tt(n):
    return np.arange(n) / SR


def expenv(n, decay, attack=0.0):
    t = tt(n)
    e = np.exp(-t / decay)
    if attack > 0:
        e *= np.clip(t / attack, 0, 1)
    return e


def hp(x, fc, order=2):
    return sosfilt(butter(order, fc, 'hp', fs=SR, output='sos'), x, axis=0)


def lp(x, fc, order=2):
    return sosfilt(butter(order, fc, 'lp', fs=SR, output='sos'), x, axis=0)


def bp(x, lo, hi, order=2):
    return sosfilt(butter(order, [lo, hi], 'bp', fs=SR, output='sos'), x, axis=0)


def pulse(freq, n, duty=0.5, vib=0.0, vib_rate=5.5):
    t = tt(n)
    f = freq * (1 + vib * np.sin(2 * np.pi * vib_rate * t) * np.clip((t - 0.08) / 0.15, 0, 1))
    ph = np.cumsum(f) / SR
    x = np.where((ph % 1.0) < duty, 1.0, -1.0)
    return lp(x, min(9000, freq * 14))


def saw(freq, n, detune=0.0):
    t = tt(n)
    out = np.zeros(n)
    for d in ((-detune, 0, detune) if detune else (0,)):
        ph = (freq * (1 + d)) * t + rng.uniform()
        out += 2 * (ph % 1.0) - 1
    return out / (3 if detune else 1)


def tri(freq, n):
    ph = (freq * tt(n)) % 1.0
    return 4 * np.abs(ph - 0.5) - 1


# ------------------------------------------------------------ sound designs
def tick_sound(k=1.0, tock=False):
    n = int(0.06 * SR)
    t = tt(n)
    f1, f2 = 2350 * k * (0.94 if tock else 1), 1180 * k * (0.96 if tock else 1)
    s = 0.9 * np.sin(2 * np.pi * f1 * t) * np.exp(-t / 0.011)
    s += 0.6 * np.sin(2 * np.pi * f2 * t) * np.exp(-t / 0.022)
    s += 0.25 * np.sin(2 * np.pi * f1 * 2.71 * t) * np.exp(-t / 0.004)
    nz = hp(rng.normal(0, 1, n), 2500) * np.exp(-t / 0.0025) * 0.8
    s += nz
    s[:24] *= np.linspace(0, 1, 24)
    return s / np.abs(s).max()


def pop_sound(freq):
    n = int(0.16 * SR)
    t = tt(n)
    s = np.sin(2 * np.pi * freq * t + 0.6 * np.sin(2 * np.pi * freq * 2 * t) * np.exp(-t / 0.03))
    s += 0.3 * tri(freq * 2, n)
    return s * expenv(n, 0.05, 0.003)


def kick(n_dec=0.32, f0=150, f1=46):
    n = int(0.5 * SR)
    t = tt(n)
    f = f1 + (f0 - f1) * np.exp(-t / 0.035)
    ph = 2 * np.pi * np.cumsum(f) / SR
    s = np.sin(ph) * np.exp(-t / n_dec)
    s += hp(rng.normal(0, 1, n), 3000) * np.exp(-t / 0.004) * 0.35
    return np.tanh(s * 1.6) / np.tanh(1.6)


def snare():
    n = int(0.3 * SR)
    t = tt(n)
    nz = bp(rng.normal(0, 1, n), 900, 7500) * np.exp(-t / 0.11)
    tone = np.sin(2 * np.pi * 196 * t) * np.exp(-t / 0.05) * 0.6
    s = nz + tone
    return s / np.abs(s).max()


def clap():
    n = int(0.35 * SR)
    t = tt(n)
    env = np.zeros(n)
    for d in (0.0, 0.011, 0.022):
        i = int(d * SR)
        env[i:] += np.exp(-(t[:n - i]) / (0.008 if d < 0.02 else 0.12))
    s = bp(rng.normal(0, 1, n), 1000, 5000) * env
    return s / np.abs(s).max()


def hat(dec=0.035):
    n = int(max(0.08, dec * 6) * SR)
    t = tt(n)
    s = hp(rng.normal(0, 1, n), 7500, 4) * np.exp(-t / dec)
    return s / np.abs(s).max()


def crash(dec=1.6):
    n = int(dec * 3 * SR)
    t = tt(n)
    s = hp(rng.normal(0, 1, n), 4500) * np.exp(-t / dec)
    for fr in (3150, 4430, 5820, 7310):
        s += 0.15 * np.sin(2 * np.pi * fr * t + rng.uniform(0, 6)) * np.exp(-t / (dec * 0.6))
    return s / np.abs(s).max()


def sub_boom(dur=1.6):
    n = int(dur * SR)
    t = tt(n)
    f = 34 + 30 * np.exp(-t / 0.15)
    return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.55)


def noise_sweep(dur, lo, hi, rise=True):
    n = int(dur * SR)
    x = rng.normal(0, 1, n)
    out = np.zeros(n)
    blk = 1024
    zi = None
    for k in range(0, n, blk):
        u = k / n
        fc = lo * (hi / lo) ** (u if rise else 1 - u)
        sos = butter(2, [fc * 0.7, min(fc * 1.4, SR / 2 - 100)], 'bp', fs=SR, output='sos')
        seg = x[k:k + blk]
        if zi is None:
            zi = np.zeros((sos.shape[0], 2))
        y, zi = sosfilt(sos, seg, zi=zi)
        out[k:k + blk] = y
    return out / (np.abs(out).max() + 1e-9)


def reverb_ir(dur=1.8, decay=0.5, seed=1):
    r = np.random.default_rng(seed)
    n = int(dur * SR)
    t = tt(n)
    ir = np.stack([r.normal(0, 1, n), r.normal(0, 1, n)], 1) * np.exp(-t / decay)[:, None]
    ir = lp(ir, 6000)
    ir[: int(0.012 * SR)] = 0
    return ir / np.sqrt((ir ** 2).sum(0)).max()


def add_reverb(bus, mix, dur=1.8, decay=0.5):
    ir = reverb_ir(dur, decay)
    wet = np.stack([fftconvolve(bus[:, 0], ir[:, 0])[:len(bus)],
                    fftconvolve(bus[:, 1], ir[:, 1])[:len(bus)]], 1)
    return bus + wet * mix


# ------------------------------------------------------------ crowd beat grid
def crowd_beats():
    th = TL['theta']
    z = np.exp(1j * th).mean(1)
    un = np.unwrap(np.angle(z))
    k = np.floor((un - np.pi / 2) / np.pi)
    out = []
    for f in range(1, len(un)):
        if k[f] > k[f - 1]:
            target = k[f] * np.pi + np.pi / 2
            fr = (target - un[f - 1]) / (un[f] - un[f - 1])
            out.append((f - 1 + fr) / FPS)
    return np.array(out)


def main():
    ticks_bus = buf()
    sfx = buf()
    music = buf()
    pad_bus = buf()
    drums = buf()

    # ---------------- metronome ticks (straight from the simulation)
    variants = {}
    for v in range(8):
        k = 0.9 + 0.025 * v
        variants[(v, 0)] = tick_sound(k, False)
        variants[(v, 1)] = tick_sound(k, True)
    hold_tick = tick_sound(1.14, False)
    for i in range(N):
        ticks = TL[f'tick_{i}']
        v = (i * 37) % 8
        pan = float(np.clip(RD.MX[i] / 6.0, -0.8, 0.8))
        for n_, t in enumerate(ticks):
            if t < RD.APPEAR[i] + 0.03 or t >= DUR:
                continue
            p = RD.presence(i, t)
            if p <= 0.01:
                continue
            R = R_at(t)
            g = 0.15 / (1 + 3.2 * R * R) * p
            snd = variants[(v, n_ % 2)]
            if i == HOLDOUT:
                if 31.0 <= t < T_DROP:
                    g, snd = 0.42, hold_tick           # spotlight the rebel
                elif t < 1.6:
                    g = 0.5                            # the lone opener
                elif t >= T_END:
                    g = max(g, 0.34)                   # alone again at the end
            place(ticks_bus, snd, t, g, pan)

    # ---------------- pops as they appear (an ascending pentatonic run)
    penta = [0, 2, 4, 7, 9]
    order = np.argsort(RD.APPEAR)
    for j, i in enumerate(order):
        tA = max(0.02, RD.APPEAR[i])
        deg = j if j < 10 else 10 + (j - 10) // 3
        midi = 69 + 12 * (deg // 5) + penta[deg % 5]
        midi = min(midi, 69 + 36)
        g = 0.22 if j < 10 else 0.10
        place(sfx, pop_sound(mtof(midi)), tA, g, float(np.clip(RD.MX[i] / 6, -0.7, 0.7)))

    # camera whooshes on the big pull-backs
    for t0, d in ((1.45, 1.5), (2.95, 2.0)):
        w = noise_sweep(d, 400, 3500) * np.sin(np.linspace(0, np.pi, int(d * SR))) ** 2
        place(sfx, w, t0, 0.10, 0)

    # ---------------- the lift: whoosh up, cans roll in, clunk
    w = noise_sweep(0.95, 300, 2500) * np.sin(np.linspace(0, np.pi, int(0.95 * SR)))
    place(sfx, w, 10.6, 0.16)
    n = int(1.1 * SR)
    roll = lp(rng.normal(0, 1, n), 220) * (0.6 + 0.4 * np.sin(2 * np.pi * 13 * tt(n)))
    roll *= np.sin(np.linspace(0, np.pi, n)) * 3
    place(sfx, np.stack([roll, roll * 0.3], 1), 10.9, 0.5)
    place(sfx, np.stack([roll * 0.3, roll], 1), 10.95, 0.5)
    thud = kick(0.18, 110, 55)
    place(sfx, thud, T_UNLOCK, 0.55)
    knock = tick_sound(0.45) * 0.9
    place(sfx, knock, T_UNLOCK + 0.005, 0.35)
    place(sfx, knock, T_UNLOCK + 0.11, 0.12)

    # ---------------- rolling rumble that follows the board's sway
    dx = np.array([RD.board_state(f, f / FPS)[1] for f in range(int(DUR * FPS))])
    vel = np.abs(np.gradient(dx)) * FPS
    env = np.interp(tt(NS), np.arange(len(vel)) / FPS, vel)
    rum = lp(rng.normal(0, 1, NS), 160, 4) * np.clip(env / 0.6, 0, 1.2)
    sfx[:NS, 0] += rum * 0.55
    sfx[:NS, 1] += rum * 0.55

    # ---------------- pad: a dissonant cluster that resolves as R rises
    t_all = tt(NS)
    def pad_voice(midis, n0, n1, amp_curve):
        seg = np.zeros(n1 - n0)
        for m in midis:
            seg += saw(mtof(m), n1 - n0, detune=0.004)
        seg = lp(seg, 900, 2)
        return seg * amp_curve
    n0, n1 = int(5.0 * SR), int(T_DROP * SR)
    Rcurve = np.interp(t_all[n0:n1], np.arange(len(R_FR)) / FPS, R_FR)
    fade_in = np.clip((t_all[n0:n1] - 5.0) / 2.0, 0, 1)
    tension = pad_voice([45, 46, 52, 58], n0, n1, (1 - np.clip((Rcurve - 0.2) / 0.6, 0, 1)) * fade_in)
    resolve = pad_voice([45, 52, 57, 60, 64], n0, n1, np.clip((Rcurve - 0.2) / 0.6, 0, 1) * fade_in)
    pre = (tension + resolve) * 0.05
    # duck the pad slightly during the holdout section and swell into the drop
    pad_bus[n0:n1, 0] += pre
    pad_bus[n0:n1, 1] += pre

    # ---------------- layers that lock onto the crowd's beat as it forms
    beats = crowd_beats()
    K = kick(0.22, 120, 48)
    S = hat(0.03)
    for b, tb in enumerate(beats):
        if tb < 18.0 or tb >= 31.3:
            continue
        R = R_at(tb)
        if R > 0.45:
            place(drums, lp(K, 900), tb, 0.55 * min(1, (R - 0.45) / 0.25))
        if R > 0.72:
            nb = beats[b + 1] if b + 1 < len(beats) else tb + 0.5
            place(drums, S, (tb + nb) / 2, 0.16 * min(1, (R - 0.72) / 0.15), 0.25)
        if R > 0.88:
            g = 0.16 * min(1, (R - 0.88) / 0.08)
            nb = beats[b + 1] if b + 1 < len(beats) else tb + 0.5
            for k_, m in ((0, 33), (1, 45)):
                tn = tb + k_ * (nb - tb) / 2
                nsmp = int(0.22 * SR)
                note = (pulse(mtof(m), nsmp, 0.5) * 0.6 + np.sin(2 * np.pi * mtof(m) * tt(nsmp))) * expenv(nsmp, 0.12, 0.004)
                place(music, lp(note, 1200), tn, g)
    # milestone chimes
    for ms, m in ((25, 72), (50, 76), (75, 81)):
        fr = np.nonzero(COUNT >= ms)[0]
        if len(fr):
            nsmp = int(1.4 * SR)
            ch = (np.sin(2 * np.pi * mtof(m) * tt(nsmp)) + 0.4 * np.sin(2 * np.pi * mtof(m) * 2.76 * tt(nsmp)) * np.exp(-tt(nsmp) / 0.2)) * expenv(nsmp, 0.5, 0.002)
            place(sfx, ch, fr[0] / FPS, 0.12)
    # "99 in sync": a low hit as the music drops out
    place(sfx, sub_boom(1.2), 31.5, 0.35)
    place(sfx, crash(1.0), 31.5, 0.05)

    # ---------------- riser + snare roll into the drop
    rdur = T_DROP - 34.6
    rs = noise_sweep(rdur, 250, 9000) * np.linspace(0, 1, int(rdur * SR)) ** 2
    place(sfx, np.stack([rs, rs[::-1] * 0 + rs], 1), 34.6, 0.20)
    nsmp = int(rdur * SR)
    tone_f = mtof(57) * 2 ** (np.linspace(0, 1, nsmp) ** 1.5 * 2)
    tone = np.sin(2 * np.pi * np.cumsum(tone_f) / SR) * np.linspace(0, 1, nsmp) ** 2
    place(sfx, tone, 34.6, 0.06)
    SN = snare()
    rolls = [(T_DROP - 2.0 + k * 0.125, 0.10 + 0.1 * k / 8) for k in range(8)]
    rolls += [(T_DROP - 1.0 + k * 0.0625, 0.18 + 0.22 * k / 15) for k in range(15)]
    for tr, g in rolls:
        place(drums, SN, tr, g, 0.1)

    # ---------------- the drop: 8 bars, A minor, 120 BPM, locked to the ticks
    chords = [(45, [57, 60, 64]), (41, [53, 57, 60]), (48, [55, 60, 64]), (43, [55, 59, 62]),
              (45, [57, 60, 64]), (41, [53, 57, 60]), (48, [55, 60, 64]), (40, [52, 56, 59])]
    melody = [
        [(0, 76, 2), (2, 81, 1), (3, 79, 1), (4, 76, 2), (6, 74, 1), (7, 76, 1)],
        [(0, 77, 3), (3, 76, 1), (4, 74, 2), (6, 72, 2)],
        [(0, 72, 2), (2, 76, 1), (3, 79, 1), (4, 84, 3), (7, 83, 1)],
        [(0, 79, 4), (4, 74, 2), (6, 71, 2)],
        [(0, 76, 2), (2, 81, 1), (3, 79, 1), (4, 76, 1), (5, 79, 1), (6, 81, 2)],
        [(0, 84, 2), (2, 81, 1), (3, 77, 1), (4, 81, 2), (6, 79, 2)],
        [(0, 76, 1), (1, 79, 1), (2, 84, 2), (4, 83, 1), (5, 84, 1), (6, 86, 2)],
        [(0, 88, 4), (4, 83, 2), (6, 80, 2)],
    ]
    E8 = BEAT / 2
    E16 = BEAT / 4
    K = kick()
    CL = clap()
    HC = hat(0.03)
    HO = hat(0.16)
    place(sfx, sub_boom(1.8), T_DROP, 0.7)
    place(drums, crash(1.8), T_DROP, 0.30)
    place(drums, crash(1.4), T_DROP + 16 * BEAT, 0.20)
    kick_times = []
    for bar in range(8):
        t0 = T_DROP + bar * 4 * BEAT
        root, triad = chords[bar]
        for b in range(4):
            tb = t0 + b * BEAT
            place(drums, K, tb, 0.85)
            kick_times.append(tb)
            if b in (1, 3):
                place(drums, CL, tb, 0.32, -0.1)
                place(drums, SN, tb, 0.18, 0.1)
            place(drums, HC, tb + E8, 0.16, 0.3)
            if bar >= 4:
                place(drums, HC, tb + E16, 0.07, -0.3)
                place(drums, HC, tb + 3 * E16, 0.07, -0.3)
            if b == 3 and bar % 2 == 1:
                place(drums, HO, tb + E8, 0.12, 0.35)
        # bass: 8ths, root with octave pops
        pat = [0, 0, 12, 0, 0, 12, 0, 12]
        for k_, o in enumerate(pat):
            m = root - 12 + o
            nsmp = int(E8 * 0.95 * SR)
            note = pulse(mtof(m), nsmp, 0.5) * 0.55 + saw(mtof(m), nsmp) * 0.3 + np.sin(2 * np.pi * mtof(m - 12) * tt(nsmp)) * 0.6
            note = lp(note, 1500) * expenv(nsmp, 0.2, 0.003)
            place(music, note, t0 + k_ * E8, 0.20)
        # pad chord for the bar
        nsmp = int(4 * BEAT * SR)
        chd = np.zeros(nsmp)
        for m in triad + [triad[0] + 12]:
            chd += saw(mtof(m), nsmp, 0.005)
        chd = lp(chd, 2200) * expenv(nsmp, 3.0, 0.02)
        place(pad_bus, np.stack([chd, chd], 1) * 0.5, t0, 0.07)
        # arp (bars 3-8)
        if bar >= 2:
            tones = triad + [triad[0] + 12]
            for k_ in range(16):
                m = tones[[0, 1, 2, 3, 2, 1, 2, 3][k_ % 8]] + 12
                nsmp = int(E16 * 0.9 * SR)
                note = pulse(mtof(m), nsmp, 0.125) * expenv(nsmp, 0.05, 0.002)
                place(music, note, t0 + k_ * E16, 0.045, 0.45 if k_ % 2 else -0.45)
        # lead
        for step, m, ln in melody[bar]:
            nsmp = int((ln * E8 - 0.012) * SR)
            note = pulse(mtof(m), nsmp, 0.25, vib=0.006) * expenv(nsmp, 0.45, 0.004)
            note[-200:] *= np.linspace(1, 0, 200)
            place(music, note, t0 + step * E8, 0.11, 0.05)
            # a quiet octave-down echo, one 8th later, panned
            place(music, note, t0 + step * E8 + 3 * E16, 0.035, -0.5)
    # final hit
    place(sfx, sub_boom(2.2), T_END, 0.65)
    place(drums, K, T_END, 0.9)
    place(drums, crash(2.4), T_END, 0.30)
    nsmp = int(4.5 * SR)
    stab = np.zeros(nsmp)
    for m in (45, 57, 60, 64, 69, 76):
        stab += saw(mtof(m), nsmp, 0.004) + 0.5 * pulse(mtof(m), nsmp, 0.25)
    stab = lp(stab, 2600) * expenv(nsmp, 1.3, 0.005)
    place(music, np.stack([stab, stab], 1) * 0.5, T_END, 0.08)

    # sidechain duck for music + pad on every kick
    duck = np.ones(len(music))
    for tk in kick_times:
        i0 = int(tk * SR)
        n = int(0.22 * SR)
        duck[i0:i0 + n] = np.minimum(duck[i0:i0 + n], 1 - 0.55 * np.exp(-tt(n) / 0.07))
    music *= duck[:, None]
    pad_bus *= duck[:, None]

    mix = ticks_bus * 1.0 + sfx * 0.9 + drums * 0.9 + music * 1.0 + pad_bus * 1.0
    mix = add_reverb(mix, 0.16, 1.8, 0.45)
    mix = mix[:NS]
    # section automation: keep the quieter story beats closer to the drop's level
    pts = [(0, 6.5), (5.0, 5.0), (12.4, 4.5), (20.0, 2.5), (31.3, 1.0), (31.5, 4.0),
           (T_DROP - 0.01, 3.0), (T_DROP, 0.0), (T_END, 0.0), (T_END + 1.5, 3.0), (60, 3.0)]
    xs = np.array([p[0] for p in pts]); ys = np.array([p[1] for p in pts])
    gdb = np.interp(tt(NS), xs, ys)
    mix *= (10 ** (gdb / 20))[:, None]
    # gentle bus compression then a hard ceiling
    env = np.sqrt(lfilter([0.002], [1, -0.998], (mix ** 2).mean(1)))
    thr = 0.18
    gain = np.where(env > thr, (thr / np.maximum(env, 1e-9)) ** 0.45, 1.0)
    mix *= gain[:, None]
    mix *= 0.89 / np.abs(mix).max()
    mix = np.tanh(mix * 1.15) / np.tanh(1.15)
    mix *= 0.89 / np.abs(mix).max()
    # tiny fades at the very ends to avoid clicks
    f = int(0.004 * SR)
    mix[:f] *= np.linspace(0, 1, f)[:, None]
    mix[-f:] *= np.linspace(1, 0, f)[:, None]
    wavfile.write('audio.wav', SR, (mix * 32767).astype(np.int16))
    # report loudness by section
    for a, b in ((0, 5), (5, 12.4), (12.4, 20), (20, 31.5), (31.5, T_DROP), (T_DROP, T_END), (T_END, 60)):
        seg = mix[int(a * SR):int(b * SR)]
        print(f'{a:5.1f}-{b:5.1f}s  rms {20 * np.log10(np.sqrt((seg ** 2).mean()) + 1e-9):6.1f} dBFS  peak {20 * np.log10(np.abs(seg).max() + 1e-9):5.1f}')


if __name__ == '__main__':
    main()
