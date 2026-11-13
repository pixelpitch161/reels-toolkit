"""Run-time patches of the kit's renderer (projects/ballfight/kit/picture.py) for older layers no Look hook can express
(plan m4 Task 4: one named opt-in per need, in Python here and in Rust as a new `Look` method whose default draws
nothing). Nothing under projects/ is edited: when `ballfight.kit.picture` is imported, the source of the functions
named below is read from the kit's file, the exact text patches are applied (each must match once), and the patched
functions replace the module's own, in its own globals.

Every opt-in is off unless a look asks for it: the patched code calls a hook only on a look that has it
(`hasattr(look, '<name>')`), and with no such look it computes exactly what the kit computes (every milestone 1 and 3
frame is unchanged; the older_vectors export checks it on the kit's own frames). The opt-ins, each a method an older
look may define (the Rust `Look` method of the same name, in bf-picture/src/look.rs):

    canvas_albedo(alb, C) -> alb     the canvas albedo of every canvas pixel ((n, 3) float64) after S.canvas_albedo,
                                     before the fading marks; C as canvas() gets it. TURNTABLES: the turning record
    canvas_paint(col, Ic, C) -> col  the lit canvas colour (alb * Ic, (n, 3) float64) before it is dithered.
                                     BAGPIPES: WAR CRY's tartan ring painted over the lit canvas
    ball_tint(kk, st, t, col) -> col ball kk's colour (its glass mix and its rim). VIOLA: VIOLA JOKE's frozen ball
    ball_glass(kk, st, t, inner, geo) -> inner   ball kk's inner glass ((n, n, 3) float64) after the sink, before it is
                                     laid on the frame; geo = dict(dx, dy, nz, band, inside) of the ball's patch (only
                                     the inside is used). BAGPIPES: the bag's tartan stretched over the swollen ball
    ball_lit(kk, st, t, lit) -> lit  ball kk's lit instrument sprite, after its own look's tint, before the sink.
                                     VIOLA: the frozen instrument greyed
    ball_after_sprite(kk, st, t, arr, s, rp)   draws on the frame (in place) after ball kk's sprite, before its rim,
                                     edge and highlight. BARI SAX: the harness's straps and ring
    ball_late(kk, F, st) -> bool     ball kk (with its own look's items_ball) is drawn after the front ropes and post,
                                     before over_ring, instead of in the painter pass. BARI SAX: up over a front rope
    rope_press(s, y, t, P, yy) -> yy the heights of a rope's points (yy = y - sag on entry; P the ring-frame points before
                                     the rope's give). BARI SAX: the ropes pressed down under the swing
    rope_events() -> [(t, side, x, z, speed)]  more rope events: added to ROPE_EV once the module is built (after the
                                     effects), sorted as the kit's list. BARI SAX: a pressed rope springing back
    after_post(img, F, drawn) -> img the frame in bytes after the post effects, before the count-in. TURNTABLES: the
                                     VHS rewind and its glyph; VIOLA: the rests and their counts

    PATCHES      [(name, fn)] in registration order
    register(name)   decorator: fn() is called once by older.install(), before the kit is imported
    apply()      call every registered patch once (older.install() does), then hook kit.picture's import
    TEXT         {function: [(opt-in, old, new)]}: every text patch, in registration order
    patch_module(module)   apply TEXT and the post-build steps to a loaded kit.picture (the import hook calls it)"""
import ast
import importlib.abc
import importlib.machinery
import sys

PATCHES = []
_APPLIED = set()
KIT_PICTURE = 'ballfight.kit.picture'
TEXT = {}                                        # function name -> [(opt-in, old, new)]
POST = []                                        # [(opt-in, fn(module))], run after the functions are patched


def register(name):
    def deco(fn):
        if any(n == name for n, _ in PATCHES):
            raise SystemExit(f'picture_patches: {name!r} is registered twice')
        PATCHES.append((name, fn))
        return fn
    return deco


def apply():
    for name, fn in PATCHES:
        if name not in _APPLIED:
            fn()
            _APPLIED.add(name)
    if not any(type(f).__name__ == '_PictureFinder' and type(f).__module__ == __name__ for f in sys.meta_path):
        sys.meta_path.insert(0, _PictureFinder())
    mod = sys.modules.get(KIT_PICTURE)
    if mod is not None:                          # imported before install(): its functions are patched now
        patch_module(mod)
    return [n for n, _ in PATCHES]


# ---------------------------------------------------------------- the mechanism
def _text(name, fn_name, old, new):
    TEXT.setdefault(fn_name, []).append((name, old, new))


