"""theme_second_line: the dark second line's milestone 3 references (Template T, Task 38).

studio/reference/themes/second_line/ (python -m tools.studio_export theme_second_line):

    theme.json      the reference config (ballfight_euphonium_vs_trombone, #48): the kit's own recording of it (in the
                    pinned cache, its SHA-256 committed: ruling M3-R44) and the bed alone under two kit base Sounds: the
                    16-bit master's SHA-256 and the buses
    bed.json        the bed alone's RMS per bus and the TUBA notes it plays (the sousaphone is the Pixel Pitch tuba,
                    M.note('TUBA', ...)); a second tempo and a second key (recordings cached); `boom`, `hit` and
                    `press_roll` called directly at three times with three gains (vectors cached)

The mood has no method that voices one chord (Bed.root is T.root_above(ch, 36) and Bed.fifth the root plus the chord's
fifth, both numbers), so it has no voicings. It has no options (it reads nothing of the config but the music) and its
bed plays the TUBA's voice."""
from . import m3refs

SPEC = dict(
    name='second_line',
    configs=('ballfight_euphonium_vs_trombone',),
    recordings_in_cache=True,
)

EXPORTS = {'theme_second_line': lambda: m3refs.theme_refs(SPEC)}
