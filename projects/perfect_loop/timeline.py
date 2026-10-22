"""The Perfect Loop v2 (IDEAS #11): 100 pendulums - ONE timeline for picture AND sound.

100 uncoupled pendulums. Pendulum k (k = 0..99) swings n_k = 51 + k times per 60 s, so
    phase_k(tp) = 2*pi * n_k * tp / 60
and every 60 s of physics each one has done a whole number of swings.

PHASE OFFSET: the video starts 8 s into the physics cycle, physics time tp = video time t + 8.
So the video opens mid-pattern (wave-on-wave), the group ladder arrives every 2-5 s, and all 100
snap into one line at t = 52 (tp = 60). Frame 1800 (tp = 68) == tp 8 == frame 0: exact loop.

Adjacent pendulums slip one full swing apart per 60 s, so at tp = 60 m / g (m, g coprime) the bobs
fall into exactly g groups. In video time (t = tp - 8): 6 @2 s, 5 @4, 4 @7, 3 @12, 2 @22, then
3 @32, 4 @37, 5 @40, 6 @42, 8 @44.5, 10 @46, 15 @48, 20 @49, 30 @50, 50 @50.8 ... 1 line @52.
In the back half the count is g = 60 / (52 - t): it races up and snaps to 1.

Every bob plucks its note (and flashes) each time it is back at its release side, once per full
swing: tp = 60 m / n_k (10 050 plucks in 60 s, ~167 per s). Pendulum n = 120 plucks every 0.5 s,
exactly on the beat of the 120 BPM groove, and every milestone above lands on that beat grid."""
import math
import numpy as np

FPS = 30
DUR = 60.0
NF = int(FPS * DUR)

# ---------------------------------------------------------------- physics (exact, analytic)
K = 100
N = np.arange(51, 51 + K)                       # swings per 60 s: 51 .. 150
PHYS_OFF = 8.0                                  # physics time = video time + 8 s
T_LINE = 60.0 - PHYS_OFF                        # 52.0 s: all 100 in one line
G_ACC = 9.81
PERIOD = 60.0 / N
LEN = G_ACC * (PERIOD / (2 * math.pi)) ** 2     # ideal simple-pendulum length (m): 0.344 .. 0.040
# horizontal release offset: wider at the long (far) end so the on-screen swing stays even in
# perspective, capped at 30 degrees for the short end (exaggerated for readability, motion stays exact)
AMP = np.minimum(0.085 * (LEN / LEN[0]) ** 0.45, LEN * math.sin(math.radians(30)))
THETA0 = np.arcsin(AMP / LEN)                   # 14.3 deg (longest) .. 30 deg (shortest)


def tp(t):
    return t + PHYS_OFF


def phase(t):
    """(K,) phase of every pendulum in radians at VIDEO time t"""
    return 2 * math.pi * N * (tp(t) / 60.0)


def theta(t):
    """(K,) swing angle; + = toward the release side"""
    return THETA0 * np.cos(phase(t))


# every pluck / flash, in video time: tp = 60 m / n  ->  t = 60 m / n - 8, kept in [0, 60)
def _trig(n):
    m0 = math.ceil(PHYS_OFF * n / 60.0 - 1e-9)
    tt = np.array([60.0 * m / n - PHYS_OFF for m in range(m0, m0 + n)])
    tt = np.round(tt, 9)
    return tt[(tt >= 0) & (tt < DUR)]


TRIG = [_trig(int(n)) for n in N]
EVENTS = sorted((float(tt), k) for k in range(K) for tt in TRIG[k])


def last_trig(k, t):
    i = np.searchsorted(TRIG[k], t + 1e-9, side='right') - 1
    if i < 0:                                   # before the first pluck of the video: wrap (loop-safe)
        return TRIG[k][-1] - DUR
    return TRIG[k][i]


def flash(t, decay=0.13):
    """(K,) 1 at each pluck, decaying (wraps across the loop seam)"""
    return np.array([math.exp(-(t - last_trig(k, t)) / decay) for k in range(K)])


def trig_density(t, win=0.03):
    """number of plucks within +-win of t (clusters = alignment)"""
    return sum(int(((tr > t - win) & (tr < t + win)).sum()) for tr in TRIG)


