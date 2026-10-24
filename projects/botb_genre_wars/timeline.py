"""ONE timeline for picture and sound. Video time = battle (sim) time: frame 0 is the empty stage as the four
seeds drop, and at the end the wall cracks back to empty exactly at 61.0 s, so the loop point is the file
boundary. 118.03 BPM: 30 bars in 61 s, bar k starts at k * BAR, so the loop seam is a bar line and the bell
(bar 10, 20.33 s) is too.

The knockouts, the winner, the lead changes and every claim come from the cached run (sim.npz, picked by
seed_search.py); nothing here is hand-placed except the bell, the crack and the captions' wording.

Beat sheet (61 s; event times from the chosen seed are printed by `python timeline.py`):
    0-2.4    empty stage, four seeds burst; genre labels pop at each base     4 GENRES / 1 STAGE
    4.07     bar 2: every part locks onto the grid                           EVERY MOVE / = A NOTE
    8.13     bar 4                                                           THE SONG FOLLOWS / THE LEADER
    fill     lead changes (the song morphs; stage lights change)             <GENRE> / TAKES OVER
    20.33    the bell: gaps snap                                             FULL. / NOW THEY FIGHT
    fight    borders churn, lead changes                                     WHO'S WINNING? / LISTEN, <GENRE> TAKES OVER
    ~31      first knockout                                                  <GENRE> IS OUT! / NO MORE <INSTR>
    ~42      second knockout                                                 2 LEFT / <A> VS <B>
    ~51      the winner floods the stage                                     <GENRE> / WINS!
    then     winner's solo (its genre takes the whole song)                  WHICH GENRE / ARE YOU?
    60.1     the hook lands again; 60.4 the wall cracks -> empty at 61.0 = frame 0"""
import math
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
import episode as E
import sim as SIM
from reelkit.ui import impact_landings
from reelkit.config import ease

FPS = 30
DUR = 61.0
NF = int(round(DUR * FPS))                  # 1830
BPM = E.BPM                                 # 118.03
BEAT = E.BEAT
BAR = E.BAR
S16 = BEAT / 4

OFFSET = E.OFFSET
GRID0 = E.GRID0
T_BELL = E.T_FIGHT                          # 20.33
T_CRACK = 60.4
T_RESTART = DUR                             # the wall is empty again: = frame 0
T_REVIVE = T_CRACK + 0.3                    # knocked-out stacks power back up for the next run
OFF_FRAMES = int(round(OFFSET * FPS))

D = SIM.load()
NAMES = list(E.NAMES)                       # ORCHESTRA ROCK EDM JAZZ (engine order)
INSTR = ['STRINGS', 'GUITAR', 'SYNTH', 'SAX']
SHOW = [2, 0, 1, 3]                         # left-to-right order on screen (matches where each genre starts)
NT = int(D['ntiles'])


def sim_time(t):
    return t % DUR


def video_time(ts):
    return ts


def sim_frame(f):
    return (f + OFF_FRAMES) % NF


OUTS = [(video_time(t), q) for t, q in D['outs']]
WINNER = D['winner']
T_WIN = video_time(D['t_win']) if D['t_win'] is not None else None
OUT_T = {q: t for t, q in OUTS}


def alive(q, t):
    """is genre q still in the battle at video time t (the restart brings everyone back)"""
    if t >= T_REVIVE:
        return True
    return not (q in OUT_T and t >= OUT_T[q])


def share(t):
    f = sim_frame(int(round(t * FPS)) % NF)
    return [float(x) / NT for x in D['count'][f]]


# ------------------------------------------------------------------ smoothed shares + the leader
def _smooth_all():
    raw = np.array([share(k / FPS) for k in range(NF)])
    a = 1 - np.exp(-1 / (0.35 * FPS))
    sm = raw.copy()
    for k in range(1, NF):
        sm[k] = sm[k - 1] + a * (raw[k] - sm[k - 1])
    # the crack (60.4-61): shares glide back to the opening's equal split, so frame 0 follows on
    k0, k1 = int(T_CRACK * FPS), NF
    for k in range(k0, k1):
        u = ease((k - k0) / (k1 - 1 - k0))
        sm[k] = sm[k0 - 1] * (1 - u) + raw[0] * u
    return sm


