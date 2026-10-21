"""Low-poly guitar-family builder (acoustic + electric) for the Pixel Pitch videos."""
import math
import numpy as np
import pp_framework as F
import violin_lowpoly as V
from piano_lowpoly import dim, mix, lerp, clamp, HOT, HOTSTR

def body_asym(g, prof, L, y0, d, top, side, colfn=None):
    pts = []
    for p in prof:
        u, wl, wr = (p[0], p[1], p[1]) if len(p) == 2 else p
        pts.append((y0 + u*L, -wl*L, wr*L))
    zf, zb = d/2, -d/2; nseg = len(pts) - 1
    for i in range(nseg):
        (ya, la, ra), (yb, lb, rb) = pts[i], pts[i+1]
        ref = ((la + ra + lb + rb)/4, (ya + yb)/2, 0)
        tc = colfn((i + .5)/nseg) if colfn else top
        g.face([(la, ya, zf), (ra, ya, zf), (rb, yb, zf), (lb, yb, zf)], tc, ref=ref)
        g.face([(la, ya, zb), (ra, ya, zb), (rb, yb, zb), (lb, yb, zb)], dim(side, .8), ref=ref)
        g.face([(ra, ya, zb), (rb, yb, zb), (rb, yb, zf), (ra, ya, zf)], side, ref=ref)
        g.face([(la, ya, zb), (lb, yb, zb), (lb, yb, zf), (la, ya, zf)], side, ref=ref)
    for (y, l, r), sg in ((pts[0], -1), (pts[-1], 1)):
        if r - l > 1e-4: g.face([(l, y, zb), (r, y, zb), (r, y, zf), (l, y, zf)], side, ref=((l + r)/2, y - sg, 0))

def stand(g, y0, d):
    metal = (40, 38, 46)
    g.box(-.22, .22, 0, .035, -.2, .14, metal, spec=.5)
    g.box(-.15, .15, .035, y0, -.05, .05, metal, spec=.5)
    g.box(-.014, .014, .035, y0 + .75, -d/2 - .08, -d/2 - .05, metal, spec=.5)

def tuners(g, xs_side, ys, zf, col=(214, 214, 222), length=.045):
    for x, y in zip(xs_side, ys):
        sg = 1 if x > 0 else -1
        g.box(min(x, x + sg*length), max(x, x + sg*length), y - .012, y + .012, zf - .03, zf - .01, col, spec=.6)

def headstock(g, kind, yn, zf, col, n, accent=(214, 214, 222)):
    z0, z1 = -.03, zf - .008
    if kind == "slot":
        V.slab_xy(g, [(-.06, yn), (.06, yn), (.065, yn + .24), (-.065, yn + .24)], z0, z1, col)
        for sg in (-1, 1):
            V.decal(g, [(sg*.036, yn + .04), (sg*.012, yn + .04), (sg*.012, yn + .2), (sg*.036, yn + .2)], z1 + .001, (20, 14, 12))
        tuners(g, [-.065]*3 + [.065]*3, [yn + .07, yn + .12, yn + .17]*2, zf, (232, 222, 200))
    elif kind == "steel":
        V.slab_xy(g, [(-.055, yn), (.055, yn), (.07, yn + .2), (.05, yn + .25), (-.05, yn + .25), (-.07, yn + .2)], z0, z1, col)
        tuners(g, [-.068]*3 + [.068]*3, [yn + .06, yn + .12, yn + .18]*2, zf)
        V.decal(g, F.ngon_xy(0, yn + .2, .018, 6), z1 + .001, (230, 220, 190))
    elif kind == "fender":
        V.slab_xy(g, [(-.035, yn), (.04, yn), (.045, yn + .1), (.05, yn + .22), (.025, yn + .3), (-.04, yn + .3), (-.06, yn + .26), (-.045, yn + .1)], z0, z1, col)
        tuners(g, [-.055]*6, [yn + .05 + .04*k for k in range(6)], zf)
    elif kind == "gibson":
        V.slab_xy(g, [(-.055, yn), (.055, yn), (.075, yn + .22), (.045, yn + .26), (-.045, yn + .26), (-.075, yn + .22)], z0, z1, col)
        tuners(g, [-.07]*3 + [.07]*3, [yn + .06, yn + .12, yn + .18]*2, zf, (226, 206, 150))
        V.decal(g, [(-.03, yn + .19), (.03, yn + .19), (.03, yn + .21), (-.03, yn + .21)], z1 + .001, (230, 200, 120))
    elif kind == "shark":
        V.slab_xy(g, [(-.035, yn), (.04, yn), (.025, yn + .3), (-.06, yn + .36), (-.075, yn + .3), (-.045, yn + .08)], z0, z1, col)
        tuners(g, [-.065]*6, [yn + .07 + .04*k for k in range(6)], zf, (30, 30, 34))
    elif kind == "oud":
        V.slab_xy(g, [(-.045, yn), (.045, yn), (.04, yn + .05), (-.04, yn + .05)], z0, z1, col)
        for k in range(5):
            y = yn + .04 + .045*k; zc = -.05 - .055*k; w = .04 - .003*k
            g.box(-w, w, y, y + .05, zc - .022, zc + .022, col)
            for sg in (-1, 1): g.box(min(sg*w, sg*(w + .045)), max(sg*w, sg*(w + .045)), y + .015, y + .035, zc - .01, zc + .01, (60, 36, 20))
    elif kind == "pan":
        V.slab_xy(g, [(-.05, yn), (.05, yn), (.06, yn + .17), (-.06, yn + .17)], z0, z1, col)
        tuners(g, [-.06]*3 + [.06]*3, [yn + .04, yn + .085, yn + .13]*2, zf)

