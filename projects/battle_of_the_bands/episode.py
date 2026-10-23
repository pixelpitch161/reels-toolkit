"""Episode 1 config: the arena, the four teams and their balance parameters. sim.py runs it.

New episode: copy this file, change the arena shape / line-up / parameters, run seed_search.py, pick a seed.
The balance below was tuned over many seeds so every team wins some runs (see README)."""
from battle import Arena, Battle, SandTeam, BallTeam, RippleTeam, VineTeam

N = 72
SHAPE = 'circle'
OFFSET = 1.4             # video time t shows sim time t + OFFSET (the run starts 1.4 s before frame 0; see timeline)
T_FIGHT = 20.0 + OFFSET  # the bell (video 20.0 s)
BEAT = 0.5               # 120 BPM; ripple rings fire on the beat
RING_PHASE = OFFSET % BEAT
SEED = 82                # picked by seed_search.py over 800 seeds (honest run, never edited)

TEAMS = dict(
    sand=dict(hopper_col=int(N / 2 - 7), width=2.5, sweep=14, sweep_T=7, pour=60, p_impact=0.7, p_down=0.004,
              p_diag=0.0025, p_side=0.0005),
    ball=dict(start=(14.0, 54.0), v0=(-20.0, 4.0), g=42.0, r_paint=1.2, r_smash=3.5, pierce_speed=16, pierce_loss=0.92,
              jitter=10),
    ripple=dict(period=BEAT, phase=RING_PHASE, speed=26.0, p_fill=0.9, p_push=0.4, reach_fill=4, reach_power=True),
    vine=dict(rate=7, rate_fight=9, max_tips=20, max_tips_fill=7, p_burrow=0.55),
)
RULES = dict(heat_T=10, power_alpha=0.5, alpha_T=6, power_hi=6.0, power_lo=0.15, close_T=4, close_p=0.06,
             ko_share=0.025, tempo_power=1.0, rounds=True)


def make(seed=SEED, teams=None, rules=None):
    arena = Arena(N, SHAPE)
    p = {k: dict(v) for k, v in TEAMS.items()}
    p['ripple']['base'] = arena.rim_cell(196, 2)
    p['vine']['base'] = arena.rim_cell(-32, 2)
    for k, v in (teams or {}).items():
        p[k].update(v)
    r = dict(RULES)
    r.update(rules or {})
    lineup = [SandTeam(**p['sand']), BallTeam(**p['ball']), RippleTeam(**p['ripple']), VineTeam(**p['vine'])]
    return Battle(arena, lineup, seed=seed, t_fight=T_FIGHT, **r)
