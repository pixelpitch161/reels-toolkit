"""Every check an episode passes before (and after) its one full render, in one place:

    focus      the subject's circle of confusion is 0 px on every frame (ring corners, balls, each fighter's parts)
    safezone   captions, labels and the HUD inside reelkit.config.SAFE (reelkit.safezone)
    levels     consistent loudness from frame 0 (reelkit.levels)
    band       each fighter's band sits above the bed (its Sound.band) while it plays
    loudness   perceived loudness per section (BS.1770 K-weighting): the intro vs the fight
    flash      no general flashes on the busiest stretch (reelkit.flashcheck on rendered luminance)
    loop       the loop point is the file boundary (reelkit.loopcheck: last 5 s + first 1 s, the audio seam)"""
import math
import multiprocessing as mp
import os
import subprocess
import sys

import numpy as np

from .episode import current, KIT

ENTRY = os.path.join(KIT, 'entry', 'episode_render.py')


def _P():
    from . import picture
    return picture


# ---------------------------------------------------------------- focus
def focus():
    P = _P()
    T = P.T
    S = P.S
    from reelkit.lens import coc_map
    worst = (0.0, None, None)
    corners = [S.to_world(sx, sz, y) for sx in (-S.AP, S.AP) for sz in (-S.AP, S.AP) for y in (0.0, S.POST_H)]
    for f in range(T.NF):
        t = f / T.FPS
        cam = P.camera(t)
        zf = cam.depth(S.SUBJECT)[0]
        pts, names = list(corners), ['ring'] * len(corners)
        fs = [P.fighter(k, t) for k in (0, 1)]
        for k in (0, 1):
            st = fs[k]
            for dy in (0.0, 2 * P.R):
                pts.append(S.to_world(st['p'][0], st['p'][1], dy))
                names.append(P.NAMES[k])
            for w, lab in P.LOOKS[k].focus_points(t, st):
                pts.append(w)
                names.append(lab)
        if T.T_FIGHT <= t < P.T_HS and 'K' in P.RUN:
            for row in P.RUN['K'][P.sim_i(t)]:
                if len(row) and not np.isnan(row[0]):
                    pts.append(S.to_world(row[0], row[1], 0.45))
                    names.append('key')
        for kind, p, rot, size, curl, a in P.parts_at(t):
            pts.append(S.to_world(p[0], p[2], p[1]))
            names.append('shard')
        c = coc_map(cam.depth(np.array(pts)), zf, 26.0, 0.17)
        i = int(np.argmax(c))
        if c[i] > worst[0]:
            worst = (float(c[i]), t, names[i])
    ok = worst[0] == 0
    print(f'focus: max subject CoC over {T.NF} frames: {worst[0]:.3f} px'
          + (f' (at {worst[1]:.2f} s, {worst[2]})' if worst[1] is not None else '') + ('  PASS' if ok else '  FAIL'))
    return ok


# ---------------------------------------------------------------- safe zone
def safezone(step=0.25, sheet=None):
    from reelkit import safezone as SZ
    P = _P()
    times = list(np.arange(0.0, P.T.NF / P.T.FPS, step))
    outside, overlaps, tracked = SZ.check(P, times)
    code = SZ.report(outside, overlaps, tracked, times)
    print('safezone:', 'PASS' if code == 0 else 'FAIL')
    return code == 0


# ---------------------------------------------------------------- audio checks
def levels(wav=None):
    ep = current()
    wav = wav or ep.path('audio.wav')
    r = subprocess.run([sys.executable, '-m', 'reelkit.levels', wav], capture_output=True, text=True)
    out = r.stdout.strip().splitlines()
    print('levels: ' + ' | '.join(l.strip() for l in out if l.startswith(('opening', 'build', 'spread', 'dips'))))
    print('levels:', out[-1] if out else r.stderr)
    return r.returncode == 0


def _stem(k):
    from scipy.io import wavfile
    sr, x = wavfile.read(os.path.join(current().check, f'stem_{k}.wav'))
    x = x.astype(float)
    x = x.mean(1) if x.ndim > 1 else x
    return sr, x / 32768


def band():
    """each fighter's band over the bed (drums, the bed's buses, fx), per section; 'while it sounds' gates on its own
    envelope. Both numbers should be clearly positive (the science / the fighters sit on top)"""
    from scipy.signal import butter, sosfilt
    from . import music as M
    T = current().T

    def bp(x, sr, lo, hi):
        return sosfilt(butter(4, [lo, hi], 'band', fs=sr, output='sos'), x)

    def db(x):
        return 20 * np.log10(np.sqrt((x ** 2).mean()) + 1e-9)

    def gated(f, bed, sr):
        w = int(0.02 * sr)
        e = np.sqrt(np.convolve(f ** 2, np.ones(w) / w, 'same'))
        on = e > e.max() * 0.1
        return db(f[on]) - db(bed[on]) if on.any() else float('nan')
    spans = [(T.T_FIGHT, 10), (10, 20), (20, 32), (32, 44), (44, M.T_KO)]
    bed_buses = [k for k in list(M.bed.buses) + ['fx'] if os.path.exists(os.path.join(current().check, f'stem_{k}.wav'))]
    ok = True
    for s in M.SOUNDS:
        lo, hi = s.band
        sr, x = _stem(s.bus)
        f = bp(x, sr, lo, hi)
        bedx = sum(bp(_stem(k)[1], sr, lo, hi) for k in bed_buses)
        vals = []
        for a, b in spans:
            sl = slice(int(a * sr), int(b * sr))
            vals.append((db(f[sl]) - db(bedx[sl]), gated(f[sl], bedx[sl], sr)))
        ok &= all(g > 0 for _, g in vals if not math.isnan(g))
        print(f'band: {s.name:9s} {lo}-{hi} Hz over the bed (dB), section / while it sounds: '
              + ' '.join(f'{v:+5.1f}/{g:+5.1f}' for v, g in vals))
    return ok


