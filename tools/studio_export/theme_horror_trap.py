"""theme_horror_trap: the horror trap's milestone 3 references (Template T, Task 35).

studio/reference/themes/horror_trap/ (python -m tools.studio_export theme_horror_trap):

    theme.json      the reference config (ballfight_bassclarinet_vs_bassoon, #55): the kit's own recording of it (seed
                    4: LIGHTS OUT at 38.6 s, then MOUNTAIN KING at 42.8 s, so the bed alone plays its dark bar and its
                    hop-led hats) and the bed alone under two kit base Sounds: the 16-bit master's SHA-256 and the buses
    voicings.json   the mood has no method that voices one chord (Bed.root is T.root_above(ch, 24), a number; the hits'
                    music-box chord is T.chord_tones(88, 100, ch), the harmony's own rule), so `voicings` is empty;
                    `box_motif` holds Bed.box_motif(ch, ch2), the music box's 2-bar lullaby, for each of the 12 roots x
                    13 qualities with the next chord of that list as ch2: bar 1 then bar 2, 16 MIDI numbers
    bed.json        the bed alone's RMS per bus; a second tempo and a second key (recordings cached); `boom`, `hit` and
                    `erupt` called directly at three times with three gains (vectors cached)

The mood has no options (it reads the config's music and the run's LIGHTS OUT / MOUNTAIN KING events) and plays no
fighter's voice."""
import multiprocessing as mp
import os

from . import m3refs

NAME = 'horror_trap'
CONFIG = 'ballfight_bassclarinet_vs_bassoon'


def _motif_child(folder, chords):
    """in a fresh interpreter (a spawn worker): Bed.box_motif(ch, ch2) of the config's bed for each chord, ch2 the
    next chord of `chords` (wrapping), each chord added to T.CHORD_PCS first as the episode's chords are"""
    for k in m3refs.THREAD_VARS:
        os.environ.setdefault(k, '1')
    E, _ = m3refs.common.kit()
    E.load(folder)
    from ballfight.kit import music as M              # noqa: N812  (the kit's own short name for it)
    T = M.T
    for ch in chords:
        if ch not in T.CHORD_PCS:
            T.CHORD_PCS[ch] = E.parse_chord(ch)[1]
    out = {}
    for j, ch in enumerate(chords):
        ch2 = chords[(j + 1) % len(chords)]
        bar1, bar2 = M.bed.box_motif(ch, ch2)
        out[ch] = dict(next=ch2, notes=[int(x) for x in list(bar1) + list(bar2)])
    return out


def box_motifs(folder, export, module):
    """voicings.json: no one-chord voicing method; the box motif for every chord the harmony parses"""
    chords = m3refs.chord_names()
    cfg_folder = m3refs.theme_config_folder(NAME, CONFIG)
    with mp.get_context('spawn').Pool(1) as pool:
        motifs = pool.apply(_motif_child, (cfg_folder, chords))
    m3refs.commit_json(folder, 'voicings.json', dict(
        about=f'kit/moods/{NAME}.py Bed: no method voices one chord (voicings is empty); box_motif: '
              'Bed.box_motif(ch, next) for every chord the harmony parses (12 roots x 13 qualities: the kit\'s '
              'QUALITY order), `next` the following chord of that list (wrapping), both added to T.CHORD_PCS '
              'first as the episode\'s chords are; `notes` is bar 1 then bar 2 as MIDI numbers',
        theme=NAME, roots=list(m3refs.ROOTS), qualities=list(m3refs.common.kit()[0].QUALITY), voicings={},
        box_motif=motifs), module)
    print(f'{export}: voicings.json: box_motif for {len(motifs)} chords')


SPEC = dict(
    name=NAME,
    configs=(CONFIG,),
    extra=box_motifs,
)

EXPORTS = {'theme_horror_trap': lambda: m3refs.theme_refs(SPEC)}
