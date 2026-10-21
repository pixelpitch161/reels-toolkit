"""Lit 3D path for the Pixel Pitch engine (shared lighting from reelkit, adapted to the pedestal stage).

Turn it on in a video script with   LIGHTING = True   (or a dict of overrides, see DEFAULTS).
pp_framework.frame() then calls render_lit() instead of the flat renderer. Everything else
(background, colours, pedestal palette, beat, captions, fx sprites) stays exactly as the video defines it.

What it adds
    key light      one directional light with REAL shadows (shadow map): the instrument shades itself
                   and drops a shadow across the pedestal top that turns with the instrument
    sky / ambient  tinted from the video's own background, so every video gets its own light colour
    rim light      backlight in the background's brightest colour (sun, floodlights, neon) on the silhouette
    brass shine    spec faces get a stepped Blinn highlight plus a background-coloured reflection
    contact AO     soft darkening where the instrument meets the pedestal (replaces the flat foot blob)
    glow           bloom from emissive parts + highlights only (backgrounds stay crisp pixel art)
"""
import math
import numpy as np
from PIL import Image, ImageFilter
from reelkit.lighting import Lights, shade
from piano_lowpoly import Geo, Cam, outline, build_pedestal, mix, RW, RH, F_PX, CX, CY, B4

YY, XX = np.mgrid[0:RH, 0:RW]
BAY = B4[YY % 4, XX % 4]

DEFAULTS = dict(
    key_dir=(-0.55, 1.25, 0.62),   # toward the light (world): upper-left, a little in front -> shadow falls back-right
    key=0.64,                      # key strength
    key_tint=0.28,                 # how much the key takes the background's highlight colour (0 = white)
    amb=0.34,                      # ambient strength (coloured by the background average)
    sky=0.08,                      # extra light on up-facing surfaces
    rim=0.55,                      # rim/backlight strength (colour = brightest part of the background)
    shadow_soft=1,                 # PCF radius in shadow-map texels (0 = hard pixel shadows)
    ao=0.55,                       # contact-shadow strength under the instrument
    spec_reflect=0.18,             # background reflection on spec (metal) faces
    bloom=0.35,
    foot_blob=False,               # keep the old flat shadow blob as well
)


def cfg(c):
    d = dict(DEFAULTS)
    if isinstance(getattr(c, "LIGHTING", None), dict): d.update(c.LIGHTING)
    return d


# ---------------------------------------------------------------- rasterisers
def _bbox(S, W_, H_):
    x0 = max(int(math.floor(S[:, 0].min())), 0); x1 = min(int(math.ceil(S[:, 0].max())), W_ - 1)
    y0 = max(int(math.floor(S[:, 1].min())), 0); y1 = min(int(math.ceil(S[:, 1].max())), H_ - 1)
    return x0, x1, y0, y1


def _bary(S, x0, x1, y0, y1):
    (ax, ay), (bx, by), (cx, cy) = S
    area = (bx-ax)*(cy-ay) - (by-ay)*(cx-ax)
    if abs(area) < 1e-12: return None
    X, Y = np.meshgrid(np.arange(x0, x1+1) + .5, np.arange(y0, y1+1) + .5)
    w0 = ((cx-bx)*(Y-by) - (cy-by)*(X-bx))/area
    w1 = ((ax-cx)*(Y-cy) - (ay-cy)*(X-cx))/area
    w2 = 1 - w0 - w1
    m = (w0 >= -1e-6) & (w1 >= -1e-6) & (w2 >= -1e-6)
    return (w0, w1, w2, m) if m.any() else None


def tri_id(fid, zb, gb, S, D, i, gid):
    """camera raster: nearest-wins on 1/z, writes face index + group"""
    x0, x1, y0, y1 = _bbox(S, RW, RH)
    if x0 > x1 or y0 > y1: return
    b = _bary(S, x0, x1, y0, y1)
    if b is None: return
    w0, w1, w2, m = b
    iz = w0/D[0] + w1/D[1] + w2/D[2]
    sub = zb[y0:y1+1, x0:x1+1]; m &= iz > sub
    sub[m] = iz[m]; fid[y0:y1+1, x0:x1+1][m] = i; gb[y0:y1+1, x0:x1+1][m] = gid


