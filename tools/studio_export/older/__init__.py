"""Milestone 4: the reference kit modules of the older fighters and themes (plan m4, Task 2, Gate O).

The kit (projects/ballfight/kit) holds no look, sound or mood for the ten older fighters or the beds of the older
episodes: they live only in their episode folders. A port first writes the module the kit's own README describes
("copy its draw functions from its latest episode's styles.py and its layers from that episode's render.py into the
Look hooks, its part of audio.py into Sound, the bed into a mood") here, outside projects/, and proves it against the
episode (origin.py: Gate O). `install()` then puts it under the name the kit imports, at run time only, so the kit
draws and plays it as if it had always been there. Nothing under projects/ is ever written.

    FIGHTERS    {roster name: its latest episode folder}  (plan m4 Table O1)
    MOODS       {mood name: its episode folder}           (Table O2; 'ballfight' is #27's own folder)
    EPISODES    every episode folder a Gate O of milestone 4 runs (the two tables', in table order)
    install()   idempotent: every older/fighters/<slug>.py and older/moods/<name>.py whose source sets
                `PORTED = True` (a top-level assignment, read without running the file) becomes importable as
                `ballfight.kit.fighters.<slug>` / `ballfight.kit.moods.<name>` (an import hook: kit.fighters.module(name)
                and music.py's `moods.<MOOD>` find it); older/picture_patches.py's registered patches are applied
                once. It writes nothing and imports nothing of the kit itself, so it can run before the kit is imported
                (m3refs and m3parts call it in every child process first). A module with PORTED = False (or none) is not
                installed: the kit draws the bare ball and plays nothing for that fighter, as before.
    installed() {kit module name: source path} of what install() put in place

A module here is written exactly like a kit module (relative imports such as `from . import base` resolve inside the
kit's package, because it is loaded under the kit's name); a fighter whose kit name already exists in the kit is never
shadowed (the kit's own module wins and install() refuses the clash)."""
import ast
import importlib.abc
import importlib.util
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

# ---------------------------------------------------------------- Table O1: each older fighter's latest episode
FIGHTERS = {
    'BAGPIPES': 'ballfight_bagpipes_vs_accordion',      # #30 v3
    'ACCORDION': 'ballfight_bagpipes_vs_accordion',     # #30 v3
    'BARI SAX': 'ballfight_bassoon_vs_barisax',         # #39 v4 (Q3: today's BARI SAX, with `steer`)
    'SYNTH': 'ballfight_turntables_vs_synth',           # #31 v3
    'TURNTABLES': 'ballfight_turntables_vs_synth',      # #31 v3
    'VIOLA': 'ballfight_viola_vs_violin',               # #42
    'UKULELE': 'ballfight_ukulele_vs_acoustic',         # #33 v3
    'VIOLIN': 'ballfight_viola_vs_violin',              # #42 (its styles import #28's)
    'HARP': 'ballfight_organ_vs_harp',                  # #35 v4
    'DRUMS': 'ballfight_drums_vs_violin',               # #28
}

# ---------------------------------------------------------------- Table O2: each older theme's episode
MOODS = {
    'drift_phonk': 'ballfight',                         # #27 (projects/ballfight's own episode)
    'balkan_fire': 'ballfight_drums_vs_violin',         # #28
    'noir_jazz': 'ballfight_trumpet_vs_sax',            # #29 v3
    'sea_shanty': 'ballfight_bagpipes_vs_accordion',    # #30 v3
    'dark_breakbeat': 'ballfight_turntables_vs_synth',  # #31 v3
    'villain_march': 'ballfight_tuba_vs_flute',         # #32 v3
    'spaghetti_western': 'ballfight_ukulele_vs_acoustic',   # #33 v3
    'gothic_horror': 'ballfight_organ_vs_harp',         # #35 v4
    'brass_house': 'ballfight_trombone_vs_tuba',        # #36
    'ska_punk': 'ballfight_bassoon_vs_barisax',         # #39 v4
    'industrial': 'ballfight_barisax_vs_bassclarinet',  # #40 v2
    'orchestral_drill': 'ballfight_viola_vs_violin',    # #42
}

EPISODES = tuple(dict.fromkeys(list(FIGHTERS.values()) + list(MOODS.values())))

FIGHTER_PKG = 'ballfight.kit.fighters'
MOOD_PKG = 'ballfight.kit.moods'


def slug(name):
    """the kit's module name of a fighter: 'BARI SAX' -> 'bari_sax' (kit/fighters/__init__.py's slug)"""
    return name.lower().replace(' ', '_')


def _ported(path):
    """does the file set PORTED = True at the top level? (read, never run)"""
    with open(path, encoding='utf-8') as fh:
        tree = ast.parse(fh.read(), path)
    value = None
    for node in tree.body:
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'PORTED' for t in node.targets):
            value = node.value.value if isinstance(node.value, ast.Constant) else None
    return value is True


def _scan(root=HERE):
    """{kit module name: path} of every ported module in <root>/fighters/ and <root>/moods/ (root: this folder)"""
    out = {}
    for sub, pkg in (('fighters', FIGHTER_PKG), ('moods', MOOD_PKG)):
        d = os.path.join(root, sub)
        if not os.path.isdir(d):
            continue
        for f in sorted(os.listdir(d)):
            if f.endswith('.py') and not f.startswith('_') and _ported(os.path.join(d, f)):
                out[f'{pkg}.{f[:-3]}'] = os.path.join(d, f)
    return out


def _kit_has(fullname):
    """the kit already has a module of this name (a file under projects/ballfight/kit/)"""
    pkg, _, leaf = fullname.rpartition('.')
    sub = pkg.split('.')[-1]
    root = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(HERE))), 'projects', 'ballfight', 'kit', sub)
    return os.path.exists(os.path.join(root, leaf + '.py'))


class _Finder(importlib.abc.MetaPathFinder):
    """finds an installed older module under the kit's package name"""

    def __init__(self):
        self.table = {}

    def find_spec(self, fullname, path=None, target=None):
        src = self.table.get(fullname)
        if src is None:
            return None
        return importlib.util.spec_from_file_location(fullname, src)


_FINDER = None
_PATCHED = False


def install(root=HERE):
    """make every ported older module importable under the kit's name, and apply the picture patches once.
    Idempotent; writes nothing; imports nothing of the kit. Returns installed(). `root`: another folder laid out as this
    one (origin_checks' temporary modules)"""
    global _FINDER, _PATCHED
    table = _scan(root)
    clash = sorted(n for n in table if _kit_has(n))
    if clash:
        raise SystemExit(f"older.install: {', '.join(clash)} already exist in the kit; an older module never shadows "
                         f"a kit module (rename or remove the older one)")
    if _FINDER is None:
        for f in sys.meta_path:                         # (a second copy of this package in the same process)
            if type(f).__name__ == '_Finder' and type(f).__module__ == __name__:
                _FINDER = f
                break
        else:
            _FINDER = _Finder()
            sys.meta_path.insert(0, _FINDER)
    _FINDER.table.clear()
    _FINDER.table.update(table)
    if not _PATCHED:
        from . import picture_patches
        picture_patches.apply()
        _PATCHED = True
    return installed()


def installed():
    return dict(_FINDER.table) if _FINDER is not None else {}
