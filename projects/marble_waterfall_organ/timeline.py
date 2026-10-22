"""Marble Waterfall Organ (IDEAS #18): the single source of event times for picture AND sound.

A glowing brass-and-glass marble machine hangs in a vibrant void. It opens with every gate open:
marbles spray off every tier, ping random notes and streak away into space. Gates click shut one by
one on the downbeat; each one routes its stream onto one instrument, so a part of the song rises out
of the chaos. The last (melody) gate sticks, wobbles, then snaps shut: the drop.

Grid: 120 BPM, 1 beat = 0.5 s, 1 bar = 2 s, so 60 s = 30 bars and every bar line is an even second.
Progression (original arrangement): | Bm | G | D | A |, starting at bar 8 (16 s) so the drop
(bar 20, 40 s) and the final chord (bar 28, 56 s) both land on Bm. The bass enters one bar early on A.

    gate  part              instrument (elements)          shuts at
    1     kick              big drum skin                   6.0 s
    2     snare + hats      snare skin + 3 glass discs     10.0 s
    3     bass              10 tuned pipes ("bloops")      14.0 s  (pickup bar on A)
    4     chords            7 glass bowls                  18.0 s
    5     chord stabs       7 tuned bars (marimba)         20.0 s
    6     arpeggio          spiral xylophone ramp (16)     24.0 s  (lift speeds up)
    7     cymbals           crash + ride discs             32.0 s
    8     melody            10 golden bell bars            sticks 35-40, snaps shut at 40.0 = drop
At 57.0 s a lever flips, every gate springs open and the wash returns into the opening chaos (loop).
"""
import math

FPS = 30
DUR = 60.0
NF = int(FPS * DUR)

BPM = 120
BEAT = 60.0 / BPM          # 0.5 s
BAR = 4 * BEAT             # 2.0 s
S16 = BEAT / 4             # 0.125 s


def bt(bar, s16=0.0):
    """time of a 16th position inside a bar"""
    return bar * BAR + s16 * S16


# ------------------------------------------------------------------ gates / sections
CH = ['kick', 'snare', 'bass', 'bowls', 'bars', 'spiral', 'cymbal', 'melody']
CH_LABEL = ['THE BEAT', 'SNARE + HATS', 'BASS', 'CHORDS', 'STABS', 'THE SPIRAL', 'CYMBALS', 'MELODY']
GATE_SHUT = [6.0, 10.0, 14.0, 18.0, 20.0, 24.0, 32.0, 40.0]
T_PULL = 3.0               # pull back to the wide shot
T_STICK = 35.0             # gate 8 starts to wobble
T_TRY = [37.0, 38.5]       # two failed attempts to close (clunk)
T_RISER = 36.0
T_GAP = 39.5               # half-beat of silence before the drop
T_DROP = 40.0
T_FLY = 40.0               # camera flies down the machine 40-48
T_WIDE = 52.0
T_LEVER = 57.0             # lever flips: all gates spring open
T_LIFT_FAST = (24.0, 25.0)  # lift ramps to double speed

# channel colours (rainbow, bottom -> top)
COL = [(255, 86, 86), (255, 156, 60), (255, 222, 84), (110, 240, 140),
       (90, 222, 255), (116, 150, 255), (196, 122, 255), (255, 112, 214)]


def ease_out_back(u, s=1.7):
    u = min(1.0, max(0.0, u)) - 1
    return 1 + u * u * ((s + 1) * u + s)


def gate_open(g, t):
    """1 = gate fully open (marbles spray), 0 = shut (stream routed onto its instrument).
    Periodic over 60 s: all open at 0, shut one by one, spring open at the lever (57 s)."""
    t = t % DUR
    if t >= T_LEVER:                                  # springs open with an overshoot
        u = (t - T_LEVER) / 0.22
        return min(1.25, ease_out_back(u, 2.2)) if u < 1 else 1.0
    ts = GATE_SHUT[g]
    if g == 7 and T_STICK <= t < T_DROP:
        return stuck(t)
    if t < ts - 0.12:
        return 1.0
    if t < ts:                                        # accelerating snap
        u = (t - (ts - 0.12)) / 0.12
        return 1.0 - u * u
    dt = t - ts                                       # tiny bounce after landing
    return abs(0.12 * math.sin(dt * 40) * math.exp(-dt * 18))


