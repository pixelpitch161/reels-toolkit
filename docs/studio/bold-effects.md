# Bold effects: how a fighter's upgraded moves are built

The owner picked upgraded attack visuals for every fighter (the round 2 review of 2026-11-24): twelve shared
upgrades (S1 to S12), new looks for all four moves of each fighter, and side banners for special-move names
(N4). This page is the guide for building one fighter's four moves. CORNET is the worked example.

## The one rule that matters most

Everything is **opt-in and look-only**.

- A fight file asks for the upgrades with `picture.effects = "bold"`. `PictureCtx::bold` is then `true`.
  Without it every frame must stay bit-for-bit what it was (the series' videos and hundreds of reference
  pictures depend on it). So: never change what a look draws when `p.bold` is false. Every change sits behind
  `if p.bold { ... }`.
- Never touch the fight: nothing in `bf-sim`, no damage, timing, sound or balance. The picture reads the
  finished recording and draws it differently, that is all.
- Engine 1 style: flat pixel shapes, dark outlines, canvas light. No 3D models, no gradients, no blur.
- Never edit `projects/`, `reelkit/` or `series/` at the repository root.

## Where the code goes

For the fighter in `crates/bf-content/src/fighters/<slug>/`:

1. Create `bold.rs` beside `look.rs`. It is a child module of the look, so it can use the look's private
   fields and helpers through `use super::{...}`.
2. In `look.rs` add, near the top:

   ```rust
   /// The same moves with the bold effects.
   #[path = "bold.rs"]
   mod bold;
   ```

3. In each hook of `look.rs` whose drawing changes, hand over first thing:

   ```rust
   fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
       if p.bold {
           return self.bold_canvas(p, ic, c);
       }
       // ... the classic code, untouched
   }
   ```

   and write `pub(super) fn bold_canvas(...)` in an `impl <Slug>Look` block in `bold.rs`. A bold hook usually
   re-does what the classic hook draws, but bolder. When only a small part changes, the bold function may call
   small private helpers of `look.rs` (make none of them behave differently). A hook the look does not have
   yet (often `after_grade`, `bold_status`, `bold_hit_size`) is added to the `impl Look for ...` block and
   does its work only when `p.bold`.
4. Touch only that fighter's folder. If you need a helper the toolkit lacks, write it in your `bold.rs` and
   say so in your report; do not edit `bf-picture` or another fighter's folder.

Read `fighters/cornet/bold.rs` and the `p.bold` lines of `fighters/cornet/look.rs` before you start.

## What the ring already does for every fighter (do not draw these again)

All in `bf-picture/src/bold/shared.rs`, automatic when bold:

