"""ONE timeline for picture and sound. Video time = race time: frame 0 is GO (all four leave their corners) and the
last two seconds slide everything back home, so 61.0 s lands on GO again (the loop point is the file boundary).

Every race event comes from the cached honest run (sim.pkl: maze + tilt seeds picked by seed_search.py, played as
simulated). Only the captions' wording and the end sequence are written here.

Beat sheet (121.97 BPM = 31 bars in 61 s; bar k starts at k * 1.9677 s):
    0-2      GO: a canon, FLOOD / DIVER / SEEKER step on beats 1/2/3, the marble's gate opens on beat 4;
             corner labels land with each instrument                         WHO SOLVES IT / FIRST?
    2-8      labels slide into the race legend; groove locks in             SAME MAZE / 4 STRATEGIES
    8-39     the race at 1 cell per beat each (interlocking 16ths); captions call the live story:
             the early leader, the fairness rule, SEEKER's dead end near the centre, lead changes
    39.4     final stretch (bar 20): 2 cells per beat each; close-up on the centre
    first    first to the centre: its lane flashes 1ST                      <NAME> WINS
    -51.2    the others finish (or stay lost)
    51.2-57  bars 26-28: the winning route lights up and plays as a melody   THE WINNING / ROUTE
    57-59    resolve; trails fade                                           WHICH WOULD / YOU BET ON?
    59-61    explorers slide home (the marble rewinds its run); 61.0 = frame 0 = GO"""
import math
import os
import sys
from collections import deque

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import maze as M
import sim as SIM
from reelkit.ui import impact_landings

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)
BPM = M.BPM                       # 121.97 = reelkit.config.loop_bpm(122, 61)
BEAT = M.BEAT
BAR = M.BAR
S16 = M.S16
T_STOP = M.T_STOP                 # 51.16: the race ends (bar 26)
T_FINAL = M.FINAL_BAR * BAR       # 39.35: final stretch
T_ROUTE0 = 26 * BAR               # 51.16: the winning route plays
T_ROUTE1 = 29 * BAR               # 57.06: resolve
T_FADE0, T_FADE1 = 57.3, 59.6     # trails fade
T_HOME0, T_HOME1 = 59.03, 60.9    # explorers slide home (bar 30 = the pickup bar)

D = SIM.load()
N = D['N']
NAMES = M.NAMES
INSTR = M.INSTR
PAINT = np.array([[40, 120, 255], [236, 40, 52], [255, 196, 60], [210, 218, 232]], float)    # ink, thread, light, steel
TEXT = [(110, 176, 255), (255, 102, 96), (255, 210, 90), (214, 222, 236)]                     # names on screen
CORNER = [M.corner(q) for q in range(4)]
GOAL = M.GOAL
WALLS = D['walls']
OPEN = set(frozenset(e) for e in D['open'])
DIST = D['dist']
D0 = D['D0']
FIN = D['fin']
STEPS = D['steps']                # per paint explorer: dicts with t, cell, parent, back, dead, (ring/depth/path)
MPOS = D['mpos']                  # marble (x, z) per frame (board units)
HITS = D['hits']                  # (t, normal speed, 'h'|'v', x, z)
MCELLS = D['mcells']
TILT = M.make_tilt(D['tilt_seed'])

ORDER_FIN = sorted((f, q) for q, f in enumerate(FIN) if f is not None)
PLACE = {q: k for k, (f, q) in enumerate(ORDER_FIN)}           # finishing place (0 = winner)
WINNER = ORDER_FIN[0][1] if ORDER_FIN else None
T_WIN = ORDER_FIN[0][0] if ORDER_FIN else None


def frame_of(t):
    return int(min(NF - 1, max(0, math.floor(t * FPS + 1e-6))))


# ------------------------------------------------------------------ per-cell uncover records
INF = 1e9
UNC_T = np.full((4, N, N), INF)           # time each explorer first reached each cell
UNC_DIR = np.full((4, N, N), -1, int)     # direction the ink came from: 0 N, 1 E, 2 S, 3 W (-1 = start, radial)


def _dir(frm, to):
    dr, dc = to[0] - frm[0], to[1] - frm[1]
    return {(-1, 0): 2, (0, 1): 3, (1, 0): 0, (0, -1): 1}[(dr, dc)]     # side of `to` the paint enters from


for q in range(3):
    for e in STEPS[q]:
        r, c = e['cell']
        UNC_T[q, r, c] = e['t']
        if e['parent'] is not None:
            UNC_DIR[q, r, c] = _dir(e['parent'], e['cell'])
