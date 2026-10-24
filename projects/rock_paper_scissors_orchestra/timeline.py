"""ONE timeline for picture and sound. Video time = sim time: frame 0 is the clump bursting out in three colours,
and the winner's pieces gather back into the same clump at exactly 61.0 s, so the loop point is the file boundary.

Everything after the hook comes from the cached honest run (sim.npz, seed picked by seed_search.py): every catch, the
swings, the wipe-out, the mop-up, the winner. Only the captions' wording is written here.

Beat sheet (110.16 BPM, 28 bars; bar k starts at k * 2.1786 s):
    0        the burst (timpani)                                      ROCK. / PAPER. / SCISSORS. (one word per beat)
    2.18     bar 1: catches begin                                     HIT WHAT YOU BEAT / IT JOINS YOU
    ...      chases swirl, the share bar sways
    dying    a team crashes to a handful                              <TEAM> / IS DYING...
    ...      its predator runs out of targets: the comeback
    T1       the first wipe-out                                       <TEAM> / IS OUT
    T1+2.4   the twist (low brass sting on the last line)             <KILLER> KILLED / <TEAM>... / AND DOOMED ITSELF
    ...      mop-up: the doomed team's stragglers dodge round the rim
    T_WIN    the last straggler is caught (full orchestra hit)        <WINNER> / WINS
    +2.7     the winner's theme, the victory swirl                    WHO WERE YOU / ROOTING FOR?
    57.2     the winner gathers into the clump, facing out at 61.0 = frame 0 (the burst)"""
import math
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import episode as E
import rps
import sim as SIM
from reelkit.ui import impact_landings

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)
BPM = E.BPM
BEAT = E.BEAT
BAR = E.BAR
S16 = E.S16

D = SIM.load()
POS = D['P'].astype(float)                 # (NF, 120, 2) sim x, y  -> world x, z
HEAD = D['H'].astype(float)
TEAM = D['team'].astype(int)
CATCH = D['catches']                        # (t, hunter, caught, new_team, old_team)
OUTS = [(float(t), int(q)) for t, q in D['outs']]
T_WIN = float(D['t_win'])
WINNER = int(D['winner'])
T_G0 = float(D['f_g0']) / FPS
NPC = POS.shape[1]
PR = E.PARAMS.r

NAMES = rps.NAMES
T1, Q_OUT = OUTS[0]
KILLER = rps.pred_of(Q_OUT)                 # wiped Q_OUT out ... and is doomed: its own predator is the winner
assert rps.prey_of(Q_OUT) == WINNER

TEXT = [(178, 192, 216), (252, 246, 228), (255, 112, 112)]       # names on screen
BAR_COL = [(128, 142, 170), (238, 230, 206), (226, 60, 66)]
GLOW = [(150, 176, 230), (255, 244, 210), (255, 96, 90)]          # catch pops


def frame_of(t):
    return int(min(NF - 1, max(0, math.floor(t * FPS + 1e-6))))


COUNT = np.stack([(TEAM == q).sum(1) for q in range(3)], 1).astype(float)
_k = 5
_pad = np.concatenate([np.repeat(COUNT[:1], _k, 0), COUNT, np.repeat(COUNT[-1:], _k, 0)])
SMOOTH = np.array([_pad[i:i + 2 * _k + 1].mean(0) for i in range(NF)])
BAR_IN, BAR_OUT = 2.0, 57.3


def shares(t):
    return SMOOTH[frame_of(t)]


def out_now(q, t):
    return any(q == qq and t >= tt for tt, qq in OUTS)


def bar_alpha(t):
    return min(1.0, max(0.0, (t - BAR_IN) / 0.4)) * (1.0 - min(1.0, max(0.0, (t - BAR_OUT) / 0.35)))


# ------------------------------------------------------------------ story times from the run
def _dying():
    """the team that crashes to a handful before the wipe-out, and when it first gets that low"""
    f1 = int(T1 * FPS)
    best = None
    for q in range(3):
        a, b = 16 * FPS, max(16 * FPS + 1, f1 - 4 * FPS)
        seg = COUNT[a:b, q]
        k = int(seg.argmin())
        low = seg[k]
        peak = COUNT[a + k:f1, q].max()
        if low <= 6 and peak >= max(15, 3 * max(low, 1)) and (best is None or low < best[1]):
            first = a + int(np.argmax(seg <= low + 2))
            best = (q, low, first / FPS)
    return best


