"""dsp: the signal-processing vectors the sound port is checked against, and what the episode's soundtrack asks of
numpy, scipy and reelkit.audio.

studio/reference/dsp/ (every .npz has a sidecar .json: vectors-convention.md):

    butter.npz        every filter design asked for while the example episode's soundtrack is built and its audio
                      checks run: scipy.signal.butter's second-order sections (reelkit.audio.sos, kit/checks.py band)
                      and its b, a form (the Pixel Pitch series' own lp)
    filters.npz       sosfilt and lfilter on a fixed 4,800-sample noise input, for 14 of those designs and for the
                      kit's other lfilter uses (the master's envelope follower, the K-weighting of the loudness check)
    resample.npz      scipy.signal.resample_poly(x, 160, 147) (44.1 kHz -> 48 kHz), and the FIR it designs
    fftconvolve.npz   scipy.signal.fftconvolve on two input pairs
    numpy.npz         np.interp, np.percentile(x, 99.97), np.convolve(..., 'same'), np.cumsum, np.linspace, the
                      sums and means the sound code takes (numpy adds a long 1-D array pairwise), the 16-bit
                      conversion, and scipy.signal.find_peaks as the levels check calls it
    ufuncs.npz        np.sin, np.cos, np.exp, np.tanh, np.log10, np.sqrt and the powers the sound code takes, and
                      the Python-scalar maths it uses (math.cos, math.sin, math.exp, float ** float), on fixed inputs
    generators.npz    every reelkit.audio generator at its default arguments and at each argument set the episode
                      uses (kick, snare, clap, hat, crash, whoosh, thud, pluck, pad, bass, chime, reverb, tape_stop,
                      saw, square, tri, adsr, expenv)

What the episode asks for is learned by wrapping the functions at run time while kit.music.main() and the audio checks
run (nothing in the kit is edited); the vectors are then made by calling the real, unwrapped functions. The work runs
in its own interpreter (`fresh`), because building the soundtrack advances the Pixel Pitch voices' shared noise
generator and must not depend on what ran before.

This module also holds what the other sound exports share: `Cases` (one vector file and its sidecar) and `fresh`."""
import contextlib
import functools
import inspect
import io
import json
import math
import os
import subprocess
import sys

from . import common

EPISODE = 'flute_vs_trombone'
TAG = 'dsp'
DIR = common.ref('dsp')
SR = 48000
N_FIXED = 4800                                             # the fixed noise input of the filter cases: 100 ms
GENERATORS = ('kick', 'snare', 'clap', 'hat', 'crash', 'whoosh', 'thud', 'pluck', 'pad', 'bass', 'chime', 'reverb',
              'tape_stop', 'saw', 'square', 'tri', 'adsr', 'expenv')
STAND_IN = {                                               # values for the arguments a generator has no default for
    'whoosh': dict(dur=0.6), 'pluck': dict(m=60), 'pad': dict(ms=(60, 64, 67), dur=1.0), 'bass': dict(m=40, dur=0.5),
    'chime': dict(m=84), 'saw': dict(f=220.0, n=N_FIXED), 'square': dict(f=220.0, n=N_FIXED),
    'tri': dict(f=220.0, n=N_FIXED), 'adsr': dict(n=N_FIXED), 'expenv': dict(n=N_FIXED, decay=0.1)}
SEEDS_PER_SHAPE = 2                                        # vectors per argument set that differs only in `seed`
FILTER_FRAMES = ('sos', 'hp', 'lp', 'bp')                  # frames to look past when naming who asked for a filter
# the 12 second-order-section designs filtered on the fixed noise: (btype, cutoffs in Hz, order), all asked for by the
# episode (the export stops if one is not); two more are taken from the episode's data-dependent requests
SOS_FIXED = (('hp', (46,), 2), ('hp', (4000,), 2), ('hp', (4200,), 2), ('lp', (160,), 2), ('lp', (700,), 2),
             ('lp', (2500,), 2), ('lp', (5000,), 2), ('bp', (1500, 8000), 2), ('bp', (1200, 5000), 2),
             ('bp', (400, 5000), 2), ('band', (600, 3200), 4), ('band', (70, 700), 4))


# ---------------------------------------------------------------- a fresh interpreter
def fresh(module, function='child'):
    """run tools.studio_export.<module>.<function>() in a new interpreter with this one's environment, from the repo
    root. Everything that touches the Pixel Pitch voices' shared noise generator is exported this way, so no result
    depends on what ran earlier in the calling process"""
    code = ('import sys\n'
            'from tools.studio_export.__main__ import check_environment\n'
            'check_environment()\n'
            f'from tools.studio_export import {module} as m\n'
            f'sys.exit(m.{function}() or 0)\n')
    sys.stdout.flush()
    r = subprocess.run([sys.executable, '-c', code], cwd=common.REPO)
    if r.returncode:
        raise SystemExit(f'{module}.{function}() failed in its own interpreter (exit code {r.returncode})')


# ---------------------------------------------------------------- one vector file and its sidecar
def jarg(v):
    """an argument as JSON: a float as its float.hex() string (every bit), an int, a bool, a string or None as it
    is, a sequence as a list, an array as its shape only"""
    import numpy as np
    if v is None or isinstance(v, (bool, str)):
        return v
    if isinstance(v, np.bool_):
        return bool(v)
    if isinstance(v, (int, np.integer)):
        return int(v)
    if isinstance(v, (float, np.floating)):
        return common.hexf(float(v))
    if isinstance(v, (list, tuple)):
        return [jarg(x) for x in v]
    if isinstance(v, np.ndarray):
        return dict(array=list(v.shape))
    if isinstance(v, dict):
        return {str(k): jarg(x) for k, x in v.items()}
    raise TypeError(f'{type(v).__name__} has no argument form')


def call_text(name, args):
    """the call as Python would write it (floats by their repr), for a reader; the exact values are in `args`"""
    import numpy as np

    def show(v):
        if isinstance(v, np.ndarray):
            return f'<array {v.shape}>'
        if isinstance(v, np.generic):
            return repr(v.item())
        if isinstance(v, tuple):
            return '(' + ', '.join(show(x) for x in v) + (',)' if len(v) == 1 else ')')
        if isinstance(v, list):
            return '[' + ', '.join(show(x) for x in v) + ']'
        return repr(v)
    return f"{name}({', '.join(f'{k}={show(v)}' for k, v in args.items())})"


