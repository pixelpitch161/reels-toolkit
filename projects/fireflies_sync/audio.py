"""Fireflies That Sync soundtrack. Everything synthesised; the arrangement is original.

The science drives the sound:
- every male flash in sim.py is a bell note at that fly's own pentatonic pitch (left = low, right = high), at the
  exact flash time. Random flashing = a chaotic sparkle; the lock-in smear -> flam -> unison is literally the
  20 flies' notes converging; a synchronised burst = one chord per flash (8 per burst, 0.55 s apart = 109 BPM);
- the burst rolling left -> right at 0.5 m/s pans and climbs (pitch follows x), doubled by an arpeggio sweep;
  in the climax two sweeps converge on the beat where the fronts meet;
- every female answer is a soft low kalimba note in the males' dark pause (call and response);
- the groove holds the dark pauses (the build); each burst is the drop.

Loop: the opening/ending sparkle and drone are one process, and the mix's tail past 60 s is folded onto 0 s.

    python audio.py      -> audio.wav, stems/sci.wav, stems/music.wav; prints levels and the bell-band check"""
import math
import os
import sys
import numpy as np
from scipy.io import wavfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit import audio as A
from reelkit.audio import SR, tt, mtof, lp, hp, bp, expenv, adsr, saw
import timeline as T
import sim as M

rng = np.random.default_rng(7)
DUR = T.DUR
EXTRA = 6.0                      # rendered past 60 s, then folded onto the start (seamless loop)
G = T.grid


# ------------------------------------------------------------------ instruments
def bell(m, dur=1.5):
    n = int(dur * SR)
    t = tt(n)
    f = mtof(m)
    k = 1.0 - 0.35 * (m - 74) / 24                       # higher bells ring shorter
    x = (np.sin(2 * np.pi * f * t) * np.exp(-t / (0.55 * k))
         + 0.32 * np.sin(2 * np.pi * 2.0 * f * t) * np.exp(-t / (0.2 * k))
         + 0.17 * np.sin(2 * np.pi * 2.76 * f * t) * np.exp(-t / (0.11 * k))
         + 0.07 * np.sin(2 * np.pi * 5.4 * f * t) * np.exp(-t / 0.045))
    x *= np.clip(t / 0.0015, 0, 1)
    click = hp(np.random.default_rng(m).normal(0, 1, n), 3000) * np.exp(-t / 0.0025) * 0.12
    return x + click


BELL = {m: bell(m) for m in T.BELLS}


def kalimba(m, dur=1.1):
    n = int(dur * SR)
    t = tt(n)
    f = mtof(m) * (1 + 0.025 * np.exp(-t / 0.04))
    ph = 2 * np.pi * np.cumsum(f) / SR
    x = np.sin(ph) * np.exp(-t / 0.42) + 0.22 * np.sin(2 * ph) * np.exp(-t / 0.18) + 0.12 * np.sin(5.9 * ph) * np.exp(-t / 0.03)
    x *= np.clip(t / 0.006, 0, 1)
    return lp(x, 2600)


def bass_note(m, dur, cut=420):
    n = int(dur * SR)
    t = tt(n)
    f = mtof(m)
    x = 0.85 * np.sin(2 * np.pi * f * t) + 0.35 * lp(saw(f, n, det=0.004, voices=2), cut)
    return x * adsr(n, 0.004, 0.12, 0.75, 0.05)


def lead(m, dur):
    n = int(dur * SR)
    t = tt(n)
    f = mtof(m) * (1 + 0.006 * np.sin(2 * np.pi * 5.2 * t) * np.clip((t - 0.15) / 0.2, 0, 1))
    ph = 2 * np.pi * np.cumsum(f) / SR
    x = np.sin(ph) + 0.18 * np.sin(2 * ph) + 0.06 * np.sin(3 * ph)
    return lp(x, 1800) * adsr(n, 0.03, 0.15, 0.8, min(0.25, dur * 0.4))


def pluck(m, dur=0.6):
    n = int(dur * SR)
    t = tt(n)
    f = mtof(m)
    x = np.sin(2 * np.pi * f * t) + 0.3 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.08)
    return x * expenv(n, 0.22, 0.002)


