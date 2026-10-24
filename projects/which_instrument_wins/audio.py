"""#26 Which Instrument Wins? - soundtrack.

Science sound (on top): every wall bounce in the honest run plays that ball's own Pixel Pitch voice. Pitch = where it
hits (left -> right = low -> high through the current chord's tones, inside the instrument's register), loudness =
impact speed, timing quantised to the 32nd grid. Gap-edge hits ring a small bell (a ball rattling in the gap is a
run of tings), glass balls clink softly, an elimination whooshes down and lands in the pit on one last dull note.
Music (under it): a dark D-minor tango at 118.03 BPM (30 bars = 61 s): habanera bass + staccato strings, eighths
from bar 10, a dominant pedal with a tremolo for the duel, the win resolves it, then the winner's solo, and a
pickup run that lands on the downbeat at frame 0. The mix wraps around: tails past 61 s fold onto the start.

    python audio.py            -> audio.wav (+ stems in check/ for the band check)
"""
import math
import os
import sys
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', '..'))
sys.path.insert(0, HERE)
from reelkit import audio as A
from reelkit.ui import impact_landings
import timeline as T
import sim
import pp_bridge as B

SR = A.SR
RUN = sim.load()
EV = RUN['EV']
T_OUT, T_LAND, T_WIN = RUN['T_OUT'], RUN['T_LAND'], RUN['T_WIN']
WINNER = int(np.where(T_OUT < 0)[0][0])
NAMES = B.NAMES
Q32 = T.BEAT / 8


def q(t):
    return round(t / Q32) * Q32


# ---------------------------------------------------------------- instrument voices (cached, level-matched)
_VC = {}
_GAIN = {}


def _fit(x, dur, cap):
    n = int(cap * SR)
    x = np.asarray(x, float)[:n].copy()
    if len(x) < n:
        x = np.pad(x, (0, n - len(x)))
    fd = int(0.06 * SR)
    x[-fd:] *= np.linspace(1, 0, fd)
    return x