class Cases:
    """one vector file: an .npz of plain numeric arrays and its sidecar .json (vectors-convention.md).

        c = Cases(path, about, tool='dsp', module=__name__)
        c.shared('noise', x)                              a member several cases use
        c.add(call, args, {'x': 'noise', 'out': y})       a case: a role names a new array or a shared member
        c.write()                                         both files, written and read back
        c.register()                                      listed in the manifest (once the export's checks passed)

    Arrays are stored as they are given (write_npz refuses anything but the convention's dtypes), so the dtype in the
    file is the one the kit produced"""

    def __init__(self, path, about, tool, module, **top):
        if not path.endswith('.npz'):
            raise ValueError(f'{path}: a vector file is named .npz')
        self.path, self.sidecar = path, path[:-4] + '.json'
        self.module = module
        self.top = dict(about=about, tool=tool, **top)
        self.cases, self.arrays = [], {}

    def shared(self, name, a):
        import numpy as np
        if name in self.arrays or '__' in name:
            raise ValueError(f'{name}: taken, or looks like a case member')
        self.arrays[name] = np.ascontiguousarray(a)
        return name

    def add(self, call, args, arrays, **more):
        import numpy as np
        cid = f'c{len(self.cases):03d}'
        names = {}
        for role, a in arrays.items():
            if isinstance(a, str):
                if a not in self.arrays:
                    raise KeyError(f'{cid}: no shared member {a!r}')
                names[role] = a
            else:
                if not isinstance(a, np.ndarray) or a.ndim < 1:
                    raise TypeError(f'{cid}.{role}: a numpy array of at least one dimension, not {type(a).__name__}')
                names[role] = f'{cid}__{role}'
                self.arrays[names[role]] = np.ascontiguousarray(a)
        self.cases.append(dict(id=cid, call=call, args=args, arrays=names, **more))
        return cid

    def write(self):
        doc = dict(self.top, cases=self.cases)
        common.write_npz(self.path, self.arrays, register=False)
        common.write_json(self.sidecar, doc, register=False)
        check_case_file(self.path, expect=self.arrays)
        return self

    def register(self):
        from . import manifest
        manifest.add(self.path, tool=self.module)
        manifest.add(self.sidecar, tool=self.module)
        print(f"{self.top['tool']}: {common.rel(self.path)}  {os.path.getsize(self.path)} bytes  "
              f'{len(self.cases)} cases, {len(self.arrays)} arrays')
        return self.path


def check_case_file(path, expect=None):
    """what every vector file must satisfy, read from disk: each member loads with allow_pickle=False in one of the
    convention's dtypes; the sidecar names every member in some case and no member that is missing; case ids are
    unique; and (with `expect`) each member is bit for bit the array that was meant to be written.
    Returns (sidecar, {member: array})"""
    import numpy as np
    sidecar = path[:-4] + '.json'
    for p in (path, sidecar):
        assert os.path.isfile(p), f'{common.rel(p)}: missing'
    with np.load(path, allow_pickle=False) as z:
        got = {n: z[n] for n in z.files}
    with open(sidecar, encoding='utf-8') as f:
        doc = json.load(f)
    for k in ('about', 'tool', 'cases'):
        assert k in doc, f'{common.rel(sidecar)}: no "{k}"'
    ids = [c['id'] for c in doc['cases']]
    assert len(ids) == len(set(ids)) and ids, f'{common.rel(sidecar)}: case ids are not unique'
    used = set()
    for c in doc['cases']:
        for k in ('id', 'call', 'args', 'arrays'):
            assert k in c, f"{common.rel(sidecar)}: case {c.get('id')} has no \"{k}\""
        for role, member in c['arrays'].items():
            assert member in got, f"{common.rel(sidecar)}: {c['id']}.{role} names {member}, which is not in the npz"
            used.add(member)
    assert used == set(got), f'{common.rel(path)}: members no case names: {sorted(set(got) - used)}'
    for n, a in got.items():
        assert a.dtype.str in common.NPZ_DTYPES, f'{common.rel(path)}: {n} has dtype {a.dtype.str}'
        assert np.isfinite(a).all() if a.dtype.kind == 'f' else True, f'{common.rel(path)}: {n} is not finite'
    if expect is not None:
        assert set(expect) == set(got), sorted(set(expect) ^ set(got))
        for n, a in expect.items():
            b = got[n]
            assert a.dtype.str == b.dtype.str and a.shape == b.shape and a.tobytes() == b.tobytes(), \
                f'{common.rel(path)}: {n} did not read back as written'
    return doc, got


# ---------------------------------------------------------------- learning what the episode asks for
def _canon(v):
    """a hashable form of an argument value; numbers compare by value (120 and 120.0 are one argument)"""
    import numpy as np
    if isinstance(v, np.ndarray):
        return ('array',) + tuple(v.shape)
    if isinstance(v, np.generic):
        return v.item()
    if isinstance(v, (list, tuple)):
        return tuple(_canon(x) for x in v)
    return v


def _plain(v):
    """an argument value to keep: numpy scalars as Python numbers, sequences as tuples, an array as None"""
    import numpy as np
    if isinstance(v, np.ndarray):
        return None
    if isinstance(v, np.generic):
        return v.item()
    if isinstance(v, (list, tuple)):
        return tuple(_plain(x) for x in v)
    return v


def _who(frame, skip=()):
    """'module function' of the nearest caller that is not one of reelkit.audio's (or the series') filter helpers;
    a function's inner functions and comprehensions count as the function"""
    f = frame
    while f is not None:
        mod, name = f.f_globals.get('__name__', '?'), f.f_code.co_qualname
        if not (name.split('.')[-1] in FILTER_FRAMES and (mod in skip or '<locals>' in name)):
            return f"{mod} {name.split('.<locals>')[0]}"
        f = f.f_back
    return '?'


class Collector:
    """wraps scipy's butter (wherever a module has bound it) and reelkit.audio's generators while the soundtrack is
    built, and remembers every distinct request: the argument values, how often, and which kit function asked"""

    def __init__(self, A):
        self.A = A
        self.designs = {}                                  # design key -> dict(args, count, users)
        self.calls = {g: {} for g in GENERATORS}           # generator -> {argument key -> dict(args, count, ...)}
        self.depth = 0
        self.tick = 0
        self._undo = []

    def _patch(self, obj, name, new):
        self._undo.append((obj, name, vars(obj)[name]))
        setattr(obj, name, new)

    def install(self):
        import scipy.signal
        butter0 = vars(scipy.signal)['butter']
        sig_b = inspect.signature(butter0)

        @functools.wraps(butter0)
        def butter(*a, **k):
            b = sig_b.bind(*a, **k)
            b.apply_defaults()
            self._design(b.arguments, sys._getframe(1))
            return butter0(*a, **k)
        for name in sorted(sys.modules):                   # reelkit.audio, drums60_lowpoly, scipy.signal itself, ...
            mod = sys.modules[name]
            if mod is not None and vars(mod).get('butter') is butter0:
                self._patch(mod, 'butter', butter)
        for g in GENERATORS:
            self._patch(self.A, g, self._generator(g, vars(self.A)[g]))

    def remove(self):
        for obj, name, old in reversed(self._undo):
            setattr(obj, name, old)
        self._undo = []

    def _design(self, a, frame):
        import numpy as np
        wn = tuple(float(x) for x in np.atleast_1d(np.asarray(a['Wn'], float)))
        fs = None if a['fs'] is None else float(a['fs'])
        key = (a['output'], a['btype'], int(a['N']), wn, fs, bool(a['analog']))
        d = self.designs.setdefault(key, dict(N=int(a['N']), Wn=wn, btype=a['btype'], output=a['output'], fs=fs,
                                              analog=bool(a['analog']), count=0, users=set(), first=self.tick))
        d['count'] += 1
        d['users'].add(_who(frame, skip=('reelkit.audio', 'drums60_lowpoly')))
        self.tick += 1

    def _generator(self, name, fn):
        sig = inspect.signature(fn)

        @functools.wraps(fn)
        def gen(*a, **k):
            b = sig.bind(*a, **k)
            b.apply_defaults()
            key = tuple((p, _canon(v)) for p, v in b.arguments.items())
            rec = self.calls[name].setdefault(key, dict(args={p: _plain(v) for p, v in b.arguments.items()},
                                                        shapes={p: tuple(v.shape) for p, v in b.arguments.items()
                                                                if hasattr(v, 'shape') and getattr(v, 'ndim', 0)},
                                                        count=0, nested=0, users=set(), first=self.tick))
            rec['count'] += 1
            rec['nested'] += self.depth > 0
            rec['users'].add(_who(sys._getframe(1)))
            self.tick += 1
            self.depth += 1
            try:
                return fn(*a, **k)
            finally:
                self.depth -= 1
        return gen


