"""The Lowest Note in the Universe: soundtrack, v2 re-cut (v1 is audio_backup.py).
Everything is synthesised; the melody and groove are original.

The science IS the instrument: every pressure-wave crest the picture launches (timeline.LAUNCH) is a
crest-kick, and the crests are the only kick in the track. v2 is never quiet:
  0-2.8 s    cold open = the climax (B-flat stack + crest-kicks + melody), then a tape-stop dive
  3.4-11.7   'real speed': hot-gas crackle + a muffled (low-passed) groove; each spin-up slam
             (year, day, hour, minute) is a crest hit and opens the filter a notch
  11.7       the drop: octave -9, a crest every 2 beats (55 BPM), then 109, 219 BPM, flutter 7 -> 15 Hz
  33.6       the same pulse train at 29.1 Hz is a pitch: B-flat0; B-flat1..4 stack, one per bar
  44.6       the original melody over the full stack; 53.4 hum prompt; the last bar builds and
             slams into the cold open (the file loops: pre-roll mastering + phase-matched stack)
All of it sits on one grid: 1 beat = the hole's note 8 octaves down (109.3 BPM).

    python audio.py            -> audio.wav, prints section levels and science-vs-music band levels"""
import math
import os
import sys
import numpy as np
from scipy.signal import lfilter, butter, sosfilt

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
from reelkit import audio as A
import timeline as T

SR = A.SR
RNG = np.random.default_rng(7)
G, C, BEAT, BAR = T.G, T.C, T.BEAT, T.BAR


# ------------------------------------------------------------------ voices
def crest_kick(dec=0.32, f0=170.0, f1=None, click=1.0, thock=1.0):
    """one wave crest: pitch-swept body tuned to B-flat1 + a short click and a B-flat4 'thock' so phones hear it"""
    f1 = f1 or T.freq(-3)                       # 58.27 Hz
    n = int(max(0.12, dec * 3.2) * SR)
    t = A.tt(n)
    f = f1 + (f0 - f1) * np.exp(-t / 0.035)
    body = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / dec)
    knock = np.sin(2 * np.pi * 2.0 * np.cumsum(f) / SR) * np.exp(-t / min(dec, 0.06)) * 0.35
    ck = A.hp(RNG.normal(0, 1, n), 1800) * np.exp(-t / 0.004) * 0.8 * click
    th = (np.sin(2 * np.pi * T.BB4 * t) + 0.4 * np.sin(2 * np.pi * 2 * T.BB4 * t)) * np.exp(-t / (0.035 + 0.02 * (thock - 1))) * 0.32 * click * thock
    return body + knock + ck + th


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


def tick(f=2400.0, dec=0.006):
    n = int(0.04 * SR)
    t = A.tt(n)
    return (np.sin(2 * np.pi * f * t) * 0.7 + A.hp(RNG.normal(0, 1, n), 3000) * 0.3) * np.exp(-t / dec)


def hum(m, dur):
    """a hummed 'mmm': soft buzz through a closed-mouth formant"""
    n = int(dur * SR)
    t = A.tt(n)
    f = A.mtof(m) * (1 + 0.005 * np.sin(2 * np.pi * 4.8 * t))
    ph = 2 * np.pi * np.cumsum(f) / SR
    x = sum(np.sin(h * ph) / h ** 1.3 for h in range(1, 14))
    return A.lp(A.bp(x, 150, 2600), 1100) * 1.6


