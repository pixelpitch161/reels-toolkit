"""Count the Thunder v2 (IDEAS #13, "v2 rework"): ONE timeline for picture and sound.

The science: light arrives essentially instantly, sound travels ~0.343 km/s. Count from the flash to
the thunder: seconds / 5 = miles (NWS rule), seconds / 3 = km (Environment Canada rule). Both rules
are approximations (343 m/s is ~4.7 s/mile, ~2.9 s/km, and it varies with temperature).

v2 arc (chaos -> lock-in, straight from the physics):
  0-3 s    cold open on the climax: overhead strike on the barn rod, flash + clap fused, full groove
  3-5 s    "HOW FAR WAS THAT?" - the storm rewinds back to the horizon (the world runs backwards)
  5-30 s   a storm far away throws many strikes at different distances. Every flash sends a sound ring
           across the prairie and its rumble sounds exactly when that ring reaches the cabin, so the
           rumbles land out of step with the flashes (natural chaos). One flash is tagged at 5.0 s and
           counted on the beat (count ticks = the hats): 15 s -> 3 MI / ~5 KM. Delays shrink as it nears.
  30-50 s  lock-in: one flash per bar on the downbeat (bright snare); the thunder is the kick, late by
           the real flash-to-bang time 2 -> 1 -> 1/2 -> 1/4 -> 1/8 s, fusing at the overhead strike (50 s)
  50-60 s  climax, then a second cell races in: 3 s, 2 s, 1 s ("the next one's closer") and its
           overhead strike is frame 0 (the cold open), so the video loops.

Every strike in STRIKES drives the flash, the bolt, its sound ring, the window shadows, the HUD, the
captions, the thunder in the mix (time AND low-pass) and the cat. Strikes on downbeats are stylised
(real lightning isn't rhythmic), but every delay matches the distance it is drawn at and captioned as."""
import math

FPS = 30
DUR = 60.0
NF = int(FPS * DUR)

C_SOUND = 0.343            # km/s (20 C); only used for the "true" distance, captions use the rules

BPM = 120
BEAT = 60.0 / BPM          # 0.5 s
BAR = 4 * BEAT             # 2.0 s

T_CO = 3.0                 # cold open ends, rewind starts
T_REW = 5.0                # rewind ends on the tagged flash
T_FILL = 27.0              # one 2/4 bar (27-28) moves the downbeats from odd to even seconds
T_OPEN = 30.0              # groove opens to full (filter wide by 32)
T_CLIMAX = 50.0            # overhead strike: flash + clap on the same downbeat

# ---------------------------------------------------------------- the strikes
# (flash time s, flash-to-bang s, azimuth deg, kind)
#   pre    struck before the rewind lands (flash unseen): only its ring (already crossing the prairie
#          at 5 s) and its rumble
#   chaos  the far storm: many strikes at different distances
#   count  tagged + counted on screen (HUD stopwatch, count ticks)
#   lock   one per bar on the downbeat (30-50 s)
#   rod    the overhead strike on the barn's lightning rod (cold open at 0 s, climax at 50 s)
#   cell2  the second cell racing in (54/56/58 s; its rod strike is frame 0)
PRE = [(-9.2, 16.3, 6), (-7.5, 17.3, 2), (-6.4, 12.0, -4), (-2.6, 11.0, -16), (1.4, 9.9, -9), (3.6, 9.3, 9)]
CHAOS = [
    (5.7, 8.6, 5),      # lands 14.3
    (6.4, 9.3, -17),    # 15.7
    (7.6, 9.1, 9),      # 16.7
    (8.5, 9.0, -5),     # 17.5
    (9.8, 8.6, 1),      # 18.4
    (10.5, 8.7, -9),    # 19.2
    (11.9, 9.2, 4),     # 21.1
    (12.9, 10.4, -14),  # 23.3
    (14.0, 8.4, 8),     # 22.4
    (14.7, 10.4, -2),   # 25.1
    (16.0, 8.2, -18),   # 24.2
    (17.0, 9.0, 6),     # 26.0
    (17.8, 9.3, -7),    # 27.1
    (19.0, 3.9, 2),     # 22.9
    (20.4, 6.1, -13),   # 26.5
    (21.3, 6.5, 9),     # 27.8
    (22.1, 13.0, -4),   # 35.1  (far: its rumble lands in the lock-in, the smear thinning out)
    (23.2, 6.6, -16),   # 29.8
    (25.1, 12.0, 3),    # 37.1  (far)
    (25.9, 4.8, -11),   # 30.7
    (26.8, 4.5, 7),     # 31.3
    (28.5, 14.8, -2),   # 43.3  (far)
]
STRIKES = [(tf, d, az, 'pre') for tf, d, az in PRE]
STRIKES += [(0.0, 0.0, 0, 'rod')]
STRIKES += [(tf, d, az, 'chaos') for tf, d, az in CHAOS]
STRIKES += [(5.0, 15.0, -12, 'count'), (24.0, 5.0, -10, 'count'), (30.0, 3.0, -6, 'count')]
STRIKES += [(32.0, 2.0, None, 'lock'), (34.0, 2.0, None, 'lock'), (36.0, 2.0, None, 'lock'),
            (38.0, 1.0, None, 'lock'), (40.0, 1.0, None, 'lock'), (42.0, 0.5, None, 'lock'),
            (44.0, 0.25, None, 'lock'), (46.0, 0.25, None, 'lock'), (48.0, 0.125, None, 'lock')]
