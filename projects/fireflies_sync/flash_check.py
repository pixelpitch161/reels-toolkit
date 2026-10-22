"""Photosensitivity check (Harding / WCAG 2.3.1-style general-flash test), adapted from Count the Thunder.

A transition counts when relative luminance changes by >= 0.1 (darker state < 0.8) over >= 25 % of
the frame, comparing each frame with the previous 1-5 frames (so slow rises are caught too). A flash
is a pair of opposing transitions; the rule is <= 3 flashes in any 1 s window. Also reports, per
strike, the largest share of the frame that brightens by >= 0.1 and the brightest mean frame.

    python flash_check.py --mp4 fireflies_sync.mp4           # the deliverable (all frames)
    python flash_check.py --times 49.8 50.6 0.4 1.0           # quick pre-render windows"""
import argparse
import subprocess
import numpy as np
import timeline as T

W, H, FPS = 360, 640, 30


def rel_lum(rgb):
    c = rgb.astype(np.float32) / 255.0
    c = np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
    return 0.2126 * c[..., 0] + 0.7152 * c[..., 1] + 0.0722 * c[..., 2]


def frames_mp4(path):
    cmd = ['ffmpeg', '-v', 'error', '-i', path, '-vf', f'scale={W}:{H}:flags=area', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-']
    raw = subprocess.run(cmd, check=True, capture_output=True).stdout
    return np.frombuffer(raw, np.uint8).reshape(-1, H, W, 3)


def analyse(Y, f0=0):
    n = len(Y)
    area = np.zeros(n)
    sign = np.zeros(n, int)
    for i in range(1, n):
        best, sg = 0.0, 0
        for k in range(1, 6):
            if i - k < 0:
                break
            d = Y[i] - Y[i - k]
            dark = np.minimum(Y[i], Y[i - k]) < 0.8
            up = ((d >= 0.1) & dark).mean()
            dn = ((d <= -0.1) & dark).mean()
            if up > best:
                best, sg = up, 1
            if dn > best:
                best, sg = dn, -1
        area[i], sign[i] = best, sg
    trans = [(i, sign[i]) for i in range(n) if area[i] >= 0.25]
    # collapse consecutive same-sign transitions into one event
    ev = []
    for i, s in trans:
        if ev and ev[-1][1] == s and i - ev[-1][0] <= 5:
            ev[-1] = (i, s)
        else:
            ev.append((i, s))
    flashes = []           # a flash = an up followed by a down (or vice versa)
    k = 0
    while k + 1 < len(ev):
        if ev[k][1] != ev[k + 1][1]:
            flashes.append(ev[k][0] + f0)
            k += 2
        else:
            k += 1
    worst = 0
    for a in flashes:
        worst = max(worst, sum(1 for b in flashes if a <= b < a + FPS))
    return area, flashes, worst


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--mp4')
    ap.add_argument('--times', nargs='*', type=float)
    a = ap.parse_args()
    if a.mp4:
        fr = frames_mp4(a.mp4)
        Y = np.stack([rel_lum(x) for x in fr])
        area, flashes, worst = analyse(Y)
        print(f'frames: {len(Y)}  general flashes (>=25% area): {len(flashes)}  worst in any 1 s: {worst}  (limit 3)')
        print(f'brightest mean frame luminance: {Y.mean((1, 2)).max():.3f}   darkest: {Y.mean((1, 2)).min():.3f}')
        for k, tb in enumerate(T.BURSTS):
            f0 = int(tb * FPS)
            seg = area[max(1, f0 - 1):f0 + int(8 * T.BEAT * FPS) + 2]
            print(f'  burst {k} @ {tb:5.2f}s  max share of frame changing by >=0.1: {seg.max() * 100:5.1f} %')
    else:
        import render as R
        for t0, t1 in zip(a.times[::2], a.times[1::2]):
            fs = range(int(t0 * FPS), int(t1 * FPS) + 1)
            Y = np.stack([rel_lum(R.render(f)) for f in fs])
            area, flashes, worst = analyse(Y, fs[0])
            print(f'{t0:5.2f}-{t1:5.2f}s  max share changing by >=0.1: {area.max() * 100:5.1f} %  flashes: {len(flashes)}  mean Y max {Y.mean((1, 2)).max():.3f}')


if __name__ == '__main__':
    main()
