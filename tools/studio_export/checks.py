"""checks: the numbers each of the kit's checks gives for the example episode.

studio/reference/checks/flute_vs_trombone.json: for every check in kit/checks.py (focus, safezone, levels, band,
loudness, flash, loop) the kit's verdict, the lines it prints, and the unrounded values behind those lines.

How each one is run (nothing in the kit is edited):

    focus      kit.checks.focus() as it is (camera maths for all 1,830 frames, no frame is drawn); the worst circle
               of confusion is read by wrapping reelkit.lens.coc_map for the call
    safezone   kit.checks.safezone() on the boxes reelkit.ui logs while a frame is drawn every 0.25 s (244 frames).
               The frames are drawn here in a pool and the kit's check then reads the logged boxes back
    levels     kit.checks.levels() (it runs `python -m reelkit.levels audio.wav`); the profile rows from
               reelkit.levels.profile()
    band       kit.checks.band() on the stem wavs kit.music.main() writes
    loudness   kit.checks.loudness()
    flash      kit.checks.flash() with its `fork` pool swapped for this module's `spawn` pool (315 frames)
    loop       kit.checks.loop() with reelkit.loopcheck.frames_from_render swapped the same way (150 + 30 frames)

Where a check only prints its numbers (rounded), the same formulas are computed here from the same inputs, and the
export stops unless they format to exactly the line the kit printed; the kit's line is the arbiter.

The soundtrack is built first, in this export's own interpreter and cache folder (the noise generator again), and
must be byte for byte the committed audio.wav. The frames are drawn in a `spawn` pool of 5 workers that each load the
episode themselves; a frame is the same whichever process draws it."""
import contextlib
import hashlib
import io
import json
import math
import multiprocessing as mp
import os
import subprocess
import sys
import time
import types

from . import common
from .dsp_vectors import fresh
from . import voices as V
from . import stems as ST

EPISODE = V.EPISODE
TAG = 'checks'
OUT = common.ref('checks', f'{EPISODE}.json')
WORKERS = 5
SAFE_STEP = 0.25                                           # kit.checks.safezone's default step
CHECKS = ('focus', 'safezone', 'levels', 'band', 'loudness', 'flash', 'loop')

_PICTURE = None


# ---------------------------------------------------------------- the frame pool
def _init():
    """pool worker start: load the episode in this process (spawn starts from nothing)"""
    global _PICTURE
    common.episode(EPISODE, TAG)
    from ballfight.kit import picture
    _PICTURE = picture


def _draw(f):
    """(frame number, the frame as the kit draws it, the boxes reelkit.ui logged while it did)"""
    from reelkit import ui
    ui.reset_drawn()
    a = _PICTURE.render(f)
    return f, a, list(ui.DRAWN)


class Frames:
    """draws frames in the pool and keeps them: frame number -> (the (640, 360, 3) uint8 frame, the logged boxes)"""

    def __init__(self, pool):
        self.pool, self.got, self.seconds = pool, {}, 0.0

    def need(self, frames):
        todo = sorted(set(int(f) for f in frames) - set(self.got))
        if todo:
            t0 = time.time()
            for f, a, boxes in self.pool.map(_draw, todo, chunksize=4):
                self.got[f] = (a, boxes)
            self.seconds += time.time() - t0
        return [self.got[int(f)] for f in frames]


def _lines(buf):
    return [line.rstrip() for line in buf.getvalue().splitlines() if line.strip()]


@contextlib.contextmanager
def swapped(obj, name, new):
    old = getattr(obj, name)
    setattr(obj, name, new)
    try:
        yield old
    finally:
        setattr(obj, name, old)


def _said(lines, text, what):
    """the export's own number must be in the kit's printed lines exactly as the kit formatted it"""
    assert any(text in line for line in lines), f'{what}: the kit did not print {text!r}; it printed {lines}'


