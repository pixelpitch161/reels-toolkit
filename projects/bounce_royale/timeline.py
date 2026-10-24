"""ONE timeline for picture and sound. Video time = run time: frame 0 is the ring snapping open as RUBY launches,
and the outro collapses the winner's ring to a point exactly at 61.0 s, so the loop point is the file boundary.

Everything after the launch comes from the cached honest run (sim.npz, seed picked by seed_search.py): every
bounce, splash, repaint, collision, pop and the winner. Only the captions' wording is written here; their times
come from the run's events.

Beat sheet (118.03 BPM = loop_bpm(118): 30 bars in 61 s; bar k starts at k * 2.0334 s):
    0-2      ring snaps open, balls launch one per beat, labels pop       LAST COLOUR / STANDING
    2-9.3    labels fly into the share bar; repaint battle                HIT A WALL = / PAINT IT, MORE WALL = / BIGGER SPLASH
    9.4-16   lead swings                                                  LOSE YOUR WALL = / YOU POP, <X> TAKES / THE LEAD
    16-20.8  the ring starts shrinking, bounces speed up                  THE RING / SHRINKS
    20.9-    near-deaths, pops, comebacks (from the run)                  1 SEGMENT LEFT! / POPS / IS BACK! / HANGS ON
    duel     two left, the bar swings                                     2 LEFT / <A> VS <B>
    win      the final pop, the winner floods the ring, its solo           <X> / WINS, WHO DID YOU / PICK?
    57-61    the ring closes to a point; at 61.0 = frame 0 it snaps open   (the hook lands again at 59.5)"""
import math
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
import royale as RY
import sim as SIM
from reelkit.config import loop_bpm
from reelkit.ui import impact_landings

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)
BPM = loop_bpm(118, DUR)            # 118.03: 30 bars
BEAT = 60.0 / BPM
BAR = 4 * BEAT
S16 = BEAT / 4
S32 = BEAT / 8

D = SIM.load()
P = RY.PARAMS
NAMES = RY.NAMES
INSTR = RY.INSTR
NT = RY.NT
WINNER = D['winner']
T_WIN = D['t_win']
POPS = [(float(t), int(q), np.array(p)) for t, q, p in D['pops']]
POP_T = {q: t for t, q, _ in POPS}
T_LAUNCH = D['t_launch']

# neon palette: wall tiles, balls, text
COL = np.array([[255, 52, 84], [44, 255, 140], [64, 124, 255], [255, 186, 40]], float)
TEXT = [(255, 96, 120), (110, 255, 170), (120, 170, 255), (255, 206, 90)]
BAR_COL = [(238, 54, 84), (40, 226, 128), (64, 118, 240), (242, 178, 40)]

T_COLLAPSE = 57.0                   # the winner's ring starts closing
T_SNAP = 0.16                       # the snap-open at frame 0


def frame_of(t):
    return int(min(NF - 1, max(0, math.floor(t * FPS + 1e-6))))


def popped(q, t):
    return q in POP_T and t >= POP_T[q]


def ring_sim(t):
    return RY.ring_radius(min(t, T_COLLAPSE), P)


def ring_vis(t):
    """drawn ring radius (sim units): snap-open at frame 0, the run's ring, the outro collapse to a point at 61"""
    r = ring_sim(t)
    if t < T_SNAP:
        u = t / T_SNAP
        k = 0.34 + 0.66 * (1 - (1 - u) ** 3) + 0.07 * math.sin(math.pi * u)        # bursts open, a hint of overshoot
        return r * min(k, 1.07)
    if t < T_SNAP + 0.18:                                                            # settles
        u = (t - T_SNAP) / 0.18
        return r * (1 + 0.04 * (1 - u) * math.cos(math.pi * u * 0.5))
    if t > T_COLLAPSE:
        u = min(1.0, (t - T_COLLAPSE) / (DUR - T_COLLAPSE))
        return r * (1 - u) ** 1.6
    return r


def ball_scale(t):
    """positions are scaled with the drawn ring in the snap and the collapse (the sim ring stays put)"""
    return ring_vis(t) / ring_sim(t)


def cam_zoom(t):
    """camera distance factor: pushes in as the ring shrinks, eases back out in the collapse (frame 0 framing)"""
    k = ring_sim(t) ** 0.55
    if t > T_COLLAPSE:
        u = min(1.0, (t - T_COLLAPSE) / (DUR - T_COLLAPSE))
        u = u * u * (3 - 2 * u)
        k = k * (1 - u) + 1.0 * u
    return k


