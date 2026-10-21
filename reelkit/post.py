"""Post effects on the float (H, W, 3) frame, in this order:
    lens.dof -> raster.apply_glows -> god_rays -> bloom -> grade -> vignette * top_scrim -> UI"""
import numpy as np
from PIL import Image, ImageFilter
from .config import W, H, XX, YY, BAYER_FULL

_GR_Y, _GR_X = np.mgrid[0:H // 2, 0:W // 2].astype(np.float32)


def god_rays(img, depth, src_xy, col, k, sky_depth=5e5, r1=45.0, r2=140.0, samples=40, decay=0.962,
             keep_dark=None):
    """screen-space volumetric light shafts from a light at src_xy (may be off-screen).
    Open sky near the light is smeared toward the viewer; every silhouette cuts a shadow shaft.
    keep_dark: optional (depth_lo, depth_hi, amount) to keep distant silhouettes dark."""
    mx, my = src_xy[0] / 2, src_xy[1] / 2
    d2 = depth[::2, ::2]
    open_sky = (d2 > sky_depth).astype(np.float32)
    d = np.hypot(_GR_X - mx, _GR_Y - my)
    src = open_sky * (np.exp(-d / r1) + 0.3 * np.exp(-d / r2))
    acc = np.zeros_like(src)
    tot, w = 0.0, 1.0
    for j in range(samples):
        sc = 0.92 * j / samples
        sx = np.clip((_GR_X + (mx - _GR_X) * sc).astype(np.int32), 0, W // 2 - 1)
        sy = np.clip((_GR_Y + (my - _GR_Y) * sc).astype(np.int32), 0, H // 2 - 1)
        acc += src[sy, sx] * w
        tot += w
        w *= decay
    acc = np.clip(acc / tot - 0.5 * src, 0, None)
    if keep_dark is not None:
        lo, hi, amt = keep_dark
        acc *= 1 - amt * ((d2 > lo) & (d2 <= hi))
    up = np.asarray(Image.fromarray(acc).resize((W, H), Image.BILINEAR))
    return img + up[..., None] * np.array(col, np.float32)[None, None] * k


def bloom(img, thr=200.0, k=0.4, exclude=None):
    """glow around bright pixels; exclude: bool mask of pixels that must stay crisp (e.g. sand grains)"""
    b = np.clip(img.max(-1, keepdims=True) - thr, 0, None) / (255 - thr)
    if exclude is not None:
        b = b * (~exclude)[..., None]
    src = img * b
    small = Image.fromarray(np.clip(src, 0, 255).astype(np.uint8)).resize((W // 4, H // 4), Image.BILINEAR)
    small = small.filter(ImageFilter.GaussianBlur(3.0))
    up = np.asarray(small.resize((W, H), Image.BILINEAR)).astype(np.float32)
    return img + up * k


def grade(img, contrast=0.18, sat=1.1, black=0.0, gamma=1.0):
    """gentle S-curve + saturation. Keep black/gamma near 0/1: very dark grades read as 'too moody'."""
    x = np.clip(img / 255.0, 0, 1)
    if black:
        x = np.clip((x - black) / (1 - black), 0, 1)
    if gamma != 1.0:
        x = x ** gamma
    x = x + contrast * (x * x * (3 - 2 * x) - x)
    g = x.mean(-1, keepdims=True)
    x = g + (x - g) * sat
    return np.clip(x, 0, 1) * 255.0


def posterize(img, step=5.0):
    """ordered-dither quantisation: the retro banding that ties the look together"""
    return np.floor(img / step + BAYER_FULL[..., None] * 0.999) * step


_VIG = {}


def vignette(strength=0.9, start=0.55, cap=0.5, cy=0.47):
    key = (strength, start, cap, cy)
    if key not in _VIG:
        r = np.hypot((XX - W / 2) / (W * 0.62), (YY - H * cy) / (H * 0.62))
        v = np.clip(r - start, 0, 1) * strength
        v = np.floor(v * 5 + BAYER_FULL * 0.999) / 5
        _VIG[key] = (1 - np.clip(v, 0, cap))[..., None]
    return _VIG[key]


_SCRIM = {}


def top_scrim(amount=0.58, top=40, span=150):
    """darken the top of frame so captions always read"""
    key = (amount, top, span)
    if key not in _SCRIM:
        v = np.clip(1 - (YY - top) / span, 0, 1) * amount
        v = np.floor(v * 6 + BAYER_FULL * 0.999) / 6
        _SCRIM[key] = (1 - v)[..., None]
    return _SCRIM[key]
