"""Frame: a PIL colour canvas plus a float depth buffer that every draw call keeps in sync,
so lens blur / god rays / bokeh can use real depth. Painter's algorithm: sort, then draw."""
import math
import numpy as np
from PIL import Image, ImageDraw
from .config import W, H, BAYER
from .camera import clip_near

SKY_DEPTH = 1e6


class Frame:
    def __init__(self, bg):
        """bg: (H, W, 3) array (e.g. a sky gradient) or an RGB tuple"""
        if isinstance(bg, np.ndarray):
            self.pil = Image.fromarray(np.clip(bg, 0, 255).astype(np.uint8))
        else:
            self.pil = Image.new('RGB', (W, H), tuple(int(c) for c in bg))
        self.draw = ImageDraw.Draw(self.pil)
        self.dep = Image.new('F', (W, H), SKY_DEPTH)
        self.ddraw = ImageDraw.Draw(self.dep)
        self.glows = []          # (x, y, radius, colour, intensity) composited after the lens

    # -------------------------------------------------- drawing
    def poly(self, cam, P, col, depth=True):
        Pc = cam.to_cam(P)
        if (Pc[:, 2] < cam.near).any():
            if (Pc[:, 2] < cam.near).all():
                return None
            Pc = clip_near(Pc, cam.near)
            if Pc is None:
                return None
        pts = [tuple(p) for p in cam.proj(Pc)]
        self.draw.polygon(pts, fill=tuple(int(c) for c in np.clip(col, 0, 255)))
        if depth:
            self.ddraw.polygon(pts, fill=float(Pc[:, 2].mean()))
        return pts

    def screen_poly(self, pts, col, z=None):
        pts = [tuple(p) for p in pts]
        self.draw.polygon(pts, fill=tuple(int(c) for c in np.clip(col, 0, 255)))
        if z is not None:
            self.ddraw.polygon(pts, fill=float(z))

    def line(self, cam, pts, col, width=1):
        Pc = cam.to_cam(np.asarray(pts, float))
        if (Pc[:, 2] < cam.near).any():
            return
        s = [tuple(p) for p in cam.proj(Pc)]
        self.draw.line(s, fill=tuple(int(c) for c in col), width=int(width))
        self.ddraw.line(s, fill=float(Pc[:, 2].mean()), width=int(width))

    def disc(self, center, r, col, n=12, rot=0.0, z=None):
        pts = [(center[0] + r * math.cos(rot + 2 * math.pi * k / n), center[1] + r * math.sin(rot + 2 * math.pi * k / n)) for k in range(n)]
        self.screen_poly(pts, col, z)

    def glow(self, x, y, r, col, inten):
        self.glows.append((x, y, r, col, inten))

    # -------------------------------------------------- access
    def array(self):
        return np.asarray(self.pil).astype(np.float32)

    def depth(self):
        return np.asarray(self.dep).copy()

    def set_array(self, arr, depth=None):
        """replace pixels (e.g. after a per-pixel lighting pass) and keep drawing on top"""
        self.pil = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
        self.draw = ImageDraw.Draw(self.pil)
        if depth is not None:
            self.dep = Image.fromarray(depth.astype(np.float32), 'F')
            self.ddraw = ImageDraw.Draw(self.dep)


def painter(items):
    """items: (depth, ...) tuples; returns far-to-near order"""
    return sorted(items, key=lambda it: -it[0])


# ------------------------------------------------------------------ glow sprites
def _glow_kernel(r):
    yy, xx = np.mgrid[-r:r + 1, -r:r + 1]
    d = np.hypot(xx, yy) / (r + 0.5)
    v = np.clip(1 - d, 0, 1) ** 1.7
    b = np.tile(BAYER, ((2 * r + 1) // 4 + 1, (2 * r + 1) // 4 + 1))[:2 * r + 1, :2 * r + 1]
    return np.floor(v * 5 + b * 0.999) / 5          # banded, dithered: reads as pixel art


GLOW = {r: _glow_kernel(r) for r in range(1, 61)}


def add_glow(img, x, y, r, col, inten):
    r = int(max(1, min(60, round(r))))
    k = GLOW[r] * inten
    x, y = int(round(x)), int(round(y))
    x0, y0, x1, y1 = x - r, y - r, x + r + 1, y + r + 1
    if x1 <= 0 or y1 <= 0 or x0 >= W or y0 >= H:
        return
    kx0, ky0 = max(0, -x0), max(0, -y0)
    kx1, ky1 = k.shape[1] - max(0, x1 - W), k.shape[0] - max(0, y1 - H)
    img[max(0, y0):min(H, y1), max(0, x0):min(W, x1)] += k[ky0:ky1, kx0:kx1, None] * np.array(col, float)[None, None]


def apply_glows(img, glows):
    for g in glows:
        add_glow(img, *g)
    return img
