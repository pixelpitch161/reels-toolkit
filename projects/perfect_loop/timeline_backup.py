# v1 BACKUP (15 pendulums; approved v1 render perfect_loop.mp4). Imports the other *_backup modules, so it runs as is.
"""The Perfect Loop (IDEAS #11) - ONE timeline for picture AND sound.

15 uncoupled pendulums. Pendulum k (k = 0..14) swings n_k = 51 + k times in 60 s, so
    phase_k(t) = 2*pi * n_k * t / 60
and at t = 60 s every pendulum has done a whole number of swings: the video is exactly one
cycle of the physics, so frame 1800 == frame 0 (a true seamless loop).

Adjacent pendulums slip one full swing apart every 60 s, so at t = 60/g s the bobs fall into
g groups (6 @10 s, 5 @12, 4 @15, 3 @20, 2 @30) and the same counts return in reverse on the
way back (3 @40, 4 @45, 5 @48, 6 @50), until one line @60.

Every bob plucks its note (and flashes) each time it is back at its release side, once per full
swing: t = 60*m/n_k. The groups ARE the rhythm (2 groups = eighths, 3 = triplets, ...).
The middle pendulum (k = 7, n = 58) plucks exactly on every beat of the 58 BPM groove."""
import math
import numpy as np

FPS = 30
DUR = 60.0
NF = int(FPS * DUR)

# ---------------------------------------------------------------- physics (exact, analytic)
K = 15
N = np.arange(51, 51 + K)                       # swings per 60 s
G_ACC = 9.81
PERIOD = 60.0 / N
LEN = G_ACC * (PERIOD / (2 * math.pi)) ** 2     # ideal simple-pendulum length (m): 0.344 .. 0.212
AMP = 0.062                                     # same horizontal release offset for every bob (m)
THETA0 = np.arcsin(AMP / LEN)                   # 10.4 deg (longest) .. 17.0 deg (shortest): small swings


def phase(t):
    """(K,) phase of every pendulum in radians"""
    return 2 * math.pi * N * (t / 60.0)


def theta(t):
    """(K,) swing angle; + = toward the release side"""
    return THETA0 * np.cos(phase(t))


# every pluck / flash: t = 60 m / n_k, m = 0 .. n_k - 1  (t = 60 is frame 0 of the loop)
TRIG = [np.array([60.0 * m / n for m in range(n)]) for n in N]
EVENTS = sorted((float(tt), k) for k in range(K) for tt in TRIG[k])


def last_trig(k, t):
    i = np.searchsorted(TRIG[k], t, side='right') - 1
    return TRIG[k][max(i, 0)]


def flash(t, decay=0.2):
    """(K,) 1 at each pluck, decaying"""
    return np.array([math.exp(-(t - last_trig(k, t)) / decay) for k in range(K)])


# ---------------------------------------------------------------- music
BPM = 58                     # = the middle pendulum (n = 58): one pluck per beat
BEAT = 60.0 / BPM
# 3 octaves of C major pentatonic, longest (k = 0) = lowest. Red = low (long wave), violet = high.
NOTES = [48, 50, 52, 55, 57, 60, 62, 64, 67, 69, 72, 74, 76, 79, 81]
GROUPS = [(10.0, 6), (12.0, 5), (15.0, 4), (20.0, 3), (30.0, 2),
          (40.0, 3), (45.0, 4), (48.0, 5), (50.0, 6)]
T_HALF = 30.0
T_BAND = 35.0                # bass + pad enter, group HUD slides up
T_GROOVE = 40.0              # soft kick on the beat
T_BUILD = 50.0
T_CLIMAX = 60.0 * 57 / 58    # 58.97 s: last near-unison strum before the realignment
HUD_IN, HUD_OUT = 35.0, 56.2


def beat(b):
    return b * BEAT


