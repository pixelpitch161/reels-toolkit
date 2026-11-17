# Engine 2 fighter guide

This is the guide for one fighter session (F1 to F40). Each session rebuilds one fighter on Engine 2. Read all of
it before you start. The worked example throughout is the TEST DUMMY, in
`studio/crates/bf-content/src/fighters2/test_dummy/`. Every name below is the name in the code. If one is missing,
the code has moved on. Grep for it, and fix this guide in the same session.

The rules from [`README.md`](README.md) still apply. Every fight goes through `bf-engine`. Both versions of a
fighter stay until every fighter has moved. There are no mixed-engine match-ups.

## 1. What a fighter session makes

One folder, `studio/crates/bf-content/src/fighters2/<slug>/`, shaped like `test_dummy/`:

| File | What it holds | The dummy's |
|---|---|---|
| `mod.rs` | `pub fn def() -> FighterDef`: the rig, the moves, the brain. It can also hold the moves inline, as the dummy's does. | `test_dummy/mod.rs` |
| `moves.rs` | One `fn` per move, each giving a `MoveDef`. This is optional when `mod.rs` stays short. | the move `fn`s in `test_dummy/mod.rs` |
| `look.rs` | A struct that implements `bf2_render::Look2` (the models) and `bf2_fx::frame::LookFx` (its own effects) | `DummyLook` |
| `sound.rs` (and `voice.rs` if it needs one) | Its part in the score (E5) | `test_dummy/sound.rs` |
| `card.rs` | Its VS card panel: `pub static CARD: FighterCard`, `pub const MOVES`, `pub const RANGE` | `test_dummy/card.rs` |

Then make these edits in `studio/crates/bf-content/src/fighters2/mod.rs`, and nowhere else:

