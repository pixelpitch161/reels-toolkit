"""theme_hot_jazz: the dark hot jazz's milestone 3 references (Template T, Task 37).

studio/reference/themes/hot_jazz/ (python -m tools.studio_export theme_hot_jazz):

    theme.json      the reference config (ballfight_trumpet_vs_clarinet, #46): the kit's own recording of it and the bed
                    alone under two kit base Sounds: the 16-bit master's SHA-256 and the buses
    voicings.json   Bed.voicing (the rhythm strings' four-note chop from D3 up, a dominant's flat ninth on top) for the
                    12 roots x 13 qualities; Bed.root (T.root_above(ch, 36)) and Bed.fifth are numbers, not voicings
    bed.json        the bed alone's RMS per bus and its notes (the bed plays the TUBA's Pixel Pitch voice itself:
                    M.note('TUBA', ...)); a second tempo and a second key (recordings cached); `boom` and `hit` called
                    directly at three times with three gains (vectors cached)

The mood has no options (it reads nothing of the config but the music). It plays one fighter's voice, the TUBA's
(`pp` 'TUBA'), which the Studio takes from the ported TUBA (bf_content::fighters::tuba)."""
from . import m3refs

SPEC = dict(
    name='hot_jazz',
    voicings=('voicing',),
    configs=('ballfight_trumpet_vs_clarinet',),
)

EXPORTS = {'theme_hot_jazz': lambda: m3refs.theme_refs(SPEC)}