def tri_min(buf, S, D):
    """orthographic depth raster: keeps the smallest linear depth"""
    H_, W_ = buf.shape
    x0, x1, y0, y1 = _bbox(S, W_, H_)
    if x0 > x1 or y0 > y1: return
    b = _bary(S, x0, x1, y0, y1)
    if b is None:            # edge-on sliver: splat its vertices so thin parts still cast
        for (sx, sy), d in zip(S, D):
            xi, yi = int(sx), int(sy)
            if 0 <= xi < W_ and 0 <= yi < H_: buf[yi, xi] = min(buf[yi, xi], d)
        return
    w0, w1, w2, m = b
    z = w0*D[0] + w1*D[1] + w2*D[2]
    sub = buf[y0:y1+1, x0:x1+1]; m &= z < sub
    sub[m] = z[m]


def line_min(buf, S, D):
    H_, W_ = buf.shape
    n = int(max(abs(S[1, 0]-S[0, 0]), abs(S[1, 1]-S[0, 1]))) + 2
    t = np.linspace(0, 1, n)
    xi = (S[0, 0] + (S[1, 0]-S[0, 0])*t).astype(int); yi = (S[0, 1] + (S[1, 1]-S[0, 1])*t).astype(int)
    z = D[0] + (D[1]-D[0])*t
    m = (xi >= 0) & (xi < W_) & (yi >= 0) & (yi < H_)
    np.minimum.at(buf, (yi[m], xi[m]), z[m])


class OrthoMap:
    """depth map seen from direction d (unit, toward the light); texel ~ world units `tex`"""
    def __init__(s, d, pts, tex):
        s.d = d / np.linalg.norm(d)
        a = np.array([0, 1.0, 0]) if abs(s.d[1]) < .95 else np.array([1.0, 0, 0])
        s.u = np.cross(a, s.d); s.u /= np.linalg.norm(s.u); s.v = np.cross(s.d, s.u)
        U, V = pts @ s.u, pts @ s.v
        pad = 4*tex
        s.u0, s.v0 = U.min() - pad, V.min() - pad
        s.tex = tex
        s.W = int((U.max() + pad - s.u0)/tex) + 2; s.H = int((V.max() + pad - s.v0)/tex) + 2
        s.buf = np.full((s.H, s.W), np.inf)
    def to(s, P):
        return np.stack([(P @ s.u - s.u0)/s.tex, (P @ s.v - s.v0)/s.tex], -1), -(P @ s.d)


# ---------------------------------------------------------------- background-driven light colours
def bg_light(img, c, ctx):
    """ambient = background average, highlight = mean of its brightest pixels (sun, floods, neon)"""
    f = img.reshape(-1, 3).astype(float)
    avg = f.mean(0)
    lum = f @ np.array([.3, .55, .15])
    hi = f[lum >= np.percentile(lum, 98.5)].mean(0)
    def norm(v, floor):
        v = np.maximum(v, 1.0); v = v/v.max()
        return floor + (1 - floor)*v
    return norm(avg, .55), norm(hi, .35)


