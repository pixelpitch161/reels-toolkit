# MELLOPHONE fighter: design

Picked by Aedan on 2026-11-16: idea 1, "Slapback", from the move-set artifact
https://claude.ai/artifact/GW9nR9YtjLGPgU3EjPBsTJ. MARIMBA (merged the same morning) already throws mallets that
ricochet off the ropes, so SLAPBACK is built as an echo rather than a bounce (recommended to Aedan on a decision card;
if he picks the plain bounce instead, only the rope step of the weapon changes).

## Intent

A new from-scratch fighter: MELLOPHONE, the marching band's forward-bell horn, whose sound slaps off the stadium's
press box a beat late. A ranged fighter that plays the room: its waves come back off the ropes, so it hits from odd
angles. It must read as different from HORN (echo ghosts on its own path, glove), TRUMPET (piston punches, HIGH C),
SOUSAPHONE (marching drill), CORNET (plunger) and MARIMBA (ricocheting mallets).

Process rules (Aedan, 2026-11-16): once the design is approved, build until merged into `claude/project-thread-h102zt`;
never install; very lenient balance check; the model is made from real web photos, on Aedan's PC.

## Moves

| Slot | Name | Kind | What it does |
| --- | --- | --- | --- |
| Weapon | SLAPBACK | `slapback` (new) | Every `every` beats (2; haste divides it), not stunned, a wave (a wide arc, half-width `half_w`) leaves the bell at the enemy at `speed`. It hits the enemy it touches (`dmg` + `dmg_step` per echo generation, knock `knock`) and is gone. At a rope it stops; if it has echoes left, `delay` beats later a new wave leaves that rope point, re-aimed at the enemy, one generation louder, one echo fewer. The first wave carries `int(n_echo)` echoes (`n0` 1, `grow` 0.5 a landed SLAPBACK hit, up to `n_max` 3). |
| LV2 | HORNS UP | `horns_up` (new) | Three fast straight stabs at the enemy on 16ths (`n`, `speed`, `dmg`), no echoes; the last one stuns for `stun_beats`. |
| LV3 | PRESS BOX | `press_box` (new) | For `dur_beats` every echo leaves as two waves, `spread` degrees either side of the enemy. |
| LOW | STADIUM ECHO | `stadium_echo` (new) | One giant wave (`half_w` big). It passes through the enemy (one hit per leg) and echoes off every rope after `delay` beats, re-aimed and louder by `dmg_step`, until `beats` beats (2 bars) are up. |

Colour: band royal (40, 24, 244), dark (18, 10, 130), the royal blue of a band uniform. Plume rose and plum, shown on
the artifact, were too close to SNARE DRUM's proposed cherry and MARIMBA's jacaranda; oxblood read as SOUSAPHONE's
crimson in a test frame. Band royal is the colour furthest (CIELAB) from every colour on the roster (nearest: TUBA's
indigo, 43 apart). Added to RULES.md's colour table.

## Constraints

- New kinds in one new engine module (`bf-sim/src/kinds/slapback.rs`, as MARIMBA's `mallets.rs`), typed settings in
  `bf-core`; no existing kind changes, nothing draws from the generator, so every existing fight replays unchanged.
- The new-fighter layout set by CORNET and MARIMBA: entry in `bf-content/data/roster_new.json`, name in
  `fighters::NEW`, counts as `30 + NEW.len()`, sockets after every kit kind.
- Windows-only app: cloud runs the crate tests; bf-app type-checked with `--target x86_64-pc-windows-gnu`.
- Never edit `projects/`, `reelkit/`, `series/`.

## Look, sound, music

- Look: each wave as a bright arc of light on the canvas in its colour, wider and brighter each generation; a rope
  flash where a wave lands, with a growing "slap" mark until its echo leaves; HORNS UP's three thin stabs; PRESS BOX's
  far rope lit like a stadium wall; STADIUM ECHO's giant arc. Shards: tubing, valves, bell chips.
- Model: low-poly silver marching mellophone from Wikimedia photos (Yamaha YMP-204MS), made on the PC
  (`studio/tools/models/mellophone_model.py`, refs in `studio/reference/models/mellophone/`).
- Voice: the shared brass block, a horn's round tone pushed brighter by the forward bell (between HORN and TRUMPET).
- Part: each wave a short bright note, each echo the same note again quieter and darker (a real slapback), climbing
  the chord with the generation; HORNS UP three stabs; PRESS BOX a two-note fanfare; STADIUM ECHO a held note with
  echoes. Personality: home F, register about F3 to C6, invented drum-corps "mello soli" motifs. Roles melody, answer.
- Card, starter pairing and home theme, score order (brass), fit traits.

## Approaches considered

1 Slapback (picked), 2 Bell Front (cone ahead of travel), 3 The Pah (afterbeat bells): see the artifact. Within 1, the
echo (delayed re-aimed wave from the rope) rather than a ricochet, to keep it apart from MARIMBA.

## Left to the builder

Exact numbers (start near MARIMBA's damage per bar), event and record key names, look details.
