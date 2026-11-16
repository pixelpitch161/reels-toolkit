# FLUGELHORN: a fighter made in the Studio (design and plan)

Owner's pick, Nov 16, 2026 (project thread "Add Flugelhorn fighter", concepts page
https://claude.ai/artifact/9nmYWqaiKBxwp9oRdvDaeF): **move set A, THE WING HORN**. Colour: the page's default was
dusty rose, but CORNET was merged meanwhile in rose brass (232, 120, 140), nearly the same, so the fighter is built in
the page's other option **sage (150, 176, 140)** (told to the owner in the thread; espresso is the fallback). Built
straight through to the merge into the build branch; no install unless asked; the balance check is lenient (the
seed finder handicaps pairs).

## Why and what

A from-scratch fighter (no Python kit module, no episode). "Flügel" is German for wing: the horn signalled the
army's flanks. It fights with wings, so it plays nothing like TRUMPET (piston punches, beam), HORN (round echo
blasts), TROMBONE (slide blade), TUBA (swallow), EUPHONIUM (mortar), SOUSAPHONE (drill) or CORNET (plunger).

## The move set

| Slot | Name | Kind | What it does |
|---|---|---|---|
| Weapon | WINGBEAT | new kind `wingbeat` | The ball's aim turns toward the enemy (`aim_rate`). Every `every` beats (a haste divides it) two wings sweep out and clap shut in front: an enemy within `reach` of the surfaces (`base`, `grow` per landed clap, up to `max_reach`) and `spread` degrees of the aim is hit (`dmg`, how `wingbeat`) and blown back along the aim (knock `knock`). Event `clap` (aim, reach, half-angle). |
| LV2 | FLANKING CALL | new kind `flank` | For `dur_beats` the ball swoops round to the enemy's side (steered toward a point `side` units beside it, speed boost `boost`); when it gets within `reach` of the surfaces it claps from there (`dmg`, how `flank`, knock `knock` sideways) and the swoop ends. Events `flank` (start), the hit. |
| LV3 | FEATHER SHOWER | `arc` (reused) | A slow fan of feathers (PIANO's GLISSANDO machinery, drawn as feathers). |
| LOW | TAKE WING | new kind `take_wing` | Below 25 HP the wings spread across the ring for `tele` beats, then one giant clap hits anything within `radius` (`dmg`, how `take_wing`, knock `knock`), and the ball glides at `mult` speed for `dur_beats` (a haste). Events `take_wing` (start), `wing_clap` (the clap). |

The new ability kinds run in the weapon's module (`bf-sim/src/kinds/wingbeat.rs`), as CORNET's and MARIMBA's do, and
work for either ball (IDENTITY CRISIS can copy them).

## Where it lives

Approaches considered: the same three as CONTRABASS CLARINET's design (hand-edit `roster.json`, a Python kit module,
or `roster_new.json`). Only `roster_new.json` fits (the kit is frozen; the export is never edited by hand).

- `bf-core`: `roster/kinds/wingbeat.rs`, `roster/abilities/{flank,take_wing}.rs`, wired like `foghorn` and `derby`.
- `bf-sim`: `kinds/wingbeat.rs` (setup, beat, step, the two abilities), its state on `Fight`, schedule sockets,
  the ability dispatch; `tests/flugelhorn_quick.rs`.
- `bf-content/src/fighters/flugelhorn/`: model (from photos: Wikimedia Commons `Yamaha_Flugelhorn_YFH-8310Z.jpg`;
  generator `studio/tools/models/flugelhorn_model.py`), look (the wings and their clap, the swoop's trail, the
  feathers, the giant wingspan), voice (the shared brass block, darker and softer than CORNET), part, card,
  personality (invented motifs: the rising-sixth flank call, the feather flutter, a gliding solo), state.
- Offered: `PORTED`, `fighters::NEW` and `entry`, cards (brass), fit, personality, the colour table in RULES.md.
  No home row of its own: like CORNET it borrows FLUTE's home config.

## Numbers (first guess; the seed finder balances)

Weapon: every 2, spread 60, base 0.9, grow 0.12, max_reach 1.8, dmg 3.2 (2.0 won 0 of 32 quick fights, 4.5 won 30; 3.2 wins 11, 25 K.O.s, mean 51 s), knock 1.3, aim_rate 6.0.
FLANKING CALL: dur_beats 3, side 1.2, boost 1.8, reach 0.5, dmg 3.0, knock 1.4, every_bars 4.
FEATHER SHOWER: arc n 9, sweep 120, beats 2, speed 3.2, dmg 0.7, every_bars 5.
TAKE WING: tele 1, radius 3.2, dmg 9.0, knock 1.8, mult 2.0, dur_beats 8.
Register (52, 77): E3 to F5.