def pad(ms, dur, cut=1100, att=0.4):
    n = int(dur * SR)
    x = sum(saw(mtof(m), n, det=0.006, voices=3) for m in ms) / max(1, len(ms))
    return lp(x, cut) * adsr(n, att, 0.3, 0.85, min(0.8, dur * 0.4))


def shaker():
    n = int(0.08 * SR)
    t = tt(n)
    return hp(np.random.default_rng(11).normal(0, 1, n), 6000) * np.exp(-t / 0.018) * 0.4


def riser(dur):
    n = int(dur * SR)
    t = tt(n)
    sweep = np.sin(2 * np.pi * np.cumsum(180 * (7.0 ** (t / dur))) / SR) * (t / dur) ** 2 * 0.25
    return A.whoosh(dur, 300, 7000, rise=True) * np.linspace(0.2, 1, n) ** 1.5 + sweep


def thump():
    """soft low pulse: the beat emerging from the flashes"""
    return lp(A.kick(punch=0.8, dec=0.2, f0=110, f1=50), 300)


KICK = A.kick()
CLAP = hp(A.clap(), 1200)
HAT = A.hat()
SNARE = hp(A.snare(), 300)
CRASH = A.crash(2.4)


class Mix2(A.Mix):
    """Mix with a long buffer so tails past 60 s can be folded onto the start"""
    def __init__(self):
        super().__init__(DUR + EXTRA)


mix = Mix2()


# ------------------------------------------------------------------ 1) bells from the flash schedule
def bell_events():
    ev_t, ev_i = M.EV_T, M.EV_I
    P = M.P[ev_i]
    h = M.P[0]
    dist = np.linalg.norm(P - h, axis=1)
    g = np.where(ev_i == 0, 1.0, np.where(ev_i < 100, 0.55, np.where(ev_i < 500, 0.4, 0.12)))
    g = g / (1 + dist / 6.0)
    # the hero's stray notes in the macro sit on top
    hero_macro = (ev_i == 0) & (ev_t > 3.9) & (ev_t < 8.2)
    g[hero_macro] *= 2.2
    random_phase = ((ev_t < T.T_A - 0.05) | (ev_t > T.T_RANDOM_BACK)) & ~hero_macro
    g[random_phase] *= 1.9
    g[(ev_t >= T.T_A - 0.2) & (ev_t < T.T_A_END)] *= 1.8          # the lock-in: 20 flies, front and centre
    pitch = np.array(T.BELLS)[M.PITCH[ev_i]]
    xs = np.where(ev_i < 500, P[:, 0] / 3.6, P[:, 0] / 14.0)
    # group into 12 ms bins per pitch (thousands of flashes in the climax)
    key = np.round(ev_t / 0.012).astype(np.int64) * 100 + pitch
    order = np.argsort(key)
    key, ev_t, g, pitch, xs = key[order], ev_t[order], g[order], pitch[order], xs[order]
    starts = np.r_[0, np.nonzero(np.diff(key))[0] + 1]
    ends = np.r_[starts[1:], len(key)]
    n = 0
    for a, b in zip(starts, ends):
        gs = g[a:b].sum()
        amp = 0.16 * min(gs, 3.2) ** 0.62
        pan = float(np.clip((xs[a:b] * g[a:b]).sum() / gs, -0.85, 0.85))
        t0 = float(ev_t[a:b].min())
        mix.place('bells', BELL[int(pitch[a])], t0, amp, pan)
        n += 1
    return n


nb = bell_events()

# ------------------------------------------------------------------ 2) female answers
for t, i in zip(M.FE_T, M.FE_I):
    p = M.PF[i]
    far = i >= M.N_FN
    m = T.FEMALE_NOTES[M.PITCH_F[i]]
    pan = float(np.clip(p[0] / (14.0 if far else 2.0), -0.8, 0.8))
    mix.place('fem', kalimba(m), t, 0.11 if far else 0.32, pan)

# ------------------------------------------------------------------ 3) opening / ending drone (one process, wrapped)
drone_ch = [38, 45, 50, 57, 64]          # D2 A2 D3 A3 E4 (Dadd9, open and hushed)
for t0 in (55.6,):
    x = pad(drone_ch, 72.0 - t0 + 1.5, cut=700, att=1.2)
    x[:int(0.8 * SR)] *= np.linspace(0, 1, int(0.8 * SR))
    lfo = 0.8 + 0.2 * np.sin(2 * np.pi * tt(len(x)) / 4.0)
    mix.place('pad', x * lfo, t0, 0.75)