_prev = None
for t, cell in MCELLS:
    r, c = cell
    if UNC_T[3, r, c] >= INF:
        UNC_T[3, r, c] = t
        if _prev is not None and abs(_prev[0] - r) + abs(_prev[1] - c) == 1:
            UNC_DIR[3, r, c] = _dir(_prev, cell)
    _prev = cell

STEP_T = [np.array([e['t'] for e in STEPS[q]]) for q in range(3)]


def last_step(q, t):
    """index of explorer q's latest step at or before t (-1 before the first)"""
    return int(np.searchsorted(STEP_T[q], t + 1e-9)) - 1


# DIVER: the thread = its stack after each step
DIVER_STACK = []
_st = []
for e in STEPS[1]:
    for b in e['back']:
        if _st and _st[-1] == b:
            _st.pop()
    _st.append(e['cell'])
    DIVER_STACK.append(list(_st))

# FLOOD: ring (BFS depth) of each step
FLOOD_DEPTH = np.array([e['depth'] for e in STEPS[0]])


# ------------------------------------------------------------------ progress, ranks, leaders
PROG = D['prog']


def progress(q, t):
    p = 0.0
    for tt, v in PROG[q]:
        if tt <= t:
            p = v
        else:
            break
    return p


def finished(q, t):
    return FIN[q] is not None and t >= FIN[q]


def ranking(t):
    """explorers in rank order at time t: finishers first (by finish time), then by progress (ties: earlier wins)"""
    def key(q):
        if finished(q, t):
            return (0, FIN[q], 0)
        # who reached the current best first
        p = progress(q, t)
        first = 0.0
        for tt, v in PROG[q]:
            if v >= p - 1e-9:
                first = tt
                break
        return (1, -p, first)
    return sorted(range(4), key=key)


LEADERS = D['leaders']            # [(t, q)] leader changes (hysteresis, from maze.Race.leaders)


def leader_at(t):
    q = LEADERS[0][1]
    for tt, qq in LEADERS:
        if tt <= t:
            q = qq
    return q


# ------------------------------------------------------------------ the winning route (corner -> centre)
def _route(q):
    if q is None:
        return []
    if q == 0:                                          # FLOOD: parent chain
        par = {e['cell']: e['parent'] for e in STEPS[0]}
        end = STEPS[0][-1]['cell']
    elif q == 1:                                        # DIVER: its thread at the finish
        return list(DIVER_STACK[-1])
    elif q == 2:                                        # SEEKER: the parent tree it built
        par = {e['cell']: e['parent'] for e in STEPS[2]}
        end = STEPS[2][-1]['cell']
    else:                                               # MARBLE: shortest path through the cells it rolled over
        seen = set(c for _, c in MCELLS)
        prev = {CORNER[3]: None}
        dq = deque([CORNER[3]])
        end = None
        while dq:
            x = dq.popleft()
            if x in GOAL:
                end = x
                break
            r, c = x
            for n in ((r - 1, c), (r, c + 1), (r + 1, c), (r, c - 1)):
                if n in seen and n not in prev and frozenset((x, n)) in OPEN:
                    prev[n] = x
                    dq.append(n)
        par = prev
    out, x = [], end
    while x is not None:
        out.append(x)
        x = par[x]
    return out[::-1]


ROUTE = _route(WINNER)
# each route cell lights on the 16th grid across bars 26-28 (3 bars = 48 sixteenths), the last on the bar-29 downbeat
_nslots = 48
ROUTE_T = [T_ROUTE0 + round(k * (_nslots - 1) / max(1, len(ROUTE) - 1)) * S16 for k in range(len(ROUTE))] if ROUTE else []


# ------------------------------------------------------------------ the marble on screen (with the rewind home)
def marble_xz(t):
    """marble centre (board units). After the race it rests; 59.03-60.9 it rewinds its whole run (accelerating), so
    at 61.0 = frame 0 it sits at its corner again."""
    if t < T_HOME0:
        return MPOS[frame_of(t)]
    u = min(1.0, (t - T_HOME0) / (T_HOME1 - T_HOME0))
    u = u * u * (3 - 2 * u)
    f_end = frame_of(min(T_STOP, FIN[3] if FIN[3] is not None else T_STOP))
    f_rel = frame_of(M.MARBLE_RELEASE)
    f = f_end - u * (f_end - f_rel)
    i = int(f)
    a = f - i
    return MPOS[i] * (1 - a) + MPOS[min(i + 1, NF - 1)] * a


