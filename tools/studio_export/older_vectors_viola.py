"""older_vectors_viola: VIOLA's three picture opt-ins (plan m4 Task 11; older/picture_patches.py, its VIOLA section;
bf-picture `Look::crowd_laugh`, `Look::flash_hit`, `Look::spark_events`), proven as Task 4's ten are
(older_vectors.py):

- off: with every patch installed (the thirteen), the base-class looks draw picture_ring/unit.json's frames, and three
  kit episodes with their own looks draw the same frames and draw logs patched and unpatched (older_vectors.py's own
  `base`, `plain` and `patched` children, run again here);
- on: the example episode (FLUTE against TROMBONE) with FLUTE's base look and a stub look on the TROMBONE's side that
  uses the three (arithmetic both languages compute alike: the Rust test's StubLook): its frames and draw logs.

    python -m tools.studio_export older_vectors_viola    writes studio/reference/older/vectors/optins_viola.json"""
import json
import os
import sys

from . import common
from . import older_vectors as OV

OPTINS = ('crowd_laugh', 'flash_hit', 'spark_events')
STUB_SIDE = 1
LAUGH = (20.0, 22.0)                             # the stub's laugh: up over 0.5 s from 20.0 s, down over 0.5 s from 22.0
SPARK_KINDS = ('strike',)                        # the stub's extra spark events (the FLUTE's strikes: x, z at 4, 5)


def stub_look(base, P):
    calls = {}

    def hit(name):
        calls[name] = calls.get(name, 0) + 1

    class Stub(base):
        spark_events = SPARK_KINDS

        def crowd_laugh(self, t):
            a, b = LAUGH
            if t < a or t >= b + 0.5:
                return 0.0
            hit('crowd_laugh')
            return min(1.0, (t - a) / 0.5) if t < b else 1.0 - (t - b) / 0.5

        def flash_hit(self, e):
            hit('flash_hit')
            return True                              # every hit of the stub's (the base flashes only slam / wave)

    return Stub, calls


def on_frames(P):
    """the laugh's rise, top and fall; just after the first strikes (their sparks) and the stub's first hits (their
    flashbulbs); a spread; the K.O."""
    fs = [150, 450, 900, 1200, 1500]
    fs += [int(round(t * 30)) for t in (20.1, 20.3, 20.6, 21.0, 21.5, 22.1, 22.3, 22.45)]
    st = [e for e in P.EV if e[1] in SPARK_KINDS][:4]
    fs += [int(round(e[0] * 30)) + 1 for e in st]
    hs = [e for e in P.EV if e[1] == 'hit' and e[2] == STUB_SIDE and e[7] not in ('slam', 'wave', 'shockwave')][:3]
    fs += [int(round(e[0] * 30)) + 1 for e in hs]
    fko = int(round(P.T_KO * 30))
    fs += [fko, fko + 12]
    return sorted(set(f for f in fs if 0 <= f < 1830))


def child_on(out, mode):
    """'on': FLUTE's base look and the stub; 'base': both base looks, the same frames"""
    from . import older
    older.install()
    common.kit()
    from ballfight.kit import fighters as FM
    holder = {}

    def look(name, P_, k):
        if k != STUB_SIDE or mode == 'base':
            return FM.base.Look(P_, k)
        Stub, calls = stub_look(FM.base.Look, P_)
        holder['calls'] = calls
        return Stub(P_, k)
    FM.look = look
    from . import frames as F
    P = F.picture(f'older_vectors_viola_{mode}')
    assert getattr(P, '_OLDER_PATCHED', False), 'the picture patches are not installed'
    frames = on_frames(P)
    rows = OV._render_all(P, frames)
    res = dict(rows=rows, frames=frames, optins=list(P.OLDER_OPTINS), calls=dict(sorted(holder.get('calls', {}).items())),
               n_flash=len(P.FLASH_T))
    with open(out, 'w', encoding='utf-8') as fh:
        json.dump(res, fh)


