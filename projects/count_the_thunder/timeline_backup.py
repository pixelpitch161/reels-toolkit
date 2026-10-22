"""Count the Thunder (IDEAS #13): ONE timeline for picture and sound.

The science: light arrives essentially instantly, sound travels ~0.343 km/s. Count from the flash to
the thunder: seconds / 5 = miles (NWS rule), seconds / 3 = km (Environment Canada rule). Both rules
are approximations (343 m/s is ~4.7 s/mile, ~2.9 s/km, and it varies with temperature).

How the science drives the music:
  first half (real time)  the count is the click track: one pitched tick per second after each flash,
                          and each thunder lands exactly where the physics says (delay = distance / c).
  back half (120 BPM)     one flash per bar on the downbeat (bright snare). The thunder is the kick,
                          late by the real flash-to-bang time: 2 s (a whole bar), 1 s, 1/2, 1/4, 1/8,
                          walking back across the bar until it fuses with its flash at the overhead
                          strike (50 s). The thunder's low-pass opens as the storm nears (rumble -> clap).

Every strike in STRIKES drives: the flash on screen, the bolt, the expanding sound ring on the
prairie, the window shadows, the HUD, the captions, the thunder in the mix and its filter."""
import math

FPS = 30
DUR = 60.0
NF = int(FPS * DUR)

C_SOUND = 0.343            # km/s (20 C); only used for the "true" distance, captions use the rules

BPM = 120
BEAT = 60.0 / BPM          # 0.5 s
BAR = 4 * BEAT             # 2.0 s; downbeats on even seconds from 30 s
T_GROOVE_IN = 30.0         # groove fades in under the rain
T_GROOVE = 35.0            # groove at full level
T_CLIMAX = 50.0            # overhead strike: flash + clap on the same downbeat
T_GROOVE_OUT = 52.0        # storm passes, groove thins
T_CALM = 57.5              # back to the calm opening state (loop)

# (flash time, flash-to-bang seconds). First half: real-time counts. Back half: one per bar.
STRIKES = [
    (0.5, 10.0),
    (12.0, 7.5),
    (21.0, 5.0),
    (27.0, 3.0),
    (34.0, 2.0),
    (36.0, 2.0),
    (38.0, 1.0),
    (40.0, 1.0),
    (42.0, 0.5),
    (44.0, 0.25),
    (46.0, 0.25),
    (48.0, 0.125),
    (50.0, 0.0),
]
N_STRIKES = len(STRIKES)
FIRST_HALF = [i for i, (tf, d) in enumerate(STRIKES) if tf < 30]
BACK_HALF = [i for i, (tf, d) in enumerate(STRIKES) if tf >= 30]
T_RUMBLE_END = 56.4        # one distant rumble as the storm leaves (its flash is behind us)


def thunder_t(i):
    tf, d = STRIKES[i]
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
# '≈' is a runtime glyph added in render.py (like the lowest-note project)
CAPTIONS = [
    (0.0, 3.9, ["SECONDS *÷ 5* = MI", "SECONDS ~÷ 3~ = KM"]),
    (4.0, 10.4, ["LIGHT: *INSTANT*", "SOUND: ~1 KM / 3 S~"]),
    (10.5, 12.4, ["*10 S* = 2 MI", "~≈ 3.3 KM~"]),
    (12.5, 19.4, ["FAR = *RUMBLE*", "AIR EATS ~HIGHS~"]),
    (19.5, 20.9, ["*7.5 S* = 1.5 MI", "~≈ 2.5 KM~"]),
    (21.0, 29.9, ["*5 S* = 1 MI", "~3 S~ = 1 KM"]),
    (30.0, 35.9, ["HEAR *THUNDER*?", "~GO INDOORS~"]),
    (36.0, 38.9, ["*0.4 MI*...", "~≈ 0.7 KM~"]),
    (39.0, 42.4, ["*0.2 MI*...", "~≈ 330 M~"]),
    (42.5, 44.2, ["*0.1 MI*...", "~≈ 170 M~"]),
    (44.25, 48.05, ["*≈ 80 M*...", "~0.05 MI~"]),
    (48.125, 49.95, ["*≈ 40 M*...", "~0.025 MI~"]),
    (50.0, 51.9, ["*0 S*", "~RIGHT ON TOP~"]),
    (52.0, 59.4, ["TRY IT: *COUNT*", "~FROM INSIDE~"]),
]

