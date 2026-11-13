"""roster: the kit fighters' numbers, exported from projects/ballfight/fighters.py instead of retyped.

studio/crates/bf-content/data/roster.json: {"about", "format": 1, "fighters": [...]}. One object per fighter, in
the order of fighters.py: "name", then the entry's own keys in their own order (color, dark, corner, view, weapon,
abilities, pp, register, idiom, and pitch where the entry has one). Every number is kept exactly: an int stays an
int, a float is written so that it reads back to the same bits, a tuple becomes a list.

The 20 fighters the kit has ported (design spec, section 10) and the ten older fighters of milestone 4
(OLDER_FIGHTERS, plan m4 Table O1) are written, all in fighters.py's order. The older ten load in the app as unported
kinds until their ports fill them; the app offers only the fighters in bf-content's registry.rs PORTED. The two
retired fighters are never written; the export stops if one would be."""
import json
import os

from . import common, manifest

OUT = os.path.join(common.REPO, 'studio', 'crates', 'bf-content', 'data', 'roster.json')
KIT_FIGHTERS = ('ACOUSTIC', 'ALTO SAX', 'BASS CLARINET', 'BASS DRUM', 'BASSOON', 'CELLO', 'CLARINET', 'CONTRABASS',
                'ELECTRIC', 'EUPHONIUM', 'FLUTE', 'HORN', 'OBOE', 'ORGAN', 'PIANO', 'SOUSAPHONE', 'TENOR SAX',
                'TROMBONE', 'TRUMPET', 'TUBA')
# Milestone 4: the older fighters, in the roster for their ports (not offered until registry.rs PORTED lists them)
OLDER_FIGHTERS = ('BAGPIPES', 'ACCORDION', 'BARI SAX', 'SYNTH', 'TURNTABLES', 'VIOLA', 'UKULELE', 'VIOLIN', 'HARP',
                  'DRUMS')
RETIRED = ('GUITAR', 'SAX')
# One string equal to a retired fighter's name is expected, and it is not a fighter: ELECTRIC's "pp" names the
# Pixel Pitch voice and model it uses, which pp_bridge calls GUITAR. Any other one stops the export.
NOT_A_FIGHTER = {('ELECTRIC', 'pp'): 'GUITAR'}


def ported():
    """the fighters that have a module in kit/fighters/ (what kit.fighters.ported() lists), read from the folder"""
    folder = os.path.join(common.REPO, 'projects', 'ballfight', 'kit', 'fighters')
    return sorted(f[:-3].replace('_', ' ').upper() for f in os.listdir(folder)
                  if f.endswith('.py') and not f.startswith('_') and f != 'base.py')


def no_retired(fighters):
    """stop if a retired fighter, or a stray string equal to its name, is about to be written"""
    def walk(x, name, path):
        if isinstance(x, dict):
            for k, v in x.items():
                if k in RETIRED:
                    raise SystemExit(f'roster: {name}: a key named {k} at {path or "the top"}')
                walk(v, name, path + (k,))
        elif isinstance(x, (list, tuple)):
            for v in x:
                walk(v, name, path)
        elif isinstance(x, str) and x in RETIRED and NOT_A_FIGHTER.get((name,) + path) != x:
            raise SystemExit(f"roster: {name}: the string {x!r} at {'.'.join(path)}")
    for f in fighters:
        if f['name'] in RETIRED:
            raise SystemExit(f"roster: {f['name']} is retired and must never be written")
        walk({k: v for k, v in f.items() if k != 'name'}, f['name'], ())


def run():
    table = common.roster()
    assert sorted(KIT_FIGHTERS) == ported(), ('the kit\'s ported fighters are no longer the 20 of the spec', ported())
    written = KIT_FIGHTERS + OLDER_FIGHTERS
    assert not set(written) & set(RETIRED) and not set(KIT_FIGHTERS) & set(OLDER_FIGHTERS)
    assert all(n in table for n in written)
    fighters = [dict(name=name, **cfg) for name, cfg in table.items() if name in written]
    assert len(fighters) == len(written) == 30
    no_retired(fighters)
    doc = dict(
        about='The numbers of the kit fighters and the ten older fighters from projects/ballfight/fighters.py, in '
              'its order: "name", then the entry as written there (ints stay ints, floats keep their bits, tuples '
              'are lists). The app offers only the fighters it has ported. Made by tools/studio_export/roster.py; '
              'do not edit by hand.',
        format=1,
        fighters=fighters)
    common.write_json(OUT, doc, sort_keys=False, register=False)
    with open(OUT, encoding='utf-8') as f:                # read it back: the same values, types and key order
        back = json.load(f)
    common.same(doc, back, 'roster', ordered=True)
    assert [f['name'] for f in back['fighters']] == [n for n in table if n in written]
    manifest.add(OUT)
    print(f"roster: {common.rel(OUT)}  {len(fighters)} fighters: {', '.join(f['name'] for f in fighters)}")


EXPORTS = {'roster': run}