# ---------------------------------------------------------------- groups
# (video time, groups): exact patterns (verified in __main__)
GROUPS = [(2.0, 6), (4.0, 5), (7.0, 4), (12.0, 3), (22.0, 2),
          (32.0, 3), (37.0, 4), (40.0, 5), (42.0, 6), (44.5, 8), (46.0, 10), (48.0, 15),
          (49.0, 20), (50.0, 30), (50.8, 50)]
RACE_FROM = 42.0                 # from here the HUD counter races: g = 60 / (52 - t)
LINE_HOLD = 0.9                  # "1" stays on the counter this long after the line-up


def race_count(t):
    """the group count the HUD shows from 42 s on (integer flips exactly when that pattern forms)"""
    if t < T_LINE:
        return min(100, int(math.floor(60.0 / (T_LINE - t) + 1e-9)))
    if t < T_LINE + LINE_HOLD:
        return 1
    return min(100, int(math.floor(60.0 / (t - T_LINE) + 1e-9)))


def groups_now(t, win=0.9):
    """(g, milestone time, closeness 0..1) when a clean group pattern is on screen, else None"""
    for tm, g in GROUPS:
        if tm < RACE_FROM and abs(t - tm) < win:
            return g, tm, 1 - abs(t - tm) / win
    return None


def group_class(t, g):
    """(K,) group index of every pendulum when g groups are formed"""
    ph = (N * tp(t) / 60.0) % 1.0
    return np.round(ph * g).astype(int) % g