# HUD windows (slide-in / slide-out)
HUD_COUNT = (0.55, 33.0)      # stopwatch panel: running count, then the two rules
HUD_GRID = (33.6, 52.6)       # bar panel: flash on the downbeat, thunder marker walks back


# ---------------------------------------------------------------- helpers
def ss(a, b, x):
    x = min(1.0, max(0.0, (x - a) / (b - a)))
    return x * x * (3 - 2 * x)


def flash_env(t, i):
    """0..1 flash brightness for strike i: one stroke, 1-frame rise, smooth decay (no flicker:
    a single luminance pulse per strike keeps us far under 3 flashes/s)"""
    tf = STRIKES[i][0]
    u = t - tf
    if u < -1e-6 or u > 1.2:
        return 0.0
    tau = 0.24 if i == N_STRIKES - 1 else 0.13      # the overhead strike glows a little longer
    return math.exp(-max(u, 0.0) / tau)             # peaks ON the strike frame (in sync with the snare)


def flash_gain(i):
    """far strikes are dimmer; the overhead strike is the brightest"""
    km = true_km(i)
    return 0.42 + 0.58 * math.exp(-km / 0.9)


def last_strike(t):
    """index of the most recent flash at or before t (or None)"""
    k = None
    for i, (tf, d) in enumerate(STRIKES):
        if tf <= t:
            k = i
    return k


def counting(t):
    """first-half count state: (strike index, seconds since flash, arrived?)"""
    k = None
    for i in FIRST_HALF:
        if STRIKES[i][0] <= t:
            k = i
    if k is None:
        return None
    tf, d = STRIKES[k]
    return k, t - tf, t >= tf + d


def count_ticks():
    """(time, count number) for every whole second between each first-half flash and its thunder"""
    out = []
    for i in FIRST_HALF:
        tf, d = STRIKES[i]
        n = 1
        while n < d - 1e-6:
            out.append((tf + n, n))
            n += 1
    return out


def storm(t):
    """0 = calm/distant, 1 = storm overhead"""
    if t < 50:
        return ss(0, 50, t) ** 1.2
    return 1 - ss(51, T_CALM, t)


def rain(t):
    """rain amount 0..1 (light from 4 s, heavy in the groove, gone by the loop)"""
    up = 0.18 * ss(3.5, 7.0, t) + 0.25 * ss(14, 26, t) + 0.57 * ss(26, 34, t)
    return up * (1 - ss(52.0, 59.0, t))


def wind(t):
    """windmill speed factor 0..1"""
    return ss(4, 46, t) * (1 - ss(51, 58.5, t))


def moon_k(t):
    """moonlight: swallowed by the storm around 30 s, back after it passes"""
    return 1 - ss(27.5, 31.5, t) + ss(53.5, 58.0, t)


def groove_level(t):
    return ss(T_GROOVE_IN, T_GROOVE, t) * (1 - ss(T_GROOVE_OUT + 0.4, 56.0, t))


def downbeats():
    t = T_GROOVE_IN
    out = []
    while t < 56.0 - 1e-6:
        out.append(t)
        t += BAR
    return out


# ---------------------------------------------------------------- cat (on the sill -> rug -> sill)
CAT_JUMP_DOWN = (30.6, 31.5)   # leaves the sill when the moon is swallowed and the 3 s clap lands
CAT_WALK = (31.5, 33.2)
CAT_WALK_BACK = (53.8, 55.6)
CAT_JUMP_UP = (55.6, 56.5)


def cat_flinch(t):
    """0..1 ear flinch / crouch right after each thunder"""
    v = 0.0
    for i in range(N_STRIKES):
        u = t - thunder_t(i)
        if 0 <= u < 0.8:
            v = max(v, math.exp(-u / 0.25))
    return v
