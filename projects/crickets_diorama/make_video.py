"""Render all 1800 frames (two worker processes) into a low-res raw buffer,
then upscale 3x nearest-neighbour and mux with the soundtrack."""
import os, subprocess, sys, time
import numpy as np
from multiprocessing import Process

W, H, FPS, NF = 360, 640, 30, 1800
RAW = 'frames_360x640.rgb'


def worker(k, nproc):
    from drender import render
    mm = np.memmap(RAW, dtype=np.uint8, mode='r+', shape=(NF, H, W, 3))
    t0 = time.time()
    for f in range(k, NF, nproc):
        mm[f] = render(f)
        if f % 150 == k:
            print(f'worker {k}: frame {f}  {time.time() - t0:.0f}s', flush=True)
    mm.flush()


if __name__ == '__main__':
    np.memmap(RAW, dtype=np.uint8, mode='w+', shape=(NF, H, W, 3)).flush()
    nproc = 2
    ps = [Process(target=worker, args=(k, nproc)) for k in range(nproc)]
    t0 = time.time()
    [p.start() for p in ps]
    [p.join() for p in ps]
    print('rendered in', round(time.time() - t0), 's')
    out = sys.argv[1] if len(sys.argv) > 1 else 'cricket_diorama.mp4'
    cmd = ['ffmpeg', '-y', '-loglevel', 'error',
           '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-s', f'{W}x{H}', '-r', str(FPS), '-i', RAW,
           '-i', 'audio.wav',
           '-vf', 'scale=1080:1920:flags=neighbor',
           '-c:v', 'libx264', '-preset', 'slow', '-crf', '18', '-pix_fmt', 'yuv420p',
           '-profile:v', 'high', '-tune', 'animation',
           '-c:a', 'aac', '-b:a', '256k', '-ar', '48000',
           '-movflags', '+faststart', '-t', '60', out]
    subprocess.run(cmd, check=True)
    print('wrote', out)
