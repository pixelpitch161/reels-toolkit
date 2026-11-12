"""fighter_stages: stage dumps of a port's look fight, for finding the first stage of a frame that differs.

    python -m tools.studio_export fighter_stages <slug> [left | right] <frame>...

draws the frames of the fighter's `left` (default) or `right` fight (fighters/<slug>/fight.json) as its look gate
does, with its real look and the kit's base Look for the other side, and writes each frame's buffers after every
stage of the renderer (stages.py's dumps) into studio/reference/.cache/stages_fighter/<slug>/<fight>/ (git-ignored,
not listed). The proving harness's ignored `stages` test (cargo test -p bf-harness --test fighter_<slug> stages --
--ignored) compares every stage of every frame dumped there."""
import importlib
import json
import os

from . import common, m3parts as MP, m3refs

OUT = os.path.join(common.CACHE, 'stages_fighter')


def run(args):
    args = list(args)
    if not args:
        raise SystemExit('fighter_stages: name the fighter (its slug), then left or right, then the frames')
    slug = args.pop(0)
    fight = args.pop(0) if args and args[0] in ('left', 'right') else 'left'
    frames = [int(a) for a in args] or [150, 600, 1500]
    spec = m3refs.normalize(importlib.import_module(f'{__package__}.fighter_{slug}').SPEC)
    folder = f'fighters/{slug}'
    with open(m3refs.ref_dir(folder, 'fight.json'), encoding='utf-8') as fh:
        doc = json.load(fh)
    cfgs = m3refs.fighter_configs(spec)
    if fight == 'left':
        cfg = cfgs[(spec['opponent'], False)].with_seed(doc['left']['seed'])
        rec = os.path.join(common.REPO, *f"{m3refs.REF}/{doc['left']['file']}".split('/'))
    else:
        cfg = cfgs[(spec['opponent'], True)].with_seed(doc['right']['seed'])
        rec = m3refs.cache_dir(folder, 'right.bfrec')
    out = os.path.join(OUT, slug, fight)
    MP.fresh(MP.stages_child, cfg, spec['name'], rec, frames, out)
    print(f"fighter_stages: {slug} {fight}: {', '.join(map(str, frames))} -> {common.rel(out)}")


EXPORTS = {'fighter_stages': run}
TAKES_ARGS = ('fighter_stages',)
