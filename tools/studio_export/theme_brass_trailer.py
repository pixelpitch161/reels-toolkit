"""theme_brass_trailer: the cinematic brass trailer's milestone 3 references (Template T, Task 30).

studio/reference/themes/brass_trailer/ (python -m tools.studio_export theme_brass_trailer):

    theme.json      the two reference configs: examples/trombone_vs_horn (#34, the plain bed on a 4-chord loop) and
                    ballfight_horn_vs_trombone_rematch (#54, THE REMATCH: `TRAILER = dict(sequel=True)`, an 8-chord loop
                    and a 5-bar tail): the bed alone under two kit base Sounds (the 16-bit master's SHA-256 and the
                    buses) and the theme options each config sets (`theme_options`: the rematch's sequel); the kit's
                    own recording of each is in the pinned cache (ruling M3-R43: the two would take the folder over its
                    budget), its SHA-256 committed
    voicings.json   Bed.trem (the high tremolo's three lowest chord tones from C#5) and Bed.pad_notes (the sequel's
                    cello pad, the three lowest from A2) for the 12 roots x 13 qualities; Bed.root and Bed.key_root
                    are numbers (T.root_above(ch, 33), T.KEY_ROOT), proven by the harmony's own tests
    bed.json        the bed alone's RMS per bus; on #34: a second tempo, a second key and the option `sequel` = true
                    (the value the kit has besides its default); `boom` and `trailer_hit` called directly at three
                    times with three gains (vectors cached)

The mood plays no fighter's voice (no brass of its own: the two brass fighters are the brass section)."""
from . import m3refs

SPEC = dict(
    name='brass_trailer',
    voicings=('trem', 'pad_notes'),
    configs=('examples/trombone_vs_horn', 'ballfight_horn_vs_trombone_rematch'),
    options=(dict(name='sequel', text='TRAILER = dict(sequel=True)\n', ff=dict(sequel=True),
                  why='TRAILER = dict(sequel=True): the sequel (#54), darker and bigger, on #34\'s 4-chord loop'),),
    recordings_in_cache=True,
)

EXPORTS = {'theme_brass_trailer': lambda: m3refs.theme_refs(SPEC)}
