# CORNET fighter: design

Approved by Aedan on 2026-11-16 (picked idea A, "Plunger & Derby", from the move-set artifact
https://claude.ai/artifact/XcyqEjXpkRQR9yKhY37kzQ).

## Intent

A new from-scratch fighter for BALLFIGHT Studio: CORNET, a 1920s New Orleans street brawler that fights with the mutes
cornet players used for the talking "wah" sound. Grabby and tricky: it reels the enemy in and spoils their attacks.
It must read as clearly different from TRUMPET (valve punches, FANFARE, DOUBLE TONGUE, HIGH C).

Process rules (Aedan, 2026-11-16): once the design is approved, build until it is merged into the build branch
`claude/project-thread-h102zt`; never install; very lenient balance check (the seed finder handicaps pairs); the model is
made from real web photos of a cornet, on Aedan's PC (image sites are blocked in the cloud).

## Moves

| Slot | Name | Kind | What it does |
| --- | --- | --- | --- |
| Weapon | PLUNGER MUTE | `plunger` (new) | Every `every` beats, if no cup is out, a rubber plunger cup on a cord flies at the enemy (aimed with a little lead). If it reaches them within the cord's reach it sticks: a small hit, then on each beat while stuck a "wah" hit plus a pull toward the cornet. It pops off after `hold` beats, or when the cord is stretched past its reach, and reels back. A stick grows the cord (`grow`, up to `max_reach`) and the grip (`hold_grow` beats, up to `hold_max`). A shield blocks the stick. |
| LV2 | WA-WA | `shield` (reused) | A wobbling wah ring around the cornet for `dur_beats`: hits are blocked ("bent away"). Same engine kind as BRASS WALL, its own look. |
| LV3 | DERBY HAT | `derby` (new) | A bowler hat is lobbed at where the enemy is heading and lands after `tele` beats. An enemy within `radius` is muffled for `dur_beats`: it keeps moving but starts no attack (the engine's "stunned" check), plus a small hit. |
| LOW | STOP-TIME | `stoptime` (new) | Four beats of stop-time. On beats 1 to 3 the enemy freezes for part of the beat (stun) and the cornet dashes at it; a dash that meets the enemy hits once. On beat 4 the band comes back in: a burst ring (the shared wave) with the big hit. |

Colour: rose brass (232, 120, 140), dark (138, 52, 70). Added to the colour table in `docs/ballfight/RULES.md`.

## Constraints

- New kinds are new modules (`bf-core/src/roster/kinds|abilities/`, `bf-sim/src/kinds|abilities/`); existing kinds are not
  changed, so no existing fight changes. The only shared-engine change is one new ball timer, `muffle`, which the
  existing `stunned()` also reads; it is 0 for every other fighter, so no existing fight changes (the replay test proves it).
- No Python reference: the fighter is defined in Rust only (roster entry added to `bf-content/data/roster.json` by hand,
  with a note); no reference folder or harness suite, a short test of its fights instead (as the quick ports have).
- Windows-only app: cloud runs the crate tests that build on Linux; bf-app is type-checked for windows-gnu.
- Never edit `projects/`, `reelkit/`, `series/`.

## Look, sound, music

- Look: the cord (dashed, sagging when slack, taut when stuck), the red rubber cup with a wooden handle, "WA"/"WAH" rings
  on each wah, the wobbling WA-WA ring, the spinning derby in flight and sat on the enemy, STOP-TIME's freeze frame and
  dash streaks, the burst ring. Shards in its colour.
- Model: a low-poly cornet from real photos (compact shepherd's-crook wrap, three piston valves, shorter bell than a
  trumpet), made on Aedan's PC.
- Voice: a mellow, darker-than-trumpet brass voice; a plunger "wah" (filter sweep) on the weapon.
- Personality: home key, middle register (about Bb3 to D5), lyrical rhythms, a turn ornament, invented motifs (2 to 4)
  in a hot-jazz, stop-time feel. Roles: melody and answer.
- Card, starter pairing and home theme, score order (brass, beside TRUMPET), fit traits for theme recommendations.

## Approaches considered

A Plunger & Derby (picked), B Carnival Virtuoso, C Bugle Corps: see the artifact. Within A, WA-WA reuses the shield kind
rather than a new deflect kind: projectiles differ per kind, so a real deflection would touch every kind; a block reads
the same on screen.

## Left to the builder

Exact numbers (start from TRUMPET's and BARI SAX's damage per bar), the event and record key names, the look's details.
