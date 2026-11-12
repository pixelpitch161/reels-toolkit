"""theme_brass_trailer: the cinematic brass trailer's milestone 3 references (Template T, Task 30).

studio/reference/themes/brass_trailer/ (python -m tools.studio_export theme_brass_trailer):

    theme.json      the two reference configs: examples/trombone_vs_horn (#34, the plain bed on a 4-chord loop) and
                    ballfight_horn_vs_trombone_rematch (#54, THE REMATCH: `TRAILER = dict(sequel=True)`, an 8-chord loop
                    and a 5-bar tail): the kit's own recording of each and the bed alone under two kit base Sounds: the
                    16-bit master's SHA-256 and the buses; each config's theme options as the fight file names them
                    (`theme_options`: {'sequel': true} for the rematch, {} for #34)
    voicings.json   Bed.trem (the high tremolo's three lowest chord tones from C#5) and Bed.pad_notes (the sequel's
                    cello pad, the three lowest from A2) for the 12 roots x 13 qualities; Bed.root and Bed.key_root
                    are numbers (T.root_above(ch, 33), T.KEY_ROOT), proven by the harmony's own tests
    bed.json        the bed alone's RMS per bus; on #34: a second tempo, a second key and the option `sequel` = true
                    (the value the kit has besides its default); `boom` and `trailer_hit` called directly at three
                    times with three gains (vectors cached)

The mood plays no fighter's voice (no brass of its own: the two brass fighters are the brass section).

Two run-time adaptations, both in this module only (nothing shared is edited):

- `theme_options`: m3refs.read_config reads the config's music but not the mood's options, so a reference config that
  sets one (the rematch's TRAILER) is recorded here: read_config is wrapped while the export runs to add the
  options the brass trailer reads, under the fight file's names. The harness's `config_fight_file` must apply them
  (reported to the controller as a shared-file addition).
- `trailer_hit`: m3parts.HOOK_CALLS passes the braam's length as the keyword `braam`, but the kit's method names it
  `braam_dur`, so m3parts.hook_child would raise TypeError. As theme_bossa_noir did for `hit`, the shared child runs
  through `_hook_child`, which first gives the kit's `Bed.trailer_hit` (run-time only, in that child process) a
  wrapper taking `braam=` and calling the kit's method with the same values in the same order.
- stem windows: m3parts._soundtrack records every window as SR samples long (`n=SR`) but caches only the samples the
  stem has (`x[i0:i0 + SR]`), so a window that runs past the end of the soundtrack (the tempo variant's
  `T_KO + 3.0`: its K.O. is at 57.7 s) has a cached length other than its recorded one. `_bed_child` runs the shared
  child and records each window's `n` as the samples there are (`min(SR, NA - i0)`); a window of a whole second is
  unchanged. Reported to the controller as a shared-file fix (then the wrapper changes nothing)."""
from . import m3refs

SPEC = dict(
    name='brass_trailer',
    voicings=('trem', 'pad_notes'),
    configs=('examples/trombone_vs_horn', 'ballfight_horn_vs_trombone_rematch'),
    options=(dict(name='sequel', text='TRAILER = dict(sequel=True)\n', ff=dict(sequel=True),
                  why='TRAILER = dict(sequel=True): the sequel (#54), darker and bigger, on #34\'s 4-chord loop'),),
)


def _options_of(config):
    """the brass trailer's options a kit config sets, under the fight file's names: TRAILER.sequel -> sequel"""
    import importlib.util
    spec = importlib.util.spec_from_file_location('m3refs_config_options', m3refs.config_source(config))
    cfg = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cfg)
    out = {}
    trailer = getattr(cfg, 'TRAILER', None)
    if trailer is not None and 'sequel' in trailer:
        out['sequel'] = bool(trailer['sequel'])
    return out


def _hook_child(cfg_folder, hooks, out_path):
    """m3parts.hook_child, with the kit bed's `trailer_hit(mix, t, taiko_g, seed, braam_dur, braam_g)` reachable as
    `trailer_hit(mix, t, taiko_g=, seed=, braam=, braam_g=)`"""
    import importlib
    from . import common, m3parts as MP
    common.kit()
    mood = importlib.import_module('ballfight.kit.moods.brass_trailer')
    kit_hit = mood.Bed.trailer_hit

    def trailer_hit(self, mix, t, taiko_g, seed, braam=None, braam_g=0.0):
        return kit_hit(self, mix, t, taiko_g, seed, braam, braam_g)
    mood.Bed.trailer_hit = trailer_hit
    return MP.hook_child(cfg_folder, hooks, out_path)


def _bed_child(cfg_folder, stems_out):
    """m3parts.bed_child, each stem window's `n` the samples the soundtrack has there"""
    from . import m3parts as MP
    out = MP.bed_child(cfg_folder, stems_out)
    from ballfight.kit import music as M              # noqa: N812  (loaded by the shared child)
    for w in out['windows']:
        w['n'] = max(0, min(w['n'], M.NA - w['i0']))
    return out


def export():
    from . import m3parts as MP
    shared = MP.hook_child, MP.bed_child, m3refs.read_config

    def read_config(config):
        return dict(shared[2](config), theme_options=_options_of(config))
    MP.hook_child, MP.bed_child, m3refs.read_config = _hook_child, _bed_child, read_config
    try:
        m3refs.theme_refs(SPEC)
    finally:
        MP.hook_child, MP.bed_child, m3refs.read_config = shared


EXPORTS = {'theme_brass_trailer': export}