# ---------------------------------------------------------------- one function per check
def focus(KC):
    import numpy as np
    import reelkit.lens as lens
    seen = dict(calls=0, points=0, worst=0.0)
    coc0 = lens.coc_map

    def coc_map(depth, *a, **k):
        c = coc0(depth, *a, **k)
        seen['calls'] += 1
        seen['points'] += int(np.size(c))
        seen['worst'] = max(seen['worst'], float(np.max(c)))
        return c
    buf = io.StringIO()
    with swapped(lens, 'coc_map', coc_map), contextlib.redirect_stdout(buf):
        ok = KC.focus()
    lines = _lines(buf)
    _said(lines, f"max subject CoC over {seen['calls']} frames: {seen['worst']:.3f} px", 'focus')
    assert ok == (seen['worst'] == 0)
    return {'pass': bool(ok), 'printed': lines, 'worst_coc': seen['worst'], 'frames': seen['calls'],
            'points': seen['points'], 'needs_frames': 0,
            'how': 'for every frame: the ring\'s 8 corners, both balls (floor and top), each look\'s focus_points, '
                   'the keys and the shards; coc_map(depth, focus depth, 26.0, 0.17); passes when the largest is 0'}


def safezone(KC, P, frames):
    import numpy as np
    from reelkit import safezone as SZ
    from reelkit import ui
    from reelkit.config import SAFE, SAFE_BUTTONS, FPS
    times = list(np.arange(0.0, P.T.NF / P.T.FPS, SAFE_STEP))          # kit.checks.safezone's own list
    numbers = [int(round(t * FPS)) for t in times]                     # reelkit.safezone.check's frame of each time
    boxes = dict(zip(numbers, (b for _, b in frames.need(numbers))))
    asked = []

    def render(f):                                           # what reelkit.safezone.check calls for each time
        asked.append(f)
        ui.DRAWN.extend(boxes[f])
    fake = types.SimpleNamespace(T=P.T, render=render)
    kept = {}
    check0 = SZ.check

    def check(mod, ts):
        kept['out'] = check0(mod, ts)
        return kept['out']
    buf = io.StringIO()
    with swapped(KC, '_P', lambda: fake), swapped(SZ, 'check', check), contextlib.redirect_stdout(buf):
        ok = KC.safezone()
    ui.reset_drawn()
    assert asked == numbers, 'reelkit.safezone asked for other frames than were drawn'
    outside, overlaps, tracked = kept['out']
    lines = _lines(buf)
    _said(lines, f'{len(times)} frames checked, {tracked} tracked items', 'safezone')
    assert ok == (not outside) and tracked == sum(len(boxes[f]) for f in numbers)
    return {'pass': bool(ok), 'printed': lines, 'step': SAFE_STEP, 'frames_checked': len(times), 'tracked': tracked,
            'needs_frames': len(numbers), 'safe': list(SAFE), 'safe_buttons': list(SAFE_BUTTONS),
            'outside': [dict(kind=k, info=i, n=d['n'], worst=d['worst'], t_first=float(d['t_first']),
                             t_last=float(d['t_last']), t_worst=float(d['t_worst']), box=list(d['box']))
                        for (k, i), d in outside.items()],
            'overlaps': [dict(caption=c, kind=k, info=i, n=d['n'], t_first=float(d['t_first']))
                         for (c, k, i), d in overlaps.items()],
            'how': 'boxes: for each checked time t (frame int(round(t * 30)), Python rounding), every box '
                   'reelkit.ui drew: [x0, y0, x1, y1, kind, info] in internal pixels, in drawing order. A box is '
                   'outside when reelkit.config.safe_overshoot is above 0; panels and labels partly off the frame '
                   'are skipped as sliding in or out, captions never are',
            'boxes': [dict(t=float(t), frame=f, boxes=[list(b) for b in boxes[f]]) for t, f in zip(times, numbers)]}


