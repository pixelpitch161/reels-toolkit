"""#40 weapon styles: none new. Crossover rule: both fighters look exactly as in their own episodes, so their draw
functions are imported from those episodes' folders (series rule: styles live there so crossovers can import them),
not copied:

    SX  projects/ballfight_bassoon_vs_barisax/styles.py (#39 v2): the BARI SAX's WRECKING BALL (`STEEL`, `Y_RIG`,
        `line3`, `cable`, `hook_block`, `hook_shape`, the ball's `harness`, the `swing_trail`), GROWL's gravelly ring
        (`honk_light`, `draw_honk_front`) and CABLE SNAP's target on the canvas (`snap_target`)
    SC  projects/ballfight_bassclarinet_vs_euphonium/styles.py (#38): the BASS CLARINET's ENDPIN ANCHOR (`whirl_light`,
        `peg_light`, `draw_arm`), MULTIPHONICS' twin (`draw_ghost_ball`), SLAP TONGUE's paddle (`draw_slap`) and
        LIGHTS OUT's pool + eruption (`shadow_pool`, `erupt_light`)

Both modules import `scene` (the shared ring) at load, as here.

The only thing this episode adds is `mirrored(pattern)` for the VS title card: each fighter's panel pattern (#38 / #39,
projects/ballfight/title.py PATTERNS) was drawn for the side it fought on in its own episode, and the sides are swapped
here, so the pattern is reflected across the card's diagonal divider onto the other panel (same pattern, mirrored)."""
import importlib.util
import os
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))


def _load(name, folder):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, '..', folder, 'styles.py'))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


SX = _load('styles39', 'ballfight_bassoon_vs_barisax')            # BARI SAX v2 (WRECKING BALL, GROWL, CABLE SNAP)
SC = _load('styles38', 'ballfight_bassclarinet_vs_euphonium')     # BASS CLARINET (ENDPIN ANCHOR, SLAP TONGUE, ...)


def mirrored(pattern):
    """the title card's panel `pattern` (fn(base rgb (H, W, 3)) -> rgb) reflected across the diagonal divider: every
    pixel shows what the pattern draws at its mirror image on the other side (nearest pixel; the panel gradient is
    symmetric about the divider, so only the pattern moves). Pixels whose mirror falls off the frame keep the plain
    gradient."""
    import title as TT
    px = TT.XX - 2.0 * TT.SIDE * TT.N[0]
    py = TT.YY - 2.0 * TT.SIDE * TT.N[1]
    xi, yi = np.rint(px).astype(int), np.rint(py).astype(int)
    inside = (xi >= 0) & (xi < TT.W) & (yi >= 0) & (yi < TT.H)
    xi, yi = np.clip(xi, 0, TT.W - 1), np.clip(yi, 0, TT.H - 1)

    if getattr(pattern, 'mirrored', False):     # already mirrored (title.py re-imports the renderer by name): keep it
        return pattern

    def fn(base):
        drawn = pattern(base)
        return np.where(inside[..., None], drawn[yi, xi], base)
    fn.mirrored = True
    return fn
