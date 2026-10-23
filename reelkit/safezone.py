"""Safe-zone check: are captions, labels and HUD panels inside the part of the frame that TikTok,
Instagram Reels and YouTube Shorts leave visible (config.SAFE)?

    python -m reelkit.safezone render.py                          # a frame every 0.5 s
    python -m reelkit.safezone render.py --step 0.25 --sheet check/safezone.png

It renders frames, reads what reelkit.ui drew (draw_captions, label, panel) and reports anything outside
the safe box, plus captions overlapping a panel or label. Panels and labels partly off the frame are
treated as sliding in/out and skipped (captions never are: a line too long for the frame is flagged). Text or panels drawn by hand (PIL draw.text, custom stamps) are not tracked:
draw them through reelkit.ui, or check them by eye on contact sheets (which show the safe box).
Exit code 1 if anything is outside."""
import argparse
import os
import numpy as np
from PIL import Image
from .config import W, H, FPS, DUR, SAFE
from . import ui
from .video import _load


def overlay(arr):
    """copy of an (H, W, 3) frame with the area outside the safe box tinted red and the box outlined"""
    img = arr.astype(np.float32).copy()
    x0, y0, x1, y1 = SAFE
    out = np.ones((H, W), bool)
    out[y0:y1, x0:x1] = False
    img[out] = img[out] * 0.62 + np.array([255, 40, 40]) * 0.38
    img = img.astype(np.uint8)
    dash = (np.arange(max(W, H)) // 3) % 2 == 0
    col = np.array([70, 255, 130], np.uint8)
    for x in range(x0, x1):
        if dash[x]:
            img[y0, x] = col
            img[y1 - 1, x] = col
    for y in range(y0, y1):
        if dash[y]:
            img[y, x0] = col
            img[y, x1 - 1] = col
    return img


def check(mod, times):
    outside, overlaps, tracked = {}, {}, 0
    for t in times:
        ui.reset_drawn()
        mod.render(int(round(t * FPS)))
        boxes = list(ui.DRAWN)
        tracked += len(boxes)
        for x0, y0, x1, y1, kind, info in boxes:
            if kind != 'caption' and (x0 < 0 or y0 < 0 or x1 > W or y1 > H):
                continue                                    # panel/label partly off-frame: sliding in or out
            over = max(SAFE[0] - x0, SAFE[1] - y0, x1 - SAFE[2], y1 - SAFE[3], 0)
            if over > 0:
                d = outside.setdefault((kind, info), dict(n=0, worst=0, t_first=t, t_last=t, box=None, t_worst=t))
                d['n'] += 1
                d['t_last'] = t
                if over > d['worst']:
                    d.update(worst=over, box=(x0, y0, x1, y1), t_worst=t)
        caps = [b for b in boxes if b[4] == 'caption']
        rest = [b for b in boxes if b[4] != 'caption']
        for c in caps:
            for o in rest:
                if c[0] < o[2] and o[0] < c[2] and c[1] < o[3] and o[1] < c[3]:
                    d = overlaps.setdefault((c[5], o[4], o[5]), dict(n=0, t_first=t))
                    d['n'] += 1
    return outside, overlaps, tracked


def report(outside, overlaps, tracked, times):
    print(f'safe box (internal px) x {SAFE[0]}-{SAFE[2]}, y {SAFE[1]}-{SAFE[3]}; '
          f'{len(times)} frames checked, {tracked} tracked items')
    if tracked == 0:
        print('  WARNING: nothing was drawn through reelkit.ui - check text and HUD by eye on contact sheets')
    if not outside:
        print('  OK: every caption, label and panel is inside the safe box')
    for (kind, info), d in sorted(outside.items(), key=lambda kv: -kv[1]['worst']):
        x0, y0, x1, y1 = d['box']
        print(f'  OUTSIDE  {kind:7s} {info[:24]!r:28s} up to {d["worst"]} px over, in {d["n"]} frames '
              f'({d["t_first"]:.1f}-{d["t_last"]:.1f} s; worst at {d["t_worst"]:.1f} s, box x {x0}-{x1} y {y0}-{y1})')
    for (cap, kind, info), d in overlaps.items():
        print(f'  OVERLAP  caption {cap[:20]!r} covers {kind} {info[:16]!r} in {d["n"]} frames (from {d["t_first"]:.1f} s)')
    return 1 if outside else 0


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('render_py')
    ap.add_argument('--step', type=float, default=0.5)
    ap.add_argument('--sheet', default=None, help='also save the worst frames with the safe box drawn on')
    a = ap.parse_args()
    mod = _load(a.render_py)
    times = list(np.arange(0.0, DUR, a.step))
    outside, overlaps, tracked = check(mod, times)
    code = report(outside, overlaps, tracked, times)
    if a.sheet:
        worst = sorted({d['t_worst'] for d in outside.values()})[:7] or [times[0], times[len(times) // 2]]
        os.makedirs(os.path.dirname(os.path.abspath(a.sheet)), exist_ok=True)
        img = Image.new('RGB', (len(worst) * (W + 6), H), (255, 255, 255))
        for k, t in enumerate(worst):
            img.paste(Image.fromarray(overlay(mod.render(int(round(t * FPS))))), (k * (W + 6), 0))
        img.save(a.sheet)
        print('sheet:', a.sheet)
    raise SystemExit(code)