# ------------------------------------------------------------------ the run's events
HIT_T, HIT_Q, HIT_A, HIT_W, HIT_N = D['hit_t'], D['hit_q'].astype(int), D['hit_a'], D['hit_w'], D['hit_n'].astype(int)
FLIP = D['flip']                                  # (hit index, tile, previous owner)
FLIP_T = HIT_T[FLIP[:, 0]] if len(FLIP) else np.zeros(0)
COLL_T, COLL_IJ, COLL_P = D['coll_t'], D['coll_ij'].astype(int), D['coll_p']
OWNER = D['owner'].astype(int)                    # (NF, 48) at the start of each frame
COUNT = D['count'].astype(float)
POS = D['pos'].astype(float)                      # (NF, 4, 2) at the start of each frame
LAST_FLIP = np.full((NF, NT), -9.0)               # time of each tile's last repaint, per frame
_lf = np.full(NT, -9.0)
_j = 0
for f in range(NF):
    while _j < len(FLIP_T) and FLIP_T[_j] < f / FPS + 1e-9:
        _lf[FLIP[_j, 1]] = FLIP_T[_j]
        _j += 1
    LAST_FLIP[f] = _lf


def owner_at(t):
    """tile owners at time t (exact: replays the repaints since the frame start)"""
    f = frame_of(t)
    own = OWNER[f].copy()
    t0 = f / FPS
    i0, i1 = np.searchsorted(FLIP_T, [t0 + 1e-9, t + 1e-9])
    for i in range(i0, i1):
        own[FLIP[i, 1]] = HIT_Q[FLIP[i, 0]]
    return own


def ball_pos(q, t):
    """sim-space position (before ball_scale), interpolated between frames"""
    x = t * FPS
    f0 = int(math.floor(x))
    u = x - f0
    a = POS[min(NF - 1, max(0, f0)), q]
    b = POS[min(NF - 1, max(0, f0 + 1)), q]
    if f0 + 1 >= NF:
        b = a
    return a + (b - a) * u


def launched(q, t):
    return t >= T_LAUNCH[q]


# smoothed shares for the bar (0.2 s)
_k = 6
_pad = np.concatenate([np.repeat(COUNT[:1], _k, 0), COUNT, np.repeat(COUNT[-1:], _k, 0)])
SMOOTH = np.array([_pad[i:i + 2 * _k + 1].mean(0) for i in range(NF)])
BAR_IN, BAR_OUT = 2.55, 57.6


def shares(t):
    return SMOOTH[frame_of(t)]


def bar_alpha(t):
    return min(1.0, max(0.0, (t - BAR_IN) / 0.4)) * (1.0 - min(1.0, max(0.0, (t - BAR_OUT) / 0.6)))


# ------------------------------------------------------------------ story beats from the run
def _first_low(q, thr=1):
    f = np.nonzero(COUNT[:, q] <= thr)[0]
    return f[0] / FPS if len(f) else None


def _lead_changes(f0, f1, hyst=3):
    lead = int(np.argmax(COUNT[f0]))
    out = []
    for f in range(f0, f1):
        m = int(np.argmax(COUNT[f]))
        if m != lead and COUNT[f, m] >= COUNT[f, lead] + hyst:
            lead = m
            out.append((f / FPS, m))
    return out


LEAD_CHANGES = _lead_changes(int(6 * FPS), frame_of(T_WIN))
LOW1 = {q: _first_low(q) for q in range(4)}


def _comeback():
    """a colour that climbs from 1 tile to >= 15 within 4 s: (t when it reaches 15, team)"""
    for q in range(4):
        t1 = LOW1[q]
        if t1 is None:
            continue
        f1 = frame_of(t1)
        end = frame_of(POP_T.get(q, DUR))
        up = np.nonzero(COUNT[f1:end, q] >= 15)[0]
        if len(up) and up[0] / FPS < 6:
            return (f1 + up[0]) / FPS, q
    return None


COMEBACK = _comeback()


