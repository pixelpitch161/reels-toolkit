"""The renderer kit's groups of the `kit` export (kit_vectors.py runs them): the 3D-to-2D machinery of reelkit, each
function on seeded inputs, for the ports in bf-picture.

studio/reference/picture_kit/ (every .npz has a sidecar .json: vectors-convention.md):

    camera.npz   reelkit.camera: Cam (the constructor's fields, to_cam, proj, wp, depth, pixel_rays, ground_hit),
                 clip_near, interp_keys
    geometry.npz reelkit.geometry: face, box, prism, lathe, catenary, thin_quad
    raster.npz   reelkit.raster: Frame (the constructor, poly, screen_poly, line, disc, set_array, array, depth) at
                 the full 360 x 640, the 60 glow kernels, apply_glows
    lighting.npz reelkit.lighting: Lights (key, fill, point) and shade
    shadows.npz  reelkit.shadows: MaskGrid (its size, raster, sample), proj_point
    lens.npz     reelkit.lens: oct_kernel, coc_map, dof
    fftconv.npz  what dof stands on: scipy.fft.rfftn and irfftn with the padded shapes fftconvolve uses, and
                 scipy.signal.fftconvolve(mode='same') itself
    post.npz     reelkit.post: bloom, grade, vignette, top_scrim
    hull.npz     scipy.spatial.ConvexHull(points).vertices for 2-D points (the order the spot's beam is drawn in)

GROUPS lists the nine in the order the export writes them. This module registers no export of its own."""
import contextlib
import math

import numpy as np

from . import common
from .kit_vectors import Group, f64, i64, rng


# ================================================================ the renderer kit: shared helpers
def hx(v):
    """the JSON form of an argument that keeps every bit: floats as float.hex() strings, also inside lists, tuples
    and dicts; None, bools, ints and strings as they are"""
    if v is None or isinstance(v, (bool, str)):
        return v
    if isinstance(v, (int, np.integer)):
        return int(v)
    if isinstance(v, (float, np.floating)):
        return common.hexf(float(v))
    if isinstance(v, dict):
        return {k: hx(x) for k, x in v.items()}
    return [hx(x) for x in v]


def cam_args(target, dist, yaw, pitch, fov, sy=None, roll=0.0, near=0.1):
    """the eight arguments of reelkit.camera.Cam by name, as Python floats (sy may be None)"""
    return dict(target=[float(v) for v in target], dist=float(dist), yaw=float(yaw), pitch=float(pitch),
                fov=float(fov), sy=None if sy is None else float(sy), roll=float(roll), near=float(near))


def cam_of(c):
    from reelkit.camera import Cam
    return Cam(**c)


RING_CAM = cam_args((0.12, 0.25, 0.0), 27.5, 0.0, 58.0, 36.0, 288.0)      # the ring's first camera key
RING_CAM_B = cam_args((0.12, 0.25, 0.0), 27.6, -0.8, 58.4, 36.0, 288.0)   # its second
CLOSE_CAM = cam_args((0.0, 0.5, 0.0), 4.0, 25.0, 35.0, 50.0, 320.0)       # close in: polygons cross the near plane


def scene():
    """projects/ballfight/scene.py, the ring (read-only; importing it loads no episode)"""
    E, _ = common.kit()
    E.activate_paths()
    import scene as S
    return S


@contextlib.contextmanager
def patched(mod, **values):
    """module globals set for the length of a with block (the kit's files are never edited)"""
    old = {k: getattr(mod, k) for k in values}
    for k, v in values.items():
        setattr(mod, k, v)
    try:
        yield
    finally:
        for k, v in old.items():
            setattr(mod, k, v)