STRIKES += [(50.0, 0.0, 0, 'rod')]
STRIKES += [(54.0, 3.0, 6, 'cell2'), (56.0, 2.0, 9, 'cell2'), (58.0, 1.0, 12, 'cell2')]
STRIKES.sort(key=lambda s: s[0])
N_STRIKES = len(STRIKES)
KIND = [s[3] for s in STRIKES]
I_ROD0 = next(i for i, s in enumerate(STRIKES) if s[3] == 'rod' and s[0] == 0.0)
I_CLIMAX = next(i for i, s in enumerate(STRIKES) if s[3] == 'rod' and s[0] == T_CLIMAX)
COUNTED = [i for i, s in enumerate(STRIKES) if s[3] == 'count'] + \
          [i for i, s in enumerate(STRIKES) if s[3] == 'cell2'][:1]          # 54 s strike is counted too
I_TAG = COUNTED[0]
LOCK = [i for i, s in enumerate(STRIKES) if s[3] == 'lock'] + [I_CLIMAX]
CELL2 = [i for i, s in enumerate(STRIKES) if s[3] == 'cell2']
SEEN = [i for i in range(N_STRIKES) if KIND[i] != 'pre']                     # flashes on screen


def thunder_t(i):
    tf, d = STRIKES[i][:2]
    return tf + d


def true_km(i):
    return STRIKES[i][1] * C_SOUND


def fmt_num(x):
    """1 decimal unless it is whole: 2 -> '2', 1.5 -> '1.5', 0.4 -> '0.4'"""
    if abs(x - round(x)) < 1e-6:
        return str(int(round(x)))
    return f'{x:.1f}'


def fmt_sec(s):
    if s >= 1:
        return fmt_num(s)
    return {0.5: '0.5', 0.25: '0.25', 0.125: '0.125', 0.0: '0'}.get(s, f'{s:.2f}')


def rule_mi(s):
    return s / 5.0


def rule_km(s):
    return s / 3.0


def mi_text(s):
    v = rule_mi(s)
    if v >= 1:
        return f'{fmt_num(v)} MI'
    if v >= 0.095:
        return f'{v:.1f} MI'
    if v > 0:
        return f'{v:.2f} MI' if v >= 0.0495 else f'{v:.3f} MI'
    return '0 MI'


def km_text(s):
    v = rule_km(s)
    if v >= 0.95:
        return f'{fmt_num(round(v, 1))} KM'
    if v >= 0.5:
        return f'{v:.1f} KM'
    if v > 0:
        return f'{int(round(v * 100) * 10)} M' if v >= 0.1 else f'{int(round(v * 1000 / 10) * 10)} M'
    return '0 M'