# (beat, chord name, pad notes, bass root) - chords change with each pattern (back half)
CHORDS = [
    (34, 'Dm7', [50, 53, 57, 60], 38),
    (36, 'G6', [50, 55, 59, 64], 43),
    (38, 'C', [48, 55, 60, 64], 36),         # 3 groups @40 s
    (41, 'Am7', [52, 57, 60, 67], 45),
    (43, 'Fmaj7', [53, 57, 60, 64], 41),     # 4 groups @45 s
    (46, 'G', [50, 55, 59, 62], 43),         # 5 @48
    (48, 'Am', [52, 57, 60, 64], 45),        # 6 @50
    (50, 'Fmaj7', [53, 57, 60, 64], 41),
    (52, 'Dm7', [50, 53, 57, 60], 38),
    (54, 'Gsus4', [50, 55, 60, 62], 43),
    (56, 'G', [50, 55, 59, 62], 43),         # dominant: resolves into the loop's C at t = 0
]


def chord_at(t):
    cur = None
    for b, name, pad, root in CHORDS:
        if t >= beat(b):
            cur = (b, name, pad, root)
    return cur


def groups_now(t, win=0.9):
    """(g, milestone time, closeness 0..1) when a clean group pattern is on screen, else None"""
    for tm, g in GROUPS:
        if abs(t - tm) < win:
            return g, tm, 1 - abs(t - tm) / win
    return None


# ---------------------------------------------------------------- captions (<= 18 chars per line)
CAPTIONS = [
    (0.0, 4.0, ["*15* PENDULUMS", "*1 PERFECT LOOP*"]),
    (4.1, 8.0, ["*51* → *65* SWINGS", "PER MINUTE"]),
    (8.1, 12.0, ["*LONGER* = ~SLOWER~"]),
    (12.1, 16.0, ["NO MOTOR", "NO MAGNETS"]),
    (16.1, 22.0, ["*3* GROUPS", "= ~TRIPLETS~"]),
    (22.1, 29.8, ["LOOKS LIKE", "*CHAOS*..."]),
    (29.9, 35.0, ["*HALFTIME:*", "2 TEAMS"]),
    (35.1, 40.0, ["IT'S *REWINDING*"]),
    (40.1, 44.9, ["COUNT THE *GROUPS*"]),
    # 45-50: "4... 5... 6..." revealed piece by piece (custom, see render.count_caption)
    (52.0, 56.0, ["TRY IT: *15 WASHERS*", "+ STRING"]),
]
COUNT_CAP = (45.0, 50.0, [(45.0, '*4*... '), (48.0, '*5*... '), (50.0, '*6*...')], 51.9)

# on-screen readouts
SWING_TAGS = (4.4, 8.0)       # "51/MIN" on the longest bob, "65/MIN" on the shortest
LEN_TAGS = (8.3, 12.0)        # "34 CM" / "21 CM" (ideal pendulums)
TIE_WIN = 1.1                 # group tie-lines glow for +-1.1 s around each milestone


def tie_alpha(t):
    """(g, alpha) for the group tie-lines"""
    for tm, g in GROUPS:
        a = 1 - abs(t - tm) / TIE_WIN
        if a > 0:
            return g, min(1.0, a * 1.6)
    return None, 0.0


def loop_check():
    """at t = 60 every bob is back at the release side"""
    return float(np.abs(np.cos(phase(60.0)) - 1).max())


if __name__ == '__main__':
    print('lengths (cm):', np.round(LEN * 100, 1))
    print('theta0 (deg):', np.round(np.degrees(THETA0), 1))
    print('plucks:', len(EVENTS), ' per s:', round(len(EVENTS) / 60, 1))
    print('loop error at 60 s:', loop_check())
    for tm, g in GROUPS:
        ph = (phase(tm) / (2 * math.pi)) % 1.0
        print(f'{tm:5.1f}s  expected {g} groups, found {len(set(np.round(ph * g) % g))} '
              f'distinct phases: {sorted(set(np.round(ph, 3)))}')
    k7 = TRIG[7]
    print('middle pendulum on the beat:', np.allclose(k7, np.arange(58) * BEAT))