| Code | What happens by itself |
| --- | --- |
| S4 | Every `hit` event pops in one of three sizes by its damage (tap under 1.6, heavy from 6): star, ring and chips, and a heavy hit shakes the camera and squashes the victim. Change a hit's size or silence it with `Look::bold_hit_size` (return `None` for no pop, for example for a tick of a long grind). |
| S6 | While a fighter is hasted (the run's `HA` array): afterimages, a circling note and a draining ring. |
| S7 | A low-HP move: the ring dims round its fighter and the move's name shows big on its banner. |
| S8 | Floor marks (`fx.mk`) live at most two beats and only the newest six show. |
| S9 | A shield (the run's `SH` array): a ring that thins as it runs down and a white flare on a `block`. `p.bold_shield_left(k, t)`, `p.bold_last_block(k, t)` give the numbers for a fighter's own shield art. |
| S10 | Ropes bow deeper and a spark runs along them. |
| S11 | A level-up: gold ring and up-arrow on the ball. |
| S12 | Slow (the ball's `st.slow`) is marked by itself. Stun, Pin, Silence and Sour are reported by the look that causes them, through `Look::bold_status` (who, kind, share of time left); the ring draws the marks. |
| N4 | The special-move names on side ribbons. |

## The toolkit for a fighter's own moves (`bf_picture::bold`)

- `bold::thick_line(fr, cam, &world_pts, col, width)`, `bold::outlined_poly(fr, cam, &world_pts, col)`,
  `bold::blob(fr, cam, world_pt, r, col)`: S2, readable weapons in the painter pass (`draw_item`). Dark
  outline, body, light edge.
- `bold::hold_time(p, who, &["how"], t)`: S1. Read a jabbing or swinging weapon's pose at this time instead of
  `t` and it holds where it touched for three frames after its hit.
- `bold::until(p, who, &["kind" or "hit:how"], t, window)`: S3. The next event within `window` seconds, with
  how far the wind-up is (0 to 1). Use it for a pull-back, a glint (`px::glint`) and a floor mark where the
  move will land (`floor::target`, `floor::lane`). `bold::since(...)` is the same looking back.
- `bold::floor` (canvas light, in `canvas` / `canvas_wave` hooks, `ic` and the `CanvasCtx`): `ring`, `disc`,
  `shade`, `lane`, `dashes`, `target`, and S5: `wave(ic, c, Family::X, w, t, tint, k)` for one of the ring's
  own shock waves and `band(ic, c, Family::X, cx, cz, r, fade, t, tint, k)` for a ring you place yourself.
  Families: `Brass` (stacked gold bands), `Strings` (wobbles like a plucked string), `Air` (soft puffs),
  `Quake` (lifted tiles), `Noise` (jagged). Tints are canvas light: `bold::light(colour)`.
- `bold::px` (crisp pixel shapes on the finished picture, in `after_grade` with the `Pic`, at screen
  positions from `bold::at(cam, x, z, y)` or the `DrawnBall`s): `disc`, `ring`, `arc`, `line`, `polyline`,
  `poly`, `star`, `cross`, `arrow`, `glint`, `brackets`, `wash` (drain the colour), `dim_outside`, and the
  outlined `ink_*` versions of each. Colours are 0 to 255.
- Colours and easing: `bold::INK`, `WHITE`, `GOLD`, `pale`, `deep`, `light`, `ease_out`, `ease_in`,
  `bold::px_per_unit`.
- Small things several fighters needed: `bold::thin_line` (a string or wire too fine for `thick_line`),
  `bold::matte_blob` (a blob without the highlight), `bold::events_in(p, who, kinds, t0, t1)` (every event in
  a stretch, for volleys and grinds), `bold::how(e)` / `bold::text(e, i)` (an event's text without a panic on
  a short event), `px::chips` (splinters flying from a point), `px::note` (a little eighth note),
  `px::speed_lines` (streaks behind a ball sent flying), `px::rnd(seed, n)` (the dice: always the same for
  the same numbers).
- The kinds of `since`, `until` and `events_in` may be a plain event kind, `"hit:how"`, or `"ability:kind"` /
  `"ability:NAME"` for one ability.

The files are short and documented; read the signatures there rather than guessing.

## Reading the fight

- Events: `p.rec.events` (time-sorted; `p.ev_t` holds the times). Python indices: `e.f(i)`, `e.i(i)`, `e.s(i)`
  with `e[0]` the time, `e[1]` the kind, `e[2]` the fighter (`e.who()`). A `hit` is
  `[t, 'hit', attacker, victim, damage, x, z, how]`; an `ability` is `[t, 'ability', who, name, kind, first]`.
- What each weapon and ability writes is documented at the top of its file in `bf-sim/src/kinds/` and
  `bf-sim/src/abilities/` (find the fighter's kinds in `bf-content/data/roster.json` or `roster_new.json`).
- Per-frame values: the ball states `c.fs[k]` / `f.fs[k]` and the run's arrays (`p.run.f(key, i, k)`).
- The existing look already finds everything the classic drawing needs. Reuse how it reads the fight.

## Looking at your work

Build the dev binary (seconds after the first time) and draw contact sheets:

```
cd studio
cargo build -p bf-cli 2>&1 | grep -E "^(warning|error)" -A12
./target/debug/ballfight moves fights/dev/bold_<slug>.toml --out <new folder> --fighter 0 --zoom
```

- `fights/dev/bold_<slug>.toml`: the fighter against the target dummy (it never attacks), bold.
- `fights/dev/bold_<slug>_low.toml`: against a real fighter with the odds against it, so its low-HP move
  fires.
- `moves` draws, for the first time each kind of event happens, frames a little before, at and after it.
  `--kinds ability:NAME,hit:how,...` picks kinds (prefix match, as the labels in `moments.json` name them),
  `--nth N` takes the Nth time instead of the first (the first use is often behind the FIGHT! card),
  `--offsets=-0.2,0.04,0.3,0.8` sets the frames, `--at 12.5,13 --only-at` draws given times, `--zoom` shows the
  ring at twice the size (leave it out to see the whole frame with the banners), `--effects classic` draws
  the old look for comparison. `--out` must be a folder that does not exist yet.
- `--list` (no `--out`) draws nothing and prints every kind of event with how often it happens and its first
  times: start there. An event is filed under the fighter in its element 2, which for some is the victim
  (`slow`, `parry`, `muffle`), so list without `--fighter` when one seems missing.
- For a look that needs something the two dev fights lack (a projectile to parry, a shield to hit), copy a dev
  fight file to your scratch folder with another opponent and use that; do not add fight files to the
  repository.
- Under the low-HP spotlight the ring is dimmed after the painter pass, so white drawn in `draw_item` or
  `over_ring` comes out grey there: the brightest marks of a low-HP move belong in `after_grade`.
- The camera looks down on the ring from about 60 degrees, so a flat thing tilted steeply towards it (a
  flipped plate, a lid, a card) is nearly edge-on and vanishes: lay such shapes flat on the floor or stand
  them upright facing the camera.
- Only still frames: do not run tests, `ballfight render`, `check` or `bench` here. Tests run on the owner's
  PC.

Look at the sheets (they are PNGs) and fix what does not read. "Close enough for the eye test" is the bar:
each of the three beats of each move should be clearly visible, bigger and clearer than classic, in the
fighter's colours. Do not polish beyond that.

## Maths

Every `sin`, `cos`, `tan`, `atan2`, `hypot`, `powf`, `powi`, `exp`, `ln` goes through `bf_core::dmath`
(`dmath::cos(a)`, `dmath::hypot(x, y)`, `dmath::TAU`), never the std methods (`a.cos()`, `f64::powi`): the
engine's frames must be the same bits on every PC, and the test `bf-core/tests/no_std_maths.rs` fails on any
direct call in these crates. `sqrt`, `abs`, `min`, `max`, `floor`, `round` are fine as methods.

## Before you report

```
cargo fmt
cargo clippy -p bf-content --all-targets -- -D warnings 2>&1 | grep -E "^(warning|error)" -A12
./target/debug/ballfight moves fights/dev/bold_<slug>.toml --out <new folder> --effects classic --kinds hit --nth 3
```

The last line is a smoke test that classic still draws. Classic frames must not change at all: do not move or
edit classic code beyond adding the `if p.bold` hand-overs, and keep helper changes to visibility only.