SMOOTH = _smooth_all()


def smooth_share(t):
    return SMOOTH[int(t * FPS) % NF]


def _lead_changes():
    """(t, genre) for the leader at 4 s and every later change (smoothed shares, hysteresis 2 % in the fill,
    1 % in the fight, at least 2.5 s apart so each one can be heard and captioned)"""
    out, lead = [], None
    t_end = T_WIN if T_WIN is not None else T_CRACK
    for f in range(int(4.0 * FPS), int(t_end * FPS), 3):
        t = f / FPS
        sh = SMOOTH[f] * np.array([alive(q, t) for q in range(4)])
        m = int(np.argmax(sh))
        if lead is None:
            lead = m
            out.append((t, m))
            continue
        hyst = 0.02 if t < T_BELL else 0.01
        if m != lead and sh[m] > sh[lead] + hyst and t - out[-1][0] >= 2.5:
            lead = m
            out.append((t, m))
    if T_WIN is not None and out[-1][1] != WINNER:
        out.append((T_WIN, WINNER))
    return out


LEADS = _lead_changes()                      # LEADS[0] = the leader once the parts lock in


def leader(t):
    """the genre whose style leads the song at video time t (the winner from the win to the crack)"""
    if T_WIN is not None and T_WIN <= t < T_REVIVE:
        return WINNER
    q = LEADS[0][1]
    for tl, m in LEADS:
        if t >= tl:
            q = m
    return q


def mood(t, tau=0.6):
    """0..1 weight per genre of the stage mood (lights): the leader eases in over ~tau s at each change"""
    w = np.zeros(4)
    prev = LEADS[0][1]
    w[prev] = 1.0
    for tl, m in LEADS[1:] + ([(T_WIN, WINNER)] if T_WIN is not None else []):
        if t < tl:
            break
        u = ease((t - tl) / tau)
        w = w * (1 - u)
        w[m] += u
    if t < LEADS[0][0]:                      # the opening (before the parts lock in): every genre a little
        u = ease(t / LEADS[0][0])
        w = np.full(4, 0.25) * (1 - u) + w * u
    if t >= T_CRACK:
        u = ease((t - T_CRACK) / (DUR - T_CRACK))
        w = w * (1 - u) + np.full(4, 0.25) * u
    return w