def build_guitar(g, t, ev, fx, s, c):
    L, d = c["L"], c["d"]; zf = d/2; y0 = c.get("y0", .22)
    stand(g, y0, d)
    body_asym(g, c["prof"], L, y0, d, c["top"], c["side"], c.get("colfn"))
    for poly in c.get("horns", []):
        V.slab_xy(g, [(x*L, y0 + u*L) for x, u in poly], -d/2, d/2, c.get("horn_col", c["top"]))
    yt = y0 + L*c.get("neck_u", 1.0); yn = y0 + L + c["neck"]; yfb0 = y0 + c["fb0"]*L
    fw0, fw1 = c.get("fbw", (.07, .055))
    V.slab_xy(g, [(-fw0 + .006, yt - .06), (fw0 - .006, yt - .06), (fw1 - .004, yn), (-fw1 + .004, yn)], -.04, zf, c.get("neckcol", c["side"]))
    fbz = zf + .025
    V.slab_xy(g, [(-fw0, yfb0), (fw0, yfb0), (fw1, yn), (-fw1, yn)], zf, fbz, c["fbcol"], spec=.2)
    g.box(-fw1, fw1, yn - .008, yn + .004, zf, fbz + .012, (236, 230, 214))
    yb = y0 + c["bridge_u"]*L
    def fret_y(k): return yn - (yn - yb)*(1 - 2 ** (-k/12))
    nf = c.get("frets", 19)
    for k in range(1, nf + 1):
        y = fret_y(k)
        if y < yfb0 + .01: break
        w = lerp(fw0, fw1, (y - yfb0)/(yn - yfb0))
        V.thin(g, (-w, y), (w, y), .007, fbz + .0008, c.get("fret_col", (196, 196, 206)))
    if nf and c.get("inlays", True):
        for k in (3, 5, 7, 9, 12, 15, 17):
            ym = (fret_y(k) + fret_y(k - 1))/2
            if ym < yfb0 + .01: continue
            for x in ((-.022, .022) if k == 12 else (0,)):
                V.decal(g, V.octo(x, ym, .01, .01, 6), fbz + .001, c.get("inlay_col", (236, 230, 214)))
    st = c["strings"]; n = len(st)
    headstock(g, c["head"], yn, zf, c.get("headcol", c["side"]), n)
    hz = zf + .002; dark = (18, 12, 10)
    h = c.get("hole")
    if h:
        yh = y0 + h.get("u", .5)*L
        if h["type"] in ("round", "rose"):
            V.decal(g, F.ngon_xy(0, yh, h["r"] + .028, 16), hz, h.get("ring", (226, 200, 150)))
            V.decal(g, F.ngon_xy(0, yh, h["r"] + .012, 16), hz + .0005, h.get("ring2", (120, 70, 36)))
            if h["type"] == "round":
                V.decal(g, F.ngon_xy(0, yh, h["r"], 16), hz + .001, dark)
            else:
                V.decal(g, F.ngon_xy(0, yh, h["r"], 16), hz + .001, (226, 206, 160))
                for a in range(8):
                    V.decal(g, F.ngon_xy(.55*h["r"]*math.cos(a*math.pi/4), yh + .55*h["r"]*math.sin(a*math.pi/4), .012, 6), hz + .0015, dark)
                V.decal(g, F.ngon_xy(0, yh, .02, 6), hz + .0015, dark)
        elif h["type"] == "oud3":
            V.decal(g, F.ngon_xy(0, yh, h["r"] + .02, 16), hz, (200, 160, 90))
            V.decal(g, F.ngon_xy(0, yh, h["r"], 16), hz + .0005, (216, 186, 120))
            for a in range(10):
                V.decal(g, F.ngon_xy(.6*h["r"]*math.cos(a*.628), yh + .6*h["r"]*math.sin(a*.628), .01, 5), hz + .001, dark)
            for sg in (-1, 1): V.decal(g, F.ngon_xy(sg*.13*L, y0 + .4*L, .035, 12), hz, dark)
        elif h["type"] == "f":
            for sg in (-1, 1):
                a, b = (sg*.2*L, y0 + .32*L), (sg*.15*L, y0 + .58*L)
                V.thin(g, a, b, .02, hz, dark)
                for (x, y) in (a, b): V.decal(g, V.octo(x, y, .022, .022, 6), hz, dark)
    for poly, col in c.get("guards", []):
        F.decal_star(g, [(x*L, y0 + u*L) for x, u in poly], zf + .003, col)
    sb, sn = c["sb"], c["sn"]
    for (u, kind, col) in c.get("pickups", []):
        y = y0 + u*L
        if kind == "single":
            g.box(-sb - .022, sb + .022, y - .022, y + .022, zf, zf + .018, col)
        elif kind == "hum":
            g.box(-sb - .026, sb + .026, y - .042, y + .042, zf, zf + .02, col)
            V.thin(g, (-sb - .02, y), (sb + .02, y), .006, zf + .0205, dim(col, .6))
        elif kind == "blade":
            g.box(-sb - .04, sb + .04, y - .03, y + .03, zf, zf + .01, (226, 214, 180))
            g.box(-sb - .03, sb + .03, y - .012, y + .012, zf + .01, zf + .022, (24, 22, 26))
        elif kind == "horseshoe":
            for sg in (-1, 1): g.box(min(sg*(sb + .03), sg*(sb + .055)), max(sg*(sb + .03), sg*(sb + .055)), y - .035, y + .035, zf, zf + .07, col, spec=.8)
            g.box(-sb - .055, sb + .055, y - .035, y + .035, zf + .055, zf + .075, col, spec=.8)
    for (x, u) in c.get("knobs", []):
        g.box(x*L - .02, x*L + .02, y0 + u*L - .02, y0 + u*L + .02, zf, zf + .03, c.get("knob_col", (220, 220, 228)), spec=.6)
    br = c["bridge"]; sz = zf + .03
    tail = None
    if br == "acoustic":
        g.box(-.12, .12, yb - .04, yb + .025, zf, zf + .018, (40, 26, 20))
        g.box(-sb - .012, sb + .012, yb - .004, yb + .007, zf + .018, zf + .032, (236, 230, 214))
        for i in range(n): V.decal(g, F.ngon_xy(lerp(-sb, sb, i/(n - 1)), yb - .025, .007, 6), zf + .0185, (236, 230, 214))
    elif br == "classical":
        g.box(-.13, .13, yb - .04, yb + .02, zf, zf + .02, (64, 38, 22))
        g.box(-sb - .012, sb + .012, yb - .004, yb + .007, zf + .02, zf + .032, (236, 230, 214))
    elif br == "oud":
        g.box(-.15, .15, yb - .02, yb + .02, zf, zf + .016, (54, 32, 18)); sz = zf + .02
    elif br == "tele":
        V.decal(g, [(-sb - .04, yb - .09), (sb + .04, yb - .09), (sb + .04, yb + .03), (-sb - .04, yb + .03)], zf + .0035, (210, 210, 218))
        for k in range(3): g.box(lerp(-sb, sb, k/2) - .02, lerp(-sb, sb, k/2) + .02, yb - .012, yb + .012, zf + .004, zf + .028, (226, 226, 232), spec=.7)
    elif br == "trem":
        V.decal(g, [(-sb - .035, yb - .06), (sb + .035, yb - .06), (sb + .035, yb + .025), (-sb - .035, yb + .025)], zf + .0035, (210, 210, 218))
        for i in range(n): g.box(lerp(-sb, sb, i/(n - 1)) - .009, lerp(-sb, sb, i/(n - 1)) + .009, yb - .015, yb + .01, zf + .004, zf + .026, (226, 226, 232))
        g.line((sb + .04, yb - .04, zf + .02), (sb + .26, yb - .26, zf + .05), (226, 226, 232))
    elif br == "stop":
        g.box(-sb - .03, sb + .03, yb - .012, yb + .012, zf, zf + .03, (226, 206, 140), spec=.7)
        g.box(-sb - .035, sb + .035, yb - .1, yb - .08, zf, zf + .025, (226, 206, 140), spec=.7)
    elif br == "floyd":
        g.box(-sb - .045, sb + .045, yb - .1, yb + .03, zf, zf + .022, (24, 24, 28), spec=.6)
        for i in range(n): g.box(lerp(-sb, sb, i/(n - 1)) - .008, lerp(-sb, sb, i/(n - 1)) + .008, yb - .095, yb - .07, zf + .022, zf + .04, (40, 40, 46))
        g.line((sb + .05, yb - .05, zf + .02), (sb + .28, yb - .25, zf + .05), (30, 30, 34))
    elif br == "trapeze":
        g.box(-sb - .04, sb + .04, yb - .012, yb + .012, zf, zf + .03, (60, 40, 26))
        ytl = y0 + .05*L
        V.slab_xy(g, [(-.05, ytl), (.05, ytl), (.035, ytl + .16), (-.035, ytl + .16)], zf, zf + .012, (214, 190, 120), spec=.6)
        tail = ytl + .15
    elif br == "pan":
        g.box(-sb - .03, sb + .03, yb - .03, yb + .015, zf, zf + .028, (214, 214, 222), spec=.7)
    B = [(lerp(-sb, sb, i/(n - 1)), yb, sz) for i in range(n)]
    N = [(lerp(-sn, sn, i/(n - 1)), yn, fbz + .006) for i in range(n)]
    if tail is not None:
        for i in range(n): g.line((lerp(-.03, .03, i/(n - 1)), tail, zf + .02), B[i], dim(c["scol"][i], .7))
    pluck_y = y0 + c["pluck_u"]*L
    pluck_layer(g, t, ev, fx, st, B, N, c["scol"], yb, yn, pluck_y, c.get("slide", False), c.get("courses", False), s)
    if c.get("sparkle") and int(t*6) % 2 == 0:
        fx.append(("spark", (.3, y0 + .8*L, zf), 0)); fx.append(("spark", (-.32, y0 + .3*L, zf), 0))