# the drone hands over to the chord pads as the lock-in burst begins (fold below wraps 60-72 s onto 0-12 s)
# chord pads, bar by bar from burst A to the end of the climax's tail
for b in range(-16, 33 * 4, 4):
    tb = G(b)
    if tb >= 55.4:
        break
    ch = T.CHORDS[T.chord_at(tb + 0.01)]
    ph, k, _ = T.phase(tb + 0.01)
    cut = 900 if ph == 'dark' else 1500
    mix.place('pad', pad(ch['pad'], T.BAR + 0.35, cut=cut, att=0.25), tb, 0.36 if ph == 'burst' else 0.3)

# ------------------------------------------------------------------ 4) drums, bass, arps, risers
def beats(b0, b1):
    return range(b0, b1)


# burst A: a soft pulse emerges as the flies lock in (gain follows the model's coherence)
for b in beats(-16, -8):
    phi = M.phi_at(G(b))
    R = abs(np.exp(1j * phi).mean())
    mix.place('drums', thump(), G(b), 0.15 + 0.6 * R ** 4)
# gap A: kick + bass enter on the flash beat, build to the first drop
for b in beats(-8, 0):
    tb = G(b)
    mix.place('drums', KICK, tb, 0.55 + 0.25 * (b + 8) / 8)
    if b >= -4:
        mix.place('drums', HAT, tb + T.BEAT / 2, 0.16)
    ch = T.CHORDS[T.chord_at(tb + 0.01)]
    for e in range(2):
        mix.place('bass', bass_note(ch['bass'] + 12 * (e % 2), T.BEAT / 2 * 0.9), tb + e * T.BEAT / 2, 0.42)
for k in range(16):                                         # snare roll into the drop
    mix.place('drums', SNARE, G(-4) + k * T.BEAT / 4, 0.06 + 0.22 * (k / 15) ** 2)


def drop(b0, big):
    """a burst: kick on every flash, clap on 2 & 4, 8th hats, 8th bass"""
    mix.place('fx', CRASH, G(b0), 0.35 + 0.1 * big)
    for k in range(8):
        tb = G(b0 + k)
        mix.place('drums', KICK, tb, 0.9)
        if k % 2:
            mix.place('drums', CLAP, tb, 0.42)
        mix.place('drums', HAT, tb + T.BEAT / 2, 0.22)
        mix.place('drums', HAT, tb, 0.08)
        ch = T.CHORDS[T.chord_at(tb + 0.01)]
        for e in range(2):
            mix.place('bass', bass_note(ch['bass'] + 12 * (e % 2), T.BEAT / 2 * 0.9, cut=520 + 120 * big), tb + e * T.BEAT / 2, 0.5)


def dark(b0, nbeats, build_from, intensity):
    """the dark pause = the build: half-time groove, then a riser + roll into the next drop"""
    for k in range(nbeats):
        b = b0 + k
        tb = G(b)
        late = k >= build_from
        if k % 2 == 0 or (late and k >= nbeats - 4):
            mix.place('drums', KICK, tb, 0.7 if not late else 0.8)
        if k % 4 == 2:
            mix.place('drums', CLAP, tb, 0.3)
        mix.place('drums', HAT, tb + T.BEAT / 2, 0.13 + 0.05 * intensity)
        if late:
            for s in range(4):
                mix.place('drums', shaker(), tb + s * T.BEAT / 4, 0.1 + 0.15 * (k - build_from) / (nbeats - build_from))
        ch = T.CHORDS[T.chord_at(tb + 0.01)]
        if k % 4 == 0:
            mix.place('bass', bass_note(ch['bass'], T.BAR * 0.6, cut=380), tb, 0.48)
        if k % 4 == 3:
            mix.place('bass', bass_note(ch['bass'] + 12, T.BEAT * 0.45, cut=380), tb + T.BEAT / 2, 0.36)
    rd = (nbeats - build_from) * T.BEAT
    mix.place('fx', riser(rd), G(b0 + build_from), 0.22 + 0.1 * intensity)
    for k in range(16):
        mix.place('drums', SNARE, G(b0 + nbeats - 4) + k * T.BEAT / 4, 0.05 + (0.2 + 0.06 * intensity) * (k / 15) ** 2)