# ---------------------------------------------------------------- music
BPM = 120                        # = pendulum n = 120 (k = 69): one pluck per beat, on the grid
BEAT = 60.0 / BPM
BAR = 4 * BEAT                   # 2.0 s
# 4 octaves of C major pentatonic from C3, five neighbouring pendulums per note, longest = lowest
_PENTA = [0, 2, 4, 7, 9]
NOTES = [48 + 12 * (d // 5) + _PENTA[d % 5] for d in (np.arange(K) * 20 // K)]

T_BAND = 27.0                    # bass + pad enter, group HUD slides up
HUD_IN, HUD_OUT = 27.0, 56.4
T_CHAOS = [(17.0, 22.0), (27.0, 32.0)]

# (time, chord, pad notes, bass root): a change with each pattern
CHORDS = [
    (0.0, 'C', [48, 55, 60, 64], 36),
    (2.0, 'Am7', [52, 57, 60, 67], 45),         # 6 groups
    (4.0, 'Fmaj7', [53, 57, 60, 64], 41),       # 5
    (7.0, 'G6', [50, 55, 59, 64], 43),          # 4
    (12.0, 'C', [48, 55, 60, 64], 36),          # 3
    (17.0, 'Am7', [52, 57, 60, 67], 45),        # chaos
    (22.0, 'Fmaj7', [53, 57, 60, 64], 41),      # 2 teams
    (27.0, 'Am7', [52, 57, 60, 67], 45),        # chaos again: bass + pad enter
    (29.0, 'Fmaj7', [53, 57, 60, 64], 41),
    (31.0, 'G', [50, 55, 59, 62], 43),
    (32.0, 'C', [48, 55, 60, 64], 36),          # 3 groups
    (37.0, 'Am7', [52, 57, 60, 67], 45),        # 4
    (40.0, 'Fmaj7', [53, 57, 60, 64], 41),      # 5
    (42.0, 'G', [50, 55, 59, 62], 43),          # 6
    (44.5, 'Em7', [52, 55, 59, 62], 40),        # 8
    (46.0, 'Fmaj7', [53, 57, 60, 64], 41),      # 10
    (48.0, 'G', [50, 55, 59, 62], 43),          # 15
    (50.0, 'Gsus4', [50, 55, 60, 62], 43),      # 30 -> riser
    (52.0, 'C', [48, 55, 60, 64, 67], 36),      # 1 LINE: the drop
    (54.0, 'Am7', [52, 57, 60, 67], 45),
    (56.0, 'Fmaj7', [53, 57, 60, 64], 41),
    (58.0, 'G', [50, 55, 59, 62], 43),          # dominant: resolves into the loop's C at 0
]


def chord_at(t):
    cur = CHORDS[0]
    for c in CHORDS:
        if t >= c[0]:
            cur = c
    return cur


def subdiv_at(t):
    """hats per beat = the current group count (the groups set the rhythm); 0 = no hats (chaos)"""
    for a, b, s in [(2.0, 4.0, 6), (4.0, 7.0, 5), (7.0, 12.0, 4), (12.0, 17.0, 3), (22.0, 27.0, 2),
                    (32.0, 37.0, 3), (37.0, 40.0, 4), (40.0, 42.0, 5), (42.0, 44.5, 6), (44.5, 52.0, 8),
                    (52.0, 60.0, 4)]:
        if a <= t < b:
            return s
    return 0


# ---------------------------------------------------------------- captions (<= 18 chars per line)
CAPTIONS = [
    (0.0, 2.0, ["*100* PENDULUMS", "*1 PERFECT LOOP*"]),
    (2.05, 4.0, ["NO MOTOR", "NO MAGNETS"]),
    (4.05, 7.0, ["*34 CM* → *4 CM*"]),
    (7.05, 12.0, ["*51* → *150* SWINGS", "PER MINUTE"]),
    (12.05, 17.0, ["*3* GROUPS", "= ~TRIPLETS~"]),
    (17.05, 22.0, ["LOOKS LIKE", "*CHAOS*..."]),
    (22.05, 27.0, ["*HALFTIME:*", "2 TEAMS"]),
    (27.05, 32.0, ["IT'S *REWINDING*"]),
    (32.05, 44.4, ["COUNT THE *GROUPS*"]),
    # 44.5-48.9: "8... 10... 15..." revealed piece by piece (render.count_caption)
    (52.0, 54.4, ["*1 LINE*"]),
    (54.5, 59.6, ["TRY IT: *15 WASHERS*", "+ STRING"]),
]
COUNT_CAP = (44.5, [(44.5, '*8*... '), (46.0, '*10*... '), (48.0, '*15*...')], 48.95)

# on-screen readouts
LEN_TAGS = (4.3, 7.0)            # "34 CM" on the longest bob, "4 CM" on the shortest
SWING_TAGS = (7.4, 11.8)         # "51/MIN" / "150/MIN"
POP_GROUPS = (2.0, 4.0, 7.0)     # "6 GROUPS" pop-ups before the HUD exists
TIE_WIN = 1.1                    # group tie-lines glow for +-1.1 s around each milestone (g <= 6)


def tie_alpha(t):
    """(g, alpha) for the group tie-lines"""
    for tm, g in GROUPS:
        if g > 6:
            continue
        a = 1 - abs(t - tm) / TIE_WIN
        if a > 0:
            return g, min(1.0, a * 1.6)
    return None, 0.0


def loop_check():
    """phase difference between video t = 60 and t = 0 (must be whole cycles)"""
    d = (phase(60.0) - phase(0.0)) / (2 * math.pi)
    return float(np.abs(d - np.round(d)).max())


if __name__ == '__main__':
    print('lengths (cm): %.1f .. %.1f' % (LEN[0] * 100, LEN[-1] * 100))
    print('amp (cm): %.1f .. %.1f  theta0 (deg): %.1f .. %.1f' % (AMP[0] * 100, AMP[-1] * 100,
                                                                  np.degrees(THETA0[0]), np.degrees(THETA0[-1])))
    print('plucks:', len(EVENTS), ' per s:', round(len(EVENTS) / 60, 1), ' per pendulum ok:',
          all(len(TRIG[k]) == N[k] for k in range(K)))
    print('loop error (cycles) between t=60 and t=0:', loop_check())
    for tm, g in GROUPS + [(T_LINE, 1)]:
        ph = (N * tp(tm) / 60.0) % 1.0
        found = len(set(np.round(ph * g).astype(int) % g))
        exact = np.abs(ph * g - np.round(ph * g)).max() < 1e-6
        print(f'{tm:5.1f}s  expected {g:3d} groups, found {found:3d}  exact {exact}  on beat {abs(tm / BEAT - round(tm / BEAT)) < 1e-9}')
    k120 = int(np.where(N == 120)[0][0])
    print('n = 120 plucks on every beat:', np.allclose(TRIG[k120], np.arange(len(TRIG[k120])) * BEAT))
    print('plucks at the line-up (t = 52):', trig_density(T_LINE, 1e-6))
    print('race:', [(t, race_count(t)) for t in (42, 43.5, 44.5, 46, 48, 49, 50, 50.8, 51.5, 52, 53, 54, 56)])
    print('notes:', NOTES[0], '..', NOTES[-1])
