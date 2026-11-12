"""theme_stadium_rock: the minor stadium rock's milestone 3 references (Template T, Task 40).

studio/reference/themes/stadium_rock/ (python -m tools.studio_export theme_stadium_rock):

    theme.json      the reference config (ballfight_electric_vs_acoustic, #44, PLUGGED = 'ELECTRIC'): the kit's own
                    recording of it (in the pinned cache, ruling M3-R44) and the bed alone under two kit base Sounds:
                    the 16-bit master's SHA-256 and the buses
    bed.json        the bed alone's RMS per bus and its notes (the bed plays the ACOUSTIC's Pixel Pitch voice itself:
                    the unplugged strum, M.note('ACOUSTIC', ...)); a second tempo, a second key and PLUGGED on the
                    other side (the right corner, ACOUSTIC) (recordings cached); `boom` and `hit` called directly at
                    three times with three gains (vectors cached)

The mood has no method that voices one chord (Bed.root is T.root_above(ch, 40), a number; the strum's notes are
T.chord_tones(64, 79, ch)[:4], the harmony's own rule, inside a method that returns a sound), so there is no
voicings.json. Its one option is PLUGGED (a fighter of the fight, default the left one: `plugged` in a fight file).
It reads the run's events (the lead changes with the 5 HP hysteresis), the story's captions, NAMES and the winner."""
from . import m3refs

SPEC = dict(
    name='stadium_rock',
    configs=('ballfight_electric_vs_acoustic',),
    options=(dict(name='plugged_right', text="PLUGGED = 'ACOUSTIC'\n", ff={'plugged': 'ACOUSTIC'},
                  why="PLUGGED on the right corner's fighter (ACOUSTIC): the arrangements swap leaders"),),
    recordings_in_cache=True,
)

EXPORTS = {'theme_stadium_rock': lambda: m3refs.theme_refs(SPEC)}
