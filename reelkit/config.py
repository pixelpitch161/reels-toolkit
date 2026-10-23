"""Shared frame constants. Change W/H/UP here only if a project really needs to."""
import numpy as np

W, H = 360, 640          # internal render size (portrait 9:16)
UP = 3                   # nearest-neighbour upscale -> 1080 x 1920
FPS = 30
DUR = 60.0
NF = int(DUR * FPS)

# Safe zone: the part of the frame TikTok, Instagram Reels and YouTube Shorts leave visible (strictest edge
# of the three, from their published margins). Captions, labels, HUD panels and key action stay inside it;
# scenery can fill the whole frame. At 1080x1920 output: x 120-888, y 288-1248 (top 15%, bottom 35%,
# left 120 px, right 192 px for the right-hand button column). Check with `python -m reelkit.safezone`.
SAFE = (40, 96, 296, 416)                 # internal px: x0, y0, x1, y1
SAFE_CX = (SAFE[0] + SAFE[2]) // 2        # 168: centre text on the safe box, not the frame (right margin is wider)

BAYER = np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]) / 16.0
BAYER_FULL = np.tile(BAYER, (H // 4, W // 4))
YY, XX = np.mgrid[0:H, 0:W].astype(np.float32)


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