drop(0, 0)
dark(8, 16, 8, 0)
drop(24, 1)
dark(32, 16, 8, 1)
drop(48, 2)
# after the climax: groove rides out under the females' answers, then hands back to the sparkle + drone
for k in range(0, 15):
    b = 56 + k
    tb = G(b)
    if tb > 55.5:
        break
    if k % 2 == 0:
        mix.place('drums', KICK, tb, 0.7 * (1 - k / 18))
    if k % 4 == 2:
        mix.place('drums', CLAP, tb, 0.28)
    mix.place('drums', HAT, tb + T.BEAT / 2, 0.14)
    ch = T.CHORDS[T.chord_at(tb + 0.01)]
    if k % 4 == 0:
        mix.place('bass', bass_note(ch['bass'], T.BAR * 0.7, cut=360), tb, 0.45)

# the burst rolling left -> right: arpeggio sweep (32 sixteenths) climbing and panning with the front
LADDER = [62, 64, 66, 69, 71, 74, 76, 78, 81, 83, 86]
for k in range(32):
    u = k / 31
    mix.place('arp', pluck(LADDER[int(round(u * 10))]), G(24) + k * T.BEAT / 4, 0.2, -0.85 + 1.7 * u)
# climax: two sweeps converge on the beat where the fronts meet, then the whole valley ping-pongs
for k in range(16):
    u = k / 16
    mix.place('arp', pluck(LADDER[int(round(u * 5))]), G(48) + k * T.BEAT / 4, 0.18, -0.85 * (1 - u))
    mix.place('arp', pluck(LADDER[10 - int(round(u * 5))]), G(48) + k * T.BEAT / 4 + T.BEAT / 8, 0.18, 0.85 * (1 - u))
CHD = [62, 66, 69, 74, 78, 81, 86, 81, 78, 74, 69, 66, 62, 66, 69, 74]
for k in range(16):
    mix.place('arp', pluck(CHD[k]), G(52) + k * T.BEAT / 4, 0.2, 0.7 if k % 2 else -0.7)
# original melody: hinted (low, soft) on burst 2, full on the climax and its tail
MEL = [(0, 69, 0.5), (0.5, 74, 0.5), (1, 76, 1.0), (2, 78, 0.75), (2.75, 76, 0.25), (3, 74, 1.0),
       (4, 71, 0.5), (4.5, 74, 0.5), (5, 78, 1.0), (6, 76, 0.5), (6.5, 74, 0.5), (7, 71, 1.0),
       (8, 74, 1.5), (9.5, 71, 0.5), (10, 69, 1.0), (11, 71, 1.0), (12, 76, 1.0), (13, 74, 0.5), (13.5, 71, 0.5), (14, 69, 1.8)]
for (bo, m, d) in MEL[:12]:
    mix.place('lead', lead(m - 12, d * T.BEAT), G(24 + bo), 0.16)
for (bo, m, d) in MEL:
    mix.place('lead', lead(m, d * T.BEAT), G(48 + bo), 0.3)
# gap 1/2: the lead answers the females with two soft notes per pause (keeps the dark musical)
for b0 in (8, 32):
    for (bo, m, d) in [(4, 69, 1.5), (6, 71, 1.5), (12, 74, 2.0), (14.5, 76, 1.2)]:
        mix.place('lead', lead(m, d * T.BEAT), G(b0 + bo), 0.12)
# risers into burst A and a whoosh on the macro push
mix.place('fx', riser(T.T_A - 10.4), 10.4, 0.16)
mix.place('fx', A.whoosh(1.6, 400, 6000), 3.6, 0.12)
mix.place('fx', A.whoosh(1.4, 6000, 400, rise=False, seed=9), 55.2, 0.12)

# ------------------------------------------------------------------ 5) ambience: creek + night air (very low)
n = mix.NS + SR * 6
r = np.random.default_rng(3)
creek = bp(r.normal(0, 1, n), 500, 2200) * (0.7 + 0.3 * np.sin(2 * np.pi * tt(n) * 0.31)) * 0.05
air = lp(r.normal(0, 1, n), 400) * 0.08
mix.place('amb', np.stack([creek + air, np.roll(creek, 900) + air], 1), 0.0, 1.0)

