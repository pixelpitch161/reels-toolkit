"""py: Python number semantics the Studio ports in bf_core::py, as inputs and the values Python gave.

studio/reference/py/vectors.json holds 2,000 rows for each of: round(x), round(x, 1), round(x, 2), round(x, 3),
round(x, 5), np.round(x, 2), a % b and a // b on floats (what the plan asks for), plus np.rint(x), fight.py's
_wrap(a) and integer a % b and a // b (the rest of the bf_core::py interface). Inputs are seeded and lean on what
goes wrong in a port: exact ties, decimal ties that are not ties in binary, one step either side of a tie, minus
zero, results that round up to the divisor, exact multiples.

Layout: {"about", "format": 1, "numbers", "rows_per_case": 2000, "cases": {name: {"expr", "inputs", "output",
"rows"}}}. A row is the inputs in the order of "inputs", then the output. Floats are float.hex() strings; ints are
JSON integers (all within 2**53)."""
import math
import random

import numpy as np

from . import common

OUT = common.ref('py', 'vectors.json')
N = 2000
SEED = 20261110
# the example episode's grid (kit/episode.py at BPM 132), for inputs shaped like the engine's own
BEAT = 0.44852941176470584
BAR = 1.7941176470588234
T_FIGHT = 3.5882352941176467
DT = 1.0 / 120.0

ROUND_EDGES = [
    0.0, 0.5, 1.5, 2.5, 3.5, 4.5, 0.05, 0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 0.85, 0.95, 0.005, 0.015, 0.025,
    0.035, 0.045, 0.125, 0.375, 0.625, 0.875, 1.005, 1.015, 1.025, 2.675, 2.665, 1.115, 0.285, 0.575, 0.0005,
    0.0015, 0.0025, 0.0625, 0.1875, 1.0005, 1.000005, 1.000015, 1.000025, 0.000005, 0.000015, 0.000025, 0.0000005,
    0.5000000000000001, 0.49999999999999994, 1.4999999999999998, 2.5000000000000004, 0.049999999999999996,
    1e-7, 1e-10, 1e-300, 5e-324, 2.2250738585072014e-308, 1e15, 1e15 + 0.5, 1e15 + 1.5, 4503599627370495.5,
    4503599627370496.0, 4503599627370497.0, 9007199254740991.0, 123456789.125, 0.1, 0.2, 0.3, 0.7, 1.0, 2.0, 10.0,
    100.0, 99.995, 9.995, 0.995, 99.5, 999.5, 0.994999999999999, 50.021568627450975, 3.5965686274509803]
MOD_EDGES = [
    (5.5, 2.0), (6.0, 3.0), (0.0, 3.0), (1.0, 3.0), (2.0, 3.0), (3.0, 3.0), (1e300, 7.0), (5e-324, 3.0), (7.0, 1e300),
    (0.3, 0.1), (1.0, 0.1), (0.7, 0.1), (0.5, 0.1), (1e-9, 1.0), (1e-17, 1.0), (1e-17, 2 * math.pi),
    (math.pi, 2 * math.pi), (2 * math.pi, 2 * math.pi), (3 * math.pi, 2 * math.pi), (4 * math.pi, 2 * math.pi),
    (6.283185307179585, 2 * math.pi), (6.283185307179587, 2 * math.pi), (61.0, BAR), (60.99999999999999, BAR),
    (T_FIGHT, BEAT), (T_FIGHT, BAR), (2 * BAR, BAR), (34 * BAR, BAR), (8 * BEAT, BEAT), (1.0, DT), (0.1, DT),
    (1.0, 1.0 / 30.0), (0.5, 0.25), (0.75, 0.25), (1e16, 3.0), (9007199254740993.0, 2.0), (1.5, 0.5), (2.5, 0.5)]
WRAP_EDGES = [0.0, math.pi, 2 * math.pi, 3 * math.pi, math.pi / 2, math.pi / 4, 1e-17, 1e-9, 1e-300, 100.0, 1000.0,
              3.141592653589793, 3.1415926535897927, 3.1415926535897936, 6.283185307179586, 6.283185307179585,
              9.42477796076938, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 0.5, 12.566370614359172]
IMOD_EDGES = [(7, 3), (6, 3), (0, 3), (1, 3), (2, 3), (3, 3), (1, 1), (0, 1), (5, 997), (996, 997), (997, 997),
              (998, 997), (2 ** 53, 997), (2 ** 53 - 1, 1000), (10 ** 15, 7), (61, 4), (136, 8), (17, 16)]


