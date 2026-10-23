"""ONE timeline for picture and sound. Video time t (0-60 s) vs battle (sim) time:

    sim time = t + OFFSET            for t < T_RESTART      (frame 0 is 1.4 s into the run: chaos from frame 0)
    sim time = t - T_RESTART         for t >= T_RESTART     (the next run's opening burst, looping into frame 0)

The knockouts, the winner and every claim come from the cached run (sim.npz, picked by seed_search.py);
nothing here is hand-placed except the bell, the crack and the captions' wording.

Beat sheet (target; the real run decides who and when):
    0-2    four bursts at once                          4 BANDS / 1 STAGE
    2-8    legend pinned to each team's base            (labels)
    8-14   FILL race, territory bar slides in          EVERY MOVE / = A NOTE
    14-20  a leader emerges                             MORE STAGE / = LOUDER
    20     the bell: gaps snap, flash                   FULL. / NOW THEY FIGHT
    22-    borders churn                                WHO'S WINNING? / LISTEN
    ~31    first knockout, its stack goes dark          <TEAM> IS OUT!
    ~41    second knockout                              2 LEFT / <A> VS <B>
    ~50    winner floods the stage                      <TEAM> WINS!
    50-58  winner's solo, wall pulses                   WHICH BAND / WOULD YOU PICK?
    58-60  wall cracks to empty, four new seeds -> loop"""
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

OFFSET = E.OFFSET
T_BELL = 20.0
T_CRACK = 58.0
T_RESTART = 58.6
OFF_FRAMES = int(round(OFFSET * FPS))       # 42

D = SIM.load()
NAMES = list(D['names'])                     # SAND BALL RIPPLE VINE
INSTR = ['DRUMS', 'LEAD', 'BASS', 'KEYS']
NT = int(D['ntiles'])


def sim_time(t):
    t = t % DUR
    return t + OFFSET if t < T_RESTART else t - T_RESTART


def video_time(ts):
    """sim time -> video time (inverse of sim_time)"""
    return ts - OFFSET if ts >= OFFSET else ts + T_RESTART


def sim_frame(f):
    return (f + OFF_FRAMES) % NF


OUTS = [(video_time(t), q) for t, q in D['outs']]          # [(video t, team)]
WINNER = D['winner']
T_WIN = video_time(D['t_win']) if D['t_win'] is not None else None
OUT_T = {q: t for t, q in OUTS}


def alive(q, t):
    """is team q still in the battle at video time t (the restart brings everyone back)"""
    if t >= T_RESTART:
        return True
    return not (q in OUT_T and t >= OUT_T[q])


def share(t):
    """team tile shares at video time t (list of 4 floats)"""
    f = sim_frame(int(round(t * FPS)) % NF)
    c = D['count'][f]
    return [float(x) / NT for x in c]


# ------------------------------------------------------------------ captions
def _caps():
    caps = [(-0.5, 2.0, ['4 BANDS', '1 STAGE'])]
    caps += [(8.0, 13.9, ['EVERY *MOVE*', '= A *NOTE*'])]
    caps += [(14.0, 19.9, ['MORE *STAGE*', '= *LOUDER*'])]
    caps += [(20.0, 22.4, ['*FULL.*', 'NOW THEY *FIGHT*'])]
    e1 = OUTS[0][0] if OUTS else 31.0
    caps += [(22.5, e1 - 0.05, ["WHO'S *WINNING?*", 'LISTEN @'])]
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
LEGEND_T = [2.0, 3.2, 4.4, 5.6]
LEGEND_END = 8.0
BAR_IN = 8.0                     # territory bar slides in from the left


def beat_env(t, decay=0.12):
    ph = t % BEAT
    return math.exp(-ph / decay)


def bar_env(t, decay=0.3):
    return math.exp(-(t % BAR) / decay)


def ease(x):
    x = min(1.0, max(0.0, x))
    return x * x * x * (x * (6 * x - 15) + 10)
