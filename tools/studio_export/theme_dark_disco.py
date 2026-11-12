"""theme_dark_disco: the dark disco's milestone 3 references (Template T, Task 31).

studio/reference/themes/dark_disco/ (python -m tools.studio_export theme_dark_disco):

    theme.json      the reference config (ballfight_altosax_vs_tenorsax, #51): the kit's own recording of it and the bed
                    alone under two kit base Sounds: the 16-bit master's SHA-256 and the buses
    voicings.json   Bed.voicing (the stab / pad voicing: the chord's tones from A3 up, at most four) for the 12 roots x
                    13 qualities; Bed.root is T.root_above(ch, 33), a number and not a voicing
    bed.json        the bed alone's RMS per bus; a second tempo and a second key (recordings cached); `hit` called
                    directly at three times with three gains (vectors cached; the mood's `hit` takes no `ring`, so the
                    shared child leaves it out of the call). The class has no `boom` (ruling M3-R2)

The mood has no options (it reads nothing of the config but the music) and plays no fighter's voice."""
from . import m3refs

SPEC = dict(
    name='dark_disco',
    voicings=('voicing',),
    configs=('ballfight_altosax_vs_tenorsax',),
)

EXPORTS = {'theme_dark_disco': lambda: m3refs.theme_refs(SPEC)}
