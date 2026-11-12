"""theme_mambo: the big-band mambo's milestone 3 references (Template T, Task 39).

studio/reference/themes/mambo/ (python -m tools.studio_export theme_mambo):

    theme.json      the reference config (ballfight_trumpet_vs_trombone, #50): the kit's own recording of it (pinned in
                    the cache, ruling M3-R44) and the bed alone under two kit base Sounds: the 16-bit master's SHA-256
                    and the buses
    voicings.json   Bed.tones (the chord's tones from G3 up, one octave: the piano stab's and the montuno's notes come
                    from it at other floors) for the 12 roots x 13 qualities; Bed.root (T.root_above(ch, 36)) and
                    Bed.fifth are numbers, not voicings
    bed.json        the bed alone's RMS per bus and its notes (the bed plays three Pixel Pitch voices itself: the
                    PIANO's, the CONTRABASS's and the old alto's, `M.note('SAX', ...)`, whose id carries a retired
                    fighter's name and stays an internal id); a second tempo and a second key (recordings cached);
                    `boom` and `hit` called directly at three times with three gains (vectors cached)
    alto_sax.json   the config with the TRUMPET replaced by the ALTO SAX (its line, role, solo and title hits), fought
                    and scored by the kit with BOTH real parts: the ALTO SAX's part and the bed's alto draw from one
                    generator (altosax60_lowpoly._RN), and the kit measures the ALTO SAX's level when its part is made,
                    before the bed's first alto note. The master, buses, notes and noise of that soundtrack (recording
                    and stems cached)

The mood has no options (it reads the config's music and the run's `ability` events: a timbale fill into each)."""
from . import m3refs

NAME = 'mambo'
CONFIG = 'ballfight_trumpet_vs_trombone'

# the ALTO SAX fight: the TRUMPET's seat taken by the ALTO SAX, everything the config gives the TRUMPET given to it
ALTO_FIGHTERS = ('ALTO SAX', 'TROMBONE')
ALTO_HANDICAP = (1.0, 1.0)
ALTO_TEXT = (f"FIGHTERS = {ALTO_FIGHTERS!r}\nDMG_SIDE = {ALTO_HANDICAP!r}\n"
             "ROLES = {'ALTO SAX': ROLES['TRUMPET'], 'TROMBONE': ROLES['TROMBONE']}\n"
             "LINES = {'ALTO SAX': LINES['TRUMPET'], 'TROMBONE': LINES['TROMBONE']}\n"
             "SOLO = {'ALTO SAX': SOLO['TRUMPET'], 'TROMBONE': SOLO['TROMBONE']}\n"
             "TITLE_HITS = {'ALTO SAX': TITLE_HITS['TRUMPET'], 'TROMBONE': TITLE_HITS['TROMBONE']}\n")
# the buses whose RMS alto_sax.json keeps (the master's SHA-256, the notes and the noise cover the rest): the two
# that share the alto generator, and the master
ALTO_RMS = ('master', 'sax', 'alto sax')


def alto_sax_fight(folder, export, module):
    """alto_sax.json: the ALTO SAX fight under the mambo, both real parts (see the module's text)"""
    from . import fights, m3parts as MP, recording
    with open(m3refs.config_source(CONFIG), encoding='utf-8', newline='') as fh:
        text = fh.read()
    vfold = m3refs.config_folder(f'theme_{NAME}_alto_sax', m3refs.config_slug(CONFIG), text + (
        '\n# ---------------------------------------------------------------- tools/studio_export theme_mambo\n'
        + ALTO_TEXT))
    ep, story = m3refs.load_config(vfold)
    assert tuple(ep.T.FIGHTERS) == ALTO_FIGHTERS, ep.T.FIGHTERS
    r = story.run_seed(ep.seed)
    rules = story.rules()
    if r['t_ko'] is None:
        raise SystemExit(f'{export}: the ALTO SAX fight (seed {ep.seed}) has no K.O.: pick another handicap')
    rpath = m3refs.cache_dir(folder, 'alto_sax', 'run.bfrec')
    recording.write_bfrec(r, rules, rpath, rect_keys=recording.rect_keys(story), register=False)
    rec = recording.read_bfrec(rpath)
    fights.check(r, rules, rec, recording.rect_keys(story))
    m3refs.pin(rpath, export)
    stems = m3refs.cache_dir(folder, 'alto_sax', 'stems.npz')
    got = MP.fresh(MP.full_sound_child, m3refs.Loaded(vfold, ALTO_FIGHTERS, ep.seed), stems, rpath)
    m3refs.pin(stems, export)
    facts = m3refs.read_config(CONFIG)
    swap = {'TRUMPET': 'ALTO SAX', 'TROMBONE': 'TROMBONE'}
    config = dict(facts, fighters=list(ALTO_FIGHTERS))
    for key in ('lines', 'roles', 'solos', 'title_hits'):
        if key in facts:
            config[key] = {swap[k]: v for k, v in facts[key].items()}
    assert got['fighters'] == list(ALTO_FIGHTERS) and got['seed'] == ep.seed, (got['fighters'], got['seed'])
    m3refs.commit_json(folder, 'alto_sax.json', dict(
        about=f"{CONFIG} with the TRUMPET's seat taken by the ALTO SAX (its LINES, ROLES, SOLO and TITLE_HITS), "
              f"DMG_SIDE {ALTO_HANDICAP}, fought by the kit and scored with both real parts (m3parts.full_sound_child): "
              f"the ALTO SAX's part and the bed's alto share altosax60_lowpoly._RN, and the part's level is measured "
              f"when it is made, before the bed's first alto note. `config` is what the fight file takes (as "
              f"theme.json's configs); `rms` keeps only {', '.join(ALTO_RMS)}",
        tool=module, format=1, theme=NAME, config=config, appended=ALTO_TEXT, seed=int(ep.seed), winner=rec['winner'],
        t_ko=rec['t_ko'], recording=m3refs.cache_rel(rpath), rec_hash=recording.rec_hash(rec),
        stems_cache=m3refs.cache_rel(stems), master_sha256=got['master_sha256'], buses=got['buses'],
        stems=got['stems'], rms={k: v for k, v in got['rms'].items() if k in ALTO_RMS}, notes=got['notes'],
        noise=got['noise'], windows=got['windows']), module)
    print(f"{export}: alto_sax.json: seed {ep.seed}, winner {rec['winner']} at {rec['t_ko']} s, buses "
          f"{', '.join(got['buses'])}, {got['notes']['count']} first-time notes, master {got['master_sha256'][:16]}...")


SPEC = dict(
    name=NAME,
    voicings=('tones',),
    configs=(CONFIG,),
    extra=alto_sax_fight,
    recordings_in_cache=True,
)

EXPORTS = {'theme_mambo': lambda: m3refs.theme_refs(SPEC)}