# ---------------------------------------------------------------- the lit render
def render_lit(c, ctx, s, t, scale, spin, fx):
    L = cfg(c)
    T = ctx["T"]; th0, th1, kb = ctx["th0"], ctx["th1"], ctx["kb"]
    img = c.background(c, ctx)
    pc0, pc1 = c.ped_cols(th0), c.ped_cols(th1); pc = {k: mix(pc0[k], pc1[k], kb) for k in pc1}
    yaw = c.YAW + c.SWAY*math.sin(2*math.pi*T/8) + spin
    cam = Cam(c.DIST, yaw, scale*c.ISCALE*s.get("size", 1.0))

    gp = Geo(); build_pedestal(gp, c.DIST, ctx["pulse"], c.FOOT*scale if L["foot_blob"] else 0, pc)
    gi = Geo()
    if scale > .01:
        ev = [(t - e[0],) + tuple(e[1:]) for e in s["ev"] if t >= e[0]]
        c.build(gi, t, ev, fx, s)

    # ---- collect faces in world space
    Pw, Nw, COL, TWO, EMIT, GID, SPEC = [], [], [], [], [], [], []
    for geo, sc in ((gp, 1.0), (gi, cam.scale)):
        if sc < 1e-3: continue
        for P, n, col, two, emit, gid, spec in geo.F:
            Pw.append(cam.world(P, sc)); Nw.append(cam.R @ n)
            COL.append(col); TWO.append(two); EMIT.append(emit); GID.append(gid); SPEC.append(spec)
    lines = [(cam.world(np.stack([a, b]), cam.scale), col, gid) for a, b, col, gid in gi.L] if cam.scale > 1e-3 else []
    nF = len(Pw)
    rend = np.zeros((RH, RW, 3), float); zb = np.zeros((RH, RW)); gb = np.zeros((RH, RW), np.int8)
    fid = np.full((RH, RW), -1, np.int32)
    Nf = np.zeros((nF, 3)); facing = np.ones(nF, bool)

    # ---- camera pass (face ids)
    for i in range(nF):
        W = Pw[i]; nw = Nw[i]
        v = cam.eye - W.mean(0); v /= np.linalg.norm(v)
        if nw @ v <= 0:
            if not TWO[i]: facing[i] = False; continue
            nw = -nw
        Nf[i] = nw
        S, D = cam.proj(W)
        if (D <= .05).any(): continue
        for k in range(1, len(W) - 1):
            tri_id(fid, zb, gb, S[[0, k, k+1]], D[[0, k, k+1]], i, GID[i])

    # ---- shadow map from the key light (instrument + pedestal cast)
    kd = np.asarray(L["key_dir"], float); kd /= np.linalg.norm(kd)
    allp = np.concatenate(Pw) if nF else np.zeros((1, 3))
    tex = c.DIST/420
    sm = OrthoMap(kd, allp, tex)
    for i in range(nF):
        if EMIT[i] and GID[i] == 1: continue                       # pedestal light strips / blob never cast
        S, D = sm.to(Pw[i])
        for k in range(1, len(S) - 1): tri_min(sm.buf, S[[0, k, k+1]], D[[0, k, k+1]])
    for W, col, gid in lines:
        S, D = sm.to(W); line_min(sm.buf, S, D)

    # ---- top-down height map of the instrument for contact AO
    inst = [i for i in range(nF) if GID[i] != 1]
    ao_map = None
    if inst and L["ao"] > 0:
        texa = c.DIST/160
        am = OrthoMap(np.array([0, 1.0, 0]), np.concatenate([Pw[i] for i in inst] + [allp[:1]]), texa)
        for i in inst:
            S, D = am.to(Pw[i])
            y = Pw[i][:, 1]                                            # store the lowest height, not depth
            for k in range(1, len(S) - 1): tri_min(am.buf, S[[0, k, k+1]], y[[0, k, k+1]])
        hb = .08*c.DIST                                              # height band that counts as "touching"
        occ = np.clip(1 - am.buf/hb, 0, 1); occ[~np.isfinite(am.buf)] = 0
        im8 = Image.fromarray((occ*255).astype(np.uint8))
        blur = np.asarray(im8.filter(ImageFilter.GaussianBlur(3))).astype(float)/255
        ao_map = (am, np.clip(blur*1.6, 0, 1))

    # ---- per-pixel shading
    m = fid >= 0
    ys, xs = np.nonzero(m); ids = fid[m]
    d = 1/zb[m]
    Q = (d[:, None]*cam.fwd + (d*(xs + .5 - CX)/F_PX)[:, None]*cam.right - (d*(ys + .5 - CY)/F_PX)[:, None]*cam.up)
    Pp = cam.eye + Q
    nrm = Nf[ids]
    vdir = -Q/np.linalg.norm(Q, axis=1, keepdims=True)

    # shadow lookup with normal offset + small PCF
    off = Pp + nrm*tex*1.5 + kd*tex*0.8
    S, D = sm.to(off)
    vis = np.zeros(len(D)); r = int(L["shadow_soft"]); cnt = 0
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            xi = np.clip(S[:, 0].astype(int) + dx, 0, sm.W - 1); yi = np.clip(S[:, 1].astype(int) + dy, 0, sm.H - 1)
            vis += D <= sm.buf[yi, xi] + tex*1.2; cnt += 1
    vis /= cnt
    vis = np.floor(vis*3 + BAY[ys, xs]*.999)/3                       # stepped, dithered penumbra (pixel-art)
    # back-facing to the key: no direct light (avoids light leaking through closed shapes)
    vis = np.where((nrm @ kd) > 0, vis, 0.0)

    amb_c, hi_c = bg_light(img, c, ctx)
    amb_c = mix_v(np.ones(3), amb_c, .6)
    Lt = Lights(amb=L["amb"]*amb_c, sky=L["sky"]*amb_c)
    keyc = L["key"]*mix_v(np.ones(3), hi_c, L["key_tint"])
    Lt.key("key", kd, keyc)
    Lt.view = cam.eye/np.linalg.norm(cam.eye)
    base = np.array(COL, float)[ids]
    emit = np.array(EMIT)[ids]; spec = np.array(SPEC, float)[ids]
    col = shade(Lt, nrm, Pp, base, kvis=vis[:, None], rimk=0.0, step=1)

    # rim / backlight in the background's highlight colour (per-pixel view vector)
    fr = (1 - np.clip(np.abs((nrm*vdir).sum(1)), 0, 1))**2.2
    rb = np.array([-.4*kd[0], .45, -1.0]); rb -= (rb @ cam.fwd)*cam.fwd; rb /= np.linalg.norm(rb)   # light from behind
    back = np.clip(nrm @ rb, 0, 1)
    isinst = np.array(GID)[ids] != 1                                  # the pedestal keeps its designed colours
    col += (L["rim"]*fr*(.35 + .65*back)*isinst)[:, None]*hi_c[None]*base*0.9

    # metal: stepped highlight + reflected background colour
    if spec.any():
        h = kd[None] + vdir; h /= np.linalg.norm(h, axis=1, keepdims=True)
        sp = np.clip((nrm*h).sum(1), 0, 1)**40*vis
        lvl = np.where(sp > .7, 110, np.where(sp > .3, 50, 0))
        col += (spec*lvl)[:, None]*mix_v(np.ones(3), hi_c, .3)[None]
        col += (spec*L["spec_reflect"]*fr)[:, None]*hi_c[None]*255*.35

    # contact AO on everything near the pedestal top
    if ao_map is not None:
        am, occ = ao_map
        Sa, _ = am.to(Pp)
        xi = np.clip(Sa[:, 0].astype(int), 0, am.W - 1); yi = np.clip(Sa[:, 1].astype(int), 0, am.H - 1)
        near = (np.abs(Pp[:, 1]) < .02*c.DIST) & (nrm[:, 1] > .7) & (np.array(GID)[ids] == 1)
        a = occ[yi, xi]*near*L["ao"]
        a = np.floor(a*4 + BAY[ys, xs]*.999)/4
        col *= (1 - a)[:, None]

    col = np.where(emit[:, None], base, col)
    col = np.clip(np.round(col/5)*5, 0, 255)
    rend[ys, xs] = col

    # ---- strings and other line work: lit by ambient + key, drawn on top
    if lines:
        lum = float(np.clip(L["amb"] + L["key"]*.6, .6, 1.15))
        for W, lc, gid in lines:
            S2, D2 = cam.proj(W)
            if (D2 <= .05).any(): continue
            n = int(max(abs(S2[1, 0]-S2[0, 0]), abs(S2[1, 1]-S2[0, 1]))) + 2
            tt = np.linspace(0, 1, n)
            X = S2[0, 0] + (S2[1, 0]-S2[0, 0])*tt; Y = S2[0, 1] + (S2[1, 1]-S2[0, 1])*tt
            IZ = (1/D2[0])*(1-tt) + (1/D2[1])*tt
            xi, yi = X.astype(int), Y.astype(int)
            k = (xi >= 0) & (xi < RW) & (yi >= 0) & (yi < RH)
            xi, yi, IZ = xi[k], yi[k], IZ[k]
            ok = IZ*1.004 >= zb[yi, xi]
            rend[yi[ok], xi[ok]] = np.clip(np.array(lc, float)*lum, 0, 255); gb[yi[ok], xi[ok]] = gid

    # ---- glow from emissive bits and highlights only
    if L["bloom"] > 0:
        src = np.clip(rend.max(-1, keepdims=True) - 205, 0, None)/50*rend
        if m.any():
            em = np.zeros((RH, RW), bool); em[ys, xs] = emit
            src = np.where(em[..., None], rend*.8, src)
        small = Image.fromarray(np.clip(src, 0, 255).astype(np.uint8)).resize((RW//3, RH//3), Image.BILINEAR)
        small = small.filter(ImageFilter.GaussianBlur(2.2))
        glow = np.asarray(small.resize((RW, RH), Image.BILINEAR)).astype(float)*L["bloom"]
    else:
        glow = 0

    r8 = rend.astype(np.uint8)
    outline(r8, zb, gb)
    msk = gb > 0
    out = img.astype(float)
    out[msk] = r8[msk]
    out = out + glow
    return np.clip(out, 0, 255).astype(np.uint8), cam


def mix_v(a, b, k): return np.asarray(a, float)*(1 - k) + np.asarray(b, float)*k
