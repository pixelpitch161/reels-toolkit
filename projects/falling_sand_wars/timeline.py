"""ONE timeline for picture and sound. Video time = sim time: frame 0 is the empty box as the four hoppers open,
and the closing roll lands the empty box upright at exactly 61.0 s, so the loop point is the file boundary.

Everything after the pour comes from the cached honest run (sim.npz, seed picked by seed_search.py): the turns
and their directions, every grain, the knockouts, the winner. Only the captions' wording is written here.

Beat sheet (94.43 BPM, 24 bars; bar k starts at k * 2.5417 s):
    0-3      empty box, four thin streams start             4 SANDS / 1 HOLE
    3-6      streams thicken; iron sinking already           IRON SINKS / PUMICE FLOATS
    6-9      clay piles steeply, glass runs flat             CLAY STICKS / GLASS FLOWS
    9-20     box fills, layers sort; share bar               LAST SAND IN / THE BOX WINS
    20.33    bar 8: the plug pops (deep impact)              THE HOLE / OPENS
    22.9-    turns on downbeats, avalanches, streams pour out onto the dune pile
    out 1/2  knockouts                                       <TEAM> IS OUT, then 2 LEFT / <A> VS <B>
    win      the last loser's final grain; plug snaps back   <TEAM> / WINS
    win-57   the winner alone; its motif                     WHICH SAND / WOULD YOU PICK?
    57.19    lid opens, the box rolls over, the winner pours out, empty and upright at 61.0 = frame 0"""
import math
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import episode as E
import sim as SIM
from reelkit.ui import impact_landings

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)
BPM = E.BPM                      # 94.43 = reelkit.config.loop_bpm(96, 61)
BEAT = E.BEAT
BAR = E.BAR
S16 = BEAT / 4

T_HOLE = E.T_HOLE
T_ROLL0 = E.T_ROLL0
T_VALVE_SHUT = E.T_VALVE_SHUT

D = SIM.load()
NAMES = ['IRON', 'PUMICE', 'CLAY', 'GLASS']
SAND = np.array([[176, 46, 42], [156, 226, 234], [232, 146, 46], [146, 86, 226]], float)     # grain albedo
TEXT = [(255, 104, 88), (160, 240, 250), (255, 184, 80), (200, 150, 255)]                  # names on screen
BAR_COL = [(214, 62, 54), (150, 228, 238), (240, 160, 56), (160, 104, 240)]

N = D['grids'].shape[1]
NEACH = int(D['n_each'])
ORDER = list(D['order'])                       # team at each hopper, left to right
OUTS = [(float(t), int(q)) for t, q in D['outs']]
OUT_T = {q: t for t, q in OUTS}
WINNER = D['winner']
T_WIN = D['t_win']
TURNS = [float(a) for a, b, c in D['turns']]
TURN_DIR = [int(c) for a, b, c in D['turns']]
THETA = D['theta'].astype(float)
COUNT = D['count'].astype(float)


def frame_of(t):
    return int(min(NF - 1, max(0, math.floor(t * FPS + 1e-6))))


def theta(t):
    """box angle (deg, CCW) at time t, interpolated between frames (the sim stores the mid-frame angle)"""
    x = t * FPS - 0.5
    f0 = int(math.floor(x))
    u = x - f0
    a = THETA[min(NF - 1, max(0, f0))]
    b = THETA[min(NF - 1, max(0, f0 + 1))]
    if f0 < 0:
        a = THETA[-1] - 360 * round((THETA[-1] - THETA[0]) / 360)      # wraps from the end of the roll
    if f0 + 1 >= NF:
        b = THETA[0] + 360 * round((THETA[-1] - THETA[0]) / 360)
    return a + (b - a) * u


def out_now(q, t):
    return q in OUT_T and t >= OUT_T[q]


# smoothed shares for the bar (0.2 s), and the bar's fade
_cnt = COUNT.copy()
_k = 6
_pad = np.concatenate([np.repeat(_cnt[:1], _k, 0), _cnt, np.repeat(_cnt[-1:], _k, 0)])
SMOOTH = np.array([_pad[i:i + 2 * _k + 1].mean(0) for i in range(NF)])
BAR_IN, BAR_OUT = 2.2, T_ROLL0 + 0.05


def shares(t):
    return SMOOTH[frame_of(t)]