def _caps():
    """(t0, t1, lines, hi, alt): impact captions; the hook wraps across the loop point"""
    hook = ['LAST COLOUR', '*STANDING*']
    c = [(DUR - 1.5, DUR + 2.0, hook, (255, 120, 220), None),
         (2.1, 5.6, ['HIT A WALL =', '*PAINT IT*'], (255, 120, 220), None),
         (5.7, 9.3, ['MORE WALL =', '*BIGGER SPLASH*'], (255, 120, 220), None),
         (9.4, 13.0, ['LOSE YOUR WALL =', '*YOU POP*'], (255, 120, 220), None)]
    lc = [x for x in LEAD_CHANGES if 13.0 <= x[0] <= 15.0]
    if lc:
        t, q = lc[-1]
        c.append((max(13.1, t), 15.9, [f'*{NAMES[q]}* TAKES', 'THE LEAD'], TEXT[q], None))
    c.append((16.0, 20.7, ['THE RING', '*SHRINKS*'], (255, 120, 220), None))
    p1t, p1q, _ = POPS[0]
    lows = sorted((t, q) for q, t in LOW1.items() if t is not None and 20.0 <= t < p1t)
    for k, (t, q) in enumerate(lows[:2]):
        end = lows[k + 1][0] - 0.05 if k + 1 < len(lows[:2]) else p1t - 0.05
        c.append((t, end, [f'*{NAMES[q]}*', '1 SEGMENT LEFT!'], TEXT[q], None))
    c.append((p1t, p1t + 2.2, [f'*{NAMES[p1q]}*', 'POPS'], TEXT[p1q], None))
    if COMEBACK:
        tb, qb = COMEBACK
        c.append((max(tb, p1t + 2.3), max(tb, p1t + 2.3) + 3.0, [f'*{NAMES[qb]}* IS', 'BACK!'], TEXT[qb], None))
    p2t, p2q, _ = POPS[1]
    l2 = LOW1[p2q]
    if l2 is not None and l2 < p2t - 6:
        c.append((l2, l2 + 2.6, [f'*{NAMES[p2q]}*', '1 SEGMENT LEFT!'], TEXT[p2q], None))
        c.append((l2 + 4.0, p2t - 1.4, [f'*{NAMES[p2q]}*', 'HANGS ON'], TEXT[p2q], None))
    c.append((p2t, p2t + 2.2, [f'*{NAMES[p2q]}*', 'POPS'], TEXT[p2q], None))
    left = [q for q in range(4) if q not in (POPS[0][1], p2q)]
    c.append((p2t + 2.3, p2t + 4.6, ['*2* LEFT', f'*{NAMES[left[0]]}* VS ~{NAMES[left[1]]}~'], TEXT[left[0]], TEXT[left[1]]))
    c.append((T_WIN, T_WIN + 2.6, [f'*{NAMES[WINNER]}*', 'WINS'], TEXT[WINNER], None))
    c.append((T_WIN + 2.7, T_COLLAPSE, ['WHO DID YOU', '*PICK?*'], (255, 120, 220), None))
    return c


CAPTIONS = _caps()
LANDINGS = []                               # (t, line, chars): a soft thud on each (t may be > DUR: wraps)
for t0, t1, lines, hi, alt in CAPTIONS:
    LANDINGS += impact_landings([(t0, t1, lines)])


def captions_at(t):
    """captions to draw at t, with the time to draw them at (the hook is drawn at t + DUR before 2 s)"""
    out = []
    for t0, t1, lines, hi, alt in CAPTIONS:
        for tt in (t, t + DUR):
            if t0 <= tt < t1:
                out.append((tt, t0, t1, lines, hi, alt))
                break
    return out


# ------------------------------------------------------------------ opening labels: "RUBY · DRUMS" beside each ball
LABEL_FLY0, LABEL_FLY1 = 2.15, 2.75          # labels fly into the share bar's name slots


def label_alpha(q, t):
    return 1.0 if t >= T_LAUNCH[q] else 0.0


# ------------------------------------------------------------------ music helpers
def beat_env(t, decay=0.12):
    return math.exp(-(t % BEAT) / decay)


def bar_env(t, decay=0.3):
    return math.exp(-(t % BAR) / decay)


def ease(x):
    x = min(1.0, max(0.0, x))
    return x * x * x * (x * (6 * x - 15) + 10)


if __name__ == '__main__':
    print(f'BPM {BPM:.2f}  bar {BAR:.4f} s')
    print('pops', [(round(t, 2), NAMES[q]) for t, q, _ in POPS], 'winner', NAMES[WINNER], round(T_WIN, 2))
    print('lead changes', [(round(t, 2), NAMES[q]) for t, q in LEAD_CHANGES])
    print('first at 1 tile', {NAMES[q]: (None if v is None else round(v, 2)) for q, v in LOW1.items()}, 'comeback', COMEBACK)
    for t0, t1, lines, hi, alt in CAPTIONS:
        print(f'  {t0:6.2f}-{t1:6.2f}  {" / ".join(lines)}')
