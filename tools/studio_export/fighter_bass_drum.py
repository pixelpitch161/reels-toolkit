"""fighter_bass_drum: BASS DRUM's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the big top config of the milestone 1 example. What
differs from the template's defaults:

- The handicap is (0.5, 0.6), BASS DRUM left (ruling M3-R34). At neutral the BASS DRUM won 60 of 60 decided trial
  fights from the left and 59 of 60 from the right (median K.O. 23 s), so no natural `right` fight existed. Its hits
  at 0.8 of FLUTE's gave it about three in four, but the fights stayed short (median K.O. 27 s) and none of the
  2,000 statistics fights it won from the left with all three abilities lasted to 44 s, which the checks need of
  the `left` fight. Both sides are turned down so the fights are long enough, the drum a little more (trial, 60
  seeds a corner: 38 of 60 from the left, 46 of 60 from the right, median K.O. 43 s); (0.45, 0.55) gave about
  three in five, (0.4, 0.5) under half.
- `sprite_size`: its look draws its own model through `_render` with PAD = 1.18 (`kit/fighters/bass_drum.py`), so
  a sprite of radius r is 2 * int(round(r * 1.18)) + 7 pixels square.
- One more scenario, `vs_tuba` (seeds 1 and 2): the pair rule `swallow_rollers` is on against the TUBA
  (`m3refs.PAIR_RULES`), so the TUBA's bell can gulp the drum mid-roll and the roll ends (`_end_roll(k, 'held')`,
  `fight.py` 5397 to 5399): BASS DRUM's half of that rule, proven by the prefix and coverage gates.
- `voice_notes`: the register's four pitches and 14, a roll (the voice plays midi < 24 as a roll on midi + 24), at
  the template's two velocities (0.9 is the boom) and two held lengths (0.12 and 1.0 s; the boom does not read the
  length and the roll and the tap do, so a third length adds size and no proof).
- One more file than the template's, `variant.json`, made by the extra part `variant_part`: the look's rolling
  sprite is the drum without its mallet (`_nomallet_geo`), and the export proves that model is the first faces of
  the fighter's own model file (`data/models/BASS DRUM.json`) to the bit (the same builder, the same centre, scale
  and turn), so the Studio draws it from that file and no second model file is needed. variant.json holds the
  face counts and the SHA-256 of the variant's faces.

    python -m tools.studio_export fighter_bass_drum          writes studio/reference/fighters/bass_drum/ and its cache
    python -m tools.studio_export cache fighters/bass_drum   regenerates the cache and checks the pins"""
import hashlib

from . import m3refs

SPEC = dict(
    name='BASS DRUM',
    opponent='FLUTE',
    fights=dict(handicap=(0.5, 0.6)),            # M3-R34: the reference handicap, BASS DRUM left
    scenarios=m3refs.default_scenarios('BASS DRUM', 'FLUTE') + [
        dict(name='vs_tuba', seeds=(1, 2), opponent='TUBA'),       # swallow_rollers: a gulp ends a roll
    ],
    voice_notes=dict(pitches=(14, 28, 35, 43, 50), durations=(0.12, 1.0)),
    sprite_size=dict(add=7, pad=1.18),           # kit/fighters/bass_drum.py PAD
    left_in_cache=True,  # storage plan B (M3-R43): its kit recording is the pinned cache file
)


def _faces_digest(faces):
    """the SHA-256 of a list of faces (P, n, colour as whole numbers, two, emit, group id 1 or 2, spec)"""
    import numpy as np
    h = hashlib.sha256()
    for P_, n, col, two, emit, gid, spec in faces:
        h.update(np.asarray(P_, '<f8').tobytes())
        h.update(np.asarray(n, '<f8').tobytes())
        h.update(repr((tuple(int(x) for x in col), bool(two), bool(emit), 2 if gid != 1 else 1,
                       float(spec))).encode('ascii'))
    return h.hexdigest()


def variant_child(cfg):
    """in a fresh process: the look's no-mallet model against the fighter's own model faces"""
    m3refs.fighter_picture(cfg)                  # (puts the series on the path, as the look's sprites need)
    import numpy as np
    import pp_bridge as B
    from ballfight.kit.fighters import bass_drum as BD
    full = B.model_faces('BASS DRUM')
    var = BD._nomallet_geo().F
    assert 0 < len(var) < len(full), (len(var), len(full))
    for i, (a, b) in enumerate(zip(full, var)):
        P_, n, col, two, emit, gid, spec = a
        Q, n2, col2, two2, emit2, gid2, spec2 = b
        same = (P_.tobytes() == np.asarray(Q, float).tobytes() and n.tobytes() == np.asarray(n2, float).tobytes()
                and tuple(int(x) for x in col) == tuple(col2) and bool(two) == bool(two2)
                and bool(emit) == bool(emit2) and (2 if gid != 1 else 1) == gid2 and float(spec) == float(spec2))
        assert same, f'the no-mallet model differs from the full model at face {i}'
    assert _faces_digest(full[:len(var)]) == _faces_digest(var)
    # the sprite drawn from the full model's first faces is the look's own rolling sprite
    import piano_lowpoly as PL
    g = PL.Geo()
    for P_, n, col, two, emit, gid, spec in full[:len(var)]:
        g.F.append((P_, n, tuple(int(c) for c in col), two, emit, 2 if gid != 1 else 1, spec))
    for r, roll in ((12, 0.0), (22, 90.0), (30, 210.0)):
        assert np.array_equal(BD._render(g, r, 10.0, roll), BD._render(BD._nomallet_geo(), r, 10.0, roll)), (r, roll)
    return dict(faces=len(var), of=len(full), sha256=_faces_digest(var))


def variant_part(spec, folder, export, module, main, left_seed, left_file):
    """variant.json: the no-mallet model is the first `faces` faces of the fighter's model file"""
    from . import m3parts
    got = m3parts.fresh(variant_child, main.with_seed(left_seed).part('variant'))
    m3refs.commit_json(folder, 'variant.json', dict(
        about=f"{spec['name']}'s rolling sprite model (kit/fighters/bass_drum.py `_nomallet_geo`: the drum without "
              f"its mallet, centred and scaled like the full model) is the first `faces` faces of "
              f"studio/crates/bf-content/data/models/BASS DRUM.json (of `of`), to the bit: the mallet is the last "
              f"thing the model builder makes. sha256: the variant's faces (P and n as little-endian float64, then "
              f"the whole-number colour, two, emit, group id 1 or 2 and spec)",
        tool=module, format=1, fighter=spec['name'], **got), module)


EXPORTS = {'fighter_bass_drum': lambda: m3refs.fighter_refs(SPEC, extra=variant_part)}
