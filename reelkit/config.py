"""Shared frame constants. Change W/H/UP here only if a project really needs to."""
import numpy as np

W, H = 360, 640          # internal render size (portrait 9:16)
UP = 3                   # nearest-neighbour upscale -> 1080 x 1920
FPS = 30
# Default length for NEW videos: 61.0 s (user, Oct 23: TikTok monetisation needs at least 1 minute; the extra
# second is a safety net). Each project sets its own DUR / NF in timeline.py, and the tools (video, loopcheck,
# safezone) read the project's length from there, so videos designed at 60 s keep their 60 s.
DUR = 61.0
NF = int(DUR * FPS)

# Safe zone: the part of the frame the TikTok / Instagram apps leave visible on a normal (organic) post.
# Measured from the user's own TikTok + Instagram screenshots of a posted reel on a tall phone (Oct 23; the
# first version used the platforms' ad margins and was far too strict: the share bar ended up covering the
# battle). On a tall phone the apps crop ~50 px off each side and cover:
#   top     the tabs / search / camera icons, down to y ~250 (output px)
#   bottom  username, caption, creator avatar from y ~1500-1700
#   right   the like / comment / share column, x > ~900, but only from y ~900 down
# So: captions, labels, counters, HUD panels and key action stay inside SAFE and out of SAFE_BUTTONS;
# scenery can fill the whole frame. At 1080x1920 output: x 60-1020, y 288-1500, minus x > 900 below y 870.
# Check with `python -m reelkit.safezone`.
SAFE = (20, 96, 340, 500)                 # internal px: x0, y0, x1, y1 (the outer box)
SAFE_BUTTONS = (300, 290, W, H)           # internal px: the right-hand button column, covered (x >= 300, y >= 290)
SAFE_CX = W // 2                          # 180: captions sit above the button column, so centre them on the frame


def safe_x1(y0, y1):
    """right-hand limit for something spanning rows y0..y1: the safe box edge, or the button column's edge if it
    reaches down into the column"""
    return SAFE_BUTTONS[0] if y1 > SAFE_BUTTONS[1] else SAFE[2]


def safe_overshoot(x0, y0, x1, y1):
    """px by which a box sticks out of the safe zone (0 = fully inside): outside the outer box, or into the
    button column"""
    over = max(SAFE[0] - x0, SAFE[1] - y0, x1 - SAFE[2], y1 - SAFE[3], 0)
    bx0, by0, bx1, by1 = SAFE_BUTTONS
    if x1 > bx0 and y1 > by0 and x0 < bx1 and y0 < by1:
        over = max(over, min(x1 - bx0, y1 - by0))
    return over

BAYER = np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]) / 16.0
BAYER_FULL = np.tile(BAYER, (H // 4, W // 4))
YY, XX = np.mgrid[0:H, 0:W].astype(np.float32)


def whole_period(period, dur=DUR):
    """nearest period that completes a whole number of cycles in the video, so anything moving with it
    (drift, twinkle, pennants, searchlights, orbits) is back where it started at the loop point.
    Use it for every periodic motion: sin(2*pi*t / whole_period(7.3))."""
    n = max(1, round(dur / period))
    return dur / n


def loop_bpm(bpm, dur=DUR, beats_per_bar=4):
    """nearest tempo at which the video holds a whole number of bars, so the groove (and anything pulsing on the
    beat) is exactly in phase at the loop point. 61 s: 96 -> 94.43 (24 bars), 120 -> 118.03 (30 bars)."""
    bars = max(1, round(dur * bpm / 60.0 / beats_per_bar))
    return bars * beats_per_bar * 60.0 / dur


def ss(a, b, x):
    """smoothstep from a to b (scalar)"""
    x = min(1.0, max(0.0, (x - a) / (b - a)))
    return x * x * (3 - 2 * x)


def ease(x):
    """quintic ease-in-out on [0, 1]"""
    x = min(1.0, max(0.0, x))
    return x * x * x * (x * (6 * x - 15) + 10)


def lerp(a, b, u):
    return a + (b - a) * u


def mixc(a, b, u):
    return np.array(a, float) * (1 - u) + np.array(b, float) * u


def nrm(v):
    v = np.asarray(v, float)
    return v / (np.linalg.norm(v) + 1e-12)


def dirv(az, el):
    """unit vector for azimuth/elevation in degrees; az 0 looks down -z, positive az to +x"""
    import math
    a, e = math.radians(az), math.radians(el)
    return np.array([math.sin(a) * math.cos(e), math.sin(e), -math.cos(a) * math.cos(e)])