def bass_note(m, dur):
    n = int((dur + 0.06) * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = A.saw(f, n, det=0.005, voices=2) * 0.55 + np.sin(2 * np.pi * f * t) * 0.8
    e = np.clip(t / 0.004, 0, 1) * np.exp(-t / 0.2) * np.clip((dur + 0.06 - t) / 0.05, 0, 1)
    return x * e


def paper(seed):
    n = int(0.05 * SR)
    t = A.tt(n)
    return A.bp(np.random.default_rng(seed).normal(0, 1, n), 1500, 7000) * np.exp(-t / 0.012)


def crackle(dur, rate=70.0, seed=11):
    """hot-gas crackle: sparse random clicks over a soft sizzle (texture, not data)"""
    n = int(dur * SR)
    r = np.random.default_rng(seed)
    x = np.zeros(n)
    k = r.poisson(rate * dur)
    pos = r.integers(0, n - 600, k)
    amp = r.exponential(1.0, k) ** 1.6
    nb = 400
    tb = A.tt(nb)
    for p, a in zip(pos, amp):
        x[p:p + nb] += r.normal(0, 1, nb) * np.exp(-tb / r.uniform(0.0006, 0.003)) * a
    x = A.bp(x, 700, 8000)
    sizzle = A.bp(r.normal(0, 1, n), 2500, 9000) * 0.06
    rumble = A.lp(r.normal(0, 1, n), 140) * 0.9
    return x * 1.0 + sizzle + rumble * 0.5


def snare_roll(mix, t0, t1, g0, g1, bus='drums'):
    """accelerating roll: 8ths, then 16ths, then 32nds over the last quarter"""
    t = t0
    while t < t1 - 1e-6:
        u = (t - t0) / (t1 - t0)
        mix.place(bus, A.snare(0.07), t, g0 + (g1 - g0) * u ** 1.4, pan=0.1 * math.sin(t * 9))
        t += BEAT / 2 if u < 0.5 else BEAT / 4 if u < 0.75 else BEAT / 8


# ------------------------------------------------------------------ processing
def rbj_lp(fc, q):
    w = 2 * math.pi * fc / SR
    al = math.sin(w) / (2 * q)
    cw = math.cos(w)
    b = np.array([(1 - cw) / 2, 1 - cw, (1 - cw) / 2])
    a = np.array([1 + al, -2 * cw, 1 - al])
    return b / a[0], a / a[0]


def lp_auto(x, t0, pts, qs=(1.1, 0.7), block=64):
    """time-varying low-pass (two cascaded biquads, ~24 dB/oct); pts = [(abs time, cutoff Hz)]"""
    n = len(x)
    tb = t0 + np.arange(0, n, block) / SR
    fcs = np.exp(np.interp(tb, [p[0] for p in pts], [math.log(p[1]) for p in pts]))
    y = x.copy()
    for q in qs:
        zi = np.zeros(2)
        out = np.empty(n)
        for j, i in enumerate(range(0, n, block)):
            b, a = rbj_lp(min(fcs[j], SR * 0.45), q)
            out[i:i + block], zi = lfilter(b, a, y[i:i + block], zi=zi)
        y = out
    return y


def tape_dive(buf, t0, length):
    """tape-stop on a stereo buffer: from t0 the playback rate falls (1-u)^1.5 to 0 over `length`; silence after"""
    i0 = int(t0 * SR)
    n = int(length * SR)
    rate = (1 - np.linspace(0, 1, n)) ** 1.5
    pos = np.cumsum(rate)
    seg = buf[i0:i0 + n + 10]
    out = np.stack([np.interp(pos, np.arange(len(seg)), seg[:, c]) for c in range(2)], 1)
    y = buf.copy()
    y[i0:i0 + n] = out * np.linspace(1, 0, n)[:, None] ** 0.35
    y[i0 + n:] = 0
    return y


def master_loop(mix, gains=None, reverb_buses=None, gain_db_pts=None, thr=0.16, ceiling=0.89, pre=3.0):
    """reelkit Mix.master, made loop-aware: anything that spills past 60 s wraps onto the start, and the
    limiter / reverb see the end of the song before the start (pre-roll), so the seam is seamless"""
    from reelkit.audio import reverb
    NS, P = mix.NS, int(pre * SR)
    gains = gains or {}
    buses = {}
    for k, b in mix.buses.items():
        x = b[:NS].copy()
        tail = b[NS:]
        x[:len(tail)] += tail[:NS]
        buses[k] = np.concatenate([x[-P:], x], 0)
    dry = sum(b * gains.get(k, 1.0) for k, b in buses.items())
    if reverb_buses:
        send = sum(buses[k] * v for k, v in reverb_buses.items() if k in buses)
        dry = dry + reverb(send) * 0.22
    m = dry
    if gain_db_pts:
        tt = A.tt(len(m)) - pre
        g = np.interp(tt % T.DUR, [p[0] for p in gain_db_pts], [p[1] for p in gain_db_pts])
        m = m * (10 ** (g / 20))[:, None]
    env = np.sqrt(lfilter([0.002], [1, -0.998], (m ** 2).mean(1)))
    gain = np.where(env > thr, (thr / np.maximum(env, 1e-9)) ** 0.45, 1.0)
    m = m * gain[:, None]
    # fast peak limiter (2 ms look-around, 60 ms release) so a few sub hits don't set the whole level
    from scipy.ndimage import maximum_filter1d
    pk = maximum_filter1d(np.abs(m).max(1), size=96)
    pk_thr = np.percentile(pk[P:], 99.5)
    g = np.minimum(1.0, pk_thr / np.maximum(pk, 1e-9))
    a = 1 - math.exp(-1 / (0.06 * SR))
    gs = lfilter([a], [1, a - 1], g, zi=[g[0] * (1 - a)])[0]
    m = (m * np.minimum(g, gs)[:, None])[P:]
    m *= ceiling / (np.abs(m).max() + 1e-9)
    m = np.tanh(m * 1.1) / np.tanh(1.1)
    m *= ceiling / (np.abs(m).max() + 1e-9)
    return m


# ------------------------------------------------------------------ the score
STACK = [  # (octave k, onset, duty, fmax, gain)
    (-4, T.T4, 0.12, 3500, 0.34),
    (-3, T.T3, 0.25, 4000, 0.24),
    (-2, T.T2, 0.35, 4500, 0.21),
    (-1, T.T1, 0.42, 5000, 0.18),
    (0, T.T0, 0.48, 6000, 0.13),
]


def duck_curve(n, t0, beats, depth=0.32):
    tt = A.tt(n) + t0
    d = np.ones(n)
    for tb in beats:
        ph = tt - tb
        d *= 1 - depth * np.where(ph >= 0, np.exp(-np.clip(ph, 0, None) / 0.12), 0.0)
    return d


def cold_open():
    """the climax as heard at the loop point (beats 0..5), then the tape-stop dive"""
    cm = A.Mix(6.0)
    t_end = T.T_THUD + 0.2
    n = int(t_end * SR)
    beats = [C(k) for k in range(-1, T.N_COLD)]
    duck = duck_curve(n, 0.0, beats)
    for k, on, duty, fmax, gain in STACK:
        x = bl_pulse(T.freq(k), n, duty, fmax, t0=T.DUR)          # phase continues from the end of the song
        cm.place('sci', x * duck, 0.0, gain)
    for k in range(T.N_COLD):
        cm.place('sci', crest_kick(0.3, 175), C(k), 0.8)
        if k % 2 == 1:
            cm.place('drums', A.clap(), C(k), 0.22)
        cm.place('drums', A.hat(), C(k) + BEAT / 2, 0.08)
        cm.place('drums', A.hat(0.02), C(k), 0.04)
    cm.place('drums', A.crash(2.5), 0.0, 0.13)
    cm.place('music', A.pad(T.CHORDS[0][1], 4 * BEAT + 0.1, cut=1300, att=0.02), 0.0, 0.08)
    cm.place('music', A.pad(T.CHORDS[1][1], 4 * BEAT + 0.1, cut=1300, att=0.05), C(4), 0.08)
    for b, m, d in T.MELODY:
        if b <= 5.0:
            cm.place('music', lead(m, d * BEAT * 0.95), C(b), 0.19, pan=-0.08)
            cm.place('music', lead(m + 12, d * BEAT * 0.9), C(b), 0.04, pan=0.15)
    for k in cm.buses:
        cm.buses[k] = tape_dive(cm.buses[k], T.T_STOP, T.T_THUD - T.T_STOP)
    return cm


def groove():
    """the continuous groove from the thud to the loop point (mono), muffled by the filter automation"""
    t0 = T.T_THUD
    n = int((T.DUR - t0 + 1.0) * SR)
    g = np.zeros(n)

    def put(sig, t, gain):
        i = int(round((t - t0) * SR))
        if i < 0:
            sig, i = sig[-i:], 0
        m = min(len(sig), n - i)
        if m > 0:
            g[i:i + m] += sig[:m] * gain

    BASS_PAT = [0, 0, 12, 0, 0, 7, 12, 0]
    ARP_PAT = [0, 1, 2, 1, 2, 3, 2, 1]
    for bar in range(-4, 22):
        tb = G(4 * bar)
        root, ch = T.CHORDS[bar % 4]
        # pad
        if tb + BAR > t0:
            pd = A.pad(ch + [ch[0] + 12], BAR + 0.08, cut=1500, att=0.06)
            put(pd, tb, 0.3 if tb < T.T_CLIMAX else 0.16)
        for e in range(8):
            te = tb + e * BEAT / 2
            if te < t0 - 1e-6:
                continue
            # bass eighths until B-flat0 takes over as the bass (and back for the hum section's last bar off)
            if te < T.T4:
                put(bass_note(root - 12 + BASS_PAT[e], BEAT / 2 * 0.92), te, 0.34)
            # arpeggio (hidden under the filter at first, revealed as it opens); rests under the melody
            if te < T.T_CLIMAX - 1e-6 or te >= T.T_LAND:
                tones = ch + [ch[0] + 12]
                put(A.pluck(tones[ARP_PAT[e]] + 12, 0.45), te, 0.13 if te < T.T_LAND else 0.08)
        for s in range(16):                                   # shaker 16ths
            ts = tb + s * BEAT / 4
            if ts < t0 - 1e-6:
                continue
            acc = 1.0 if s % 4 == 2 else 0.55 if s % 2 == 0 else 0.35
            put(A.hp(RNG.normal(0, 1, int(0.04 * SR)), 5000) * np.exp(-A.tt(int(0.04 * SR)) / 0.012), ts, 0.07 * acc)
    pts = [(t0, 340), (T.T_YEAR - 0.02, 340), (T.T_YEAR + 0.02, 520), (T.T_DAY - 0.02, 520), (T.T_DAY + 0.02, 760),
           (T.T_HOUR - 0.02, 760), (T.T_HOUR + 0.02, 1100), (T.T_MIN - 0.02, 1100), (T.T_MIN + 0.02, 1500),
           (T.T9 - 0.02, 1800), (T.T9 + 0.02, 3000), (T.T_HATS, 4200), (T.T8 - 0.02, 4600), (T.T8 + 0.02, 7000),
           (T.T7 - 0.02, 8000), (T.T7 + 0.02, 12000), (T.T6, 18000), (T.DUR + 2, 18000)]
    g = lp_auto(g, t0, pts)
    return g, t0


def main():
    mix = A.Mix(T.DUR)
    n_all = int(T.DUR * SR)

    # ---- 0-2.8 s cold open (the climax), tape-stop dive to real speed
    cm = cold_open()
    for k, b in cm.buses.items():
        mix.bus(k)[:len(b)] += b[:len(mix.bus(k))]
    mix.place('fx', A.whoosh(0.8, 2600, 90, rise=False), T.T_STOP - 0.05, 0.22)
    mix.place('fx', crest_kick(0.9, 95, 31, click=0.3), T.T_THUD, 0.6)            # sub thud: real speed
    # ---- hot-gas crackle bed (texture), strongest while frozen, then tucked under the groove
    cr = crackle(T.T9 + 3.0 - (T.T_STOP - 0.3), seed=11)
    cr *= env_pts(len(cr), [(T.T_STOP - 0.3, 0), (T.T_THUD, 1.0), (T.T_YEAR, 0.9), (T.T9, 0.45), (T.T9 + 3.0, 0)], T.T_STOP - 0.3)
    mix.place('amb', np.stack([cr, np.roll(cr, 2400)], 1), T.T_STOP - 0.3, 0.3)

    # ---- the groove: muffled under water, opening a notch with every slam, fully open by -6
    gv, gt0 = groove()
    gv = gv * env_pts(len(gv), [(gt0, 0.0), (gt0 + 0.04, 1.0), (T.T9 - 0.02, 1.0), (T.T9 + 0.02, 1.45),
                                (T.T4 - 0.4, 1.45), (T.T4, 1.3), (T.T2, 1.3), (T.T0, 1.0), (T.DUR + 1, 1.0)], gt0)
    mix.place('music', np.stack([gv, np.roll(gv, 480)], 1), gt0, 1.0)

    # ---- spin-up slams: each is a crest hit (+ swell, counter ticks, calendar / clock)
    for i, (th, k) in enumerate(T.SPIN_HITS):
        mix.place('sci', crest_kick(0.5, 165, click=1.0, thock=1.8), th, 0.95)
        mix.place('fx', A.whoosh(0.5, 400, 6000), th - 0.48, 0.1)
        mix.place('fx', A.chime((82, 84, 86, 89)[i], 1.2), th, 0.05)
        mix.place('drums', A.crash(1.2), th, 0.05)
    for st, k in T.STEPS:
        if T.T_YEAR - 1.0 < st <= T.T9:
            mix.place('fx', tick(1300 * 2 ** ((k + 57) / 40)), st, 0.045)
        if T.T_STOP <= st <= T.T_THUD:
            mix.place('fx', tick(1300 * 2 ** ((k + 57) / 40), 0.004), st, 0.03)
    gap = T.T_DAY - T.T_YEAR
    for j in range(12):                                                             # calendar pages
        mix.place('fx', paper(j), T.T_YEAR + gap * (j + 0.45) / 12, 0.05, pan=0.4)
    n16 = int(round((T.T9 - T.T_DAY) / (BEAT / 4)))
    for j in range(n16):                                                            # clockwork ticks on the 16ths
        mix.place('fx', tick(3200 if j % 4 == 0 else 2600, 0.003), T.T_DAY + j * BEAT / 4, 0.035 if j % 4 == 0 else 0.02, pan=0.4)
    for tl, a, g in T.LAUNCH:                                                       # time-lapse build crests
        if g == 'A' and a < 1.0:
            mix.place('sci', crest_kick(0.16, 150, click=0.7), tl, 0.5)
    rs = A.whoosh(2.4, 300, 7500) * np.linspace(0.15, 1, int(2.4 * SR)) ** 2
    mix.place('fx', rs, T.T9 - 2.4, 0.22)
    snare_roll(mix, G(-4), T.T9, 0.03, 0.16)

    # ---- the drop: crests are the kick, doubling 55 -> 109 -> 219 BPM, then a flutter
    mix.place('drums', A.crash(3.0), T.T9, 0.16)
    mix.place('sci', crest_kick(0.7, 180, 40), T.T9, 0.5)
    for tl, a, g in T.LAUNCH:
        if T.T9 - 0.01 <= tl < T.T4 - 1e-6:
            k = T.octave(tl + 1e-4)
            dec = {-9: 0.38, -8: 0.28, -7: 0.17, -6: 0.085, -5: 0.045}[k]
            gain = {-9: 0.95, -8: 0.85, -7: 0.75, -6: 0.62, -5: 0.55}[k]
            mix.place('sci', crest_kick(dec, 170 if k >= -7 else 140, click=1.0 if k >= -7 else 0.6,
                                        thock={-9: 2.4, -8: 2.0, -7: 1.5}.get(k, 1.0)), tl, gain)
    for n in range(4, 88):                                                          # backbeat claps on 2 and 4
        tb = G(n)
        if n % 4 in (1, 3) and not (T.T5 <= tb < T.T4):
            mix.place('drums', A.clap(), tb, 0.2 if tb < T.T8 else 0.22)
    nb = int(round((T.DUR - T.T_HATS) / BEAT))
    for j in range(nb):                                                             # hats from 16 s
        tb = T.T_HATS + j * BEAT
        mix.place('drums', A.hat(), tb + BEAT / 2, 0.1 if tb < T.T4 else 0.08)
        if tb >= T.T8:
            mix.place('drums', A.hat(0.02), tb, 0.04)
            if tb >= T.T7 and tb < T.T4:
                mix.place('drums', A.hat(0.015), tb + BEAT / 4, 0.025)
                mix.place('drums', A.hat(0.015), tb + 3 * BEAT / 4, 0.025)
    for tm, c in ((T.T8, 0.09), (T.T7, 0.1)):
        mix.place('drums', A.crash(2.0), tm, c)
        mix.place('fx', A.whoosh(0.6, 500, 6000), tm - 0.55, 0.08)
    snare_roll(mix, G(28), T.T6, 0.04, 0.17)
    fl = A.whoosh(T.T4 - T.T6, 250, 8000) * np.linspace(0.2, 1, int((T.T4 - T.T6) * SR)) ** 2
    mix.place('fx', fl, T.T6, 0.18)

    # ---- 33.6 s: the pulse train is a pitch, B-flat0; B-flat1..4 stack (they run to the loop point)
    t_start = T.T4
    n = int((T.DUR - t_start) * SR)
    beats = [G(j) for j in range(40, 88)]
    duck = duck_curve(n, t_start, beats)
    for k, on, duty, fmax, gain in STACK:
        f = T.freq(k)
        m = int((T.DUR - on) * SR)
        x = bl_pulse(f, m, duty, fmax, t0=on)
        e = env_pts(m, [(on, 0.0), (on + (0.03 if k == -4 else 0.35), 1.0), (T.T_LAND + 0.4, 1.0), (T.T_LAND + 1.5, 0.5),
                        (T.T_BUILD, 0.5), (T.DUR - 0.25, 1.0), (T.DUR, 1.0)], on)
        i0 = int((on - t_start) * SR)
        mix.place('sci', x * e * duck[i0:i0 + m], on, gain)
        if k > -4:
            mix.place('fx', A.chime(70 + 12 * k + 12, 1.2), on, 0.04)
    mix.place('drums', A.crash(3.0), T.T4, 0.15)
    mix.place('sci', crest_kick(0.7, 180, 40), T.T4, 0.45)

    # ---- crest-kick on every beat from B-flat0 to the loop point
    for tl, a, g in T.LAUNCH:
        if g == 'C':
            big = tl in (T.T4, T.T_CLIMAX, T.T_LAND)
            mix.place('sci', crest_kick(0.5 if big else 0.3, 175), tl, 1.0 if big else 0.8)

    # ---- 44.6 s climax: the original melody over the stack
    mix.place('drums', A.crash(2.5), T.T_CLIMAX, 0.13)
    mix.place('drums', A.crash(3.5), T.T_LAND, 0.14)
    for b, m, d in T.MELODY:
        tb = T.T_CLIMAX + b * BEAT
        mix.place('music', lead(m, d * BEAT * 0.95), tb, 0.19, pan=-0.08)
        mix.place('music', lead(m + 12, d * BEAT * 0.9), tb, 0.04, pan=0.15)

    # ---- 53.4 s hum prompt over the groove; the last bar builds and slams into the cold open
    hd = G(86.5) - (T.T_LAND + 0.3)
    for m, gn in ((70, 0.13), (58, 0.05)):
        hm = hum(m, hd)
        hm *= env_pts(len(hm), [(T.T_LAND + 0.3, 0), (T.T_LAND + 1.0, 1), (G(84), 1.0), (G(86.5), 0)], T.T_LAND + 0.3)
        mix.place('sci', hm, T.T_LAND + 0.3, gn)
    snare_roll(mix, T.T_BUILD, T.DUR, 0.04, 0.2)
    rb = A.whoosh(T.DUR - T.T_BUILD, 300, 8000) * np.linspace(0.1, 1, int((T.DUR - T.T_BUILD) * SR)) ** 2
    mix.place('fx', rb, T.T_BUILD, 0.2)

    out = master_loop(mix, reverb_buses={'music': 0.9, 'fx': 0.6, 'sci': 0.18, 'drums': 0.25},
                      gains={'sci': 1.0, 'music': 1.0, 'amb': 1.0})
    A.write('audio.wav', out)

    # ---------------- checks
    secs = [(0, T.T_STOP), (T.T_STOP, T.T_THUD), (T.T_THUD, T.T_YEAR), (T.T_YEAR, T.T_MIN), (T.T_MIN, T.T9), (T.T9, T.T8),
            (T.T8, T.T7), (T.T7, T.T6), (T.T6, T.T4), (T.T4, T.T_CLIMAX), (T.T_CLIMAX, T.T_LAND), (T.T_LAND, T.T_BUILD), (T.T_BUILD, 60)]
    print('section levels (dBFS):')
    for a, b, rms, pk in A.section_levels('audio.wav', secs):
        print(f'  {a:5.1f}-{b:5.1f}s  rms {rms:6.1f}  peak {pk:5.1f}')
    win = [(a, a + 2.0) for a in np.arange(0, 60, 2.0)]
    full = [r for _, _, r, _ in A.section_levels('audio.wav', win)]
    hi = [r for _, _, r in A.band_level('audio.wav', 250, 16000, win)]
    print('2 s windows: full rms min %.1f max %.1f | above 250 Hz (phone) min %.1f max %.1f' % (min(full), max(full), min(hi), max(hi)))
    print('  quietest phone windows:', sorted(zip(hi, [w[0] for w in win]))[:3])
    stems = {k: v[:n_all].mean(1) for k, v in mix.buses.items()}

    def lvl(x, lo, hi_, a, b):
        y = sosfilt(butter(4, [lo, hi_], 'bp', fs=SR, output='sos'), x)
        seg = y[int(a * SR):int(b * SR)]
        return 20 * np.log10(np.sqrt((seg ** 2).mean()) + 1e-12)
    music = stems['music'] + stems.get('drums', 0)
    print('science vs music+drums (stems, dB):')
    for a, b, lo, hi_, name in ((0, 2.7, 25, 600, 'cold-open stack'), (5, 11.6, 40, 160, 'spin-up hits'), (5, 11.6, 1500, 5000, 'hit clicks'),
                                (11.7, 18.2, 40, 160, 'kick body -9'), (11.7, 18.2, 1500, 5000, 'kick click -9'),
                                (18.3, 24.8, 40, 160, 'kick body -8'), (33.7, 44.5, 25, 600, 'B-flat stack'),
                                (44.6, 53.4, 25, 600, 'stack in climax'), (44.6, 53.4, 1500, 5000, 'clicks in climax'),
                                (53.4, 57.8, 300, 1500, 'hum')):
        s, mu = lvl(stems['sci'], lo, hi_, a, b), lvl(music, lo, hi_, a, b)
        print(f'  {name:16s} {lo:5d}-{hi_:<5d}Hz  science {s:6.1f}  music {mu:6.1f}  -> {s - mu:+5.1f} dB')
    def peaks(x, lo, hi_, a, b, q):
        y = sosfilt(butter(4, [lo, hi_], 'bp', fs=SR, output='sos'), x)[int(a * SR):int(b * SR)]
        fr = np.sqrt((y[:len(y) // 480 * 480].reshape(-1, 480) ** 2).mean(1))
        return 20 * np.log10(np.percentile(fr, q) + 1e-12)
    def onset(x, lo, hi_, times):
        y = sosfilt(butter(4, [lo, hi_], 'bp', fs=SR, output='sos'), x)
        return 20 * np.log10(np.median([np.sqrt((y[int(t * SR):int((t + 0.015) * SR)] ** 2).mean()) for t in times]) + 1e-12)
    for a, b, name in ((11.7, 18.2, '-9'), (18.3, 24.8, '-8'), (24.9, 29.2, '-7'), (44.6, 53.4, 'climax')):
        ks = [t for t in T.KICKS if a <= t < b]
        print(f'  crest onsets {name:6s}: click 1.5-5k {onset(stems["sci"], 1500, 5000, ks):6.1f} vs music median {peaks(music, 1500, 5000, a, b, 50):6.1f}'
              f' | thock 400-1000 {onset(stems["sci"], 400, 1000, ks):6.1f} vs {peaks(music, 400, 1000, a, b, 50):6.1f}')
    print('loop seam: |x[-1]-x[0]| =', np.abs(out[-1] - out[0]).max().round(3), ' rms last/first 50 ms (dB):',
          round(20 * np.log10(np.sqrt((out[-2400:] ** 2).mean())), 1), round(20 * np.log10(np.sqrt((out[:2400] ** 2).mean())), 1))


if __name__ == '__main__':
    main()