def collect():
    """build the episode's soundtrack (kit.music.main) and run the audio checks that design filters, with the
    Collector in place. Returns (the collector, kit.music, reelkit.audio)"""
    common.episode(EPISODE, TAG)
    from ballfight.kit import music as M
    from ballfight.kit import checks as KC
    from reelkit import audio as A
    import pp_bridge as B
    for name in M.NAMES:                                   # load the two voices' Pixel Pitch modules now: other series
        B.scene(M.CFG[M.NAMES.index(name)]['pp'])          # modules design filters as they are imported (not ours)
    col = Collector(A)
    col.install()
    try:
        with contextlib.redirect_stdout(io.StringIO()):
            M.main()
            KC.band()
            KC.loudness()
    finally:
        col.remove()
    return col, M, A


# ---------------------------------------------------------------- the vector files
def _rng(seed):
    import numpy as np
    return np.random.default_rng(seed)


def _design_call(d):
    wn = d['Wn'][0] if len(d['Wn']) == 1 else list(d['Wn'])
    fs = '' if d['fs'] is None else f", fs={d['fs']:g}"
    return f"scipy.signal.butter({d['N']}, {wn!r}, {d['btype']!r}{fs}, output={d['output']!r})"


def _design_args(d):
    return dict(N=d['N'], Wn=[common.hexf(x) for x in d['Wn']], btype=d['btype'], output=d['output'],
                fs=None if d['fs'] is None else common.hexf(d['fs']))


def _butter(d):
    import scipy.signal
    wn = d['Wn'][0] if len(d['Wn']) == 1 else list(d['Wn'])
    return scipy.signal.butter(d['N'], wn, d['btype'], output=d['output'], fs=d['fs'])


def butter_file(col):
    """every design the episode asked for, in a fixed order (by output form, kind, order, cutoff)"""
    import numpy as np
    c = Cases(os.path.join(DIR, 'butter.npz'), tool='dsp', module=__name__,
              about='scipy.signal.butter for every design asked for while the flute_vs_trombone soundtrack is built '
                    '(kit.music.main) and its band and loudness checks run',
              notes=dict(
                  Wn='the cutoff(s) as float.hex(): one for a low- or high-pass, two for a band-pass; in Hz when fs '
                     'is given, else as a fraction of the Nyquist frequency (the Pixel Pitch series passes f / '
                     '(44100 / 2))',
                  btype="as the caller wrote it: 'hp', 'lp', 'bp' (reelkit.audio.sos) or 'high', 'low', 'band'",
                  sos='output "sos": float64 (sections, 6), each row b0 b1 b2 a0 a1 a2, as scipy orders the sections',
                  ba='output "ba": float64 b and a',
                  requests='how many times the design was asked for',
                  used_by='the kit functions that asked (module, then function)'))
    for key in sorted(col.designs, key=lambda k: (k[0], k[1], k[2], k[3], -1.0 if k[4] is None else k[4])):
        d = col.designs[key]
        assert not d['analog'], d
        out = _butter(d)
        if d['output'] == 'sos':
            arrays = dict(sos=np.asarray(out))
            assert arrays['sos'].dtype == np.float64 and arrays['sos'].shape[1] == 6
        else:
            arrays = dict(b=np.asarray(out[0]), a=np.asarray(out[1]))
        c.add(_design_call(d), _design_args(d), arrays, requests=d['count'], used_by=sorted(d['users']))
    return c.write()


def _find(col, btype, hz, order):
    for key, d in col.designs.items():
        if d['output'] == 'sos' and d['btype'] == btype and d['N'] == order and d['Wn'] == tuple(map(float, hz)):
            return d
    raise SystemExit(f'dsp: the episode no longer asks for butter({order}, {hz}, {btype!r}) as second-order sections; '
                     f'pick another design for SOS_FIXED')


