"""A 6-8 s low-res clip (360x640, 30 fps, with sound) of one fighter's signature weapon at work and its desperation
move, from an honest fight against a sparring partner, for the user to approve before a new fighter is tuned and
rendered. Two windows: the 3.5 s where its weapon lands the most hits (once it has grown a little), then its
desperation move from just before it fires. Labels name what each window shows.

    python -m ballfight preview HARP [--vs TROMBONE] [--seconds 7]    -> projects/ballfight/previews/harp/harp_preview.mp4

The fighter needs its roster entry (fighters.py) and its look + sound module (kit/fighters/<name>.py); the partner is
any ported fighter (default: the first other one). The fight is a normal honest run at 118 BPM in E minor on the
big-top bed; a seed is picked where the fighter's desperation move fires with room to play out (if it never drops
below 25 HP, the partner hits harder, x1.5 then x2)."""
import os
import subprocess
import time
import types

import numpy as np

from . import episode as E
from . import fighters as FM

LOOP = ['Em', 'Am', 'B7', 'Em', 'C', 'Am', 'B7', 'B7']


def _config(name, partner, side=None, seed=None):
    from reelkit.config import loop_bpm
    bars = int(round(61.0 / (4 * 60.0 / loop_bpm(120, 61.0))))
    chords = (['Em', 'B7'] + LOOP * 8)[:bars]
    return types.SimpleNamespace(FIGHTERS=(name, partner), BPM=120, MOOD='big_top', KEY='E', SCALE='harmonic minor',
                                 CHORDS=chords, LOOP=LOOP, SEED=seed, DMG_SIDE=side)


def _find_seed(name, partner, folder, tries=80):
    """(seed, handicap, t_low, t_ko): an honest run where the fighter's desperation move fires >= 3 s before the K.O."""
    from fighters import FIGHTERS
    low = FIGHTERS[name]['abilities']['low']['name']
    for side in (None, (1.0, 1.5), (1.0, 2.0)):
        E.load(folder, _config(name, partner, side))
        from . import story
        for seed in range(1, tries + 1):
            r = story.run_seed(seed, record=False)
            t_ko = r['t_ko'] or 61.0
            fired = [e[0] for e in r['events'] if e[1] == 'ability' and e[2] == 0 and e[3] == low]
            if fired and fired[0] > 12 and fired[0] + 3.0 < t_ko:
                return seed, side, fired[0], t_ko
    raise SystemExit(f"{name}'s desperation move never fired in {tries} seeds x 3 handicaps vs {partner}")


def _windows(P, k, t_low, seconds):
    half = seconds / 2
    hits = np.array([e[0] for e in P.EV if e[1] == 'hit' and e[2] == k and e[0] < t_low - 1.0])
    lo, hi = P.T.T_FIGHT + 6.0, max(P.T.T_FIGHT + 6.0, t_low - half - 1.0)
    best, t_sig = -1, lo
    for t0 in np.arange(lo, hi + 1e-6, 0.25):
        n = int(((hits >= t0) & (hits < t0 + half)).sum())
        if n > best:
            best, t_sig = n, float(t0)
    t_des = t_low - 0.6
    return [(t_sig, t_sig + half, 'SIGNATURE'), (t_des, t_des + half, 'DESPERATION')]


def _render_frame(f):
    from . import picture
    return picture.render(f)


def main(name, vs=None, seconds=7.0, sound=True, procs=2):
    t_start = time.time()
    E.activate_paths()
    from fighters import FIGHTERS
    name = name.upper()
    if name not in FIGHTERS:
        raise SystemExit(f'{name} is not in the roster (projects/ballfight/fighters.py)')
    if FM.module(name) is None:
        raise SystemExit(f'{name} has no look / sound module yet: kit/fighters/{FM.slug(name)}.py')
    ported = [n for n in FM.ported() if n != name]
    partner = (vs or ported[0]).upper()
    folder = os.path.join(E.BALLFIGHT, 'previews', FM.slug(name))
    os.makedirs(folder, exist_ok=True)
    with open(os.path.join(os.path.dirname(folder), '.gitignore'), 'w') as f:
        f.write('*\n')
    seed, side, t_low, t_ko = _find_seed(name, partner, folder)
    ep = E.load(folder, _config(name, partner, side, seed))
    from . import picture as P
    if not os.path.exists(P.SPR_PATH):
        P.build_sprites()
    wins = _windows(P, 0, t_low, seconds)
    low = FIGHTERS[name]['abilities']['low']['name']
    style = FIGHTERS[name]['weapon'].get('style', FIGHTERS[name]['weapon']['kind'])
    frames = [(f, lab) for a, b, lab in wins for f in range(int(a * 30), int(b * 30))]
    from multiprocessing import get_context
    with get_context('fork').Pool(procs) as Pl:
        imgs = Pl.map(_render_frame, [f for f, _ in frames], chunksize=4)
    from reelkit.ui import label
    raw = os.path.join(ep.check, 'preview.rgb')
    with open(raw, 'wb') as fo:
        for (f, lab), img in zip(frames, imgs):
            img = img.copy()
            txt = f'{name} {"SIGNATURE: " + style.upper() if lab == "SIGNATURE" else "DESPERATION: " + low}'
            img[100:112, 20:20 + 6 * len(txt) + 4] = (img[100:112, 20:20 + 6 * len(txt) + 4] * 0.3).astype(np.uint8)
            label(img, txt, 22, 102, (255, 236, 160), 1)
            fo.write(np.ascontiguousarray(img).tobytes())
    out = os.path.join(folder, f'{FM.slug(name)}_preview.mp4')
    from reelkit.config import W, H
    cmd = ['ffmpeg', '-y', '-loglevel', 'error', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-s', f'{W}x{H}', '-r', '30',
           '-i', raw]
    if sound:
        from . import music as M
        from reelkit import audio as A
        m, _ = M.master(M.score())
        parts = [m[int(a * A.SR):int(a * A.SR) + int(round((b - a) * 30)) * A.SR // 30] for a, b, _ in wins]
        wav = os.path.join(ep.check, 'preview.wav')
        A.write(wav, np.concatenate(parts))
        cmd += ['-i', wav, '-c:a', 'aac', '-b:a', '192k']
    cmd += ['-c:v', 'libx264', '-preset', 'fast', '-crf', '20', '-pix_fmt', 'yuv420p', '-shortest', out]
    subprocess.run(cmd, check=True)
    os.remove(raw)
    print(f'{name} vs {partner}, seed {seed}' + (f', partner damage x{side[1]:g}' if side else '') +
          f': signature {wins[0][0]:.1f}-{wins[0][1]:.1f} s, {low} at {t_low:.1f} s (K.O. {t_ko:.1f} s)')
    print(f'preview: {out}  ({len(frames) / 30:.1f} s, {time.time() - t_start:.0f} s to make)')
    return out