- Add `mod <slug>;`.
- Add the name to `MOVED`. Its type is `[&str; N]`, so raise `N`. Also add the name to `NAMES`.
- Give it an arm in `def()` (a `OnceLock`, like the dummy's), in `look()` and in `look_fx()`.
- Once `card.rs` exists, give it arms in `card()`, `card_moves()` and `card_range()`. Until then, `card()` falls
  back to the fighter's current-engine card.

Never remove `Engine::One`. `bf_content::engines(name)` gives `[One, Two]` for a name in `MOVED`. Never edit another
fighter's folder. Never edit the shared crates (`bf2-sim`, `bf2-render`, `bf2-fx`, `bf2-direct`, `bf-sound`), with one
exception: you may add a building block every fighter can use, such as a new `TrailStyle` or a mesh helper in
`bf2_render::geom`. Name it in the session's ledger, and give it a test in its own crate.

Also add a fight file, `studio/fights/engine2/<slug>_vs_test_dummy.toml`. To make it, start the app with
`BALLFIGHT_DEV=1`, switch the Match-up screen's **Engine** control to Engine 2, pick `<NAME>` against TEST DUMMY, and
save. You can also copy `studio/fights/engine2/test_dummy.toml` and set `fighters = ["<NAME>", "TEST DUMMY"]`. Both
give `engine = 2`. The dummy's `dmg_scale = 2.0` is tuned for the dummy. Leave it until the balance says otherwise.

## 2. Session flow

Each step says what Aedan sees and approves. Do not move past an approval step without his answer.

1. **Brainstorm the move set with Aedan.** Use `superpowers:brainstorming`. Publish a concepts artifact with one card
   per move. Each card shows:
   - its phases (wind-up, active and recovery steps);
   - its hit kind: light, heavy, launch, grab, projectile or special;
   - its camera and time cues (`Cue`);
   - the effect ideas;
   - a sketch.

   Aedan picks and edits the moves before any code is written. *He approves the move list.*
2. **Real photo references for the model.** Image sites are blocked in the cloud. Start a Remote Control session on
   the PC with `start_remote_control`. Have it search the web and save 6 to 12 photos of the real instrument to
   `/mnt/project-files/ballfight-studio/refs/<slug>/`: the whole instrument, its details, and how it is held. Have it
   list the sources. Build the low-poly model from those photos, not from memory.
3. **Build, in this order:**
   - the moves (section 3.1);
   - the look (3.2, 3.3);
   - the sound (3.5);
   - the VS card panel (3.6).

   After each part, run its tests (section 6).
4. **Lenient balance.** On the dummy match-up, run:

   ```
   ballfight --json balance fights/engine2/<slug>_vs_test_dummy.toml
   ```

   Run it from `studio/`, without `--write`. Read `confirm` in the output.
   - **It passes when** both of these hold:
     - `win_share` is between 0.30 and 0.70;
     - the K.O. lands in the window (49 to 53 s of video) in at least 10 % of seeds. That share is
       `sum(ko_hist[19..23]) / n`, because bin `i` holds K.O.s with `30 + i <= t < 31 + i`.
     The text output prints `win_share` as "wins x % of ...".
   - **Not the 40-60 % band.** The command's own "even (40 to 60 %...)" line is the series' band, and it does not
     need to say yes. Perfect balance comes later, from the handicap.
   - **Cross-check the K.O. share** with `ballfight search <file> --max 400`. Its fits count only seeds that meet
     the whole beat sheet, so it reads lower.
   - **A fighter cannot fight itself.** A fight file never names one fighter twice. So once another real fighter is
     on Engine 2, also balance `<slug>` against that fighter, with the same rule.
5. **Review clips.** On the PC, in release, render the dummy match-up, plus a moved-fighter match-up when one exists.
   First run `ballfight search <file> --stop 10` to find a seed that fits. Then:

   ```
   ballfight render <file> --out <folder> --seed N
   ```

   For a quick look, add `--clip A:B`, which renders frames A to B-1. Take 6 stills from the finished
   1080 x 1920 video:
   - a light hit;
   - a heavy hit;
   - a launch or a grab;
   - the special;
   - a projectile, when it has one;
   - the K.O.

   Use `ffmpeg -i <video>.mp4 -vf "select=eq(n\,N)" -frames:v 1 <still>.png`. `ballfight frame` draws only
   current-engine fights. Copy the clip and the stills to `/mnt/project-files/ballfight-studio/engine2/<slug>/`.
   *Stop for Aedan's verdict, then iterate.*
6. **Merge** into the build branch after he approves. **Never install** unless Aedan asks. Then run the quick install
   check in `docs/studio/HANDOFF.md`.
7. **Clean up the disk** (HANDOFF "Disk space"). After merging and pushing, run
   `powershell -ExecutionPolicy Bypass -File studio\scripts\clean-builds.ps1 -Apply`. Keep at most 3 build folders of
   your own, and at least 150 GB free on F:. Build in the main folder, not a worktree, unless it is busy. If it is,
   use `.wt\<name>` with `CARGO_TARGET_DIR=<repo>\.wt\<name>-target`.

## 3. The systems' APIs

### 3.1 `bf2-sim`: rig, moves, hits, projectiles, brain, events

A fighter is a `bf2_sim::FighterDef`, with these fields:

- `name` (a `String`; the roster name, in capitals);
- `r` (the ball's radius, 0.6 for the dummy);
- `mass`;
- `hp_scale` (more HP means more, smaller hits);
- `rig`;
- `moves`;
- `brain`;
- `colour` (sRGB `[u8; 3]`; it must be unique; see `docs/ballfight/RULES.md`).

`FighterDef::check()` must give `Ok(())`.

**Rig.** Build it with `Rig::new(vec![Bone { name, parent, length, radius }, ..])`. List the bones parent first. The
dummy's rig is `arm` (0.5 long, 0.12 thick) from the ball, then `fist` (0.35, 0.16, parent `Some(ARM)`). Keep the
bone indices as `const`s (`ARM`, `FIST`).

**Moves.** A `MoveDef` has these fields:

- `name` (`&'static str`, in capitals; the card and the HUD write it in sentence case);
- `tags` (`Vec<Tag>`);
- `windup`, `active`, `recovery` (steps per phase);
- `keys`;
- `hits`;
- `spawns`;
- `cues`;
- `guard` (the share of damage taken during the move; 1.0 means no guard);
- `armor`;
- `lunge`;
- `hop`.

The dummy builds every move from `base(name, tags, (windup, active, recovery))` with struct update. Copy that
pattern.

- **`Tag`:** `Light`, `Heavy`, `Special`, `Melee`, `Ranged`, `Grab`, `Low`.
  - `Special` without `Low` makes a special move. The first two get caption targets at 12 s and 24 s
    (`bf_engine::sheet2`). They are the HUD's SP1 and SP2 rows.
  - The first `Tag::Low` move is the low-HP ability (`bf2_direct::shadow::low_name`), with a caption target at 40 s.
  - Give a fighter two specials and one low move, as the series' fighters have. The beat sheet asks for all but one
    of the captions the pair can make.
- **Keys.** A `Key { at, ease, joints }` sets each joint at a share `at` (0 to 1) of the whole move.
  - `joints` is `vec![(bone, Joint { yaw, pitch, ext })]`, and `Joint::REST` is the rest pose.
  - `Ease` is one of `Linear`, `In`, `Out`, `InOut` or `Snap`.
  - Every move pulls back in the wind-up, snaps through in the active steps, and overshoots in the recovery. Then it
    ends on `settle()`, which eases both joints to rest at 1.0. `jab()` is the smallest example and
    `dummy_buster()` the largest.
- **Hits.** A `HitDef` has these fields:
  - `bones` (which bones hit);
  - `window` (active steps, as `(from, to)`);
  - `damage`, `hitstop` (steps frozen), `knock`, `lift` (above 0, it launches), `hitstun`;
  - `strength` (0 to 1: the effect and sound tier; see section 4);
  - `multi` (it can hit again in the window);
  - `grab`.

  The dummy's helper is `hit(bones, window, damage)`.
- **Grab.** `grab: Some(Grab { hold, bone, slam, throw })` holds the target at the tip of `bone` for `hold` steps,
  then slams it down (`slam`) or throws it (`throw`). See `grab_slam()`.
- **Projectiles.** A `Spawn { at, proj: ProjDef { .. } }` fires at active step `at`. `ProjDef` has these fields:
  - `name`, `speed`, `up`, `gravity`, `radius`, `life`;
  - `hit` (a `HitDef` with empty `bones`);
  - `bounces`, `homing`.

  See `toss()`.
- **Cues.** A `CueAt { phase, step, cue }` uses `Phase::{Windup, Active, Recovery}`. The `Cue` values are:
  - `PunchIn`;
  - `Tilt(deg)`;
  - `SlowMo { scale }`;
  - `WhipBack`;
  - `Flash`;
  - `Shake(amount)`;
  - `Custom(&'static str)`.

  The director reads them (3.4), and `bf2_fx::wire` turns `Flash` and `Shake` into screen effects.

**Brain.** A `Brain` has these fields:

- `choices`;
- `speed`;
- `think` (steps between decisions);
- `low` (`Some(index)` of the low move);
- `orbit` (the share of time it circles instead of closing in).

A `Choice` has these fields:

- `mv` (the move's index);
- `range` (`(near, far)` metres);
- `weight`;
- `cooldown` (steps);
- `needs_meter` (true for the meter special);
- `air` (`Some(false)` means only on the ground).

The dummy uses the helper `choice(mv, range, weight, cooldown)`. Cooldowns pace the fight, so tune them first when the
K.O. comes too early or too late.

**Events v2.** `bf2_sim::run(&Setup2 { fighters, rules, seed }, record)` gives a `Recording2`. Its `events` are
`Ev { step, who, kind: EvKind }`. The `EvKind` values are:

- `MoveStart`, `Windup`, `Active`, `Recovery`;
- `Contact`, `Block`, `Clash`;
- `Launch`, `Land`, `Bounce`, `Slam`;
- `Grabbed`, `Released`;
- `Spawn`, `Expire`, `ProjHit`;
- `Cue`, `Ability`, `LowHp`, `Ko`.

Looks, effects, the director and sound all key off these events. Nothing reads the moves' code.

### 3.2 `bf2-render`: the look

`Look2` (`bf2_render::look2`) has two required methods:

- `setup(&mut self, r: &mut Renderer, gpu: &Gpu)` makes the meshes once. For example,
  `r.meshes.add(gpu, geom::cylinder(1.0, 1.0, 20))`.
- `draw(&self, f: &FightFrame<'_>, k: usize, out: &mut Scene) -> Vec<u32>` adds fighter `k`'s weapon, its instrument
  inside the glass, and its lights for one frame.
  - It reads `f.snap.pos[k]`, `f.snap.facing[k]` and `f.snap.joints[k]`, and the recording as `f.rec`.
  - It returns the ids of the projectiles it drew. The stage draws a default glowing sphere for the others.

`instrument()` is optional.

- **Meshes:** `bf2_render::geom` has `icosphere`, `box_`, `plane`, `cylinder` and `capsule`, plus `from_sprite_mesh`,
  which turns a current-engine `bf_picture::sprite::Mesh` into 3D. Build a low-poly instrument from boxes,
  cylinders and spheres, as the dummy's metronome does.
- **Materials:** `Material::toon()` for the weapon, `Material::matte()`, and `Material::glass(tint)`.
  `Instance::new(mesh, material, model).coloured(rgb)` places one.
- **Lights:** `bf2_render::Light { pos, colour /*linear HDR*/, range, priority }`. Use priority 1 for a fighter's
  lights; the stage's are 0. The dummy's orange projectiles each carry one.
- **Space:** `to_render` converts sim space (floor x, y, height z) to the renderer's space.
  `bf2_render::stage::ball_centre` gives a ball's centre. `bf2_sim::solve` gives a pose's segments, which a capsule
  per bone follows. See `DummyLook::draw`.

### 3.3 `bf2-fx`: effects

The shared effects come for free. `bf2_fx::wire::wire` turns every event into preset records (section 4): contacts
by tier, blocks, clashes, launches, landings, slams, projectiles, the K.O., and the `Flash` and `Shake` cues. A
fighter adds only its signature. It does this through `bf2_fx::frame::LookFx`, implemented on the same look struct:

```rust
fn fx(&self, k: usize, rec: &Recording2, step_to_t: &dyn Fn(u32) -> f64, out: &mut FxTimeline) -> Result<(), FxError>
```

The method goes through `rec.events` for `e.who == k` and pushes one `FxRecord` per effect. Each record is
`FxRecord { t0 /*video s: step_to_t(e.step)*/, life, exit, env, seed, anchor, kind }`.

- **`exit: Exit`:** `Fade`, `Dissolve`, `Burst`, `Shrink` or `FlyOff`. Every record has one.
- **`env: Env::new(attack, hold, release)`:** the release must be at least `bf2_fx::presets::MIN_FALL` (0.08 s).
- **`seed`:** take it from the event's step and index, so every PC draws the same.
- **`anchor: Anchor`:** `Anchor::Bone { k, bone }` follows a bone, and `Anchor::World(point)` stays put.
- **`kind: FxKind`:** `Burst(BurstDef)`, `Trail(TrailDef)`, `Ring(RingDef)`, `Dome(DomeDef)`, `Beam(BeamDef)`,
  `Decal(DecalDef)`, `Light(LightDef)` or `Screen(ScreenDef)`.

The dummy's signature effects:

- a `TrailStyle::Smear` on the fist through HAYMAKER;
- a `TrailStyle::Lightning` trail through DUMMY BUSTER;
- a `Dome` and a `Beam` on its final hit;
- a ribbon (`trail::PROJ`) behind each TOSS projectile;
- smouldering `Decal`s where GRAB SLAM lands.

Its colours are written hot (HDR, as `GREY_HOT` and `ORANGE_HOT`), because the bloom brings them down.

No per-fighter shader hook exists yet. If a fighter needs its own shader, add it as a shared building block in
`bf2-fx` (section 1's exception), with its WGSL in `bf2-fx/src/shaders/` and its test in `bf2-fx`. Then name it in
the ledger.

### 3.4 `bf2-direct`: camera and time

The director turns the recording into the time map (`TimeMap`: hitstop, slow-mo ramps, the K.O. cinematic) and the
camera track. It runs in the render job (`bf_export::job2::direct`) and in the seed search
(`bf_engine::Prepared::candidate`), so the beat sheet is judged in video seconds. A fighter steers it only through
its moves' `Cue`s and what its hits do.

| Cue or event | What it gives (`ShotKind`) |
|---|---|
| `Cue::PunchIn` | Closer (distance x 0.62) and toward the attacker over the wind-up. It holds through the strike. |
| `Cue::Tilt(deg)` | Rolls the camera until the move's Recovery. |
| `Cue::WhipBack` | Back to the base shot fast, with a 6 % overshoot. |
| `Cue::SlowMo { scale }` | Slow motion that ramps in 0.15 s before the cue and holds 0.25 s past Recovery. It marks the move as a special shot. |
| `EvKind::Launch` | Follows the launched ball up, with the camera 8 degrees lower. |
| `EvKind::Slam` | A low angle, 14 degrees lower. |
| `EvKind::Clash` | A quick push-in to 0.9. |
| `EvKind::Ko` | A 0.35 s freeze, then 0.3x speed for 0.8 s, pushing in on the loser and orbiting. Then it pulls out on the winner. |

The defaults are in `DirectCfg::default()`. Big shots (punch-in, slam, K.O.) are at least `big_shot_gap` = 2.5 s
apart unless the later one has a higher priority. Slow motion and the K.O. add at most `max_extra` = 4 s of video
in all. Use `SlowMo` on the meter special only, as the dummy does. Give heavy moves a `PunchIn`.

### 3.5 Sound

Engine 2 sound is milestone E5 (`docs/superpowers/plans/2026-11-17-engine2-e5.md`), merged: the render job builds
the soundtrack with `bf_content::build_soundtrack2` (its step is the Soundtrack line; the app's preview take is
still silent):

- **The shared events.** `bf-sound` plays them on the time map for every fighter (`bf_sound::music::v2`):
  - `impact(strength, tag, seed)`, light, heavy or special by strength;
  - `freeze_swell` under hitstop;
  - the varispeed `warp` through slow motion (`bf_sound::warp`);
  - `splice` at the K.O. (`bf_sound::warp`).
- **The fighter's part** lives in `fighters2/<slug>/sound.rs`, in the shape of `test_dummy/sound.rs`. It has a type
  that implements `Sound`, which reads only the shadow recording (`play`, `title_blast`, `count_in`, `stinger`,
  `solo`, `fall_apart`, `reassemble`, `last_bar`).
- **The fighter's identity.** `fighters2::{cfg, voice, sound, personality, card_mesh}` get an arm for the fighter.
  A moved fighter can reuse its current-engine voice and personality (`crate::fighters::voice(name)`).

The names above are checked against `test_dummy/sound.rs`.

### 3.6 The card

`card.rs` holds a `pub static CARD: bf_content::FighterCard`. Its fields are:

- `weapon`;
- `text` (it names every move the brain picks, in sentence case);
- `abilities` (three lines, for the moves of `MOVES`, in order);
- `colour`;
- `usually_plays`;
- `music_title`;
- `music_text`;
- `can_play_bass`;
- `emoji`;
- `tags`.

`MOVES: [&str; 3]` names the roster moves the ability lines describe: SP1, SP2 and LOW. `RANGE` is the brain's
nearest to farthest choice range in words, such as `"1.1 to 9 m"`.

The dummy's test `dummy_card_names_its_moves` (in `fighters2/mod.rs`) pins all three. Copy it for your fighter.

## 4. Presets

Start from these. `bf2_fx::wire` already calls them, so add the signature on top. Every one takes a
`presets::Hit { t0, seed, at, normal, colour, strength }`. `Tier::of(strength, special)` reads a strength as a tier:

- light: below 0.4;
- heavy: 0.4 to 0.85;
- special: above 0.85, or any `Tag::Special` move.

| Preset | What it gives |
|---|---|
| `hit_light` | Light tier: about 600 sparks, a small ring, a short light and a little shake. No screen effects. |
| `hit_heavy` | Heavy tier: about 6k sparks and 1.5k embers, a ring that bends the screen, a hot crack in the canvas, real shake, radial blur and a bloom push. |
| `special_impact` | Special tier: about 40k sparks, embers and smoke, a dome, two rings, a scorch, a flash, a shockwave, radial blur, chroma, speed lines, hard shake and a long flickering light. |
| `launch_trail(h, k)` | A ribbon behind fighter `k`'s launched ball, sparks kicked up from under it, and speed lines. |
| `land_dust` | A low ring of dust as a ball lands. |
| `slam_crater` | Debris, dust, a cracked and smouldering crater, and a ring that bends the screen. |
| `clash_spark` | A white starburst both ways and a hard little light. |
| `block_flare` | A shield flare and sparks glancing back. |
| `ko_burst(h, winner_colour)` | About 120k shards and embers, a big flash, a two-frame invert, the picture greyed except the winner's colour, letterbox bars and a shockwave. |
| `proj_spawn_flash` | A muzzle flash as a projectile is thrown. |
| `proj_expire_puff` | A puff and some embers as a projectile goes. |

Every preset scales its counts, sizes, lights, shake and screen effects with `strength`. So set `HitDef.strength`
for the tier you mean: the dummy's jab is 0.25, its haymaker 0.7, and its special's last hit 1.0.

## 5. Quality bar

- **No abrupt exits.** Every effect, summoned mesh, sound and light ends with an exit. The exit audit (section 6) runs
  20 seeds and checks three things for every record of the look's `fx`:
  - `life > 0`;
  - `env.release >= MIN_FALL`;
  - the brightness one 1/240 s sub-step before the end is under 2 % of its peak.

  It also checks `FxTimeline::check_all()`.
- **Light and heavy read differently.** They differ in particle count, size, light, shake and screen effects. A heavy
  hit has at least 8x the particles of a light one. The presets do this when the strengths are right.
- **Every move reads.** Each has a readable wind-up, contact and recovery: anticipation, a snap through, and a
  follow-through.
- **The colour.** The fighter's colour is unique (`docs/ballfight/RULES.md`) and is used: on the weapon, in the
  signature effects, and in the card's `colour`.
- **Aedan's batch-1 notes apply.** Nothing cuts off, and the impact frames look good.

## 6. Tests to run, only these

Run them from `studio/` with `CARGO_INCREMENTAL=0`. Never run the full suite; GitHub CI does that. Test only what
changed.

- `cargo test -p bf-content --lib fighters2::<slug>`. Write three tests in your folder, copying the dummy's in
  `fighters2/mod.rs`:
  - `<slug>_checks`: `def().check() == Ok(())`;
  - `<slug>_look_records_all_have_exits`: the exit audit for seeds 1 to 20;
  - `<slug>_card_names_its_moves`.

  Also run `cargo test -p bf-content --lib fighters2`, which keeps the dummy's tests green.
- `cargo test -p bf-content --lib registry`. This checks `MOVED`, `engines`, `offered` and `name_status`.
- `cargo test -p bf-engine`, only when you touched the seam (`bf-engine` or the fight-file reading).
- `cargo test -p bf-core --test no_std_maths`, when you added code to an engine crate. Engine code uses
  `bf_core::dmath`, never `f64::sin` and the like.
- `cargo check --workspace --all-targets`.
- `cargo fmt` and `cargo clippy -p bf-content --all-targets`.
- On the PC, when a snapshot count changes: `cargo test --release -p bf-app --test ui matchup`. A new name in `MOVED`
  adds the fighter to the Engine 2 picker.

## 7. Checklist

Paste this into the session's progress checklist.

```
<NAME> on Engine 2

○ Brainstorm the moves with Aedan; concepts artifact approved
○ Photo references saved to refs/<slug>/ (Remote Control)
○ fighters2/<slug>/: def() with rig, moves, brain; <slug>_checks green
○ look.rs: Look2 model, LookFx signature; exit audit green
○ sound.rs and card.rs; card test green
○ fighters2/mod.rs: mod, MOVED, NAMES, def/look/look_fx/card arms; registry tests green
○ fights/engine2/<slug>_vs_test_dummy.toml
○ Lenient balance: win share 30-70 %, K.O. in window >= 10 % of seeds
○ Review clip and 6 stills to engine2/<slug>/; Aedan's verdict
○ cargo check --workspace --all-targets, fmt, clippy
○ Merged after approval (no install); quick install check
○ clean-builds.ps1 -Apply; <= 3 build folders, >= 150 GB free on F:
```
