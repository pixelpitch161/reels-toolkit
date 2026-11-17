# BALLFIGHT Studio Engine 2: fight engine and GPU renderer rework

Status: sent to Aedan for approval (2026-11-17). Thread "Rework rendering for advanced fight visuals".

## Intent and why

Aedan wants the fights to look like an over-the-top anime fight: heavy particles, dramatic lighting, motion blur,
camera work, full-screen flashes, hits that feel enormous. The "Punchy" attempt (closed PR #9, branch
`claude/project-thread-rqffes`) added a presentation layer on top of the current engine. Aedan liked the ideas but
not how they came out, and suspected the engine. Our investigation agreed. Today:

- the picture is a CPU software renderer, ported pixel for pixel from the Python/Pillow kit, drawing at 360 x 640
  and blowing up 3x with blocky nearest-neighbour scaling. Every effect turns into chunky 3x3 pixels, and heavy
  particles, blur and lighting are slow on the CPU. wgpu draws only the app's interface;
- the fight is a flat 2D simulation that records a 30 fps snapshot. A hit happens in one instant. Moves have no
  wind-up, contact moment or recovery, so the picture has nothing to dramatise and had to invent anticipation
  (Punchy's clock ran ahead and slowed down to fake it);
- the camera has 9 nearly fixed keys for the whole minute.

Aedan's decisions (thread, 2026-11-17):

- rework **both** the fight engine and the renderer, built for the most headroom in visuals *and* mechanics;
- earlier fights do **not** need to replay the same. Every fighter will be reworked to some extent, and move sets
  and mechanics may change;
- the shared systems get built first. After that, **one session per fighter**, so each gets the highest quality;
- Engine 2 is built **beside** the current engine (not on Bevy, not replacing it in place). The app keeps working
  while fighters move over.

Success means a fight on Engine 2 reads as a crisp, cinematic anime clash at full 1080 x 1920. Each fighter's moves
have wind-ups, impacts and exits that the camera, lighting, time and sound all play up. A fighter session can reach
that quality using the shared toolkit, without inventing engine features.

## Goals

1. A new deterministic fight engine whose moves have phases (wind-up, strike, recovery), real hit reactions
   (freeze-on-hit, knockback, launches, juggles, slams, wall and rope bounces) and cinematic specials that cue the
   camera and the time.
2. A new GPU renderer (wgpu, on Aedan's Radeon RX 9070 XT) drawing at full 1080 x 1920 in HDR, with real lights and
   shadows. It's used for both the app preview and the final video.
3. A shared effects toolkit rich enough for every fighter's signature look: GPU particles in the tens of thousands,
   trails and ribbons, motion blur, bloom, god rays, radial blur, shockwave distortion, flashes, decals, emissive
   lights from effects, and grading.
4. A camera director and time control: framing, punch-ins on wind-ups, tilts, whip pans, shake, slow-mo ramps,
   hitstop, and a K.O. cinematic. Sound and music follow the same timeline.
5. The app (preview, seed search, auto-balance, render queue, Matchups, VS card) works with Engine 2 fighters next
   to current-engine ones.
6. A hidden test dummy fighter that exercises every system, and a fighter guide that each fighter session follows.

## Non-goals

- Porting fighters during the systems work. Apart from the test dummy, fighters move over later, one session each.
- Removing the current engine. It stays until the last fighter has moved over, then gets a separate clean-up.
- Bit-for-bit parity with the Python kit or with current-engine fights.
- New game modes (boss fights, brackets, brawls, arenas). They're still "later". Engine 2 shouldn't rule them out,
  for example by assuming exactly two fighters in its core types.
- Linux or macOS support.

## Anti-goals (failure even if it technically works)

- Effects that look cheap or small: jagged lines, sparse particles, effects that pop in or cut off abruptly
  (Aedan's batch-1 notes: Bari Sax crane and Trombone PEDAL TONE bells cut off; impact frames looked bad).
- Drama bolted onto moves that don't have it. Anticipation, contact and follow-through must exist in the fight
  itself.
- A dishonest fight. A fight is still chosen by its seed and never edited by hand.
- A systems layer so rigid that a fighter session has to fight it. The toolkit gives building blocks, and a fighter
  may add its own shader or effect code in its folder.
- The app breaking, or losing fighters, while fighters move over.

## Constraints

- Rust, wgpu, Windows only. The app is only ever used by Aedan on DESKTOP-KLHSCI6 (Ryzen 7 9800X3D, 64 GB, RX 9070
  XT 16 GB).
- Never edit `projects/`, `reelkit/` or `series/`. GUITAR and SAX stay retired as fighters.
- Video spec is unchanged: 61.0 s, 1080 x 1920, 30 fps, h264 + aac, loop at the file boundary, the safe zone,
  consistent loudness, one unique colour per instrument.
- Full-screen flashes are now allowed (Aedan, 2026-11-17). The photosensitivity check stays, as a cap on how often
  flashes happen (Punchy used at most 3 per second and at least 0.5 s apart). **Theirs to confirm, see Open
  questions.**
- Determinism. The fight (simulation) must be reproducible from its fight file and seed on every run, because the
  seed search, auto-balance and Matchups depend on it. The picture only needs to be reproducible on Aedan's PC
  (same GPU and driver). Tests compare pictures within a tolerance, not byte for byte.
- Testing: only the tests for what changed, never the full suite during feature work. Installs use the quick
  install check. GPU tests are Windows only.
- Disk: build in the main folder, own copies only under `.wt` (max 3), and run `studio\scripts\clean-builds.ps1
  -Apply` after merging (HANDOFF.md "Disk space").
- Subagents and workers run on claude-opus-5-5.

## Approaches considered

| | Approach | Trade-off |
|---|---|---|
| **A (picked)** | **Engine 2, our own, beside the current engine.** New sim crate and new wgpu renderer written for this app. Fighters move over one session at a time. | Most control and headroom, no outside limits, and the app keeps working. The most up-front work. |
| B | Built on Bevy (existing Rust game engine). | Faster first visuals (lighting, bloom, particle crates). But it's a large dependency we don't control, frame-exact offline capture into ffmpeg is awkward, the egui app would need re-hosting (bevy_egui), and build times (already ~42 min to compile tests) grow. |
| C | Replace the current engine in place. | No duplication, but the app has no working fighters until each fighter's session is done. |

Aedan picked A (decision card, 2026-11-17 05:16Z).

## Design

### 1. Shape: crates and where things live

New crates in `studio/crates/`. The current `bf-sim` and `bf-picture` are left alone.

- `bf2-sim`: the fight engine (headless, CPU, deterministic). It holds the move framework, physics, hit
  resolution, AI and the recording.
- `bf2-render`: the wgpu renderer. It holds the scene, materials, lights, shadows, the post stack and
  offscreen-to-ffmpeg readback. It shares the app's wgpu device when the app is running.
- `bf2-fx`: the effects toolkit (particles, ribbons, decals, screen effects, presets), built on `bf2-render`.
- `bf2-direct`: the director. It holds the camera rig and shot rules, the time map (fight time to video time) and
  the event cues for sound and HUD.
- Fighter content: `bf-content/src/fighters2/<name>/` with `moves.rs` (move set for the sim), `look.rs` (models,
  materials, effects), `sound.rs`/`voice.rs` (reuse or adapt the current ones) and `card.rs`. Each fighter's folder
  is self-contained, as today.
- A `FightEngine` seam in `bf-app`/`bf-story`/`bf-export`. Search, balance, preview, takes and render jobs call
  through it, so a matchup runs on the engine its fighters belong to.

`bf-core`'s fight file gains `engine = 2`. A missing field means the current engine, so old fight files and the
Matchups history keep working.

### 2. Fight engine (`bf2-sim`)

**Space.** A 2D ring floor plus a **height axis** (this was my proposal, and Aedan hasn't objected). Balls can be
launched, juggled, spiked into the canvas, bounced off ropes and walls, and land. Full 3D rigid-body physics is out:
it costs a lot more and adds little you'd see.

**Clock.** A fixed step of 240 Hz (builder may pick 120 Hz if 240 Hz is too slow for search). The recording keeps
**every step**, not 30 fps snapshots. Slow-mo down to about 0.25x and sub-frame motion blur both need states
between video frames.

**Fighters as rigs.** Each fighter is a ball (body, HP, mass, radius) plus an articulated **weapon rig**: named
bones such as a bow, a slide, a bell or mallets, posed by the current move. Hitboxes and hurtboxes attach to bones.
The renderer draws the same rig, so the picture shows exactly the pose that hit.

**Moves.** A move is data plus optional code:

- phases with durations in steps: `windup` (telegraph), `active` (hitboxes live), `recovery`; optional
  `charge`/`hold`, multi-hit chains and follow-ups;
- a pose track for each bone across the phases, with easing, so swings have anticipation and follow-through;
- on-hit result: damage, **hitstop** (both fighters freeze for N steps of fight time), knockback vector including
  vertical launch, hitstun, juggle decay, wall-bounce or slam flags, and a "strength" from 0 to 1 that the effects
  and camera scale with;
- tags: light, heavy or special; melee, ranged or grab; and the camera and time cues a special asks for (for
  example "punch-in on wind-up, 0.3x at contact, whip back");
- projectiles and summons as general entities with their own small move state (a dart, a bell, a note wave).

**Behaviour (AI).** Fights stay autonomous. Each fighter has a behaviour profile that, from its state and the shared
seeded RNG, picks moves: weights by range, cooldowns, a meter for specials, the low-HP ability. Current-engine
abilities (unlock timers, the low-HP ability, levels) are re-expressed as moves and meters. The builder keeps the
concepts the app and captions rely on: ability names, "first use" captions, levels.

**Hit resolution.** Each step: advance moves, then integrate physics, then test hitbox against hurtbox and blocks
(clash rules when two active hitboxes meet), then apply results. Order is fixed and documented. Everything uses
`bf-core`'s deterministic maths (`dmath`) and RNG, so results don't depend on the CPU or thread count.

**Events v2.** A rich, typed event list: `move_start`, `windup`, `active`, `contact {point, normal, strength,
move, bones}`, `block`, `clash`, `launch`, `land`, `bounce`, `slam`, `special_start/cue`, `ability`, `low_hp`,
`ko`. The director, effects, sound, captions and HUD all read these.

**Speed target.** Seed search and the 3,000-fight balance confirmation must stay usable. Target: the full
auto-balance (29 measurements) finishes in about the same time as today on 8 cores. The builder measures today's
time first and reports it. The sim records nothing during search (only the outcome figures). The full recording is
made only for the chosen seed.

**Fit to the video.** The fight still has to fit the 61.0 s video with its intro, K.O. and outro. Because hitstop
and slow-mo stretch video time, the seed search uses the director's time map (section 4) to check the fit.
Tunable limits (total slow-mo seconds, hitstop caps) keep the director's stretching bounded, so most seeds fit.

### 3. Renderer (`bf2-render`) and effects (`bf2-fx`)

**Output.** It renders offscreen at 1080 x 1920 (internally with MSAA or 1.5x supersampling, builder's call) in
HDR `rgba16f`. Tone-mapping and grading happen at the end, then the frame is read back and piped to ffmpeg through
the existing `bf-export` encode path (ffmpeg discovery, loop-clean encode and checks are reused). In the app, the
preview draws the same scene straight into an egui wgpu callback, so what you preview is what renders.

**Art direction (my proposal, correctable).** Keep the BALLFIGHT identity: the low-poly instrument models inside
glass balls, the ring, the arena, crowd and LED boards. Draw them crisp at full resolution with real lighting:
stylised (toon-banded) shading on the models, rim light in the fighter's colour, specular glass, and contact shadows
and shadow maps from the spotlight and from effect lights. The existing mesh data (`bf-picture/src/sprite.rs`
meshes) is the starting point for the instrument models. Fighter sessions may refine or remodel from real
instrument photos, per the standing rule.

**Lighting.** Clustered or forward+ lighting with many dynamic point and spot lights. Every effect can emit light
(a sax blast lights the canvas and the ropes orange). There are god rays from the spot, theme-driven arena lighting,
and dramatic lighting changes on specials (house lights drop, a key light on the attacker).

**Effects toolkit.**

- GPU compute particles (100k+ live), deterministic from a seed and the fight time. Emitters can be sparks,
  embers, smoke, debris, liquid, confetti or musical notes. They support forces (gravity, drag, vortex, attractors),
  collision with the canvas and ropes, and soft particles.
- Ribbons and trails tied to weapon bones (swing arcs, slash smears) with fade-out. Mesh effects (shock rings,
  domes, beams) and flipbook sprites.
- Decals on the canvas and ropes (scorch, cracks, paint). They last and fade.
- Screen space: per-object motion blur (velocity buffer, plus sub-frame accumulation for the biggest moves), radial
  and zoom blur toward impacts, shockwave distortion, chromatic aberration, bloom with lens dirt, speed lines,
  full-screen flashes and inverts, letterbox for specials, vignette, film grain, and a per-theme colour grade (LUT).
- **Exits are mandatory.** Every effect and summoned model has an exit (dissolve, burst, shrink, fly off). The
  toolkit enforces a lifetime with an end animation, so nothing just disappears.
- **Presets.** "Light hit", "heavy hit" and "special impact" presets combine particles, light, shake, blur and
  sound cues. They scale with `strength`. A fighter can use, retune or replace them.
- Fighters may add their own WGSL shaders and effect code inside their folder.

**HUD, captions, count-in and VS card.** The current look stays: pixel font, layouts and VS card panels. It's drawn
as a 2D overlay at full resolution (the pixel font scaled cleanly), composited over the 3D frame. The builder decides
whether to port the CPU draws or redraw them on the GPU, as long as the look matches.

**Performance target.** Render a 61 s video at 30 fps or faster on the 9070 XT, and keep the preview smooth at full
speed.

### 4. Director and time (`bf2-direct`)

**Time map.** A monotonic map from video time to fight time, built from the events. It holds hitstop (fight time
stops), slow-mo ramps into specials (eased in, 0.25 to 0.5x, snapping back to normal speed on contact), and the K.O.
cinematic. Picture, sound, music and HUD all read this one map, so they stay in sync.

**Camera.** A rig with position, target, roll, FOV, plus shake on a trauma model. Shot rules:

- always frame both fighters inside the safe zone;
- punch-in and tilt on wind-ups of heavy moves and specials;
- whip pans and snap-back after big hits;
- follow launches upward;
- low angles on slams;
- the K.O. cinematic (slow push, freeze, shatter, grey-out, slam title).

Moves can request shots through their cues. The director resolves clashes between requests, with priority on the
bigger move, and limits how often big moves happen so the minute breathes.

**Arena reacts.** Ropes wobble on bounces, the crowd jumps on big hits, the LEDs flash in the attacker's colour, and
the lighting follows the beat (Aedan's earlier picks, now done properly).

### 5. Sound and music

`bf-sound` is reused. Sound effects are placed in **video time** through the time map. During slow-mo they are
stretched or pitched down (builder's call per sound, and fighters can supply slow-mo variants). Hitstop gets a
"freeze" treatment: a held tail or reverb swell. The Composed and Classic music keep running at video time and do
not slow down, but react to the v2 events (accents on contacts, drops on specials, the K.O. hit). Loudness checks
are unchanged.

### 6. App integration

- Each fighter is registered as current-engine or Engine 2. A matchup is offered on Engine 2 only when both
  fighters are on Engine 2. Mixed matchups aren't offered.
- **Both versions stay until every fighter has moved (Aedan, 2026-11-17).** A ported fighter keeps its
  current-engine version in current-engine matchups. Its Engine 2 version fights other Engine 2 fighters, the test
  dummy, or itself in a mirror match, which is what fighter sessions use for review clips. When the last fighter
  has moved, a separate switch-over retires the current engine. Old fight files and the Matchups history keep
  working throughout.
- Search, balance, preview, takes and the render queue go through the `FightEngine` seam. The interface doesn't
  change beyond a small "Engine 2" mark where it helps.
- Fight files gain `engine = 2`. The handicap, seed and theme fields work as today.

### 7. Test dummy and the fighter guide

- **TEST DUMMY**: a hidden Engine 2 fighter with one move of each kind (light melee, heavy melee with launch, grab
  and slam, projectile, special with full camera and time cues) and an effect of each kind. Two dummies fight a
  whole video. That's the proof for the systems milestone and the reference each fighter session copies from.
- **Fighter guide**: `docs/studio/engine2/FIGHTER-GUIDE.md`. It covers how a fighter session goes:
  1. brainstorm the move set with Aedan, with a concepts artifact;
  2. real photo references for the model (run on the PC via Remote Control, since image sites are blocked in the
     cloud);
  3. build the moves, the look, the sound and the VS card panel;
  4. a lenient balance check;
  5. clips for Aedan's review;
  6. merge into the build branch.

  It also gives the API of each system, the presets, the quality bar (no abrupt exits, light and heavy hits must
  read differently) and which tests to run.

### 8. Milestones (systems first, then fighters)

Each milestone is one or more build sessions. Aedan reviews a clip at E3 and E5. After E0 is approved, each
milestone has its own plan.

| | Milestone | Done when |
|---|---|---|
| E0 | Plan and seams. The `FightEngine` seam, fight file `engine = 2`, empty crates, today's speed measured. | The app builds and runs unchanged. |
| E1 | Fight engine core: rigs, moves with phases, hit resolution, hitstop, launches, bounces, AI, recording, events v2. | Two dummy fighters fight reproducibly. Search and balance speed reported against target. |
| E2 | Renderer core: scene, models, toon and glass materials, lights, shadows, HDR, offscreen readback to ffmpeg, app preview. | A dummy fight renders to a correct 61 s video. |
| E3 | Effects toolkit: particles, trails, decals, screen effects, presets, exits. | **Aedan reviews a clip** of the dummies with every effect. |
| E4 | Director and time: time map, camera rules, cues, K.O. cinematic, arena reacts, search fit check. | Dummy video with slow-mo specials and camera work. |
| E5 | Sound and music on the time map. HUD, captions, count-in and VS card on the new renderer. | **Aedan reviews a full dummy video** with sound. |
| E6 | App integration: engine registry, matchup rules, render queue, Matchups. Fighter guide written. | Ready for fighter sessions. |
| F1..F40 (all 40 fighters) | One session per fighter, in the order Aedan picks (batch 1 favourites first is my suggestion: Violin, Trombone, Accordion, Tuba, Cello, Bari Sax). | Each fighter is approved by Aedan and merged. |

**Size (rough, my estimate).** E0 to E6 is milestone-scale work, comparable to the original M0 to M2, so several
long build sessions. Each fighter session is a session of its own. Token cost is significant. The account hit its
monthly limit around 2026-11-14 and it resets on the 17th.

## Decided by Aedan

- Rework both engine and renderer for maximum headroom. Old fights needn't replay the same. Move sets may change.
- Systems first, then one session per fighter.
- Engine 2 beside the current engine.
- Full-screen flashes allowed (earlier thread).
- Keep both engine versions of a fighter until every fighter has moved (decision card, 2026-11-17 05:21Z).

## My calls (Aedan can correct)

- 2D floor plus a height axis, not full 3D physics.
- Sim at 240 Hz, recording every step.
- Keep the low-poly-instruments-in-glass-balls identity, crisp and lit, with toon-banded shading.
- Keep the HUD, captions and VS card look, just crisp.
- Music runs in video time and does not slow down in slow-mo.
- No mixed-engine matchups.
- Flash cap kept as a photosensitivity check (max 3 per second).
- Fighter count: 40 (the roster installed in 1.5.0). The toolkit is scoped for all 40.
- Seed-search fit: the director's time map is headless and cheap (built from events only, no camera or picture),
  so the search runs the real time map for every seed. The K.O. window (today 49 to 53 s) and the balance band
  (49 to 52 s) are measured in video time.
- Music: the full Composed system carries over. The story layer (`bf-story`: lead changes, desperation move,
  final exchange, K.O. tape stop, victory solo) is rebuilt from the v2 events, and Classic stays one click away.
- Render checks: safe zone, loudness, loop, the K.O. window and the flash cap carry over unchanged. The focus and
  band checks are tied to the current picture, so they get new definitions for Engine 2 (builder's call, and the
  milestone plan states them).
- Shipping: each systems milestone merges into the build branch with Engine 2 and the dummy hidden, so the app
  installed on the PC is unchanged until fighters arrive. Aedan reviews the E3 and E5 clips as video files
  rendered from the command line, like the Punchy clips. Versions and compat files follow the normal release
  process only when an install is asked for.

## Left to the builder

Crate internals, the wgpu pipeline layout, the shading model details, MSAA versus supersampling, the particle data
layout, the move data format (Rust builders versus data files), the exact AI scheme, the exact sim rate (240 or 120
Hz) within the speed target, the HUD port method, test layout.

## Open questions

- Flash cap: keep the 3-per-second photosensitivity cap? (My call is yes unless Aedan says otherwise.)
