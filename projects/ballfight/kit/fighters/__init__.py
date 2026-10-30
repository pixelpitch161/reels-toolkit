"""Each fighter's look and sound, one module per fighter (kit/fighters/<name>.py, e.g. 'BARI SAX' -> bari_sax.py), as
last built in its own episode. The numbers that decide how it fights stay in the roster (projects/ballfight/fighters.py,
the engine's table); everything that decides how it looks and sounds lives here, so a crossover needs no drawing or
sound code of its own.

Ported so far: FLUTE (#32 v2, as #41 drew it), TROMBONE (#34, as #41), CLARINET and OBOE (#37), ACOUSTIC (#33 v3), ELECTRIC (new in #44), ORGAN (#35 v4), PIANO v2 (#45),
TRUMPET (#29 v3, for #46), EUPHONIUM (#38, for #48), CELLO and CONTRABASS (new in #47), ALTO SAX and TENOR SAX
(new in #51), BASSOON (#39 v4, for #52), SOUSAPHONE (new in #53). Porting another
fighter = moving its draw / sound code from its latest episode folder into its module here (the episode folder stays as
delivered), then checking a few frames against that episode's own renderer (README, "Making an episode")."""
import importlib

from . import base


def slug(name):
    return name.lower().replace(' ', '_')


def module(name):
    """the fighter's module, or None when it isn't ported yet"""
    try:
        return importlib.import_module(f'{__name__}.{slug(name)}')
    except ModuleNotFoundError as e:
        if e.name and e.name.endswith(slug(name)):
            return None
        raise


def look(name, P, k):
    m = module(name)
    if m is None:
        print(f'note: {name} has no look module yet (kit/fighters/{slug(name)}.py): drawing the bare ball')
        return base.Look(P, k)
    return m.Look(P, k)


def sound(name, M, k):
    m = module(name)
    if m is None:
        print(f'note: {name} has no sound module yet (kit/fighters/{slug(name)}.py): silent')
        return base.Sound(M, k)
    return m.Sound(M, k)


def ported():
    import os
    here = os.path.dirname(os.path.abspath(__file__))
    return sorted(f[:-3].replace('_', ' ').upper() for f in os.listdir(here)
                  if f.endswith('.py') and not f.startswith('_') and f != 'base.py')