def inst_gain(name):
    if name not in _GAIN:
        lo, hi = B.entrant(name)[3]
        x = B.voice(name, (lo + hi) // 2, 0.8, 0.18)[:int(0.3 * SR)]
        x = A.hp(x, 120) if name != 'TUBA' else x
        r = np.sqrt((x ** 2).mean()) + 1e-6
        _GAIN[name] = min(4.0, 0.16 / r)
    return _GAIN[name]


def note(name, midi, dur=0.16, cap=0.5):
    key = (name, int(midi), round(dur, 3), cap)
    if key not in _VC:
        x = B.voice(name, int(midi), 0.85, dur)
        _VC[key] = _fit(x, dur, cap) * inst_gain(name)
    return _VC[key]


def pitch_for(name, chord, xfrac):
    """chord tones inside the instrument's register, picked left (low) -> right (high) by where the ball hit"""
    lo, hi = B.entrant(name)[3]
    if name == 'DRUMS':
        kit = [36, 41, 45, 38, 48, 38]
        return kit[min(len(kit) - 1, int(xfrac * len(kit)))]
    pcs = T.CHORD_TONES[chord]
    cand = [m for m in range(lo, hi + 1) if m % 12 in pcs]
    return cand[min(len(cand) - 1, int(np.clip(xfrac, 0, 0.999) * len(cand)))]


# ---------------------------------------------------------------- bed synths (original)
def strings_stab(ms, dur=0.13, bright=2400):
    n = int((dur + 0.12) * SR)
    x = sum(A.saw(A.mtof(m), n, det=0.004, voices=3) for m in ms) / len(ms)
    env = A.adsr(n, 0.004, 0.06, 0.55, 0.09)
    env[int(dur * SR):] *= np.exp(-np.arange(n - int(dur * SR)) / SR / 0.04)
    return A.lp(x, bright) * env


def tremolo(ms, dur):
    n = int(dur * SR)
    t = A.tt(n)
    x = sum(A.saw(A.mtof(m), n, det=0.006, voices=3) for m in ms) / len(ms)
    am = 0.55 + 0.45 * np.sin(2 * np.pi * (T.BPM / 60 * 4) * t)            # 16th tremolo, locked to the grid
    return A.lp(x, 1800) * am * A.adsr(n, 0.25, 0.1, 0.9, 0.2)


def dbass(m, dur=0.22):
    n = int((dur + 0.25) * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = np.sin(2 * np.pi * f * t) + 0.35 * np.sin(4 * np.pi * f * t) + 0.12 * A.saw(f, n)
    return A.lp(x, 900) * A.expenv(n, 0.22, 0.004)


def timp(m, dur=1.2):
    n = int(dur * SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = np.sin(2 * np.pi * f * t) + 0.5 * np.sin(2 * np.pi * f * 1.5 * t) * np.exp(-t / 0.2)
    return x * A.expenv(n, 0.45, 0.003) + A.lp(np.random.default_rng(9).normal(0, 1, n), 400) * np.exp(-t / 0.03) * 0.4


def roll(dur, f0=0.4):
    """snare roll crescendo (32nds)"""
    n = int(dur * SR)
    out = np.zeros(n)
    k, t = 0, 0.0
    while t < dur - 0.01:
        a = int(t * SR)
        s = A.snare(0.06) * (f0 + (1 - f0) * t / dur)
        out[a:a + len(s)] += s[:n - a]
        t += Q32 / 1.0
        k += 1
    return out


def chord_notes(ch, octave=0):
    r = T.CHORD_ROOT[ch] + 12 + 12 * octave
    return [r + (p - T.CHORD_ROOT[ch]) % 12 for p in T.CHORD_TONES[ch]]


def main():
    mix = A.Mix(T.DUR + 4.0)
    # ------------------------------------------------ the bed
    for b in range(T.N_BARS):
        t0 = b * T.BAR
        ch = T.CHORDS[b]
        root = T.CHORD_ROOT[ch]
        solo = T.SOLO_BAR <= b < T.SOLO_BAR + 4
        duel = 22 <= b <= 24
        pickup = b == T.N_BARS - 1
        # habanera bass, two cells per bar: root . . root | fifth third
        for cell in range(2):
            c0 = t0 + cell * 2 * T.BEAT
            fifth = root + 7 if root + 7 <= 52 else root - 5
            third = root + (3 if ch in ('Dm', 'Gm') else 4)
            pat = [(0.0, root, 1.0), (0.75, root, 0.7), (1.0, fifth, 0.85), (1.5, third, 0.75)]
            if pickup and cell == 1:
                pat = [(0.0, root, 1.0), (0.5, root + 2, 0.8), (1.0, root + 4, 0.85), (1.5, root + 7, 0.9)]
            for off, m, v in pat:
                if b == 24 and c0 + off * T.BEAT >= T_WIN:
                    continue
                mix.place('bass', dbass(m), c0 + off * T.BEAT, 0.55 * v)
        # staccato strings: marcato quarters, eighths once the pace picks up; sparse under the solo
        sts = chord_notes(ch, octave=1)                       # violin register: the tango's staccato strings
        if solo:
            hits = [0, 2]
        elif t0 >= T.T_FASTER and not pickup:
            hits = [k * 0.5 for k in range(8)]
        else:
            hits = [0, 1, 2, 3]
        for h in hits:
            th = t0 + h * T.BEAT
            if b == 24 and th >= T_WIN:
                continue
            acc = 1.0 if h in (0, 2) else 0.72
            mix.place('strings', strings_stab(sts + [sts[0] + 12]), th, 1.0 * acc, pan=-0.25)
        # light pulse: kick on 1 and 3, a rim "chick" on the habanera's dotted note
        for k in range(4):
            tk = t0 + k * T.BEAT
            if b == 24 and tk >= T_WIN:
                continue
            if k % 2 == 0:
                mix.place('drums', A.kick(0.8, 0.25, 120, 44), tk, 0.33 if not solo else 0.2)
            mix.place('drums', A.hat(0.03), tk + 0.5 * T.BEAT, 0.07)
        if duel and b < 24:
            mix.place('strings', tremolo([57, 61, 64, 67], T.BAR + 0.05), t0, 0.16 + 0.06 * (b - 22), pan=0.25)
            mix.place('drums', timp(33, 0.8), t0, 0.28)
        if b == 24:
            mix.place('strings', tremolo([57, 61, 64, 67, 70], T_WIN - t0), t0, 0.26, pan=0.25)
            n_roll = max(0.0, T_WIN - (t0 + 2 * T.BEAT))
            if n_roll > 0.2:
                mix.place('drums', roll(n_roll, 0.2), t0 + 2 * T.BEAT, 0.16)
        if pickup:
            mix.place('drums', roll(T.BAR, 0.15), t0, 0.2)
            run = [45, 49, 52, 55, 57, 61, 64, 67]
            for k, m in enumerate(run):
                mix.place('strings', strings_stab([m, m + 12], 0.1, 3000), t0 + 2 * T.BEAT + k * T.BEAT / 4, 0.7 + 0.06 * k)
    # the win: a big tonic hit (timpani, crash, full chord), held, then a roar from the stands
    tw = q(T_WIN)
    mix.place('drums', A.kick(1.2, 0.5, 160, 40), tw, 0.6)
    mix.place('drums', timp(38, 2.0), tw, 0.55)
    mix.place('drums', A.crash(2.4), tw, 0.22)
    mix.place('strings', A.pad([50, 57, 62, 65, 69, 74], T.T_SOLO - tw + 0.3, cut=2600, att=0.01), tw, 0.22)
    roar = A.lp(A.hp(np.random.default_rng(12).normal(0, 1, int(3.5 * SR)), 300), 2200) * np.sin(np.linspace(0, np.pi, int(3.5 * SR))) ** 1.5
    mix.place('fx', roar, tw + 0.1, 0.05)
    # the pickup swells right up to the file boundary (reverse cymbal), so the seam lands on the frame-0 downbeat
    rc = A.crash(1.4)[::-1].copy()
    mix.place('drums', rc, NA / SR - len(rc) / SR, 0.5)
    # downbeat hit at frame 0 (the pickup lands here)
    mix.place('drums', A.crash(1.6), 0.0, 0.16)
    # ------------------------------------------------ the science: bounces in the run
    alive = 16
    last_note = np.full(16, -9.0)
    n_alive_at = lambda t: 16 - int(((T_OUT >= 0) & (T_OUT <= t)).sum())
    t_cut = T_WIN + 0.05
    for e in EV:
        t, b, kind, sp, x, y = e[0], int(e[1]), int(e[2]), e[3], e[4], e[5]
        if t > t_cut and kind in (0, 1, 2):
            continue
        name = NAMES[b]
        nal = n_alive_at(t)
        bal = 1.0 / math.sqrt(max(1.0, nal / 2.0))          # 16 voices at once -> each quieter; a duel -> full
        if kind == 0 and sp >= 1.2:
            if t - last_note[b] < 0.15:
                continue
            last_note[b] = t
            tq = max(0.0, q(t))
            ch = T.chord_at(tq)
            m = pitch_for(name, ch, (x / (T.R_RING + 0.3) + 1) / 2)
            v = float(np.clip(sp / 7.0, 0.3, 1.0))
            mix.place('bounce', note(name, m), tq, 0.5 * v * bal, pan=float(np.clip(x / 3.0, -0.6, 0.6)))
        elif kind == 2:
            if t - last_note[b] < Q32 * 0.9:
                continue
            first = t - last_note[b] > 0.4
            last_note[b] = t
            tq = q(t)
            mix.place('fx', A.chime(100, 0.25), tq, 0.05 + 0.03 * min(1.0, sp / 6), pan=float(np.clip(x / 3.0, -0.6, 0.6)))
            if first:
                ch = T.chord_at(tq)
                m = pitch_for(name, ch, (x / (T.R_RING + 0.3) + 1) / 2)
                mix.place('bounce', note(name, m), tq, 0.55 * bal)
        elif kind == 1 and sp >= 2.0 and nal <= 12:
            o = int(e[6])
            if t - last_note[b] < 0.1:
                continue
            mix.place('fx', A.chime(103 + (b % 3), 0.12), q(t), 0.018, pan=float(np.clip(x / 3.0, -0.6, 0.6)))
        elif kind == 3:
            mix.place('fx', A.whoosh(0.45, 400, 3000, rise=False, seed=b), t, 0.08, pan=float(np.clip(x / 3.0, -0.6, 0.6)))
        elif kind == 4:
            lo, hi = B.entrant(name)[3]
            ch = T.chord_at(t)
            m = min(c for c in range(lo, hi + 1) if c % 12 == T.CHORD_ROOT[ch] % 12) if name != 'DRUMS' else 41
            dull = A.lp(note(name, m, 0.3, 0.7), 900)
            mix.place('bounce', dull, t, 0.45)
            mix.place('fx', A.thud(0.3, 90, 40, seed=b), t, 0.22)
    # ------------------------------------------------ the winner's solo (its own voice, the motif in its register)
    wname = NAMES[WINNER]
    lo, hi = B.entrant(wname)[3]
    shift = 0
    while max(m for _, m, _ in T.SOLO) + shift > hi + 7:
        shift -= 12
    while min(m for _, m, _ in T.SOLO) + shift < lo - 2:
        shift += 12
    for bt, m, d in T.SOLO:
        dur = d * T.BEAT * 0.95
        x = B.voice(wname, m + shift, 0.9, dur)
        x = _fit(x, dur, dur + 0.5) * inst_gain(wname)
        mix.place('solo', x, T.T_SOLO + bt * T.BEAT, 0.62, pan=0.0)
    # the fly-back: each ball plays one note as it lifts off, a rising cascade on the pickup chord into frame 0
    import render as render_caps
    starts = sorted((render_caps.reset_start(b), b) for b in range(16))
    for k, (ts, b) in enumerate(starts):
        name = NAMES[b]
        m = pitch_for(name, 'A7', min(0.99, 0.15 + 0.85 * k / 15))
        mix.place('bounce', note(name, m), q(ts), 0.17, pan=float(np.clip((k - 7.5) / 12, -0.6, 0.6)))
    # caption landings: soft thuds
    for tl, k, nch in impact_landings(render_caps.CAPTIONS):
        mix.place('fx', A.thud(), tl, 0.10 if k == 0 else 0.07)
    out, stems = master(mix)
    A.write(os.path.join(HERE, 'audio.wav'), out)
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    for k, s in stems.items():
        A.write(os.path.join(HERE, 'check', f'stem_{k}.wav'), s)
    spans = [(0, 4), (4, 20), (20, 32), (32, 44), (44, 49.4), (49.4, 51), (51, 58.9), (58.9, 61)]
    for a, b, rms, pk in A.section_levels(os.path.join(HERE, 'audio.wav'), spans):
        print(f'{a:5.1f}-{b:5.1f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


BUS_GAIN = {'bounce': 3.6, 'solo': 1.9, 'fx': 1.6, 'bass': 0.55, 'strings': 1.35, 'drums': 0.6}
# section automation (dB): the wall of 16 at full, the middle pulled back, then a build through the duel, the win
# and the solo; ends where it starts so the seam has no level step
GAIN_PTS = [(0, 0.0), (3.6, 0.0), (5.0, -4.0), (20, -4.0), (32, -2.5), (40, -1.0), (44, 0.8), (49.3, 1.8),
            (49.5, 3.0), (51, 2.5), (58.9, 2.0), (60.7, 0.0), (61.0, 0.0)]


# AAC codes 1024-sample frames and 61.0 s isn't a whole number of them (2859.4), so the encoder would pad the last one
# with 13 ms of zeros, a dropout right before the loop (lesson from #23). The soundtrack is built as a loop exactly
# 2860 AAC frames long (61.0133 s) and muxed untrimmed (mux.py), so the decoded audio wraps sample-perfectly.
NA = -(-int(round(T.DUR * SR)) // 1024) * 1024


def master(mix, thr=0.22, ceiling=0.89):
    """like reelkit Mix.master, but the overflow past the loop length (note and reverb tails) folds onto the start,
    so frame 0 isn't dry; returns the master and the scaled stems (NA samples: whole AAC frames)"""
    NS = NA
    buses = {k: v * BUS_GAIN.get(k, 1.0) for k, v in mix.buses.items()}
    send = buses.get('strings', 0) * 0.8 + buses.get('bounce', 0) * 0.5 + buses.get('solo', 0) * 0.7 + buses.get('fx', 0) * 0.4
    wet = A.reverb(send) * 0.22
    full = sum(buses.values()) + wet

    def fold(x):
        y = x[:NS].copy()
        tail = x[NS:]
        y[:len(tail)] += tail[:NS]
        return y
    m = fold(full)
    g = np.interp(np.arange(NS) / SR, [p[0] for p in GAIN_PTS] + [NA / SR], [p[1] for p in GAIN_PTS] + [GAIN_PTS[-1][1]])
    gl = (10 ** (g / 20))[:, None]
    m = m * gl
    from scipy.signal import lfilter
    env = np.sqrt(lfilter([0.002], [1, -0.998], (np.concatenate([m[-SR:], m]) ** 2).mean(1)))[SR:]
    gain = np.where(env > thr, (thr / np.maximum(env, 1e-9)) ** 0.3, 1.0)
    m = m * gain[:, None]
    s1 = ceiling / (np.abs(m).max() + 1e-9)
    m = np.tanh(m * s1 * 1.1) / np.tanh(1.1)
    s2 = ceiling / (np.abs(m).max() + 1e-9)
    m *= s2
    stems = {}
    for k in ('bounce', 'solo', 'bass', 'strings', 'drums', 'fx'):
        if k in buses:
            stems[k] = fold(buses[k]) * gl * gain[:, None] * s1 * s2
    return m, stems


if __name__ == '__main__':
    main()
