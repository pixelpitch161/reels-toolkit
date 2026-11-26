"""theme_dark_tango: the dark tango's milestone 3 references (Template T, Task 33).

studio/reference/themes/dark_tango/ (python -m tools.studio_export theme_dark_tango):

    theme.json      the reference config (ballfight_cello_vs_contrabass, #47): the kit's own recording of it and the bed
                    alone under two kit base Sounds: the 16-bit master's SHA-256 and the buses
    voicings.json   Bed.voicing (the bandoneon's right hand: the chord's tones from D4 up, four notes) for the 12 roots x
                    13 qualities; Bed.root is T.root_above(ch, 38), a number and not a voicing, and the chop's chord
                    (`sorted(set(50 + (pc - 50) % 12 for pc in pcs))[:4]`) is made inside Bed.chop
    stops.json      the tango stops (Bed.stop_times / Bed.apply_stops) without a fighter that asks for them: the bed
                    alone with both sides kit base Sounds whose `stops()` give fixed times (`fixed`: before FIGHT!, on
                    FIGHT!, on a beat, a repeat, two in one beat, one too close to the next beat to cut, one at the
                    K.O.), and the times CELLO's own `stops()` gives on the config's run (`cello`: its rope slams, on
                    the left side only); each with the 16-bit master's SHA-256, the buses, the master's RMS, the notes
                    and the noise stream (stems cached), and the fighter whose line the bed's `theme()` takes
    bed.json        the bed alone's RMS per bus; a second tempo and a second key (recordings cached); `boom` and `hit`
                    called directly at three times with three gains (vectors cached)

The mood has no options (it reads the config's music, its FIGHTERS, ROLES and LINES, and the parts' stops) and plays no
fighter's voice. With both fighters the kit's base Sound (which has no `stops`), the bed alone makes no stop."""
from . import common, m3parts as MP, m3refs

NAME = 'dark_tango'
CONFIG = 'ballfight_cello_vs_contrabass'


def fixed_stops(T, t_ko):
    """the `fixed` variant's stops, left side then right side, as each part's `stops()` gives them (unsorted, with
    times the bed drops): before FIGHT! and at the K.O. (dropped), FIGHT! itself, a time on a beat, a repeat, two
    stops inside one beat, one 30 ms before a beat (its hit plays, its cut is too short and skipped)"""
    beat, tf = T.BEAT, T.T_FIGHT
    left = [tf + 20 * beat, 12.0, tf - 0.5, tf, 12.0]
    right = [t_ko, t_ko - 1.0, 30 * beat - 0.03, 25.0, 25.1]
    return [left, right]


def _stops_child(cfg_folder, stems_out, which):
    """in a fresh interpreter: the bed alone on the config in `cfg_folder` (both fighters the kit's base Sound),
    each Sound given a `stops()` returning the `which` variant's times; the soundtrack's summary, the times given,
    the times the bed kept (Bed.stop_times) and the theme line (Bed.theme())"""
    MP.fresh_check(cfg_folder)
    E, _ = common.kit()
    E.load(cfg_folder)
    from ballfight.kit import music as M              # noqa: N812  (the kit's own short name for it)
    import pp_bridge as B
    for k in (0, 1):
        B.scene(M.CFG[k]['pp'])
    import drums60_lowpoly as D
    assert [type(s).__module__.rsplit('.', 1)[-1] for s in M.SOUNDS] == ['cello', 'contrabass'], M.SOUNDS
    if which == 'fixed':
        given = fixed_stops(M.T, M.T_KO)
    else:
        assert which == 'cello'
        given = [[float(t) for t in M.SOUNDS[0].stops()], []]
        assert not hasattr(M.SOUNDS[1], 'stops')
    sounds = []
    for k in (0, 1):
        s = M.FIGHTER_MODULES.base.Sound(M, k)
        s.stops = (lambda ts: (lambda: list(ts)))(given[k])
        sounds.append(s)
    M.SOUNDS = sounds
    kept = M.bed.stop_times()
    out = MP._soundtrack(M, B, D, stems_out)
    out.update(seed=int(M.EP.seed), names=list(M.NAMES), t_fight=common.hexf(float(M.T.T_FIGHT)),
               t_ko=common.hexf(float(M.T_KO)), given=[[common.hexf(float(t)) for t in g] for g in given],
               kept=[common.hexf(float(t)) for t in kept], theme_line=M.bed.theme())
    return out


def stops(folder, export, module):
    """stops.json: the bed alone under two base Sounds that ask for stops (the fixed times, then CELLO's own)"""
    cfg_folder = m3refs.theme_config_folder(NAME, CONFIG)
    variants = []
    for which, why in (('fixed', 'fixed times on both sides: before FIGHT! and at the K.O. (dropped), FIGHT! itself, '
                                 'a time on a beat, a repeat, two in one beat, one 30 ms before a beat (cut skipped)'),
                       ('cello', "CELLO's own stops() on the config's run (its rope slams), on the left side")):
        path = m3refs.cache_dir(folder, 'stops', f'{which}.npz')
        got = MP.fresh(_stops_child, cfg_folder, path, which)
        m3refs.pin(path, export)
        assert got['kept'], f'{which}: the bed kept no stop'
        variants.append(dict(name=which, why=why, stems_cache=f'{folder}/stops/{which}.npz',
                             **{k: got[k] for k in ('given', 'kept', 'master_sha256', 'buses', 'stems', 'notes',
                                                    'noise', 'windows', 'theme_line')},
                             rms=dict(master=got['rms']['master'])))
        print(f"{export}: stops {which}: {len(got['kept'])} kept of {sum(map(len, got['given']))} given, 16-bit "
              f"master {got['master_sha256'][:16]}...")
    m3refs.commit_json(folder, 'stops.json', dict(
        about=f'kit/moods/{NAME}.py Bed.stop_times / apply_stops: the bed alone on {CONFIG}\'s recording with both '
              f'fighters kit/fighters/base.py Sound, each given a stops() that returns `given` (left side, right side; '
              f'floats as float.hex()); `kept` is what stop_times() made of them (sorted, T_FIGHT <= t < T_KO); the '
              f'16-bit master\'s SHA-256, the buses, the stems, the master\'s RMS, the first-time notes and the noise '
              f'stream as bed.json\'s variants (stem windows cached); `theme_line` is the fighter whose line the bed\'s '
              f'theme() takes (the melody role: theme.json\'s `roles` and `lines`)',
        tool=module, format=1, theme=NAME, config=CONFIG, variants=variants), module)


SPEC = dict(
    name=NAME,
    voicings=('voicing',),
    configs=(CONFIG,),
    extra=stops,
    recordings_in_cache=True,
)

EXPORTS = {'theme_dark_tango': lambda: m3refs.theme_refs(SPEC)}
