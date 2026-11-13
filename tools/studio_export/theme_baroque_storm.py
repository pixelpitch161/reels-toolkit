"""theme_baroque_storm: the baroque storm's milestone 3 references (Template T, Task 28).

studio/reference/themes/baroque_storm/ (python -m tools.studio_export theme_baroque_storm):

    theme.json      the reference config (examples/clarinet_vs_oboe, #37): its recording (committed here unless an
                    identical one is already committed) and the bed alone under two kit base Sounds: the 16-bit
                    master's SHA-256 and the buses
    voicings.json   Bed.harp_rh and Bed.vln for the 12 roots x 13 qualities (the HARP_RH / VLN tables first, then the
                    chord-tone rule); Bed.root is T.root_above(ch, 36), a number and not a voicing, proven by the
                    harmony's own tests
    bed.json        the bed alone's RMS per bus and notes; the variants (a second tempo, a second key: the bed moves
                    its fixed runs by `self.off` from D); `boom` called directly (the only optional hook the class has)

Cache files (pinned in pins.json): the bed's stems, the two variants' recordings and stems, the hook buses."""
from . import m3refs

SPEC = dict(
    name='baroque_storm',
    voicings=('harp_rh', 'vln'),
    configs=('examples/clarinet_vs_oboe',),
    recordings_in_cache=True,  # storage plan B (M3-R43): its kit recording is the pinned cache file
)

EXPORTS = {'theme_baroque_storm': lambda: m3refs.theme_refs(SPEC)}
