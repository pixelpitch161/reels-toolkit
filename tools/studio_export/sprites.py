"""sprites: the instrument inside each glass ball, as the Python kit draws it.

studio/reference/sprites/{FLUTE,TROMBONE}.npz (+ .json, the sidecar): RGBA sprites, one case each, in four groups:

    picture   kit.picture.sprite(k, radius, yaw, roll) for radius 12, 16, 22 and 30, yaw every 6 degrees, roll 0 and
              the roll of the fighter's view (plus any roll a look asks for)
    episode   the same call for every (radius, yaw, roll) the episode's 1,830 frames ask for that `picture` lacks
    title     title.big_sprite(k): the close-up on the VS card, made by the look's own close-up function (FLUTE)
              or by the model renderer at the view's yaw and roll (TROMBONE); the case says which (drawn_by)
    model     sprites.render_sprite(name, radius, yaw, roll), the Pixel Pitch model, for a fighter whose look draws
              its own sprite instead (FLUTE: its model is a 1 px line at ball size), so its mesh has sprites to be
              tested against too

The export checks itself before it lists a file: the committed recording is the fight the kit draws (the sprites
the frames ask for depend on where the balls are); the required radii, yaws and rolls are all there; the sprites
the frames ask for (worked out from the kit's state functions) are exactly the ones real renders of sample frames
ask for; each title close-up equals a fresh draw; every array reads back without pickle.

`run_one(name)` writes one fighter's two files and nothing else (`prepare()` once for several); the `sprites`
export writes both milestone 1 fighters'."""
import math
import os

import numpy as np

from . import common, manifest
from . import frames as F

OUT = common.ref('sprites')
RADII = (12, 16, 22, 30)
YAWS = tuple(range(0, 360, 6))
SAMPLE_STEP = 61                           # besides the golden frames, every 61st frame is rendered for the check


def asked(P, f):
    """the sprites frame f asks kit.picture.sprite for: {(k, radius, yaw key, roll)}, by the lines of draw_ball that
    turn a ball's state into them (yaw key: the yaw rounded to a multiple of 6, not wrapped)"""
    S, R = P.S, P.R
    t = f / F.FPS
    cam = P.camera(t)
    out = set()
    for k in (0, 1):
        st = P.fighter(k, t)
        if not st['vis'] or st['scale'] < 0.05:
            continue
        sink = st.get('sink', 0.0)
        pw = S.to_world(st['p'][0], st['p'][1], (R * (1 - 0.75 * sink) if sink else R) + st['lift'])
        rp = R * cam.f / cam.to_cam(pw[None])[0][2] * st['scale']
        if sink:
            rp = rp * (1 - 0.35 * sink)
        yc, sw, roll = P.CFG[k]['view']
        glass = np.array(P.LOOKS[k].glass, float)
        for L in P.LOOKS:
            glass, roll = L.ball(k, st, t, glass, roll)
        yaw = yc + sw * math.sin(1.25 * st['p'][0] + 0.45 * st['p'][1])
        if P.LOOKS[k].ball_sprite(k, st, t, rp, yaw) is not None:
            continue
        out.add((k, int(np.clip(round(rp * P.LOOKS[k].sprite_k), 12, 30)), int(round(yaw / 6.0)) * 6, int(roll)))
    return out


def really_asked(P, frames):
    """{frame: the sprites a real render of it asks for}, by wrapping kit.picture.sprite"""
    out, log = {}, set()
    orig = P.sprite

    def sprite(k, radius_px, yaw_deg, roll):
        log.add((k, int(np.clip(round(radius_px), 12, 30)), int(round(yaw_deg / 6.0)) * 6, int(roll)))
        return orig(k, radius_px, yaw_deg, roll)
    P.sprite = sprite
    try:
        for f in frames:
            log.clear()
            P.render(f)
            out[f] = set(log)
    finally:
        P.sprite = orig
    return out