def levels(KC, wav):
    import numpy as np
    from reelkit import levels as LV
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        ok = KC.levels()
    lines = _lines(buf)
    r = subprocess.run([sys.executable, '-m', 'reelkit.levels', wav], capture_output=True, text=True)   # as the kit
    module = [line.rstrip() for line in r.stdout.strip().splitlines()]
    assert (r.returncode == 0) == ok and module[-1] in ('PASS', 'FAIL')
    # the profile, and main()'s summary lines again from it
    sr, x = LV.load(wav)
    rows = LV.profile(sr, x, dur=round(len(x) / sr / LV.WIN) * LV.WIN)
    db = np.array([v[1] for v in rows])
    hits = np.array([v[2] for v in rows])
    half = len(db) // 2
    ref = db[-10:].mean()
    opening, build = db[:5].mean(), db[half:].mean() - db[:half].mean()
    dips = [(t, v) for t, v, _ in rows if v < ref - 3.0]
    rest = np.delete(db, np.argmin(db)) if len(dips) == 1 else db
    spread = np.ptp(rest)
    for (t, v, h), line in zip(rows, module[1:]):
        assert line == f'{t:4.0f}-{t + LV.WIN:<3.0f} {v:6.1f} {h:6.1f}'.rstrip(), (line, t, v, h)
    _said(module, f'ending (last 20 s): {ref:.1f} dBFS', 'levels')
    _said(module, f'opening (0-10 s): {opening:.1f} dBFS ({opening - ref:+.1f} dB vs ending), '
                  f'{hits[:5].mean():.1f} hits/s', 'levels')
    _said(module, f'build (30-60 s vs 0-30 s): {build:+.1f} dB', 'levels')
    _said(module, f'excluded): {spread:.1f} dB', 'levels')
    _said(module, f'dips (> 3 dB below the ending): {len(dips)}', 'levels')
    passed = bool(opening >= ref - 1.5 and len(dips) <= 1 and spread <= 5.0 and build <= 1.5)
    assert passed == ok
    return {'pass': bool(ok), 'printed': lines, 'module_output': module, 'needs_frames': 0, 'window_s': LV.WIN,
            'rows': [dict(t=float(t), dbfs=float(v), hits_per_s=float(h)) for t, v, h in rows],
            'ending': float(ref), 'opening': float(opening), 'opening_minus_ending': float(opening - ref),
            'opening_hits_per_s': float(hits[:5].mean()), 'build': float(build), 'spread': float(spread),
            'dips': [dict(t=float(t), dbfs=float(v), below_ending=float(v - ref)) for t, v in dips],
            'targets': dict(opening_within=1.5, dip_below=3.0, dips_allowed=1, spread_max=5.0, build_max=1.5),
            'how': 'rows: reelkit.levels.profile on the mono mix of audio.wav (mean of the channels / 32768): the RMS '
                   'in dBFS of each 2 s window and the rough hit rate. There are round(61.013 / 2) = 31 windows; the '
                   'last one (60-62 s) holds only the 1.013 s that are left. ending = mean of the last 10 windows, '
                   'opening = mean of the first 5, build = mean of windows 15.. minus mean of windows 0..14, spread = '
                   'loudest minus quietest window with the one allowed dip left out'}