# ------------------------------------------------------------------ 6) breaths before each drop, carve the bell band
for tb in (T.T_B1, T.T_B2, T.T_B3):
    for name in ('drums', 'bass', 'pad', 'arp', 'lead'):
        bb = mix.bus(name)
        i0, i1 = int((tb - 0.16) * SR), int(tb * SR) - 64
        bb[i0:i1] *= np.linspace(1, 0, i1 - i0)[:, None] ** 3
MUSIC = ['drums', 'bass', 'pad', 'arp', 'lead', 'fx']
for name in MUSIC:
    bb = mix.bus(name)
    bb -= 0.62 * bp(bb, 1000, 5200)                         # ~ -8 dB in the bells' band


def master(buses, gains, sends, gain_db_pts, thr=0.26, ceiling=0.89):
    NS = int(DUR * SR)
    dry = sum(b * gains.get(k, 1.0) for k, b in buses.items())
    send = sum(buses[k] * v for k, v in sends.items() if k in buses)
    wet = A.reverb(send, dur=2.8, decay=0.7) * 0.24
    full = dry + wet
    out = full[:NS].copy()
    ne = min(len(full) - NS, int(EXTRA * SR))
    out[:ne] += full[NS:NS + ne]                              # fold the tail onto the start: seamless loop
    g = np.interp(tt(NS), [p[0] for p in gain_db_pts], [p[1] for p in gain_db_pts])
    out = out * (10 ** (g / 20))[:, None]
    from scipy.signal import lfilter
    pre = int(SR)                                             # run the envelope around the loop seam
    e2 = np.r_[(out[-pre:] ** 2).mean(1), (out ** 2).mean(1)]
    env = np.sqrt(lfilter([0.002], [1, -0.998], e2))[pre:]
    gain = np.where(env > thr, (thr / np.maximum(env, 1e-9)) ** 0.45, 1.0)
    out = out * gain[:, None]
    out *= ceiling / (np.abs(out).max() + 1e-9)
    out = np.tanh(out * 1.5) / np.tanh(1.5)
    return out * ceiling / (np.abs(out).max() + 1e-9), g, gain


GAINS = {'bells': 1.0, 'fem': 1.0, 'drums': 0.62, 'bass': 0.6, 'pad': 0.42, 'arp': 0.5, 'lead': 0.5, 'fx': 0.6, 'amb': 0.5}
SENDS = {'bells': 0.8, 'fem': 0.6, 'pad': 0.5, 'lead': 0.45, 'arp': 0.55, 'fx': 0.3}
PTS = [(0, 0), (11.5, 0.4), (12.2, 1.2), (16.6, 1.4), (21.0, 2.6), (25.4, 1.8), (34.2, 3.4), (38.6, 2.4), (47.4, 5.4), (51.8, 3.6), (55.5, 0.6), (56.5, 0), (60, 0)]


def main():
    out, g, comp = master(mix.buses, GAINS, SENDS, PTS)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    # stems on the same scale (same automation + the master's gain curve) for the band checks
    os.makedirs(os.path.join(HERE, 'stems'), exist_ok=True)
    NS = int(DUR * SR)
    for name, keys in [('sci', ['bells', 'fem']), ('music', MUSIC + ['amb'])]:
        sub = {k: mix.buses[k] for k in keys}
        dry = sum(b * GAINS.get(k, 1.0) for k, b in sub.items())
        send = sum(sub[k] * v for k, v in SENDS.items() if k in sub)
        full = dry + (A.reverb(send, dur=2.8, decay=0.7) * 0.24 if not np.isscalar(send) else 0)
        st = full[:NS].copy()
        st[:int(EXTRA * SR)] += full[NS:NS + int(EXTRA * SR)]
        st = st * (10 ** (g / 20))[:, None] * comp[:, None]
        wavfile.write(os.path.join(HERE, 'stems', f'{name}.wav'), SR, (np.clip(st * 0.25, -1, 1) * 32767).astype(np.int16))
    print(f'bell placements: {nb}')
    spans = [(0, 4), (4, 8), (8, 12.2), (12.2, 16.6), (16.6, 21), (21, 25.4), (25.4, 30), (30, 34.2), (34.2, 38.6),
             (38.6, 43), (43, 47.4), (47.4, 51.8), (51.8, 56), (56, 60)]
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


if __name__ == '__main__':
    main()