def filters_file(col, M):
    import numpy as np
    from scipy.signal import sosfilt, lfilter
    c = Cases(os.path.join(DIR, 'filters.npz'), tool='dsp', module=__name__,
              about='scipy.signal.sosfilt and lfilter on fixed noise inputs, for designs the episode asks for and '
                    'for the kit\'s other lfilter uses',
              notes=dict(
                  noise=f'np.random.default_rng({N_FIXED}).normal(0, 1, {N_FIXED}): the fixed mono input',
                  noise2=f'np.random.default_rng({N_FIXED + 1}).normal(0, 1, ({N_FIXED}, 2)): the fixed stereo input '
                         f'(filtered along axis 0, each column on its own)',
                  design='the butter call whose output the filter is (the same design is a case of butter.npz)'))
    x = c.shared('noise', _rng(N_FIXED).normal(0, 1, N_FIXED))
    x2 = c.shared('noise2', _rng(N_FIXED + 1).normal(0, 1, (N_FIXED, 2)))
    picks = [_find(col, *f) for f in SOS_FIXED]
    # two designs whose cutoffs come from the fight: the highest high-pass kit.music.colour asked for (the most hurt
    # a fighter sounds) and the first band-pass kit.music.wah asked for
    for user, pick in (('ballfight.kit.music colour', lambda d: -d['Wn'][0]),
                       ('ballfight.kit.music wah', lambda d: d['first'])):
        own = [d for d in col.designs.values() if d['output'] == 'sos' and user in d['users'] and d not in picks]
        if not own:
            raise SystemExit(f'dsp: the episode asks for no filter from {user}')
        picks.append(min(own, key=pick))
    for d in picks:
        sos = np.asarray(_butter(d))
        y = sosfilt(sos, c.arrays[x], axis=0)
        assert y.dtype == np.float64 and y.shape == (N_FIXED,)
        c.add('scipy.signal.sosfilt(sos, noise, axis=0)', dict(design=_design_args(d)),
              dict(sos=sos, x=x, out=y), design=_design_call(d), used_by=sorted(d['users']))
    d = _find(col, 'lp', (5000,), 2)                       # reelkit.audio.reverb filters its stereo impulse response
    sos = np.asarray(_butter(d))
    c.add('scipy.signal.sosfilt(sos, noise2, axis=0)', dict(design=_design_args(d)),
          dict(sos=sos, x=x2, out=sosfilt(sos, c.arrays[x2], axis=0)), design=_design_call(d),
          used_by=['reelkit.audio reverb'])
    # lfilter: the series' own low-pass (the flute's chiff), the master's envelope follower, the K-weighting
    ba = [d for d in col.designs.values() if d['output'] == 'ba']
    assert len(ba) == 1 and ba[0]['btype'] == 'low', [(_design_call(d), sorted(d['users'])) for d in ba]
    b, a = (np.asarray(v) for v in _butter(ba[0]))
    c.add('scipy.signal.lfilter(b, a, noise)', dict(design=_design_args(ba[0])),
          dict(b=b, a=a, x=x, out=lfilter(b, a, c.arrays[x])), design=_design_call(ba[0]),
          used_by=sorted(ba[0]['users']))
    b, a = np.array([0.002]), np.array([1.0, -0.998])
    sq = c.shared('noise_squared', c.arrays[x] ** 2)
    c.add('scipy.signal.lfilter([0.002], [1, -0.998], noise ** 2)', dict(b=jarg(b.tolist()), a=jarg(a.tolist())),
          dict(b=b, a=a, x=sq, out=lfilter([0.002], [1, -0.998], c.arrays[sq])),
          used_by=['ballfight.kit.music master'], note='the compressor\'s envelope follower, on a mean square')
    b1, a1 = [1.53512485958697, -2.69169618940638, 1.19839281085285], [1.0, -1.69065929318241, 0.73248077421585]
    b2, a2 = [1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621]
    y1 = lfilter(b1, a1, c.arrays[x2], axis=0)
    y2 = lfilter(b2, a2, y1, axis=0)
    c.add('scipy.signal.lfilter(b1, a1, noise2, axis=0)', dict(b=jarg(b1), a=jarg(a1)),
          dict(b=np.array(b1), a=np.array(a1), x=x2, out=y1), used_by=['ballfight.kit.checks loudness'],
          note='BS.1770 K-weighting, the first stage (the shelf)')
    c.add('scipy.signal.lfilter(b2, a2, lfilter(b1, a1, noise2, axis=0), axis=0)',
          dict(b1=jarg(b1), a1=jarg(a1), b2=jarg(b2), a2=jarg(a2)),
          dict(b=np.array(b2), a=np.array(a2), x=x2, out=y2), used_by=['ballfight.kit.checks loudness'],
          note='BS.1770 K-weighting, both stages: `b`, `a` are the second stage, applied to the case before')
    return c.write()


