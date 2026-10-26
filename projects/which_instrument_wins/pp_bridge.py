"""Bridge to the Pixel Pitch series (series/instrument-history): the 16 entrants' modern low-poly models and
their synth voices, used read-only. Nothing in the series folder is edited; its modules are imported as libraries
(the same way the series' own scripts import each other).

    from pp_bridge import ENTRANTS, model_faces, voice
    faces = model_faces('TUBA')          # [(P (k,3), n (3,), rgb, gid)] normalised to fit a unit sphere
    x = voice('TUBA', 45, 0.8, 0.25)     # one note at 48 kHz (float64), from the tuba video's own synth
"""
import math
import os
import sys
import contextlib
import io
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
PP = os.path.abspath(os.path.join(HERE, '..', '..', 'series', 'instrument-history'))
if PP not in sys.path:
    sys.path.insert(0, PP)

PP_SR = 44100
OUT_SR = 48000

# (roster name, Pixel Pitch module, scene key or None = the module's modern showcase scene, register (low, high) midi,
#  model yaw offset in degrees so the instrument faces the camera at rest, sprite size factor)
ENTRANTS = [
    ('PIANO', 'piano_lowpoly', 'grand', (48, 84), 0, 1.0),
    ('VIOLIN', 'violin_lowpoly', 'modern', (62, 88), 0, 1.0),
    ('CELLO', 'cello60_lowpoly', None, (40, 64), 0, 1.0),
    ('HARP', 'harp60_lowpoly', None, (50, 84), 0, 1.0),
    ('FLUTE', 'flute60_lowpoly', None, (72, 93), 0, 1.0),
    ('CLARINET', 'clarinet60_lowpoly', None, (55, 79), 0, 1.0),
    ('SAX', 'altosax60_lowpoly', None, (56, 78), 0, 1.0),
    ('TRUMPET', 'trumpet_lowpoly', None, (60, 82), 0, 1.0),
    ('TROMBONE', 'trombone60_lowpoly', None, (45, 67), 0, 1.0),
    ('TUBA', 'tuba60_lowpoly', None, (33, 55), 0, 1.0),
    ('HORN', 'horn60_lowpoly', None, (48, 70), 0, 1.0),
    ('GUITAR', 'electric60_lowpoly', None, (52, 76), 0, 1.0),
    ('DRUMS', 'drums60_lowpoly', None, (36, 50), 0, 1.0),
    ('ACCORDION', 'accordion60_lowpoly', None, (55, 79), 0, 1.0),
    ('BAGPIPES', 'bagpipes60_lowpoly', None, (67, 81), 0, 1.0),
    ('UKULELE', 'ukulele60_lowpoly', None, (60, 81), 0, 1.0),
]
NAMES = [e[0] for e in ENTRANTS]        # #26's 16-entrant roster: keep it at 16 (its run, roster and sprites use it)
# Instruments used only by later projects (BALLFIGHT episodes): same fields, reachable through entrant() / voice() /
# model_faces(), but NOT part of #26's roster. Append new instruments here, not to ENTRANTS.
EXTRA = [
    ('ACOUSTIC', 'acoustic_lowpoly', 'steel', (40, 76), 0, 1.0),       # #33 BALLFIGHT: the modern steel-string
    ('TURNTABLES', 'turntables60_lowpoly', None, (50, 74), 0, 1.0),    # #31 BALLFIGHT: the modern deck (organ stabs; midi < 12 = scratches)
    ('SYNTH', 'synth60_lowpoly', None, (57, 81), 0, 1.0),              # #31 BALLFIGHT: the modern poly synth
    ('ORGAN', 'organ60_lowpoly', 'sulp', (36, 84), 0, 1.0),            # #35 BALLFIGHT: Saint-Sulpice case + reeds
    ('OBOE', 'oboe_lowpoly', 'ball', (58, 91), 0, 1.0),                # #37 BALLFIGHT: modern oboe (radii x1.6 for sprites) + double-reed voice
]
DRUM_KEYS = (36, 38, 41, 45, 48, 49)    # kick, snare, floor / low / high tom, crash
_MODS = {}


def _quiet_import(name):
    with contextlib.redirect_stdout(io.StringIO()):
        return __import__(name)


def module(name):
    """import a Pixel Pitch module once; framework modules get pp_framework.setup() so their scenes have events"""
    if name not in _MODS:
        m = _quiet_import(name)
        if hasattr(m, 'SCENES') and hasattr(m, 'synth') and not getattr(m, '_pp_setup', False):
            import pp_framework as F
            with contextlib.redirect_stdout(io.StringIO()):
                F.setup(m)
            m._pp_setup = True
        _MODS[name] = m
    return _MODS[name]


def entrant(name):
    for e in ENTRANTS + EXTRA:
        if e[0] == name:
            return e
    raise KeyError(name)


def scene(name):
    _, modname, key, *_ = entrant(name)
    m = module(modname)
    if key is None:
        return m.SCENES[0]
    return next(s for s in m.SCENES if s['key'] == key)


