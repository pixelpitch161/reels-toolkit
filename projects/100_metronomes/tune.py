import numpy as np, sys
from sim import *
def summary(p, seed=7):
    out = run(p, seed=seed)
    fl = sync_flags(out['theta'], out['psi'])
    cnt = fl.sum(1)
    ts = [12,16,18,20,22,24,26,28,30,32,34,35,36,37,38,39,40]
    s = ' '.join(f"{cnt[int(t*FPS)]:3d}" for t in ts)
    hold = fl[:, HOLDOUT]
    bad = np.nonzero(cnt < 100)[0]
    full = (bad.max()+1)/FPS if len(bad) else 0
    n99 = np.nonzero(cnt >= 99)[0]
    first99 = n99[0]/FPS if len(n99) else None
    return s, full, first99
print('      ' + ' '.join(f"{t:3d}" for t in [12,16,18,20,22,24,26,28,30,32,34,35,36,37,38,39,40]))
for a in sys.argv[1:]:
    p = dict(PARAMS); 
    for kv in a.split(','):
        k,v = kv.split('='); p[k]=float(v)
    seed = int(p.pop('seed', 7))
    s, full, f99 = summary(p, seed)
    print(a, '\n      ', s, ' full@', full, ' 99@', f99)
