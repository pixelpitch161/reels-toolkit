"""Mozart's Dice Waltz (IDEAS #7) - ONE timeline drives picture AND sound.

The dice literally compose the track: every roll's sum picks one bar from the 1792 table
(waltz_table.TABLE[sum-2][column]); that bar lands on the parchment staff and the fortepiano
plays it. Back half: more cups roll at once and their waltzes stack (every column shares one
harmony, verified in waltz_table.py), the sum histogram on the wall fills into the 2-12 triangle.

    0-4    hook: dice clack in the tray, bar 1 drops onto the staff     759 TRILLION / WALTZES
    4-8    the parchment table of 176 bars unfurls on the floor         FROM 2 DICE / (1792)
    8-12   roll -> 7 -> row 7 lights -> bar flies to the staff          DICE SUM = BAR
    12-16  camera drifts over the table grid; waltz builds bar by bar   11 SUMS x 16 BARS
    16-20  sum histogram lights up on the wall; dice-pair panel         7 BEATS 2 / 6 TO 1
    20-24  columns 8 and 16 glow (1-2 real choices); roll counter       46 QUADRILLION ROLLS...
    24-28  counter snaps down; waltz #1 finishes                        ...759 TRILLION / WALTZES
    28-35  calendar flips, moon arcs across the window; year counter    ALL OF THEM: / 720M YEARS
    35-40  second cup: two waltzes at once                              2 AT ONCE
    40-46  ten cups, layers stack on new timbres                        10 DICE CUPS
    46-52  climax: dice rain, histogram locks to the triangle           7 ALWAYS WINS
    52-60  thins to one fortepiano bar, dice settle into one cup, loop  ROLL 2 DICE / WRITE BAR 1
"""
import math
import numpy as np
import waltz_table as WT

FPS = 30
DUR = 60.0
NF = int(FPS * DUR)

BAR = 1.2                  # one 3/8 bar = 1.2 s (an eighth = 0.4 s): a lively salon waltz
S16 = BAR / 6              # one sixteenth
FLIGHT = 0.42              # dice: throw -> first contact

# ---------------------------------------------------------------- sections
T_TABLE = 4.0              # chart starts to unfurl
T_UNFURL = (4.3, 7.0)
T_DEMO = 8.0               # the slow, explained roll (sum 7)
T_W1 = 9.0                 # waltz #1 plays continuously from bar 1
T_GRID = 12.0
T_HIST = 16.0
T_QUAD = 20.0
T_SNAP = 24.0
T_YEARS = 28.0
T_TWO = 35.0               # back half: waltzes stack (bar k at T_TWO + (k-1)*BAR)
T_TEN = T_TWO + 4 * BAR    # 39.8: ten cups
T_CLIMAX = T_TWO + 9 * BAR  # 45.8: dice rain
T_THIN = T_TWO + 14 * BAR  # 51.8: layers drop out
T_FINAL = T_TWO + 15 * BAR  # 53.0: last bar, one fortepiano
T_SETTLE = 54.6            # dice hop back into the cup
T_SHAKE = 58.2             # cup shakes...
T_LOOP_THROW = 59.83       # ...and throws: the dice are in the air at t = 0 (loop)
T_CLEAR = (55.4, 57.4)     # the staff empties (right to left) so the loop starts clean
T_REROLL = (56.0, 58.8)    # the chart rolls back up

# ---------------------------------------------------------------- waltz #1 (front half)
W1_SUMS = [8, 6, 7, 9, 5, 7, 8, 6, 10, 7, 4, 8, 6, 9, 7, 7]
W1_BARS = [WT.bar_for(c, s) for c, s in enumerate(W1_SUMS)]


def w1_play(k):
    """start time of bar k (0-15) of waltz #1 in the continuous run"""
    return T_W1 + k * BAR


SOLO_PLAYS = [(0, 0.9), (1, 5.6)]          # bars 1 and 2 heard alone while the setup is shown

# ---------------------------------------------------------------- dice pairs for each sum
_rng = np.random.default_rng(1792)


def pair_for(s, rng=_rng):
    opts = [(a, s - a) for a in range(1, 7) if 1 <= s - a <= 6]
    return opts[rng.integers(len(opts))]


