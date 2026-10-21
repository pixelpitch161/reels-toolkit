"""Depth of field with an octagonal (8-blade) aperture.

Use a LOCKED focus on the subject for the whole video (viewers want the subject sharp; a rack
focus away from it read as a mistake in testing). `sharp` keeps a band around the focus plane
fully crisp, so the whole subject (e.g. cricket + thermometer, plate + sand) never softens.

    zf = subject camera depth
    K  = aperture_k * cam.f * min(1.2, ref_dist / zf)    # less blur when the subject is far away
    img = dof(img, coc_map(depth, zf, K, sharp))"""
import math
import numpy as np
from scipy.signal import fftconvolve

LEVELS = [0.0, 1.0, 2.0, 3.4, 5.2, 7.6, 10.5]


def oct_kernel(r):
    n = int(math.ceil(r))
    yy, xx = np.mgrid[-n:n + 1, -n:n + 1].astype(float)
    ang = np.arctan2(yy, xx)
    rad = r * math.cos(math.pi / 8) / np.cos(((ang + math.pi / 8) % (math.pi / 4)) - math.pi / 8)
    k = (np.hypot(xx, yy) <= rad + 0.35).astype(float)
    return k / k.sum()


_KERNELS = {}


def kernel(r):
    if r not in _KERNELS:
        _KERNELS[r] = oct_kernel(r)
    return _KERNELS[r]


def coc_map(depth, zf, K, sharp=0.05, levels=LEVELS):
    """circle-of-confusion radius (px) per pixel"""
    return np.clip(K * np.maximum(0.0, np.abs(1 - zf / np.maximum(depth, 1e-3)) - sharp), 0, levels[-1])


def dof(img, coc, levels=LEVELS):
    """masked, normalised blur stack: sharp foreground never bleeds into blurred background"""
    out = img.copy()
    prev = img
    for i in range(1, len(levels)):
        r, lo = levels[i], levels[i - 1]
        if not (coc > lo).any():
            break
        m = (coc >= r * 0.7).astype(np.float32)
        kern = kernel(r)
        den = fftconvolve(m, kern, mode='same')
        num = np.stack([fftconvolve(img[..., c] * m, kern, mode='same') for c in range(3)], -1)
        blur = np.where(den[..., None] > 1e-3, num / np.maximum(den[..., None], 1e-3), img)
        w = np.clip((coc - lo) / (r - lo), 0, 1)[..., None]
        out = np.where((coc > lo)[..., None], prev * (1 - w) + blur * w, out)
        prev = blur
    return out


def add_bokeh(img, x, y, r, col, inten, coc=None, depth=None, zb=None):
    """octagonal bokeh disc for an out-of-focus light. With depth/zb/coc it hides behind sharp foreground."""
    H, W = img.shape[:2]
    n = int(math.ceil(r)) + 1
    x0, y0 = int(round(x)) - n, int(round(y)) - n
    yy, xx = np.mgrid[-n:n + 1, -n:n + 1].astype(float)
    ang = np.arctan2(yy, xx)
    rad = r * math.cos(math.pi / 8) / np.cos(((ang + math.pi / 8) % (math.pi / 4)) - math.pi / 8)
    d = np.hypot(xx, yy)
    disc = np.clip(rad + 0.6 - d, 0, 1)
    rim = disc * np.clip(1.6 - (rad - d), 0, 1)
    a = (disc * 0.62 + rim * 0.3) * inten * 1.5 / (1 + 0.025 * r * r)
    xa, ya = max(0, x0), max(0, y0)
    xb, yb = min(W, x0 + 2 * n + 1), min(H, y0 + 2 * n + 1)
    if xb <= xa or yb <= ya:
        return
    a = a[ya - y0:yb - y0, xa - x0:xb - x0]
    if depth is not None and zb is not None:
        vis = (depth[ya:yb, xa:xb] > zb * 0.93)
        if coc is not None:
            vis |= coc[ya:yb, xa:xb] > 0.6 * r
        a = a * vis
    c = np.array(col, float)[None, None]
    sub = img[ya:yb, xa:xb]
    img[ya:yb, xa:xb] = sub + (c - sub * 0.45) * np.clip(a, 0, 1)[..., None]
