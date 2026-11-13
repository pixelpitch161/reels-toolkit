"""fighter_contrabass: CONTRABASS's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the template's default config (m3refs.py's
docstring). Its part does not branch on its role, so only its kit role is exported.

Three keys differ from the defaults:

- `handicap`: (0.27, 0.4), not (1.0, 1.0) (ruling M3-R34). At (1.0, 1.0) CONTRABASS won 300 of 300 trial fights
  with a median K.O. at 16.6 s, so no natural `right` fight existed and no winning fight lasted to 44 s, which the
  checks need of the `left` fight. Both sides are turned down so the fights are long enough, and CONTRABASS more so
  FLUTE wins a real share. Measured on 300 fights each before choosing: (0.25, 0.4) 4.3 %, (0.27, 0.4) 70.7 %
  (median K.O. 49.2 s), (0.3, 0.4) 99.7 %; (0.65, 1.0) 15.3 %, (0.8, 1.0) 97.7 % (median K.O. under 25 s).
- `left_in_cache` (ruling M3-R36): the left fight (seed 1, 2,352 events, K.O. at 48.7 s) is a 160,802-byte
  recording, and with it the folder was 312,580 bytes, over its 300 KB budget; so it is the pinned cache file
  `left.bfrec` with its SHA-256 in fight.json, and every gate that reads it needs the cache.
- `sprite_size`: the look draws its own model through padded_sprite with PAD = 1.25, so a sprite of radius r is
  2 * int(round(r * 1.25)) + 7 pixels square.

    python -m tools.studio_export fighter_contrabass          writes studio/reference/fighters/contrabass/ and its cache
    python -m tools.studio_export cache fighters/contrabass   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='CONTRABASS',
    opponent='FLUTE',
    fights=dict(handicap=(0.27, 0.4)),           # the reference handicap (ruling M3-R34)
    sprite_size=dict(add=7, pad=1.25),           # kit/fighters/contrabass.py PAD
    left_in_cache=True,                          # ruling M3-R36: the left recording (160,802 B) takes the folder over
)

EXPORTS = {'fighter_contrabass': lambda: m3refs.fighter_refs(SPEC)}
