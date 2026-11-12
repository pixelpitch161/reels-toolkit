"""theme_bossa_noir: the minor bossa noir's milestone 3 references (Template T, Task 29).

studio/reference/themes/bossa_noir/ (python -m tools.studio_export theme_bossa_noir):

    theme.json      the reference config (ballfight_flute_vs_clarinet, #49): the kit's own recording of it and the bed
                    alone under two kit base Sounds: the 16-bit master's SHA-256 and the buses
    voicings.json   Bed.voicing (the nylon strings' four-note shape, E3 up) for the 12 roots x 13 qualities
    bed.json        the bed alone's RMS per bus; a second tempo and a second key (recordings cached); `boom` and `hit`
                    called directly at three times with three gains (vectors cached)

The mood has no options (it reads nothing of the config but the music) and plays no fighter's voice."""
from . import m3refs

SPEC = dict(
    name='bossa_noir',
    voicings=('voicing',),
    configs=('ballfight_flute_vs_clarinet',),
)

EXPORTS = {'theme_bossa_noir': lambda: m3refs.theme_refs(SPEC)}
