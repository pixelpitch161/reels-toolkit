"""footprint overlap audit on layout.npz: standing footprints (w x t) and fallen footprints (w x h*sin(rest))
of dominoes in different lanes, separating-axis test, 2 mm tolerance.  python overlap_check.py"""
import numpy as np
from scipy.spatial import cKDTree
c = dict(np.load('layout.npz'))
dom = np.where(c['kind'] == 0)[0]
P = np.stack([c['cx'], c['cz']], 1)


def rect(i, fallen):
    x, z, fx, fz = c['cx'][i], c['cz'][i], c['fx'][i], c['fz'][i]
    w = c['dw'][i] / 2
    lx, lz = fz, -fx
    if fallen:
        a0, a1 = c['dt'][i] / 2 - c['dt'][i], c['dt'][i] / 2 + c['dh'][i] * np.sin(c['thr'][i])
    else:
        a0, a1 = -c['dt'][i] / 2, c['dt'][i] / 2
    return np.array([[x + fx * a + lx * b, z + fz * a + lz * b] for a, b in ((a0, -w), (a1, -w), (a1, w), (a0, w))])


def hit(A, B, tol=0.002):
    for Q in (A, B):
        for k in range(4):
            e = Q[(k + 1) % 4] - Q[k]
            n = np.array([-e[1], e[0]]) / (np.linalg.norm(e) + 1e-12)
            pa, pb = A @ n, B @ n
            if pa.max() - tol <= pb.min() or pb.max() - tol <= pa.min():
                return False
    return True


names = {0: 'trunk', 1: 'fan', 2: 'funnel', 3: 'bundle', 4: 'bell', 5: 'chord', 6: 'feeder', 7: 'field', 8: 'spiral',
         9: 'stair', 10: 'giant', 11: 'frame', 12: 'bar', 13: 'final'}
tree = cKDTree(P[dom])
for fallen in (False, True):
    bad = {}
    pts = []
    for a, b in tree.query_pairs(0.07):
        i, j = dom[a], dom[b]
        if c['lane'][i] == c['lane'][j] or c['nxt'][i] == j or c['nxt'][j] == i:
            continue
        if c['nxt'][i] >= 0 and c['nxt'][c['nxt'][i]] == j or c['nxt'][j] >= 0 and c['nxt'][c['nxt'][j]] == i:
            continue
        if c['sec'][i] in (9, 10) or c['sec'][j] in (9, 10):
            continue
        if hit(rect(i, fallen), rect(j, fallen)):
            k = (names[c['sec'][i]], names[c['sec'][j]])
            bad[k] = bad.get(k, 0) + 1
            pts.append((round(c['cx'][i], 2), round(c['cz'][i], 2), round(c['t0'][i], 2)))
    print('fallen' if fallen else 'standing', 'overlaps:', sum(bad.values()), bad)
    if pts:
        print('   e.g.', pts[:6])
