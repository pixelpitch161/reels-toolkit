"""On-screen text and panels.

Captions: list of (t0, t1, [lines]). In a line, *word* = highlight colour, ~word~ = alt colour.
They type on (55 chars/s), slide in, and fade out.

Safe zone (config.SAFE): captions start at the top of the safe box and are centred on it. Lines of up to
14 chars use the big size, up to 21 chars the small size; longer lines overflow the box (the safe-zone
check flags them). Keep HUD panels and labels inside the box too: never in the bottom 35 % of the frame
or under the right-hand button column.

Everything drawn here is recorded in DRAWN (reset each frame by the checker), so
`python -m reelkit.safezone render.py` can verify placement."""
import numpy as np
from .config import W, SAFE, SAFE_CX, ease, ss
from .font import ADV, text_mask, stamp, rich_line

CREAM = (252, 240, 214)
AMBER = (255, 198, 92)
SKYB = (140, 200, 255)
MINT = (124, 246, 200)
CORAL = (255, 118, 96)
PINK = (255, 120, 200)
CYAN = (110, 240, 255)
INK = (12, 16, 30)

CAP_Y0 = SAFE[1] + 4          # 100: first caption line
CAP_BIG_MAX = 14              # chars per line at the big size (scale 3) that fit the safe box
CAP_SMALL_MAX = 21            # chars per line at the small size (scale 2)

# (x0, y0, x1, y1, kind, info) for every caption line, label and panel drawn since the last reset
DRAWN = []


def reset_drawn():
    DRAWN.clear()


def _note(x0, y0, x1, y1, kind, info=''):
    DRAWN.append((int(x0), int(y0), int(x1), int(y1), kind, info))


def _plain(line):
    return line.replace('*', '').replace('~', '')


def caption_scale(lines):
    """3 (big) if every line fits the safe box at the big size, else 2 (small)"""
    return 3 if max(len(_plain(l)) for l in lines) <= CAP_BIG_MAX else 2


def draw_captions(img, t, caps, y0=None, cx=None, base=CREAM, hi=AMBER, alt=SKYB, cps=55):
    """y0 / cx default to the top centre of the safe box; override only with a reason"""
    y0 = CAP_Y0 if y0 is None else y0
    cx = SAFE_CX if cx is None else cx
    for t0, t1, lines in caps:
        if t0 <= t < t1:
            u = t - t0
            nreveal = int(u * cps) + 1
            slide = int(round(6 * (1 - ease(u / 0.25))))
            fade = 1 - ss(t1 - 0.12, t1, t)
            y = y0 + slide
            used = 0
            lens = [len(_plain(l)) for l in lines]
            scale = caption_scale(lines)
            for ln, pl in zip(lines, lens):
                rich_line(img, ln, cx, y, scale, base, hi, alt, nchars=nreveal - used, alpha=fade)
                width = pl * ADV * scale - scale
                x0 = int(round(cx - width / 2))
                _note(x0, y, x0 + width, y + 7 * scale, 'caption', _plain(ln))
                used += pl
                y += 7 * scale + 7


def panel(draw, x0, y0, x1, y1, fill=(16, 20, 38), edge=(62, 74, 112)):
    """HUD box with a drop shadow (draw on the PIL image after post, before text). Keep it in config.SAFE."""
    draw.rectangle([x0 + 2, y0 + 2, x1 + 2, y1 + 2], fill=(6, 8, 18))
    draw.rectangle([x0, y0, x1, y1], fill=fill)
    draw.rectangle([x0, y0, x1, y1], outline=edge)
    draw.line([x0 + 1, y0 + 1, x1 - 1, y0 + 1], fill=(40, 50, 84))
    _note(x0, y0, x1 + 3, y1 + 3, 'panel')


def slide_offset(t, t_in, t_out, dist=160, dur=0.35):
    """panel slide-in/out offset in px (0 = fully on screen)"""
    a = min(ease((t - t_in) / dur), 1 - ease((t - t_out) / dur))
    return int(round((1 - a) * dist))


def label(img, text, x, y, col, scale=2, outline=(10, 20, 26)):
    m = text_mask(text, scale)
    stamp(img, m, int(x), int(y), col, outline=outline)
    _note(int(x), int(y), int(x) + m.shape[1], int(y) + m.shape[0], 'label', text)
    return m.shape[1]