# ---------------------------------------------------------------- captions (<= 18 chars per line)
# '≈' is a runtime glyph added in render.py
CAPTIONS = [
    (0.0, 2.95, ["SECONDS *÷ 5* = MI", "SECONDS ~÷ 3~ = KM"]),
    (3.0, 4.95, ["HOW *FAR*", "~WAS THAT?~"]),
    (5.0, 8.95, ["COUNT IT:", "*1... 2... 3...*"]),
    (9.0, 14.95, ["LIGHT: *INSTANT*", "SOUND: ~1 KM / 3 S~"]),
    (15.0, 19.95, ["FAR = *RUMBLE*", "AIR EATS ~HIGHS~"]),
    (20.0, 23.95, ["*15 S* = 3 MI", "~≈ 5 KM~"]),
    (24.0, 29.95, ["IT'S *COMING*"]),
    (30.0, 35.95, ["HEAR *THUNDER*?", "~GO INDOORS~"]),
    (36.0, 38.95, ["*0.4 MI*...", "~≈ 0.7 KM~"]),
    (39.0, 42.45, ["*0.2 MI*...", "~≈ 330 M~"]),
    (42.5, 44.2, ["*0.1 MI*...", "~≈ 170 M~"]),
    (44.25, 48.05, ["*≈ 80 M*...", "~0.05 MI~"]),
    (48.125, 49.95, ["*≈ 40 M*...", "~0.025 MI~"]),
    (50.0, 51.95, ["*0 S*", "~RIGHT ON TOP~"]),
    (52.0, 56.95, ["TRY IT: *COUNT*", "~FROM INSIDE~"]),
    (57.0, 59.9, ["THE NEXT ONE'S", "~CLOSER~"]),
]

# HUD windows (slide-in / slide-out)
HUD_COUNT = (5.0, 33.6)       # stopwatch panel: the counted strikes (15 s, 5 s, 3 s)
HUD_GRID = (33.8, 52.6)       # bar panel: flash on the downbeat, thunder marker walks back
HUD_NEXT = (54.0, 59.45)      # second cell: 3 S / 2 S / 1 S (slides out before the loop point)


# ---------------------------------------------------------------- helpers
def ss(a, b, x):
    x = min(1.0, max(0.0, (x - a) / (b - a)))
    return x * x * (3 - 2 * x)


def ease(x):
    x = min(1.0, max(0.0, x))
    return x * x * x * (x * (6 * x - 15) + 10)


def rew_u(t):
    """0..1 progress through the rewind (3-5 s)"""
    return min(1.0, max(0.0, (t - T_CO) / (T_REW - T_CO)))


def rewinding(t):
    return T_CO <= t < T_REW


def phase(t):
    """clock for periodic motion (rain fall, smoke, rocking chair, cat tail): runs forward, then 10 s
    backwards during the 2 s rewind; total over the video = 48 s (a multiple of every period: 3, 4, 6 s)"""
    if t < T_CO:
        return t
    if t < T_REW:
        u = rew_u(t)
        # speed: 1 -> -9 -> ... integrates to -10 over the rewind (smooth start, hard stop at 5 s)
        return T_CO - 10.0 * (u * u * (3 - 2 * u))
    return t - 12.0


def flash_env(t, i):
    """0..1 flash brightness for strike i: one stroke, peaks ON the strike frame, smooth decay
    (one luminance pulse per strike, no restrike flicker)"""
    if KIND[i] == 'pre':
        return 0.0
    tf = STRIKES[i][0]
    u = t - tf
    if u < -1e-6 or u > 1.2:
        return 0.0
    tau = 0.24 if KIND[i] == 'rod' else 0.13
    return math.exp(-max(u, 0.0) / tau)


def flash_gain(i):
    """far strikes are dimmer; the overhead strike is the brightest"""
    km = true_km(i)
    return 0.4 + 0.6 * math.exp(-km / 0.9)


def last_strike(t, among=None):
    k = None
    for i in (among if among is not None else SEEN):
        if STRIKES[i][0] <= t:
            k = i
    return k


def counting(t):
    """count state for the HUD: (strike index, seconds since flash, arrived?)"""
    k = None
    for i in COUNTED:
        if STRIKES[i][0] <= t:
            k = i
    if k is None:
        return None
    tf, d = STRIKES[k][:2]
    return k, t - tf, t >= tf + d


def count_ticks():
    """(time, count number) for every whole second between each counted flash and its thunder"""
    out = []
    for i in COUNTED:
        tf, d = STRIKES[i][:2]
        n = 1
        while n < d - 1e-6:
            out.append((tf + n, n))
            n += 1
    return out