def _child(mode, out):
    import subprocess
    env = dict(os.environ)
    for k in OV.THREADS:
        env[k] = '1'
    subprocess.run([sys.executable, '-m', 'tools.studio_export.older_vectors_viola', mode, out], cwd=common.REPO,
                   check=True, env=env)


def run():
    scratch = os.path.join(common.CACHE, 'older_vectors_viola')
    os.makedirs(scratch, exist_ok=True)
    got = {}
    for mode, extra in [('base', ())] + [(m, (ex,)) for ex in OV.OFF_EXAMPLES for m in ('plain', 'patched')]:
        path = os.path.join(scratch, f"off_{mode}_{'_'.join(extra) or 'x'}.json")
        OV.child(mode, path, *extra)
        with open(path, encoding='utf-8') as fh:
            got[(mode,) + extra] = json.load(fh)
        print(f'older_vectors_viola: off {mode} {" ".join(extra)} done')
    with open(common.ref('picture_ring', 'unit.json'), encoding='utf-8') as fh:
        unit = json.load(fh)
    want = {f: s for f, s in unit['frames']}
    wrong = [r[0] for r in got[('base',)]['unit'] if want[r[0]] != r[1]]
    if wrong:
        raise SystemExit(f'older_vectors_viola: with the patches installed the base-class looks draw frames {wrong} '
                         f'differently from picture_ring/unit.json')
    for ex in OV.OFF_EXAMPLES:
        a, b = got[('plain', ex)]['rows'], got[('patched', ex)]['rows']
        if a != b:
            raise SystemExit(f'older_vectors_viola: {ex}: the patched kit draws frames '
                             f'{[x[0] for x, y in zip(a, b) if x != y]} differently')
    on = {}
    for mode in ('base', 'on'):
        path = os.path.join(scratch, f'{mode}.json')
        _child(mode, path)
        with open(path, encoding='utf-8') as fh:
            on[mode] = json.load(fh)
    for name in OPTINS:
        if name not in on['on']['optins']:
            raise SystemExit(f'older_vectors_viola: opt-in {name} is not installed')
        if not on['on']['calls'].get(name) and name != 'spark_events':
            raise SystemExit(f'older_vectors_viola: the stub never used {name} ({on["on"]["calls"]})')
    changed = [r[0] for r, q in zip(on['on']['rows'], on['base']['rows']) if r[1] != q[1]]
    if len(changed) * 2 < len(on['on']['rows']):
        raise SystemExit(f'older_vectors_viola: the stub changes too few frames (changed: {changed})')
    doc = dict(about="VIOLA's picture opt-ins of older/picture_patches.py (bf-picture Look methods of the same names): "
                     'off, the kit unchanged with every patch installed; on, the example episode with the stub look of '
                     "tools/studio_export/older_vectors_viola.py on the TROMBONE's side",
               tool='older_vectors_viola', episode=OV.EXAMPLE, stub_side=STUB_SIDE, optins=list(OPTINS),
               laugh=list(LAUGH), spark_kinds=list(SPARK_KINDS),
               off=dict(base_frames=len(got[('base',)]['unit']), examples=list(OV.OFF_EXAMPLES)),
               on=dict(rows=on['on']['rows'], base_rows=on['base']['rows'], calls=on['on']['calls'],
                       n_flash=on['on']['n_flash'], base_n_flash=on['base']['n_flash'], changed=changed))
    path = os.path.join(OV.OUT, 'optins_viola.json')
    common.write_json(path, doc)
    print(f'older_vectors_viola: {common.rel(path)}  {os.path.getsize(path)} bytes; {len(changed)} of '
          f'{len(on["on"]["rows"])} frames changed')


EXPORTS = {'older_vectors_viola': run}

if __name__ == '__main__':                       # a child: python -m tools.studio_export.older_vectors_viola <mode> <out>
    child_on(sys.argv[2], sys.argv[1])
