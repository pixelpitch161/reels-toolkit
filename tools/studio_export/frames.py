"""frames, title: what the example episode looks like, frame by frame, as the Python kit draws it.

    frames   studio/reference/frames/flute_vs_trombone/f<frame>.png + frames.json: kit.picture.render(frame) for the
             frames `select` picks from the recording (the title card, the doors, the count-in, every ability, weapon
             hits, low HP, a cobra swat, the K.O. and the scripted tail); frames.json gives each frame's number, time,
             the reason it was chosen and tags that say which layers it shows. Also studio/reference/frames/
             exceptions.toml, written once with an empty list and never overwritten (the Rust lane owns it after).
    title    .../title/f<frame>.png + noise.png + title.json: title.draw(noise, t) over one fixed seeded-noise image.
             The card is 94 % opaque over a dimmed arena, so the full frames alone cannot test it.

Both exports check what they are about to write before any file is listed (the required reasons are all present, at
least 40 frames, every PNG reads back as the array that was drawn) and stop with a message if a check fails.

`picture(tag)` and `first_at(t)` are shared with the other picture exports (pillow_vectors, sprites, stages)."""
import io
import math
import os
import sys

import numpy as np

from . import common, manifest

EPISODE = 'flute_vs_trombone'
W, H, FPS, NF = 360, 640, 30, 1830
OUT = common.ref('frames', EPISODE)
INDEX = os.path.join(OUT, 'frames.json')
TITLE_OUT = os.path.join(OUT, 'title')
TITLE_INDEX = os.path.join(TITLE_OUT, 'title.json')
EXCEPTIONS = common.ref('frames', 'exceptions.toml')
RECORDING = common.ref('fights', EPISODE, 'ep392.bfrec')
MIN_FRAMES = 40
NOISE_SEED = 20261110                                  # the title vectors' backdrop
NOISE_CALL = 'np.random.default_rng(seed).integers(0, 256, size=(640, 360, 3), dtype=np.uint8)'
WEAPON_HIT = {'striker': 'fang', 'blade': 'blade'}     # weapon kind -> the `how` of the hits it lands
SLOTS = ('unlock1', 'unlock2', 'low')
EXCEPTIONS_TEXT = '''\
# Golden frames the Studio does not reproduce bit for bit (spec section 7, "Proving the look").
# The bar: a listed frame differs by at most 2 levels on at most 0.5 % of its pixels, and every entry has a reason.
# An entry is one table per frame, replacing the empty list below:
#
#   [[exceptions]]
#   episode = "flute_vs_trombone"
#   frame = 1234
#   reason = "why this frame cannot be made identical"
#
# Written once by `python -m tools.studio_export frames` and never overwritten. After an edit, record the new bytes
# with `python -m tools.studio_export adopt studio/reference/frames/exceptions.toml`.
exceptions = []
'''


# ---------------------------------------------------------------- the kit's renderer
def picture(tag):
    """the kit's renderer (ballfight.kit.picture) for the example episode. It builds everything at import: the run
    (seed 392), every effect, both looks. `tag` names the private copy of the episode the first export of this
    process loads (its caches land there); a later export in the same process gets the same module"""
    P = sys.modules.get('ballfight.kit.picture')
    if P is None:
        common.episode(EPISODE, tag)
        from ballfight.kit import picture as P            # noqa: N812  (the kit's own short name for it)
    assert P.EP.name == EPISODE and P.EP.seed == 392, (P.EP.name, P.EP.seed)
    assert tuple(P.NAMES) == ('FLUTE', 'TROMBONE') and (P.W, P.H, P.FPS, P.T.NF) == (W, H, FPS, NF)
    return P


def first_at(t):
    """the first frame whose time (frame / 30) is at or after t"""
    f = int(math.ceil(t * FPS))
    while f > 0 and (f - 1) / FPS >= t:
        f -= 1
    while f / FPS < t:
        f += 1
    return f