def band(KC, M, check_dir):
    """kit.checks.band(), then the same numbers unrounded (its lines only print one decimal)"""
    import numpy as np
    from scipy.io import wavfile
    from scipy.signal import butter, sosfilt
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        ok = KC.band()
    lines = _lines(buf)
    T = M.T

    def stem(k):
        sr, x = wavfile.read(os.path.join(check_dir, f'stem_{k}.wav'))
        x = x.astype(float)
        x = x.mean(1) if x.ndim > 1 else x
        return sr, x / 32768

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
    bed_buses = [k for k in list(M.bed.buses) + ['fx'] if os.path.exists(os.path.join(check_dir, f'stem_{k}.wav'))]
    fighters, good = [], True
    for s, line in zip(M.SOUNDS, lines):
        lo, hi = s.band
        sr, x = stem(s.bus)
        f = bp(x, sr, lo, hi)
        bedx = sum(bp(stem(k)[1], sr, lo, hi) for k in bed_buses)
        vals = []
        for a, b in spans:
            sl = slice(int(a * sr), int(b * sr))
            vals.append((db(f[sl]) - db(bedx[sl]), gated(f[sl], bedx[sl], sr)))
        good &= all(g > 0 for _, g in vals if not math.isnan(g))
        mine = (f'band: {s.name:9s} {lo}-{hi} Hz over the bed (dB), section / while it sounds: '
                + ' '.join(f'{v:+5.1f}/{g:+5.1f}' for v, g in vals))
        assert mine.rstrip() == line, (mine, line)
        assert not any(math.isnan(g) for _, g in vals), 'a fighter is silent for a whole section'
        fighters.append(dict(name=s.name, bus=s.bus, band=[lo, hi], section=[float(v) for v, _ in vals],
                             while_it_sounds=[float(g) for _, g in vals]))
    assert bool(good) == bool(ok)
    return {'pass': bool(ok), 'printed': lines, 'needs_frames': 0,
            'spans': [[float(a), float(b)] for a, b in spans], 'bed_buses': bed_buses, 'fighters': fighters,
            'how': 'on the 16-bit stem wavs (mono mix / 32768), each band-passed with butter(4, band, "band", '
                   'fs=48000) as sections: section = dB of the fighter\'s stem minus dB of the bed (the bed buses '
                   'added up) over each span; while_it_sounds = the same over the samples where the fighter\'s 20 ms '
                   'moving RMS is above a tenth of its largest in the span. dB = 20 log10(rms + 1e-9). Passes when '
                   'every while_it_sounds is above 0'}


def loudness(KC, M, wav):
    import numpy as np
    from scipy.io import wavfile
    from scipy.signal import lfilter
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        diff = KC.loudness()
    lines = _lines(buf)
    T = M.T
    sr, x = wavfile.read(wav)
    x = x.astype(float) / 32768.0
    b1, a1 = [1.53512485958697, -2.69169618940638, 1.19839281085285], [1.0, -1.69065929318241, 0.73248077421585]
    b2, a2 = [1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621]
    y = lfilter(b2, a2, lfilter(b1, a1, x, axis=0), axis=0)

    def lufs(z):
        return -0.691 + 10 * np.log10((z ** 2).mean(0).sum() + 1e-12)
    fight = lufs(y[int(T.T_FIGHT * sr):int(M.T_KO * sr)])
    intro = lufs(y[:int(T.T_FIGHT * sr)])
    assert float(intro - fight) == float(diff)
    _said(lines, f'intro (title card + count-in) {intro:.1f} LUFS vs the fight {fight:.1f}: {intro - fight:+.1f} LU',
          'loudness')
    return {'printed': lines, 'needs_frames': 0, 'intro_lufs': float(intro), 'fight_lufs': float(fight),
            'intro_minus_fight': float(diff),
            'spans': dict(intro=[0, int(T.T_FIGHT * sr)], fight=[int(T.T_FIGHT * sr), int(M.T_KO * sr)]),
            'how': 'audio.wav / 32768 through the two BS.1770 K-weighting filters (48 kHz coefficients, in '
                   'kit/checks.py), then -0.691 + 10 log10(sum over the channels of the mean square + 1e-12) over '
                   'the samples of each span (first and one-past-last sample in `spans`). The kit prints the '
                   'difference and returns it; it sets no pass mark'}


class _ForkShim:
    """stands in for `multiprocessing` inside kit.checks.flash: get_context('fork').Pool(n).map(_lum, frames)
    becomes the luminance of frames drawn in this module's spawn pool"""

    def __init__(self, frames):
        self.frames, self.asked = frames, []

    def get_context(self, method):
        return self

    def Pool(self, procs):
        return self

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        return False

    def map(self, fn, numbers):
        from reelkit.flashcheck import rel_lum
        numbers = list(numbers)
        self.asked = numbers
        return [rel_lum(a) for a, _ in self.frames.need(numbers)]      # kit.checks._lum(f) = rel_lum(P.render(f))


