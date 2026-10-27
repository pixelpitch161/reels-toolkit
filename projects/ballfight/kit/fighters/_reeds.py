"""Shared by the reed fighters (CLARINET, OBOE; from #37's styles.py and render.py, copied unchanged): screen masks,
ribbons along ring-frame paths, the dithered lit lane under a beam or a zip line, and the woodwind shatter parts
(sticks of the body, nickel key chips, bell chips, cane blades). Not a fighter itself."""
import math

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, BAYER_FULL
import scene as S

CANE = np.array([232, 210, 152], float)
CANE_D = np.array([170, 140, 84], float)
THREAD = np.array([60, 150, 86], float)
INK = np.array([18, 10, 26], float)
WHITE = np.array([255, 252, 240], float)
WOOD = np.array([34, 26, 30], float)
NICKEL = np.array([214, 216, 226], float)
LAV_SPARK = [(226, 214, 255), (190, 168, 255), (255, 250, 255)]
SOUR = [(170, 190, 70), (210, 220, 120), (120, 140, 40)]
KO_CHIPS = [(232, 210, 152), (214, 216, 226), (40, 32, 36)]


def _mask(polys, lines=(), width=1):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if len(pts) >= 3:
            d.polygon([tuple(map(float, p)) for p in pts], fill=255)
    for pts in lines:
        if len(pts) >= 2:
            d.line([tuple(map(float, p)) for p in pts], fill=255, width=width)
    return np.asarray(m, np.float32) / 255.0


def blend(img, mk, col, alpha):
    if alpha <= 0:
        return img
    img += (np.asarray(col, float)[None, None] - img) * (mk * alpha)[..., None]
    return img


def add(img, mk, col, k):
    img += mk[..., None] * np.asarray(col, float)[None, None] * k
    return img


def ribbon(cam, P, hw):
    """screen polygon of a ribbon along ring-frame points P (n, 3: x, z, y) with half-width hw (world units),
    widened across the path in the ring plane"""
    P = np.asarray(P, float)
    d = np.gradient(P[:, :2], axis=0)
    d /= np.linalg.norm(d, axis=1, keepdims=True) + 1e-9
    n = np.stack([-d[:, 1], d[:, 0]], 1)
    hw = np.broadcast_to(np.asarray(hw, float), (len(P),))
    L = P[:, :2] + n * hw[:, None]
    Rr = P[:, :2] - n * hw[:, None]
    a = cam.wp(S.to_world(L[:, 0], L[:, 1], P[:, 2]))
    b = cam.wp(S.to_world(Rr[:, 0], Rr[:, 1], P[:, 2]))
    return np.vstack([a, b[::-1]])


def ring_exit(o, d, A=None):
    """distance from o along d to the ropes (the ring is |x|, |z| <= A): beams stop there"""
    A = S.A if A is None else A
    out = 99.0
    for i in (0, 1):
        if abs(d[i]) > 1e-9:
            out = min(out, (math.copysign(A, d[i]) - o[i]) / d[i])
    return max(0.0, out)


def beam_lane(sxp, szp, o, aim, length, width, ys, xs):
    """(N,) light on the canvas under a live beam: a thin lit lane (dithered)"""
    d = np.array([math.cos(aim), math.sin(aim)])
    length = min(length, ring_exit(o, d))
    rx, rz = sxp - o[0], szp - o[1]
    along = rx * d[0] + rz * d[1]
    across = np.abs(-rx * d[1] + rz * d[0])
    lane = np.clip(1 - across / (0.16 + width), 0, 1) * (along > 0) * (along < length)
    return np.floor(lane * 3 + BAYER_FULL[ys, xs] * 0.9) / 3


def draw_woodwind_shard(P, arr, cam, kind, p, rot, size):
    """#37's woodwind shatter parts: 'wood' (a stick of the body), 'reed' (a cane blade), 'key' (a nickel chip), 'bell'
    (a black curved chip)"""
    sx_, sz_ = p[0], p[2]
    d = np.array([math.cos(rot), math.sin(rot)])
    nn = np.array([-d[1], d[0]])
    if kind in ('wood', 'reed'):
        ln_, wd_ = (0.26, 0.035) if kind == 'wood' else (0.12, 0.03)
        q4 = [np.array([sx_, sz_]) + d * ln_ * a1 + nn * wd_ * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
        P.layer_blend(arr, [sp], WOOD if kind == 'wood' else CANE, 1.0)
        return arr
    col = NICKEL if kind == 'key' else WOOD * 1.3
    sz = size * (0.7 if kind == 'key' else 1.5)
    tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
           np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
    sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
    shade_k = 0.75 + 0.25 * math.cos(rot * 2)
    P.layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
    return arr


def tail_theta(P, k, st, t, gl):
    """#37's glide home: the aim eases from where it ended to its opening aim (both fighters)"""
    th_end = float(P.RUN['TH'][-1, k])
    st['th'] = th_end + P._wrap(P.TH0[k] - th_end) * gl