def png_bytes(arr):
    """an (H, W, 3) uint8 array as the bytes of an RGB PNG, written by Pillow with its default settings"""
    from PIL import Image
    assert arr.dtype == np.uint8 and arr.ndim == 3 and arr.shape[2] == 3, (arr.dtype, arr.shape)
    buf = io.BytesIO()
    Image.fromarray(arr).save(buf, 'PNG')
    return buf.getvalue()


def read_png(path):
    """(mode, array) of a PNG on disk"""
    from PIL import Image
    with Image.open(path) as im:
        return im.mode, np.array(im)


def write_png(path, arr):
    """write the array as an RGB PNG, read it back, compare, then list it in the manifest"""
    common.write_bytes(path, png_bytes(arr))
    mode, back = read_png(path)
    assert mode == 'RGB' and back.dtype == np.uint8 and np.array_equal(back, arr), f'{common.rel(path)} read back'
    manifest.add(path, tool=__name__)


def forget_stale(folder, keep, prefix='f'):
    """remove the PNGs of an earlier selection (f*.png in `folder` that `keep` does not name) and their manifest
    lines, so the folder holds exactly what its index lists"""
    stale = [n for n in sorted(os.listdir(folder)) if n.startswith(prefix) and n.endswith('.png') and n not in keep] \
        if os.path.isdir(folder) else []
    if not stale:
        return
    gone = {common.rel(os.path.join(folder, n)) for n in stale}
    for n in stale:
        os.remove(os.path.join(folder, n))
    with manifest._Lock():                                # (the manifest has no call of its own for this)
        entries = manifest.load()
        manifest._save([e for e in entries if e['path'] not in gone])
    print(f"{__name__}: removed {len(stale)} PNGs of an earlier selection: {', '.join(stale)}")


# ---------------------------------------------------------------- which frames
def doors_moving(TT, frames):
    """the frames of `frames` on which the title card is drawn with its doors part-way (neither shut nor gone)"""
    out = []
    for f in frames:
        off = TT.door_offset(f / FPS)
        if off is not None and 0.0 < off < TT.OPEN_PX - 1:
            out.append(f)
    return out


