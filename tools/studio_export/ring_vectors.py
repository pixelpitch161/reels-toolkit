"""ring: Python's numbers for the ring renderer (`bf-picture`'s `ring`, `scene` and `look` modules).

    python -m tools.studio_export ring          (about a minute: it renders some 80 frames)

writes, under studio/reference/picture_ring/:

unit.json           numbers of kit.picture and scene.py for the example episode drawn with the BASE-CLASS look
                    (kit/fighters/base.py Look) for both fighters, which is what the Studio's DefaultLook is: the
                    scene's geometry, the module-level derivations, fighter(k, t) in each phase, the clocks, the
                    camera, the ropes, the shards, the HUD's rows, the focus check's points, the order of every
                    look hook (while the module builds and in 64 frames), reelkit.ui.DRAWN after each of those
                    frames, and the SHA-256 of each finished frame. Floats are float.hex() strings.
stage_hashes.json   for frames 150, 600 and 1500, the SHA-256 of the buffers kit.picture.render holds after a
                    stage (the bytes of the array as the renderer holds it: C order, little-endian, in the dtype
                    named beside the hash; see stages.py for what each stage is and how it is captured):
                    "base_looks": every drawing stage whose buffer the Studio must equal to the bit, with the
                    base-class look for both fighters; "episode": floor, arena and skirt of the episode as it is
                    (FLUTE's and TROMBONE's own looks). The float stages between the lens and the cut to bytes
                    are not hashed: they pass through the FFT and are compared by value against local dumps.

The base-class numbers come from a second Python process (`python -m tools.studio_export.ring_vectors <file>`):
kit.picture builds everything at import for the looks it finds, and the other exports of this process must keep
the episode's own. In that process `ballfight.kit.fighters.look` is replaced before the renderer is imported;
nothing in the kit is edited. The Studio's tests: studio/crates/bf-picture/src/ring/tests.rs (unit.json) and
studio/crates/bf-picture/tests/ring_stages.rs (stage_hashes.json)."""
import hashlib
import json
import os
import subprocess
import sys

import numpy as np

from . import common
from . import frames as F
from . import stages as ST

OUT = common.ref('picture_ring')
STAGE_FRAMES = (150, 600, 1500)
# the stages whose buffers are whole numbers or bytes drawn without a transcendental in between: bit-exact
EXACT = ('floor', 'arena', 'skirt', 'canvas', 'painter', 'over_ring', 'sparks', 'particles', 'countdown', 'hud',
         'title')
EARLY = ('floor', 'arena', 'skirt')


def stage_hashes(P, names):
    """{frame: {stage: {role: [dtype, sha256]}}} of P.render's buffers after the stages `names`"""
    out = {}
    for f in STAGE_FRAMES:
        rec, _ = ST.capture(P, f)
        out[str(f)] = {name: {role: [a.dtype.str, hashlib.sha256(np.ascontiguousarray(a).tobytes()).hexdigest()]
                              for role, a in rec.got[name].items()} for name in names}
    return out


