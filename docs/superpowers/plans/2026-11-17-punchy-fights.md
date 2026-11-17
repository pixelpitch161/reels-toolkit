# Punchy Fights Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A "Punchy" picture look (default for new renders, Classic one click away) that adds hit impact, a subtle
action camera with a K.O. zoom, low-HP drama, a music-reactive arena and a bigger K.O.

**Architecture:** A new `bf-picture/src/punch/` module built once from the recording's events when the `Picture`
loads (`Punch::new`), queried per frame. `ring/frame.rs` calls it only when the fight file's `picture.look` is
`punchy`; with `classic` (and for every fight file that names no look) the frame is byte-identical to today.

**Tech Stack:** Rust (workspace `studio/`), existing `bf-picture` helpers (`Cam`, `Frame::glow`, `pil`, `Img`).

**Spec:** `docs/superpowers/specs/2026-11-17-punchy-fights-design.md`

## Global Constraints

- Never changes a fight; reads events (`hit`: `[kind, t, attacker, victim, damage, x, y, what, sub]`), `HP`, positions,
  `t_ko`, `t_hs`, the beat grid only.
- Classic output unchanged bit for bit; a fight file without `look` reads as `classic`.
- No full-screen flashes; HUD, captions, title card untouched except the low-HP bar pulse.
- Frames stay pure functions of `f`; Punchy deterministic.
- Last 3 s (58-61 s) and everything before `t_fight` identical to Classic (loop seam, title card).
- Test only changed parts (bf-core fightfile tests, bf-picture punch + ring tests, bf-app touched tests).

## Review Focus

- A hit in the last 0.4 s before the K.O.: no hit-stop (the K.O. has its own), clock still equals `t` at `t_ko`.
- Many hits in a burst: the picture clock stays monotonic and catches up (lag <= 0.25 s at all times).
- A fight file written by an older app (no `look`): Classic.
- A hit at the ring edge: the camera push never moves the HUD; zoom keeps ring centre within the frame.
- 0-damage / `key` / quiet hits: no impact.

---

### Task 1: `picture.look` in the fight file

**Files:** Modify `bf-core/src/fightfile/{mod.rs,read.rs,write.rs}`; test in `bf-core/src/fightfile/tests.rs`.
- `PictureSeeds.look: String` (`"classic"` | `"punchy"`), consts `LOOKS = ["classic", "punchy"]`; read `look` as an
  optional string in `[picture]` (unknown value is an error naming the field), default `classic`; write it only
  when not `classic` (so old files round-trip unchanged).
- Tests: `look_defaults_to_classic`, `look_round_trips`, `unknown_look_is_refused`.

### Task 2: `punch::clock` (micro hit-stop)

**Files:** Create `bf-picture/src/punch/{mod.rs,clock.rs}`.
- `PunchClock::new(hits: &[(f64 /*t*/, f64 /*strength*/)], t_lo: f64, t_hi: f64)`; `fn at(&self, t) -> f64`.
- Lag per hit: rises with slope 0.85 for `round(1+3s)` frames, then falls back to 0 over 6 frames; total lag = max
  over hits; `tp = t - lag`. Hits outside `[t_lo, t_hi - 0.4]` ignored.
- Tests: identity with no hits; monotonic over a dense burst; `at(t_hi) == t_hi`; lag <= 0.25.

### Task 3: Punch effects (impact, low HP, arena, K.O., camera)

**Files:** `punch/{impact.rs,camera.rs,lowhp.rs,arena.rs,ko.rs}`; wire in `ring/mod.rs` (`PictureCtx.punch:
Option<Punch>`), `ring/frame.rs`, `ring/state.rs` (`camera`), `ring/arena.rs` (`led_colour`, crowd bob),
`ring/hud.rs` (bar pulse).
- Impact: shockwave ellipse on the floor (attacker colour, 0.35 s), struck-ball kick (screen-independent world offset
  along the hit direction, spring back 0.25 s), debris particles from an own `Particles` (seeded from `fx_seed`),
  bigger glow, fringe on heavy hits.
- Camera: `dist` scale and target nudge: heavy-hit push `-5 % * s` eased back 0.6 s; final 6 s before K.O. `-8 %`;
  K.O. zoom `-40 %` to the loser's K.O. point through `t_hs + 1.0`, eased back by `t_solo`; nothing from 58 s.
- Low HP (< 25, before K.O.): seeded crack polyline on the ball sprite growing per hit taken; smoke wisps; HUD bar
  glow pulse at 60-120 bpm by HP.
- Arena: LED pulse 0.35 -> 0.6 depth on downbeats, leader lean by HP gap, sweep on a lead change (0.6 s), crowd bob.
- K.O.: desaturate except a disc around the K.O. point from `t_ko` to `t_hs + 0.8`, radial speed lines, extra shake.
- Tests: `classic_frames_unchanged` (a frame equals the build without punch), `punchy_is_deterministic`,
  `punchy_matches_classic_before_fight_and_after_58s`, render one Punchy fight and run flashcheck + safezone.

### Task 4: App switch

**Files:** `bf-app/src/model/draft.rs` (`look: Option<String>`), `engine/convert.rs` (`ff.picture.look =
draft.look.unwrap_or("punchy")`), `state/mod.rs` (`Action::SetLook`), `screens/render.rs` ("Look" segmented
Classic / Punchy beside Caption, hint line). Update only the touched UI tests.

### Task 5: Merge

Commit, push `claude/project-thread-rqffes`, open a draft PR into `claude/project-thread-h102zt`, merge once the
changed tests pass. Post stills to Aedan.
