"""Prime-Number Cicadas (IDEAS #12) - ONE timeline drives picture AND sound.

Periodical cicadas live 13 or 17 years underground, then emerge together. Brood XIX (13-year) and
Brood XIII (17-year) emerged in the same year in 2024, the first time since 1803 for these two
broods (next: 2245). 13 and 17 are prime, so a 13/17 pair lines up only every 13 x 17 = 221 years.

The music IS the calendar: 1 year = 1 sixteenth at 120 BPM, so 1803 -> 2024 = 221 sixteenths
= 27.625 s. XIX hits every 13 sixteenths, XIII every 17: a real 13-against-17 polyrhythm over an
original 4/4 groove that lands together only on 2024 (a downbeat, the climax). The same lists drive
the bursts on screen, the HUD ticks and the calls in the mix.

    0-4    hook: soil cut-away, two nymphs side by side            13 x 17 = 221 YRS
    4-8    both burst out, trees erupt, wide establishing shot     2024: THESE 2 / LAST MET 1803
    8-12   split view underground, progress rings 13 vs 17         13 YRS VS 17 YRS
    12-16  13 and 17 glow as numbers                               13 & 17: / BOTH PRIME
    16-20  ghost 12- and 18-year broods meet over and over         12 & 18: MEET / EVERY 36 YRS
    20-23  ghosts fade, 13 & 17 return, HUD lanes ready            13 & 17: / EVERY 221 YRS
    23.4   1803: both erupt, the year race starts (groove)          1803 -> ...  /  1 YEAR = ...
    29.75  1854 / 1855 near miss (a sixteenth apart)               SO CLOSE... / 1 YEAR APART
    33.7   -                                                       WHY PRIME? / STILL DEBATED
    38.4   -                                                       XIX: 17 TIMES / XIII: 13 TIMES
    44.5   1972 / 1973 near miss                                   1 YEAR... AGAIN
    48.7   build                                                   221 YRS LATER...
    51.0   2024: both halves erupt, sky fills, full groove         2024
    54-60  eggs drop, nymphs burrow, camera back to the soil       TRY IT: / CLAP 13 / TAP 17

Note: the backlog beat sheet put the near misses at 35-40 s and 46-50 s; with the exact
1 year = 1 sixteenth rule and 2024 on the 51 s downbeat they fall at 29.75 s and 44.5 s,
so the captions follow the music (the rule is the point of the video).
"""
import math
import numpy as np

FPS = 30
DUR = 60.0
NF = int(FPS * DUR)

BPM = 120
BEAT = 60.0 / BPM          # 0.5 s
S16 = BEAT / 4             # 0.125 s = one year in the race
BAR = 4 * BEAT             # 2 s
GRID0 = 1.0                # bar lines at odd seconds, so 2024 lands on a downbeat

