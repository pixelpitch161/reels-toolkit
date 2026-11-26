"""theme_prog_duel: the prog-rock keyboard duel's milestone 3 references (Template T, Task 36).

studio/reference/themes/prog_duel/ (python -m tools.studio_export theme_prog_duel):

    theme.json      the reference config (ballfight_piano_vs_organ, #45): the kit's own recording of it (a pinned
                    cache file, ruling M3-R43: at 391,704 bytes it would take the folder over its budget) and the bed
                    alone under two kit base Sounds: the 16-bit master's SHA-256 and the buses
    bed.json        the bed alone's RMS per bus; a second tempo and a second key (recordings cached); `boom` called
                    directly at three times with three gains (vectors cached). The class has `boom` and no `hit`

No voicings.json: the mood has no keyboards of its own and no voicing method. Its only pitch rules are Bed.root
(T.root_above(ch, 28)) and Bed.fifth (the root + 7), numbers and not voicings, which the bed alone proves through
every chord the configs play. The mood has no options (it reads nothing of the config but the music) and plays no
fighter's voice."""
from . import m3refs

SPEC = dict(
    name='prog_duel',
    voicings=(),
    configs=('ballfight_piano_vs_organ',),
    recordings_in_cache=True,  # storage plan B (M3-R43): its kit recording is the pinned cache file
)

EXPORTS = {'theme_prog_duel': lambda: m3refs.theme_refs(SPEC)}