def bar_alpha(t):
    return min(1.0, max(0.0, (t - BAR_IN) / 0.4)) * (1.0 - min(1.0, max(0.0, (t - BAR_OUT) / 0.35)))


# ------------------------------------------------------------------ captions (impact style)
def _caps():
    c = []   # (t0, t1, lines, hi, alt)
    c.append((0.0, 3.05, ['4 *SANDS*', '1 *HOLE*'], (255, 206, 120), None))
    c.append((3.15, 6.05, ['*IRON* SINKS', '~PUMICE~ FLOATS'], TEXT[0], TEXT[1]))
    c.append((6.15, 9.05, ['*CLAY* STICKS', '~GLASS~ FLOWS'], TEXT[2], TEXT[3]))
    c.append((9.4, 19.7, ['LAST SAND IN', 'THE BOX *WINS*'], (255, 206, 120), None))
    c.append((T_HOLE, T_HOLE + 2.4, ['THE *HOLE*', '*OPENS*'], (255, 206, 120), None))
    if len(OUTS) >= 1:
        t1, q1 = OUTS[0]
        end1 = min(t1 + 2.4, OUTS[1][0] - 0.05) if len(OUTS) >= 2 else t1 + 2.4
        c.append((t1, end1, [f'*{NAMES[q1]}*', 'IS OUT'], TEXT[q1], None))
    if len(OUTS) >= 2:
        t2, q2 = OUTS[1]
        left = [q for q in range(4) if q not in (OUTS[0][1], q2)]
        quick = t2 - OUTS[0][0] < 2.5
        c.append((t2, t2 + 2.0, [f'*{NAMES[q2]}*', 'OUT TOO' if quick else 'IS OUT'], TEXT[q2], None))
        c.append((t2 + 2.1, min(max(t2 + 4.5, 44.0), (T_WIN or 99.0) - 0.05), ['*2* LEFT', f'*{NAMES[left[0]]}* VS ~{NAMES[left[1]]}~'],
                  TEXT[left[0]], TEXT[left[1]]))
    if T_WIN is not None:
        c.append((T_WIN, T_WIN + 2.5, [f'*{NAMES[WINNER]}*', 'WINS'], TEXT[WINNER], None))
        c.append((T_WIN + 2.6, T_ROLL0 - 0.1, ['WHICH SAND', 'WOULD YOU *PICK?*'], (255, 206, 120), None))
    return c


CAPTIONS = _caps()
LANDINGS = []                                   # (t, line, chars): a soft thud on each
for t0, t1, lines, hi, alt in CAPTIONS:
    LANDINGS += impact_landings([(t0, t1, lines)])


# ------------------------------------------------------------------ the dune pile (strata of the drained sand)
EX_T = (D['ex_f'].astype(float) + (D['ex_s'].astype(float) + 1) / 3.0) / FPS     # exit times (sorted)
EX_M = D['ex_m'].astype(int) - 1                                                 # team of each exiting grain
TOTAL = len(EX_T)
FALL_LAG = 0.55                                 # roughly how long a grain takes to fall to the pile
T_ERODE0, T_ERODE1 = 0.4, 18.5                  # the wind blows last run's pile away while the hoppers pour
NBANDS = 28
BANDS = []
for k in range(NBANDS):
    a, b = int(k * TOTAL / NBANDS), max(int(k * TOTAL / NBANDS) + 1, int((k + 1) * TOTAL / NBANDS))
    BANDS.append(np.bincount(EX_M[a:b], minlength=4) / max(1, b - a))
BANDS = np.array(BANDS)                         # team mix of each slice of the drain, first drained = bottom
BANDS8 = np.array([BANDS[int(k * NBANDS / 8):int((k + 1) * NBANDS / 8)].mean(0) for k in range(8)])


def pile_volume(t):
    """number of grains in the pile at time t (fraction of TOTAL)"""
    if t < T_HOLE - 1.0:
        u = min(1.0, max(0.0, (t - T_ERODE0) / (T_ERODE1 - T_ERODE0)))
        return 1.0 - u * u * (3 - 2 * u)
    return float(np.searchsorted(EX_T, t - FALL_LAG)) / max(1, TOTAL)


def beat_env(t, decay=0.12):
    return math.exp(-(t % BEAT) / decay)


def bar_env(t, decay=0.3):
    return math.exp(-(t % BAR) / decay)


def ease(x):
    x = min(1.0, max(0.0, x))
    return x * x * x * (x * (6 * x - 15) + 10)