def resample_file():
    import numpy as np
    import scipy.signal
    from scipy.signal import _signaltools as ST
    import pp_bridge as B
    c = Cases(os.path.join(DIR, 'resample.npz'), tool='dsp', module=__name__,
              about='scipy.signal.resample_poly(x, 160, 147): how pp_bridge.voice brings a 44,100 Hz Pixel Pitch '
                    'note to 48,000 Hz (scipy\'s defaults: a Kaiser 5.0 window, zero padding)',
              notes=dict(
                  firwin='the low-pass scipy designs inside: firwin(2 * 1600 + 1, 1 / 160, window=("kaiser", 5.0)), '
                         'before it is scaled by up = 160',
                  h='the filter as scipy hands it to upfirdn(h, x, 160, 147): firwin * 160 with n_pre_pad zeros in '
                    'front; the output keeps samples n_pre_remove .. n_pre_remove + n_out of what upfirdn returns',
                  n_out='ceil(len(x) * 160 / 147)'))
    seen = {}
    firwin0, upfirdn0 = ST.firwin, ST.upfirdn

    def firwin(*a, **k):
        h = firwin0(*a, **k)
        seen['firwin'] = (a, k, np.array(h, copy=True))
        return h

    def upfirdn(h, x, up, down, **k):
        y = upfirdn0(h, x, up, down, **k)
        seen['upfirdn'] = (np.array(h, copy=True), up, down, k, y.shape[0])
        return y
    raw = {}
    resample0 = B._resample

    def keep_raw(x):
        raw['x'] = np.array(x, copy=True)
        return resample0(x)
    B._resample = keep_raw
    try:
        B.voice('TROMBONE', 52, 0.9, 0.12)                 # (a trombone note draws no noise: nothing else moves)
    finally:
        B._resample = resample0
    spike = np.zeros(1001)
    spike[500] = 1.0
    inputs = [('noise', _rng(4410).normal(0, 1, 4410), 'np.random.default_rng(4410).normal(0, 1, 4410)'),
              ('spike', spike, 'zeros(1001) with x[500] = 1: the filter itself, every 147th tap'),
              ('trombone', raw['x'], "the 44,100 Hz note behind pp_bridge.voice('TROMBONE', 52, 0.9, 0.12) "
                                     '(trombone60_lowpoly.synth), before it is resampled'),
              ('short', _rng(100).normal(0, 1, 100), 'np.random.default_rng(100).normal(0, 1, 100): shorter than '
                                                     'the filter')]
    for name, x, what in inputs:
        ST.firwin, ST.upfirdn = firwin, upfirdn
        try:
            y = scipy.signal.resample_poly(x, 160, 147)
        finally:
            ST.firwin, ST.upfirdn = firwin0, upfirdn0
        n_out = -(-len(x) * 160 // 147)
        assert y.dtype == np.float64 and y.shape == (n_out,), (name, y.shape, n_out)
        fa, fk, taps = seen['firwin']
        h, up, down, kw, n_full = seen['upfirdn']
        assert fa == (3201, 1 / 160) and fk == dict(window=('kaiser', 5.0)), (fa, fk)
        assert (up, down) == (160, 147) and kw == dict(axis=0, mode='constant', cval=0), (up, down, kw)
        n_pre_pad = len(h) - len(taps)
        assert n_pre_pad == 147 - 1600 % 147 and np.array_equal(h[n_pre_pad:], taps * 160) and not h[:n_pre_pad].any()
        n_pre_remove = (1600 + n_pre_pad) // 147
        if 'firwin' not in c.arrays:
            c.shared('firwin', taps)
            c.shared('h', h)
        else:
            assert np.array_equal(c.arrays['firwin'], taps) and np.array_equal(c.arrays['h'], h)
        c.add('scipy.signal.resample_poly(x, 160, 147)',
              dict(up=160, down=147, n_in=len(x), n_out=n_out, n_pre_pad=n_pre_pad, n_pre_remove=n_pre_remove,
                   upfirdn_len=n_full),
              dict(x=x, out=y, firwin='firwin', h='h'), input=name, what=what)
    return c.write()


def fftconvolve_file(A):
    import numpy as np
    from scipy.signal import fftconvolve
    c = Cases(os.path.join(DIR, 'fftconvolve.npz'), tool='dsp', module=__name__,
              about='scipy.signal.fftconvolve(a, b) (mode "full", 1-D): the convolution of reelkit.audio.reverb',
              notes=dict(out='float64, len(a) + len(b) - 1'))
    r = _rng(1299)
    a1, b1 = r.normal(0, 1, 1000), r.normal(0, 1, 300) * np.exp(-np.arange(300) / 60.0)
    c.add('scipy.signal.fftconvolve(a, b)', dict(n_a=1000, n_b=300), dict(a=a1, b=b1, out=fftconvolve(a1, b1)),
          what='noise against a short decaying noise burst (default_rng(1299): a, then b before its exp(-i / 60))')
    r = _rng(59999)
    a2 = r.normal(0, 1, 12000) * np.exp(-np.arange(12000) / 3000.0)
    b2 = A.lp(r.normal(0, 1, 48000) * np.exp(-A.tt(48000) / 0.55), 5000)
    b2[:int(0.02 * SR)] = 0
    c.add('scipy.signal.fftconvolve(a, b)', dict(n_a=12000, n_b=48000), dict(a=a2, b=b2, out=fftconvolve(a2, b2)),
          what='a decaying burst against one second shaped like the reverb\'s impulse response (noise * exp(-t / '
               '0.55), low-passed at 5 kHz, the first 20 ms zero)')
    for case in c.cases:
        out = c.arrays[case['arrays']['out']]
        assert out.dtype == np.float64 and out.shape == (case['args']['n_a'] + case['args']['n_b'] - 1,)
    return c.write()


def numpy_file(A, wav):
    import numpy as np
    c = Cases(os.path.join(DIR, 'numpy.npz'), tool='dsp', module=__name__,
              about='the numpy calls of the sound code whose result is more than one rounding: interp, percentile, '
                    'convolve, cumsum, linspace, and sums and means (numpy adds a long 1-D array pairwise); and '
                    'scipy.signal.find_peaks as the levels check uses it',
              notes=dict(out='a scalar result is stored as a 1-element float64 array, and as float.hex() in '
                             '`result`'))

    def scalar(v):
        return np.array([float(v)]), common.hexf(float(v))
    # ---- np.interp
    n = 2400
    x = _rng(2400).normal(0, 1, n)
    t = A.tt(n)
    for semis, dur, what in ((3.0, 0.05, 'kit.music.gliss(x, 3.0, 0.05): a slide up into the note'),
                             (-10.0, 0.9, 'kit.music.gliss(x, -10, 0.9): a fall, read past the end (right=0.0)')):
        off = -semis * (1 - np.clip(t / max(dur, 1e-3), 0, 1)) ** 1.5
        rate = 2 ** (off / 12)
        pos = np.cumsum(rate) - rate[0]
        c.add('np.interp(pos, np.arange(n), x, right=0.0)', dict(n=n, right=jarg(0.0)),
              dict(pos=pos, x=x, out=np.interp(pos, np.arange(n), x, right=0.0)), what=what)
    rate = np.linspace(1, 0, 1200) ** 1.5
    pos = np.cumsum(rate)
    c.add('np.interp(pos, np.arange(n), x)', dict(n=n), dict(pos=pos, x=x, out=np.interp(pos, np.arange(n), x)),
          what='reelkit.audio.tape_stop: pos = cumsum(linspace(1, 0, 1200) ** 1.5)')
    xp, fp = np.array([1.0, 2.5, 2.5, 7.0, 40.0]), np.array([-3.0, 0.5, 2.0, -1.0, 4.0])
    q = np.concatenate([np.linspace(-2.0, 45.0, 95), xp])
    c.add('np.interp(q, xp, fp)', dict(), dict(q=q, xp=xp, fp=fp, out=np.interp(q, xp, fp)),
          what='a few points, queries below, between, on and above them (the ends are held; xp repeats once)')
    # ---- np.percentile
    r = _rng(9997)
    for name, m in (('1-D, 10,000 values', np.abs(r.normal(0, 0.3, 10000))),
                    ('2-D (4800, 2): flattened first, as on the master\'s |mix|', np.abs(r.normal(0, 0.3, (4800, 2)))),
                    ('1-D, 10,001 values: (n - 1) * 0.9997 is a whole number', np.abs(r.normal(0, 0.3, 10001)))):
        out, hx = scalar(np.percentile(m, 99.97))
        c.add('np.percentile(x, 99.97)', dict(q=jarg(99.97)), dict(x=m, out=out), result=hx, what=name)
    # ---- np.convolve(..., 'same')
    r = _rng(960)
    for nx, w in ((N_FIXED, 960), (101, 10), (100, 7)):
        f = r.normal(0, 0.2, nx)
        k = np.ones(w) / w
        out = np.convolve(f ** 2, k, 'same')
        assert out.shape == (nx,)
        c.add("np.convolve(f ** 2, np.ones(w) / w, 'same')", dict(n=nx, w=w), dict(f=f, k=k, out=out),
              what='kit/checks.py band: the gate\'s 20 ms moving mean square' if w == 960 else
                   'a short case: where an even and an odd window sit')
    # ---- np.cumsum, np.linspace
    f = 46 + (150 - 46) * np.exp(-A.tt(N_FIXED) / 0.04)
    c.add('np.cumsum(f)', dict(n=N_FIXED), dict(f=f, out=np.cumsum(f)),
          what='reelkit.audio.kick\'s frequency sweep: a running sum, one add per sample')
    for a, b, nn in ((0, 1, 240), (1, 0, 20160), (0, math.pi, 4801), (0.6, 1.0, 1000), (1, 0, 1)):
        c.add('np.linspace(a, b, n)', dict(a=jarg(float(a)), b=jarg(float(b)), n=nn),
              dict(out=np.linspace(a, b, nn)), what=f'np.linspace({a!r}, {b!r}, {nn})')
    # ---- sums and means
    r = _rng(20001)
    for nn in (N_FIXED, 20001):
        v = r.normal(0, 0.2, nn)
        out, hx = scalar((v ** 2).mean())
        c.add('(x ** 2).mean()', dict(n=nn), dict(x=v, out=out), result=hx,
              what='the mean square of a 1-D array (numpy sums it pairwise)')
    v2 = r.normal(0, 0.2, (N_FIXED, 2))
    out, hx = scalar((v2 ** 2).mean())
    c.add('(x ** 2).mean()', dict(shape=[N_FIXED, 2]), dict(x=v2, out=out), result=hx,
          what='the mean square of a stereo array, all values (the RMS of a block)')
    c.add('(x ** 2).mean(1)', dict(shape=[N_FIXED, 2]), dict(x=v2, out=(v2 ** 2).mean(1)),
          what='the mean square of each row (the compressor\'s input)')
    c.add('(x ** 2).sum(0)', dict(shape=[N_FIXED, 2]), dict(x=v2, out=(v2 ** 2).sum(0)),
          what='the sum of squares of each column (the reverb\'s normalisation)')
    out, hx = scalar((v2 ** 2).mean(0).sum())
    c.add('(x ** 2).mean(0).sum()', dict(shape=[N_FIXED, 2]), dict(x=v2, out=out), result=hx,
          what='the loudness check\'s power: the mean square of each column, added')
    out, hx = scalar(np.abs(v2).max())
    c.add('np.abs(x).max()', dict(shape=[N_FIXED, 2]), dict(x=v2, out=out), result=hx, what='the peak')
    # ---- the 16-bit conversion of reelkit.audio.write
    w = _rng(32767).normal(0, 0.45, (N_FIXED, 2))
    w[:8, 0] = (1.0, -1.0, 0.5 / 32767, -0.5 / 32767, 1.5 / 32767, -1.5 / 32767, 0.99999, -0.99999)
    i16 = (np.clip(w, -1, 1) * 32767).astype(np.int16)
    c.add('(np.clip(x, -1, 1) * 32767).astype(np.int16)', dict(shape=[N_FIXED, 2]),
          dict(x=w, out=i16.astype(np.float64)),
          what='reelkit.audio.write: clip, scale, cut toward zero. The int16 values are stored as float64 (exact); '
               'some inputs are beyond +-1')
    # ---- scipy.signal.find_peaks, as reelkit.levels.profile calls it (the hits-per-second column of the levels check)
    from scipy.signal import find_peaks as peaks0
    from reelkit import levels as LV
    seen = {}

    def find_peaks(x, **k):
        seen['x'], seen['k'] = np.array(x, copy=True), k
        return peaks0(x, **k)
    LV.find_peaks = find_peaks
    try:
        sr, mono = LV.load(wav)
        LV.profile(sr, mono, dur=round(len(mono) / sr / LV.WIN) * LV.WIN)
    finally:
        LV.find_peaks = peaks0
    assert seen['k'] == dict(height=6.0, distance=4), seen['k']
    steps = np.round(_rng(6100).normal(0, 5, 2000))
    for x, what in ((seen['x'], 'the real input: reelkit.levels.profile on the episode\'s audio.wav hands it the '
                                'level of each 10 ms frame minus the lowest of the five before it (dB)'),
                    (steps, 'np.round(default_rng(6100).normal(0, 5, 2000)): whole numbers, so there are flat tops '
                            'and equal peaks closer than 4 samples')):
        pk, props = peaks0(x, height=6.0, distance=4)
        c.add('scipy.signal.find_peaks(x, height=6.0, distance=4)', dict(height=jarg(6.0), distance=4),
              dict(x=x, out=pk.astype(np.float64), heights=props['peak_heights']), what=what,
              note='out: the peak indices, stored as float64 (exact); heights: x at each peak')
    return c.write()


def ufuncs_file():
    import numpy as np
    c = Cases(os.path.join(DIR, 'ufuncs.npz'), tool='dsp', module=__name__,
              about='the transcendental functions the sound code takes, on fixed inputs: numpy\'s array loops, and '
                    'the Python-scalar maths (math.cos, math.sin, math.exp, float ** float) one value at a time',
              notes=dict(
                  why='numpy\'s array loops and the C runtime behind Python\'s math module are different code from '
                      'each other and from the Studio\'s dmath; these pairs show the size of the gap (the stems are '
                      'the bar, at -120 dBFS)',
                  x='float64 inputs: np.random.default_rng(seed).uniform(lo, hi, 2048) unless `what` says otherwise',
                  python='a case whose call starts with "python:" was computed one element at a time with Python '
                         'floats'))
    n = 2048

    def u(seed, lo, hi):
        return _rng(seed).uniform(lo, hi, n)
    for call, fn, seed, lo, hi, what in (
            ('np.sin(x)', np.sin, 1, 0.0, 16 * math.pi, 'small phases'),
            ('np.sin(x)', np.sin, 2, 0.0, 1.0e6, 'phases as large as a long note\'s 33rd harmonic reaches'),
            ('np.cos(x)', np.cos, 3, 0.0, 16 * math.pi, 'small phases'),
            ('np.exp(x)', np.exp, 4, -40.0, 0.0, 'decays'),
            ('np.exp(x)', np.exp, 5, -1.0, 1.0, 'near zero'),
            ('np.tanh(x)', np.tanh, 6, -8.0, 8.0, 'the limiter and the drive'),
            ('np.sqrt(x)', np.sqrt, 7, 0.0, 4.0, 'correctly rounded everywhere: a control'),
            ('x ** 1.5', lambda x: x ** 1.5, 8, 0.0, 1.2, 'glides and fades'),
            ('x ** 0.5', lambda x: x ** 0.5, 9, 0.0, 1.2, 'the tape stop\'s fade'),
            ('x ** 0.62', lambda x: x ** 0.62, 10, 0.0, 1.2, 'the compressor\'s exponent (big_top)'),
            ('x ** 1.3', lambda x: x ** 1.3, 11, 0.0, 1.0, 'the brass voice\'s brightness'),
            ('x ** 2', lambda x: x ** 2, 12, -1.5, 1.5, 'a square (numpy multiplies)'),
            ('2 ** x', lambda x: 2 ** x, 13, -2.0, 2.0, 'semitone offsets / 12 as pitch ratios'),
            ('20 ** x', lambda x: 20 ** x, 14, 0.0, 1.05, 'the riser\'s band centres'),
            ('np.sin(np.pi * 0.5 * x) ** 2', lambda x: np.sin(np.pi * 0.5 * x) ** 2, 15, 0.0, 1.0, 'the wah opening')):
        x = u(seed, lo, hi)
        y = fn(x)
        assert y.dtype == np.float64 and y.shape == x.shape
        c.add(call, dict(seed=seed, lo=jarg(lo), hi=jarg(hi)), dict(x=x, out=y), what=what)
    x = 10.0 ** u(16, -12.0, 1.0)
    c.add('np.log10(x)', dict(seed=16), dict(x=x, out=np.log10(x)), what='x = 10.0 ** uniform(-12, 1): levels in dB')
    for call, fn, x, what in (
            ('python: math.cos(x)', math.cos, (np.linspace(-1, 1, 41) + 1) * math.pi / 4,
             'Mix.place: the left gain of pan = linspace(-1, 1, 41), x = (pan + 1) * pi / 4'),
            ('python: math.sin(x)', math.sin, (np.linspace(-1, 1, 41) + 1) * math.pi / 4, 'Mix.place: the right gain'),
            ('python: math.exp(x)', math.exp, -np.arange(1, 34) / 2.2,
             'trombone60_lowpoly.brass2: exp(-k / 2.2) for the harmonics k = 1..33'),
            ('python: x ** 0.8', lambda v: v ** 0.8, np.arange(1, 34).astype(float), 'brass2: k ** tilt'),
            ('python: x ** 1.6', lambda v: v ** 1.6, u(17, 0.0, 1.0), 'kit.music.colour: (1 - hp) ** 1.6'),
            ('python: 10 ** (x / 20)', lambda v: 10 ** (v / 20), u(18, -4.0, 3.5), 'kit.music.colour: dB to gain'),
            ('python: 2 ** ((x - 69) / 12)', lambda v: 2 ** ((v - 69) / 12), np.arange(20, 110).astype(float),
             'reelkit.audio.mtof without its 440.0: midi 20..109')):
        x = np.ascontiguousarray(x, float)
        y = np.array([fn(float(v)) for v in x])
        c.add(call, dict(), dict(x=x, out=y), what=what)
    return c.write()


def _signature(fn):
    out = []
    for p in inspect.signature(fn).parameters.values():
        out.append(dict(name=p.name, required=p.default is p.empty,
                        default=None if p.default is p.empty else jarg(p.default)))
    return out


def generators_file(col, A):
    """each generator at its defaults (with STAND_IN for what has no default), then at every argument set the
    episode's soundtrack used. Sets that differ only in `seed` get vectors for the first SEEDS_PER_SHAPE seeds used;
    `episode_calls` lists every set either way. reverb and tape_stop take a buffer, so their parameter sets are
    applied to stand-in buffers stored with the case"""
    import numpy as np
    c = Cases(os.path.join(DIR, 'generators.npz'), tool='dsp', module=__name__,
              about='reelkit.audio generators (48,000 Hz, float64): each at its default arguments and at each '
                    'argument set the flute_vs_trombone soundtrack uses',
              notes=dict(
                  args='the call\'s arguments with the defaults filled in; floats as float.hex()',
                  why='"default": the signature\'s defaults (stand_in lists the values given to arguments that have '
                      'none); "episode": an argument set kit.music.main() used',
                  uses='how many times the soundtrack build made the call (0 for a default the episode never uses)',
                  nested='how many of those calls came from inside another generator (chime calls expenv)',
                  seeds=f'thud is called with {len(col.calls["thud"])} argument sets that differ mostly in `seed`: '
                        f'for each (dur, f0, f1) the first {SEEDS_PER_SHAPE} seeds used have vectors, and '
                        f'episode_calls lists them all',
                  reverb='reverb(x, ...) on stand-in buffers: `impulse` (a 1 s stereo buffer whose first row is 1, '
                         '1: the output is the first second of the impulse response itself, as fftconvolve returns '
                         'it) and `burst` (0.25 s of stereo noise). The episode calls it once, on the whole send',
                  tape_stop='tape_stop(x, t0, length) on the stand-in buffer `tape` with t0 = 0.05 s (the episode '
                            'stops three buses at the K.O. time; only int(t0 * 48000) depends on it)'),
              stand_in={g: {k: jarg(v) for k, v in a.items()} for g, a in STAND_IN.items()},
              signatures={g: _signature(vars(A)[g]) for g in GENERATORS})
    r = _rng(48000)
    impulse = np.zeros((SR, 2))
    impulse[0] = 1.0
    c.shared('impulse', impulse)
    c.shared('burst', r.normal(0, 0.2, (SR // 4, 2)) * np.exp(-np.arange(SR // 4) / 3000.0)[:, None])
    i0, n15 = int(0.05 * SR), int(1.5 * SR)
    t = A.tt(i0 + n15)
    tape = np.stack([np.sin(2 * np.pi * 220.0 * t) * 0.5 + r.normal(0, 0.05, len(t)),
                     np.sin(2 * np.pi * 331.0 * t) * 0.4 + r.normal(0, 0.05, len(t))], 1)
    c.shared('tape', tape)
    calls = {}                                             # generator -> rows of episode_calls

    def bound(g, args):
        b = inspect.signature(vars(A)[g]).bind(**args)
        b.apply_defaults()
        return dict(b.arguments)

    def make(g, args, why, rec=None):
        fn = vars(A)[g]
        arrays = {}
        if g == 'reverb':
            outs = {}
            for role in ('impulse', 'burst'):
                outs[role] = fn(c.arrays[role], **{k: v for k, v in args.items() if k != 'x'})
            ids = []
            for role, y in outs.items():
                assert y.dtype == np.float64 and y.shape == c.arrays[role].shape
                a = dict(args, x=c.arrays[role])
                ids.append(c.add(call_text(f'reelkit.audio.{g}', a), {k: jarg(v) for k, v in a.items()},
                                 dict(x=role, out=y), generator=g, why=why, input=role,
                                 uses=rec['count'] if rec else 0, nested=rec['nested'] if rec else 0,
                                 used_by=sorted(rec['users']) if rec else []))
            return ids
        if g == 'tape_stop':
            a = dict(args, x=c.arrays['tape'], t0=0.05)
            y = fn(**a)
            assert y.dtype == np.float64 and y.shape == tape.shape
            return [c.add(call_text(f'reelkit.audio.{g}', a), {k: jarg(v) for k, v in a.items()},
                          dict(x='tape', out=y), generator=g, why=why, input='tape',
                          uses=rec['count'] if rec else 0, nested=rec['nested'] if rec else 0,
                          used_by=sorted(rec['users']) if rec else [],
                          episode_t0=jarg(rec['args']['t0']) if rec else None)]
        y = np.asarray(fn(**args))
        assert y.dtype == np.float64 and y.ndim == 1, (g, y.dtype, y.shape)
        arrays['out'] = y
        return [c.add(call_text(f'reelkit.audio.{g}', args), {k: jarg(v) for k, v in args.items()}, arrays,
                      generator=g, why=why, uses=rec['count'] if rec else 0, nested=rec['nested'] if rec else 0,
                      used_by=sorted(rec['users']) if rec else [])]

    def key_of(args, drop=()):
        return tuple((k, _canon(v)) for k, v in args.items() if k not in drop)
    for g in GENERATORS:
        used = sorted(col.calls[g].values(), key=lambda rec: rec['first'])
        if g == 'reverb':
            default = bound(g, dict(x=None))
        elif g == 'tape_stop':
            default = bound(g, dict(x=None, t0=None))
        else:
            default = bound(g, STAND_IN.get(g, {}))
        drop = ('x', 't0') if g in ('reverb', 'tape_stop') else ()
        dkey = key_of(default, drop)
        same = [rec for rec in used if key_of(rec['args'], drop) == dkey]
        made = {dkey: make(g, default, 'default', same[0] if same else None)}
        shapes = {}
        rows = []
        for rec in used:
            k = key_of(rec['args'], drop)
            if k not in made:
                shape = key_of(rec['args'], drop + ('seed',))
                shapes[shape] = shapes.get(shape, 0) + 1
                if 'seed' not in rec['args'] or shapes[shape] <= SEEDS_PER_SHAPE:
                    made[k] = make(g, dict(rec['args']), 'episode', rec)
            rows.append(dict(args={p: (dict(array=list(rec['shapes'][p])) if p in rec['shapes'] else jarg(v))
                                   for p, v in rec['args'].items()},
                             uses=rec['count'], nested=rec['nested'], used_by=sorted(rec['users']),
                             cases=made.get(k, [])))
        calls[g] = rows
    c.top['episode_calls'] = calls
    return c.write()


# ---------------------------------------------------------------- the checks the files must pass
FILES = ('butter', 'filters', 'resample', 'fftconvolve', 'numpy', 'ufuncs', 'generators')


def check():
    """what the dsp export must leave on disk, read back from the files alone (the export runs this last): every
    file keeps the vector convention and holds float64 only; the brief's counts are met; and the stored outputs are
    what scipy and numpy give for the stored inputs"""
    import numpy as np
    import scipy.signal as sg
    got = {name: check_case_file(os.path.join(DIR, f'{name}.npz')) for name in FILES}
    for name, (doc, arr) in got.items():
        assert doc['tool'] == 'dsp', name
        for member, a in arr.items():
            assert a.dtype.str == '<f8', f'{name}.npz: {member} is {a.dtype.str}, the DSP vectors are float64'

    def same(a, b, where):
        assert a.dtype == b.dtype and a.shape == b.shape and a.tobytes() == b.tobytes(), where
    # butter: each design again from its arguments
    doc, arr = got['butter']
    n_sos = 0
    for c in doc['cases']:
        a = c['args']
        wn = [float.fromhex(x) for x in a['Wn']]
        out = sg.butter(a['N'], wn[0] if len(wn) == 1 else wn, a['btype'], output=a['output'],
                        fs=None if a['fs'] is None else float.fromhex(a['fs']))
        if a['output'] == 'sos':
            n_sos += 1
            assert arr[c['arrays']['sos']].shape == ((a['N'] + 1) // 2 * (2 if len(wn) == 2 else 1), 6), c['id']
            same(np.asarray(out), arr[c['arrays']['sos']], c['id'])
        else:
            same(np.asarray(out[0]), arr[c['arrays']['b']], c['id'])
            same(np.asarray(out[1]), arr[c['arrays']['a']], c['id'])
    assert n_sos >= 12, n_sos
    # filters: the fixed input is 4,800 samples; at least 12 second-order-section filters on it
    doc, arr = got['filters']
    assert arr['noise'].shape == (N_FIXED,) and arr['noise2'].shape == (N_FIXED, 2)
    n = 0
    for c in doc['cases']:
        m = {k: arr[v] for k, v in c['arrays'].items()}
        if 'sos' in m:
            same(sg.sosfilt(m['sos'], m['x'], axis=0), m['out'], c['id'])
            n += c['arrays']['x'] == 'noise'
        elif c['call'].count('lfilter') == 1:
            same(sg.lfilter(m['b'], m['a'], m['x'], axis=0), m['out'], c['id'])
        assert m['out'].shape == m['x'].shape, c['id']
    assert n >= 12, f'{n} sosfilt cases on the fixed noise, the brief asks for 12'
    assert sum('lfilter' in c['call'] for c in doc['cases']) >= 3
    # resample_poly: three inputs or more, each output ceil(n * 160 / 147) long
    doc, arr = got['resample']
    assert len(doc['cases']) >= 3
    for c in doc['cases']:
        x, out = arr[c['arrays']['x']], arr[c['arrays']['out']]
        assert len(out) == -(-len(x) * 160 // 147) == c['args']['n_out'], c['id']
        same(sg.resample_poly(x, 160, 147), out, c['id'])
    assert arr['firwin'].shape == (3201,) and len(arr['h']) == 3201 + doc['cases'][0]['args']['n_pre_pad']
    # fftconvolve: two pairs
    doc, arr = got['fftconvolve']
    assert len(doc['cases']) == 2
    for c in doc['cases']:
        a, b, out = (arr[c['arrays'][k]] for k in ('a', 'b', 'out'))
        assert len(out) == len(a) + len(b) - 1, c['id']
        same(sg.fftconvolve(a, b), out, c['id'])
    # numpy: interp, percentile(99.97) and convolve 'same' are there and are what numpy gives
    doc, arr = got['numpy']
    seen = set()
    for c in doc['cases']:
        m = {k: arr[v] for k, v in c['arrays'].items()}
        call = c['call']
        if call.startswith('np.interp(pos'):
            kw = dict(right=float.fromhex(c['args']['right'])) if 'right' in c['args'] else {}
            same(np.interp(m['pos'], np.arange(c['args']['n']), m['x'], **kw), m['out'], c['id'])
            seen.add('interp')
        elif call.startswith('np.percentile'):
            q = float.fromhex(c['args']['q'])
            assert q == 99.97 and float.fromhex(c['result']) == np.percentile(m['x'], q) == m['out'][0], c['id']
            seen.add('percentile')
        elif call.startswith('np.convolve'):
            same(np.convolve(m['f'] ** 2, m['k'], 'same'), m['out'], c['id'])
            seen.add('convolve')
        elif call == 'np.cumsum(f)':
            same(np.cumsum(m['f']), m['out'], c['id'])
            seen.add('cumsum')
        elif call.startswith('scipy.signal.find_peaks'):
            pk, props = sg.find_peaks(m['x'], height=float.fromhex(c['args']['height']), distance=c['args']['distance'])
            same(pk.astype(np.float64), m['out'], c['id'])
            same(props['peak_heights'], m['heights'], c['id'])
            seen.add('find_peaks')
        if 'result' in c:
            assert m['out'].shape == (1,) and m['out'][0] == float.fromhex(c['result']), c['id']
    assert seen == {'interp', 'percentile', 'convolve', 'cumsum', 'find_peaks'}, seen
    # generators: each one at its defaults; every set the episode used is listed, and has vectors unless it is one
    # of many seeds
    doc, arr = got['generators']
    assert sorted(doc['signatures']) == sorted(GENERATORS) == sorted(doc['episode_calls'])
    by_id = {c['id']: c for c in doc['cases']}
    for g in GENERATORS:
        mine = [c for c in doc['cases'] if c['generator'] == g]
        assert any(c['why'] == 'default' for c in mine), f'{g}: no vector at its default arguments'
        params = [p['name'] for p in doc['signatures'][g]]
        for row in doc['episode_calls'][g]:
            assert sorted(row['args']) == sorted(params) and row['uses'] > 0, (g, row)
            assert row['cases'] or 'seed' in row['args'], f'{g}: an episode argument set has no vector: {row}'
            assert all(by_id[i]['generator'] == g for i in row['cases'])
    for c in doc['cases']:
        out = arr[c['arrays']['out']]
        assert out.ndim == (2 if c['generator'] in ('reverb', 'tape_stop') else 1) and len(out) > 0, c['id']
    return {name: len(doc['cases']) for name, (doc, _) in got.items()}


# ---------------------------------------------------------------- the export
def child():
    """the whole export, in its own interpreter"""
    col, M, A = collect()
    assert M.NA == 2928640 and SR == A.SR, (M.NA, A.SR)
    n_sos = sum(d['output'] == 'sos' for d in col.designs.values())
    print(f'dsp: the soundtrack and its checks asked for {len(col.designs)} filter designs ({n_sos} as second-order '
          f"sections) in {sum(d['count'] for d in col.designs.values())} calls, and called "
          f"{sum(bool(v) for v in col.calls.values())} of reelkit.audio's {len(GENERATORS)} generators")
    files = [butter_file(col), filters_file(col, M), resample_file(), fftconvolve_file(A),
             numpy_file(A, M.EP.path('audio.wav')),
             ufuncs_file(), generators_file(col, A)]
    counts = check()
    for f in files:                                        # listed only now that the checks passed
        f.register()
    print('dsp: every file read back and checked: ' + ', '.join(f'{k} {v}' for k, v in counts.items()) + ' cases')
    return 0


def run():
    fresh('dsp_vectors')


EXPORTS = {'dsp': run}
ORDER = {'dsp': 40}                                        # before voices (50), stems (55) and checks (60) under `all`