def blocky(r, palette, shape, block):
    """an array of `shape` (h, w) or (h, w, c) made of block x block squares of palette values: few distinct
    values, so a full frame of it packs small"""
    h, w = shape[:2]
    small = r.integers(0, len(palette), (-(-h // block), -(-w // block)) + tuple(shape[2:]))
    a = np.asarray(palette)[small]
    return np.ascontiguousarray(np.repeat(np.repeat(a, block, 0), block, 1)[:h, :w])


def flat(parts, width):
    """a list of (k, width) point lists as one (sum k, width) float64 array and the k of each"""
    parts = [np.asarray(p, np.float64).reshape(-1, width) for p in parts]
    return (np.concatenate(parts) if parts else np.zeros((0, width))), i64(len(p) for p in parts)


# ================================================================ reelkit.camera
def camera_set(r):
    cams = [RING_CAM, RING_CAM_B, cam_args((0.12, 0.25, 0.0), 26.7, 0.3, 58.0, 36.0, 288.0),
            cam_args((0.0, 0.0, 0.0), 10.0, 0.0, 0.0, 50.0),                      # level; sy=None
            cam_args((0.0, 1.0, 0.0), 12.0, 180.0, 20.0, 45.0),
            cam_args((1.0, 0.5, -2.0), 8.0, 90.0, 35.0, 60.0, 200.0),
            cam_args((1.0, 0.5, -2.0), 8.0, -90.0, -15.0, 60.0, 400.5),           # from below
            cam_args((0.0, 0.4, 0.0), 6.0, 37.0, 48.0, 40.0, 300.0, roll=7.5),
            cam_args((-2.0, 0.0, 3.0), 15.0, -130.0, 75.0, 28.0, None, roll=-30.0, near=0.5),
            CLOSE_CAM]
    for _ in range(8):
        cams.append(cam_args(r.uniform(-3, 3, 3), r.uniform(2, 40), r.uniform(-180, 180), r.uniform(-30, 85),
                             r.uniform(20, 75), None if r.random() < 0.3 else r.uniform(100, 500),
                             0.0 if r.random() < 0.6 else r.uniform(-40, 40), 0.1 if r.random() < 0.7 else 0.3))
    return cams


def group_camera():
    from reelkit import camera as CM
    from reelkit import config as C
    g = Group('camera', 'reelkit.camera: Cam, clip_near and interp_keys on seeded inputs', NEAR=common.hexf(CM.NEAR),
              note='args.cam holds the eight arguments of Cam by name (floats as float.hex() strings, sy null for '
                   'None). "Cam(**cam)": pos (3,), M (3, 3: the rows right, up, fwd), target, view and scalars = '
                   '[f, cx, cy, near]. Points are (k, 3) rows, screen points (k, 2). pixel_rays and ground_hit take '
                   'float64 xs and ys of shape (rows, columns), as kit.picture gives them. clip_near: "none" is '
                   '[True] where Python returned None (then out has no rows). interp_keys: the keys are the rows '
                   'of key_t, key_target and key_rest = [dist, yaw, pitch, fov, sy], called with the default '
                   'ease.')
    r = rng(18, 1)
    grid_y, grid_x = np.mgrid[0:C.H:64, 0:C.W:45].astype(np.float64)

    def fields(cam):
        assert cam.M.dtype == np.float64 and cam.M.shape == (3, 3) and cam.pos.shape == (3,)
        return dict(pos=cam.pos, M=cam.M, target=cam.target, scalars=f64([cam.f, cam.cx, cam.cy, cam.near]))

    for n, c in enumerate(camera_set(r)):
        cam, A = cam_of(c), dict(cam=hx(c))
        g.add('Cam(**cam)', A, view=cam.view, **fields(cam))
        k = (1, 3, 4, 8, 12, 5)[n % 6]
        P = cam.target[None] + r.uniform(-6, 6, (k, 3))
        P[-1] = cam.target                                # the look-at point itself: on the optical axis
        if k > 2:
            P[1, 0] = cam.pos[0]                          # level with the camera in x
        g.add('Cam(**cam).to_cam(P)', A, P=P, out=cam.to_cam(P))
        g.add('Cam(**cam).wp(P)', A, P=P, out=cam.wp(P))
        g.add('Cam(**cam).depth(P)', A, P=P, out=cam.depth(P))
        Pc = r.uniform(-3, 3, (k + 2, 3))
        Pc[:, 2] = r.uniform(-0.5, 30, k + 2)
        Pc[0, 2], Pc[1, 2] = 1e-3, 5e-4                   # on and under the floor of the divide
        g.add('Cam(**cam).proj(Pc)', A, Pc=Pc, out=cam.proj(Pc))
        if n % 2 == 0:
            whole = n % 4 == 0                            # pixel centres as kit.picture has them, or in between
            xs = grid_x.copy() if whole else grid_x + r.uniform(0, 44, grid_x.shape)
            ys = grid_y.copy() if whole else grid_y + r.uniform(0, 63, grid_y.shape)
            out = cam.pixel_rays(xs, ys)
            assert out.dtype == np.float64 and out.shape == xs.shape + (3,)
            g.add('Cam(**cam).pixel_rays(xs, ys)', A, xs=xs, ys=ys, out=out)
            for y0 in (0.0, -1.15):
                X, Z, t = cam.ground_hit(xs, ys, y0)
                g.add('Cam(**cam).ground_hit(xs, ys, y0)', dict(cam=hx(c), y0=common.hexf(y0)), xs=xs, ys=ys, X=X,
                      Z=Z, t=t)

    for n in range(20):
        k = int(r.integers(3, 9))
        near = (0.1, 0.1, 0.5, 1e-3)[n % 4]
        Pc = r.uniform(-4, 4, (k, 3))
        Pc[:, 2] = r.uniform(-1.0, 3.0, k)
        if n == 3:
            Pc[:, 2] = np.abs(Pc[:, 2]) + near            # all in front: the polygon as it is
        if n == 7:
            Pc[:, 2] = -np.abs(Pc[:, 2])                  # all behind: None
        if n == 11:
            Pc[0, 2] = near                               # a corner on the plane counts as in front
        if n == 15:
            Pc[:, 2] = near - 1.0
            Pc[2, 2] = near + 1.0                         # one corner in front: a triangle
        out = CM.clip_near(Pc, near)
        g.add('reelkit.camera.clip_near(Pc, near)', dict(near=common.hexf(near)), Pc=Pc,
              out=np.zeros((0, 3)) if out is None else out, none=np.array([out is None]))

    aim = (0.12, 0.25, 0.0)
    ring_keys = [(0.0, aim, 27.5, 0.0, 58.0, 36, 288), (8.0, aim, 27.6, -0.8, 58.4, 36, 288),
                 (20.0, aim, 27.4, 0.7, 57.7, 36, 288), (32.0, aim, 27.5, -0.6, 58.2, 36, 288),
                 (44.0, aim, 26.9, 0.0, 58.0, 36, 288), (50.2, aim, 26.7, 0.3, 58.0, 36, 288),
                 (52.7, aim, 27.3, 0.4, 58.4, 36, 288), (58.0, aim, 27.4, 0.2, 58.2, 36, 288),
                 (61.0, aim, 27.5, 0.0, 58.0, 36, 288)]
    moving_keys = [(1.0, (0.0, 0.0, 0.0), 10.0, 0.0, 30.0, 40.0, 300.0),
                   (2.5, (1.0, 0.5, -1.0), 14.0, 45.0, 50.0, 35.0, 280.0),
                   (6.0, (-2.0, 1.0, 0.5), 8.0, -20.0, 20.0, 55.0, 330.0)]
    for keys, times in ((ring_keys, (-1.0, 0.0, 1 / 30, 3.7, 8.0, 14.25, 43.999, 44.0, 50.2, 51.0, 913 / 30, 60.5,
                                     61.0, 70.0)),
                        (moving_keys, (0.0, 1.0, 1.3, 2.5, 4.0, 6.0, 9.0))):
        for t in times:
            cam = CM.interp_keys(keys, t)
            g.add('reelkit.camera.interp_keys(keys, t)', dict(t=common.hexf(t)), key_t=f64(k[0] for k in keys),
                  key_target=np.array([k[1] for k in keys], np.float64),
                  key_rest=np.array([[float(v) for v in k[2:]] for k in keys]), **fields(cam))
    return g


# ================================================================ reelkit.geometry
def faces_arrays(faces):
    """a list of (Q, n) faces as three arrays: every face's corners one after another, how many each face has,
    and the normals"""
    Q, counts = flat([q for q, _ in faces], 3)
    return dict(Q=Q, counts=counts, n=np.stack([np.asarray(n, np.float64) for _, n in faces]))


def group_geometry():
    from reelkit import geometry as G
    g = Group('geometry', 'reelkit.geometry: face, box, prism, lathe, catenary and thin_quad on seeded inputs',
              note='A list of faces is three arrays: Q (the corners of all faces, one face after another), counts '
                   '(corners per face) and n (one unit normal per face). Scalar arguments are in args by their '
                   'Python names (floats as float.hex() strings, None as null).')
    r = rng(18, 2)
    for n in range(18):
        k = (3, 4, 4, 5, 6, 3)[n % 6]
        c = r.uniform(-3, 3, 3)
        u, v = r.normal(0, 1, 3), r.normal(0, 1, 3)
        a = np.sort(r.uniform(0, 2 * math.pi, k))
        pts = c[None] + np.cos(a)[:, None] * u[None] + np.sin(a)[:, None] * v[None] + r.normal(0, 0.02, (k, 3))
        center = None if n % 3 == 0 else [float(x) for x in c + r.normal(0, 1.0, 3)]
        up = bool(n % 2)
        Q, nn = G.face(pts.tolist() if n % 4 == 0 else pts, center, up)
        assert Q.dtype == np.float64 and nn.dtype == np.float64
        g.add('reelkit.geometry.face(pts, center, up)', dict(center=hx(center), up=up), pts=pts, Q=Q, n=nn)

    for n, rot in enumerate((0.0, 0.0, math.pi / 4, 0.3, -1.2, math.pi / 2, 2.0, 0.01)):
        a = [float(x) for x in r.uniform(-3, 3, 3)] + [float(x) for x in r.uniform(0.05, 2, 3)] + [rot]
        g.add('reelkit.geometry.box(cx, cy, cz, hx, hy, hz, rot)',
              dict(zip(('cx', 'cy', 'cz', 'hx', 'hy', 'hz', 'rot'), hx(a))), **faces_arrays(G.box(*a)))

    post = 3.0 + 0.24                                     # scene.POST: the ring's corner posts
    wx, wz = (-post - post) / math.sqrt(2.0), (-post + post) / math.sqrt(2.0)
    prisms = [dict(cx=wx, cz=wz, r=0.085, y0=0.0, y1=1.42, n=6, r1=None, rot=0.0, tilt=(0.0, 0.0)),
              dict(cx=wx, cz=wz, r=0.16, y0=0.42 - 0.11, y1=0.42 + 0.11, n=6, r1=None, rot=0.0, tilt=(0.0, 0.0)),
              dict(cx=wx, cz=wz, r=0.11, y0=1.42, y1=1.42 + 0.06, n=6, r1=None, rot=0.0, tilt=(0.0, 0.0)),
              dict(cx=0.0, cz=0.0, r=1.0, y0=0.0, y1=2.0, n=8, r1=None, rot=0.0, tilt=(0.0, 0.0)),
              dict(cx=1.0, cz=-2.0, r=0.8, y0=-0.5, y1=1.5, n=8, r1=0.4, rot=0.2, tilt=(0.0, 0.0)),
              dict(cx=-1.5, cz=0.5, r=0.5, y0=0.0, y1=3.0, n=10, r1=None, rot=-0.7, tilt=(0.3, -0.2)),
              dict(cx=0.25, cz=0.75, r=1.2, y0=0.1, y1=0.2, n=3, r1=0.0, rot=1.0, tilt=(0.0, 0.0)),
              dict(cx=0.0, cz=0.0, r=2.0, y0=0.0, y1=1.0, n=4, r1=None, rot=math.pi / 4, tilt=(0.0, 0.0)),
              dict(cx=2.0, cz=2.0, r=0.3, y0=1.0, y1=1.1, n=12, r1=0.6, rot=0.0, tilt=(-0.1, 0.05)),
              dict(cx=0.0, cz=0.0, r=1.0, y0=0.0, y1=1.0, n=5, r1=None, rot=3.0, tilt=(0.0, 0.0))]
    for p in prisms:
        g.add('reelkit.geometry.prism(cx, cz, r, y0, y1, n, r1, rot, tilt)', hx(p), **faces_arrays(G.prism(**p)))

    lathes = [dict(cx=0.0, cz=0.0, prof=[(0.0, 0.3), (0.5, 0.5), (1.0, 0.2)], n=8, rot=0.0, cy=0.0),
              # a point at each end: the first band takes its normal from the far side, and there is no cap
              dict(cx=1.5, cz=-2.0, prof=[(0.0, 0.0), (0.2, 0.4), (0.6, 0.45), (0.9, 0.0)], n=6, rot=0.3, cy=0.25),
              # a band thinner than 1e-4 at both ends is left out; a top radius of 1e-3 or less has no cap
              dict(cx=0.0, cz=1.0, prof=[(0.0, 0.0), (0.1, 0.00005), (0.4, 0.3), (0.5, 0.0005)], n=5, rot=0.0,
                   cy=0.0),
              dict(cx=-1.0, cz=0.0, prof=[(0.0, 0.5), (1.0, 0.5)], n=12, rot=-1.0, cy=-0.5),
              dict(cx=0.5, cz=0.5, prof=[(0.0, 0.2), (0.3, 0.0), (0.5, 0.25), (0.7, 0.002)], n=7, rot=2.5, cy=1.0),
              dict(cx=0.0, cz=0.0, prof=[(0.0, 0.4), (0.3, 0.6), (0.5, 0.6), (0.6, 0.3), (0.9, 0.35)], n=3, rot=0.1,
                   cy=0.0)]
    for p in lathes:
        faces = G.lathe(p['cx'], p['cz'], p['prof'], p['n'], p['rot'], p['cy'])
        g.add('reelkit.geometry.lathe(cx, cz, prof, n, rot, cy)', hx({k: v for k, v in p.items() if k != 'prof'}),
              prof=np.array(p['prof'], np.float64), **faces_arrays(faces))

    for n, count in enumerate((2, 5, 15, 3, 9, 33)):
        a, b = [float(x) for x in r.uniform(-4, 4, 3)], [float(x) for x in r.uniform(-4, 4, 3)]
        sag = (0.0, 0.3, 0.12, -0.2, 1.5, 0.05)[n]
        out = G.catenary(a, b, sag, count)
        assert out.dtype == np.float64 and out.shape == (count, 3)
        g.add('reelkit.geometry.catenary(a, b, sag, n)', dict(a=hx(a), b=hx(b), sag=common.hexf(sag), n=count),
              out=out)

    for n in range(8):
        a, b = [float(x) for x in r.uniform(-4, 4, 3)], [float(x) for x in r.uniform(-4, 4, 3)]
        if n == 5:
            b = [a[0], a[1], a[2] + 2.0]                  # along z: the cross product is zero, the quad a line
        w = float(r.uniform(0.01, 0.5))
        g.add('reelkit.geometry.thin_quad(a, b, w)', dict(a=hx(a), b=hx(b), w=common.hexf(w)),
              out=G.thin_quad(a, b, w))
    return g


# ================================================================ reelkit.raster
GLOW_SMALL = (48, 64)                                     # (W, H) the small apply_glows cases run at


def group_raster():
    from reelkit import config as C
    from reelkit import raster as R
    from reelkit.raster import Frame
    g = Group('raster', 'reelkit.raster: Frame at 360 x 640, the glow kernels and apply_glows', W=C.W, H=C.H,
              SKY_DEPTH=common.hexf(R.SKY_DEPTH),
              note='Every Frame case starts from Frame(bg) (args.bg: the tuple; or the array "bg" for '
                   '"Frame(bg_array)"), runs its calls in order and stores what the frame then holds: rgb = '
                   'np.asarray(fr.pil) (uint8, (640, 360, 3)) and dep = fr.depth() (float32, (640, 360)). The calls '
                   'of one case are rows: P or pts holds every point list one after another, counts the points '
                   'per call, col one colour per call. poly: depth per call; ret and ret_counts are the returned '
                   'screen points (0 points where Python returned None). screen_poly and disc: z is used where '
                   'has_z. apply_glows: glows rows are [x, y, r, red, green, blue, intensity]; the cases with '
                   'args.W and args.H ran with reelkit.raster.W and H set to that size (the picture decides the '
                   'size in the port).')
    r = rng(18, 3)
    S = scene()

    def pictures(fr):
        rgb, dep = np.array(fr.pil), fr.depth()
        assert rgb.dtype == np.uint8 and rgb.shape == (C.H, C.W, 3)
        assert dep.dtype == np.float32 and dep.shape == (C.H, C.W)
        return dict(rgb=rgb, dep=dep)

    def colour():
        return r.uniform(-20, 290, 3) if r.random() < 0.3 else np.floor(r.uniform(0, 256, 3))

    for bg in ((10, 8, 18), (0, 0, 0), (255, 255, 255), (10.9, 8.2, 18.5), (300, -5, 127.99), (-0.9, 255.9, 256.0)):
        g.add('Frame(bg)', dict(bg=hx(bg)), **pictures(Frame(bg)))
    palette = f64([-3.5, 0.0, 0.4, 0.999, 1.0, 17.5, 127.999, 128.0, 254.5, 255.0, 255.4, 256.0, 300.25, 1e6, 12.25,
                   200.0])
    bg = blocky(r, palette, (C.H, C.W, 3), 20)
    g.add('Frame(bg_array)', {}, bg=bg, **pictures(Frame(bg)))

    # ---- poly
    def poly_case(c, bg, polys):
        cam, fr = cam_of(c), Frame(bg)
        rets = [fr.poly(cam, P, col, depth=d) for P, col, d in polys]
        P, counts = flat([p[0] for p in polys], 3)
        ret, ret_counts = flat([[] if x is None else x for x in rets], 2)
        g.add('fr.poly(Cam(**cam), P, col, depth)', dict(cam=hx(c), bg=hx(bg)), P=P, counts=counts,
              col=np.stack([np.asarray(p[1], np.float64) for p in polys]), depth=np.array([p[2] for p in polys]),
              ret=ret, ret_counts=ret_counts, **pictures(fr))
        return rets

    def random_polys(count, centre, spread, size):
        out = []
        for _ in range(count):
            k = int(r.integers(3, 9))
            c = np.asarray(centre) + r.uniform(-1, 1, 3) * spread
            out.append((c[None] + r.uniform(-size, size, (k, 3)), colour(), bool(r.random() < 0.85)))
        return out

    for c in (RING_CAM, RING_CAM_B, RING_CAM, cam_args((0.0, 0.4, 0.0), 9.0, 37.0, 48.0, 40.0, 300.0, roll=7.5),
              cam_args((1.0, 0.5, -2.0), 8.0, -90.0, -15.0, 60.0, 400.5), RING_CAM_B):
        poly_case(c, (10, 8, 18), random_polys(30, c['target'], (7.0, 1.5, 7.0), 1.5))
    close = cam_of(CLOSE_CAM)
    clipped = 0
    for _ in range(3):
        rets = poly_case(CLOSE_CAM, (40, 0, 60), random_polys(30, close.pos, 3.0, 2.5))
        clipped += sum(x is None for x in rets)
    assert clipped >= 5, 'the close camera should cull some polygons'
    # the ring's own geometry as kit.picture draws it: the tiers far to near, the skirt, the corner posts
    cam = cam_of(RING_CAM)
    tiers = [(cam.depth(Q.mean(0))[0], Q, col) for Q, n, col, _ in S.STANDS if np.dot(n, cam.pos - Q.mean(0)) > 0]
    tiers.sort(key=lambda it: -it[0])
    polys = [(Q, col * 0.55, True) for _, Q, col in tiers]
    polys += [(Q, col * 0.8, True) for Q, n, col in S.SKIRT_FACES if np.dot(n, cam.pos - Q.mean(0)) > 0]
    for name in ('back', 'red', 'blue', 'front'):
        polys += [(Q, col * 0.9, True) for Q, n, col in S.POSTS[name] if np.dot(n, cam.pos - Q.mean(0)) > 0]
    poly_case(RING_CAM, (10, 8, 18), polys)

    # ---- line
    for c, centre, spread in ((RING_CAM, RING_CAM['target'], 5.0), (RING_CAM_B, RING_CAM['target'], 5.0),
                              (CLOSE_CAM, close.pos, 3.0), (CLOSE_CAM, CLOSE_CAM['target'], 1.5)):
        cam, fr, lines = cam_of(c), Frame((10, 8, 18)), []
        for _ in range(12):
            k = int(r.integers(2, 16))
            P = np.asarray(centre) + r.uniform(-1, 1, 3) * spread + np.cumsum(r.uniform(-0.6, 0.6, (k, 3)), 0)
            lines.append((P, colour(), int(r.integers(1, 4))))
            fr.line(cam, P, lines[-1][1], lines[-1][2])
        P, counts = flat([q[0] for q in lines], 3)
        g.add('fr.line(Cam(**cam), pts, col, width)', dict(cam=hx(c), bg=hx((10, 8, 18))), P=P, counts=counts,
              col=np.stack([q[1] for q in lines]), width=i64(q[2] for q in lines), **pictures(fr))
    # one point behind the camera hides the whole line, however much of it is in view
    right, up, fwd = close.M
    ahead = [close.pos + fwd * 3.0 + right * a + up * b for a, b in ((-0.8, -0.5), (0.0, 0.6), (0.9, -0.2))]
    lines = [(np.array(ahead), np.array([250.0, 240.0, 60.0]), 3),
             (np.array(ahead + [close.pos - fwd * 1.0]), np.array([255.0, 0.0, 0.0]), 2),
             (np.array([close.pos - fwd * 0.5 + right * 0.1] + ahead[::-1]), np.array([0.0, 255.0, 0.0]), 1),
             (np.array([ahead[0] + up * 0.4, ahead[2] + up * 0.4]), np.array([40.0, 90.0, 250.0]), 2)]
    fr = Frame((10, 8, 18))
    for P, col, width in lines:
        fr.line(close, P, col, width)
    assert len({tuple(px) for px in np.array(fr.pil).reshape(-1, 3)}) == 3, 'two of the four lines are drawn'
    P, counts = flat([q[0] for q in lines], 3)
    g.add('fr.line(Cam(**cam), pts, col, width)', dict(cam=hx(CLOSE_CAM), bg=hx((10, 8, 18))), P=P, counts=counts,
          col=np.stack([q[1] for q in lines]), width=i64(q[2] for q in lines), **pictures(fr))

    # ---- screen_poly and disc
    for _ in range(3):
        fr, rows = Frame((10, 8, 18)), []
        for _ in range(20):
            k = int(r.integers(3, 8))
            c = r.uniform((-30, -30), (C.W + 30, C.H + 30))
            pts = c[None] + r.uniform(-60, 60, (k, 2))
            z = None if r.random() < 0.4 else float(r.uniform(1, 50))
            rows.append((pts, colour(), z))
            fr.screen_poly(pts, rows[-1][1], z)
        pts, counts = flat([q[0] for q in rows], 2)
        g.add('fr.screen_poly(pts, col, z)', dict(bg=hx((10, 8, 18))), pts=pts, counts=counts,
              col=np.stack([q[1] for q in rows]), has_z=np.array([q[2] is not None for q in rows]),
              z=f64(0.0 if q[2] is None else q[2] for q in rows), **pictures(fr))
    for _ in range(3):
        fr, rows = Frame((10, 8, 18)), []
        for _ in range(20):
            centre = [float(v) for v in r.uniform((-20, -20), (C.W + 20, C.H + 20))]
            rad, count = float(r.uniform(0.4, 60)), int(r.choice([12, 12, 6, 8, 20, 3]))
            rot = 0.0 if r.random() < 0.5 else float(r.uniform(-3, 3))
            z = None if r.random() < 0.4 else float(r.uniform(1, 50))
            rows.append((centre, rad, colour(), count, rot, z))
            fr.disc(centre, rad, rows[-1][2], count, rot, z)
        g.add('fr.disc(center, r, col, n, rot, z)', dict(bg=hx((10, 8, 18))), center=np.array([q[0] for q in rows]),
              r=f64(q[1] for q in rows), col=np.stack([q[2] for q in rows]), n=i64(q[3] for q in rows),
              rot=f64(q[4] for q in rows), has_z=np.array([q[5] is not None for q in rows]),
              z=f64(0.0 if q[5] is None else q[5] for q in rows), **pictures(fr))

    # ---- set_array, array, depth
    depths = np.array([0.5, 1.0, 12.25, 27.5, 1e6, 3.3], np.float32)
    for n in range(3):
        arr = blocky(r, palette, (C.H, C.W, 3), (20, 24, 40)[n])
        arr = arr.astype(np.float32) if n % 2 == 0 else arr
        depth = None if n == 1 else blocky(r, depths, (C.H, C.W), (40, 0, 20)[n])
        fr = Frame((10, 8, 18))
        fr.set_array(arr, depth)
        out = fr.array()
        assert out.dtype == np.float32 and out.shape == (C.H, C.W, 3)
        extra = {} if depth is None else dict(depth=depth)
        g.add('fr.set_array(arr, depth); fr.array()', dict(bg=hx((10, 8, 18))), arr=arr, array=out, **extra,
              **pictures(fr))

    # ---- the glow kernels and apply_glows
    assert sorted(R.GLOW) == list(range(1, 61))
    for rad in range(1, 61):
        g.add('reelkit.raster.GLOW[r]', dict(r=rad), out=R.GLOW[rad])

    def glow_rows(count, w, h):
        rows = []
        for _ in range(count):
            x, y = r.uniform(-25, w + 25), r.uniform(-25, h + 25)
            if r.random() < 0.3:
                x, y = math.floor(x) + 0.5, math.floor(y) + 0.5    # halves: round() goes to the even pixel
            rad = r.choice([0.2, 1.0, 2.5, 3.5, 16.0, 46.0, 60.0, 75.0]) if r.random() < 0.5 else r.uniform(0.3, 66)
            col = np.floor(r.uniform(0, 256, 3)) if r.random() < 0.5 else r.uniform(0, 255, 3)
            rows.append([x, y, rad, *col, r.uniform(0.05, 1.2)])
        return np.array(rows, np.float64)

    def glow_case(img, rows, **args):
        glows = [(q[0], q[1], q[2], (q[3], q[4], q[5]), q[6]) for q in rows.tolist()]
        out = R.apply_glows(img.copy(), glows)
        assert out.dtype == img.dtype and out.shape == img.shape
        g.add('reelkit.raster.apply_glows(img, glows)', args, in_=img, glows=rows, out=out)

    shades = f64([0.0, 10.0, 18.5, 100.25, 200.0, 255.0, 31.0, 77.75])
    w, h = GLOW_SMALL
    with patched(R, W=w, H=h):
        for n in range(6):
            img = blocky(r, shades, (h, w, 3), 4)
            glow_case(img.astype(np.float32) if n % 2 else img, glow_rows(8, w, h), W=w, H=h)
    glow_case(np.zeros((C.H, C.W, 3), np.float32), glow_rows(8, C.W, C.H))
    glow_case(np.zeros((C.H, C.W, 3)) + f64([10.5, 20.25, 30.0]), glow_rows(8, C.W, C.H))
    return g


# ================================================================ reelkit.lighting
def lights_arrays(spec):
    def rows(items, k):
        return np.array([q[k] for q in items], np.float64).reshape(-1, 3)
    keys, fills, points = spec.get('keys', []), spec.get('fills', []), spec.get('points', [])
    return dict(amb=f64(spec['amb']), sky=f64(spec['sky']), view=f64(spec['view']), key_d=rows(keys, 0),
                key_rgb=rows(keys, 1), fill_d=rows(fills, 0), fill_rgb=rows(fills, 1), point_pos=rows(points, 0),
                point_rgb=rows(points, 1), point_radius=f64(q[2] for q in points))


def lights_of(spec):
    from reelkit.lighting import Lights
    L = Lights(amb=tuple(spec['amb']), sky=tuple(spec['sky']))
    L.view = np.array(spec['view'], np.float64)
    for n, (d, rgb) in enumerate(spec.get('keys', [])):
        L.key(f'key{n}', list(d), tuple(rgb))
    for d, rgb in spec.get('fills', []):
        L.fill(list(d), tuple(rgb))
    for pos, rgb, radius in spec.get('points', []):
        L.point(list(pos), tuple(rgb), radius)
    return L


def ring_lights():
    """the ring's lights as kit.picture.lights sets them at rest (the fighters' colours stand in for an episode's)"""
    S = scene()
    cam = cam_of(RING_CAM)
    d = S.SPOT - S.SPOT_AIM
    return dict(amb=(0.36, 0.33, 0.44), sky=(0.10, 0.08, 0.14), view=[float(v) for v in cam.view],
                keys=[([float(v) for v in d / np.linalg.norm(d)], (1.05, 1.0, 0.96))],
                points=[([float(v) for v in S.RED_LIGHT], (1.1, 0.385, 0.33), 2.6),
                        ([float(v) for v in S.BLUE_LIGHT], (0.33, 0.55, 1.1), 2.6)])


def unit_rows(r, count):
    v = r.normal(0, 1, (count, 3))
    return v / np.sqrt((v * v).sum(1))[:, None]


def group_lighting():
    from reelkit import lighting as LT
    g = Group('lighting', 'reelkit.lighting: Lights and shade on seeded inputs',
              note='The lights of a case are arrays: amb, sky, view (3,), and one row per light in key_d, key_rgb, '
                   'fill_d, fill_rgb, point_pos, point_rgb and point_radius; key_d and fill_d are the directions '
                   'as passed to Lights.key and Lights.fill (not yet unit length). "Lights": keys_out and '
                   'fills_out are the unit directions the object then holds. shade: nrm and ctr are (N, 3), base '
                   '(3,) or (N, 3), kvis (N, keys) where the case has one (else None); rimk, trans, emit and step '
                   'are in args.')
    r = rng(18, 4)
    S = scene()

    def random_lights(nk, nf, npt, bright):
        return dict(amb=r.uniform(0.05, 0.5, 3) * bright, sky=r.uniform(0.0, 0.25, 3), view=unit_rows(r, 1)[0],
                    keys=[(r.normal(0, 3, 3), r.uniform(0.2, 1.3, 3) * bright) for _ in range(nk)],
                    fills=[(r.normal(0, 1, 3), r.uniform(0.05, 0.5, 3)) for _ in range(nf)],
                    points=[(r.uniform(-4, 4, 3), r.uniform(0.2, 1.5, 3) * bright, float(r.uniform(0.5, 4)))
                            for _ in range(npt)])

    specs = [ring_lights()] + [random_lights(nk, nf, npt, b) for nk, nf, npt, b in
                               ((1, 0, 0, 1.0), (2, 1, 1, 1.0), (0, 2, 3, 1.0), (1, 2, 2, 2.5), (0, 0, 0, 1.0))]
    for spec in specs:
        L = lights_of(spec)
        g.add('Lights(amb, sky) with key(name, d, rgb), fill(d, rgb), point(pos, rgb, radius)', {},
              keys_out=np.array([k[1] for k in L.keys], np.float64).reshape(-1, 3),
              fills_out=np.array([f[0] for f in L.fills], np.float64).reshape(-1, 3), **lights_arrays(spec))

    def shade_case(spec, nrm, ctr, base, kvis=None, rimk=0.55, trans=0.0, emit=0.0, step=5):
        out = LT.shade(lights_of(spec), nrm, ctr, base, kvis, rimk, trans, emit, step)
        assert out.dtype == np.float64 and out.shape == (len(np.atleast_2d(nrm)), 3)
        extra = {} if kvis is None else dict(kvis=kvis)
        g.add('reelkit.lighting.shade(L, nrm, ctr, base, kvis, rimk, trans, emit, step)',
              dict(rimk=common.hexf(rimk), trans=common.hexf(trans), emit=common.hexf(emit), step=hx(step)),
              nrm=np.atleast_2d(np.asarray(nrm, np.float64)), ctr=np.atleast_2d(np.asarray(ctr, np.float64)),
              base=np.asarray(base, np.float64), out=out, **extra, **lights_arrays(spec))

    # the ring as kit.picture shades it: every tier that faces the camera in one call, then single faces
    ring = ring_lights()
    cam = cam_of(RING_CAM)
    tiers = [(Q, n, col) for Q, n, col, _ in S.STANDS if np.dot(n, cam.pos - Q.mean(0)) > 0]
    shade_case(ring, np.array([t[1] for t in tiers]), np.array([t[0].mean(0) for t in tiers]),
               np.array([t[2] for t in tiers], np.float64), rimk=0.2)
    for Q, n, col in [f for f in S.SKIRT_FACES if np.dot(f[1], cam.pos - f[0].mean(0)) > 0][:4]:
        shade_case(ring, n[None], Q.mean(0)[None], col, rimk=0.3)
    for name in ('back', 'red'):
        for Q, n, col in S.POSTS[name][::5]:
            shade_case(ring, n[None], Q.mean(0)[None], col, rimk=0.6)
    rope = np.array([[0, 1.0, 0.3]]) / np.linalg.norm([0, 1, 0.3])
    shade_case(ring, rope, np.array([[1.2, 0.8, -2.0]]), np.array([236, 230, 222]), rimk=0.5)
    # seeded: every kind of light, with and without the shadow terms, dim and bright (the highlight roll-off)
    for n, (nk, nf, npt, bright) in enumerate(((1, 0, 0, 1.0), (1, 1, 2, 1.0), (2, 2, 1, 1.0), (0, 0, 0, 1.0),
                                               (0, 1, 3, 1.0), (2, 0, 2, 3.0), (1, 2, 0, 0.3), (2, 1, 3, 1.5),
                                               (1, 0, 1, 1.0), (2, 2, 2, 1.0), (1, 1, 1, 2.0), (1, 0, 2, 1.0))):
        spec, count = random_lights(nk, nf, npt, bright), (50, 50, 50, 1, 17, 50)[n % 6]
        nrm, ctr = unit_rows(r, count), r.uniform(-5, 5, (count, 3))
        base = np.floor(r.uniform(0, 256, 3)) if n % 3 == 0 else np.floor(r.uniform(0, 256, (count, 3)))
        kvis = None
        if n % 4 == 1:
            kvis = (r.random((count, nk)) < 0.6).astype(np.float32)       # as shadows.Occluders.vis gives it
        if n % 4 == 2:
            kvis = r.uniform(0, 1, (count, nk))
        shade_case(spec, nrm, ctr, base, kvis, rimk=float(r.uniform(0, 0.8)),
                   trans=0.4 if kvis is not None and n % 2 == 0 else 0.0, emit=(0.0, 0.0, 0.25)[n % 3],
                   step=(5, 5, 8.0, 1, 2.5)[n % 5])
    return g


# ================================================================ reelkit.shadows
def group_shadows():
    from reelkit import shadows as SH
    g = Group('shadows', 'reelkit.shadows: MaskGrid and proj_point on seeded inputs',
              note='args.ext is the grid (x0, x1, z0, z1, units per pixel). MaskGrid(ext): out = [W, H]. raster: '
                   'pts holds the polygons one after another ((x, z) rows), counts the corners of each; out is '
                   'the float32 mask (H, W). sample: X and Z are float64, out float32.')
    r = rng(18, 5)
    S = scene()
    canvas = (-5.4, 5.4, -5.4, 5.4, 0.03)                 # kit.picture.CANVAS_GRID
    grids = [canvas, (-2.0, 2.0, -1.5, 1.5, 0.05), (0.0, 1.0, 0.0, 1.0, 0.1), (-3.0, 3.0, -3.0, 3.0, 0.07),
             (0, 10, 0, 5, 0.3), (-1.0, 1.0, -2.0, 2.0, 0.013)]
    for ext in grids:
        grid = SH.MaskGrid(ext)
        g.add('reelkit.shadows.MaskGrid(ext)', dict(ext=hx([float(v) for v in ext])), out=i64([grid.W, grid.H]))

    def raster_case(ext, polys, blur):
        ext = tuple(float(v) for v in ext)
        out = SH.MaskGrid(ext).raster(polys, blur)
        assert out.dtype == np.float32
        pts, counts = flat(polys, 2)
        g.add('reelkit.shadows.MaskGrid(ext).raster(polys_xz, blur)', dict(ext=hx(ext), blur=common.hexf(blur)),
              pts=pts, counts=counts, out=out)
        return out

    masks = []
    for n in range(10):
        ext = grids[1 + n % 5]
        x0, x1, z0, z1, _ = ext
        polys = []
        for _ in range(int(r.integers(1, 9))):
            k = int(r.integers(3, 8))
            c = r.uniform((x0 - 0.3, z0 - 0.3), (x1 + 0.3, z1 + 0.3))
            polys.append(c[None] + r.uniform(-0.25, 0.25, (k, 2)) * (x1 - x0))
        masks.append((ext, raster_case(ext, polys, (0.0, 0.7, 2.0, 3.0, 0.7)[n % 5])))
    # the canvas shadows as kit.picture.canvas_shadows casts them from the spot: posts, rope bands, two balls
    spot = S.SPOT
    for balls in (((-1.2, 0.6, 0.0), (1.5, -0.4, 0.0)), ((0.2, 2.6, 0.9), (-2.7, -2.7, 0.0))):
        polys = []
        for name in S.CORNERS:
            w = S.to_world(*S.CORNERS[name])
            polys.append(SH.proj_point(np.array([w + [-0.09, 0, 0], w + [0.09, 0, 0], w + [0.09, S.POST_H, 0],
                                                 w + [-0.09, S.POST_H, 0]]), spot))
        for a_name, b_name in S.ROPE_SIDES:
            for y in S.ROPE_Y:
                ends = [S.to_world(*S.CORNERS[a_name], y), S.to_world(*S.CORNERS[b_name], y)]
                P = np.linspace(ends[0], ends[1], 5)
                for a, b in zip(P[:-1], P[1:]):
                    polys.append(SH.proj_point(np.array([a + [0, -0.02, 0], b + [0, -0.02, 0], b + [0, 0.02, 0],
                                                         a + [0, 0.02, 0]]), spot))
        for sx, sz, lift in balls:
            pw = S.to_world(sx, sz, S.R_BALL)
            polys.append(SH.proj_point(np.array([[pw[0] + S.R_BALL * math.cos(u), S.R_BALL + lift,
                                                  pw[2] + S.R_BALL * math.sin(u)]
                                                 for u in np.linspace(0, 2 * math.pi, 13)[:-1]]), spot))
        masks.append((canvas, raster_case(canvas, polys, 0.7)))

    for ext, mask in masks[1::2]:
        x0, x1, z0, z1, res = ext
        count = 400 if ext == canvas else 60
        X, Z = r.uniform(x0 - 0.5, x1 + 0.5, count), r.uniform(z0 - 0.5, z1 + 0.5, count)
        X[:4], Z[:4] = (x0, x1, x0 + res, x1 - res), (z0, z1, z1 - res, z0 + res)      # the grid's own corners
        out = SH.MaskGrid(ext).sample(mask, X, Z)
        assert out.dtype == np.float32 and out.shape == X.shape
        g.add('reelkit.shadows.MaskGrid(ext).sample(mask, X, Z)', dict(ext=hx([float(v) for v in ext])), mask=mask,
              X=X, Z=Z, out=out)

    for n in range(10):
        k = int(r.integers(1, 13))
        lp = [float(v) for v in (r.uniform(-3, 3), r.uniform(2, 10), r.uniform(-3, 3))] if n else \
            [float(v) for v in spot]
        Q = r.uniform(-4, 4, (k, 3))
        Q[:, 1] = r.uniform(-0.5, lp[1] * 1.2, k)         # some corners near and above the light: clamped
        if n % 3 == 0:
            Q[0, 1] = lp[1] * 0.93
        scale = (0.93, 0.93, 0.5, 0.99)[n % 4]
        out = SH.proj_point(Q, np.array(lp), scale)
        assert out.dtype == np.float64 and out.shape == (k, 2)
        g.add('reelkit.shadows.proj_point(Q, lp, max_scale)', dict(lp=hx(lp), max_scale=common.hexf(scale)), Q=Q,
              out=out)
    return g


# ================================================================ reelkit.lens and the FFT convolution under it
DOF_FULL_BLOCK = 8                                        # the full-frame dof case: its inputs are 8 x 8 blocks


def coc_field(r, h, w, top, zero_frac):
    """a circle-of-confusion map: smooth 8 px patches from 0 to `top` (clipped to the lens's 10.5), a little noise,
    a share of pixels exactly in focus"""
    low = r.uniform(-0.3 * top, top, (h // 8 + 1, w // 8 + 1))
    f = np.kron(low, np.ones((8, 8)))[:h, :w] + r.uniform(-0.02, 0.02, (h, w)) * top
    f[r.random((h, w)) < zero_frac] = 0.0
    return np.clip(f, 0, 10.5)


def group_lens():
    from reelkit import config as C
    from reelkit import lens as LN
    g = Group('lens', 'reelkit.lens: oct_kernel, coc_map and dof on seeded inputs',
              LEVELS=[common.hexf(v) for v in LN.LEVELS],
              note='coc_map: depth is float32 and zf a numpy float64 (as Frame.depth() and Cam.depth give them), '
                   'so out is float64. dof: "in" is the float32 picture (h, w, 3), coc float64 (h, w); out is '
                   'float64, or float32 (the picture as it was) where no pixel is out of focus. The case with '
                   'args.block ran at the full 640 x 360: its picture and coc are the stored small arrays with '
                   'every value repeated block x block times (np.repeat on both axes), and out_sub is '
                   'out[::block, ::block]. All cases use the default levels.')
    r = rng(18, 6)
    # (at 5.05, 7.2 and 10.45 a pixel sits between the octagon widened by 0.34 and by 0.35)
    for rad in LN.LEVELS[1:] + [0.5, 1.5, 2.5, 4.0, 12.0, 5.05, 7.2, 10.45]:
        k = LN.oct_kernel(rad)
        assert k.dtype == np.float64 and k.shape[0] == k.shape[1] == 2 * math.ceil(rad) + 1
        g.add('reelkit.lens.oct_kernel(r)', dict(r=common.hexf(rad)), out=k)

    for n in range(8):
        depth = r.uniform(0.5, 60, (20, 30)).astype(np.float32)
        depth[r.random(depth.shape) < 0.15] = 1e6         # sky
        depth[0, :4] = (1e-3, 5e-4, 1e-4, 0.0)            # under the floor of the divide
        zf = np.float64((27.5, 26.9, 3.0, 55.0)[n % 4] + r.uniform(-0.5, 0.5))
        depth[1, :2] = zf                                 # exactly in focus (to float32)
        K, sharp = (26.0, 26.0, 10.0, 40.0)[n % 4], (0.17, 0.05, 0.0, 0.3)[n % 4]
        out = LN.coc_map(depth, zf, K, sharp)
        assert out.dtype == np.float64
        g.add('reelkit.lens.coc_map(depth, np.float64(zf), K, sharp)',
              dict(zf=common.hexf(float(zf)), K=common.hexf(K), sharp=common.hexf(sharp)), depth=depth, out=out)

    def picture(h, w, kind):
        if kind == 'blocks':
            return blocky(r, np.arange(256.0), (h, w, 3), 6).astype(np.float32)
        if kind == 'floats':
            return r.uniform(0, 255, (h, w, 3)).astype(np.float32)
        return r.integers(0, 256, (h, w, 3)).astype(np.float32)          # as Frame.array() gives it

    for h, w, top, zero_frac, kind in ((96, 64, 12.0, 0.2, 'bytes'), (64, 48, 3.0, 0.5, 'bytes'),
                                       (64, 48, 0.9, 0.3, 'bytes'), (40, 32, 0.0, 1.0, 'bytes'),
                                       (30, 40, 12.0, 0.1, 'floats'), (36, 50, 7.0, 0.3, 'bytes'),
                                       (47, 33, 12.0, 0.0, 'bytes'), (64, 48, 12.0, 0.4, 'blocks'),
                                       (20, 20, 12.0, 0.2, 'bytes'), (50, 72, 5.0, 0.6, 'bytes'),
                                       (64, 48, 12.0, 0.97, 'bytes')):
        img, coc = picture(h, w, kind), coc_field(r, h, w, top, zero_frac)
        out = LN.dof(img, coc)
        assert out.dtype == (np.float64 if (coc > 0).any() else np.float32) and out.shape == img.shape
        g.add('reelkit.lens.dof(img, coc)', {}, in_=img, coc=coc, out=out)
    b = DOF_FULL_BLOCK
    img = r.integers(0, 256, (C.H // b, C.W // b, 3)).astype(np.float32)
    coc = coc_field(r, C.H // b, C.W // b, 12.0, 0.3)
    out = LN.dof(np.repeat(np.repeat(img, b, 0), b, 1), np.repeat(np.repeat(coc, b, 0), b, 1))
    assert out.shape == (C.H, C.W, 3) and out.dtype == np.float64 and (coc > 7.6).any()
    g.add('reelkit.lens.dof(img, coc)', dict(block=b), in_=img, coc=coc, out_sub=np.ascontiguousarray(out[::b, ::b]))
    return g


def pairs(a):
    """a complex array as a real one with a last axis [real, imaginary] (the vector files hold no complex dtype)"""
    return np.ascontiguousarray(np.stack([a.real, a.imag], -1))


def group_fftconv():
    import scipy.fft as sf
    from scipy.signal import fftconvolve
    from reelkit import lens as LN
    g = Group('fftconv', 'scipy.fft.rfftn / irfftn and scipy.signal.fftconvolve as reelkit.lens.dof uses them',
              note='Complex arrays are stored with a last axis [real, imaginary]. rfftn: x is float32 or float64 '
                   '(the dtype of the case), s = args.s the padded shape, out complex64 or complex128 as pairs. '
                   'irfftn: X complex128 as pairs, s = args.s, out float64. Every s is a product of 2, 3 and 5 '
                   'only, as fftconvolve pads to (scipy.fft.next_fast_len(n, True)). fftconvolve: a float32 mask '
                   'or picture plane and a float64 lens kernel (reelkit.lens.kernel(args.r)), out float64.')
    r = rng(18, 7)
    # (shape of x, padded shape s, 'f': the float32 transform alone, 'fb': also a float64 one and the way back)
    plans = [((6, 7), (8, 8), 'fb'), ((10, 9), (12, 10), 'fb'), ((16, 9), (16, 9), 'fb'), ((20, 22), (25, 25), 'fb'),
             ((30, 13), (30, 16), 'fb'), ((33, 40), (36, 45), 'f'), ((45, 50), (48, 50), 'f'),
             ((60, 55), (64, 60), 'f'), ((70, 20), (72, 20), 'fb'), ((81, 8), (81, 8), 'fb'),
             ((98, 30), (100, 32), 'f'), ((40, 120), (40, 125), 'f'), ((2, 3), (2, 3), 'fb'), ((5, 4), (5, 4), 'fb'),
             # long columns: past 300 samples a multiple of 4 is transformed another way when columns are left over
             ((300, 10), (300, 10), 'f'), ((320, 10), (320, 10), 'f'), ((360, 4), (360, 4), 'fb'),
             # the lengths of the full frame: 648 and 675 down the columns, 375 and 384 along the rows
             ((640, 10), (648, 10), 'f'), ((640, 2), (648, 2), 'fb'), ((648, 4), (648, 4), 'fb'),
             ((670, 6), (675, 6), 'f'), ((670, 2), (675, 2), 'fb'), ((2, 362), (3, 375), 'fb'),
             ((4, 380), (4, 384), 'fb')]
    for shape, s, what in plans:
        assert all(sf.next_fast_len(v, True) == v for v in s), s
        x = r.uniform(0, 255, shape).astype(np.float32)
        X = sf.rfftn(x, s)
        assert X.dtype == np.complex64 and X.shape == (s[0], s[1] // 2 + 1)
        g.add('scipy.fft.rfftn(x, s)', dict(s=list(s)), x=x, out=pairs(X))
        if 'b' in what:
            k = r.uniform(0, 1, (min(5, shape[0]), min(5, shape[1])))
            K = sf.rfftn(k, s)
            assert K.dtype == np.complex128
            g.add('scipy.fft.rfftn(x, s)', dict(s=list(s)), x=k, out=pairs(K))
            prod = X * K
            assert prod.dtype == np.complex128
            back = sf.irfftn(prod, s)
            assert back.dtype == np.float64 and back.shape == tuple(s)
            g.add('scipy.fft.irfftn(X, s)', dict(s=list(s)), X=pairs(prod), out=back)
    for n, (h, w) in enumerate(((64, 48), (30, 40), (47, 33), (60, 40), (20, 20), (36, 50), (25, 48), (50, 71))):
        rad = LN.LEVELS[1 + n % 6]
        a = (r.random((h, w)) < 0.6).astype(np.float32) if n % 2 == 0 else \
            r.integers(0, 256, (h, w)).astype(np.float32)
        out = fftconvolve(a, LN.kernel(rad), mode='same')
        assert out.dtype == np.float64 and out.shape == (h, w)
        g.add("scipy.signal.fftconvolve(a, reelkit.lens.kernel(r), mode='same')", dict(r=common.hexf(rad)), a=a,
              out=out)
    return g


# ================================================================ reelkit.post
BLOOM_SMALL = ((32, 48), (48, 64))                        # (W, H) the small bloom cases run at


def group_post():
    from reelkit import config as C
    from reelkit import post as PO
    g = Group('post', 'reelkit.post: bloom, grade, vignette and top_scrim',
              note='bloom: img is float64 (h, w, 3), exclude a bool (h, w) where the case has one (else None); '
                   'the cases with args.W and args.H ran with reelkit.post.W and H set to that size (the picture '
                   'decides the size in the port). grade: img float64. vignette and top_scrim: out is the '
                   'float64 (640, 360, 1) table.')
    r = rng(18, 8)

    def bloom_case(img, thr, k, exclude, **size):
        out = PO.bloom(img, thr, k, exclude)
        assert out.dtype == np.float64 and out.shape == img.shape
        extra = {} if exclude is None else dict(exclude=exclude)
        g.add('reelkit.post.bloom(img, thr, k, exclude)', dict(thr=common.hexf(thr), k=common.hexf(k), **size),
              img=img, out=out, **extra)

    def lit(h, w, lamps, quarters):
        """a dim picture with bright spots, clipped to 0..255 as kit.picture hands it over; `quarters` keeps it to
        multiples of 0.25 (it packs smaller)"""
        img = r.uniform(0, 150, (h, w, 3))
        yy, xx = np.mgrid[0:h, 0:w]
        for _ in range(lamps):
            cy, cx, rad = r.uniform(0, h), r.uniform(0, w), r.uniform(1.5, max(h, w) / 5)
            img += (np.hypot(xx - cx, yy - cy) < rad)[..., None] * r.uniform(60, 260, 3)
        img = np.clip(img, 0, 255)
        return np.round(img * 4) / 4 if quarters else img

    for n, (thr, k) in enumerate(((200.0, 0.4), (225.0, 0.22), (225.0, 0.22), (150.0, 1.0), (254.0, 0.5),
                                  (225.0, 0.22))):
        w, h = BLOOM_SMALL[n == 5]
        with patched(PO, W=w, H=h):
            exclude = (r.random((h, w)) < 0.3) if n in (2, 4) else None
            bloom_case(lit(h, w, 6, n not in (0, 1)), thr, k, exclude, W=w, H=h)
    # the full frame: 8 px blocks of whole levels with a few lamps, so the file stays small
    img = blocky(r, np.arange(0.0, 200.0, 8.0), (C.H, C.W, 3), 8)
    yy, xx = np.mgrid[0:C.H, 0:C.W]
    for cy, cx, rad in ((120, 80, 12), (630, 350, 20)):
        img[np.hypot(xx - cx, yy - cy) < rad] = (255.0, 240.0, 226.0)
    bloom_case(img, 225.0, 0.22, None)

    for n, kw in enumerate((dict(), dict(contrast=0.18, sat=1.18), dict(black=0.05), dict(gamma=0.9),
                            dict(contrast=0.3, sat=0.8, black=0.02, gamma=1.2), dict(contrast=0.0, sat=1.0),
                            dict(sat=0.0), dict(contrast=-0.2, sat=2.0))):
        full = dict(contrast=0.18, sat=1.1, black=0.0, gamma=1.0)
        full.update(kw)
        img = r.uniform(-20, 280, (12, 16, 3)) if n % 2 else r.uniform(0, 255, (12, 16, 3))
        out = PO.grade(img, **full)
        assert out.dtype == np.float64
        g.add('reelkit.post.grade(img, contrast, sat, black, gamma)', hx(full), img=img, out=out)

    for strength, start, cap, cy in ((0.9, 0.55, 0.5, 0.47), (0.45, 0.55, 0.5, 0.47), (1.2, 0.3, 0.8, 0.5),
                                     (0.6, 0.0, 0.25, 0.4), (2.0, 0.7, 1.0, 0.47),
                                     # here one pixel's band depends on v * 5 being a float32 product
                                     (0.78, 0.43, 0.3, 0.6)):
        out = PO.vignette(strength, start, cap, cy)
        assert out.dtype == np.float64 and out.shape == (C.H, C.W, 1)
        g.add('reelkit.post.vignette(strength, start, cap, cy)',
              hx(dict(strength=strength, start=start, cap=cap, cy=cy)), out=out)
    for amount, top, span in ((0.58, 110, 120), (0.45, 100, 90), (0.3, 0, 640), (1.0, 200, 50), (0.58, 110.5, 33.3)):
        out = PO.top_scrim(amount, top, span)
        assert out.dtype == np.float64 and out.shape == (C.H, C.W, 1)
        g.add('reelkit.post.top_scrim(amount, top, span)', hx(dict(amount=amount, top=top, span=span)), out=out)
    return g


# ================================================================ scipy.spatial.ConvexHull (2-D)
def group_hull():
    from scipy.spatial import ConvexHull
    g = Group('hull', 'scipy.spatial.ConvexHull(pts).vertices for 2-D points',
              note='pts is (n, 2) float64; out the indices of the hull corners in the order scipy gives them '
                   '(counter-clockwise for the usual axes; int32 as scipy returns them). The first cases are the '
                   "spot's beam as kit.picture.render builds it: the can's screen position, then 32 points of the "
                   "pool's rim. The port follows Qhull's build up to its merging of facets and reports the inputs "
                   'that need it; none of these cases does (the test fails a case the port does not follow).')
    r = rng(18, 9)
    S = scene()

    def case(pts, kind):
        pts = np.ascontiguousarray(pts, np.float64)
        out = ConvexHull(pts).vertices
        assert out.dtype == np.int32 and out.ndim == 1
        g.add('scipy.spatial.ConvexHull(pts).vertices', dict(kind=kind), pts=pts, out=out)

    for n, c in enumerate((RING_CAM, RING_CAM_B, cam_args((0.12, 0.25, 0.0), 27.4, 0.7, 57.7, 36.0, 288.0),
                           cam_args((0.12, 0.25, 0.0), 26.9, 0.0, 58.0, 36.0, 288.0),
                           cam_args((0.12, 0.25, 0.0), 26.7, 0.3, 58.0, 36.0, 288.0),
                           cam_args((0.12, 0.25, 0.0), 27.3, 0.4, 58.4, 36.0, 288.0),
                           cam_args((0.12, 0.25, 0.0), 27.44, -0.31, 58.13, 36.0, 288.0),
                           cam_args((0.12, 0.25, 0.0), 27.52, 0.52, 57.9, 36.0, 288.0))):
        cam = cam_of(c)
        if n >= 4:                                        # the camera shake moves the optical centre
            cam.cx += float(r.uniform(-2.5, 2.5))
            cam.cy += float(r.uniform(-2.5, 2.5))
        aim = S.SPOT_AIM if n < 6 else S.SPOT_AIM + (np.array([1.3, 0.0, -0.8]) * 0.55 - S.SPOT_AIM) * 0.7
        s0 = cam.wp(S.SPOT[None])[0]
        pool_r = S.SPOT[1] * math.tan(math.acos(S.SPOT_COS[0])) * 0.92
        circ = cam.wp(np.array([aim + [pool_r * math.cos(a), 0.0, pool_r * math.sin(a)]
                                for a in np.linspace(0, 2 * math.pi, 33)[:-1]]))
        case(np.vstack([s0[None], circ]), 'beam')
    for count in (3, 4, 5, 6, 8, 10, 13, 20, 33, 40, 64, 100, 200, 7, 9, 16):
        case(r.uniform(-100, 100, (count, 2)) if count % 2 else r.normal(0, 30, (count, 2)), 'cloud')
    a = np.linspace(0, 2 * math.pi, 17)[:-1]
    case(np.stack([180 + 90 * np.cos(a), 320 + 40 * np.sin(a)], 1), 'ellipse')
    case(np.stack([50 * np.cos(-a), 50 * np.sin(-a)], 1), 'circle listed clockwise')
    case(np.vstack([np.stack([np.cos(a), np.sin(a)], 1), r.uniform(-0.5, 0.5, (20, 2))]), 'circle and inside')
    case([[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0], [2.0, 1.5]], 'rectangle and its centre')
    case([[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]], 'triangle')
    case([[0.0, 0.0], [0.0, 1.0], [1.0, 0.0]], 'triangle listed clockwise')
    case([[x, y] for y in range(4) for x in range(4)], 'grid 4 x 4')
    case([[0.0, 0.0], [2.0, 0.0], [4.0, 0.0], [4.0, 2.0], [4.0, 4.0], [2.0, 4.0], [0.0, 4.0], [0.0, 2.0]],
         'square with edge midpoints')
    case([[1.0, 1.0], [5.0, 2.0], [3.0, 6.0], [1.0, 1.0], [5.0, 2.0], [3.0, 3.0]], 'repeated points')
    # repeated points among the points still outside the hull: which of two equal points becomes the corner
    case([[4.0, 4.0], [2.0, 0.0], [0.0, 5.0], [4.0, 5.0], [5.0, 2.0], [5.0, 3.0], [5.0, 3.0]], 'repeated points')
    for count in (8, 13, 20, 31, 40, 60):
        pts = r.uniform(0, 100, (count, 2))
        pts[count // 2:] = pts[:count - count // 2]
        case(pts, 'cloud with every point twice')
    # a sliver of a first triangle (Qhull's narrow hull): points along a line, and tight clusters
    for count, noise in ((12, 1e-7), (30, 1e-4), (50, 1e-9), (20, 1e-2)):
        pts = r.uniform(0, 1, (count, 2))
        pts[:, 1] = pts[:, 0] * float(r.uniform(-2, 2)) + r.normal(0, noise, count)
        case(pts, 'near a line')
    for count, noise in ((25, 1e-6), (40, 1e-3)):
        centres = r.uniform(-1, 1, (4, 2))
        case(centres[r.integers(0, 4, count)] + r.normal(0, noise, (count, 2)), 'tight clusters')
    for count in (9, 14, 22):
        case(r.integers(-1000, 1000, (count, 2)).astype(float), 'whole numbers')
    # more slivers, and clouds that are their own mirror image (edges equally far from their furthest points)
    for n in range(24):
        count, noise = int(r.integers(5, 40)), 10.0 ** float(r.uniform(-9, -1))
        pts = r.uniform(0, 1, (count, 2))
        pts[:, 1] = pts[:, 0] * float(r.uniform(-2, 2)) + r.normal(0, noise, count)
        case(pts, 'near a line')
    for n in range(12):
        half = r.uniform(0.5, 100, (int(r.integers(3, 16)), 2))
        case(np.vstack([half, half * [-1.0, 1.0]]), 'mirrored cloud')
    # the leftmost point is also the lowest and the rightmost the highest: Qhull looks through all points for the
    # third corner of its first triangle and stops at the first that is good enough, not at the best
    case([[2.0, 2.0], [0.0, 1.0], [0.0, 4.0], [5.0, 5.0]], 'two extreme points')
    case([[0.42717604299714074, -0.6911374948232389], [0.3227164644367029, -0.3463056723841018],
          [0.7160104492788701, -0.9169147034270686], [0.47731479194666127, -0.730268616104071],
          [0.31495193284849243, -0.32473265210769015], [0.11716190341539512, -0.15693398989070725],
          [0.8181044010146792, -1.0852328527811017], [0.8445617849851076, -1.2324762025028602]], 'two extreme points')
    for n in range(6):
        count = int(r.integers(4, 12))
        along = np.sort(r.uniform(0, 10, count))
        case(np.stack([along, along * float(r.uniform(0.3, 3)) + r.uniform(-1.5, 1.5, count)], 1)[r.permutation(count)],
             'two extreme points')
    return g


GROUPS = (group_camera, group_geometry, group_raster, group_lighting, group_shadows, group_lens, group_fftconv,
          group_post, group_hull)
