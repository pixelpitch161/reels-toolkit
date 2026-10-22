"""Chaos Makes a Perfect Curve - soundtrack. Everything is synthesised and original.

    python audio.py        -> audio.wav, prints section levels and the science-band checks

The science drives the sound, from the same events as the picture (balls.npz via motion.py):
- every peg contact is a ping, pitched by the peg's column on C major pentatonic (C5 = centre,
  one step per half peg spacing), so each ball plays its own 12-note random walk, and most walks
  end near the tonic. As balls pile up (1 -> 10 -> 100 -> 1,000) the pings get shorter and denser:
  melody -> rain -> hail;
- every landing is a wooden tock plus its bin's note;
- each bin holds one note of a wide C6/9 chord (G1 ... C4 in the middle ... E6) and sounds as
  loud as the bin is full, so the growing curve IS the chord voicing (strong middle, quiet edges);
- after the drop, an arpeggio picks bin notes with probabilities equal to the real bin counts.
Music (original): pads from 4 s, light ticks from 8 s, a 120 BPM groove from 26 s (F G C Am),
the drop at 46 s on C, drain + elevator clanks at 52-57 s, the hail returns at 57.5 s into the
loop's bass hit at 0 s."""
import math
import os
import sys
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit import audio as A
import timeline as T
import board as B
import motion as M

SR = A.SR
NS = int(T.DUR * SR)
_cache = {}


def tt(n):
    return np.arange(n) / SR


# ---------------------------------------------------------------- instruments
DECAYS = [0.6, 0.4, 0.26, 0.16, 0.1, 0.06, 0.035]


def ping(m, dec):
    """glass bead on a brass peg: bright attack, a bell-ish partial set, a tiny click"""
    key = ('p', m, dec)
    if key in _cache:
        return _cache[key]
    f = A.mtof(m)
    n = int(min(1.6, dec * 5 + 0.03) * SR)
    t = tt(n)
    out = np.zeros(n)
    for ratio, amp, dk in ((1.0, 1.0, 1.0), (2.0, 0.32, 0.5), (2.76, 0.22, 0.3), (5.4, 0.1, 0.12)):
        if f * ratio < 12000:
            out += amp * np.sin(2 * np.pi * f * ratio * t + ratio) * np.exp(-t / (dec * dk))
    click = A.hp(np.random.default_rng(m).normal(0, 1, n), 3000) * np.exp(-t / 0.0012) * 0.18
    out = (out + click) * np.clip(t / 0.0008, 0, 1)
    _cache[key] = out
    return out


def tock(dec=0.05, seed=0):
    """a ball landing in a wooden bin"""
    key = ('t', dec, seed % 5)
    if key in _cache:
        return _cache[key]
    n = int(0.18 * SR)
    t = tt(n)
    x = A.bp(np.random.default_rng(seed % 5 + 10).normal(0, 1, n), 500, 2600) * np.exp(-t / 0.008) * 0.6
    x += np.sin(2 * np.pi * (180 + 40 * (seed % 5)) * t) * np.exp(-t / dec) * 0.7
    _cache[key] = x
    return x


def bell(m, dur=1.4, bright=1.0):
    n = int(dur * SR)
    t = tt(n)
    f = A.mtof(m)
    x = np.sin(2 * np.pi * f * t) + 0.35 * bright * np.sin(2 * np.pi * f * 3.0 * t) * np.exp(-t / 0.2) + 0.2 * bright * np.sin(2 * np.pi * f * 4.07 * t) * np.exp(-t / 0.12)
    return x * A.expenv(n, dur * 0.3, attack=0.002)


def marimba(m, dur=0.5):
    key = ('mar', m, dur)
    if key in _cache:
        return _cache[key]
    n = int(dur * SR)
    t = tt(n)
    f = A.mtof(m)
    x = np.sin(2 * np.pi * f * t) * np.exp(-t / (dur * 0.35)) + 0.25 * np.sin(2 * np.pi * f * 3.93 * t) * np.exp(-t / 0.04)
    x *= np.clip(t / 0.002, 0, 1)
    _cache[key] = x
    return x