@contextlib.contextmanager
def _no_stands():
    """while building, skip the display stands and floor discs (the models stand on pedestals in their videos; inside a
    ball only the instrument should show). Runtime patches only: no series file is edited."""
    import piano_lowpoly as P
    import pp_shapes as S
    patched = []

    def patch(obj, attr, fn):
        if hasattr(obj, attr):
            patched.append((obj, attr, getattr(obj, attr)))
            setattr(obj, attr, fn)

    noop = lambda *a, **k: None
    for modname, fn in (('violin_lowpoly', 'stand'), ('trumpet_lowpoly', 'trumpet_stand'), ('pp_guitar', 'stand'),
                        ('ukulele60_lowpoly', 'stand'), ('altosax60_lowpoly', 'sax_stand')):
        patch(module(modname), fn, noop)
    rod0 = S.rod

    def rod(g, a, b, r=.009, col=(48, 46, 54)):
        if a[1] < 0.012 or b[1] < 0.012 or tuple(col) == (48, 46, 54):
            return None                                   # poles that start on the floor / plain stand rods
        return rod0(g, a, b, r, col)
    patch(S, 'rod', rod)
    ngon0 = P.Geo.ngon

    def ngon(self, cx, y, cz, r, col, n=8, **kw):
        if y < 0.012:
            return None                                   # floor discs under stands
        return ngon0(self, cx, y, cz, r, col, n, **kw)
    patch(P.Geo, 'ngon', ngon)
    try:
        yield
    finally:
        for obj, attr, val in reversed(patched):
            setattr(obj, attr, val)


def build_geo(name, t=1.0, stands=False):
    """the instrument's Geo as its video builds it (resting, no notes playing), by default without its display stand"""
    import piano_lowpoly as P
    _, modname, key, *_ = entrant(name)
    m = module(modname)
    s = scene(name)
    g = P.Geo()
    fx = []
    with (contextlib.nullcontext() if stands else _no_stands()):
        if modname in ('piano_lowpoly', 'violin_lowpoly'):
            m.build_inst(s['key'], g, t, [], fx, s)
        else:
            m.build(g, t, [], fx, s)
    return g


def model_faces(name):
    """faces of the modern model, centred on its bounding box and scaled to fit a unit sphere.
    Returns [(P (k,3), n (3,), rgb (3,), gid, spec)]; lines are dropped (strings read as noise at ball size)."""
    g = build_geo(name)
    _, _, _, _, yaw, size = entrant(name)
    V = np.concatenate([f[0] for f in g.F])
    lo, hi = V.min(0), V.max(0)
    c = (lo + hi) / 2
    r = np.linalg.norm(V - c, axis=1).max()
    a = math.radians(yaw)
    R = np.array([[math.cos(a), 0, math.sin(a)], [0, 1, 0], [-math.sin(a), 0, math.cos(a)]])
    out = []
    for P_, n, col, two, emit, gid, spec in g.F:
        Q = ((P_ - c) / r * size) @ R.T
        out.append((Q, R @ n, np.array(col, float), bool(two), bool(emit), int(gid), float(spec)))
    return out


def _resample(x):
    from scipy.signal import resample_poly
    return resample_poly(x, 160, 147)          # 44.1 kHz -> 48 kHz


def voice(name, midi, vel, dur):
    """one note in the entrant's own Pixel Pitch voice, resampled to 48 kHz.
    dur in seconds (the note's held length; tails ring past it as the synth makes them)."""
    _, modname, key, *_ = entrant(name)
    m = module(modname)
    s = scene(name)
    if modname == 'piano_lowpoly':
        x = m.inst_note('grand', midi, vel, 0.0, 3)
    elif modname == 'violin_lowpoly':
        x = m.bowed(m.midi_hz(midi), dur, vel, 'modern')
    else:
        if modname == 'drums60_lowpoly':                 # the kit's synth is keyed by drum (GM numbers), not pitch
            midi = min(DRUM_KEYS, key=lambda k: abs(k - midi))
        fake = dict(s)
        fake['ev'] = [(0.0, midi, vel, dur, True, 1)]
        if isinstance(getattr(m, '_NOTE', None), dict):   # organ / synth / turntables cache notes by (scene t0, event
            m._NOTE.clear()                               # index): with one fake event every call would return the first
        with contextlib.redirect_stdout(io.StringIO()):
            x = m.synth(fake, 0)
    return _resample(np.asarray(x, float))


if __name__ == '__main__':
    import time
    for nm in NAMES:
        t0 = time.time()
        f = model_faces(nm)
        t1 = time.time()
        lo, hi = entrant(nm)[3]
        x = voice(nm, (lo + hi) // 2, 0.8, 0.2)
        t2 = time.time()
        print(f'{nm:10s} key={scene(nm)["key"]:12s} faces {len(f):4d}  build {1000 * (t1 - t0):5.0f} ms  '
              f'note {len(x) / OUT_SR:4.2f} s peak {np.abs(x).max():.3f} rms {np.sqrt((x ** 2).mean()):.3f}  {1000 * (t2 - t1):4.0f} ms')
