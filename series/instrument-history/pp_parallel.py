"""Parallel renderer for Pixel Pitch videos:  python3 pp_parallel.py video_script.py out.mp4 [workers]
Renders frames across processes into a raw buffer, then encodes once with the same ffmpeg settings as run()."""
import os, sys, subprocess, time, importlib.util
import numpy as np
from multiprocessing import Pool
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pp_framework as F
from piano_lowpoly import RW, RH, UP, FPS

def load(path):
    real_run = F.run; F.run = lambda c: None          # import the script without rendering it
    spec = importlib.util.spec_from_file_location("video", path); m = importlib.util.module_from_spec(spec)
    sys.argv = [path, "--none"]; spec.loader.exec_module(m); F.run = real_run
    return m

C = None
def _init(path, wav):
    global C
    C = load(path); F.setup(C)
    import wave
    with wave.open(wav) as w: x = np.frombuffer(w.readframes(w.getnframes()), np.int16)/32767
    F.analyze(C, x)

def _work(args):
    raw, a, b, n = args
    buf = np.memmap(raw, np.uint8, "r+", shape=(n, RH, RW, 3))
    for fi in range(a, b): buf[fi] = np.asarray(F.frame(C, fi/FPS, fi))
    buf.flush(); return b - a

if __name__ == "__main__":
    script, out = sys.argv[1], sys.argv[2]; nw = int(sys.argv[3]) if len(sys.argv) > 3 else os.cpu_count()
    t0 = time.time()
    c = load(script); F.setup(c); wav = out.replace(".mp4", ".wav"); F.render_audio(c, wav)
    n = int(c.TOTAL*FPS); raw = out + ".raw"
    np.memmap(raw, np.uint8, "w+", shape=(n, RH, RW, 3)).flush()
    step = 24; jobs = [(raw, a, min(n, a + step), n) for a in range(0, n, step)]
    with Pool(nw, _init, (script, wav)) as p:
        done = 0
        for k in p.imap_unordered(_work, jobs):
            done += k
            if done % 240 < step: print(f"{done}/{n} frames  {time.time()-t0:.0f}s", flush=True)
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{RW}x{RH}",
        "-r", str(FPS), "-i", raw, "-i", wav, "-vf", f"scale={RW*UP}:{RH*UP}:flags=neighbor", "-c:v", "libx264",
        "-pix_fmt", "yuv420p", "-crf", "16", "-preset", "medium", "-c:a", "aac", "-b:a", "192k"]
        + (["-ar", str(c.AUDIO_RATE)] if getattr(c, "AUDIO_RATE", None) else []) + ["-shortest", out], check=True)   # optional AUDIO_RATE (gapless loops)
    os.remove(raw); print("wrote", out, f"{c.TOTAL:.1f}s in {time.time()-t0:.0f}s")