def _patched_source(src, fn_name, patches, path):
    """the function's source with its patches applied (each must match exactly once), and its first line"""
    tree = ast.parse(src, path)
    node = next((n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == fn_name), None)
    if node is None:
        raise SystemExit(f'picture_patches: {path} has no function {fn_name}()')
    text = ast.get_source_segment(src, node)
    for name, old, new in patches:
        n = text.count(old)
        if n != 1:
            raise SystemExit(f'picture_patches: opt-in {name}: its patch of {fn_name}() matches {n} times, not once: '
                             f'{old[:80]!r}')
        text = text.replace(old, new)
    return text, node.lineno


def patch_module(module):
    """apply every text patch to the loaded kit.picture module (once), then the post-build steps"""
    if getattr(module, '_OLDER_PATCHED', False):
        return
    path = module.__file__
    with open(path, encoding='utf-8') as fh:
        src = fh.read()
    for fn_name, patches in TEXT.items():
        text, line = _patched_source(src, fn_name, patches, path)
        code = ast.parse(text, path)
        ast.increment_lineno(code, line - 1)
        exec(compile(code, path, 'exec'), module.__dict__)
    for _, fn in POST:
        fn(module)
    module._OLDER_PATCHED = True
    module.OLDER_OPTINS = [n for n, _ in PATCHES]


class _PictureLoader(importlib.abc.Loader):
    def __init__(self, inner):
        self.inner = inner

    def create_module(self, spec):
        return self.inner.create_module(spec)

    def exec_module(self, module):
        self.inner.exec_module(module)
        patch_module(module)

    def get_source(self, fullname):
        return self.inner.get_source(fullname)


class _PictureFinder(importlib.abc.MetaPathFinder):
    """finds kit.picture as the path finder does, and patches it as soon as its module body has run"""

    def find_spec(self, fullname, path=None, target=None):
        if fullname != KIT_PICTURE:
            return None
        spec = importlib.machinery.PathFinder.find_spec(fullname, path)
        if spec is None or spec.loader is None:
            return spec
        spec.loader = _PictureLoader(spec.loader)
        return spec


# ---------------------------------------------------------------- the opt-ins
@register('canvas_albedo')
def _canvas_albedo():
    _text('canvas_albedo', 'render',
          "        alb = S.canvas_albedo(sxp, szp)\n",
          "        alb = S.canvas_albedo(sxp, szp)\n"
          "        if any(hasattr(Lk, 'canvas_albedo') for Lk in LOOKS):     # (older opt-in canvas_albedo)\n"
          "            C0 = Ctx(t=t, i=i, in_fight=in_fight, fs=fs, sxp=sxp, szp=szp, ys=ys, xs=xs)\n"
          "            for Lk in LOOKS:\n"
          "                if hasattr(Lk, 'canvas_albedo'):\n"
          "                    alb = Lk.canvas_albedo(alb, C0)\n")


@register('canvas_paint')
def _canvas_paint():
    _text('canvas_paint', 'render',
          "        col = alb * Ic\n",
          "        col = alb * Ic\n"
          "        for Lk in LOOKS:                                          # (older opt-in canvas_paint)\n"
          "            if hasattr(Lk, 'canvas_paint'):\n"
          "                col = Lk.canvas_paint(col, Ic, C)\n")


@register('ball_tint')
def _ball_tint():
    _text('ball_tint', 'draw_ball',
          "    col = COL[k]\n",
          "    col = COL[k]\n"
          "    for L_ in LOOKS:                                                 # (older opt-in ball_tint)\n"
          "        if hasattr(L_, 'ball_tint'):\n"
          "            col = L_.ball_tint(k, st, t, col)\n")


@register('ball_glass')
def _ball_glass():
    _text('ball_glass', 'draw_ball',
          "        inner = inner * (1 - 0.72 * sink)\n    m = inside[sl]\n",
          "        inner = inner * (1 - 0.72 * sink)\n"
          "    if any(hasattr(L_, 'ball_glass') for L_ in LOOKS):              # (older opt-in ball_glass)\n"
          "        geo = dict(dx=dx, dy=dy, nz=nz, band=band, inside=inside)\n"
          "        for L_ in LOOKS:\n"
          "            if hasattr(L_, 'ball_glass'):\n"
          "                inner = L_.ball_glass(k, st, t, inner, geo)\n"
          "    m = inside[sl]\n")


@register('ball_lit')
def _ball_lit():
    _text('ball_lit', 'draw_ball',
          "            lit = over[1](lit)\n",
          "            lit = over[1](lit)\n"
          "        for L_ in LOOKS:                                             # (older opt-in ball_lit)\n"
          "            if hasattr(L_, 'ball_lit'):\n"
          "                lit = L_.ball_lit(k, st, t, lit)\n")


