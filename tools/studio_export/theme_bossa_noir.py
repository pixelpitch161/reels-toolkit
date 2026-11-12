"""theme_bossa_noir: the minor bossa noir's milestone 3 references (Template T, Task 29).

studio/reference/themes/bossa_noir/ (python -m tools.studio_export theme_bossa_noir):

    theme.json      the reference config (ballfight_flute_vs_clarinet, #49): the kit's own recording of it and the bed
                    alone under two kit base Sounds: the 16-bit master's SHA-256 and the buses
    voicings.json   Bed.voicing (the nylon guitar's four-note shape, E3 up) for the 12 roots x 13 qualities
    bed.json        the bed alone's RMS per bus; a second tempo and a second key (recordings cached); `boom` and `hit`
                    called directly at three times with three gains (vectors cached)

The mood has no options (it reads nothing of the config but the music) and plays no fighter's voice.

`hit`: m3parts.HOOK_CALLS passes the chord as the keyword `chord`, but every kit mood's `hit` names that argument
`ch`, so m3parts.hook_child raises TypeError on it. Until the shared child passes it under the method's own name,
this export runs the shared child through `_hook_child`, which first gives the kit's `Bed.hit` (run-time only, in
that child process) a wrapper that takes the keyword `chord` and calls the kit's method with the same values in the
same order. Nothing the bed plays changes; the call's arguments are recorded as the shared child records them."""
from . import m3refs

SPEC = dict(
    name='bossa_noir',
    voicings=('voicing',),
    configs=('ballfight_flute_vs_clarinet',),
)


def _hook_child(cfg_folder, hooks, out_path):
    """m3parts.hook_child, with the kit bed's `hit(mix, t, ch, g, ring)` reachable as `hit(mix, t, chord=, g=, ring=)`"""
    import importlib
    from . import common, m3parts as MP
    common.kit()
    mood = importlib.import_module('ballfight.kit.moods.bossa_noir')
    kit_hit = mood.Bed.hit

    def hit(self, mix, t, chord, g=1.0, ring=0.9):
        return kit_hit(self, mix, t, chord, g, ring)
    mood.Bed.hit = hit
    return MP.hook_child(cfg_folder, hooks, out_path)


def export():
    from . import m3parts as MP
    shared = MP.hook_child
    MP.hook_child = _hook_child
    try:
        m3refs.theme_refs(SPEC)
    finally:
        MP.hook_child = shared


EXPORTS = {'theme_bossa_noir': export}
