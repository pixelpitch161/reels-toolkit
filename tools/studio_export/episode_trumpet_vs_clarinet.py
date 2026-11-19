"""episode_trumpet_vs_clarinet: the kit episode ballfight_trumpet_vs_clarinet (#46, under hot_jazz) as a pair gate
(Template P, plan Task 57).

    python -m tools.studio_export episode_trumpet_vs_clarinet        writes studio/reference/episodes/trumpet_vs_clarinet/
    python -m tools.studio_export cache episodes/trumpet_vs_clarinet regenerates the cache and checks the pins

The kit's recording of the config (seed 8410) is the theme folder's pinned cache copy
(themes/hot_jazz/trumpet_vs_clarinet.bfrec, ruling M3-R44: pointed at, never copied; recording_in_cache keeps it out
of the committed folder if those bytes ever change); the fight file is studio/fights/trumpet_vs_clarinet.toml. The
config sets no pair rule; its own links (plan Task 57) are the two fighters' kinds meeting:
- the HIGH C beam against jumps: TRUMPET's low ability charges one beat with its aim locked on where the CLARINET
  will be (`ability/HIGH C`, `charge`), then fires the blast (`beam`), which the CLARINET's TWELFTH JUMP (`jump`)
  can leave behind or which lands (`hit/beam`);
- stun on the punches: CLARINET's SQUEAK (`ability/SQUEAK`, `stun`, `hit/stun`) stuns the TRUMPET, whose VALVE COMBO
  (`combo`, its pistons `punch`, a landed one `hit/piston`) starts no combo while stunned; the jump's zip line cuts
  (`hit/cut`). Only CLARINET stuns here, so every `stun` is the TRUMPET stunned.
HIGH C first comes over 36 s after FIGHT! in the plain and swapped fights (it waits for the low turn) and SQUEAK 14 s,
so the early_low groups (Template F's: early levels and LOW_HP 92, both corners) put both inside the window."""
from . import m3refs

EARLY_LOW = dict(LV_DMG=m3refs.LV, LOW_HP=92.0)       # m3refs.default_scenarios' early_low

SPEC = dict(
    config='ballfight_trumpet_vs_clarinet',
    links=('combo', 'punch', 'hit/piston', 'ability/HIGH C', 'charge', 'beam', 'hit/beam', 'jump', 'hit/cut',
           'ability/SQUEAK', 'stun', 'hit/stun'),
    scenarios=[dict(name='plain', seeds=[1, 2, 3], swapped=False),
               dict(name='swapped', seeds=[1, 2, 3], swapped=True),
               dict(name='early_low', seeds=[1, 2, 3], swapped=False, overrides=EARLY_LOW),
               dict(name='early_low_swapped', seeds=[1, 2, 3], swapped=True, overrides=EARLY_LOW)],
    weapon_hit={'TRUMPET': 'piston', 'CLARINET': 'cut'},   # the `how` of each weapon's hits (fight.py 970, 2390)
    recording_in_cache=True,                               # ruling M3-R44
)

EXPORTS = {'episode_trumpet_vs_clarinet': lambda: m3refs.episode_refs(SPEC)}