CUP_COLS = {   # cup id -> colour of its notes / glow ring
    'A': (250, 238, 214), 'B': (255, 128, 104), 'C': (124, 236, 196), 'D': (140, 196, 255),
    'E': (255, 200, 90), 'F': (255, 130, 210), 'G': (120, 236, 255), 'H': (196, 160, 255),
    'I': (190, 240, 110), 'J': (255, 170, 140)}
CUPS = list(CUP_COLS)


class Roll:
    __slots__ = ('cup', 't_throw', 't_land', 'sum', 'dice', 'col', 'bar', 't_play', 'where', 'seed', 'demo')

    def __init__(self, cup, t_land, s, col, t_play, where='tray', demo=False, seed=0):
        self.cup, self.t_land, self.sum, self.col, self.t_play = cup, t_land, s, col, t_play
        self.t_throw = t_land - FLIGHT
        self.dice = pair_for(s)
        self.bar = WT.bar_for(col, s) if col is not None else None
        self.where, self.demo, self.seed = where, demo, seed


ROLLS = []
# front half: cup A only
ROLLS.append(Roll('A', 0.25, W1_SUMS[0], 0, 0.9, seed=1))
ROLLS.append(Roll('A', 5.0, W1_SUMS[1], 1, 5.6, seed=2))
ROLLS.append(Roll('A', 8.5, W1_SUMS[2], 2, w1_play(2), demo=True, seed=3))
for k in range(3, 16):
    ROLLS.append(Roll('A', w1_play(k) - 1.0, W1_SUMS[k], k, w1_play(k), seed=10 + k))


def cups_for_bar(k):
    """which cups compose bar k (0-15) of the stacked back half"""
    if k < 4:
        return CUPS[:2]
    if k < 14:
        return CUPS
    if k == 14:
        return CUPS[:3]
    return CUPS[:1]


def w2_play(k):
    return T_TWO + k * BAR


_rb = np.random.default_rng(35)
W2 = {c: [] for c in CUPS}               # cup -> list of (col, sum, bar, t_play)
for k in range(16):
    for ci, c in enumerate(cups_for_bar(k)):
        s = int(_rb.integers(1, 7) + _rb.integers(1, 7))       # fair dice
        if c == 'A' and k == 15:
            s = 7                                             # the lead ends on the final cadence bar
        land = w2_play(k) - 0.7 + 0.03 * ci
        ROLLS.append(Roll(c, land, s, k, w2_play(k), where='tray' if c in 'AB' else 'floor', seed=100 + 16 * ci + k))
        W2[c].append((k, s, WT.bar_for(k, s), w2_play(k)))

# climax dice rain: pairs that only feed the histogram (no bars)
RAIN = []
_rr = np.random.default_rng(82)
for i in range(150):
    t = T_CLIMAX + 0.2 + _rr.uniform(0, T_THIN - T_CLIMAX - 0.6)
    s = int(_rr.integers(1, 7) + _rr.integers(1, 7))
    RAIN.append((t, s, _rr.uniform(-1.45, 1.45), _rr.uniform(-1.2, 1.4), _rr.uniform(0, 6.283), int(_rr.integers(1 << 30))))
RAIN.sort()

ROLLS.sort(key=lambda r: r.t_land)
LEAD = [r for r in ROLLS if r.cup == 'A']


def lead_roll(t):
    """the cup-A roll that owns the tray at time t (dice in flight or resting)"""
    cur = None
    for r in LEAD:
        if r.t_throw <= t:
            cur = r
    return cur


# ---------------------------------------------------------------- histogram (counts by sum)
HIST_EVENTS = sorted([(r.t_land, r.sum) for r in ROLLS] + [(t + 0.5, s) for t, s, *_ in RAIN])
TRIANGLE = np.array([1, 2, 3, 4, 5, 6, 5, 4, 3, 2, 1], float) / 36.0


def hist_counts(t):
    c = np.zeros(11)
    for te, s in HIST_EVENTS:
        if te > t:
            break
        c[s - 2] += 1
    return c


