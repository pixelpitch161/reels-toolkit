"""theme_prog_duel: the prog-rock keyboard duel's milestone 3 references (Template T, Task 36).

studio/reference/themes/prog_duel/ (python -m tools.studio_export theme_prog_duel):

    theme.json      the reference config (ballfight_piano_vs_organ, #45): the kit's own recording of it and the bed
                    alone under two kit base Sounds: the 16-bit master's SHA-256 and the buses
    voicings.json   empty: the mood has no keyboards of its own and no voicing method. Its only pitch rules are
                    Bed.root (T.root_above(ch, 28)) and Bed.fifth (the root + 7), numbers and not voicings, which the
                    bed alone proves through every chord the configs play
    bed.json        the bed alone's RMS per bus; a second tempo and a second key (recordings cached); `boom` called
                    directly at three times with three gains (vectors cached). The class has `boom` and no `hit`

The mood has no options (it reads nothing of the config but the music) and plays no fighter's voice."""
from . import common, m3refs

NAME = 'prog_duel'


def no_voicings(folder, export, module):
    """voicings.json with no voicing method: the theme suite's voicing gate reads the file before it looks at the
    methods, so a mood without one still writes it (its `voicings` empty, as theme.json's `voicing_methods`)"""
    m3refs.commit_json(folder, 'voicings.json', dict(
        about=f'kit/moods/{NAME}.py Bed has no voicing method (no chord is voiced: the bass plays roots, fifths and '
              f'octaves by T.root_above(ch, 28)), so no chord is listed',
        theme=NAME, roots=list(m3refs.ROOTS), qualities=list(common.kit()[0].QUALITY), voicings={}), module)
    print(f'{export}: voicings.json: no voicing method')


SPEC = dict(
    name=NAME,
    voicings=(),
    configs=('ballfight_piano_vs_organ',),
    extra=no_voicings,
)


def key_variant_with_loop(facts, key_from):
    """m3refs.key_variant, its appended lines also setting LOOP to the other config's loop. The reference config sets
    LOOP itself, so without this the key variant keeps #45's A-minor LOOP over the new CHORDS: the kit's PIANO ROLL
    (kit/story.py roll_rows) voices each row on T.LOOP's chord and raises KeyError on 'Am', and the Studio's harness
    gives the fight file the other config's loop (bed.json's `ff`), so both sides must play that loop"""
    c, text, ff = _key_variant(facts, key_from)
    head, chords = text.split('CHORDS = ', 1)
    return c, f"{head}LOOP = {ff['chord_loop']!r}\nCHORDS = {chords}", ff


_key_variant = m3refs.key_variant


def export():
    m3refs.key_variant = key_variant_with_loop
    try:
        m3refs.theme_refs(SPEC)
    finally:
        m3refs.key_variant = _key_variant


EXPORTS = {'theme_prog_duel': export}