def gong(f0=98.0, dur=5.0):
    n = int(dur * SR)
    t = tt(n)
    x = np.zeros(n)
    for r, a, d in ((1.0, 1.0, 2.2), (1.48, 0.6, 1.6), (2.0, 0.45, 1.3), (2.76, 0.35, 0.9), (3.4, 0.25, 0.7), (4.1, 0.2, 0.5), (5.43, 0.12, 0.35)):
        x += a * np.sin(2 * np.pi * f0 * r * t * (1 + 0.002 * np.sin(2 * np.pi * 0.7 * t))) * np.exp(-t / d)
    x *= np.clip(t / 0.004, 0, 1)
    x += A.lp(np.random.default_rng(3).normal(0, 1, n), 1800) * np.exp(-t / 0.05) * 0.3
    return x


def boom(f0=65.0, dur=1.4):
    n = int(dur * SR)
    t = tt(n)
    f = f0 + f0 * 1.2 * np.exp(-t / 0.05)
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / (dur * 0.35))
    return np.tanh(1.6 * x) + 0.25 * A.hp(np.random.default_rng(4).normal(0, 1, n), 2000) * np.exp(-t / 0.004)


def bass_note(m, dur, vel=1.0, cut=900):
    n = int((dur + 0.06) * SR)
    t = tt(n)
    f = A.mtof(m)
    x = A.saw(f, n, det=0.004, voices=2) * 0.6 + np.sin(2 * np.pi * f * t)
    x = A.lp(x, cut) * A.adsr(n, 0.005, 0.12, 0.7, 0.05)
    return np.tanh(1.3 * x) * vel


def tick(dec=0.018, seed=9):
    n = int(0.08 * SR)
    t = tt(n)
    return A.hp(np.random.default_rng(seed).normal(0, 1, n), 7000) * np.exp(-t / dec) * 0.6


def clank(seed=0):
    """elevator bucket clank: metal ring kept above and below the ping band's centre"""
    n = int(0.25 * SR)
    t = tt(n)
    r = np.random.default_rng(seed)
    x = A.bp(r.normal(0, 1, n), 3500, 9000) * np.exp(-t / 0.01) * 0.6
    for f, a in ((3720, 0.4), (5130, 0.3), (210, 0.5)):
        x += a * np.sin(2 * np.pi * f * t) * np.exp(-t / 0.06)
    return x


def rev_whoosh(dur, lo=400, hi=7000):
    return A.whoosh(dur, lo, hi, rise=True)[::1]


# ---------------------------------------------------------------- the science: pings + landings
def heard_contacts():
    """(time placed, ball, contact k) for every contact that sounds"""
    out = []
    for i in range(M.N):
        run = M.RUN[i]
        for k in range(12):
            te = M.TC[i, k]
            if run == 1:
                if te >= T.T_FREEZE:
                    continue
                te = te + T.DUR if te < 0 else te
            out.append((te, i, k))
    out.sort()
    return out


def pings(mix):
    ev = heard_contacts()
    times = np.array([e[0] for e in ev])
    lo = np.searchsorted(times, times - 0.05)
    hi = np.searchsorted(times, times + 0.05)
    dens = np.maximum(hi - lo, 1).astype(float)
    for (te, i, k), d in zip(ev, dens):
        c = int(M.CC[i, k])
        kind = M.KIND[i]
        solo = kind in (0, 1, 3)              # hero, second, gold
        dec = 0.6 if solo else 0.6 / math.sqrt(d)
        dec = min(DECAYS, key=lambda v: abs(v - dec))
        g = (0.5 if solo else 0.34 * d ** -0.45) * (0.9 + 0.1 * math.sin(te * 13.1 + i))
        pan = float(np.clip(c / 13.0, -0.85, 0.85))
        bus = 'ping'
        mix.place(bus, ping(T.ping_note(c), dec), te, g, pan)
    return times, dens