def stuck(t):
    """gate 8 holdout: judders half-open, two failed attempts, then the snap at the drop"""
    a = 0.6 + 0.12 * math.sin(t * 23.0) * math.sin(t * 9.1) + 0.06 * math.sin(t * 61.0)
    if t < T_STICK + 0.3:
        a = 1.0 + (a - 1.0) * (t - T_STICK) / 0.3
    for tt in T_TRY:                                  # swings almost shut, bounces back open
        if tt - 0.1 <= t < tt + 0.45:
            if t < tt:
                u = (t - (tt - 0.1)) / 0.1
                a = a + (0.06 - a) * u * u
            else:
                u = (t - tt) / 0.45
                a = 0.06 + (a - 0.06) * ease_out_back(u, 1.4)
    if t >= T_DROP - 0.5:                             # anticipation: lifts wide open, then snaps
        u = (t - (T_DROP - 0.5)) / 0.38
        if u < 1:
            a = a + (0.95 - a) * u
        else:
            v = (t - (T_DROP - 0.12)) / 0.12
            a = 0.95 * (1 - v * v)
    return max(0.0, a)


def n_shut(t):
    t = t % DUR
    if t >= T_LEVER:
        return 0
    return sum(1 for g in range(8) if t >= GATE_SHUT[g])


# ------------------------------------------------------------------ harmony
CHORDS = ['Bm', 'G', 'D', 'A']
T_CHORDS = 14.0            # first chord (A pickup bar)


def chord_idx(bar):
    """0..3 for bars from 7 on, else -1 (no harmony yet: wash + drone)"""
    if bar < 7 or bar > 28:
        return -1
    return (bar - 8) % 4