class Rand:
    """seeded draws built on random.Random.random() alone, the one stream Python guarantees across versions"""

    def __init__(self, seed):
        self.r = random.Random(seed)

    def u(self, lo, hi):
        return lo + (hi - lo) * self.r.random()

    def i(self, lo, hi):
        """an int in lo..hi, both ends included"""
        return lo + int(self.r.random() * (hi - lo + 1))

    def pick(self, seq):
        return seq[int(self.r.random() * len(seq))]

    def sign(self):
        return -1.0 if self.r.random() < 0.5 else 1.0


def round_inputs(rg, nd):
    """N floats for rounding to `nd` decimals: the edge list with both signs, then families in a fixed rotation"""
    xs = []
    for e in ROUND_EDGES:
        xs += [e, -e]
    scale = 2.0 ** (nd + 1)
    while len(xs) < N:
        fam = len(xs) % 8
        if fam == 0:                                      # a decimal tie as written: 12.345 for 2 decimals
            digits = ''.join(str(rg.i(0, 9)) for _ in range(nd))
            x = float(f"{rg.i(0, rg.pick((1, 9, 99, 9999)))}.{digits}5")
        elif fam == 1:                                    # an exact tie in binary: whole + odd / 2**(nd + 1)
            x = rg.i(0, rg.pick((0, 9, 999))) + (2 * rg.i(0, int(scale) // 2 - 1) + 1) / scale
        elif fam == 2:                                    # one step either side of an exact tie
            x = rg.i(0, 99) + (2 * rg.i(0, int(scale) // 2 - 1) + 1) / scale
            x = math.nextafter(x, x + rg.sign())
        elif fam == 3:                                    # a short decimal with one or two digits too many
            digits = ''.join(str(rg.i(0, 9)) for _ in range(nd + rg.i(1, 2)))
            x = float(f"{rg.i(0, rg.pick((1, 99, 9999)))}.{digits}")
        elif fam == 4:                                    # an engine time: t_fight + (step + 1) * dt
            x = T_FIGHT + (rg.i(0, 6599) + 1) * DT
        elif fam == 5:
            x = rg.u(0.0, rg.pick((1.0, 10.0, 1000.0, 1e6)))
        elif fam == 6:
            x = rg.u(0.0, rg.pick((1e-3, 1e-5, 1e-7)))
        else:                                             # a value that already has `nd` decimals or fewer
            x = float(f"{rg.i(0, 999)}.{''.join(str(rg.i(0, 9)) for _ in range(rg.i(0, nd)))}" + '0')
        xs.append(x * rg.sign())
    return xs[:N]


def mod_inputs(rg):
    """N (a, b) float pairs for % and //: edges in all four sign combinations, then families in rotation; b != 0"""
    ps = []
    for a, b in MOD_EDGES:
        ps += [(a, b), (-a, b), (a, -b), (-a, -b)]
    divisors = (2 * math.pi, math.pi, BEAT, BAR, BEAT / 4, 1.0 / 30.0, DT, 0.25, 1.0, 4.0, 8.0)
    while len(ps) < N:
        fam = len(ps) % 6
        if fam == 0:                                      # an angle against a full turn (fight.py: th % (2 pi))
            a, b = rg.u(-30.0, 30.0), 2 * math.pi
        elif fam == 1:                                    # a time against the grid (t % BEAT, t // BAR)
            a, b = rg.u(0.0, 61.0), rg.pick(divisors[2:])
        elif fam == 2:                                    # a multiple of the divisor as a float product
            b = rg.pick(divisors) * rg.sign()
            a = rg.i(-200, 200) * b
        elif fam == 3:                                    # one step off a multiple
            b = rg.pick(divisors) * rg.sign()
            a = rg.i(-200, 200) * b
            a = math.nextafter(a, a + rg.sign())
        elif fam == 4:
            a, b = rg.u(-1000.0, 1000.0), rg.u(0.01, 50.0) * rg.sign()
        else:                                             # far apart in size
            a, b = rg.sign() * 2.0 ** rg.u(-30.0, 30.0), rg.sign() * 2.0 ** rg.u(-30.0, 30.0)
        ps.append((a, b))
    return ps[:N]


def wrap_inputs(rg):
    xs = []
    for e in WRAP_EDGES:
        xs += [e, -e]
    while len(xs) < N:
        fam = len(xs) % 4
        if fam == 0:
            x = rg.u(-30.0, 30.0)
        elif fam == 1:                                    # the difference of two angles, as in thrust_step
            x = math.atan2(rg.u(-1.0, 1.0), rg.u(-1.0, 1.0)) - rg.u(0.0, 2 * math.pi)
        elif fam == 2:                                    # at and next to a multiple of pi / 2
            x = rg.i(-12, 12) * (math.pi / 2)
            x = rg.pick((x, math.nextafter(x, math.inf), math.nextafter(x, -math.inf)))
        else:
            x = rg.u(-1e-6, 1e-6)
        xs.append(x)
    return xs[:N]


def imod_inputs(rg):
    ps = []
    for a, b in IMOD_EDGES:
        ps += [(a, b), (-a, b), (a, -b), (-a, -b)]
    while len(ps) < N:
        fam = len(ps) % 3
        if fam == 0:                                      # a beat counter against a period (kb % every)
            a, b = rg.i(-50, 400), rg.i(1, 16)
        elif fam == 1:
            a, b = rg.i(-10 ** 6, 10 ** 6), rg.i(1, 1000) * int(rg.sign())
        else:                                             # a big value against a small prime (hash(s) % 997)
            a, b = rg.i(-2 ** 53, 2 ** 53), rg.pick((997, 1000, 7, -997))
        ps.append((a, b))
    return ps[:N]


def _wrap(a):
    """fight.py _wrap, operation for operation"""
    return (a + math.pi) % (2 * math.pi) - math.pi


def cases():
    h = common.hexf
    out = {}

    def one(name, expr, xs, fn, output='f64'):
        rows = []
        for x in xs:
            y = fn(x)
            if output == 'int':
                assert type(y) is int and abs(y) <= 2 ** 53, (name, x, y)
                rows.append([h(x), y])
            else:
                rows.append([h(x), h(float(y))])
        out[name] = dict(expr=expr, inputs=['x'], output=output, rows=rows)

    def two(name, expr, ps, fn):
        out[name] = dict(expr=expr, inputs=['a', 'b'], output='f64', rows=[[h(a), h(b), h(fn(a, b))] for a, b in ps])

    one('round', 'round(x)', round_inputs(Rand(SEED + 0), 0), round, output='int')
    for k, nd in enumerate((1, 2, 3, 5)):
        one(f'round_{nd}', f'round(x, {nd})', round_inputs(Rand(SEED + 1 + k), nd), lambda x, nd=nd: round(x, nd))
    one('np_round_2', 'np.round(x, 2)', round_inputs(Rand(SEED + 5), 2), lambda x: np.round(x, 2))
    ps = mod_inputs(Rand(SEED + 6))
    two('mod', 'a % b', ps, lambda a, b: a % b)
    two('floordiv', 'a // b', ps, lambda a, b: a // b)
    one('rint', 'np.rint(x)', round_inputs(Rand(SEED + 7), 0), np.rint)
    one('wrap_pi', '(a + math.pi) % (2 * math.pi) - math.pi', wrap_inputs(Rand(SEED + 8)), _wrap)
    out['wrap_pi']['inputs'] = ['a']
    ips = imod_inputs(Rand(SEED + 9))
    out['imod'] = dict(expr='a % b', inputs=['a', 'b'], output='int', rows=[[a, b, a % b] for a, b in ips])
    out['ifloordiv'] = dict(expr='a // b', inputs=['a', 'b'], output='int', rows=[[a, b, a // b] for a, b in ips])
    for name, c in out.items():
        assert len(c['rows']) == N, (name, len(c['rows']))
    return out


def run():
    doc = dict(
        about='Python number semantics for bf_core::py: seeded inputs and the value Python gave for each. A row is '
              'the inputs in the order of "inputs", then the output. Made by tools/studio_export/py_vectors.py.',
        format=1,
        numbers=dict(
            f64='a Python float.hex() string: [-]0x1.<13 hex digits>p<signed decimal exponent>; zero is 0x0.0p+0 '
                'or -0x0.0p+0; a subnormal is 0x0.<13 hex digits>p-1022. Every float input is one of these, and '
                'so is every output of a case whose "output" is "f64" (compare bit for bit: the sign of zero is '
                'part of the answer)',
            int='a JSON integer, within 2**53. round(x) returns a Python int, so it has no minus zero: round(-0.3) '
                'is 0, where np.rint(-0.3) is -0.0. imod and ifloordiv take and give ints'),
        rows_per_case=N,
        cases=cases())
    common.write_json(OUT, doc)                           # written and listed in the manifest
    print(f"py: {common.rel(OUT)}  {len(doc['cases'])} cases x {N} rows")


EXPORTS = {'py': run}