# ------------------------------------------------------------------ captions (impact style)
def _caps():
    e = []                                   # (t0, priority, lines, min hold, max hold)
    e.append((T_CRACK - 0.3 - DUR, 9, ['4 *GENRES*', '1 *STAGE*'], 0, 3.5))     # = the end's hook, across the seam
    e.append((2.65, 9, ['BATTLE OF THE BANDS', '*GENRE WARS*'], 1.3, 1.4))      # the series title tag
    e.append((2 * BAR, 5, ['EVERY *MOVE*', '= A *NOTE*'], 3.8, 9))
    e.append((4 * BAR, 6, ['THE *SONG* FOLLOWS', 'THE *LEADER*'], 3.9, 4.0))
    L = NAMES[leader(6 * BAR)]
    e.append((6 * BAR, 5, [f'*{L}* LEADS', f'= *{L}* SOUND'], 2.0, 9))
    e.append((7 * BAR, 4, ['MORE *STAGE*', '= *LOUDER*'], 3.0, 9))
    e.append((T_BELL, 9, ['*FULL.*', 'NOW THEY *FIGHT*'], 2.4, 2.6))
    e.append((T_BELL + 2.65, 3, ["WHO'S *WINNING?*", 'LISTEN @'], 2.5, 9))
    for tl, q in LEADS[1:]:
        if T_WIN is not None and tl >= T_WIN:
            continue
        e.append((tl, 7, [f'*{NAMES[q]}*', 'TAKES OVER'], 2.4, 3.2))
    if OUTS:
        t, q = OUTS[0]
        e.append((t, 10, [f'*{NAMES[q]}* IS OUT!', f'NO MORE {INSTR[q]}'], 2.8, 3.4))
    if len(OUTS) >= 2:
        t, q = OUTS[1]
        left = [k for k in range(4) if k not in (OUTS[0][1], q)]
        left = [k for k in SHOW if k in left]
        e.append((t, 10, ['*2* LEFT', f'{NAMES[left[0]]} VS {NAMES[left[1]]}'], 2.8, 9))
    if T_WIN is not None:
        e.append((T_WIN, 11, [f'*{NAMES[WINNER]}*', 'WINS!'], 3.0, 3.0))
        e.append((T_WIN + 3.05, 8, ['WHICH *GENRE*', 'ARE *YOU?*'], 0, 30))
    e.append((T_CRACK - 0.3, 12, ['4 *GENRES*', '1 *STAGE*'], 0, 30))
    # a caption holds until the next one, but a lower-priority caption may not cut one before its minimum hold:
    # a lead change waits until that caption is done; anything else is dropped
    e.sort(key=lambda a: a[0])
    kept = []
    while e:
        t0, pr, lines, mind, maxd = e.pop(0)
        if kept:
            k0, kpr, _, kmin, _ = kept[-1]
            if t0 < k0 + kmin and pr <= kpr:
                if pr == 7:
                    e.append((k0 + kmin + 0.05, pr, lines, mind, maxd))
                    e.sort(key=lambda a: a[0])
                continue
        kept.append((t0, pr, lines, mind, maxd))
    caps = []
    for k, (t0, pr, lines, mind, maxd) in enumerate(kept):
        t1 = kept[k + 1][0] - 0.05 if k + 1 < len(kept) else DUR + 0.6
        t1 = min(t1, t0 + maxd)
        if t1 - t0 >= 1.2 or k + 1 == len(kept):
            caps.append((t0, t1, lines))
    # fight-phase fillers so the text changes every few seconds: a blank stretch over 2.4 s or a hold over 5.2 s
    # gets a caption that fits how many genres are left
    t_end = T_WIN if T_WIN is not None else T_CRACK

    def filler(t):
        left = [q for q in SHOW if alive(q, t)]
        if len(left) == 4:
            return ["WHO'S *WINNING?*", 'LISTEN @']
        if len(left) == 3:
            return ['*3* LEFT', ' '.join(NAMES[q] for q in left)]
        return ['WHO TAKES', 'THE *SONG?*']
    out = []
    for k, (t0, t1, lines) in enumerate(caps):
        nxt = caps[k + 1][0] if k + 1 < len(caps) else DUR
        if T_BELL <= t0 < t_end and t1 - t0 > 5.2:
            out.append((t0, t0 + 3.2, lines))
            out.append((t0 + 3.25, t1, filler(t0 + 3.25)))
        else:
            out.append((t0, t1, lines))
        if T_BELL <= t1 < t_end and nxt - t1 > 2.4:
            out.append((t1 + 0.05, min(nxt - 0.05, t1 + 4.5), filler(t1 + 0.05)))
    return out


CAPTIONS = _caps()
LANDINGS = impact_landings([c for c in CAPTIONS if c[0] >= 0])       # a soft thud on each line landing
LABEL_T = [0.15, 0.5, 0.85, 1.2]             # genre labels pop at their bases (in SHOW order)
LABEL_END = 7.9


def beat_phase(t):
    return ((t - GRID0) % BEAT) / BEAT


def beat_env(t, decay=0.12):
    return math.exp(-((t - GRID0) % BEAT) / decay)


def bar_env(t, decay=0.3):
    return math.exp(-((t - GRID0) % BAR) / decay)


if __name__ == '__main__':
    print('seed', int(D['seed']), 'bell', round(T_BELL, 2), 'win', NAMES[WINNER] if WINNER is not None else '-',
          round(T_WIN, 2) if T_WIN else '-')
    print('outs', [(round(t, 2), NAMES[q]) for t, q in OUTS])
    print('leads', [(round(t, 2), NAMES[q]) for t, q in LEADS])
    for t0, t1, lines in CAPTIONS:
        print(f'  {t0:6.2f}-{t1:6.2f}  ' + ' / '.join(lines))