def flash(KC, P, frames):
    import numpy as np
    import reelkit.flashcheck as FC
    shim = _ForkShim(frames)
    kept = {}
    analyse0 = FC.analyse

    def analyse(Y, *a, **k):
        kept['Y'] = Y
        kept['out'] = analyse0(Y, *a, **k)
        return kept['out']
    buf = io.StringIO()
    with swapped(KC, 'mp', shim), swapped(FC, 'analyse', analyse), contextlib.redirect_stdout(buf):
        ok = KC.flash()
    lines = _lines(buf)
    area, flashes, worst = kept['out']
    f0, f1 = shim.asked[0], shim.asked[-1] + 1
    assert shim.asked == list(range(f0, f1)) and kept['Y'].dtype == np.float32
    i = int(np.argmax(area))
    lows = [t for t, w, n in P.STORY['firsts'] if n in [P.CFG[k]['abilities']['low']['name'] for k in (0, 1)]]
    t0 = max(P.T.T_FIGHT, min(lows + [P.T_KO - 8]) - 1.0)
    t1 = min(P.T.DUR, P.T_KO + 1.5)
    assert (int(t0 * P.T.FPS), int(t1 * P.T.FPS)) == (f0, f1)
    _said(lines, f'flash: {t0:.1f}-{t1:.1f} s, {f1 - f0} frames: general flashes {len(flashes)}, worst in any 1 s '
                 f'{worst}; largest share of frame changing >= 0.1: {100 * area[i]:.1f} % at '
                 f'{(f0 + i) / P.T.FPS:.2f} s', 'flash')
    assert ok == (worst <= 3)
    return {'pass': bool(ok), 'printed': lines, 'needs_frames': f1 - f0, 't0': float(t0), 't1': float(t1),
            'f0': f0, 'f1': f1, 'low_moves_first_used': [float(t) for t in lows],
            'general_flashes': len(flashes), 'flash_frames': [int(f0 + j) for j in flashes], 'worst_in_1s': int(worst),
            'largest_area': float(area[i]), 'largest_area_frame': f0 + i, 'largest_area_t': (f0 + i) / P.T.FPS,
            'area': [float(v) for v in area],
            'how': 'the stretch runs from 1 s before the first desperation move (or 8 s before the K.O. if that is '
                   'earlier; never before FIGHT!) to 1.5 s after the K.O.: frames f0 .. f1 - 1. Each frame\'s '
                   'relative luminance (reelkit.flashcheck.rel_lum, float32) goes to reelkit.flashcheck.analyse: '
                   'area[j] is the largest share of the frame whose luminance changed by 0.1 or more against any of '
                   'the 5 frames before frame f0 + j (darker state below 0.8); a transition needs a share of 0.25; '
                   'two opposite transitions make a flash; passes when no 1 s holds more than 3'}


