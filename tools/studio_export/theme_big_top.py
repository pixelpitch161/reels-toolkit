"""theme_big_top: the big top's milestone 3 references (Template T's export for the theme milestone 1 ported), and the
chord lists of every kit episode config (milestone 3, Task 2).

studio/reference/themes/big_top/ (python -m tools.studio_export theme_big_top):

    theme.json      the reference config (examples/flute_vs_trombone, the episode of milestone 1): its recording (milestone
                    1's fights/flute_vs_trombone/ep392.bfrec, pointed at, not copied) and the bed alone under two
                    kit base Sounds: the 16-bit master's SHA-256 and the buses
    voicings.json   Bed.voice and Bed.organ for the 12 roots x 13 qualities (the tables first, then the chord-tone rule)
    chords.json     every kit config whose two fighters are kit fighters (the 17 of the milestone 3 plan's Table S4 and
                    milestone 1's): its tempo, bar count, key and scale, CHORDS, and CHORDS split as the fight file
                    writes it (the intro, LOOP, the bars after the last whole loop)

No cache files: everything is small enough to commit."""
from . import common, m3refs

# the kit configs whose two fighters are both kit fighters: milestone 1's, then Table S4's 17
KIT_CONFIGS = (
    'examples/flute_vs_trombone',
    'examples/clarinet_vs_oboe',
    'examples/trombone_vs_horn',
    'ballfight_horn_vs_trombone_rematch',
    'examples/trombone_vs_tuba',
    'examples/bassclarinet_vs_euphonium',
    'ballfight_altosax_vs_tenorsax',
    'ballfight_bassclarinet_vs_bassoon',
    'ballfight_bassdrum_vs_tuba',
    'ballfight_cello_vs_contrabass',
    'ballfight_electric_vs_acoustic',
    'ballfight_euphonium_vs_trombone',
    'ballfight_flute_vs_clarinet',
    'ballfight_flute_vs_sousaphone',
    'ballfight_oboe_vs_bassoon',
    'ballfight_piano_vs_organ',
    'ballfight_trumpet_vs_clarinet',
    'ballfight_trumpet_vs_trombone',
)


def chord_lists(folder, export, module):
    """chords.json: every kit config's chords, split as format 2 of the fight file writes them"""
    rows = [m3refs.read_config(c) for c in KIT_CONFIGS]
    roster = common.roster()
    for r in rows:
        assert all(n in roster for n in r['fighters']), r['config']
        assert len(r['loop']) in (4, 8) and len(r['tail']) <= 8, r['config']
    m3refs.commit_json(folder, 'chords.json', dict(
        about='every kit episode config whose two fighters are kit fighters: BPM, the bar count the kit derives '
              '(loop_bpm, then round(61 / BAR)), KEY, SCALE, CHORDS, and CHORDS as [intro 2] + LOOP * n + tail: '
              '`intro`, `loop` (the config\'s LOOP) and `tail` (the bars after the last whole loop)',
        configs=rows), module, sort_keys=False)
    print(f'{export}: chords.json: {len(rows)} kit configs')


SPEC = dict(
    name='big_top',
    voicings=('voice', 'organ'),
    configs=('examples/flute_vs_trombone',),
    extra=chord_lists,
)

EXPORTS = {'theme_big_top': lambda: m3refs.theme_refs(SPEC)}
