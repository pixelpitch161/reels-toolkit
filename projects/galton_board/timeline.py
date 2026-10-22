"""Chaos Makes a Perfect Curve (IDEAS #14) - ONE timeline for picture AND sound.

Engagement brief: chaos and sound from frame 0, never quiet, build to a satisfying lock-in.
The picture and the music both come from one list of balls (sim.py -> balls.npz): every peg
contact is a ping (pitch = the peg's column), every landing plays its bin's chord note, and each
bin's note gets louder as the bin fills, so the growing curve is the chord voicing.

Beat sheet (60 s, 120 BPM, 1 bar = 2 s):
    0-4    cold open mid-pour: a torrent hails through the pegs           1 IN 4,096
    4-8    freeze + dim, the torrent rewinds; ONE hero ball keeps falling  1 BALL = 12 COIN FLIPS
           (12 eighth-note pings) - establishing pull-back to the hall
    8-12   a second ball, L/R arrows at every peg                         LEFT OR RIGHT? 50 : 50
    12-16  Pascal's numbers light up row by row, one ping per row         12 ROWS = 4,096 PATHS
    16-20  10 balls stream, melodies overlap                              MIDDLE: 924 WAYS / EDGE: 1 WAY
    20-26  staged golden ball goes left 12 times -> edge bin, gong        WHAT IF ... / 1 IN 4,096
    26-32  100 balls, bins jump like an equaliser, groove enters          RANDOM...
    32-40  300 balls, a hump forms, bin chord + bass                      ...BUT NOT MESSY
    40-46  1,000-ball flood, live counts flicker                          (counts)
    46-52  drop: the ideal curve traces over the real bins                THE BELL CURVE
    52-60  trapdoors drain, bucket elevator refills the hopper, the
           torrent restarts at 57.5 s = the cold open (seamless loop)     TRY IT: FLIP 12 COINS / COUNT THE HEADS
"""
FPS = 30
DUR = 60.0
NF = int(FPS * DUR)

BPM = 120
BEAT = 60.0 / BPM            # 0.5 s
BAR = 4 * BEAT               # 2 s

# ---------------------------------------------------------------- the balls (sim.py builds them from this)
SEED_MAIN = 1889             # the honest run: 1,000 balls, real 50/50 bounces
SEED_TORRENT = 4096          # the cold-open torrent (a flash-forward that rewinds away)
H_SLOW = 0.25                # hop time of the solo balls: one bounce per eighth note
H_FAST = 0.125               # everyone else: one bounce per sixteenth

T_FREEZE = 4.0               # the torrent freezes ...
T_REWIND = (4.4, 5.4)        # ... and rewinds back up into the hopper
T_TORRENT = (-2.5, 3.95)     # torrent releases (negative = the last 2.5 s of the loop, from 57.5 s)
TORRENT_RATE = 105.0         # balls per second

HERO_T0 = 4.5                # first peg contact; contacts every eighth -> 4.5 ... 7.25
SECOND_T0 = 8.5              # 8.5 ... 11.25, with L/R arrows
STREAM_T0, STREAM_N, STREAM_GAP = 16.0, 8, 0.375      # balls 3-10
GOLD_T0 = 21.0               # staged all-left path, 21.0 ... 23.75, lands ~24.1
GOLD_FADE = (25.2, 26.0)     # it is a "what if": it dissolves and is never counted
RAIN = (26.0, 31.4, 90)      # balls 11-100
MASS = (32.0, 38.8, 200)     # balls 101-300
FLOOD = (40.0, 44.6, 700)    # balls 301-1000
T_DRAIN = 52.4               # trapdoors open
T_LIFT = (53.4, 57.4)        # bucket elevator carries the balls back up
T_POUR = 57.5                # hopper gate opens = the cold open again

# ---------------------------------------------------------------- explanation beats
ARROWS = (8.3, 12.0)         # L/R arrows at each contact of the second ball
PASCAL_T0, PASCAL_STEP = 12.25, 0.25                  # row k lights at 12.25 + 0.25 k, bins at 15.25
PASCAL_OUT = 20.0
BIN_WAYS = (16.2, 20.0)      # "924" over the middle bin, "1" over the edges
COUNTS = (39.6, 52.6)        # live counts over every bin
CURVE = (46.0, 47.6)         # ideal curve traces left -> right
CURVE_OUT = 52.6
T_GROOVE = 26.0
T_DROP = 46.0
T_THIN = 52.0

# hopper fill level (0..1), piecewise linear and equal at 0 and 60 s
HOPPER = [(0.0, 0.80), (3.95, 0.50), (4.4, 0.50), (5.4, 1.0), (26.0, 1.0), (31.4, 0.91), (32.0, 0.91), (38.8, 0.72),
          (40.0, 0.72), (44.6, 0.06), (53.4, 0.06), (57.4, 1.0), (57.5, 1.0), (60.0, 0.80)]

# ---------------------------------------------------------------- captions (<= 18 chars per line)
CAPTIONS = [
    (-0.5, 4.0, ["*1 IN 4,096*"]),              # fully on at frame 0 (it types in at 59.5 s)
    (4.1, 8.0, ["1 BALL =", "*12* COIN FLIPS"]),
    (8.1, 12.0, ["~LEFT~ OR *RIGHT*?", "50 : 50"]),
    (12.1, 16.0, ["*12* ROWS =", "*4,096* PATHS"]),
    (16.1, 20.0, ["MIDDLE: *924* WAYS", "EDGE: ~1~ WAY"]),
    (20.1, 24.55, ["*WHAT IF* IT GOES", "~LEFT~ EVERY TIME?"]),
    (24.6, 26.0, ["*1 IN 4,096*"]),
    (26.1, 32.0, ["RANDOM..."]),
    (32.1, 40.0, ["...BUT NOT *MESSY*"]),
    (46.1, 52.0, ["THE *BELL CURVE*"]),
    (52.1, 55.6, ["TRY IT:", "FLIP *12* COINS"]),
    (55.7, 59.3, ["~COUNT~ THE", "*HEADS*"]),
    (59.5, 60.5, ["*1 IN 4,096*"]),
]


def hopper_level(t):
    import numpy as np
    return float(np.interp(t % DUR, [p[0] for p in HOPPER], [p[1] for p in HOPPER]))


# ---------------------------------------------------------------- music
# bins (left -> right) hold the notes of one wide C6/9 chord: every other step of C major
# pentatonic, centred on C4 in the middle bin. Pings use the same scale one octave up, one step
# per half peg spacing, so a ball's last ping and its landing note belong together.
PENT = [0, 2, 4, 7, 9]
TONIC_PING = 72              # C5 = column 0
TONIC_BIN = 60               # C4 = the middle bin


def pent_note(step, tonic):
    o, i = divmod(int(step), 5)
    return tonic + 12 * o + PENT[i]


def ping_note(c):
    return pent_note(c, TONIC_PING)


def bin_note(b):
    return pent_note(2 * (b - 6), TONIC_BIN)


# groove chords, one per bar from 26 s: (bass root, pad notes)
CHORDS = [(36, [48, 55, 62, 64, 67]),        # C add9
          (45, [48, 52, 57, 60, 64]),        # Am7
          (41, [48, 53, 57, 60, 64]),        # Fmaj7
          (43, [50, 55, 60, 62, 67])]        # G sus4


def chord_at(t):
    """groove chord: the drop at 46 s lands on C; the groove enters at 26 s on F (IV - V - I)"""
    return CHORDS[int((t - T_DROP) // BAR) % 4]