@register('ball_after_sprite')
def _ball_after_sprite():
    _text('ball_after_sprite', 'draw_ball',
          "        rg[ms] = np.clip(lit[ms], 0, 255)\n",
          "        rg[ms] = np.clip(lit[ms], 0, 255)\n"
          "    for L_ in LOOKS:                                                 # (older opt-in ball_after_sprite)\n"
          "        if hasattr(L_, 'ball_after_sprite'):\n"
          "            L_.ball_after_sprite(k, st, t, arr, s, rp)\n")


@register('ball_late')
def _ball_late():
    _text('ball_late', 'render',
          "    items = []\n    for name in ('red', 'blue'):\n",
          "    items = []\n"
          "    late_items = []                                                   # (older opt-in ball_late)\n"
          "    for name in ('red', 'blue'):\n")
    _text('ball_late', 'render',
          "        items.append((dz, 'ball', k))\n"
          "        for dz_, pay in LOOKS[k].items_ball(F, st):\n"
          "            items.append((dz_, 'look', (k, pay)))\n",
          "        late_ = any(hasattr(L_, 'ball_late') and L_.ball_late(k, F, st) for L_ in LOOKS)\n"
          "        dest = late_items if late_ else items\n"
          "        dest.append((dz, 'ball', k))\n"
          "        for dz_, pay in LOOKS[k].items_ball(F, st):\n"
          "            dest.append((dz_, 'look', (k, pay)))\n")
    _text('ball_late', 'render',
          "    draw_post('front')\n    arr, depth = fr.array(), fr.depth()\n",
          "    draw_post('front')\n"
          "    late_items.sort(key=lambda it: -it[0])                            # (older opt-in ball_late)\n"
          "    for dz, kind, obj in late_items:\n"
          "        if kind == 'ball':\n"
          "            st = fs[obj]\n"
          "            arr, depth = fr.array(), fr.depth()\n"
          "            drawn[obj] = draw_ball(arr, depth, cam, obj, st, t)\n"
          "            fr.set_array(arr, depth)\n"
          "        else:\n"
          "            LOOKS[obj[0]].draw_item(F, obj[1])\n"
          "    arr, depth = fr.array(), fr.depth()\n")


@register('rope_press')
def _rope_press():
    _text('rope_press', 'rope_points',
          "    sag = 0.035 * np.sin(np.pi * u)\n"
          "    P = P + outn[None] * disp[:, None]\n"
          "    return S.to_world(P[:, 0], P[:, 1], y - sag)",
          "    sag = 0.035 * np.sin(np.pi * u)\n"
          "    yy = y - sag\n"
          "    for L_ in LOOKS:                                                 # (older opt-in rope_press)\n"
          "        if hasattr(L_, 'rope_press'):\n"
          "            yy = L_.rope_press(s, y, t, P, yy)\n"
          "    P = P + outn[None] * disp[:, None]\n"
          "    return S.to_world(P[:, 0], P[:, 1], yy)")


def _more_rope_events(P):
    extra = [tuple(ev) for L in P.LOOKS if hasattr(L, 'rope_events') for ev in L.rope_events()]
    if extra:
        import numpy as np
        P.ROPE_EV = sorted(P.ROPE_EV + extra)
        P.ROPE_T = np.array([r[0] for r in P.ROPE_EV])


@register('rope_events')
def _rope_events():
    POST.append(('rope_events', _more_rope_events))


@register('after_post')
def _after_post():
    _text('after_post', 'render',
          "    img = np.ascontiguousarray(img)\n",
          "    img = np.ascontiguousarray(img)\n"
          "    for Lk in LOOKS:                                                  # (older opt-in after_post)\n"
          "        if hasattr(Lk, 'after_post'):\n"
          "            img = Lk.after_post(img, F, drawn)\n")


