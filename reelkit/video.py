"""Render every frame in parallel worker processes into a raw memmap, then upscale 3x
nearest-neighbour and mux with the soundtrack.

    python -m reelkit.video render.py audio.wav out.mp4                       # full render, all cores
    python -m reelkit.video render.py audio.wav out.mp4 --frames 41.5:44.0    # patch: re-render only 41.5-44 s
    python -m reelkit.video render.py audio.wav out.mp4 --only-mux            # new audio, same picture

The raw frames (frames_raw.rgb, ~1.2 GB, git-ignored) are kept after a full render so later fixes
re-render only the frames they change and re-mux (no quality loss: the raw is lossless). Delete it with
--discard-raw (or by hand) once the video is approved.

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


def project_nf(render_py, fps=FPS):
    """frame count of the project that render_py belongs to: NF (or DUR * fps) from the timeline.py next to it,
    else the toolkit default (config.NF, 61 s)"""
    tl = os.path.join(os.path.dirname(os.path.abspath(render_py)), 'timeline.py')
    if os.path.exists(tl):
        try:
            src = open(tl).read()
            ns = {}
            for line in src.splitlines():                      # only the plain constants: no imports, no side effects
                s = line.split('#')[0].strip()
                if s.startswith(('FPS', 'DUR', 'NF')) and '=' in s and '(' not in s.split('=', 1)[1].replace('int(', ''):
                    try:
                        exec(s, {'int': int, 'round': round}, ns)
                    except Exception:
                        pass
            if 'NF' in ns:
                return int(ns['NF'])
            if 'DUR' in ns:
                return int(round(float(ns['DUR']) * ns.get('FPS', fps)))
        except OSError:
            pass
    return NF


def _worker(path, raw, k, nproc, nf, cwd, f0=0, f1=None):
    os.chdir(cwd)
    mod = _load(path)
    mm = np.memmap(raw, dtype=np.uint8, mode='r+', shape=(nf, H, W, 3))
    t0 = time.time()
    f1 = nf if f1 is None else f1
    for f in range(f0 + k, f1, nproc):
        mm[f] = mod.render(f)
        if (f - f0) % 150 == k:
            print(f'worker {k}: frame {f}  {time.time() - t0:.0f}s', flush=True)
    mm.flush()


def render_video(render_py, audio, out, nproc=None, nf=None, fps=FPS, raw='frames_raw.rgb', crf=18,
                 frames=None, only_mux=False, keep_raw=True):
    """frames=(f0, f1): re-render only frames f0..f1-1 into the existing raw, then re-mux.
    only_mux: skip rendering (e.g. new audio). keep_raw: keep the lossless raw for later patches.
    nf: frame count; default = the project's own length from its timeline.py (project_nf)."""
    nf = nf or project_nf(render_py, fps)
    nproc = nproc or os.cpu_count() or 2
    cwd = os.getcwd()
    size = nf * H * W * 3
    if frames is not None or only_mux:
        if not os.path.exists(raw) or os.path.getsize(raw) != size:
            raise SystemExit(f'{raw} missing or wrong size: run a full render first (patches need the kept raw)')
    else:
        np.memmap(raw, dtype=np.uint8, mode='w+', shape=(nf, H, W, 3)).flush()
    if not only_mux:
        f0, f1 = frames if frames is not None else (0, nf)
        f0, f1 = max(0, f0), min(nf, f1)
        ps = [Process(target=_worker, args=(render_py, raw, k, nproc, nf, cwd, f0, f1)) for k in range(nproc)]
        t0 = time.time()
        [p.start() for p in ps]
        [p.join() for p in ps]
        if any(p.exitcode for p in ps):
            raise SystemExit('a render worker failed')
        print(f'rendered frames {f0}-{f1 - 1} in', round(time.time() - t0), 's', flush=True)
    cmd = ['ffmpeg', '-y', '-loglevel', 'error',
           '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-s', f'{W}x{H}', '-r', str(fps), '-i', raw,
           '-i', audio,
           '-vf', f'scale={W * UP}:{H * UP}:flags=neighbor',
           '-c:v', 'libx264', '-preset', 'slow', '-crf', str(crf), '-pix_fmt', 'yuv420p',
           '-profile:v', 'high', '-tune', 'animation',
           '-c:a', 'aac', '-b:a', '256k', '-ar', '48000',
           '-movflags', '+faststart', '-t', str(nf / fps), out]
    subprocess.run(cmd, check=True)
    if not keep_raw:
        os.remove(raw)
    print('wrote', out, flush=True)


def _parse_frames(s, fps=FPS):
    """'41.5:44' (seconds) or 'f1245:f1320' (frame indices) -> (f0, f1)"""
    a, b = s.split(':')
    conv = (lambda x: int(x[1:])) if a.startswith('f') else (lambda x: int(round(float(x) * fps)))
    return conv(a), conv(b)


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
    ap.add_argument('--frames', default=None, help="re-render only this range: '41.5:44' s or 'f1245:f1320'")
    ap.add_argument('--only-mux', action='store_true', help='no rendering: re-mux the kept raw (e.g. new audio)')
    ap.add_argument('--discard-raw', action='store_true', help='delete the raw frames after muxing')
    a = ap.parse_args()
    render_video(a.render_py, a.audio, a.out, a.procs, frames=_parse_frames(a.frames) if a.frames else None,
                 only_mux=a.only_mux, keep_raw=not a.discard_raw)
