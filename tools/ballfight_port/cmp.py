"""compare: python cmp.py frames <dirA> <dirB> | wav <a.wav> <b.wav> | buses <a.npz> <b.npz> [t0 t1]"""
import sys, os, glob
import numpy as np
what = sys.argv[1]
if what == 'frames':
    a, b = sys.argv[2], sys.argv[3]
    for fa in sorted(glob.glob(os.path.join(a, 'f*.npy'))):
        fb = os.path.join(b, os.path.basename(fa))
        if not os.path.exists(fb):
            continue
        x, y = np.load(fa).astype(int), np.load(fb).astype(int)
        d = np.abs(x - y).max(-1)
        n = int((d > 0).sum())
        if n:
            ys, xs = np.nonzero(d)
            print(os.path.basename(fa), f'{n:6d} px differ ({100*n/d.size:.2f} %), max {d.max()}, bbox x {xs.min()}-{xs.max()} y {ys.min()}-{ys.max()}')
        else:
            print(os.path.basename(fa), 'identical')
elif what == 'wav':
    from scipy.io import wavfile
    sa, x = wavfile.read(sys.argv[2]); sb, y = wavfile.read(sys.argv[3])
    x, y = x.astype(int), y.astype(int)
    n = min(len(x), len(y))
    d = np.abs(x[:n] - y[:n])
    print('len', len(x), len(y), ' identical' if (len(x) == len(y) and d.max() == 0) else f' max diff {d.max()} LSB, {int((d > 0).sum())} samples differ, rms diff {np.sqrt((d.astype(float)**2).mean()):.3g}')
elif what == 'buses':
    A, B = np.load(sys.argv[2]), np.load(sys.argv[3])
    sr = 48000
    t0 = float(sys.argv[4]) if len(sys.argv) > 4 else 0.0
    t1 = float(sys.argv[5]) if len(sys.argv) > 5 else 1e9
    keys = sys.argv[6].split(',') if len(sys.argv) > 6 else sorted(set(A.files) | set(B.files))
    for k in keys:
        if k not in A.files or k not in B.files:
            print(f'{k:10s} only in', 'A' if k in A.files else 'B')
            continue
        x, y = A[k], B[k]
        i0, i1 = int(t0 * sr), int(min(t1 * sr, len(x), len(y)))
        x, y = x[i0:i1], y[i0:i1]
        pk = max(np.abs(x).max(), np.abs(y).max(), 1e-12)
        d = np.abs(x - y).max()
        print(f'{k:10s} max |diff| {d:.3g} (peak {pk:.3g}, rel {d/pk:.3g})')
