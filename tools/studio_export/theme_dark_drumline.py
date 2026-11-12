"""theme_dark_drumline: the dark drumline's milestone 3 references (Template T, Task 32).

studio/reference/themes/dark_drumline/ (python -m tools.studio_export theme_dark_drumline):

    theme.json      the two reference configs: ballfight_flute_vs_sousaphone (#53, the bed as built) and
                    ballfight_bassdrum_vs_tuba (#56: `DRUMLINE = dict(bass_drums=False, after_ko=True)`, the bed's own
                    bass drums off in the fight and back from the K.O. on): the kit's own recording of each (in the
                    pinned cache, ruling M3-R44) and the bed alone under two kit base Sounds: the 16-bit master's
                    SHA-256 and the buses; each config's theme options as the fight file names them (`theme_options`:
                    {'bass_drums': false, 'after_ko': true} for #56, none for #53)
    voicings.json   Bed.bd_notes (the five pitched bass drums: #38's BD_38 table for Cm Ab Fm G, else the four highest
                    chord tones from C3 to Eb4 and the root from G2) for the 12 roots x 13 qualities; Bed.troot
                    (T.root_above(ch, 36)) is a number, not a voicing
    bed.json        the bed alone's RMS per bus; on #53: a second tempo, a second key and the options
                    (`bass_drums` = false; `after_ko` = true alone, which changes nothing while the drums are on);
                    `boom` and `press_roll` called directly at three times with three gains (vectors cached)

The mood plays no fighter's voice (no low brass or woodwinds of its own: the fighters are the band's bass line and
melody). Its `timp` and `marimba` are what BASS DRUM's part doubles its 1812 OVERTURE hymn on (`_pit`, under
`hasattr`): the pair gate of #56 (Task 49) proves that call; the bed alone never makes it."""
from . import m3refs

SPEC = dict(
    name='dark_drumline',
    voicings=('bd_notes',),
    configs=('ballfight_flute_vs_sousaphone', 'ballfight_bassdrum_vs_tuba'),
    options=(dict(name='no_bass_drums', text='DRUMLINE = dict(bass_drums=False)\n', ff=dict(bass_drums=False),
                  why='DRUMLINE = dict(bass_drums=False): the bed\'s own five bass drums off everywhere (#56\'s '
                      'option without its after_ko), on #53'),
             dict(name='after_ko', text='DRUMLINE = dict(after_ko=True)\n', ff=dict(after_ko=True),
                  why='DRUMLINE = dict(after_ko=True) alone: the drums were never off, so the bed is the default '
                      'one (the option matters only with bass_drums=False, which #56 proves), on #53')),
    recordings_in_cache=True,
)

EXPORTS = {'theme_dark_drumline': lambda: m3refs.theme_refs(SPEC)}
