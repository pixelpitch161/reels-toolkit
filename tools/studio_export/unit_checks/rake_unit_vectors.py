# What it prints: the closed-form motions of the weapon kind `rake` and the ability `sweep` (ACOUSTIC's SIX-STRING
# RAKE and BIG STRUM, fight.py 556 to 564): `Fight.rake_lateral` and `Fight.sweep_front` on a grid of times before,
# inside and after the stroke, both directions, as the bits of each float (hex of the IEEE 754 pattern); and
# `Fight.aim_blend` for a few headings and aims (a blend below 1, the plain aim at 1).
# The expected numbers in studio/crates/bf-core/src/roster/kinds/rake.rs (tests) and
# studio/crates/bf-sim/src/kinds/rake.rs (tests) are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/rake_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for the rake's unit tests. Usage: rake_unit_vectors.py <repo root>"""
import struct
import sys
import types

import numpy as np

clone = sys.argv[1]
sys.path.insert(0, clone + '/projects/ballfight')
import fight as F  # noqa: E402


def bits(x):
    return '0x%016x' % struct.unpack('<Q', struct.pack('<d', float(x)))[0]


print('// rake_lateral: (tau, dur, S, direction, bits)')
for d in (1.0, -1.0):
    for S in (0.6, 0.37):
        for tau in (-0.01, 0.0, 0.004166666666666667, 0.03, 0.05, 0.08, 0.1234, 0.15, 0.16, 0.2):
            print(f'({tau!r}, 0.16, {S!r}, {d!r}, {bits(F.Fight.rake_lateral(tau, 0.16, S, d))}),')
print('// sweep_front: (tau, dur, reach, R, bits)')
for tau in (-0.01, 0.0, 0.004166666666666667, 0.1, 0.3333, 0.5, 0.7, 0.9, 0.9090909090909091, 1.0):
    print(f'({tau!r}, 0.9090909090909091, 9.0, 0.5, {bits(F.Fight.sweep_front(tau, 0.9090909090909091, 9.0, 0.5))}),')
print('// aim_blend: (dx, dy, vx, vy, k, bits)')
for (d, v, k) in (((1.3, -0.4), (0.2, 3.1), 0.47), ((-2.0, 0.5), (-1.9, -2.2), 0.47), ((0.3, 0.9), (2.9, 0.8), 1.0),
                  ((-1.1, -1.7), (0.0, 0.0), 0.47), ((0.0, 0.0), (1.0, 2.0), 0.3)):
    B = types.SimpleNamespace(v=np.array(v, float))
    print(f'({d[0]!r}, {d[1]!r}, {v[0]!r}, {v[1]!r}, {k!r}, '
          f'{bits(F.Fight.aim_blend(B, np.array(d, float), k))}),')
