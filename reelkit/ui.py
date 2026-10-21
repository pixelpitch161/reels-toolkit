"""On-screen text and panels.

Captions: list of (t0, t1, [lines]). In a line, *word* = highlight colour, ~word~ = alt colour.
They type on (55 chars/s), slide in, and fade out. Keep lines <= 18 chars for the big size."""
import numpy as np
from .config import W, ease, ss
from .font import text_mask, stamp, rich_line

CREAM = (252, 240, 214)
AMBER = (255, 198, 92)
SKYB = (140, 200, 255)
MINT = (124, 246, 200)
CORAL = (255, 118, 96)
PINK = (255, 120, 200)
CYAN = (110, 240, 255)
INK = (12, 16, 30)


def draw_captions(img, t, caps, y0=42, base=CREAM, hi=AMBER, alt=SKYB, cps=55):
    for t0, t1, lines in caps:
        if t0 <= t < t1:
            u = t - t0
            nreveal = int(u * cps) + 1
            slide = int(round(6 * (1 - ease(u / 0.25))))
            fade = 1 - ss(t1 - 0.12, t1, t)
            y = y0 + slide
            used = 0
            lens = [len(l.replace('*', '').replace('~', '')) for l in lines]
            scale = 3 if max(lens) <= 18 else 2
            for ln, pl in zip(lines, lens):
                rich_line(img, ln, W // 2, y, scale, base, hi, alt, nchars=nreveal - used, alpha=fade)
                used += pl
                y += 7 * scale + 7


def panel(draw, x0, y0, x1, y1, fill=(16, 20, 38), edge=(62, 74, 112)):
    """HUD box with a drop shadow (draw on the PIL image after post, before text)"""
    draw.rectangle([x0 + 2, y0 + 2, x1 + 2, y1 + 2], fill=(6, 8, 18))
    draw.rectangle([x0, y0, x1, y1], fill=fill)
    draw.rectangle([x0, y0, x1, y1], outline=edge)
    draw.line([x0 + 1, y0 + 1, x1 - 1, y0 + 1], fill=(40, 50, 84))


def slide_offset(t, t_in, t_out, dist=160, dur=0.35):
    """panel slide-in/out offset in px (0 = fully on screen)"""
    a = min(ease((t - t_in) / dur), 1 - ease((t - t_out) / dur))
    return int(round((1 - a) * dist))


def label(img, text, x, y, col, scale=2, outline=(10, 20, 26)):
    m = text_mask(text, scale)
    stamp(img, m, int(x), int(y), col, outline=outline)
    return m.shape[1]
