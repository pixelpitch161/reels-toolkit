"""fighter_sousaphone: SOUSAPHONE's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the template's defaults (m3refs.py's docstring): the
big top config of the milestone 1 example at the neutral handicap (the SOUSAPHONE wins 73.8 % of 2,000 seeds). Three
keys differ from the defaults:

- `must`: its march tramples the enemy's baskets (`fight.py` `_trample_baskets`, and COMPANY FRONT's wall), and
  FLUTE is the fighter with baskets, so a trampled basket (`trample_basket`) is a required row of the coverage
  table.
- `left`: seed 5, fixed. At the neutral handicap the fights are short (median K.O. 30.5 s) and the left fight must
  end after 44 s (the framework checks it); seed 5 is the lowest statistics seed where the SOUSAPHONE wins from the
  left with all three abilities and a K.O. after 44 s (44.55 s).
- `sprite_size`: the look draws its own model through contrabass.padded_sprite with PAD = 1.22, so a sprite of
  radius r is 2 * int(round(r * 1.22)) + 7 pixels square.

    python -m tools.studio_export fighter_sousaphone          writes studio/reference/fighters/sousaphone/ and its cache
    python -m tools.studio_export cache fighters/sousaphone   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='SOUSAPHONE',
    opponent='FLUTE',
    fights=dict(
        must=('trample_basket',),                # the march tramples FLUTE's baskets (plan, Task 12)
        left=5,                                  # fixed (the framework checks its K.O. is after 44 s)
    ),
    sprite_size=dict(add=7, pad=1.22),           # kit/fighters/sousaphone.py PAD
)

EXPORTS = {'fighter_sousaphone': lambda: m3refs.fighter_refs(SPEC)}