def rolls(P, k):
    """the rolls ball k's sprite is exported at: 0 (the brief), its view's roll, and any roll a look asks for"""
    roll0 = P.CFG[k]['view'][2]
    out = [0, int(roll0)] + [int(r) for L in P.LOOKS for r in L.sprite_rolls(k, roll0)]
    return sorted(set(out), key=out.index)


def one(P, k, radius, yaw, roll):
    """kit.picture.sprite on an empty cache: a pure function of its arguments. (The kit's cache key wraps the yaw
    into 0..359 but a look's own sprite is drawn from the unwrapped yaw, so on a warm cache a call can return the
    sprite of another yaw 360 degrees away.)"""
    P._SPR.clear()
    a = P.sprite(k, radius, yaw, roll)
    P._SPR.clear()
    return a


def close_up(P, k):
    """(sprite, args): the VS card's close-up of fighter k, title.big_sprite(k) with the title module's own
    sprite cache emptied first, and how it is made. 'closeup': the look gave the card its own close-up function
    (title.CLOSEUP), which takes the radius alone. 'model': sprites.render_sprite at the view's yaw and roll, which
    big_sprite may read back from its file cache (check/sprites_title.npz). Either way the sprite must equal a
    fresh draw made here"""
    import sprites as SP
    TT, name = P.TT, P.NAMES[k]
    r = int(TT.BALLS[k]['r'] * 0.8)
    TT._SPR.clear()
    big = TT.big_sprite(k)
    if name in TT.CLOSEUP:
        fresh, args = TT.CLOSEUP[name](r), dict(k=k, radius=r, drawn_by='closeup')
    else:
        yc, sw, roll = P.CFG[k]['view']
        fresh = SP.render_sprite(P.CFG[k]['pp'], r, float(yc), float(roll))
        args = dict(k=k, radius=r, yaw=yc, roll=roll, drawn_by='model')
    assert np.array_equal(big, fresh), f"{name}: title.big_sprite is not a fresh {args['drawn_by']} draw"
    return big, args


def build(P, k, episode_keys):
    """(cases, arrays, facts) for fighter k"""
    import sprites as SP
    name, pp = P.NAMES[k], P.CFG[k]['pp']
    own = P.LOOKS[k].sprite(12, 0.0, 0.0) is not None           # its look draws the sprite itself
    cases, arrays = [], {}

    def add(group, call, a, **args):
        assert a.dtype == np.uint8 and a.ndim == 3 and a.shape[0] == a.shape[1] and a.shape[2] == 4, a.shape
        cid = f'c{len(cases):04d}'
        arrays[f'{cid}__out'] = np.ascontiguousarray(a)
        cases.append(dict(id=cid, call=call, args=dict(group=group, **args, size=int(a.shape[0])),
                          arrays=dict(out=f'{cid}__out')))

    have = set()
    for roll in rolls(P, k):
        for r in RADII:
            for yaw in YAWS:
                add('picture', 'kit.picture.sprite(k, radius, yaw, roll)', one(P, k, r, yaw, roll), k=k, radius=r,
                    yaw=yaw, roll=roll, key=f'{pp}_{r}_{yaw % 360}_{roll}')
                have.add((r, yaw, roll))
    for r, yk, roll in sorted(episode_keys):
        if (r, yk, roll) not in have:
            add('episode', 'kit.picture.sprite(k, radius, yaw, roll)', one(P, k, r, yk, roll), k=k, radius=r, yaw=yk,
                roll=roll, key=f'{pp}_{r}_{yk % 360}_{roll}')
            have.add((r, yk, roll))
    big, how = close_up(P, k)
    add('title', 'title.big_sprite(k)', big, **how)
    roll0 = P.CFG[k]['view'][2]
    if own:
        for roll in rolls(P, k):
            for r in RADII:
                for yaw in YAWS:
                    add('model', 'sprites.render_sprite(name, radius, yaw, roll)',
                        SP.render_sprite(pp, r, float(yaw), float(roll)), name=pp, radius=r, yaw=yaw, roll=roll)
    else:                                                       # the kit's sprite IS the model's
        for r, yaw, roll in ((12, 0, 0), (30, 204, int(roll0))):
            assert np.array_equal(one(P, k, r, yaw, roll), SP.render_sprite(pp, r, float(yaw), float(roll)))
    facts = dict(fighter=name, k=k, pp=pp, view=list(P.CFG[k]['view']), sprite_k=P.LOOKS[k].sprite_k,
                 drawn_by='look' if own else 'model', rolls=rolls(P, k),
                 episode_keys=[list(x) for x in sorted(episode_keys)])
    return cases, arrays, facts


