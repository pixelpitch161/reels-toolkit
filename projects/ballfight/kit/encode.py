"""The one full render and the loop-clean encode.

render: reelkit.video renders every frame in 2 worker processes into the episode's kept lossless raw
(frames_raw.rgb, for later --frames patches), then projects/maze_race/mux.py encodes it loop-clean (a keyframe on the
last frame, x264 mbtree off, the soundtrack muxed as whole AAC frames) into <episode>/<name>.mp4."""
import importlib.util
import os
import time

from .episode import current, PROJECTS
from .checks import ENTRY


def _maze_mux():
    spec = importlib.util.spec_from_file_location('maze_mux', os.path.join(PROJECTS, 'maze_race', 'mux.py'))
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def mux(out=None):
    ep = current()
    out = out or ep.out
    cwd = os.getcwd()
    os.chdir(ep.dir)
    try:
        _maze_mux().main(raw='frames_raw.rgb', audio='audio.wav', out=out)
    finally:
        os.chdir(cwd)
    return out


def render(frames=None, only_mux=False, procs=None):
    """the full render (or a --frames patch / --only-mux) into the kept raw, then ONE loop-clean encode (reelkit.video's
    own encode is skipped: the mux re-encodes anyway)"""
    import numpy as np
    from multiprocessing import get_context
    from reelkit import video as V
    from reelkit.config import W, H
    ep = current()
    nf = ep.T.NF
    nproc = procs or os.cpu_count() or 2
    raw = os.path.join(ep.dir, 'frames_raw.rgb')
    size = nf * H * W * 3
    t0 = time.time()
    if frames is not None or only_mux:
        if not os.path.exists(raw) or os.path.getsize(raw) != size:
            raise SystemExit('frames_raw.rgb missing or wrong size: run a full render first (patches need the kept raw)')
    else:
        np.memmap(raw, dtype=np.uint8, mode='w+', shape=(nf, H, W, 3)).flush()
    if not only_mux:
        f0, f1 = frames if frames is not None else (0, nf)
        ctx = get_context('fork')
        ps = [ctx.Process(target=V._worker, args=(ENTRY, raw, k, nproc, nf, ep.dir, max(0, f0), min(nf, f1)))
              for k in range(nproc)]
        [p.start() for p in ps]
        [p.join() for p in ps]
        if any(p.exitcode for p in ps):
            raise SystemExit('a render worker failed')
        print(f'rendered frames {f0}-{f1 - 1} in {time.time() - t0:.0f} s', flush=True)
    t1 = time.time()
    out = mux()
    print(f'render {t1 - t0:.0f} s, loop-clean encode {time.time() - t1:.0f} s -> {out}')
    return out