# ---------------------------------------------------------------- playback model (picture + sound)
def staff_state(t):
    """what the staff shows: list of (bar slot 0-15, cup, bar number, t_appear) and the playhead"""
    items = []
    if t < T_YEARS + 6.5:
        for k, r in enumerate(LEAD[:16]):
            appear = r.t_land + (1.2 if r.demo else 0.35)
            if t >= appear:
                items.append((k, 'A', r.bar, appear))
    else:
        for c in CUPS:
            for (k, s, bar, tp) in W2[c]:
                appear = tp - 0.32
                if t >= appear:
                    items.append((k, c, bar, appear))
    if t > T_CLEAR[0]:
        keep = 16 * (1 - (t - T_CLEAR[0]) / (T_CLEAR[1] - T_CLEAR[0]))
        items = [it for it in items if it[0] < keep]
    return items


def playhead(t):
    """(slot float, alpha) for the glowing playhead on the staff"""
    for k, tp in SOLO_PLAYS:
        if tp <= t < tp + BAR:
            return k + (t - tp) / BAR, 1.0
    if T_W1 <= t < T_W1 + 16 * BAR:
        return (t - T_W1) / BAR, 1.0
    if T_YEARS + 1.0 <= t < T_YEARS + 1.0 + 4 * BAR:
        return (t - T_YEARS - 1.0) / BAR, 0.6
    if T_TWO <= t < T_TWO + 16 * BAR:
        return (t - T_TWO) / BAR, 1.0
    return None, 0.0


def playing_notes(t):
    """lead-layer MIDI notes sounding at t (for the self-playing fortepiano keys)"""
    out = []

    def add(bar, tp, second=False):
        if tp <= t < tp + BAR:
            rh, lh = WT.bar_notes(bar, second)
            for o, d, p in rh + lh:
                a = tp + o * S16
                if a <= t < a + d * S16 * 0.9:
                    out.append(p)
    for k, tp in SOLO_PLAYS:
        add(W1_BARS[k], tp)
    for k in range(16):
        add(W1_BARS[k], w1_play(k), k == 7)
        add(W2['A'][k][2], W2['A'][k][3], k == 7)
    return out


# ---------------------------------------------------------------- captions (<= 18 chars per line)
CAPTIONS = [
    (0.0, 4.0, ["*759 TRILLION*", "WALTZES"]),
    (4.1, 8.0, ["FROM *2 DICE*", "(1792)"]),
    (8.1, 12.0, ["*DICE SUM* = BAR"]),
    (12.1, 16.0, ["*11* SUMS × *16* BARS"]),
    (16.1, 20.0, ["*7* BEATS *2*", "6 TO 1"]),
    (20.1, 24.0, ["46 QUADRILLION", "ROLLS..."]),
    (24.1, 28.0, ["...*759 TRILLION*", "WALTZES"]),
    (28.1, 35.0, ["ALL OF THEM:", "*720M* YEARS"]),
    (35.1, 39.8, ["*2* AT ONCE"]),
    (39.9, 45.8, ["*10* DICE CUPS"]),
    (45.9, 52.0, ["*7* ALWAYS WINS"]),
    (52.1, 59.4, ["ROLL *2 DICE*", "WRITE BAR 1"]),
]

# HUD panels (slide in from the bottom)
HUD_PAIRS = (16.2, 20.0)
HUD_ROLLS = (20.2, 28.0)          # number snaps from 11^16 to 2 x 11^14 at T_SNAP
HUD_YEARS = (28.3, 34.9)


def moon_phase(t):
    """0..1 progress of the time-lapse (moon arcs across the window several times)"""
    if t < T_YEARS + 0.3 or t > T_TWO - 0.2:
        return 0.0
    u = (t - T_YEARS - 0.3) / (T_TWO - 0.5 - T_YEARS)
    return u


def beat_env(t, decay=0.15):
    """0..1 pulse on every bar downbeat while a waltz is playing"""
    for t0, n in ((T_W1, 16), (T_TWO, 16)):
        if t0 <= t < t0 + n * BAR:
            return math.exp(-((t - t0) % BAR) / decay)
    for k, tp in SOLO_PLAYS:
        if tp <= t < tp + BAR:
            return math.exp(-(t - tp) / decay)
    return 0.0