def check_cases(P, k, cases, episode_keys):
    """the file has what the brief asks for and what the episode draws"""
    pic = {(c['args']['radius'], c['args']['yaw'], c['args']['roll']) for c in cases
           if c['args']['group'] in ('picture', 'episode')}
    want = {(r, yaw, roll) for r in RADII for yaw in YAWS for roll in rolls(P, k)}
    assert want <= pic, f'{P.NAMES[k]}: sprites missing: {sorted(want - pic)[:8]}'
    assert 0 in rolls(P, k) and int(P.CFG[k]['view'][2]) in rolls(P, k)
    assert set(episode_keys) <= pic, f'{P.NAMES[k]}: the frames ask for {sorted(set(episode_keys) - pic)[:8]}'
    assert sum(c['args']['group'] == 'title' for c in cases) == 1, 'the title close-up'
    side = 5 if P.LOOKS[k].sprite(12, 0.0, 0.0) is not None else 7
    for c in cases:
        a = c['args']
        if a['group'] == 'title':                     # says how it is made, and names only what was passed
            assert a['drawn_by'] in ('closeup', 'model') and ('yaw' in a) == ('roll' in a) == \
                (a['drawn_by'] == 'model'), (c['id'], a)
            if a['drawn_by'] == 'closeup':
                continue                              # (its size is the look's own business)
        by_model = a['group'] == 'model' or a.get('drawn_by') == 'model'
        assert a['size'] == 2 * a['radius'] + (7 if by_model else side), (c['id'], a)


def prepare(P=None):
    """(P, rec, per_frame): the kit picture (frames.picture, made here when not given), the committed recording
    proven to be the fight it draws, and the sprites each frame asks for, proven against real renders of sample
    frames. What run_one needs, shared when it writes several fighters"""
    P = P or F.picture('sprites')
    rec = F.check_recording(P)                        # what the frames ask for depends on the fight
    per_frame = {f: asked(P, f) for f in range(F.NF)}
    sample = sorted({fr['frame'] for fr in F.select(P, rec['events'])[0]} | set(range(0, F.NF, SAMPLE_STEP)))
    P._SPR.clear()
    real = really_asked(P, sample)
    P._SPR.clear()
    wrong = [f for f in sample if real[f] != per_frame[f]]
    assert not wrong, f'frames {wrong[:6]}: a render asks for {real[wrong[0]]}, worked out {per_frame[wrong[0]]}'
    print(f'sprites: the sprites worked out for {len(sample)} sample frames are the ones their renders ask for')
    return P, rec, per_frame


