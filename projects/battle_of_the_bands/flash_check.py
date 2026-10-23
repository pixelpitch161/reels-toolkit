"""Photosensitivity check (Harding / WCAG 2.3.1-style general-flash test), adapted from marble_waterfall_organ (checks the bell, knockouts, win flood and crack).

A transition counts when relative luminance changes by >= 0.1 (darker state < 0.8) over >= 25 % of the
frame, comparing each frame with the previous 1-5 frames. A flash is a pair of opposing transitions;
the rule is <= 3 flashes in any 1 s window. Also reports the largest share of the frame changing at the
drop and on the bar-line bursts.

    python flash_check.py battle_of_the_bands.mp4"""
import subprocess
import sys
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


def analyse(Y):
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
    ev = []
    for i, s in trans:
        if ev and ev[-1][1] == s and i - ev[-1][0] <= 5:
            ev[-1] = (i, s)
        else:
            ev.append((i, s))
    flashes = []
    k = 0
    while k + 1 < len(ev):
        if ev[k][1] != ev[k + 1][1]:
            flashes.append(ev[k][0])
            k += 2
        else:
            k += 1
    worst = max([sum(1 for b in flashes if a <= b < a + FPS) for a in flashes] or [0])
    return area, flashes, worst


if __name__ == '__main__':
    fr = frames_mp4(sys.argv[1])
    Y = np.stack([rel_lum(x) for x in fr])
    area, flashes, worst = analyse(Y)
    print(f'frames: {len(Y)}  general flashes (>=25% area): {len(flashes)}  worst in any 1 s: {worst}  (limit 3)')
    print(f'largest share of frame changing by >=0.1 anywhere: {area.max() * 100:.1f} % at {area.argmax() / FPS:.2f} s')
    for tb in [T.T_BELL] + [t for t, q in T.OUTS] + ([T.T_WIN] if T.T_WIN else []) + [T.T_CRACK, 0.0]:
        f0 = int(tb * FPS)
        print(f'  {tb:5.1f}s  max share changing: {area[f0:f0 + 15].max() * 100:5.1f} %')
