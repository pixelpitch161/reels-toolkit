# CONTRABASS CLARINET: the first fighter made in the Studio (design and plan)

Owner's pick, Nov 16, 2026 (project thread "Add Contrabass Clarinet fighter", concepts page
https://claude.ai/artifact/4mjPVp9jwnfbyqQtfCWmhe): **move set B, THE FOGHORN**, the card name **CONTRABASS
CLARINET**, colour **abyss teal (24, 124, 132)**. Built straight through to the merge into the build branch; no
install unless asked; balance is the seed finder's job (fights auto-balance), so the proof is small.

## Why and what

The 31st fighter and the first with no Python kit module or past episode behind it: its numbers, look, voice, part
and motifs are invented here. It must stand apart from its neighbours CONTRABASS (steel blue, walking bass) and
BASS CLARINET (slate blue, whirls round a planted endpin): it never orbits or plants a peg, its body is a silver
folded "paperclip" (not a straight black one), and its colour is a dark teal far from both blues.

## The move set

| Slot | Name | Kind | What it does |
|---|---|---|---|
| Weapon | FOGHORN | new kind `foghorn` | Every bar (`every` 4 beats; a haste divides it) the bell, which always turns toward the enemy, blows a wide slow cone (`spread` degrees each side, `reach` from the surface, live `dur` s). The first touch of a blast hits lightly (`dmg`) and shoves the enemy along the blast; while the blast lasts it keeps pushing. If the shoved enemy hits a rope within `slam_t` s it takes the wall slam (`dmg_wall`). A landed wall slam grows the reach (`grow`, up to `max_reach`). A shield blocks the blast. |
| LV2 | FOG BANK | `shield` (reused) | It sinks into a fog bank for `dur_beats`: attacks pass through. Drawn as fog, not a shell. |
| LV3 | MOTORBOAT | `haste` (reused) | The low notes rattle "putt-putt": double speed for `dur_beats`, so a blast every 2 beats; a rattling wake. |
| LOW | SHIP'S HORN | `beam` (reused) | Below 25 HP: one enormous horn blast across the ring (TRUMPET's HIGH C machinery, drawn as a fog-light beam). |

Reusing `shield`, `haste` and `beam` follows `docs/ballfight/RULES.md` ("existing ability kinds are reused wherever
they fit"); only the weapon is new, so no existing fight changes.

## Where it lives (approach)

Approaches considered: (1) add the entry to `roster.json` by hand; (2) a second roster file for fighters made in the
Studio; (3) build it as a Python kit module first and export it. (3) is ruled out: the kit is frozen and `projects/`
is never edited. (1) breaks the export's contract ("do not edit by hand", its manifest hash and every test that
counts 30 kit entries). **Chosen: (2)**, `studio/crates/bf-content/data/studio_roster.json` (same format 1), which
`registry::roster()` appends after the exported roster. Everything else follows HANDOFF.md "How to add a fighter".

- `bf-core/src/roster/kinds/foghorn.rs`: `FoghornCfg`, parse/check/ball_start/main16, wired into `WeaponCfg`.
- `bf-sim/src/kinds/foghorn.rs`: setup (bell faces the enemy), beat (the blast), step (aim, cone test, push, wall
  slam); its state on `Fight`; called from `schedule.rs` beside the swallow's sockets. Events `horn` (blast start:
  aim, reach, spread, dur), hits `foghorn` and `wall`, `block`.
- `bf-content/src/fighters/contrabass_clarinet/`: `mod.rs`, `look.rs` (the cone of sound and its rings, the fog bank,
  the motorboat wake, the ship's-horn beam, a fog-and-rings title pattern), `voice.rs` (BASS CLARINET's reed synth an
  octave lower with a strong growl below D2: the reed "flap"), `sound.rs` (horn blasts, wall-slam booms, the fog
  swell, putt-putt 16ths, the big horn), `card.rs`, `state.rs`, `personality.rs` (invented motifs: the two-note
  foghorn call, the putt-putt, a harbour-at-night solo).
- Model `data/models/CONTRABASS CLARINET.json`: a generated low-poly silver paperclip body with gold keys and an
  up-turned bell (generator `tools/studio_export/new_models/contrabass_clarinet.py`).
- Offered: `PORTED`, `fighters::entry`, the colour table in `RULES.md`, a cards section (woodwinds), a starter episode
  row CONTRABASS CLARINET vs FLUTE (the lowest against the highest) in `industrial`.

## Numbers (first guess; the seed finder balances)

Weapon: every 4, spread 28, base 1.6, grow 0.12, max_reach 2.6, dur 0.55, push 1.8 (speed boost), dmg 1.5,
dmg_wall 2.6, slam_t 0.5, steer 18, aim_rate 5.0 (first guess 3.0 / 6.0 won 30 of 32 quick fights; these win 18 of 32,
K.O. at 49 s on average). FOG BANK: dur_beats 6, every_bars 6. MOTORBOAT: mult 2.0, dur_beats 8,
every_bars 6. SHIP'S HORN: charge_beats 1, lead 1.0, len 7.0, width 1.3, dmg 11.0, knock 1.8, hold 0.45.
Register (26, 58): D1 to Bb3.

## Plan

1. Roster: kind module in bf-core, `studio_roster.json`, registry appends it (tests: the entry loads, 31 fighters).
2. Sim: kind module, sockets, a unit test (a blast shoves and a wall slam lands).
3. Content folder: model, voice, sound, look, card, personality, state; dispatch, PORTED, starter, colour table.
4. Proof (small, per HANDOFF step 4): `bf-sim/tests/contrabass_clarinet_quick.rs` against 4 fighters; a search, a
   render and a few frames looked at; `cargo fmt`, `clippy -D warnings`, `cargo test --workspace`.
5. Merge into `claude/project-thread-h102zt`, push. No version bump or install unless the owner asks.
