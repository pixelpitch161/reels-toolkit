"""fighter_trumpet: TRUMPET's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the big top config of the milestone 1 example. That
config has no LINES['TRUMPET'], so the template's soundtracks are the part without a line (the valve fingerings).

Three things are TRUMPET's own:

- The handicap is (2.0, 1.0), TRUMPET left (ruling M3-R34). At neutral the TRUMPET won 2.5 % of the decided fights
  against FLUTE and 45 % of the fights had no K.O.; with its hits doubled it wins about four in five and nearly every
  fight is decided, so both look fights and the statistics say something.
- `sprite_size`: its look draws its own sprite, the model at a radius held to 12..19 (`Look.sprite`), so
  2 * clip(r, 12, 19) + 7 pixels.
- One more file than the template's, `sound_line.json` (with the cache files `stems_line.npz` and
  `sound_line_rms.json`), made by the extra part `line_part`: the `left` soundtrack again with a line for the
  TRUMPET, the base config's FLUTE line an octave down, so that the part's other half is proven too (with a line
  the punches play it instead of the valve fingerings; plan, Task 8). The tests `the_line_takes_the_punches` and
  `the_line_stems` (ignored: the cache) in bf-harness/tests/fighter_trumpet.rs read them.

    python -m tools.studio_export fighter_trumpet          writes studio/reference/fighters/trumpet/ and its cache
    python -m tools.studio_export cache fighters/trumpet   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='TRUMPET',
    opponent='FLUTE',
    fights=dict(handicap=(2.0, 1.0)),            # M3-R34: the reference handicap, TRUMPET left
    sprite_size=dict(add=7, clip=(12, 19)),
)


def line_child(cfg, name, stems_out, rec_path):
    """m3parts.sound_child with a line for the fighter: the base config's FLUTE line an octave down"""
    from . import frames as F
    from . import m3parts
    m3parts.fresh_check(cfg.folder)
    ep, _ = cfg.load()
    line = [n - 12 for n in ep.T.LINES['FLUTE']]
    assert name not in ep.T.LINES and all(58 <= n <= 86 for n in line)
    ep.T.LINES[name] = line
    M, B, D = m3parts.load_music(cfg, loaded=True)
    F.check_recording(M.P, rec_path)
    k = list(M.NAMES).index(name)
    assert M.T.LINES[name] == line
    M.SOUNDS[1 - k] = M.FIGHTER_MODULES.base.Sound(M, 1 - k)
    out = m3parts._soundtrack(M, B, D, stems_out)
    out.update(k=k, fighters=list(M.NAMES), seed=int(M.EP.seed), line=line)
    return out


def line_part(spec, folder, export, module, main, left_seed, left_file):
    """sound_line.json and its cache file, staged like the template's parts"""
    import os
    from . import common, m3parts
    name = spec['name']
    stems = m3refs.cache_dir(folder, 'stems_line.npz')
    left_path = os.path.join(common.REPO, *f'{m3refs.REF}/{left_file}'.split('/'))
    doc = m3parts.fresh(line_child, main.with_seed(left_seed).part('line'), name, stems, left_path)
    m3refs.pin(stems, export)
    m3refs.commit_json(folder, 'sound_line.json', dict(
        about=f"{name}'s part with a line (LINES['{name}']: the punches play it instead of the valve fingerings): "
              f"the soundtrack of the `left` fight as sound.json's `left`, with the base config's FLUTE line an "
              f"octave down as its line. The RMS profiles are in the cache (the folder's budget)",
        tool=module, format=1, fighter=name, line=doc['line'],
        left=dict({k: v for k, v in doc.items() if k not in ('rms', 'line')}, cache=f'{folder}/stems_line.npz',
                  rms_cache=f'{folder}/sound_line_rms.json')), module)
    m3refs.cache_json(folder, 'sound_line_rms.json', dict(
        about="the line soundtrack's RMS per 100 ms of every stem and the master (dB, 3 decimals)", rms=doc['rms']),
        export)


EXPORTS = {'fighter_trumpet': lambda: m3refs.fighter_refs(SPEC, extra=line_part)}