def landings(mix):
    ev = []
    for i in range(M.N):
        tl = M.TLAND[i]
        if M.RUN[i] == 1:
            if tl >= T.T_FREEZE:
                continue
            tl = tl + T.DUR if tl < 0 else tl
        ev.append((tl, i))
    ev.sort()
    times = np.array([e[0] for e in ev])
    dens = np.maximum(np.searchsorted(times, times + 0.1) - np.searchsorted(times, times - 0.1), 1)
    for (tl, i), d in zip(ev, dens):
        b = int(M.BIN[i])
        pan = float(np.clip((b - 6) / 8.0, -0.8, 0.8))
        if M.RUN[i] == 2:
            mix.place('fx', gong(98.0, 5.5), tl, 0.5)
            mix.place('fx', boom(49.0, 1.8), tl, 0.5)
            mix.place('land', bell(T.bin_note(b) + 24, 2.5), tl, 0.25, pan)
            continue
        solo = M.KIND[i] in (0, 1)
        g = 0.45 if solo else 0.3 / math.sqrt(d)
        mix.place('land', tock(0.06 if solo else 0.03, i), tl, g, pan)
        mix.place('land', marimba(T.bin_note(b) + 12, 0.6 if solo else 0.25), tl, (0.4 if solo else 0.16 / math.sqrt(d)), pan)


def drain_rattle(mix):
    """1,000 balls falling through the trapdoors into the base"""
    for i in M.MAIN:
        td = M.DRAIN_T[i] + 0.12
        mix.place('land', tock(0.02, i), td, 0.045, float(np.clip((M.BIN[i] - 6) / 8.0, -0.8, 0.8)))


def bin_chord(mix):
    """13 sustained voices, each as loud as its bin is full"""
    fs = 100
    ts = np.arange(0, T.DUR, 1 / fs)
    counts = np.array([M.bin_counts(t) for t in ts], float)              # (n, 13)
    # smooth attack/release
    sm = np.zeros_like(counts)
    a_up, a_dn = 1 - math.exp(-1 / (fs * 0.25)), 1 - math.exp(-1 / (fs * 0.5))
    cur = counts[-1].copy() * 0 + counts[0]
    for j in range(len(ts)):
        d = counts[j] - cur
        cur = cur + np.where(d > 0, a_up, a_dn) * d
        sm[j] = cur
    tA = tt(NS)
    bloom = np.interp(tA, [0, T.T_DROP - 0.2, T.T_DROP + 0.4, T.T_THIN, T.T_DRAIN + 1, 60], [1, 1, 1.7, 1.6, 1.0, 1.0])
    out = np.zeros(NS)
    for b in range(B.NB):
        m = T.bin_note(b)
        f = A.mtof(m)
        amp = np.interp(tA, ts, np.minimum(1.0, sm[:, b] / 170.0) ** 0.75)
        if amp.max() < 1e-4:
            continue
        vib = 1 + 0.003 * np.sin(2 * np.pi * (4.7 + 0.3 * b) * tA + b)
        ph = 2 * np.pi * f * np.cumsum(vib) / SR
        tone = np.sin(ph) + 0.35 * np.sin(2 * ph + 0.3) + 0.15 * np.sin(3 * ph + 0.7) + 0.5 * np.sin(ph * 1.003 + 1.1)
        w = 0.8 if m < 48 else (1.0 if m < 76 else 0.7)
        out += tone * amp * w
    out *= bloom * 0.11
    mix.place('chord', A.lp(out, 3200), 0.0, 1.0)
    return ts, sm


# ---------------------------------------------------------------- music
def pad_chord(mix, notes, t0, dur, g=0.5, cut=1300, att=0.4):
    mix.place('music', A.pad(notes, dur + 0.5, cut=cut, att=att), t0, g)


def pascal_times(k):
    """when each peg of Pascal row k (k = 12: the bins) lights, left to right"""
    tk = T.PASCAL_T0 + T.PASCAL_STEP * k
    return [tk + j * 0.2 / (k + 1) for j in range(k + 1)]


