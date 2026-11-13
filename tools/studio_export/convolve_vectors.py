"""convolve: the order in which np.convolve adds its products on the reference PC, for the Studio's
bf_sound::dsp::conv (convolve_same, convolve_full, blas_dot).

studio/reference/dsp/convolve.npz (and its sidecar .json, vectors-convention.md):

    np.dot(x, y)                 one BLAS dot product (cblas_ddot) of 1-D float64 arrays, at lengths on both sides
                                 of the kernel's blocks of 16 and 32
    np.convolve(a, v, 'same')    and mode 'full', at window lengths on both sides of numpy's small-window loop
    np.convolve(a, v)            (at most 11) and of the dot product's blocks, and at the engine's own windows (6,
                                 40, 48, 60: moving averages, np.ones(w) / w); the window second, or first and
                                 longer, or as long as the signal

The inputs are seeded noise (np.random.default_rng), the outputs numpy's own; the export checks each output against
numpy again before it writes."""
import os

from . import common
from .dsp_vectors import Cases

PATH = os.path.join(common.ref('dsp'), 'convolve.npz')
DOT_LENGTHS = (1, 2, 7, 15, 16, 17, 31, 32, 33, 47, 48, 49, 64, 65, 129)
WINDOWS = (1, 2, 6, 11, 12, 16, 17, 32, 33, 40, 48, 60, 65)
AVERAGES = (6, 40, 48, 60)


def run():
    import numpy as np
    c = Cases(PATH, tool='convolve', module=__name__,
              about='np.dot of 1-D float64 arrays (one cblas_ddot) and np.convolve(a, v, mode) for float64, whose '
                    'sums numpy adds in its own order (small_correlate, DOUBLE_dot and the BLAS dot product)',
              notes=dict(out='float64; a dot product is a 1-element array, and float.hex() in `result`'))
    r = np.random.default_rng(1616)
    for n in DOT_LENGTHS:
        x, y = r.normal(0, 1, n), r.normal(0, 1, n) * 10.0 ** r.uniform(-2, 2)
        out = np.array([np.dot(x, y)])
        c.add('np.dot(x, y)', dict(n=n), dict(x=x, y=y, out=out), result=common.hexf(float(out[0])),
              what=f'{n} products')

    def conv(a, v, what):
        k = len(c.cases)
        a, v = c.shared(f'a{k:03d}', a), c.shared(f'v{k:03d}', v)
        for mode in ('same', 'full'):
            na, nv = len(c.arrays[a]), len(c.arrays[v])
            out = np.convolve(c.arrays[a], c.arrays[v], mode)
            assert out.dtype == np.float64 and out.shape == ((max(na, nv),) if mode == 'same' else (na + nv - 1,))
            call = "np.convolve(a, v, 'same')" if mode == 'same' else 'np.convolve(a, v)'
            c.add(call, dict(n_a=na, n_v=nv), dict(a=a, v=v, out=out), what=what)
    for w in WINDOWS:
        conv(r.normal(0, 1, 2 * w + 30), r.normal(0, 1, w) * 10.0 ** r.uniform(-2, 2), f'noise, a {w}-sample window')
    for w in AVERAGES:
        conv(r.normal(0, 1, 2 * w + 30), np.ones(w) / w, f'noise, the moving average np.ones({w}) / {w}')
    conv(r.normal(0, 1, 20), r.normal(0, 1, 53), 'the window first: v is the longer, so numpy swaps them')
    conv(r.normal(0, 1, 37), r.normal(0, 1, 37), 'as long as each other: numpy keeps the order')
    c.write()
    # read back: every output is numpy's for the stored inputs
    with np.load(PATH) as z:
        for case in c.cases:
            m = {role: z[name] for role, name in case['arrays'].items()}
            if case['call'] == 'np.dot(x, y)':
                want = np.array([np.dot(m['x'], m['y'])])
            else:
                mode = 'same' if case['call'].endswith("'same')") else 'full'
                want = np.convolve(m['a'], m['v'], mode)
            assert want.tobytes() == m['out'].tobytes(), case['id']
    c.register()
    return 0


EXPORTS = {'convolve': run}