def tilt(t):
    return TILT(t)


# ------------------------------------------------------------------ captions (impact style), from the run's events
def _first(evts, a, b):
    for t, x in evts:
        if a <= t <= b:
            return t, x
    return None


def _caps():
    """impact captions from the run: fixed hook + setup, then the live story (who leads, the fairness rule, lead
    changes, SEEKER's dead end near the centre, FLOOD the tortoise), the winner, the route, the question"""
    c = []                                          # (t0, t1, lines, hi, alt)
    gold = (255, 206, 120)
    c.append((0.0, 3.85, ['WHO SOLVES IT', '*FIRST?*'], gold, None))
    c.append((3.95, 7.85, ['SAME MAZE', '4 *STRATEGIES*'], gold, None))
    # the early race: every lead change in 7.5-13.5 s (the first says LEADS), then the fairness rule
    lc = LEADERS + [(T_STOP, None)]
    early = [(t, q) for (t, q), (t2, _) in zip(lc, lc[1:]) if 7.5 <= t <= 13.5 and t2 - t >= 1.5]   # skip blips
    if not early:
        early = [(7.95, leader_at(9.5))]
    nxt = [t for t, q in LEADERS if t > early[-1][0]] + [T_STOP]
    for k, (t, q) in enumerate(early):
        t_next = early[k + 1][0] if k + 1 < len(early) else nxt[0]
        t0 = max(t, 7.95)
        t1 = t0 + min(3.4, max(2.3, t_next - 0.1 - t0))
        c.append((t0, t1, [f'*{NAMES[q]}*', 'LEADS' if k == 0 else 'TAKES OVER'], TEXT[q], None))
    t0 = max(13.95, c[-1][1] + 0.1)
    c.append((t0, t0 + 5.2, ['FAIR RACE: SAME', '*SQUARES* PER BEAT'], gold, None))
    t_free = t0 + 5.3
    # the live story after that: SEEKER's dead ends near the centre, lead changes (FLOOD = the tortoise);
    # each caption holds ~3 s or until the next event
    cand = sorted([(t, 'trap', None) for t, cell in D['traps'] if t >= t_free] +
                  [(t, 'lead', q) for t, q in LEADERS[1:] if t >= t_free])
    last_end = t_free
    for k, (t, kind, q) in enumerate(cand):
        if t < last_end or (T_WIN is not None and t > T_WIN - 3.0):
            continue
        t_next = next((u for u, kk, qq in cand[k + 1:] if u >= t + 2.2), 99.0)
        end = min(t + 3.0, t_next - 0.1)
        if kind == 'trap':
            c.append((t, end, ['*SO CLOSE...*', 'DEAD END'], TEXT[2], None))
        elif q == 0:
            c.append((t, end, ['SLOW AND', '*STEADY...?*'], TEXT[0], None))
        else:
            c.append((t, end, [f'*{NAMES[q]}*', 'TAKES OVER'], TEXT[q], None))
        last_end = end + 0.1
    if T_WIN is not None:
        c.append((T_WIN, min(T_WIN + 2.8, T_ROUTE0 - 0.1), [f'*{NAMES[WINNER]}*', 'WINS'], TEXT[WINNER], None))
    c.append((T_ROUTE0 + 0.05, T_ROUTE1 - 0.1, ['THE WINNING', '*ROUTE*'], TEXT[WINNER] if WINNER is not None else gold, None))
    c.append((T_ROUTE1 + 0.1, 58.9, ['WHICH WOULD', 'YOU *BET ON?*'], gold, None))
    return sorted(c)


CAPTIONS = _caps()
LANDINGS = []
for t0, t1, lines, hi, alt in CAPTIONS:
    LANDINGS += impact_landings([(t0, t1, lines)])


# ------------------------------------------------------------------ corner labels -> legend lanes
LABEL_T = [0.0, BEAT, 2 * BEAT, 3 * BEAT]           # each lands with its instrument's first note (MARBLE: its gate)
LANE_T = [2.2 + 0.35 * q for q in range(4)]         # ... then slides down into its lane
LEGEND_IN = 2.0                                     # the legend backing fades in
LEGEND_OUT = (T_HOME0, T_HOME0 + 0.5)               # ... and out before the reset


def beat_env(t, decay=0.12):
    return math.exp(-(t % BEAT) / decay)


def bar_env(t, decay=0.3):
    return math.exp(-(t % BAR) / decay)


def ease(x):
    x = min(1.0, max(0.0, x))
    return x * x * x * (x * (6 * x - 15) + 10)