def music(mix):
    Bt, Br = T.BEAT, T.BAR
    # ---- 0-4: cold-open hit + sub under the hail
    mix.place('fx', boom(65.4, 1.8), 0.0, 0.7)
    mix.place('bass', bass_note(36, 3.6, 0.6, 400), 0.0, 0.5)
    mix.place('music', A.pad([48, 55, 64, 67], 4.0, cut=900, att=0.05), 0.0, 0.35)
    # ---- 4.0 freeze: the hail stops dead, a low "whomp"; 4.4-5.4 rewind swell
    mix.place('fx', boom(82.0, 1.2), T.T_FREEZE, 0.45)
    mix.place('fx', rev_whoosh(1.2, 600, 8000), T.T_REWIND[0] - 0.2, 0.22)
    # ---- 4-26: pads under the solo melodies, ticks from 8 s
    pad_chord(mix, [48, 55, 62, 64], 4.0, 4.0, 0.42, 1100, 0.6)                # Cadd9 under the hero
    pad_chord(mix, [45, 52, 57, 60, 64], 8.0, 2.0, 0.4, 1100)
    pad_chord(mix, [48, 55, 60, 64, 67], 10.0, 2.0, 0.4, 1100)
    pad_chord(mix, [41, 53, 57, 60, 64], 12.0, 2.0, 0.42, 1200)
    pad_chord(mix, [43, 55, 59, 62, 67], 14.0, 2.0, 0.42, 1200)
    for b, ch in enumerate([[48, 55, 60, 64], [45, 57, 60, 64], [41, 57, 60, 65], [43, 55, 62, 67]]):
        pad_chord(mix, ch, 16.0 + b * Br, Br, 0.45, 1400, 0.15)
    pad_chord(mix, [36, 43, 48, 55], 20.0, 6.2, 0.5, 900, 0.8)                  # suspense drone under the golden ball
    for k in range(int(8.0 / (Bt / 2))):                                        # light ticks 8-20 s (eighths)
        tb = 8.0 + k * Bt / 2
        if tb < 20.0:
            mix.place('drums', tick(), tb, 0.13 if k % 2 == 0 else 0.08)
    for k in range(int(4.0 / Bt)):                                               # soft pulse 12-20
        for tb in (12.0 + k * Bt, 16.0 + k * Bt):
            mix.place('bass', bass_note(36 if tb < 16 else T.CHORDS[0][0], Bt * 0.8, 0.5, 500), tb, 0.3)
    for k in range(8):
        tb = 16.0 + k * Bt
        mix.place('drums', A.kick(0.8, 0.25, 120, 44), tb, 0.25 if k % 2 == 0 else 0.0)
    # Pascal rows light: every peg of the row pings left -> right (pitch = its column), the row's
    # bell rises one step per row, then the 4,096 chord when the bins light
    for k in range(13):
        tk = T.PASCAL_T0 + T.PASCAL_STEP * k
        for j, tp in enumerate(pascal_times(k)):
            c = 2 * j - k
            mix.place('curve', ping(T.ping_note(c), 0.16), tp, 0.3 / (k + 1) ** 0.35, float(np.clip(c / 13.0, -0.85, 0.85)))
        mix.place('curve', bell(T.pent_note(k, 60), 1.0, 0.6), tk, 0.22, -0.5 + k / 12)
    mix.place('music', A.pad([48, 55, 60, 64, 67, 72], 1.6, cut=2200, att=0.02), T.PASCAL_T0 + T.PASCAL_STEP * 12, 0.45)
    for k in range(8):                                                           # heartbeat kick 12-16
        mix.place('drums', A.kick(0.7, 0.22, 110, 44), 12.0 + k * Bt, 0.24 if k % 2 == 0 else 0.14)
    # golden ball: riser into the gong, then into the groove
    mix.place('fx', A.whoosh(2.2, 300, 6000), M.TLAND[M.GOLD] - 2.2, 0.12)
    mix.place('fx', A.whoosh(1.4, 400, 7000), T.T_GROOVE - 1.4, 0.14)
    # ---- 26-52 groove (bars of 2 s); chords F G C Am, the drop at 46 s on C
    t = T.T_GROOVE
    while t < T.T_DRAIN - 0.01:
        root, padn = T.chord_at(t + 0.01)
        sec = 0 if t < 32 else (1 if t < 40 else (2 if t < 46 else 3))
        pad_chord(mix, padn, t, Br, [0.35, 0.42, 0.48, 0.6][sec], [1000, 1300, 1600, 2400][sec], 0.12)
        for e in range(8):                                                       # bass 8ths
            tb = t + e * Bt / 2
            if tb >= T.T_DRAIN:
                break
            m = root + [0, 0, 12, 0, 7, 0, 12, 7][e] if sec >= 1 else root + [0, 0, 0, 0, 7, 0, 0, 7][e]
            if sec == 0 and e % 2:
                continue
            mix.place('bass', bass_note(m, Bt / 2 * 0.85, 1.0, 700 + 300 * sec), tb, 0.36 if e % 2 == 0 else 0.26)
        for q in range(4):                                                       # beats
            tb = t + q * Bt
            if tb >= T.T_DRAIN:
                break
            kick_on = (q % 2 == 0) if sec == 0 else True
            if kick_on:
                mix.place('drums', A.kick(1.0 if sec < 3 else 1.15, 0.3, 150, 44), tb, 0.5)
            if sec >= 1 and q % 2 == 1:
                mix.place('drums', A.snare(0.13), tb, 0.26 if sec < 3 else 0.32)
                if sec == 3:
                    mix.place('drums', A.clap(), tb, 0.2)
            for s16 in range(4 if sec >= 2 else 2):
                th = tb + s16 * Bt / (4 if sec >= 2 else 2)
                mix.place('drums', A.hat(), th, 0.1 if s16 % 2 == 0 else 0.06)
            if sec >= 1:
                mix.place('drums', A.hat(open_=True), tb + Bt / 2, 0.05)
        t += Br
    # build into the drop: snare roll 44-46 + riser
    for k in range(16):
        tb = 44.0 + k * 0.125
        mix.place('drums', A.snare(0.07), tb, 0.06 + 0.16 * k / 15)
    mix.place('fx', A.whoosh(2.0, 300, 7000), T.T_DROP - 2.0, 0.18)
    mix.place('drums', A.crash(3.0), T.T_DROP, 0.4)
    mix.place('fx', boom(65.4, 1.6), T.T_DROP, 0.6)
    mix.place('drums', A.crash(2.0), 50.0, 0.18)
    # ---- 52-57.5: drain + elevator: thin out, clanks on eighths, pad + bass hold
    mix.place('drums', A.crash(2.5), T.T_DRAIN - 0.4, 0.2)
    mix.place('fx', A.whoosh(1.6, 300, 5000, rise=False), T.T_DRAIN, 0.2)
    pad_chord(mix, [43, 55, 59, 62, 67], 52.0, 2.0, 0.45, 1000)                # G
    pad_chord(mix, [48, 55, 60, 64, 67], 54.0, 2.0, 0.42, 1000)                # C
    pad_chord(mix, [45, 52, 57, 60, 64], 56.0, 2.0, 0.42, 1100)                # Am
    pad_chord(mix, [41, 53, 57, 60, 65], 58.0, 2.0, 0.45, 1300)                # F -> C at the loop
    for k, (tb, m) in enumerate([(52.0, 43), (54.0, 36), (56.0, 45), (58.0, 41)]):
        for e in range(4):
            mix.place('bass', bass_note(m + (12 if e == 2 else 0), Bt * 0.8, 0.8, 600), tb + e * Bt, 0.28)
    a, b = T.T_LIFT
    k = 0
    tb = a
    lift_notes = [T.pent_note(s, 60) for s in range(0, 11)]
    while tb < b:
        mix.place('drums', clank(k), tb, 0.13 + 0.04 * (k % 2 == 0), 0.45)
        u = (tb - a) / (b - a)                                                   # the balls ride back up the scale
        mix.place('curve', marimba(lift_notes[min(10, int(u * 11))] + (12 if k % 2 else 0), 0.3), tb, 0.16, 0.35)
        if k % 2 == 0:
            mix.place('drums', A.kick(0.8, 0.22, 120, 44), tb, 0.28)
        k += 1
        tb += Bt / 2
    mix.place('fx', A.whoosh(2.4, 300, 6500), 60.0 - 2.4, 0.16)               # riser into the loop's hit
    mix.place('drums', A.crash(1.5)[::-1].copy(), 60.0 - 1.5, 0.12)          # reverse cymbal into frame 0


