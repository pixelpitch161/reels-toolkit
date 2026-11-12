"""fighter_bassoon: BASSOON's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the template's default config (m3refs.py's
docstring). Its part does not branch on its role, so only its kit role is exported.

Two departures from the template:

1. The handicap is (0.41, 0.6), not (1.0, 1.0) (ruling M3-R34). At (1.0, 1.0) BASSOON won 99.6 % of the 2,000
   statistics fights with a median K.O. at 23 s, so no natural `right` fight existed and no winning fight lasted
   to 44 s, which the checks need of the `left` fight. Both sides are turned down so the fights are long enough
   (median K.O. 49 s), and BASSOON a little more so FLUTE wins a real share (BASSOON 77.8 % of the 2,000). The
   kit's own pairs turn BASSOON down too (0.85 against OBOE, 0.70 against BASS CLARINET).
2. NEEDS A SHARED FIX (reported; remove `rect_keys` and `refs` below once the framework has it). BASSOON's record
   keys HOOK and HOP are rectangular float arrays in every fight (two rows of three a frame; HOOK is NaN where no
   hook is out), but the kit's `story.REC_KEYS` does not list them, so `recording.write_bfrec` would write them as
   JSON, where NaN has no place (recording-format.md 2: every float in JSON is finite), and the export stops at the
   first recording. Until the framework names the kit kinds' rectangular keys itself, this module adds the two keys
   to `story.REC_KEYS` in the export's own process, before the recordings are written: they are then `.npy`
   members (as `K` is, NaN and all), which is also what the Studio's engine records (`Frames::push_f64`). The
   parts' processes are not touched: they read the recording back and compare it with the kit's run value for
   value. Nothing under projects/ is edited.

    python -m tools.studio_export fighter_bassoon          writes studio/reference/fighters/bassoon/ and its cache
    python -m tools.studio_export cache fighters/bassoon   regenerates the cache and checks the pins"""
from . import common, m3refs

SPEC = dict(
    name='BASSOON',
    opponent='FLUTE',
    fights=dict(handicap=(0.41, 0.6)),           # the reference handicap (departure 1)
)

RECT_KEYS = ('HOOK', 'HOP')                      # departure 2: rectangular in every fight, absent from REC_KEYS


def rect_keys():
    """departure 2: this process writes HOOK and HOP as .npy members"""
    _, story = common.kit()
    story.REC_KEYS = tuple(story.REC_KEYS) + tuple(k for k in RECT_KEYS if k not in story.REC_KEYS)


def refs():
    rect_keys()
    return m3refs.fighter_refs(SPEC)


EXPORTS = {'fighter_bassoon': refs}
