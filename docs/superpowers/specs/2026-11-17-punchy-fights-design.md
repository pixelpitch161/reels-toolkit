# Punchy fights: shared visual layer (design)

Date: 2026-11-17. Thread "Make fights more visually engaging". Requested by Aedan.

## Why

Aedan: "in general there doesn't feel like there's a lot of impact for the fighters' attacks", and the moves "could be
much more visually polished and exciting". This design is **part 1**: one shared presentation layer that lifts every
fight at once. **Part 2** (later, separate designs) redraws the moves of fighters Aedan names, a few at a time.

## Goals

1. **Hit impact** on every damaging hit, scaled by damage.
2. **Subtle action camera**, with one dramatic moment: the zoom on the K.O. shatter.
3. **Low-HP drama** for a fighter under 25 HP.
4. **Arena reacts to the music** more than it does today.
5. **Bigger K.O.**

## Non-goals and anti-goals

- Never changes a fight: everything reads the recording's events and arrays (`hit`, `block`, `HP`, positions, K.O.
  time) and the show's beat grid. No sim, seed or roster change.
- No full-screen flashes; `flashcheck` and `safezone` must pass. The HUD, captions and title card are untouched.
- Not wild: Aedan asked for a *pretty subtle* camera. Motion sickness or a ring that leaves the frame is a failure.
- No edits to `projects/`, `reelkit/`, `series/`. Windows only.
- The audio is not touched; the picture never drifts from the audio by more than a few frames, and only around a hit.

## Approach

Picked: a **"Punchy" look**, a render setting beside the music's Composed/Classic, **default for new renders, Classic
one click away**. Classic stays the bit-exact Python port, so golden frames, older renders and all reference tests are
unchanged. Rejected: changing the Classic look in place (breaks parity tests and older videos); per-fighter work for
these effects (40x the cost, the events are already shared).

## The design

All built once in `bf-picture` from the events when the `Picture` loads (like `_fx_build`), evaluated per frame, so
frames stay pure functions of their number.

**1. Hit impact** (per `hit` event with damage `d`; strength `s = clamp(d / 20, 0.25, 1)`):
- Micro hit-stop: picture time slows to ~15 % for `round(1 + 3 s)` frames, then catches up over ~6 frames, so the
  picture is back on the audio's clock within ~0.3 s. One picture clock (`punch_clock(t)`) feeds the sim lookup and
  the effects clock in Punchy; overlapping hits merge. Not during the count-in or after the K.O. (that has its own
  hit-stop).
- A shockwave ring on the floor at the contact point, in the attacker's colour, growing and fading over ~0.35 s.
- The struck ball kicks a few pixels along the hit direction and springs back (picture offset only).
- A small burst of debris particles plus brighter, larger sparks; heavy hits (`s >= 0.7`) add a local chromatic
  fringe and a stronger shake (still held under a cap).
- Blocks get a lighter version (no hit-stop, grey ring).

**2. Camera** (Punchy keys layered on the existing 9 keys):
- On each heavy hit a quick ~4-6 % push-in toward the hit, eased back over ~0.6 s; small yaw lean toward the action.
- Final exchange (last ~6 s before the K.O.): a slow push-in of ~8 %.
- K.O.: through the hit-stop and slow-mo shatter, a dramatic zoom (~35-45 %) onto the loser's shards, then an eased
  pull-back to the normal frame before the winner's solo. The ring may crop during the zoom, the HUD never moves.
- 58-61 s ends exactly on the Classic camera so the loop to frame 0 is seamless.

**3. Low-HP drama** (fighter below 25 HP, until the K.O.):
- Cracks on its glass ball that grow with each hit taken below 25 HP (seeded per fight, so deterministic).
- Thin smoke wisps and occasional sparks from the ball.
- Its HUD HP bar pulses like a heartbeat (faster as HP falls); this is the one HUD change.

**4. Arena reacts to the music** (the LED boards already chase and pulse on the beat):
- Stronger pulse on downbeats, soft floor light pools that bloom on each bar.
- The arena lighting leans toward the leader's colour by HP difference.
- On a lead change, a colour sweep runs around the LED boards to the new leader.
- The crowd bobs on the beat, harder in the final exchange.

**5. Bigger K.O.**:
- Everything but the loser's shards and the K.O. text desaturates toward grey through the hit-stop and slow-mo,
  colour returning as the camera pulls back.
- Radial speed lines from the K.O. point; "K.O." slams in with overshoot and a shake.

## Testing (only the changed parts)

- Classic: existing golden-frame / replay tests unchanged (run only the bf-picture ring tests that cover the switch).
- Punchy: new tests that it is deterministic (same frame twice = same bytes), the picture clock returns to the audio
  clock, the last frames equal Classic's (loop), and `flashcheck` + `safezone` pass on one Punchy render.
- Eye test on a few rendered stills / a short clip, posted to Aedan.

## Left to the builder

Exact numbers (strengths, durations, zoom amounts, colours), how the setting is stored and shown in the app (copy the
music arrangement setting's pattern), and how the new passes slot into `frame.rs`'s stages.