def pluck_layer(g, t, ev, fx, strings, B, N, scol, yb, yn, pluck_y, slide, courses, s):
    FL = V.FLIP[0]; active = {}
    for age, m, v, dur, tp, cs in ev:
        if age < min(dur + .15, 1.4):
            active.setdefault(V.string_idx(strings, m), []).append((age, m))
    offs = (-.0045, .0045) if courses else (0.,)
    for i in range(len(strings)):
        b, nn = np.array(B[i]), np.array(N[i])
        if i in active:
            age, m = min(active[i])
            amp = .007*math.exp(-age*4) + .0015
            col = HOTSTR if age < .45 else mix(HOTSTR, scol[i], clamp((age - .45)/.6))
            for o in offs:
                pts = []
                for k in range(9):
                    u = k/8; p = b*(1 - u) + nn*u; p[0] += o + amp*math.sin(math.pi*u)*FL; pts.append(p)
                for k in range(8): g.line(pts[k], pts[k+1], col)
            kf = m - strings[i]
            if kf > 0 and not slide:
                mid = ((1 - 2 ** (-kf/12)) + (1 - 2 ** (-(kf - 1)/12)))/2
                fp = nn + (b - nn)*mid + [0, 0, .006]
                g.box(fp[0] - .018, fp[0] + .018, fp[1] - .014, fp[1] + .014, fp[2] - .004, fp[2] + .01, HOT, emit=True)
        else:
            for o in offs: g.line(b + [o, 0, 0], nn + [o, 0, 0], scol[i])
    if slide:
        past = [e for e in ev if e[2] >= .5]
        if past:
            cur = min(past, key=lambda e: e[0]); idx = sorted(past, key=lambda e: e[0])
            prev = idx[1] if len(idx) > 1 else cur
            k1 = cur[1] - strings[V.string_idx(strings, cur[1])]; k0 = prev[1] - strings[V.string_idx(strings, prev[1])]
            kf = lerp(k0, k1, F.ease(cur[0]/.09))
        else:
            kf = 3
        y = yn - (yn - yb)*(1 - 2 ** (-kf/12))
        w = lerp(abs(N[0][0]), abs(B[0][0]), (yn - y)/(yn - yb)) + .03
        z = lerp(N[0][2], B[0][2], (yn - y)/(yn - yb))
        g.box(-w, w, y - .016, y + .016, z + .004, z + .03, (222, 226, 234), spec=.9)
    chord_done = set()
    for age, m, v, dur, tp, cs in ev:
        i = V.string_idx(strings, m); bp, np_ = np.array(B[i]), np.array(N[i])
        u = (pluck_y - bp[1])/(np_[1] - bp[1]); pp = bp + (np_ - bp)*u
        if age < .08 and cs < 3: fx.append(("spark", tuple(pp + [0, 0, .02]), age))
        if tp and cs >= 3 and age < .15 and age not in chord_done:
            chord_done.add(age)
            a0, n0 = np.array(B[0]), np.array(N[0]); a1, n1 = np.array(B[-1]), np.array(N[-1])
            u0 = (pluck_y - a0[1])/(n0[1] - a0[1])
            fx.append(("strum", tuple(a0 + (n0 - a0)*u0 + [0, 0, .03]), tuple(a1 + (n1 - a1)*u0 + [0, 0, .03]), age))
        if tp and v >= .5 and age < .9: fx.append(("note", (pp[0] + .42, pluck_y + .7, pp[2]), age))
