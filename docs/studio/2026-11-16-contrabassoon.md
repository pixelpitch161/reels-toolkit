# CONTRABASSOON: design and plan

> **For agentic workers:** built natively in one session (Aedan: build straight through after approval, few tokens,
> lenient balance check). Steps use checkbox syntax.

**Goal:** a new fighter, CONTRABASSOON, made from scratch: Set C "THE PLUMBING" of the concepts page
(https://claude.ai/artifact/Dn2BBwf5xPf9DYjZLu33M9), ball colour oxblood (150, 34, 44). Aedan picked "C, Oxblood"
on Nov 16, 2026.

**Architecture:** one new weapon kind `plumbing` (roster settings in `bf-core/src/roster/kinds/plumbing.rs`, engine in
`bf-sim/src/kinds/plumbing.rs`) and three new ability kinds whose engine code lives in the weapon's module, as VIOLA's
do. Existing kinds are not changed, so no existing fight changes. The look rebuilds the pipes from the fight's events,
as HARP's strings do. Rust is the source of truth: the entry is added to `bf-content/data/roster.json` by hand.

## Approaches (from the concepts page)

- A THE BEAST (growl pulls the enemy in), B THE RATTLE (buzz marks tick damage), C THE PLUMBING (pipes as walls).
  Aedan picked C. It is the only one that changes the ring itself, so it plays unlike BASSOON (hops and stomps) and
  CONTRABASS CLARINET (foghorn shove).

## The moves

- **Weapon U-BEND.** Every `every` beats (4: once a bar; a haste divides it) it lays a U-shaped piece of tube
  (legs `leg` long, `gap` apart, `thick` thick) ahead of the enemy (`lead` units along its heading), its bend toward
  the enemy. A spot is refused if the pipe would touch either ball or another pipe, or leave the ring; other spots
  are tried, and if none fits no pipe is laid that beat. At most `max_pipes` (3); the oldest goes. Both balls bounce
  off pipes (the heading reflects, the speed is unchanged). The enemy takes `dmg` per pipe bounce (each pipe hits at
  most once per `cool` s), growing by `dmg_grow` per landed hit up to `dmg_max`, and gets speed boost `knock`. Its
  own bounce gives it speed boost `own_boost`.
- **LV2 WATER KEY** (`water_key`, every 3 bars): for one beat every pipe end and its own ball spray; an enemy within
  `reach` of a spray takes `dmg` once per spray and is knocked away.
- **LV3 HOT PIPES** (`hot_pipes`, every 6 bars): for `dur_beats` 8 the pipes glow and hit `mult` (2.0) times harder.
- **LOW FIVE AND A HALF METRES** (`pipe_box`): a square of tube of half-side `half` round the enemy for `beats`
  beats. The enemy bounces inside it and takes `dmg` per wall hit (cool `cool`); its owner passes through it.

Ball skips: a ball that cannot be knocked (walking, swan, strut, dive), is airborne, rewinding, smashed, swallowed or
held is not bounced by pipes (it can still be hurt where `damage` allows).

## Look, sound, card

- Model from real photos (Heckel and VSL reference pages): four parallel dark maple tubes, three silver U-bends, a
  thin S crook with the reed on top, a metal bell pointing down, silver keys. Generated as `data/models/CONTRABASSOON.json`.
- Look: the ball with that model, the pipes in the ring (oxblood-dark wood with silver bends; red-orange glow while
  HOT PIPES lasts; spray droplets on WATER KEY; the box on the low move).
- Voice: BASSOON's double-reed voice an octave down, darker. Part: like BASSOON's (a note on each landed hit, a low
  clank on pipes). Personality: home B flat 0 area, invented motifs (a riff stepping down in fourths, a pipe-echo
  figure). Card colour "Oxblood"; `docs/ballfight/RULES.md` colour table gets the row.

## Tasks

- [ ] 1. Roster kinds (`bf-core`): `plumbing` weapon, `water_key`, `hot_pipes`, `pipe_box` abilities, wired into
      `roster/mod.rs` and `roster/abilities.rs`. Test: the roster parses the new entry.
- [ ] 2. Engine (`bf-sim`): `kinds/plumbing.rs` (state, beat, step, three starts), wired into `fight.rs`,
      `schedule.rs`, `abilities/mod.rs`. Tests: unit tests (a pipe is laid, a bounce reflects and hurts, the box holds
      the enemy) and `tests/contrabassoon_quick.rs` (fights against 4 fighters end; a K.O. in a normal share).
- [ ] 3. Content: folder `fighters/contrabassoon/` (look, voice, sound, card, personality, model), registry `PORTED`,
      cards section, starter pairing and home theme, RULES.md colour.
- [ ] 4. Balance check (lenient): seed search on a pairing finds fits; one render and a few frames looked at.
- [ ] 5. `cargo fmt`, clippy, `cargo test --workspace`; merge into `claude/project-thread-h102zt` and push.