def loop(KC, P, frames, wav):
    import numpy as np
    from scipy.io import wavfile
    from reelkit import loopcheck as LC
    T = P.T
    kept = {}

    def frames_from_render(path, idx, procs=None):
        kept['idx'] = list(idx)
        return [a for a, _ in frames.need(idx)]
    pic0, aud0 = LC.check_picture, LC.check_audio

    def check_picture(tail, head, t_tail0, end=60.0):
        kept['tail'], kept['head'], kept['t0'], kept['end'] = tail, head, t_tail0, end
        kept['picture'] = pic0(tail, head, t_tail0, end)
        return kept['picture']

    def check_audio(path):
        kept['audio'] = aud0(path)
        return kept['audio']
    buf = io.StringIO()
    with swapped(LC, 'frames_from_render', frames_from_render), swapped(LC, 'check_picture', check_picture), \
            swapped(LC, 'check_audio', check_audio), contextlib.redirect_stdout(buf):
        ok = KC.loop()
    lines = _lines(buf)
    nt, nh = int(LC.TAIL_S * T.FPS), int(LC.HEAD_S * T.FPS)
    assert kept['idx'] == list(range(T.NF - nt, T.NF)) + list(range(nh))
    tail, head = kept['tail'], kept['head']
    # the picture numbers behind the lines (reelkit.loopcheck.check_picture's formulas, with its own mad())
    steps = [LC.mad(a, b) for a, b in zip(tail[:-1], tail[1:])] + [LC.mad(a, b) for a, b in zip(head[:-1], head[1:])]
    normal = max(float(np.median(steps)), 0.5)
    seam = LC.mad(tail[-1], head[0])
    ratio = seam / normal
    _said(lines, f'seam: last frame -> frame 0 changes {seam:.2f} vs normal step {normal:.2f} ({ratio:.1f}x, limit '
                 f'2.5x)', 'loop')
    big = [(i, s / normal) for i, s in enumerate(steps) if s / normal > 4]
    d0 = [LC.mad(f, head[0]) for f in tail[:-LC.FPS]]
    # the sound across the seam (reelkit.loopcheck.check_audio's formulas)
    sr, x = wavfile.read(wav)
    x = x.astype(np.float64)
    x = x.mean(1) if x.ndim > 1 else x
    if np.abs(x).max() > 2:
        x /= 32768.0
    n = int(0.02 * sr)

    def lv(s):
        return 20 * np.log10(np.sqrt((s ** 2).mean()) + 1e-9)
    la, lb = lv(x[-n:]), lv(x[:n])
    step = abs(la - lb)
    typical = np.median(np.abs(np.diff(x[:sr])))
    jump = abs(x[-1] - x[0])
    _said(lines, f'audio level across the seam: last 20 ms {la:.1f} dBFS vs first 20 ms {lb:.1f} ({step:.1f} dB, '
                 f'limit 6)', 'loop')
    _said(lines, f'audio click: sample jump {jump:.3f} at the seam (typical step {typical:.4f})', 'loop')
    fails = kept['picture'][0] + kept['audio'][0]
    assert ok == (not fails)
    return {'pass': bool(ok), 'printed': lines, 'needs_frames': nt + nh,
            'frames': dict(tail=[T.NF - nt, T.NF], head=[0, nh]),
            'picture': dict(fails=int(kept['picture'][0]), seam=float(seam), normal=float(normal),
                            ratio=float(ratio), steps=[float(s) for s in steps],
                            big_jumps=[dict(step=int(i), ratio=float(q)) for i, q in big],
                            nearest_to_frame_0=dict(frame=T.NF - nt + int(np.argmin(d0)), mad=float(min(d0)))),
            'audio': dict(fails=int(kept['audio'][0]), last_20ms_db=float(la), first_20ms_db=float(lb),
                          step_db=float(step), typical_step=float(typical), jump=float(jump)),
            'how': 'picture: mad(a, b) = mean absolute RGB difference of two frames (int16 subtraction). steps = '
                   'the mad of each frame to the next over the last 150 frames, then over the first 30; normal = '
                   'max(median(steps), 0.5); seam = mad(last frame, frame 0); fails when seam / normal is above 2.5, '
                   'when a one-off step is above 6x normal, or when a frame of the last 5 s (its final second left '
                   'out) is nearer to frame 0 than half the seam and below 3.0 while the seam ratio is above 1.5. '
                   'audio: the mono mix; dB of the last and first 20 ms (20 log10(rms + 1e-9)); a click when the '
                   'jump from the last to the first sample is above max(0.08, 12 x the median sample step of the '
                   'first second)'}


# ---------------------------------------------------------------- the checks the file must pass
def check():
    """what the checks export must leave on disk: every check is there with the kit's lines and its verdict, and the
    counts the brief names can be read from it"""
    assert os.path.isfile(OUT), f'{common.rel(OUT)}: missing'
    with open(OUT, encoding='utf-8') as f:
        doc = json.load(f)
    c = doc['checks']
    assert sorted(c) == sorted(CHECKS), sorted(c)
    for name in CHECKS:
        assert c[name]['printed'] and isinstance(c[name]['needs_frames'], int), name
        assert name == 'loudness' or isinstance(c[name]['pass'], bool), name
    assert len(c['levels']['rows']) == round(ST.NA / ST.SR / c['levels']['window_s']) == 31
    assert c['levels']['module_output'][-1] in ('PASS', 'FAIL')
    assert len(c['band']['fighters']) == 2 and all(len(f['section']) == 5 for f in c['band']['fighters'])
    assert len(c['flash']['area']) == c['flash']['f1'] - c['flash']['f0'] == c['flash']['needs_frames']
    assert len(c['loop']['picture']['steps']) == 150 - 1 + 30 - 1 and c['loop']['needs_frames'] == 180
    assert len(c['safezone']['boxes']) == c['safezone']['frames_checked'] == c['safezone']['needs_frames']
    assert c['safezone']['tracked'] == sum(len(b['boxes']) for b in c['safezone']['boxes'])
    assert c['focus']['frames'] == doc['episode_facts']['frames']
    assert doc['frames_drawn'] <= sum(c[n]['needs_frames'] for n in CHECKS)
    return {name: c[name].get('pass') for name in CHECKS}


