"""rules_map: two short Python fights whose recorded `rules` the Rust engine must give from a fight file.

studio/reference/fights/rules_map/<case>.bfrec and index.json.

A recording's `rules` is Python's `vars(rules)` (recording-format.md section 6.1): the attributes a `Rules` object
holds, which are its constructor's and whatever keywords the episode passed. Two of those are attributes only an
episode adds, and whether they are there shows in the content hash:

    no_handicap   the episode with no DMG_SIDE (None): kit/story.py `rules()` passes no `dmg_side`, so the rules
                  have no such attribute. A fight file with no `handicap` line (1.0 each) must give the same map.
    swat_off      the episode with RULES = dict(swat_cobras=False): `setattr` sets the attribute to False, so the
                  rules list `swat_cobras: false`. A fight file with the pair rule `swat_cobras = false` must give
                  the same map.

Each case is the example episode's config with that one line changed (a copy under the cache folder, as `scenarios`
makes its swapped episode; nothing under projects/ is edited), fought by story.run_seed(SEED) with
t_max = t_fight + WINDOW: the recording holds the first WINDOW seconds. index.json lists each case with its file,
the line changed, the seed, the content hash and the `rules` keys. The Rust test is
crates/bf-sim/tests/rules_map.rs."""
import os

from . import common, fights, manifest, recording

EPISODE = 'flute_vs_trombone'
TAG = 'rules_map'
FOLDER = common.ref('fights', 'rules_map')
INDEX = os.path.join(FOLDER, 'index.json')
SEED = 1
WINDOW = 3.0                                              # s of fighting in each recording
CASES = (                                                 # name, the episode.py line, its replacement
    ('no_handicap', 'DMG_SIDE = (2.0, 0.40)', 'DMG_SIDE = None'),
    ('swat_off', 'RULES = dict(swat_cobras=True)', 'RULES = dict(swat_cobras=False)'),
)


def case_episode(name, old, new):
    """the example's episode.py with one line changed, under studio/reference/.cache/episodes/rules_map_<name>/"""
    src = os.path.join(common.REPO, 'projects', 'ballfight', 'examples', EPISODE, 'episode.py')
    with open(src, encoding='utf-8', newline='') as f:
        text = f.read()
    assert text.count(old) == 1, f'{common.rel(src)}: expected {old!r} once'
    data = text.replace(old, new).encode('utf-8')
    folder = os.path.join(common.CACHE, 'episodes', f'{TAG}_{name}', EPISODE)
    dst = os.path.join(folder, 'episode.py')
    if not (os.path.exists(dst) and open(dst, 'rb').read() == data):
        common.write_bytes(dst, data)
    E, _ = common.kit()
    return E.load(folder)


def run():
    rows = []
    for name, old, new in CASES:
        ep = case_episode(name, old, new)
        _, story = common.kit()
        T = ep.T
        assert tuple(T.FIGHTERS) == ('FLUTE', 'TROMBONE'), T.FIGHTERS
        over = dict(t_max=T.T_FIGHT + WINDOW)
        rules = story.rules(**over)
        attrs = vars(rules)
        if name == 'no_handicap':
            assert T.DMG_SIDE is None and 'dmg_side' not in attrs, attrs
            assert attrs.get('swat_cobras') is True, attrs
        else:
            assert attrs.get('swat_cobras') is False and tuple(attrs['dmg_side']) == (2.0, 0.40), attrs
        out = os.path.join(FOLDER, f'{name}.bfrec')
        r = story.run_seed(SEED, **over)
        recording.write_bfrec(r, rules, out, rect_keys=story.REC_KEYS, register=False)
        rec = recording.read_bfrec(out)
        fights.check(r, rules, rec, story.REC_KEYS)
        manifest.add(out)
        rows.append(dict(
            name=name, file=common.rel(out)[len(common.rel(common.REFERENCE)) + 1:], changed=dict(old=old, new=new),
            seed=SEED, overrides=over, frames=len(rec['arrays']['HP']), events=len(rec['events']),
            hash=recording.rec_hash(rec), rules=sorted(rec['rules'])))
        print(f"rules_map: {common.rel(out)}  {os.path.getsize(out)} bytes  frames {rows[-1]['frames']}  "
              f"events {rows[-1]['events']}  hash {rows[-1]['hash']}  rules {rows[-1]['rules']}")
    doc = dict(
        about='Two short Python fights of the example episode with one config line changed each, for the '
              'recorded rules map (vars(rules)) of a fight file with no handicap and of one with the pair rule '
              'swat_cobras = false; made by tools/studio_export/rules_map.py (its docstring has the rules); do not '
              'edit by hand. Each fight is the first "window" seconds after "t_fight".',
        tool='rules_map',
        format=1,
        episode=EPISODE,
        window=WINDOW,
        fights=rows)
    common.write_json(INDEX, doc, sort_keys=False)
    print(f'rules_map: {common.rel(INDEX)}  {len(rows)} fight(s)')


EXPORTS = {'rules_map': run}
