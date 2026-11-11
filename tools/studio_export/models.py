"""models: the two fighters' low-poly instrument models, as mesh data the Studio ships with.

studio/crates/bf-content/data/models/{FLUTE,TROMBONE}.json: pp_bridge.model_faces(name), the Pixel Pitch model
centred on its bounding box and scaled to fit a unit sphere, face by face, in the order the sprite renderer draws
them: P (k x 3 vertices), n (unit normal), rgb, two (two-sided), emit (unshaded), gid, spec. Every float is written
by its shortest round-trip repr, so it reads back to the same bits.

Each file is read back and compared value for value, and a sprite drawn from the file alone must equal the kit's own
(sprites.render_sprite): the file holds everything the sprite renderer needs.

`model(name)` writes one roster name's file and nothing else (a fighter's port calls it from its own export); the
`models` export writes the two milestone 1 fighters'."""
import json
import os

import numpy as np

from . import common, manifest
from . import frames as F

OUT = os.path.join(common.REPO, 'studio', 'crates', 'bf-content', 'data', 'models')
FORMAT = 1
VIEWS = ((12, 0.0, 0.0), (22, 205.0, 30.0), (30, 96.0, -15.0))       # (radius, yaw, roll) of the proof sprites


def faces_doc(name, faces, entrant, scene_key):
    _, module, _, register, yaw, size = entrant
    rows = []
    for P_, n, rgb, two, emit, gid, spec in faces:
        assert P_.dtype == np.float64 and P_.ndim == 2 and P_.shape[1] == 3 and len(P_) >= 3, P_.shape
        assert n.dtype == np.float64 and n.shape == (3,) and rgb.dtype == np.float64 and rgb.shape == (3,)
        assert type(two) is bool and type(emit) is bool and type(gid) is int and type(spec) is float
        assert abs(float(np.linalg.norm(n)) - 1.0) < 1e-9, 'a normal is not a unit vector'
        rows.append(dict(P=P_.tolist(), n=n.tolist(), rgb=rgb.tolist(), two=two, emit=emit, gid=gid, spec=spec))
    return dict(
        about=f"{name}: pp_bridge.model_faces('{name}'), the Pixel Pitch model the sprite renderer draws, centred "
              f"on its bounding box and scaled to fit a unit sphere",
        format=FORMAT, name=name,
        source=dict(module=module, scene=scene_key, yaw=yaw, size=size),
        fields=dict(P='the face\'s vertices, k x 3 (x, y, z; y up), in drawing order (a fan from the first vertex)',
                    n='its unit normal', rgb='its colour, 0..255 (floats as the bridge gives them; the sprite '
                    'renderer truncates each to an int)', two='drawn from both sides',
                    emit='its colour is used as it is, unshaded', gid='group id: 1 marks the ground group, anything '
                    'else is the instrument (the sprite renderer maps it to 2)',
                    spec='specular strength (0: none)'),
        note='faces are in the order the model builder made them; the sprite renderer draws them in this order '
             '(a later face wins only where it is strictly nearer). Lines (strings) are not part of the model.',
        faces=rows)


def geo_from(doc):
    """the Geo that sprites._geo builds, made from the JSON alone"""
    import piano_lowpoly as PL
    g = PL.Geo()
    for f in doc['faces']:
        g.F.append((np.array(f['P'], float), np.array(f['n'], float), tuple(int(c) for c in f['rgb']), f['two'],
                    f['emit'], 2 if f['gid'] != 1 else 1, f['spec']))
    return g


def check(name, path, doc, faces):
    """the file says what the bridge said, and draws the same sprites"""
    import sprites as SP
    with open(path, encoding='utf-8') as fh:
        back = json.load(fh)
    common.same(common.plain(doc), back, name)
    assert len(back['faces']) == len(faces)
    for f, (P_, n, rgb, two, emit, gid, spec) in zip(back['faces'], faces):
        assert np.array(f['P'], float).tobytes() == P_.tobytes() and np.array(f['n'], float).tobytes() == n.tobytes()
        assert np.array(f['rgb'], float).tobytes() == rgb.tobytes()
        assert (f['two'], f['emit'], f['gid'], f['spec']) == (two, emit, gid, spec)
    SP._FACES.pop(name, None)
    want = [SP.render_sprite(name, r, yaw, roll) for r, yaw, roll in VIEWS]
    kept = SP._FACES[name]
    try:
        SP._FACES[name] = geo_from(back)
        got = [SP.render_sprite(name, r, yaw, roll) for r, yaw, roll in VIEWS]
    finally:
        SP._FACES[name] = kept
    for (r, yaw, roll), a, b in zip(VIEWS, want, got):
        assert a.any() and np.array_equal(a, b), f'{name}: the sprite from the file differs (r {r}, yaw {yaw})'


def model(name, P=None):
    """write studio/crates/bf-content/data/models/<name>.json for one roster name, and only that file: the Pixel
    Pitch model its roster entry names (`pp`), read back and proven by its sprites. `P` is a loaded kit picture
    (frames.picture), made here when not given: the bridge needs the kit's paths. Returns the path"""
    P = P or F.picture('models')
    import pp_bridge as B
    pp = common.roster()[name]['pp']
    os.makedirs(OUT, exist_ok=True)
    faces = B.model_faces(pp)
    again = B.model_faces(pp)
    assert all(a[0].tobytes() == b[0].tobytes() and a[1].tobytes() == b[1].tobytes() for a, b in zip(faces, again))
    doc = faces_doc(pp, faces, B.entrant(pp), B.scene(pp).get('key'))
    path = os.path.join(OUT, f'{name}.json')
    common.write_json(path, doc, register=False)
    check(pp, path, doc, faces)
    manifest.add(path, tool=__name__)
    sides = sorted({len(f['P']) for f in doc['faces']})
    print(f"models: {common.rel(path)}  {len(faces)} faces ({', '.join(map(str, sides))} vertices)  "
          f"{os.path.getsize(path) / 1e3:.0f} kB")
    return path


def run():
    """the two milestone 1 fighters' models (the episode's fighters, whose models carry their own names)"""
    P = F.picture('models')
    for name in sorted(P.NAMES):
        assert P.CFG[P.NAMES.index(name)]['pp'] == name, name
        model(name, P)


EXPORTS = {'models': run}
