"""Battle of the Bands: Genre Wars (IDEAS #25). Episode config on the #20 engine, which is imported unchanged
from projects/battle_of_the_bands/battle.py.

The four genres each reuse one engine team (no new simulation code):
    ORCHESTRA (gold)  = SAND    a cascading wall of sound pours from the hopper
    ROCK      (red)   = BALL    smashing ricochets
    EDM       (cyan)  = RIPPLE  a ring on every beat
    JAZZ      (blue)  = VINE    improvised wandering lines
Engine index order stays SAND, BALL, RIPPLE, VINE (= 0 ORCHESTRA, 1 ROCK, 2 EDM, 3 JAZZ) so the balance tuned in
#20 carries over. Changes for 61 s at 118.03 BPM: the ripple period is one beat (0.5083 s), the bell sits on bar
10 (20.33 s), and the hopper sweep is a whole-cycle period (61 / 9 s)."""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', '..'))
sys.path.append(os.path.join(HERE, '..', 'battle_of_the_bands'))     # the engine (battle.py), not edited; appended so
                                                                    # this project's own modules (sim, scene...) win
from battle import Arena, Battle, SandTeam, BallTeam, RippleTeam, VineTeam
from reelkit.config import loop_bpm

DUR = 61.0
N = 72
SHAPE = 'circle'
OFFSET = 0.0             # video time = sim time: frame 0 is the empty stage as the four seeds drop
BPM = loop_bpm(118, DUR)  # 118.03: 30 bars in 61 s
BEAT = 60.0 / BPM        # 0.5083 s; EDM rings fire on the beat
BAR = 4 * BEAT
GRID0 = 0.0              # bar k starts at k * BAR: the loop point is a bar line
T_FIGHT = GRID0 + 10 * BAR   # the bell: 20.33 s, bar 10
RING_PHASE = GRID0 % BEAT
SEED = 189               # picked by seed_search.py (5 fits in 192 runs; honest run, never edited)

NAMES = ['ORCHESTRA', 'ROCK', 'EDM', 'JAZZ']
COLORS = [(255, 196, 64), (242, 62, 66), (40, 232, 226), (86, 112, 255)]

TEAMS = dict(
    sand=dict(hopper_col=int(N / 2 - 7), width=2.5, sweep=14, sweep_T=DUR / 9, pour=60, p_impact=0.7, p_down=0.004,
              p_diag=0.0025, p_side=0.0005),
    ball=dict(start=(14.0, 54.0), v0=(-20.0, 4.0), g=42.0, r_paint=1.2, r_smash=3.5, pierce_speed=16, pierce_loss=0.92,
              jitter=10),
    ripple=dict(period=BEAT, phase=RING_PHASE, speed=26.0, p_fill=0.9, p_push=0.4, reach_fill=4, reach_power=True),
    vine=dict(rate=7, rate_fight=9, max_tips=20, max_tips_fill=7, p_burrow=0.55),
)
RULES = dict(heat_T=10, power_alpha=0.5, alpha_T=6, power_hi=6.0, power_lo=0.15, close_T=4, close_p=0.06,
             ko_share=0.025, tempo_power=1.0, rounds=True)


def make(seed=None, teams=None, rules=None):
    seed = SEED if seed is None else seed
    arena = Arena(N, SHAPE)
    p = {k: dict(v) for k, v in TEAMS.items()}
    p['ripple']['base'] = arena.rim_cell(196, 2)
    p['vine']['base'] = arena.rim_cell(-32, 2)
    for k, v in (teams or {}).items():
        p[k].update(v)
    r = dict(RULES)
    r.update(rules or {})
    lineup = [SandTeam(name=NAMES[0], color=COLORS[0], **p['sand']),
              BallTeam(name=NAMES[1], color=COLORS[1], **p['ball']),
              RippleTeam(name=NAMES[2], color=COLORS[2], **p['ripple']),
              VineTeam(name=NAMES[3], color=COLORS[3], **p['vine'])]
    return Battle(arena, lineup, seed=seed, t_fight=T_FIGHT, **r)