def loudness(wav=None):
    """perceived loudness per section (ITU-R BS.1770 K-weighting): RMS weighs every frequency alike, the ear hears
    bright clangs louder (#40 v2: an intro that matched the fight in RMS still read loud)"""
    from scipy.io import wavfile
    from scipy.signal import lfilter
    from . import music as M
    T = current().T
    sr, x = wavfile.read(wav or current().path('audio.wav'))
    x = x.astype(float) / 32768.0
    b1, a1 = [1.53512485958697, -2.69169618940638, 1.19839281085285], [1.0, -1.69065929318241, 0.73248077421585]
    b2, a2 = [1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621]
    y = lfilter(b2, a2, lfilter(b1, a1, x, axis=0), axis=0)

    def lufs(z):
        return -0.691 + 10 * np.log10((z ** 2).mean(0).sum() + 1e-12)
    fight = lufs(y[int(T.T_FIGHT * sr):int(M.T_KO * sr)])
    intro = lufs(y[:int(T.T_FIGHT * sr)])
    print(f'loudness: intro (title card + count-in) {intro:.1f} LUFS vs the fight {fight:.1f}: {intro - fight:+.1f} LU')
    return intro - fight


# ---------------------------------------------------------------- flash
def _lum(f):
    from reelkit.flashcheck import rel_lum
    return rel_lum(_P().render(f))


def flash(t0=None, t1=None, procs=2):
    """render a stretch (default: the busiest one, the desperation moves to just after the K.O.) and run
    reelkit.flashcheck.analyse on its luminance"""
    from reelkit.flashcheck import analyse
    P = _P()
    if t0 is None:
        lows = [t for t, w, n in P.STORY['firsts'] if n in [P.CFG[k]['abilities']['low']['name'] for k in (0, 1)]]
        t0 = max(P.T.T_FIGHT, min(lows + [P.T_KO - 8]) - 1.0)
        t1 = min(P.T.DUR, P.T_KO + 1.5)
    f0, f1 = int(t0 * P.T.FPS), int(t1 * P.T.FPS)
    with mp.get_context('fork').Pool(procs) as Pl:
        Y = np.array(Pl.map(_lum, range(f0, f1)))
    area, flashes, worst = analyse(Y)
    i = int(np.argmax(area))
    ok = worst <= 3
    print(f'flash: {t0:.1f}-{t1:.1f} s, {f1 - f0} frames: general flashes {len(flashes)}, worst in any 1 s {worst}; '
          f'largest share of frame changing >= 0.1: {100 * area[i]:.1f} % at {(f0 + i) / P.T.FPS:.2f} s'
          + ('  PASS' if ok else '  FAIL'))
    return ok


# ---------------------------------------------------------------- loop
def loop(wav=None, procs=2):
    from reelkit import loopcheck as LC
    ep = current()
    T = ep.T
    nt, nh = int(LC.TAIL_S * T.FPS), int(LC.HEAD_S * T.FPS)
    idx = list(range(T.NF - nt, T.NF)) + list(range(nh))
    fr = LC.frames_from_render(ENTRY, idx, procs)
    fails, lines = LC.check_picture(fr[:nt], fr[nt:], (T.NF - nt) / T.FPS, T.NF / T.FPS)
    wav = wav or ep.path('audio.wav')
    if os.path.exists(wav):
        f2, l2 = LC.check_audio(wav)
        fails += f2
        lines += l2
    print('loop check (the loop point must be the file boundary):')
    print('\n'.join(lines))
    print('loop:', 'PASS' if not fails else f'{fails} FAIL(s)')
    return not fails


def verify(mp4):
    """after the render: ffprobe, loop + flash on the mp4"""
    out = subprocess.run(['ffprobe', '-v', 'error', '-show_entries', 'stream=codec_name,width,height:format=duration',
                          '-of', 'compact', mp4], capture_output=True, text=True).stdout.strip()
    print('ffprobe:', out.replace('\n', ' | '))
    ok = True
    for mod in ('reelkit.loopcheck', 'reelkit.flashcheck'):
        r = subprocess.run([sys.executable, '-m', mod, mp4], capture_output=True, text=True)
        last = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else r.stderr.strip()[-200:]
        print(f'{mod}: {last}')
        ok &= r.returncode == 0
    return ok