def select(P):
    """(frames, absent): the golden frames, chosen from the recording by fixed rules. `frames` is a list of
    dict(frame, rules, reasons) in frame order (a frame two rules pick is listed once, with both); `absent` lists the
    brief's rules that this fight cannot satisfy, with the reason"""
    T, TT, EV, names = P.T, P.TT, P.EV, P.NAMES
    picks, absent = [], []

    def add(f, rule, reason):
        assert 0 <= f < NF, (f, rule)
        picks.append((int(f), rule, reason))

    def first_event(test):
        return next((e for e in EV if test(e)), None)

    f_vs = first_at(T.TITLE[1][0])
    add(0, 'frame_0', f'frame 0: the title card as {T.TITLE[0][1]} slams in (the loop point)')
    add(13, 'frame_13', f'frame 13: the title card with {T.TITLE[0][1]} landed, before VS slams in on frame {f_vs}')
    add(27, 'frame_27', f'frame 27: the title card as {T.TITLE[2][1]} slams in')
    closed = [f for f in range(first_at(T.T_DOORS[1])) if TT.door_offset(f / FPS) == 0.0]
    opening = doors_moving(TT, range(first_at(T.T_DOORS[1])))
    add(closed[-1], 'doors_last_closed', 'the last frame with the title card shut (the VS pulse on beat 4)')
    add(opening[len(opening) // 2], 'doors_mid', 'the doors half-way through sliding open')
    for tc, txt in T.COUNT:
        add(first_at(tc), 'count_' + txt.strip('!').lower(), f'the first frame of the count-in\'s "{txt}"')
    for k in (0, 1):
        for slot in SLOTS:
            ab = P.CFG[k]['abilities'][slot]['name']
            e = first_event(lambda e: e[1] == 'ability' and e[2] == k and e[3] == ab)
            for d in (0.2, 0.6):
                rule = f'{names[k]}_{slot}_{d}s'
                if e is None:
                    absent.append(dict(rule=rule, why=f'{names[k]} never uses {ab} in this fight'))
                else:
                    add(first_at(e[0] + d), rule, f"{d} s after {names[k]}'s first {ab} (at {e[0]} s)")
        how = WEAPON_HIT[P.CFG[k]['weapon']['kind']]
        e = first_event(lambda e: e[1] == 'hit' and e[2] == k and e[7] == how)
        add(first_at(e[0]), f'{names[k]}_weapon_hit', f"on {names[k]}'s first weapon hit ('{how}', at {e[0]} s)")
        low = [f for f in range(first_at(T.T_FIGHT), first_at(P.T_KO)) if P.hp_at(k, f / FPS) < 25.0]
        add(low[0], f'{names[k]}_low_hp', f"the first frame with {names[k]}'s HP below 25 "
                                          f"({P.hp_at(k, low[0] / FPS):.2f})")
    for kind, what in (('parry', 'parry'), ('swat', 'cobra swat')):
        e = first_event(lambda e: e[1] == kind)
        if e is None:
            absent.append(dict(rule=kind, why=f"the recording has no '{kind}' event"))
        else:
            add(first_at(e[0]), kind, f'on the first {what} (at {e[0]} s)')
    add(first_at(P.T_KO), 'ko', 'the K.O. frame (the first frame of the hit-stop)')
    for d, more in ((0.2, ''), (0.6, ''), (1.5, ' (the WINS caption)')):
        add(first_at(P.T_KO + d), f'ko_{d}s', f'{d} s after the K.O.{more}')
    add(first_at((P.T_SOLO + T.T_REASSEMBLE) / 2), 'solo_mid', "the middle of the winner's solo")
    add(first_at(P.CAP_COMMENT[0][0] + 0.6), 'comment', '0.6 s into the COMMENT caption (both lines landed)')
    add(first_at((T.T_REASSEMBLE + T.T_GLIDE) / 2), 'reassembly_mid', "the loser's pieces half-way home")
    add(first_at((T.T_GLIDE + T.T_REV) / 2), 'glide_mid', 'both balls half-way through the glide to their corners')
    add(1828, 'frame_1828', 'frame 1828: the doors almost shut, name A falling in as a ghost')
    add(1829, 'frame_1829', 'frame 1829: the last frame, the card shut (the loop seam)')
    # more than the brief lists (it asks for at least 40): states its own picks leave out
    t0, who, name = P.STORY['caps'][0]
    add(first_at(t0 + 0.1), 'extra_caption_punch',
        f'extra: the first ability caption on the frame it lands ({name}: the scale punch, the shake, the sand puff)')
    for k in (0, 1):
        e = first_event(lambda e: e[1] == 'level' and e[2] == k and e[3] == 2)
        add(first_at(e[0] + 0.1), f'extra_level_{names[k]}',
            f"extra: 0.1 s after {names[k]} reaches level 2 (the unlock glow sweeping its ability's name)")
    e = first_event(lambda e: e[1] == 'slam')
    if e is not None:
        add(first_at(e[0]), 'extra_slam', f'extra: on the slam of {names[e[2]]} (at {e[0]} s): the bell on the canvas')
    add(first_at(P.T_KO + 0.1), 'extra_ko_caption_punch', 'extra: the K.O. caption on the frame it lands')
    add(first_at(P.CAP_WINS[0][0] + 0.22 + 0.1), 'extra_wins_second_line',
        "extra: the WINS caption's second line on the frame it lands")
    closing = doors_moving(TT, range(first_at(T.DUR - T.T_CLOSE), NF))
    add(closing[len(closing) // 2 - 1], 'extra_doors_closing', 'extra: the doors sliding shut over the last bar')
    frames = []
    for f in sorted({p[0] for p in picks}):
        mine = [p for p in picks if p[0] == f]
        frames.append(dict(frame=f, rules=[p[1] for p in mine], reasons=[p[2] for p in mine]))
    return frames, absent


def required_rules(P):
    """every rule the brief asks for, by id"""
    names = P.NAMES
    out = ['frame_0', 'frame_13', 'frame_27', 'doors_last_closed', 'doors_mid', 'count_3', 'count_2', 'count_1',
           'count_fight', 'parry', 'swat', 'ko', 'ko_0.2s', 'ko_0.6s', 'ko_1.5s', 'solo_mid', 'comment',
           'reassembly_mid', 'glide_mid', 'frame_1828', 'frame_1829']
    for n in names:
        out += [f'{n}_{slot}_{d}s' for slot in SLOTS for d in (0.2, 0.6)] + [f'{n}_weapon_hit', f'{n}_low_hp']
    return out


def check_selection(P, frames, absent):
    """the selection has what the brief asks for; what it lacks is exactly what this fight cannot have"""
    have = {r for fr in frames for r in fr['rules']}
    missing = [r for r in required_rules(P) if r not in have]
    gone = sorted(a['rule'] for a in absent)
    assert sorted(missing) == gone, f'required frames missing without a reason: {sorted(set(missing) - set(gone))}'
    # A parry is the lance cutting a projectile (fight.py kill_keys); neither of these two fighters shoots any, so
    # the engine cannot log one for this pair. Any other absence is a mistake.
    assert set(gone) <= {'parry'}, f'frames this fight should have: {gone}'
    if 'parry' in gone:
        assert not any(e[1] == 'parry' for e in P.EV)
    assert len(frames) >= MIN_FRAMES, f'{len(frames)} frames, the brief asks for at least {MIN_FRAMES}'
    assert [fr['frame'] for fr in frames] == sorted({fr['frame'] for fr in frames})


# ---------------------------------------------------------------- what a frame shows
TAG_MEANINGS = {
    'FLUTE': "an action layer of FLUTE's look is drawn (any FLUTE:* tag but FLUTE:ball and FLUTE:sprite)",
    'TROMBONE': "an action layer of TROMBONE's look is drawn (any TROMBONE:* tag but TROMBONE:ball and "
                "TROMBONE:slide)",
    'FLUTE:ball': 'its glass ball is drawn (its own cool glass tint)',
    'FLUTE:sprite': "its look's own pixel-art flute is drawn in the ball (not the Pixel Pitch model)",
    'FLUTE:basket': 'at least one basket is on the canvas',
    'FLUTE:cobra': 'at least one cobra is up',
    'FLUTE:strike': 'a cobra is lunging or recoiling',
    'FLUTE:swatted': 'a cobra was slapped flat by the lance and is still down or curling back up',
    'FLUTE:dance': 'SNAKE DANCE is running (the cobras leave their baskets)',
    'FLUTE:haste': 'FLUTTER TONGUE is running',
    'FLUTE:wave': "PICCOLO's ring is crossing the canvas",
    'FLUTE:shards': 'its silver shards are flying (it lost)',
    'TROMBONE:ball': 'its glass ball is drawn',
    'TROMBONE:slide': 'its slide lance is drawn (always, while its ball is whole: at rest it still shows)',
    'TROMBONE:thrust': 'the lance is thrusting: the lit lane on the canvas, the thumping ball, the flaring crook',
    'TROMBONE:trail': 'the streak a fast thrust leaves',
    'TROMBONE:wave': "SAD TROMBONE's ring is crossing the canvas",
    'TROMBONE:slow': 'the slowed enemy drags its droopy wobble',
    'TROMBONE:haste': 'POWER SLIDE is running',
    'TROMBONE:slam_decal': "PEDAL TONE's target or stomp ring is on the canvas",
    'TROMBONE:bell': "PEDAL TONE's giant bell is in the frame",
    'TROMBONE:solo': 'it plays its victory solo on the slide',
    'TROMBONE:shards': 'its brass shards are flying (it lost)',
    'card': 'the VS title card is drawn', 'card_closed': 'the card is shut (no HUD under it)',
    'doors': 'the card is sliding open or shut',
    'rev': 'before FIGHT!: both balls rev in their corners', 'fight': 'the fight, up to the K.O.',
    'hitstop': 'the 0.35 s hit-stop on the K.O.', 'tail': 'after the hit-stop', 'solo': "the winner's solo",
    'reassembly': "the loser's pieces fly home", 'glide': 'the last bar: both balls glide to their corners',
    'countdown': 'a count-in number or FIGHT! is on screen',
    'hud': 'the fight HUD and the ability lists are drawn', 'hud:low_hp': 'an HP bar is in its low-HP pulse',
    'hud:sweep': 'an unlock glow sweeps an ability name', 'hud:pill': 'an ability in use glows on its pill',
    'hud:flash': 'an HP bar flashes on a fresh hit', 'hp_numbers': 'at least one HP number is drawn on a ball',
    'caption:ability': 'an ability caption is up', 'caption:ko': 'the K.O. caption is up',
    'caption:wins': 'the WINS caption is up', 'caption:comment': 'the COMMENT caption is up',
    'burst': 'an ink starburst is drawn', 'fringe': 'chromatic fringing on a big hit or the K.O.',
    'moment': 'an ability moment dims the frame (and draws speed lines)',
    'particles': 'reelkit.fx particles are alive', 'marks': 'fading marks are on the canvas',
    'flares': 'an attack light is lit', 'sparks': 'hit sparks are flying', 'shimmer': 'a heat shimmer warps the frame',
    'glass_shards': 'the K.O. shatter is on screen', 'flashbulbs': 'the crowd may fire flashbulbs',
}


def tags(P, f):
    """the tags of frame f (TAG_MEANINGS), read off the kit's own state functions at that frame's time"""
    T, TT, RUN = P.T, P.TT, P.RUN
    t = f / FPS
    fs = [P.fighter(k, t) for k in (0, 1)]
    i = P.sim_i(t)
    in_fight = T.T_FIGHT <= t < P.T_HS
    tc = P.fx_clock(t)
    out = set()
    off = TT.door_offset(t)
    card = off is not None and off < TT.OPEN_PX - 1
    closed = card and off == 0.0
    if card:
        out |= {'card', 'card_closed' if closed else 'doors'}
    out.add('rev' if t < T.T_FIGHT else 'fight' if t < P.T_KO else 'hitstop' if t < P.T_HS else 'tail')
    if P.T_SOLO <= t < T.T_REASSEMBLE:
        out.add('solo')
    if T.T_REASSEMBLE <= t < T.T_GLIDE:
        out.add('reassembly')
    if t >= T.T_GLIDE:
        out.add('glide')
    count = list(T.COUNT)
    if any(tc_ <= t < (count[j + 1][0] if j + 1 < len(count) else tc_ + 0.85) for j, (tc_, _) in enumerate(count)):
        out.add('countdown')
    waves = RUN['WAVES'][i] if in_fight else []
    slams = RUN['SLAMS'][i] if in_fight else []
    shown = [st['vis'] and st['scale'] >= 0.05 for st in fs]
    for k, n in enumerate(P.NAMES):
        if shown[k]:
            out.add(f'{n}:ball')
        if in_fight and t < P.T_KO and fs[k]['haste'] > 0:
            out.add(f'{n}:haste')
        if any(w[0] == k for w in waves):
            out.add(f'{n}:wave')
    parts = P.parts_at(t)
    if parts:
        out.add('glass_shards')
        if len(P.LOOKS[P.LOSE].shards):
            out.add(f'{P.NAMES[P.LOSE]}:shards')
    k = P.NAMES.index('FLUTE')
    L = P.LOOKS[k]
    if shown[k]:
        out.add('FLUTE:sprite')
    cobs = L.cobras_at(t, fs)
    if any(bk > 0.02 for home, bk, cp, t_drop, strike, aim, bid in cobs):
        out.add('FLUTE:basket')
    for home, bk, cp, t_drop, strike, aim, bid in cobs:
        if bk > 0.02 and t > t_drop:
            out.add('FLUTE:cobra')
        if strike is not None:
            out.add('FLUTE:strike')
        if bid < 100 and L.swat_lean(bid, min(t, P.T_KO)) is not None:
            out.add('FLUTE:swatted')
    if t < P.T_KO and L.dancing_k(t) > 0:
        out.add('FLUTE:dance')
    k = P.NAMES.index('TROMBONE')
    L, st = P.LOOKS[k], fs[k]
    if shown[k] and not st['shattered']:
        out.add('TROMBONE:slide')
        prev = P.fighter(k, max(0.0, t - 1.0 / FPS))
        if st['ext'] - prev['ext'] > 0.12 and abs((st['th'] - prev['th'] + math.pi) % (2 * math.pi) - math.pi) < 0.3:
            out.add('TROMBONE:trail')
    if in_fight and st['thr_k'] > 0.05:
        out.add('TROMBONE:thrust')
    if in_fight and fs[1 - k]['slow'] > 0:
        out.add('TROMBONE:slow')
    if any(s[0] == k for s in slams):
        out.add('TROMBONE:slam_decal')
    from ballfight.kit.fighters import trombone
    if any(s[0] == k and trombone.bell_height(t, s[3], s[4]) is not None for s in slams):
        out.add('TROMBONE:bell')
    if P.WIN == k and P.T_SOLO <= t < T.T_REASSEMBLE:
        out.add('TROMBONE:solo')
    for n, rest in (('FLUTE', ('FLUTE:ball', 'FLUTE:sprite')), ('TROMBONE', ('TROMBONE:ball', 'TROMBONE:slide'))):
        if any(x.startswith(n + ':') and x not in rest for x in out):
            out.add(n)
    # the shared layers
    if not closed:
        out.add('hud')
        rows = [r for k in (0, 1) for r in P.ability_rows(k, t)]
        if any(r['state'] == 'unlocked' and r['sweep'] is not None and 0.0 <= r['sweep'] < 1.0 for r in rows):
            out.add('hud:sweep')
        if any(r['state'] == 'unlocked' and r['glow'] > 0.004 for r in rows):
            out.add('hud:pill')
        ko = [k == P.LOSE and P.T_KO <= t < T.T_GLIDE for k in (0, 1)]
        if any(P.hp_at(k, t) <= 25.0 and not ko[k] for k in (0, 1)):
            out.add('hud:low_hp')
        if any(e[7] != 'key' and e[7] not in P.LOOKS[e[2]].quiet_hits
               for _, e in P.events_between(t - 0.12, t, ('hit',))):
            out.add('hud:flash')
        if any(shown[k] and fs[k]['scale'] >= 0.95 and not (T.T_FIGHT - 0.05 <= t < T.T_FIGHT + 0.85)
               and not P.under_something(k, t) for k in (0, 1)):
            out.add('hp_numbers')
    for name, caps in (('ability', P.CAP_ABIL[0] + P.CAP_ABIL[1]), ('ko', P.CAP_KO), ('wins', P.CAP_WINS),
                       ('comment', P.CAP_COMMENT)):
        if any(t0 <= t < t1 for t0, t1, lines in caps):
            out.add('caption:' + name)
    P.BU._assign()
    if any(int((t - t0) * FPS + 1e-6) < P.BU.frames for t0, e in P.BU.tl.alive(t)):
        out.add('burst')
    if any(P.LOOKS[e[2]].fringe(e) for _, e in P.events_between(t - 0.12, t, ('hit',))) or P.T_KO <= t < P.T_HS + 0.3:
        out.add('fringe')
    if P.MO.dim_k(t) > 0.005:
        out.add('moment')
    if len(P.PA.alive(tc)[1]):
        out.add('particles')
    if P.MK.live(tc):
        out.add('marks')
    if P.FL.at(tc):
        out.add('flares')
    if P.events_between(t - 0.32, t, ('hit', 'parry', 'block', 'wall', 'cleared', 'fade')):
        out.add('sparks')
    if (T.T_FIGHT <= t < P.T_HS and RUN['WAVES'][P.sim_i(min(t, P.T_KO))]) or \
            any(e[1] == 'slam' and 0 <= t - e[0] < 0.6 and t < P.T_HS for e in P.EV):
        out.add('shimmer')
    if len(P.FLASH_T[(P.FLASH_T <= t) & (P.FLASH_T > t - 0.1)]):
        out.add('flashbulbs')
    unknown = out - set(TAG_MEANINGS)
    assert not unknown, unknown
    return sorted(out)


# ---------------------------------------------------------------- frames
def timeline(P):
    """the times the selection rules read, as exact floats"""
    T = P.T
    h = common.hexf
    return dict(beat=h(T.BEAT), bar=h(T.BAR), t_fight=h(T.T_FIGHT), t_ko=h(P.T_KO), t_hitstop_end=h(P.T_HS),
                t_solo=h(P.T_SOLO), t_reassemble=h(T.T_REASSEMBLE), t_glide=h(T.T_GLIDE), t_rev=h(T.T_REV),
                t_doors=[h(T.T_DOORS[0]), h(T.T_DOORS[1])], t_close=h(T.T_CLOSE), dur=h(T.DUR),
                count=[[h(tc), txt] for tc, txt in T.COUNT], title=[[h(t0), w] for t0, w in T.TITLE])


def run():
    P = picture('frames')
    frames, absent = select(P)
    check_selection(P, frames, absent)
    names = {f'f{fr["frame"]:04d}.png' for fr in frames}
    forget_stale(OUT, names)
    rows, shots = [], {}
    for fr in frames:
        f = fr['frame']
        img = P.render(f)
        assert img.shape == (H, W, 3) and img.dtype == np.uint8, (f, img.shape, img.dtype)
        shots[f] = img
        write_png(os.path.join(OUT, f'f{f:04d}.png'), img)
        rows.append(dict(frame=f, t=f / FPS, file=f'f{f:04d}.png', rules=fr['rules'],
                         reason='; '.join(fr['reasons']), tags=tags(P, f)))
        print(f"frames: f{f:04d}  {rows[-1]['reason']}")
    # a frame is a pure function of its number: drawn again, last to first, the same pixels
    again = [frames[j]['frame'] for j in sorted({0, len(frames) // 2, len(frames) - 1}, reverse=True)]
    for f in again:
        assert np.array_equal(P.render(f), shots[f]), f'frame {f} drawn a second time differs'
    doc = dict(
        about='golden frames of the example episode as the Python kit draws them: kit.picture.render(frame), '
              'internal size, RGB',
        tool='frames', episode=EPISODE, seed=P.EP.seed, fighters=list(P.NAMES), winner=P.NAMES[P.WIN],
        call='kit.picture.render(frame)', fps=FPS, size=[W, H], mode='RGB', nf=NF,
        recording=dict(file=common.rel(RECORDING), note='the same fight: story.run_seed(392)'),
        timeline=timeline(P),
        rule_notes=dict(
            first_at='a frame "at" or "after" a time t is the first frame whose time frame / 30 is at or after t',
            extra='rules named extra_* are picks beyond the brief\'s list (it asks for at least 40 frames)'),
        absent=absent,
        tag_notes='Every frame shows something only FLUTE\'s look and something only TROMBONE\'s look draw (its '
                  'pixel-art sprite and glass, the slide lance at rest; on the title card its panel and close-up), '
                  'so no frame here is drawable with a default look for either fighter. The plain tags FLUTE and '
                  'TROMBONE therefore mark action layers only; the always-on layers have tags of their own.',
        tags=TAG_MEANINGS, frames=rows)
    common.write_json(INDEX, doc)
    if not os.path.exists(EXCEPTIONS):                    # the Rust lane owns it from here on: never overwritten
        common.write_text(EXCEPTIONS, EXCEPTIONS_TEXT)
        manifest.adopt(EXCEPTIONS)
        print(f'frames: wrote {common.rel(EXCEPTIONS)} (an empty list)')
    size = sum(os.path.getsize(os.path.join(OUT, n)) for n in names)
    print(f'frames: {len(rows)} frames, {size / 1e6:.2f} MB of PNG -> {common.rel(OUT)}')
    if absent:
        print('frames: not in this fight: ' + '; '.join(f"{a['rule']} ({a['why']})" for a in absent))


# ---------------------------------------------------------------- the title card's own vectors
def title_cases(P):
    """[(frame, rule, reason)]: each word's slam (its first frame, +2 and +5 frames), the hold, four frames of the
    doors opening and four of them closing"""
    T, TT = P.T, P.TT
    out = []
    for j, (t0, word) in enumerate(T.TITLE):
        f0 = first_at(t0)
        for d in (0, 2, 5):
            out.append((f0 + d, f'slam_{j}_{d}', f'{word} slams in on frame {f0}: {d} frames after'))
    end_open = first_at(T.T_DOORS[1])
    # the hold: the last word has settled (title.slam_k is 1 from 0.26 s after its slam) and its shake has died
    still = [f for f in range(first_at(T.TITLE[-1][0] + 0.26), end_open)
             if TT.door_offset(f / FPS) == 0.0 and TT.shake(f / FPS) == (0.0, 0.0)]
    out.append((still[len(still) // 2], 'hold', 'the hold: every word landed, no shake, the doors shut'))
    opening = doors_moving(TT, range(end_open))
    closing = sorted(set(doors_moving(TT, range(first_at(T.DUR - T.T_CLOSE), NF))) | {NF - 1})   # the last: shut
    for name, fl in (('opening', opening), ('closing', closing)):
        for n, j in enumerate(np.linspace(0, len(fl) - 1, 5)[1:]):
            f = fl[int(round(float(j)))]
            out.append((f, f'{name}_{n}', f'the doors {name}: frame {f} of {fl[0]}..{fl[-1]}'))
    return out


def check_title(cases):
    rules = [r for f, r, why in cases]
    want = [f'slam_{j}_{d}' for j in (0, 1, 2) for d in (0, 2, 5)] + ['hold'] + \
           [f'{n}_{j}' for n in ('opening', 'closing') for j in range(4)]
    assert sorted(rules) == sorted(want), sorted(set(want) ^ set(rules))
    assert len({f for f, r, why in cases}) == len(cases), 'two title cases share a frame'


def run_title():
    P = picture('title')
    TT = P.TT
    cases = title_cases(P)
    check_title(cases)
    noise = np.random.default_rng(NOISE_SEED).integers(0, 256, size=(H, W, 3), dtype=np.uint8)
    names = {f'f{f:04d}.png' for f, rule, why in cases}
    forget_stale(TITLE_OUT, names)
    write_png(os.path.join(TITLE_OUT, 'noise.png'), noise)
    rows = []
    for n, (f, rule, why) in enumerate(sorted(cases)):
        t = f / FPS
        src = noise.copy()
        out = TT.draw(src, t)
        assert np.array_equal(src, noise), 'title.draw changed its input'
        assert out.shape == (H, W, 3) and out.dtype == np.uint8
        assert not np.array_equal(out, noise), f'frame {f}: the card drew nothing'
        write_png(os.path.join(TITLE_OUT, f'f{f:04d}.png'), out)
        off, (sx, sy) = TT.door_offset(t), TT.shake(t)
        rows.append(dict(id=f'c{n:03d}', call='title.draw(noise, t)', args=dict(frame=f, t=common.hexf(t)),
                         file=f'f{f:04d}.png', rule=rule, reason=why,
                         expect=dict(door_offset=common.hexf(off), shake=[common.hexf(sx), common.hexf(sy)])))
        print(f'title: f{f:04d}  {why}')
    doc = dict(
        about='the VS title card alone: title.draw(noise, t) over one fixed seeded-noise image, at the times that '
              'show each word slamming in, the hold, and the doors opening and closing',
        tool='title', episode=EPISODE, seed=P.EP.seed, fps=FPS, size=[W, H], mode='RGB',
        noise=dict(file='noise.png', seed=NOISE_SEED, call=NOISE_CALL),
        note='t is frame / 30. title.draw returns uint8 for a uint8 image. expect.door_offset and expect.shake are '
             'title.door_offset(t) and title.shake(t) at that time.',
        cases=rows)
    common.write_json(TITLE_INDEX, doc)
    size = sum(os.path.getsize(os.path.join(TITLE_OUT, n)) for n in names | {'noise.png'})
    print(f'title: {len(rows)} cards + noise.png, {size / 1e6:.2f} MB of PNG -> {common.rel(TITLE_OUT)}')


EXPORTS = {'frames': run, 'title': run_title}