def curve_arp(mix, ts, sm):
    """46-52 s: 16ths picking bin notes with probability = the real bin counts"""
    rng = np.random.default_rng(46)
    t = T.T_DROP
    while t < T.T_DRAIN - 0.05:
        j = min(len(ts) - 1, int(t * 100))
        p = sm[j] / max(sm[j].sum(), 1e-6)
        b = rng.choice(B.NB, p=p)
        mix.place('curve', marimba(T.bin_note(b) + 12, 0.35), t, 0.3, float(np.clip((b - 6) / 7.0, -0.8, 0.8)))
        t += T.BEAT / 4


def hopper_rattle(mix):
    """balls rolling down the chute into the hopper (53.4-57.4) - kept very quiet"""
    rng = np.random.default_rng(7)
    a, b = T.T_LIFT
    for tb in np.sort(rng.uniform(a + 0.3, b, 260)):
        mix.place('land', tock(0.015, int(tb * 100)), float(tb), 0.03, float(rng.uniform(0.1, 0.6)))


def ambience(mix):
    n = NS
    room = A.lp(np.random.default_rng(1).normal(0, 1, n), 320) * 0.04
    mix.place('amb', room, 0.0, 0.5)


def main():
    mix = A.Mix(T.DUR)
    pings(mix)
    landings(mix)
    drain_rattle(mix)
    ts, sm = bin_chord(mix)
    music(mix)
    curve_arp(mix, ts, sm)
    hopper_rattle(mix)
    ambience(mix)
    gains = {'ping': 1.0, 'land': 0.9, 'chord': 1.0, 'curve': 1.0, 'music': 0.85, 'bass': 0.8, 'drums': 1.2, 'fx': 0.9, 'amb': 1.0}
    # carve the music under the pings (300-3000 Hz) so the science sound stays on top
    for k in ('music', 'drums', 'fx', 'bass'):
        if k in mix.buses:
            bb = mix.buses[k]
            mix.buses[k] = bb - 0.6 * A.bp(bb, 300, 3000)
    # wrap tails that run past 60 s back to the start (seamless loop)
    for k, bb in mix.buses.items():
        tail = bb[mix.NS:mix.NS + SR * 5].copy()
        bb[:len(tail)] += tail
        bb[mix.NS:] = 0
    out = mix.master(gains=gains, reverb_buses={'ping': 0.35, 'land': 0.25, 'chord': 0.6, 'curve': 0.4, 'music': 0.9, 'fx': 0.5},
                     gain_db_pts=[(0, 0), (25.5, 0), (26.5, 0.5), (45.8, 0.8), (46.2, 1.5), (52, 1.5), (53, 0.5), (60, 0)])
    A.write(os.path.join(HERE, 'audio.wav'), out)
    spans = [(0, 4), (4, 8), (8, 12), (12, 16), (16, 20), (20, 26), (26, 32), (32, 40), (40, 46), (46, 52), (52, 57.5), (57.5, 60)]
    print('section levels:')
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'  {a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')
    # band check on the dry stems: science (pings, landings, curve arp) vs the music
    from scipy.signal import butter, sosfilt
    sos = butter(4, [300, 3000], 'bp', fs=SR, output='sos')

    def stem(keep):
        return sum(mix.buses[k][:mix.NS] * gains.get(k, 1.0) for k in keep if k in mix.buses).mean(1)
    ys = sosfilt(sos, stem(['ping', 'land', 'curve']))
    yr = sosfilt(sos, stem(['music', 'bass', 'drums', 'fx', 'amb']))
    lvl = lambda y, a, b: 20 * np.log10(np.sqrt((y[int(a * SR):int(b * SR)] ** 2).mean()) + 1e-12)
    print('science band 300-3000 Hz (pings + landings + curve arp) vs the music:')
    for a, b in spans:
        print(f'  {a:5.1f}-{b:5.1f}s  margin {lvl(ys, a, b) - lvl(yr, a, b):5.1f} dB')
    # quietest 0.5 s window anywhere (the brief: never quiet)
    x = out.mean(1)
    w = int(0.5 * SR)
    r = np.sqrt(np.convolve(x * x, np.ones(w) / w, mode='valid'))[::SR // 20]
    print(f'quietest 0.5 s window: {20 * np.log10(r.min() + 1e-12):.1f} dBFS at {np.argmin(r) / 20:.2f} s')


if __name__ == '__main__':
    main()
