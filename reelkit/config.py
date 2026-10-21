"""Shared frame constants. Change W/H/UP here only if a project really needs to."""
import numpy as np

W, H = 360, 640          # internal render size (portrait 9:16)
UP = 3                   # nearest-neighbour upscale -> 1080 x 1920
FPS = 30
DUR = 60.0
NF = int(DUR * FPS)

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
