# SOPRANO SAX fighter: design

Approved by Aedan on 2026-11-16 (picked idea C, "Hornet Buzz", from the move-set artifact
https://claude.ai/artifact/YMLdKVdHV6zV4FoYe7nrC1).

## Intent

A new from-scratch fighter for BALLFIGHT Studio: SOPRANO SAX, the smallest, highest, buzziest saxophone. Its nasal
buzz becomes a swarm of hornets that orbit the ball and hunt the enemy. It must read clearly apart from ALTO SAX
(spotlight), TENOR SAX (crowd), BARI SAX (wrecking ball) and the other reeds.

Process rules (Aedan, 2026-11-16): once the design is approved, build until it is merged into the build branch
`claude/project-thread-h102zt`; never install; very lenient balance check; the model is made from real web photos, on
Aedan's PC (photos already in `F:\!Ai Stuff\InstrumentBattleMachine\fighter-refs\soprano-sax\`, guide in
`/mnt/project-files/ballfight-studio/fighter-refs/soprano-sax-reference.md`).

## Moves

| Slot | Name | Kind | What it does |
| --- | --- | --- | --- |
| Weapon | HORNET SWARM | `swarm` (new) | Hornets orbit the ball (`start`, up to `max`). Every `every` beats, when not stunned, hornets peel off (one, plus one for every two grown, never more than are orbiting) and chase the enemy, steering toward it, for up to `life` beats. A chaser that touches the enemy stings (`dmg`, a small knock) and flies home; a sting adds a hornet to the swarm. A blocked or timed-out chaser flies home with no growth. |
| LV2 | KICK THE HIVE | `hive` (new) | Every orbiting hornet (at least 3) charges the enemy at once for `dur_beats`, each stinging once (`dmg`). |
| LV3 | BUZZ OFF | `buzz` (new) | For `dur_beats` the swarm wraps tight around the ball: hits on it are blocked (the shared shield timer), and an enemy that touches it is stung (`dmg`, `knock`, at most twice a beat). |
| LOW | QUEEN | `queen` (new) | A big queen hornet chases the enemy for `dur_beats`, stinging hard (`dmg`, `knock`) at most once every `gap` beats, bouncing off after each sting. |

Colour: signal green (0, 232, 16), dark (0, 104, 12). Mint (the brainstorm's pick) is TROMBONE's; signal green is the
colour furthest from every taken and reserved colour (nearest: ELECTRIC). The hornets are drawn yellow and black.

## Constraints

- New kinds are one new module (`bf-sim/src/kinds/swarm.rs`, with typed settings in `bf-core/src/roster/kinds/swarm.rs`
  and `bf-core/src/roster/abilities/{hive,buzz,queen}.rs`); no shared engine code changes behaviour, so no existing
  fight changes (the replay test proves it). No draws from the fight's generator.
- Copied abilities (EUPHONIUM's IDENTITY CRISIS) work on a ball without a swarm: KICK THE HIVE sends 3 hornets.
- Roster entry in `bf-content/data/roster_new.json`. Windows-only app: the cloud runs the crate tests that build on
  Linux; bf-app is type-checked for windows-gnu. Never edit `projects/`, `reelkit/`, `series/`.

## Look, sound, music

- Look: orbiting hornets (yellow and black, wings blurring), chasers with a dotted buzz trail, a sting spark, the
  hive's charge, the tight armour ring of BUZZ OFF, the big queen with a crown stripe. Shards in its colour.
- Model: a low-poly straight soprano saxophone from the photo guide (thin gold cone, black mouthpiece, six pearl keys,
  small flat bell), made on Aedan's PC.
- Voice: a bright, nasal, buzzy reed. Part: a darting melody of quick runs, a short trill per launch, a stab on each
  sting, a tremolo buzz under BUZZ OFF.
- Personality: high register (about Ab3 to E6 written, played Bb3 to C6), quick chromatic runs and trills, invented
  motifs. Roles: melody and answer.
- Card, starter pairing and home theme, score order (reeds, beside ALTO SAX), fit traits.

## Approaches considered

A Snake Charmer, B Needle Note, C Hornet Buzz (picked): see the artifact. Within C, BUZZ OFF reuses the shared shield
timer for its block (a new block would touch every kind) and adds its contact sting in the swarm module.

## Left to the builder

Exact numbers (start near CORNET's damage per bar), event and record key names, the look's details.
