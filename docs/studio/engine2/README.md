# Engine 2

Engine 2 is a new fight engine and a new GPU renderer, built **beside** the current engine (`bf-sim` +
`bf-picture`). The aim is over-the-top, anime-style fights: moves with wind-ups and real impacts, full 1080 x 1920
GPU rendering, heavy particles, dramatic lighting, camera work and time control.

- Design (approved by Aedan, 2026-11-17): `docs/superpowers/specs/2026-11-17-engine2-design.md`
- E0 plan: `docs/superpowers/plans/2026-11-17-engine2-e0.md`
- E1 plan: `docs/superpowers/plans/2026-11-17-engine2-e1.md`; its speeds: [`e1-measurements.md`](e1-measurements.md)

## Rules every session follows

- **Every fight goes through `bf-engine`.** Nothing calls `bf_sim::setup_from` directly outside `bf-engine`,
  `bf-sim` itself, `bf-story`, `bf-harness` and tests. The fight file's `engine` setting (absent means the current
  engine) decides which engine fights it.
- **Both versions of a fighter stay until every fighter has moved** (Aedan, 2026-11-17). A fighter session adds
  `Engine::Two` to the fighter in `bf-content`'s `engines()` and never removes `Engine::One`. There are no
  mixed-engine matchups (`pair_engines`).
- One session per fighter, after the systems milestones (E0 to E6).
- Engine 2 changes nothing the user sees until fighters arrive. The dummy and the Engine 2 crates stay hidden.

## The crates

| Crate | Role | Built in |
|---|---|---|
| `bf-engine` | The gate from a fight file to a fight, on either engine | E0, grows in E1 and E6 |
| `bf2-sim` | Fight engine: floor plus height, rigs, moves in phases, hits, AI, recording, events v2 | E1 |
| `bf2-render` | wgpu renderer at 1080 x 1920 HDR, lights, shadows, readback for ffmpeg, preview | E2 |
| `bf2-fx` | Particles, trails, decals, screen effects, hit presets, mandatory exits | E3 |
| `bf2-direct` | Time map (hitstop, slow-mo), camera director, arena reactions | E4 |
| `bf-content/src/fighters2/<name>/` | One fighter's Engine 2 moves, look, sound and card | F sessions |

## Milestones

| | Milestone | Status |
|---|---|---|
| E0 | Plan and seams: `engine` in fight files, `bf-engine`, `engines()`, empty crates, today's speeds | in progress |
| E1 | Fight engine core | done (the PC speed run comes with E2's PC session) |
| E2 | Renderer core | not started |
| E3 | Effects toolkit, then a clip for Aedan's review | not started |
| E4 | Director and time: `bf2-direct` (time map, camera director, arena reactions) wired into the render job; `bf_engine::ko_video_t` for the K.O. window in video time | done |
| E5 | Sound and music on the time map; HUD, captions and VS card. Then a full video for Aedan's review | done in the cloud (stills and a K.O. clip; the full-length follow and steady renders run on the PC: `e5-measurements.md`) |
| E6 | App integration and the fighter guide | not started |
| F1..F40 | One session per fighter | not started |