def chord_at(t):
    return chord_idx(int((t % DUR) // BAR))


# MIDI pitches per part
BASS = {0: (35, 42, 47), 1: (31, 38, 43), 2: (38, 45, 50), 3: (33, 40, 45)}      # root, fifth, octave
BASS_PAT = [(0, 0), (3, 2), (6, 0), (8, 1), (11, 0), (14, 2)]                   # (16th, which)
BOWLS = {0: (62, 66, 71), 1: (62, 67, 71), 2: (62, 66, 69), 3: (64, 69, 73)}
STABS = {0: (66, 71, 74), 1: (67, 71, 74), 2: (66, 69, 74), 3: (64, 69, 73)}
ARP_TONES = {0: (59, 62, 66, 71, 74, 78, 83), 1: (55, 59, 62, 67, 71, 74, 79),
             2: (62, 66, 69, 74, 78, 81, 86), 3: (57, 61, 64, 69, 73, 76, 81)}
ARP_PAT = [0, 1, 2, 3, 4, 5, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3]
D5, E5, Fs5, G5, A5, B5, Cs6, D6, E6, Fs6 = 74, 76, 78, 79, 81, 83, 85, 86, 88, 90
MELODY = {   # bar -> [(16th, pitch)]  (original)
    20: [(0, B5), (3, A5), (6, Fs5), (8, D5), (10, E5), (12, Fs5), (14, A5)],
    21: [(0, B5), (3, A5), (6, Fs5), (8, G5), (12, E5), (14, Fs5)],
    22: [(0, A5), (3, Fs5), (6, D5), (8, E5), (10, Fs5), (12, A5), (14, D6)],
    23: [(0, Cs6), (3, B5), (6, A5), (8, E5), (11, A5), (14, Cs6)],
    24: [(0, D6), (3, Cs6), (6, B5), (8, Fs5), (10, A5), (12, B5), (14, D6)],
    25: [(0, D6), (3, Cs6), (6, B5), (8, G5), (12, B5), (14, D6)],
    26: [(0, Fs6), (3, E6), (6, D6), (8, A5), (10, D6), (12, E6), (14, Fs6)],
    27: [(0, E6), (3, Cs6), (6, A5), (8, Cs6), (11, E6), (14, Cs6)],
    28: [(0, B5)],
}

PIPES = [31, 33, 35, 38, 40, 42, 43, 45, 47, 50]          # 10 bass pipes (low -> high)
BOWL_P = [62, 64, 66, 67, 69, 71, 73]                    # 7 glass bowls
BAR_P = [64, 66, 67, 69, 71, 73, 74]                     # 7 tuned bars
BELL_P = [D5, E5, Fs5, G5, A5, B5, Cs6, D6, E6, Fs6]     # 10 bell bars


def song_events():
    """every routed (gate-shut) marble hit: (t, channel, element_key, pitch, velocity).
    element_key: index into that tier's element list (see machine.py)."""
    ev = []
    end = T_LEVER - 1e-6

    def add(t, ch, el, p, v):
        if t < end and not (T_GAP <= t < T_DROP):
            ev.append((round(t, 6), ch, el, p, v))
    # 1 kick: every beat from gate 1
    for b in range(3, 29):
        for k in range(4):
            add(bt(b, 4 * k), 0, 0, None, 1.0 if k == 0 else 0.88)
    # 2 snare (on 2 and 4, build rolls) + hats on glass discs (3 discs)
    for b in range(5, 29):
        if b == 18:
            for k in range(8):
                add(bt(b, 2 * k), 1, 0, None, 0.55 + 0.04 * k)
        elif b == 19:
            for k in range(8):
                add(bt(b, k), 1, 0, None, 0.7 + 0.02 * k)
            for k in range(8):
                add(bt(b, 8 + 0.5 * k), 1, 0, None, 0.8 + 0.02 * k)
        else:
            add(bt(b, 4), 1, 0, None, 0.95)
            add(bt(b, 12), 1, 0, None, 1.0)
            if b in (11, 15, 23, 27):
                add(bt(b, 14), 1, 0, None, 0.5)
                add(bt(b, 15), 1, 0, None, 0.6)
        if b == 19:
            continue
        pos = range(0, 16, 1) if b == 18 else range(0, 16, 2)
        for s in pos:
            disc = 1 + (s // 2) % 2 if s % 2 == 0 else 3
            if b == 18:
                disc = 1 + s % 3
            add(bt(b, s), 1, disc, None, 0.5 if s % 4 else 0.62)
        if b >= 20 and b != 28:
            for s in (7, 15):
                add(bt(b, s), 1, 3, None, 0.42)
    # 3 bass pipes: tresillo on chord tones (pickup bar 7 on A)
    for b in list(range(7, 19)) + list(range(20, 29)):
        ci = chord_idx(b)
        pat = [(0, 0)] if b == 28 else BASS_PAT
        for s, w in pat:
            p = BASS[ci][w]
            add(bt(b, s), 2, PIPES.index(p), p, 1.0 if s == 0 else 0.85)
    # 4 chord bowls: beats 1 and 3, rolled upward (one marble per note)
    for b in list(range(9, 20)) + list(range(20, 29)):
        ci = chord_idx(b)
        ss = [0] if b in (19, 28) else [0, 8]
        for s in ss:
            for j, p in enumerate(BOWLS[ci]):
                add(bt(b, s) + 0.035 * j, 3, BOWL_P.index(p), p, 0.9 - 0.08 * j)
    # 5 chord stabs on tuned bars: the off-beat eighths
    for b in list(range(10, 19)) + list(range(20, 29)):
        ci = chord_idx(b)
        ss = [0] if b == 28 else [2, 6, 10, 14]
        for s in ss:
            for p in STABS[ci]:
                add(bt(b, s), 4, BAR_P.index(p), p, 0.8 if s in (2, 10) else 0.68)
    # 6 spiral arpeggio: one marble per bar races down the ramp, ringing 16 bars on 16ths
    for b in list(range(12, 19)) + list(range(20, 29)):
        ci = chord_idx(b)
        for k, i in enumerate(ARP_PAT):
            add(bt(b, k), 5, k, ARP_TONES[ci][i], 0.75 if k % 4 == 0 else 0.6)
    # 7 cymbals: crash (element 0) at phrase starts, ride (element 1) on every beat
    for b in list(range(16, 18)) + list(range(20, 29)):
        for k in range(4):
            if k == 0 and b in (16, 20, 24, 28):
                add(bt(b, 0), 6, 0, None, 1.0)
            elif b != 28:
                add(bt(b, 4 * k), 6, 1, None, 0.75 if k % 2 else 0.6)
    # 8 melody on the bell bars
    for b, notes in MELODY.items():
        for s, p in notes:
            add(bt(b, s), 7, BELL_P.index(p), p, 1.0 if s == 0 else 0.88)
    ev.sort()
    return ev


# ------------------------------------------------------------------ spray (gate open) rate
SPRAY_RATE = 8.5          # marbles per second per open gate


def spray_rate(g, t):
    t = t % DUR
    a = gate_open(g, t)
    r = SPRAY_RATE if a > 0.3 else 0.0
    if g == 7 and T_GAP - 0.4 <= t < T_DROP:          # the stuck gate holds its breath before the snap
        r = 0.0
    if T_LEVER <= t < T_LEVER + 0.6:                  # the spill when the lever flips
        r *= 3.0
    return r


# ------------------------------------------------------------------ lift speed (turns of the screw)
LIFT_SLOW = 0.55           # turns per second


def lift_speed(t):
    a, b = T_LIFT_FAST
    t = t % DUR
    if t < a:
        k = 1.0
    elif t < b:
        k = 1.0 + (t - a) / (b - a)
    elif t < T_LEVER:
        k = 2.0
    elif t < T_LEVER + 1.5:
        k = 2.0 - (t - T_LEVER) / 1.5
    else:
        k = 1.0
    return LIFT_SLOW * k


def _lift_turns_raw(t):
    n = 600
    s, dt = 0.0, t / n if n else 0
    for i in range(n):
        s += lift_speed((i + 0.5) * dt) * dt
    return s


_LT60 = _lift_turns_raw(DUR)
LIFT_SCALE = round(_LT60 / 2) * 2 / _LT60          # an even number of turns over 60 s -> screw + marbles loop
_LT = [_lift_turns_raw(k * 0.1) * LIFT_SCALE for k in range(601)]


def lift_turns(t):
    t = t % DUR
    i = min(599, int(t / 0.1))
    u = (t - i * 0.1) / 0.1
    return _LT[i] + (_LT[i + 1] - _LT[i]) * u


# ------------------------------------------------------------------ envelopes
def beat_env(t, decay=0.12, t0=6.0):
    """pulse on every beat once the kick is in (until the lever)"""
    t = t % DUR
    if t < t0 or t >= T_LEVER + 0.3 or (T_GAP <= t < T_DROP):
        return 0.0
    return math.exp(-((t - t0) % BEAT) / decay)


def bar_env(t, decay=0.3):
    t = t % DUR
    if t < T_CHORDS or t >= T_LEVER + 0.5:
        return 0.0
    return math.exp(-(t % BAR) / decay)


def drop_k(t):
    """0 -> 1 as the full song is on (drop) -> 0 at the lever"""
    t = t % DUR
    if t < T_DROP:
        return 0.0
    if t < T_LEVER:
        return min(1.0, (t - T_DROP) / 0.3)
    return max(0.0, 1 - (t - T_LEVER) / 1.0)


def chaos_k(t):
    """how much of the machine is spraying (1 = every gate open)"""
    return sum(min(1.0, gate_open(g, t)) for g in range(8)) / 8.0


# ------------------------------------------------------------------ captions (top band)
CAPTIONS = [
    (-0.5, 3.0, ['1,000 MARBLES', '*0* GATES']),
    (6.0, 9.9, ['GATE 1:', 'THE *BEAT*']),
    (10.0, 13.9, ['GATE 2:', '*SNARE* + HATS']),
    (14.0, 17.9, ['GATE 3:', 'THE *BASS*']),
    (18.0, 19.9, ['GATE 4:', '*CHORDS*']),
    (20.0, 23.9, ['*5* OF 8']),
    (24.0, 29.9, ['GATE 6:', 'THE *SPIRAL*']),
    (32.0, 34.9, ['*7* OF 8']),
    (35.0, 39.45, ['LAST GATE...']),
    (40.0, 43.0, ['*8* OF 8']),
    (43.3, 51.6, ['EVERY NOTE =', '*1 MARBLE*']),
    (59.5, 60.5, ['1,000 MARBLES', '*0* GATES']),
]


if __name__ == '__main__':
    ev = song_events()
    print('routed hits:', len(ev), ' lift turns/60 s:', round(lift_turns(59.999), 3), 'scale', round(LIFT_SCALE, 4))
    import collections
    c = collections.Counter(e[1] for e in ev)
    print({CH[k]: v for k, v in sorted(c.items())})
    full = [e for e in ev if 40 <= e[0] < 56]
    print('hits/s in the full song (40-56):', round(len(full) / 16, 1))
    for g in range(8):
        print(g + 1, CH[g], 'shut', GATE_SHUT[g], ' open(0)', gate_open(g, 0), 'open(59.99)', round(gate_open(g, 59.99), 3))
