"""theme_dark_disco: the dark disco's milestone 3 references (Template T, Task 31).

studio/reference/themes/dark_disco/ (python -m tools.studio_export theme_dark_disco):

    theme.json      the reference config (ballfight_altosax_vs_tenorsax, #51): the kit's own recording of it and the bed
                    alone under two kit base Sounds: the 16-bit master's SHA-256 and the buses
    voicings.json   Bed.voicing (the stab / pad voicing: the chord's tones from A3 up, at most four) for the 12 roots x
                    13 qualities; Bed.root is T.root_above(ch, 33), a number and not a voicing
    bed.json        the bed alone's RMS per bus; a second tempo and a second key (recordings cached); `hit` called
                    directly at three times with three gains (vectors cached). The class has no `boom` (ruling M3-R2)

The mood has no options (it reads nothing of the config but the music) and plays no fighter's voice.

`hit`: the dark disco's `hit(mix, t, ch, g=1.0)` has no `ring`, but m3parts.HOOK_CALLS passes `ring=` to every `hit`,
so m3parts.hook_child raises TypeError on it. Until the shared child passes only the arguments the method takes, this
export runs the shared child through `_hook_child`, which first gives the kit's `Bed.hit` (run-time only, in that child
process) a wrapper that takes `ring` and leaves it out of the call. Nothing the bed plays changes; the call's arguments
are recorded as the shared child records them (with the unused `ring`)."""
from . import m3refs

SPEC = dict(
    name='dark_disco',
    voicings=('voicing',),
    configs=('ballfight_altosax_vs_tenorsax',),
)


def _hook_child(cfg_folder, hooks, out_path):
    """m3parts.hook_child, with the kit bed's `hit(mix, t, ch, g)` also taking the `ring` the shared calls pass"""
    import importlib
    from . import common, m3parts as MP
    common.kit()
    mood = importlib.import_module('ballfight.kit.moods.dark_disco')
    kit_hit = mood.Bed.hit

    def hit(self, mix, t, ch, g=1.0, ring=None):
        return kit_hit(self, mix, t, ch, g)
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


EXPORTS = {'theme_dark_disco': export}