# ---------------------------------------------------------------- VIOLA (plan m4 Task 11): three more opt-ins
# (#42's render.py draws three things of the VIOLA that neither a Look hook nor the ten opt-ins above can express;
# each is off unless a look defines it, and without such a look the kit computes exactly what it computed before)
#
#   crowd_laugh(t) -> lk             the crowd's laugh (0..1) at t: heads bob "ha-ha-ha", bodies rock and lean, brighter,
#                                    every third one's arms up (#42 arena(), VIOLA JOKE)
#   flash_hit(e) -> bool             does this hit of the look's set off the crowd's flashbulbs (instead of the
#                                    `e[7] in flash_hits` test): #42 flashes only VIOLA JOKE's note (`e[8] == 2`), a test
#                                    the kind alone cannot make
#   spark_events                     (attribute) more event kinds that throw the default spark spray (#42's sparks() also
#                                    sparks 'crushed': a pizzicato dart flattened by a rolling note)
@register('crowd_laugh')
def _crowd_laugh():
    _text('crowd_laugh', 'arena',
          "    wk = win_k(t)\n    items = []\n",
          "    wk = win_k(t)\n"
          "    lk = 0.0                                                          # (older opt-in crowd_laugh)\n"
          "    for L_ in LOOKS:\n"
          "        if hasattr(L_, 'crowd_laugh'):\n"
          "            lk = max(lk, L_.crowd_laugh(t))\n"
          "    items = []\n")
    _text('crowd_laugh', 'arena',
          "            rh = 0.15 * cam.f / dh[j]\n",
          "            rh = 0.15 * cam.f / dh[j]\n"
          "            sway = 0.0                                                # (older opt-in crowd_laugh)\n"
          "            if lk > 0:\n"
          "                ph = CROWD_PH[j] * 6.28\n"
          "                y -= lk * (1.2 + 1.2 * abs(math.sin(2 * math.pi * 5.2 * t + ph))) * max(1.0, rh * 0.8)\n"
          "                sway = lk * 1.3 * math.sin(2 * math.pi * 1.7 * t + ph) * max(1.0, rh * 0.6)\n")
    _text('crowd_laugh', 'arena',
          "            body = np.clip(S.CROWD_COLS[CROWD_CI[j]] * tint * spill * crowd_lit[j], 0, 255)\n",
          "            if lk > 0:                                                # (older opt-in crowd_laugh)\n"
          "                body = np.clip(S.CROWD_COLS[CROWD_CI[j]] * tint * spill * crowd_lit[j] * (1 + 0.25 * lk), 0, 255)\n"
          "            else:\n"
          "                body = np.clip(S.CROWD_COLS[CROWD_CI[j]] * tint * spill * crowd_lit[j], 0, 255)\n")
    _text('crowd_laugh', 'arena',
          "            draw.ellipse([x - rh, y - rh, x + rh, y + rh], fill=tuple(int(v) for v in body * 0.62))\n",
          "            if lk > 0:                                                # (older opt-in crowd_laugh)\n"
          "                draw.ellipse([x - rh + sway, y - rh, x + rh + sway, y + rh], fill=tuple(int(v) for v in body * 0.62))\n"
          "            else:\n"
          "                draw.ellipse([x - rh, y - rh, x + rh, y + rh], fill=tuple(int(v) for v in body * 0.62))\n")
    _text('crowd_laugh', 'arena',
          "                draw.line([x - rh * 0.6, y - rh * 0.8, x + rh * 0.6, y - rh * 0.8], "
          "fill=tuple(int(v) for v in np.clip(body * 1.5 + 30, 0, 255)))\n"
          "            ph = CROWD_PH[j]\n",
          "                if lk > 0:                                            # (older opt-in crowd_laugh)\n"
          "                    draw.line([x - rh * 0.6 + sway, y - rh * 0.8, x + rh * 0.6 + sway, y - rh * 0.8],\n"
          "                              fill=tuple(int(v) for v in np.clip(body * 1.5 + 30, 0, 255)))\n"
          "                else:\n"
          "                    draw.line([x - rh * 0.6, y - rh * 0.8, x + rh * 0.6, y - rh * 0.8], "
          "fill=tuple(int(v) for v in np.clip(body * 1.5 + 30, 0, 255)))\n"
          "            if lk > 0.3 and rh >= 1.2 and (j % 3 == 0):              # (older opt-in crowd_laugh): arms up\n"
          "                ya_ = y + 0.9 * rh\n"
          "                draw.line([x - 1.6 * rh, ya_, x - 2.1 * rh + sway, ya_ - 1.6 * rh * lk], "
          "fill=tuple(int(v) for v in body * 0.8))\n"
          "                draw.line([x + 1.6 * rh, ya_, x + 2.1 * rh + sway, ya_ - 1.6 * rh * lk], "
          "fill=tuple(int(v) for v in body * 0.8))\n"
          "            ph = CROWD_PH[j]\n")


def _flash_by_hit(P):
    if not any(hasattr(L, 'flash_hit') for L in P.LOOKS):
        return
    import numpy as np

    def flashes(e):
        L = P.LOOKS[e[2]]
        return L.flash_hit(e) if hasattr(L, 'flash_hit') else e[7] in L.flash_hits
    P.FLASH_EV = [e[0] for e in P.EV if (e[1] == 'hit' and flashes(e)) or e[1] == 'ability'] + \
                 [P.T_KO + 0.03 * j for j in range(16)]
    P.FLASH_T = np.array(sorted(P.FLASH_EV))


@register('flash_hit')
def _flash_hit():
    POST.append(('flash_hit', _flash_by_hit))


@register('spark_events')
def _spark_events():
    _text('spark_events', 'sparks',
          "('hit', 'parry', 'block', 'wall', 'cleared', 'fade')):",
          "('hit', 'parry', 'block', 'wall', 'cleared', 'fade')\n"
          "                                  + tuple(x for L_ in LOOKS for x in getattr(L_, 'spark_events', ()))):"
          "   # (older opt-in spark_events)")