# ---------------------------------------------------------------- the calendar
Y0, Y1 = 1803, 2024
T_2024 = 51.0
T_1803 = T_2024 - (Y1 - Y0) * S16      # 23.375
XIX_P, XIII_P = 13, 17
XIX_YEARS = [Y0 + XIX_P * k for k in range((Y1 - Y0) // XIX_P + 1)]     # 1803 ... 2024 (18 years)
XIII_YEARS = [Y0 + XIII_P * k for k in range((Y1 - Y0) // XIII_P + 1)]  # 1803 ... 2024 (14 years)
NEAR = [(1854, 1855), (1972, 1973)]    # XIII 1854 / XIX 1855, XIX 1972 / XIII 1973 (calc)
assert XIX_YEARS[-1] == Y1 and XIII_YEARS[-1] == Y1


def year_t(y):
    return T_1803 + (y - Y0) * S16


def race_year(t):
    """the year shown by the counter at time t (float, clamped)"""
    return min(Y1, max(Y0, Y0 + (t - T_1803) / S16))


# ---------------------------------------------------------------- sections
T_CLIMB0 = 2.6             # hook nymphs start to climb
T_POP0 = 4.5               # hook emergence ("2024")
T_REVEAL = 4.0
T_BURROW0 = (6.0, 7.6)     # the new generation burrows down after the hook burst
T_SPLIT = 8.0
RING_T0, RING_RATE = 8.4, 5.0          # progress rings: 5 "years" per second
RING_DONE = {'L': RING_T0 + 13 / RING_RATE, 'R': RING_T0 + 17 / RING_RATE}   # 11.0, 11.8
T_PRIME = 12.0
T_PING = 12.35
T_GHOST = 16.0
GH_T0, GH_RATE = 16.5, 36.0            # ghost race: 36 years per second
GH_T1 = 19.9
G12 = [GH_T0 + k * 12 / GH_RATE for k in range(20) if GH_T0 + k * 12 / GH_RATE <= GH_T1]
G18 = [GH_T0 + k * 18 / GH_RATE for k in range(20) if GH_T0 + k * 18 / GH_RATE <= GH_T1]
G_MEET = [GH_T0 + k * 36 / GH_RATE for k in range(10) if GH_T0 + k * 36 / GH_RATE <= GH_T1]   # 16.5 17.5 18.5 19.5
T_221 = 20.0
T_PANEL_IN, T_PANEL_OUT = 16.2, 53.6
T_LANES = 20.3             # ghost lanes morph into XIX / XIII lanes
T_GROOVE = 23.0            # bar line just before 1803
T_BUILD = 47.0
T_CLIMAX = T_2024
T_END = 54.0               # climax over, ending starts
T_EGGS = (54.0, 55.9)
T_BURROW1 = (55.6, 58.2)
T_HOME = 58.6              # camera back at the opening close-up

# ---------------------------------------------------------------- broods
SIDES = ('L', 'R')
PERIOD = {'L': XIX_P, 'R': XIII_P}
NAME = {'L': 'XIX', 'R': 'XIII'}
COL = {'L': (255, 150, 58), 'R': (78, 228, 214)}          # XIX warm orange, XIII teal
GHOST_COL = (196, 178, 255)
YEARS = {'L': XIX_YEARS, 'R': XIII_YEARS}

# ---------------------------------------------------------------- emergences (one list -> bursts, ticks, calls)
EMERGE = []        # dicts: side, t, year, kind, n, life
EMERGE.append(dict(side='L', t=T_POP0, year=2024, kind='hook', n=320, life=3.3))
EMERGE.append(dict(side='R', t=T_POP0, year=2024, kind='hook', n=320, life=3.3))
for s in SIDES:
    for y in YEARS[s]:
        kind = 'climax' if y == Y1 else ('joint' if y == Y0 else 'race')
        n = {'race': 210, 'joint': 300, 'climax': 520}[kind]
        life = {'race': 1.28, 'joint': 1.5, 'climax': T_END + 1.6 - T_2024}[kind]
        EMERGE.append(dict(side=s, t=year_t(y), year=y, kind=kind, n=n, life=life))
EMERGE.sort(key=lambda e: (e['t'], e['side']))
for i, e in enumerate(EMERGE):
    e['id'] = i
RACE_HITS = {s: [year_t(y) for y in YEARS[s]] for s in SIDES}
SKY_FILL = (T_2024, T_END + 1.4)       # extra fliers fill the sky at the climax

# birds that swoop through a crown and snatch one cicada: (t0, dur, side)
BIRDS = [(year_t(1829) + 0.25, 1.5, 'L'), (year_t(1888) + 0.3, 1.6, 'R'), (year_t(1933) + 0.25, 1.5, 'L'),
         (year_t(1990) + 0.3, 1.6, 'R'), (T_2024 + 0.6, 1.8, 'L'), (T_2024 + 1.1, 1.8, 'R')]


def last_emergence(side, t):
    """most recent emergence time for a side before t (for shells on bark, etc.)"""
    ts = [e['t'] for e in EMERGE if e['side'] == side and e['t'] <= t]
    return ts[-1] if ts else None


def n_emerged(side, t):
    return sum(1 for e in EMERGE if e['side'] == side and e['t'] <= t)


# ---------------------------------------------------------------- nymphs underground
def _climb(u):
    return u * u * (3 - 2 * u)


def nymph_state(side, t):
    """(s, size, present): s = position along its tunnel (0 chamber .. 1 surface exit),
    size 0..1 (hatchlings are small), present = drawn underground at all"""
    wig = 0.0
    # hook: in the chamber, then climb and pop at 4.5
    if t < T_CLIMB0:
        return 0.0, 1.0, True
    if t < T_POP0:
        return _climb((t - T_CLIMB0) / (T_POP0 - T_CLIMB0)), 1.0, True
    a, b = T_BURROW0
    if t < a:
        return 1.0, 0.0, False
    if t < b:
        u = (t - a) / (b - a)
        return 1.0 - _climb(u), 0.35 + 0.65 * u, True
    # the race: climb in the last 18% of each cycle, burrow down in the first 12%
    P = PERIOD[side] * S16
    if t < T_1803 - 0.18 * P:
        return 0.0, 1.0, True
    if t < T_2024:
        ph = ((t - T_1803) % P) / P if t >= T_1803 else 1.0 + (t - T_1803) / P
        if ph < 0.12:
            u = ph / 0.12
            return 1.0 - _climb(u), 0.35 + 0.65 * u, True
        if ph < 0.82:
            return 0.0, 1.0, True
        return _climb((ph - 0.82) / 0.18), 1.0, True
    a, b = T_BURROW1
    if t < a:
        return 1.0, 0.0, False
    if t < b:
        u = (t - a) / (b - a)
        return 1.0 - _climb(u), 0.35 + 0.65 * u, True
    return 0.0, 1.0, True


def ring_frac(side, t):
    """8-12 s progress ring (years underground / cycle)"""
    if t < RING_T0:
        return 0.0
    return min(1.0, (t - RING_T0) * RING_RATE / PERIOD[side])


def ghost_year(t):
    return max(0.0, (min(t, GH_T1) - GH_T0) * GH_RATE)


# ---------------------------------------------------------------- captions (*x* = XIX orange, ~x~ = XIII teal)
CAPTIONS = [
    (0.0, 4.0, ["*13* × ~17~ = 221 YRS"]),
    (4.1, 8.0, ["2024: THESE 2", "LAST MET 1803"]),
    (8.1, 12.0, ["*13 YRS* VS ~17 YRS~"]),
    (12.1, 16.0, ["*13* & ~17~:", "BOTH PRIME"]),
    (16.1, 20.0, ["12 & 18: MEET", "EVERY 36 YRS"]),
    (20.1, 23.3, ["*13* & ~17~:", "EVERY 221 YRS"]),
    (23.4, 26.4, ["1803 → ..."]),
    (26.5, 29.6, ["1 YEAR =", "1 SIXTEENTH NOTE"]),
    (29.7, 33.6, ["SO CLOSE...", "1 YEAR APART"]),
    (33.7, 38.3, ["WHY PRIME?", "STILL DEBATED"]),
    (38.4, 44.4, ["*XIX*: 17 TIMES", "~XIII~: 13 TIMES"]),
    (44.5, 48.6, ["1 YEAR... AGAIN"]),
    (48.7, 50.95, ["221 YRS LATER..."]),
    (51.0, 54.0, ["2024", "*XIX* + ~XIII~"]),
    (54.2, 59.0, ["TRY IT:", "*CLAP 13* / ~TAP 17~"]),
]

# small one-line tags (scale 1): (t0, t1, text, colour, y)
TAGS = [
    (4.7, 8.0, "ONLY OVERLAP: CENTRAL ILLINOIS", (236, 226, 204), 606),
    (33.9, 38.3, "IDEAS: FEWER HYBRIDS? PREDATORS?", (236, 226, 204), 114),
    (51.7, 54.0, "NEXT TIME: 2245", (236, 226, 204), 114),
]


# ---------------------------------------------------------------- audio helpers
def beat_index(t):
    return (t - GRID0) / BEAT


def on_grid(t, step=S16):
    return abs(((t - GRID0) / step) - round((t - GRID0) / step)) < 1e-6


assert on_grid(T_1803) and on_grid(T_2024, BAR) and all(on_grid(x, BEAT) for x in G_MEET)
