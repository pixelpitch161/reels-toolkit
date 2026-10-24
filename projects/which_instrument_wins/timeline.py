"""#26 Which Instrument Wins? - ONE timeline for picture and sound.

Idea: the trending "Who Will Win? 16 entrants" format with 16 instruments (all modelled and voiced in Pixel Pitch).
16 glass balls, each holding an instrument, drop into a big rotating ring with one gap. Fall out through the gap and
you're out; the gap slowly widens, so eliminations speed up. Honest 2D physics (gravity, bouncing, collisions), the
seed picked for pacing, never edited.
Hook: "16 INSTRUMENTS / 1 SURVIVOR" (impact captions, first 2 s).
Music mechanism: every wall bounce plays that ball's own instrument (its Pixel Pitch voice). Pitch = where it hits
(left -> right = low -> high through the current chord, in the instrument's register), loudness = impact speed,
timing quantised to the 32nd grid of a dark D-minor tango (habanera bass + staccato strings, 118.03 BPM = 30 bars in
61 s). So the arrangement starts as a wall of 16 instruments and thins with every elimination to a duel, then a solo.

Beat sheet (61 s; events after 2 s are the run's own: the seed was chosen so they land near these targets):
    0-2     16 balls drop from the formation at the top of the ring        16 INSTRUMENTS / 1 SURVIVOR
    2-8     ring turns, the gap opens and sweeps past; first falls          FALL OUT = / YOU'RE OUT
    8-20    eliminations every ~3 s; the roster dims                        (X) IS OUT
    20-32   gap widens; an underdog survives a near-miss on the gap edge     STILL ALIVE?!
    32-44   down to 3                                                        FINAL 3
    44-50   the duel: close calls at the gap                                 (no caption)
    ~50     last fall                                                        (X) WINS
    50-58   the winner hovers in the spotlight and plays its solo; the colosseum's torches light up   WHO DID YOU / PICK?
    58-61   the 15 lift out of the pit and fly back into formation; the ring closes; at 61.0 = frame 0 they drop again
"""
FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(118, DUR)        # 118.03: 30 bars in 61 s, so the groove is in phase at the loop point
BEAT = 60.0 / BPM
BAR = 4 * BEAT
N_BARS = 30

# ---------------------------------------------------------------- physics rules (the same for every seed)
# Tuned once on median pacing over 10 seeds (check/rulesearch.py: 40 rule sets scored against the beat sheet), then frozen;
# the seed search only picks among honest runs of these rules.
SIM_DT = 1.0 / 480.0            # 16 substeps per frame
R_RING = 2.0                    # inner radius of the ring wall (world units)
RING_T = 0.22                   # wall thickness (gap end caps are this long)
R_BALL = 0.30                   # every ball the same size and weight (fairness)
GRAV = 14.0                     # world units / s^2
SPIN = 2 * 3.141592653589793 * 20 / DUR   # 2.06 rad/s: exactly 20 turns in 61 s, so the ring is back where it started at the loop point
E_WALL = 1.02                   # wall restitution: a touch over 1 keeps the bouncing lively (speed is capped)
E_BALL = 0.98
V_CAP = 9.5                     # speed cap (about the speed of a fall across the whole ring)
WALL_GRIP = 0.0                 # share of the relative tangential speed a bounce loses to the turning wall
GAP_OPEN = (1.15, 1.9)          # the gap opens between these times (closed at frame 0: the loop point)
GAP_W0 = 0.303                  # rad (17.4 deg) once open: a ball needs 0.301 rad, so only a near-perfect line gets out
GAP_W1 = 0.48                   # rad (27.5 deg) by GAP_T1
GAP_T1 = 48.0
FORMATION_Y = 0.30              # centre of the 4 x 4 starting formation (ring frame)
FORMATION_STEP = 0.66

# ---------------------------------------------------------------- story targets (seed search scores runs against these)
T_FIRST_OUT = (3.5, 7.5)
T_LAST_OUT = (48.0, 50.6)       # the win
T_FINAL3 = (34.0, 42.5)         # when the 4th-last falls (3 remain)
T_DUEL = (41.5, 46.5)           # when the 3rd-last falls (2 remain)
MAX_LULL = 6.5                  # no longer stretch than this without an elimination (before the duel)

# ---------------------------------------------------------------- scripted tail (after the honest result)
T_LIFT = 0.7                    # winner bounces this long after the win, then lifts to the spotlight
LIFT_DUR = 1.3
SOLO_BAR = 25                   # bar index (0-based) where the winner's solo starts: 50.83 s
T_SOLO = SOLO_BAR * BAR
T_RESET = 58.4                  # the 15 lift out of the pit; everyone flies back into formation by 61.0
T_CLOSE = (58.6, 60.3)          # the gap closes (frame 0 has it closed)


def bar_t(b):
    return b * BAR


def beat_env(t, decay=0.12):
    import math
    return math.exp(-((t % BEAT) / decay))

# ---------------------------------------------------------------- music (dark D-minor tango)
# chords per bar (30 bars); midi roots: D=50, G=43, A=45, Bb=46
CHORDS = (['Dm', 'Dm', 'Gm', 'A7', 'Dm', 'Bb', 'Gm', 'A7'] +          # 0-16 s   the wall of 16
          ['Dm', 'Dm', 'Gm', 'A7', 'Dm', 'Bb', 'Gm', 'A7'] +          # 16-32 s  faster pulse from bar 10
          ['Bb', 'Gm', 'A7', 'A7'] +                                  # 32-41 s  final 4, final 3
          ['Dm', 'Gm', 'A7', 'A7', 'A7'] +                            # 41-51 s  the duel on the dominant, resolved by the win
          ['Dm', 'Gm', 'A7', 'Dm'] +                                  # 51-59 s  the winner's solo
          ['A7'])                                                     # 59-61 s  pickup into frame 0
CHORD_TONES = {'Dm': (2, 5, 9), 'Gm': (7, 10, 2), 'A7': (9, 1, 4, 7), 'Bb': (10, 2, 5)}   # pitch classes
CHORD_ROOT = {'Dm': 38, 'Gm': 43, 'A7': 45, 'Bb': 46}
T_FASTER = 10 * BAR             # 20.3 s: strings go to eighths
# the winner's solo: an original 4-bar motif in D minor (beat offsets from T_SOLO, midi, beats), cello register
SOLO = [(0, 57, 1.0), (1, 62, 0.75), (1.75, 64, 0.25), (2, 65, 1.0), (3, 64, 0.5), (3.5, 62, 0.5),
        (4, 67, 1.5), (5.5, 65, 0.25), (5.75, 64, 0.25), (6, 62, 1.0), (7, 58, 1.0),
        (8, 57, 0.5), (8.5, 61, 0.5), (9, 64, 0.5), (9.5, 67, 0.5), (10, 65, 0.75), (10.75, 64, 0.25), (11, 62, 0.5), (11.5, 61, 0.5),
        (12, 62, 2.0), (14, 57, 1.0), (15, 50, 1.0)]


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]
