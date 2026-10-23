"""ONE timeline for picture and sound. Video time = battle (sim) time: frame 0 is the empty stage as the four
seeds drop, and at the end the wall cracks back to empty exactly at 60.0 s, so the loop point is the video's own
start/end (user, Oct 23). The music grid's bar lines sit at GRID0 + 2k s (GRID0 = 1.4: the grid the run was
searched on), so the bell at 21.4 s is a bar line.

The knockouts, the winner and every claim come from the cached run (sim.npz, picked by seed_search.py);
nothing here is hand-placed except the bell, the crack and the captions' wording.

Beat sheet (seed 82):
    0-2.4    empty stage, four seeds burst              4 BANDS / 1 STAGE
    2.5-9.3  legend pinned to each team's base          (labels)
    9.4-15   FILL race, territory bar slides in         EVERY MOVE / = A NOTE
    15-21    a leader emerges                           MORE STAGE / = LOUDER
    21.4     the bell: gaps snap                        FULL. / NOW THEY FIGHT
    24-      borders churn                              WHO'S WINNING? / LISTEN
    31.1     first knockout, its stack goes dark        <TEAM> IS OUT!
    42.5     second knockout                            2 LEFT / <A> VS <B>
    51.6     winner floods the stage                    <TEAM> WINS!
    51.6-59  winner's solo, wall pulses                 WHICH BAND / WOULD YOU PICK?
    59.4-60  wall cracks to empty -> loops to frame 0 (empty, four seeds)"""
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import episode as E
import sim as SIM

FPS = 30
DUR = 60.0
NF = 1800
BPM = 120
BEAT = 60.0 / BPM
BAR = 4 * BEAT
S16 = BEAT / 4

OFFSET = E.OFFSET                            # 0: video time = sim time
GRID0 = E.GRID0                              # video time of a bar line
T_BELL = E.T_FIGHT                           # 21.4
T_CRACK = 59.4
T_RESTART = 60.0                             # the wall is empty again: = frame 0
T_REVIVE = T_CRACK + 0.35                    # knocked-out stacks power back up for the next run
OFF_FRAMES = int(round(OFFSET * FPS))

D = SIM.load()
NAMES = list(D['names'])                     # SAND BALL RIPPLE VINE
INSTR = ['DRUMS', 'LEAD', 'BASS', 'KEYS']
NT = int(D['ntiles'])


def sim_time(t):
    return t % DUR


def video_time(ts):
    """sim time -> video time (the same clock)"""
    return ts


def sim_frame(f):
    return (f + OFF_FRAMES) % NF


OUTS = [(video_time(t), q) for t, q in D['outs']]          # [(video t, team)]
WINNER = D['winner']
T_WIN = video_time(D['t_win']) if D['t_win'] is not None else None
OUT_T = {q: t for t, q in OUTS}


def alive(q, t):
    """is team q still in the battle at video time t (the restart brings everyone back)"""
    if t >= T_REVIVE:
        return True
    return not (q in OUT_T and t >= OUT_T[q])


def share(t):
    """team tile shares at video time t (list of 4 floats)"""
    f = sim_frame(int(round(t * FPS)) % NF)
    c = D['count'][f]
    return [float(x) / NT for x in c]


# ------------------------------------------------------------------ captions
def _caps():
    caps = [(-0.5, 2.4, ['4 BANDS', '1 STAGE'])]
    caps += [(9.4, 15.3, ['EVERY *MOVE*', '= A *NOTE*'])]
    caps += [(15.4, T_BELL - 0.1, ['MORE *STAGE*', '= *LOUDER*'])]
    caps += [(T_BELL, T_BELL + 2.4, ['*FULL.*', 'NOW THEY *FIGHT*'])]
    e1 = OUTS[0][0] if OUTS else 31.0
    caps += [(T_BELL + 2.5, e1 - 0.05, ["WHO'S *WINNING?*", 'LISTEN @'])]
    if OUTS:
        t, q = OUTS[0]
        caps += [(t, t + 2.8, [f'*{NAMES[q]}* IS OUT!', f'NO MORE {INSTR[q]}'])]
    if len(OUTS) >= 2:
        t, q = OUTS[1]
        left = [k for k in range(4) if k not in (OUTS[0][1], q)]
        caps += [(t, t + 2.8, ['*2* LEFT', f'{NAMES[left[0]]} VS {NAMES[left[1]]}'])]
    if T_WIN is not None:
        caps += [(T_WIN, T_WIN + 3.0, [f'*{NAMES[WINNER]}*', 'WINS!'])]
        caps += [(T_WIN + 3.05, T_CRACK - 0.1, ['WHICH BAND', 'WOULD YOU *PICK?*'])]
    caps += [(59.5, 60.5, ['4 BANDS', '1 STAGE'])]
    return caps


CAPTIONS = _caps()
# legend: each team's label pops on at its base, one after another, then fades at 8 s
LEGEND_T = [2.5, 3.6, 4.7, 5.8]
LEGEND_END = 9.3
BAR_IN = 9.4                     # territory bar slides in from the left (a bar line)


def beat_phase(t):
    """0..1 position inside the current beat of the music grid"""
    return ((t - GRID0) % BEAT) / BEAT


def beat_env(t, decay=0.12):
    return math.exp(-((t - GRID0) % BEAT) / decay)


def bar_env(t, decay=0.3):
    return math.exp(-((t - GRID0) % BAR) / decay)


def ease(x):
    x = min(1.0, max(0.0, x))
    return x * x * x * (x * (6 * x - 15) + 10)