# ---------------------------------------------------------------- the export
def child():
    from . import manifest
    if not os.path.isfile(ST.WAV):
        raise SystemExit(f'checks: {common.rel(ST.WAV)} is missing: run the stems export first')
    t_start = time.time()
    M, B, D = V.load(TAG)
    with contextlib.redirect_stdout(io.StringIO()):
        wav = M.main()                                       # audio.wav and check/stem_<bus>.wav, in the cache folder
    with open(wav, 'rb') as f, open(ST.WAV, 'rb') as g:
        sha = hashlib.sha256(f.read()).hexdigest()
        assert sha == hashlib.sha256(g.read()).hexdigest(), 'this soundtrack is not the committed audio.wav'
    from ballfight.kit import checks as KC
    from ballfight.kit import picture as P
    T = P.T
    out = {}
    out['focus'] = focus(KC)
    out['levels'] = levels(KC, wav)
    out['band'] = band(KC, M, M.EP.check)
    out['loudness'] = loudness(KC, M, wav)
    P.render(0)                                              # the title card's sprite cache, before the workers start
    t_pool = time.time()
    with mp.get_context('spawn').Pool(WORKERS, initializer=_init) as pool:
        frames = Frames(pool)
        out['safezone'] = safezone(KC, P, frames)
        out['flash'] = flash(KC, P, frames)
        out['loop'] = loop(KC, P, frames, wav)
        same = frames.need([0])[0][0]
    import numpy as np
    assert np.array_equal(same, P.render(0)), 'frame 0 drawn in a worker is not frame 0 drawn here'
    doc = dict(
        about=f'{EPISODE}: what each check of kit/checks.py gives for the episode: the kit\'s verdict (`pass`), the '
              f'lines it prints (`printed`), the values behind them unrounded, and how many drawn frames it needs',
        tool='checks', format=1, episode=EPISODE,
        source=dict(kind='python', commit=common.kit_commit()),
        episode_facts=dict(seed=M.EP.seed, names=list(M.NAMES), winner=int(P.WIN), t_ko=float(P.T_KO),
                           t_fight=float(T.T_FIGHT), fps=T.FPS, frames=T.NF, audio_wav_sha256=sha),
        numbers='plain JSON numbers are the kit\'s float64 values (shortest repr, exact); times in seconds, levels '
                'in dB',
        needs_frames={name: out[name]['needs_frames'] for name in CHECKS},
        frames_drawn=len(frames.got),
        checks={name: out[name] for name in CHECKS})
    common.write_json(OUT, doc, register=False)
    got = check()
    manifest.add(OUT)
    print(f'checks: {common.rel(OUT)}  {os.path.getsize(OUT)} bytes')
    for name in CHECKS:
        verdict = {True: 'PASS', False: 'FAIL', None: ''}[out[name].get('pass')]
        print(f"checks: {name:9s} {verdict:4s} needs {out[name]['needs_frames']:3d} drawn frames | "
              + ' | '.join(out[name]['printed'][-2:])[:400])
    print(f'checks: {len(frames.got)} frames drawn in {frames.seconds:.0f} s by {WORKERS} workers (pool up '
          f'{time.time() - t_pool:.0f} s); the whole export {time.time() - t_start:.0f} s; verdicts {got}')
    return 0


def run():
    fresh('checks')


EXPORTS = {'checks': run}
ORDER = {'checks': 60}                                       # after stems (55): it needs the committed audio.wav
