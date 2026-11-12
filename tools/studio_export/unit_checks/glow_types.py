"""Which type the kit's st['glow'] (and st['scale'], st.get('sink')) has per frame of a fighter's left fight.
Usage (clone root, PYTHONHASHSEED=0, PYTHONPATH=clone): python glow_types.py <slug>"""
import collections
import importlib
import json
import sys

from tools.studio_export import m3refs

if __name__ == '__main__':
    slug = sys.argv[1]
    spec = m3refs.normalize(importlib.import_module(f'tools.studio_export.fighter_{slug}').SPEC)
    with open(m3refs.ref_dir(f'fighters/{slug}', 'fight.json'), encoding='utf-8') as fh:
        doc = json.load(fh)
    cfg = m3refs.fighter_configs(spec)[(spec['opponent'], False)].with_seed(doc['left']['seed']).part('glow_types')
    cfg.load()
    from ballfight.kit import picture as P          # noqa: N812
    seen = collections.Counter()
    first = {}
    for f in range(P.T.NF):
        t = f / P.FPS
        for k in (0, 1):
            st = P.fighter(k, t)
            for key in ('glow', 'scale', 'sink'):
                if key in st:
                    tag = (P.NAMES[k], key, type(st[key]).__name__)
                    seen[tag] += 1
                    first.setdefault(tag, (f, float(st[key])))
    for tag in sorted(seen):
        print(tag, seen[tag], 'first at frame, value:', first[tag])