def storm(t):
    """0 = calm/distant, 1 = storm overhead"""
    if t < T_CO:
        return 1.0
    if t < T_REW:
        return 1.0 + (storm(T_REW) - 1.0) * ease(rew_u(t))
    if t < T_CLIMAX:
        return 0.1 + 0.9 * ss(T_REW, T_CLIMAX, t) ** 1.25
    return 1.0 - 0.22 * ss(51.5, 53.5, t) * (1 - ss(55.0, 58.5, t))


def rain(t):
    """rain amount 0..1: downpour in the cold open, light at the horizon, heavy from ~30 s to the loop"""
    if t < T_CO:
        return 1.0
    if t < T_REW:
        return 1.0 + (rain(T_REW) - 1.0) * ease(rew_u(t))
    return min(1.0, 0.2 + 0.22 * ss(12, 24, t) + 0.58 * ss(24, 33, t))


def wind(t):
    """windmill speed factor 0..1"""
    if t < T_CO:
        return 1.0
    if t < T_REW:
        return 1.0 + (wind(T_REW) - 1.0) * ease(rew_u(t))
    return 0.15 + 0.85 * ss(5, 44, t)


def moon_k(t):
    """moonlight: hidden by the storm overhead, back during the rewind, swallowed again around 30 s"""
    if t < T_CO:
        return 0.0
    if t < T_REW:
        return ease(rew_u(t))
    return 1 - ss(27.5, 31.5, t)


def ring_k(t):
    """sound rings only exist in the 'replayed' world after the rewind (fade in as it ends)"""
    return ss(4.55, 5.0, t)


def bed_cutoff(t):
    """low-pass on the music bed (Hz): open in the cold open, muffled at the horizon, opening as the storm
    nears, fully open from 32 s. (The rewind is processed separately.)"""
    if t < T_REW:
        return 20000.0
    pts = [(5.0, 320.0), (12.0, 420.0), (18.0, 560.0), (24.0, 820.0), (27.0, 1300.0), (28.0, 2000.0),
           (30.0, 4200.0), (32.0, 20000.0)]
    if t >= pts[-1][0]:
        return 20000.0
    for (a, fa), (b, fb) in zip(pts, pts[1:]):
        if t < b:
            u = (t - a) / (b - a)
            return math.exp(math.log(fa) + (math.log(fb) - math.log(fa)) * u)
    return 20000.0


def bars():
    """bar starts (s) and lengths: 0, 2 (cold open) | rewind | 5, 7, ... 25 | 2/4 fill at 27 | 28 ... 58"""
    out = [(0.0, BAR), (2.0, 1.0)]
    t = T_REW
    while t < T_FILL - 1e-6:
        out.append((t, BAR))
        t += BAR
    out.append((T_FILL, BAR / 2))
    t = T_FILL + BAR / 2
    while t < DUR - 1e-6:
        out.append((t, BAR))
        t += BAR
    return out


# ---------------------------------------------------------------- cat
# cold open: on the sill, bolts at the strike -> rug; the rewind plays that backwards (back on the sill
# at 5 s); leaves the sill again when the moon is swallowed; returns before the loop point.
CO_JUMP = (0.1, 0.5)
CO_RUN = (0.5, 1.4)
CAT_JUMP_DOWN = (30.6, 31.5)
CAT_WALK = (31.5, 33.2)
CAT_WALK_BACK = (53.4, 55.2)
CAT_JUMP_UP = (55.2, 56.1)


def cat_clock(t):
    """('co', time in the cold-open bolt) or ('main', t)"""
    if t < T_CO:
        return 'co', t
    if t < T_REW:
        return 'co', T_CO * (1 - ease(rew_u(t)))
    return 'main', t


def cat_flinch(t):
    """0..1 ear flinch / crouch right after each nearby-enough thunder (scaled by how loud it is)"""
    v = 0.0
    for i in range(N_STRIKES):
        u = t - thunder_t(i)
        if 0 <= u < 0.8 and STRIKES[i][0] <= t:
            if t < T_REW and KIND[i] not in ('rod',):
                continue
            g = min(1.0, math.exp(-true_km(i) / 2.2) * 1.4)
            v = max(v, g * math.exp(-u / 0.25))
    return v
