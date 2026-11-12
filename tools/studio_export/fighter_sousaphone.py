"""fighter_sousaphone: SOUSAPHONE's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the template's defaults (m3refs.py's docstring): the
big top config of the milestone 1 example at the neutral handicap. One addition: its march tramples the enemy's
baskets (`fight.py` `_trample_baskets`, and COMPANY FRONT's wall), and FLUTE is the fighter with baskets, so a
trampled basket (`trample_basket`) is a required row of the coverage table.

The left fight is seed 5, not the lowest seed the SOUSAPHONE wins with all three abilities. At the neutral handicap
its fights against FLUTE are short (median K.O. 30.5 s of 2,000 seeds), and the check part runs the kit's own
`music.main()`, which prints the level of the section from 44 s to the K.O. and raises on an empty one. Seed 5 is
the lowest of the statistics seeds where the SOUSAPHONE wins from the left, COMPANY FRONT fires and the K.O. comes
after 44 s (44.55 s; 7 of the 2,000 do). Handicap and damage scale stay the template's.

The left recording on disk. m3refs.fighter_refs stages `left.bfrec` for finish(), and then its picture, sound and
check parts read that file from the folder: fine for FLUTE and TROMBONE, whose left fights are files the set
already held, but a new fighter's folder has no `left.bfrec` yet when the parts start. So, in an export or remake
run only (never in a cache run, which writes no committed file), this module also writes the staged bytes to the
folder at once: the same bytes finish() writes at the end. It patches m3refs.commit_bytes for the length of its own
call and edits no shared file.

The padded sprite's size. sprites.check_cases (called by m3parts.look_child) expects a look-drawn sprite of radius
r to be 2 r + 5 pixels square, FLUTE's size. The SOUSAPHONE draws its own model through contrabass.padded_sprite
with PAD = 1.22: 2 * int(round(r * PAD)) + 7 pixels. So the look part runs through this module's look_child, which
checks that size itself and hands the shared check the size it expects; every other check there stays as it is.

    python -m tools.studio_export fighter_sousaphone          writes studio/reference/fighters/sousaphone/ and its cache
    python -m tools.studio_export cache fighters/sousaphone   regenerates the cache and checks the pins"""
import os

from . import common, m3parts, m3refs

SPEC = dict(
    name='SOUSAPHONE',
    opponent='FLUTE',
    fights=dict(
        must=('trample_basket',),                # the march tramples FLUTE's baskets (plan, Task 12)
        left=5,                                  # a K.O. after 44 s (the module docstring)
    ),
)
FOLDER = 'fighters/sousaphone'
PAD = 1.22                                        # kit/fighters/sousaphone.py PAD: the sprite's crop margin
_look_child = m3parts.look_child                  # the shared part, before refs() puts look_child in its place


def look_child(*args):
    """m3parts.look_child (in its fresh process: m3parts.start imports this module there), with
    sprites.check_cases given the padded sprite's size rule (the module docstring)"""
    from . import sprites
    shared = sprites.check_cases

    def check_cases(P, k, cases, episode_keys):   # noqa: N803  (the shared function's names)
        seen = []
        for c in cases:
            a = c['args']
            if a['group'] in ('picture', 'episode'):
                assert a['size'] == 2 * int(round(a['radius'] * PAD)) + 7, (c['id'], a)
                a = dict(a, size=2 * a['radius'] + 5)
            seen.append(dict(c, args=a))
        return shared(P, k, seen, episode_keys)

    sprites.check_cases = check_cases
    try:
        return _look_child(*args)
    finally:
        sprites.check_cases = shared


def refs():
    """m3refs.fighter_refs(SPEC), with the left recording on disk before the parts read it and the look part run
    through look_child (the module docstring)"""
    staged = m3refs.commit_bytes

    def commit_bytes(folder, name, data, tool):
        out = staged(folder, name, data, tool)
        if (folder, name) == (FOLDER, 'left.bfrec') and m3refs.run_state().mode != 'cache':
            path = m3refs.ref_dir(folder, name)
            if not (os.path.isfile(path) and open(path, 'rb').read() == data):
                common.write_bytes(path, data)
        return out

    m3refs.commit_bytes = commit_bytes
    m3parts.look_child = look_child
    try:
        return m3refs.fighter_refs(SPEC)
    finally:
        m3refs.commit_bytes = staged
        m3parts.look_child = _look_child


EXPORTS = {'fighter_sousaphone': refs}