def base_look_numbers():
    """everything the base-class look gives: (the unit numbers, its stage hashes). Run in a process of its own"""
    common.kit()
    from ballfight.kit import fighters as FM

    LOG = []
    HOOKS = ('setup', 'state', 'ball', 'ball_sprite', 'hides_hp', 'shake', 'rope', 'spark', 'hit_glow', 'fx_setup',
             'fx_event', 'fx_after', 'ko_fx', 'fx_end', 'light', 'shadows', 'shadows_free', 'canvas', 'canvas_wave',
             'canvas_after', 'canvas_overlay', 'items_ball', 'items_free', 'draw_item', 'over_ring', 'draw_shard',
             'shimmer', 'shimmer_after', 'glows', 'fringe', 'fringe_size', 'after_grade')


    class Spy(FM.base.Look):
        """the base-class look, noting every hook call: '<its side>.<hook>' (with the ball and the phase where a hook
        is called per ball)"""


    def spy(name):
        def call(self, *a, **kw):
            tag = f'{self.k}.{name}'
            if name == 'state':
                tag += f'({a[0]},{a[3]})'
            elif name in ('ball', 'ball_sprite', 'hides_hp'):
                tag += f'({a[0]})'
            LOG.append(tag)
            return getattr(FM.base.Look, name)(self, *a, **kw)
        return call


    for _n in HOOKS:
        setattr(Spy, _n, spy(_n))
    FM.look = lambda name, P, k: Spy(P, k)
    P = F.picture('ring_base_looks')
    assert all(type(L) is Spy for L in P.LOOKS), [type(L) for L in P.LOOKS]
    S, T = P.S, P.T
    build_log = [x for x in LOG if '.rope' not in x]     # (the kit calls rope() twice for an event that shakes them)


    def hx(x):
        if isinstance(x, (list, tuple, np.ndarray)):
            return [hx(v) for v in x]
        return float(x).hex()


    def sha(lines):
        return hashlib.sha256('\n'.join(lines).encode()).hexdigest()


    out = {}
    out['about'] = 'kit.picture and scene.py numbers for flute_vs_trombone (seed 392) with the base-class look for both'
    out['spot_cos'] = hx(S.SPOT_COS)
    out['scene'] = dict(
        n_crowd=len(S.CROWD), n_stands=len(S.STANDS), n_barrier=len(S.BARRIER),
        crowd=[[j, hx(S.CROWD[j][0]), S.CROWD[j][1], hx(S.CROWD[j][2]), S.CROWD[j][3], hx(S.CROWD[j][4])]
               for j in (0, 1, 2, 700, len(S.CROWD) - 1)],
        stands=[[j, hx(S.STANDS[j][0]), hx(S.STANDS[j][1]), hx(S.STANDS[j][2]), hx(S.STANDS[j][3])] for j in (0, 1, 777, 1119)],
        barrier=[[j, hx(S.BARRIER[j][0]), hx(S.BARRIER[j][1]), hx(S.BARRIER[j][2])] for j in (0, 1, 127)],
        skirt=[[j, hx(S.SKIRT_FACES[j][0]), hx(S.SKIRT_FACES[j][1]), hx(S.SKIRT_FACES[j][2])] for j in (0, 4, 11)],
        posts=[[c, j, hx(S.POSTS[name][j][0]), hx(S.POSTS[name][j][1]), hx(S.POSTS[name][j][2])]
               for c, name in enumerate(S.CORNERS) for j in (0, 6, 9, len(S.POSTS[name]) - 1)],
        n_posts=[len(S.POSTS[name]) for name in S.CORNERS],
        corner_col=[hx(S.CORNER_COL[name]) for name in S.CORNERS],
        red_light=hx(S.RED_LIGHT), blue_light=hx(S.BLUE_LIGHT),
        stand_c=[[j, hx(P.STAND_C[j])] for j in (0, 1, 1119)], bar_c=[[j, hx(P.BAR_C[j])] for j in (0, 127)],
        static_casters=hx(np.array(P.STATIC_CASTERS)))
    pts = [(0.0, 0.0), (0.3, 0.1), (1.55, 0.0), (0.9, 0.95), (-1.32, 0.01), (2.99, 0.5), (3.01, -0.5), (3.56, 3.0),
           (-3.5, -3.6), (0.2, -0.35), (-0.41, 0.02), (0.6, 0.6)]
    out['albedo'] = [[hx(x), hx(z), hx(S.canvas_albedo(np.array([x]), np.array([z]))[0])] for x, z in pts]
    out['derived'] = dict(
        nsim=P.NSIM, t_ko=hx(P.T_KO), t_hs=hx(P.T_HS), win=P.WIN, t_solo=hx(P.T_SOLO), solo_t=hx(P.SOLO_T),
        rest=hx(P.REST), omega=hx(P.OMEGA), th0=hx(P.TH0), sd0=hx(P.SD0), p_ko=hx(P.P_KO), base_len=hx(P.BASE_LEN),
        frets0=[int(v) for v in P.FRETS0], glide_arc=[[hx(n), hx(a)] for n, a in P.GLIDE_ARC],
        light_rgb=hx(P.LIGHT_RGB), crowd_tint=hx(P.CROWD_TINT), fl_col=hx(P.FL_COL),
        shard_seed=int(T.SHARD_SEED), fx_seed=int(T.FX_SEED),
        n_rope_ev=len(P.ROPE_EV), rope_ev=[[j, hx(P.ROPE_EV[j][0]), int(P.ROPE_EV[j][1])] + hx(P.ROPE_EV[j][2:])
                                           for j in (0, 1, len(P.ROPE_EV) - 1)],
        wave_ev=hx(P.WAVE_EV), n_flash=len(P.FLASH_T), flash_t=hx(P.FLASH_T[[0, 1, 2, 20, -17, -1]]),
        n_status=len(P.STATUS),
        abil_unlock=[[None if P.ABIL_UNLOCK[k].get(s) is None else hx(P.ABIL_UNLOCK[k][s]) for s, _ in P.SLOTS] for k in (0, 1)],
        abil_uses=[[hx(P.ABIL_USES[k][s]) for s, _ in P.SLOTS] for k in (0, 1)],
        abil_hold=[[hx(P.ABIL_HOLD[k][s]) for s, _ in P.SLOTS] for k in (0, 1)],
        shake=[[hx(t), hx(P.SHK.offset(t))] for t in (5.0, 20.0, P.T_KO, P.T_KO + 0.02, P.T_KO + 0.2)],
        n_shake=len(P.SHK.tl.t0), n_flares=len(P.FL.tl.t0), n_bursts=len(P.BU._ev), n_marks=len(P.MK.tl.t0),
        n_moments=len(P.MO.tl.t0), n_particles=len(P.PA.cols['t0']),
        moments=hx(sorted(P.MO.tl.t0)), ko_particle=hx([P.PA.cols['p'][0], P.PA.cols['v'][0], P.PA.cols['life'][0]]))
    times = [0.0, 1.0, 3.5, T.T_FIGHT, 5.0, 20.0, 33.3, P.T_KO - 0.01, P.T_KO, P.T_KO + 0.2, P.T_HS, P.T_HS + 0.5,
             P.T_KO + 3.0, P.T_SOLO + 1.0, T.T_REASSEMBLE + 0.5, T.T_GLIDE - 0.1, T.T_GLIDE + 0.3, T.T_REV + 0.1, 60.9,
             T.T_GLIDE, T.T_GLIDE - 0.12, T.T_REV]        # (the first instants of the glide home and of the rev)
    keys = ('vis', 'shield', 'haste', 'glow', 'scale', 'th', 'blen', 'frets', 'level', 'shattered', 'om', 'sd', 'ext',
            'slow', 'thr_k', 'lift')


    def state(k, t):
        st = P.fighter(k, t)
        d = {key: (hx(st[key]) if isinstance(st[key], (float, np.floating)) else int(st[key])) for key in keys}
        d['p'] = hx(st['p'])
        return d


    out['fighter'] = [[hx(t), state(0, t), state(1, t)] for t in times]
    out['clock'] = [[hx(t), P.sim_i(t), hx(P.fx_clock(t)), hx(P.hp_at(0, t)), hx(P.hp_at(1, t)), hx(P.win_k(t)),
                     hx(P.solo_env(t)), hx(P.spot_aim(t)), hx(P.part_time(t))] for t in times]


    def cam(t):
        c = P.camera(t)
        return [hx(t), hx(c.pos), hx(c.M), hx(c.f), hx(c.cx), hx(c.cy)]


    out['camera'] = [cam(t) for t in (0.0, 5.0, 20.0, 44.0, P.T_KO, P.T_KO + 0.05, 59.0, 61.0)]
    ropes = []
    ro = [e for e in P.EV if e[1] == 'rope']
    for (s, y, t, n) in ((0, 0.42, 0.0, 15), (1, 0.80, 2.0, 5), (2, 1.18, ro[3][0] + 0.05, 15),
                         (P.ROPE_EV[3][1], 0.80, ro[3][0] + 0.05, 15), (P.ROPE_EV[40][1], 0.42, ro[40][0] + 0.2, 5),
                         (3, 0.80, P.WAVE_EV[0][0] + 0.3, 15), (0, 0.42, P.T_KO + 5.0, 15),
                         # the ends of a ring's wobble on the ropes (0.15 < dt < 1.2)
                         (1, 0.42, P.WAVE_EV[0][0] + 0.16, 15), (1, 0.42, P.WAVE_EV[0][0] + 1.17, 15),
                         (1, 0.42, P.WAVE_EV[0][0] + 1.21, 15)):
        ropes.append([int(s), hx(y), hx(t), n, hx(P.rope_points(s, y, t, n))])
    out['ropes'] = ropes
    out['parts'] = dict(
        n=len(P.PARTS), n_steps=int(P.TRAJ.shape[1]),
        parts=[[j, P.PARTS[j]['kind'], hx(P.PARTS[j]['p']), hx(P.PARTS[j]['v']), hx(P.PARTS[j]['rot']), hx(P.PARTS[j]['w']),
                hx(P.PARTS[j]['size']), hx(P.PARTS[j]['curl'])] for j in (0, 1, 23)],
        traj=[[j, i, hx(P.TRAJ[j, i])] for j, i in ((0, 0), (0, 1), (0, 100), (5, 400), (23, 840))],
        at=[[hx(t), [[j, hx(q[1]), hx(q[2]), hx(q[3])] for j, q in enumerate(P.parts_at(t)) if j in (0, 7, 23)]]
            for t in (P.T_HS - 0.01, P.T_HS + 0.3, P.T_HS + 2.0, T.T_REASSEMBLE + 0.4, T.T_GLIDE - 0.2, T.T_GLIDE)])
    row_t = [4.0, 20.0, 33.3, P.T_KO, P.T_KO + 0.2, 58.0, T.T_GLIDE + 0.1]
    for k in (0, 1):
        for s, _ in P.SLOTS:
            tu = P.ABIL_UNLOCK[k].get(s)
            if tu is not None:
                row_t += [tu + 0.1, tu + 0.6, tu, tu + P.SWEEP_T]      # (and the sweep's first and last instant)
            row_t += [te + 0.05 for te in P.ABIL_USES[k][s][:1]]
    out['rows'] = [[hx(t), [[[r['tag'], r['name'], r['state'], hx(r['glow']), None if r['sweep'] is None else hx(r['sweep'])]
                             for r in P.ability_rows(k, t)] for k in (0, 1)],
                    [[P.status(k, t)[0], hx(P.status(k, t)[1])] for k in (0, 1)]] for t in sorted(row_t)]
    out['led'] = [[hx(am), hx(t), hx(wk), hx(P.led_colour(am, t, wk))]
                  for am, t, wk in ((0.049, 5.0, 0.0), (3.2, 5.1, 0.0), (5.5, 20.37, 0.5), (1.0, 55.0, 1.0))]
    ev_hit = [e for e in P.EV if e[1] == 'hit']
    wave_i = [i for i in range(P.NSIM) if len(P.RUN['WAVES'][i])]
    frames = sorted(set(list(range(0, 1830, 45)) + [43, 44, 107, 108, 109, 150, 600, 1500, 1829]
                        + [int(round(P.T_KO * 30)) + j for j in (-1, 0, 1, 2, 5, 11, 12, 30)]
                        + [int(ev_hit[j][0] * 30) + 1 for j in (0, 5, 60)] + [wave_i[0] + 108, wave_i[len(wave_i) // 2] + 108]
                        + [int(T.T_GLIDE * 30) + j for j in (-4, 0, 3)]))
    hooks, shas, drawn = [], [], []
    box_frames = {}
    for f in frames:
        del LOG[:]
        P.ui.reset_drawn()
        img = P.render(f)
        hooks.append([f, len(LOG), sha(LOG)])
        shas.append([f, hashlib.sha256(np.ascontiguousarray(img).tobytes()).hexdigest()])
        boxes = [f'{x0},{y0},{x1},{y1},{kind},{info}' for x0, y0, x1, y1, kind, info in P.ui.DRAWN]
        drawn.append([f, len(boxes), sha(boxes)])
        if f in (43, 108, 600, 1829) or f == int(round(P.T_KO * 30)) + 30:
            box_frames[str(f)] = boxes
    out['drawn'] = drawn                 # reelkit.ui.DRAWN after each frame: what the safe-zone check reads
    out['drawn_full'] = box_frames
    cocs = []
    from reelkit.lens import coc_map
    for f in (0, 150, 600, int(round(P.T_KO * 30)) + 30, 1700, 1829):   # kit/checks.py focus(), one frame
        t = f / T.FPS
        c = P.camera(t)
        pts = [S.to_world(sx, sz, y) for sx in (-S.AP, S.AP) for sz in (-S.AP, S.AP) for y in (0.0, S.POST_H)]
        names = ['ring'] * len(pts)
        for k in (0, 1):
            st = P.fighter(k, t)
            for dy in (0.0, 2 * P.R):
                pts.append(S.to_world(st['p'][0], st['p'][1], dy))
                names.append(P.NAMES[k])
        for kind, p, rot, size, curl, a in P.parts_at(t):
            pts.append(S.to_world(p[0], p[2], p[1]))
            names.append('shard')
        coc = coc_map(c.depth(np.array(pts)), c.depth(S.SUBJECT)[0], 26.0, 0.17)
        cocs.append([f, names, hx(coc), hx(c.depth(np.array(pts)))])
    out['focus'] = cocs
    out['hooks_build'] = [len(build_log), sha(build_log)]
    out['hooks_build_head'] = build_log[:6]
    out['hooks'] = hooks
    del LOG[:]
    P.render(600)
    out['hooks_600'] = list(LOG)
    out['frames'] = shas
    return out, stage_hashes(P, EXACT)


def compact(doc):
    """one top-level key a line, each value compact (the unit numbers are 100 KB of hex floats)"""
    return '{\n' + ',\n'.join(f' {json.dumps(k)}: ' + json.dumps(doc[k], sort_keys=True, separators=(',', ':'))
                              for k in sorted(doc)) + '\n}\n'


def run():
    P = F.picture('ring')                    # the episode as it is: its own looks
    F.check_recording(P)
    episode = stage_hashes(P, EARLY)
    scratch = os.path.join(common.CACHE, 'ring', 'base_looks.json')
    os.makedirs(os.path.dirname(scratch), exist_ok=True)
    subprocess.run([sys.executable, '-m', 'tools.studio_export.ring_vectors', scratch], cwd=common.REPO, check=True)
    with open(scratch, encoding='utf-8') as fh:
        got = json.load(fh)
    unit, base = got['unit'], got['stages']
    common.write_text(os.path.join(OUT, 'unit.json'), compact(unit))
    common._register(os.path.join(OUT, 'unit.json'), __name__)
    doc = dict(
        about='SHA-256 of the buffers kit.picture.render holds after a stage, for frames 150, 600 and 1500 of '
              'flute_vs_trombone (seed 392): [dtype, sha256 of the array bytes in C order] per stage and role '
              '(rgb: (640, 360, 3); depth: (640, 360))',
        tool='ring', episode=F.EPISODE, frames=list(STAGE_FRAMES),
        base_looks=dict(what='the base-class look (kit/fighters/base.py Look) for both fighters', stages=base),
        episode_looks=dict(what="the episode as it is: FLUTE's and TROMBONE's own looks", stages=episode))
    common.write_json(os.path.join(OUT, 'stage_hashes.json'), doc)
    for name in ('unit.json', 'stage_hashes.json'):
        path = os.path.join(OUT, name)
        print(f'ring: {common.rel(path)}  {os.path.getsize(path)} bytes')


EXPORTS = {'ring': run}

if __name__ == '__main__':                   # the child: the base-class looks, in a process of their own
    unit, stages = base_look_numbers()
    with open(sys.argv[1], 'w', encoding='utf-8', newline='\n') as fh:
        json.dump(dict(unit=unit, stages=stages), fh)
