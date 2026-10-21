"""Render every frame in parallel worker processes into a raw memmap, then upscale 3x
nearest-neighbour and mux with the soundtrack.

    python -m reelkit.video render.py audio.wav out.mp4            # all cores
    python -m reelkit.video render.py audio.wav out.mp4 --procs 2

The render module must define render(f) -> (H, W, 3) uint8 for frame index f.
Typical speed: 0.5-1.5 s per frame at 360x640 -> 10-25 min for 60 s on 2 cores."""
import argparse
import importlib.util
import os
import subprocess
import sys
import time
import numpy as np
from multiprocessing import Process
from .config import W, H, FPS, NF, UP


def _load(path):
    path = os.path.abspath(path)
    sys.path.insert(0, os.path.dirname(path))
    spec = importlib.util.spec_from_file_location('render_mod', path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def _worker(path, raw, k, nproc, nf, cwd):
    os.chdir(cwd)
    mod = _load(path)
    mm = np.memmap(raw, dtype=np.uint8, mode='r+', shape=(nf, H, W, 3))
    t0 = time.time()
    for f in range(k, nf, nproc):
        mm[f] = mod.render(f)
        if f % 150 == k:
            print(f'worker {k}: frame {f}  {time.time() - t0:.0f}s', flush=True)
    mm.flush()


def render_video(render_py, audio, out, nproc=None, nf=NF, fps=FPS, raw='frames_raw.rgb', crf=18):
    nproc = nproc or os.cpu_count() or 2
    cwd = os.getcwd()
    np.memmap(raw, dtype=np.uint8, mode='w+', shape=(nf, H, W, 3)).flush()
    ps = [Process(target=_worker, args=(render_py, raw, k, nproc, nf, cwd)) for k in range(nproc)]
    t0 = time.time()
    [p.start() for p in ps]
    [p.join() for p in ps]
    if any(p.exitcode for p in ps):
        raise SystemExit('a render worker failed')
    print('rendered in', round(time.time() - t0), 's', flush=True)
    cmd = ['ffmpeg', '-y', '-loglevel', 'error',
           '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-s', f'{W}x{H}', '-r', str(fps), '-i', raw,
           '-i', audio,
           '-vf', f'scale={W * UP}:{H * UP}:flags=neighbor',
           '-c:v', 'libx264', '-preset', 'slow', '-crf', str(crf), '-pix_fmt', 'yuv420p',
           '-profile:v', 'high', '-tune', 'animation',
           '-c:a', 'aac', '-b:a', '256k', '-ar', '48000',
           '-movflags', '+faststart', '-t', str(nf / fps), out]
    subprocess.run(cmd, check=True)
    os.remove(raw)
    print('wrote', out, flush=True)


def frames_sheet(video, times, out, w=180, h=320, cols=7):
    """contact sheet from a finished mp4 (needs ffmpeg)"""
    from PIL import Image
    import tempfile
    tmp = tempfile.mkdtemp()
    ims = []
    for t in times:
        p = os.path.join(tmp, f'{t}.png')
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-ss', str(t), '-i', video, '-frames:v', '1', '-vf', f'scale={w}:{h}', p], check=True)
        ims.append(Image.open(p))
    rows = (len(ims) + cols - 1) // cols
    sheet = Image.new('RGB', (cols * w, rows * h), (255, 255, 255))
    for k, im in enumerate(ims):
        sheet.paste(im, ((k % cols) * w, (k // cols) * h))
    sheet.save(out)
    return out


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('render_py')
    ap.add_argument('audio')
    ap.add_argument('out')
    ap.add_argument('--procs', type=int, default=None)
    a = ap.parse_args()
    render_video(a.render_py, a.audio, a.out, a.procs)