def run_one(name, ready=None):
    """write studio/reference/sprites/<name>.npz and .json for one fighter of the picture, and only those files;
    `ready` is prepare()'s result (made here when not given). Returns the two files' size in bytes"""
    P, rec, per_frame = ready or prepare()
    k = list(P.NAMES).index(name)
    try:
        keys = {x[1:] for s in per_frame.values() for x in s if x[0] == k}
        cases, arrays, facts = build(P, k, keys)
        check_cases(P, k, cases, keys)
        npz, js = os.path.join(OUT, f'{name}.npz'), os.path.join(OUT, f'{name}.json')
        common.write_npz(npz, arrays, register=False)
        doc = dict(
            about=f"{name}'s sprites as the Python kit draws them, RGBA: the instrument inside its glass ball "
                  f"(kit.picture.sprite), the VS card's close-up, and the Pixel Pitch model where the look "
                  f"draws its own",
            tool='sprites', episode=F.EPISODE, **facts, radii=list(RADII), yaw_step=6,
            recording=F.recording_note(rec, 'the export read this file and found it equal to the run the kit '
                                            'drew from. episode_keys and the `episode` cases follow from where '
                                            'the balls are in this fight; the other groups do not depend on '
                                            'it'),
            groups=dict(picture='kit.picture.sprite at the four radii, 60 yaws, and each roll in `rolls`',
                        episode='the same call for each of `episode_keys` (what the frames ask for) that '
                                '`picture` does not already hold',
                        title="title.big_sprite(k), the VS card's close-up. args.drawn_by says how it is "
                              "made: 'closeup' = the look's own close-up function, called with the radius "
                              "alone (FLUTE: Look.title_closeup of kit/fighters/flute.py), so the case has "
                              "no yaw or roll; 'model' = sprites.render_sprite(pp, radius, float(yaw), "
                              "float(roll)) at the view's yaw and roll",
                        model='sprites.render_sprite(name, radius, yaw, roll): the Pixel Pitch model, for a '
                              'fighter whose look draws its own sprite'),
            note='Every picture, episode and model case is the call on an empty sprite cache. A title case '
                 'is title.big_sprite(k) with the title module\'s in-memory cache emptied first; for a '
                 'model close-up big_sprite may still read its file cache (check/sprites_title.npz), so the '
                 'export also compares every close-up with a fresh draw. kit.picture.sprite rounds the radius, '
                 'clips it to 12..30 and rounds the yaw to a multiple of 6 (yk), then calls the look\'s own '
                 'sprite(r, float(yk), float(roll)) or, when the look has none, sprites.render_sprite(pp, r, '
                 'float(yk), float(roll)). Its cache key is "<pp>_<r>_<yk % 360>_<int(roll)>" (`key`): the yaw '
                 'is wrapped in the key but not in the call, so a look\'s sprite at yaw -6 and at yaw 354 share '
                 'a key though they are drawn differently; the first one asked for stays. episode_keys lists '
                 '(radius, yk, roll) with yk as the frames give it (negative for FLUTE). Arrays are (size, '
                 'size, 4) uint8: size is 2 * radius + 7 for the model renderer, 2 * radius + 5 for '
                 'FLUTE\'s own sprite and close-up. A pixel is part of the sprite where alpha > 0.',
            cases=cases)
        common.write_json(js, doc, register=False)
        z = np.load(npz, allow_pickle=False)
        assert set(z.files) == set(arrays) == {c['arrays']['out'] for c in cases}
        for key, a in arrays.items():
            b = z[key]
            assert b.dtype == a.dtype and b.shape == a.shape and b.tobytes() == a.tobytes(), key
        manifest.add(npz, tool=__name__)
        manifest.add(js, tool=__name__)
        size = os.path.getsize(npz) + os.path.getsize(js)
        groups = {g: sum(c['args']['group'] == g for c in cases) for g in ('picture', 'episode', 'title', 'model')}
        print(f"sprites: {common.rel(npz)}  {len(cases)} sprites {groups}  drawn by its {facts['drawn_by']}  "
              f"the frames ask for {len(keys)}  {size / 1e3:.0f} kB")
        return size
    finally:
        P._SPR.clear()                    # a later export of this process must not meet a sprite of another yaw


def run():
    ready = prepare()
    total = sum(run_one(name, ready) for name in ready[0].NAMES)
    print(f'sprites: {total / 1e6:.2f} MB -> {common.rel(OUT)}')


EXPORTS = {'sprites': run}