DYING = _dying()


def snap8(t):
    """snap a caption start to the eighth-note grid (so its landing thud sits in the groove)"""
    e = BEAT / 2
    return round(t / e) * e


# ------------------------------------------------------------------ captions (impact style): (t0, t1, lines, hi, alt, stagger, y0)
STAGGER = 0.22
CAP_Y0 = 100                       # reelkit.ui.CAP_Y0: top of the safe box


def _caps():
    c = []
    for k, w in enumerate(['ROCK.', 'PAPER.', 'SCISSORS.']):          # one word per beat, each in its team colour
        c.append((BEAT * (k + 1) - 0.10, 2.95, [f'*{w}*'], TEXT[k], None, STAGGER, CAP_Y0 + 28 * k))
    c.append((3.05, 7.9, ['HIT WHAT YOU *BEAT*', 'IT *JOINS* YOU'], (255, 206, 120), None, STAGGER, None))
    if DYING is not None:
        q, low, td = DYING
        t0 = snap8(td)
        c.append((t0, t0 + 2.6, [f'*{NAMES[q]}*', 'IS DYING...'], TEXT[q], None, STAGGER, None))
    t0 = T1
    c.append((t0, t0 + 2.3, [f'*{NAMES[Q_OUT]}*', 'IS *OUT*'], TEXT[Q_OUT], None, STAGGER, None))
    t2 = snap8(T1 + 2.45)
    tw_end = min(t2 + 4.3, T_WIN - 0.1)
    c.append((t2, tw_end, [f'*{NAMES[KILLER]}* KILLED', f'~{NAMES[Q_OUT]}~...', 'AND DOOMED ITSELF'], TEXT[KILLER], TEXT[Q_OUT], 0.6, None))
    c.append((T_WIN, T_WIN + 2.6, [f'*{NAMES[WINNER]}*', 'WINS'], TEXT[WINNER], None, STAGGER, None))
    c.append((T_WIN + 2.7, 57.0, ['WHO WERE YOU', '*ROOTING* FOR?'], (255, 206, 120), None, STAGGER, None))
    return c


CAPTIONS = _caps()
LANDINGS = []                                   # (t, line, chars): a soft thud on each
for t0, t1, lines, hi, alt, stg, y0 in CAPTIONS:
    LANDINGS += impact_landings([(t0, t1, lines)], stagger=stg)
HOOK_LAND = [BEAT * (k + 1) for k in range(3)]   # ROCK. / PAPER. / SCISSORS. land on beats 1-3
_tw = [c for c in CAPTIONS if 'DOOMED' in c[2][-1]][0]
T_TWIST = _tw[0]
T_TWIST_LAST = _tw[0] + 2 * _tw[5] + 0.10        # the twist's last line lands (low brass sting)
LEGEND = (3.6, 7.9)                             # the rule legend under the second caption

# ------------------------------------------------------------------ catches for picture + sound
CT = CATCH[:, 0] if len(CATCH) else np.zeros(0)
C_CAUGHT = CATCH[:, 2].astype(int) if len(CATCH) else np.zeros(0, int)
C_NEW = CATCH[:, 3].astype(int) if len(CATCH) else np.zeros(0, int)
C_OLD = CATCH[:, 4].astype(int) if len(CATCH) else np.zeros(0, int)


def recent_catches(t, span=0.3):
    """indices of catches in the last `span` seconds"""
    a = np.searchsorted(CT, t - span)
    b = np.searchsorted(CT, t, side='right')
    return range(a, b)


def beat_env(t, decay=0.12):
    return math.exp(-(t % BEAT) / decay)


def bar_env(t, decay=0.35):
    return math.exp(-(t % BAR) / decay)


def ss(a, b, x):
    x = min(1.0, max(0.0, (x - a) / (b - a)))
    return x * x * (3 - 2 * x)


def blaze(t):
    """0..1: the chandelier blazes for the winner (from the win to the gather), back to normal by 61.0"""
    return ss(T_WIN - 0.1, T_WIN + 0.5, t) * (1 - ss(57.2, 60.6, t))
