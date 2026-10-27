# 18. Marble Waterfall Organ

Board row: `docs/IDEAS.md` (#18). Series rules: none.

## 18. Marble Waterfall Organ — APPROVED
**Direction (user, Oct 22):** satisfying-first: cool and satisfying to watch and listen to; science is optional
flavour. Engagement brief: chaos and sound from frame 0, never quiet, build to a satisfying lock-in, exaggerate.
**Environment (user request):** abstract, as if in a vibrant void of space: no room, no ground.
**Concept:** a glowing marble machine hangs in a colourful void. It opens with every gate open: marbles spray off
every level, bounce off random notes and streak away into space. Gates click shut one by one, each trapping a
stream of marbles into a part of the song; fewer marbles escape as order takes over. The final gate sticks, then
snaps shut: the drop.
- **Hook:** "1,000 MARBLES" / "0 GATES" (13 + 7 chars, in the first 2 s). Alt: "1,000 MARBLES" / "1 MACHINE".
- **What you see:** a tall machine of brass and glowing glass, floating in a vibrant void. A spiral screw lift up
  the middle carries marbles to the top, where they pour down tiers of instruments: tuned bars, glass bowls,
  drum skins, tuned pipes, cymbal discs. 8 flip-gates; each steers a stream of marbles into its own channel.
- **How it plays:** every marble hit is a note. Gates open = marbles bounce off whatever they hit and spray into
  the void as streaks of light (a chaotic wash of random notes). Each gate that clicks shut routes its stream onto
  one instrument on the beat, so a part rises out of the chaos: kick → snare/hats → bass pipes → chord bowls →
  arpeggio ramp → melody. The last gate (melody) sticks and wobbles while its marbles still spray random notes;
  when it snaps shut = the drop, the full song at 120 BPM. All synthesised and original.
- **Environment (the abstract void):** no floor or room: a deep gradient void (indigo → magenta → teal) with
  dithered nebula clouds, star glitter in parallax layers, and floating low-poly crystals and rings drifting
  past. The void reacts to the music: nebula colours shift with each chord change; floating shapes pulse on the
  kick. Light: a small glowing star orb behind the machine (key light) and warm lantern nodes on each tier (local
  lights). Shadows land on the machine's own tiers and on a floating stone plinth under its base.
- **Payoff / ending:** the drop: all 8 channels at once, rainbow marble trails, nebula bursts, the camera flies
  down through the machine. At 57 s a lever flips, all gates spring open and marbles spill into the void again,
  looping to the opening chaos.

**Beat sheet (60 s):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–3 | Cold open: marbles overflowing every tier, bouncing off random bars, streaking into the void | 1,000 MARBLES / 0 GATES | Chaotic wash of random notes |
| 3–6 | Pull back: the whole machine floating in the nebula void | (hook stays) | Wash + low pad swell |
| 6–10 | Gate 1 shuts: marbles funnel onto a drum skin; void shapes pulse | GATE 1: / THE BEAT | Kick pulse rises out of the wash |
| 10–14 | Gate 2: marbles tick across glass discs | GATE 2 | Snare + hats; groove forms |
| 14–19 | Gate 3: marbles drop into tuned pipes | GATE 3: BASS | Bass line ("bloops") |
| 19–24 | Gates 4 + 5: chord bowls; fewer marbles escape | 5 OF 8 | Chords; wash thinning |
| 24–30 | Lift speeds up; one stream rides a long spiral xylophone ramp round the machine | GATE 6 | Fast arpeggio glissando |
| 30–35 | Nebula pulses with the chords; camera orbits; gate 7 adds cymbals | 7 OF 8 | Groove nearly complete |
| 35–38 | Holdout: top gate sticks and wobbles; melody marbles still spray random notes | LAST GATE… | Random melody notes over riser + snare roll |
| 38–40 | It snaps shut | 8 OF 8 | Drop |
| 40–52 | Full song: rainbow trails, nebula bursts, camera flies down through the machine | EVERY NOTE = / 1 MARBLE | All 8 channels + melody on the grid |
| 52–57 | Wide shot of the machine glowing in the void; final chorus | (no caption) | Final chorus |
| 57–60 | Lever flips, gates spring open, marbles spill into the void; loop | (no caption) | Wash returns into the opening chaos |

**Flags:**
- Keep it clearly original. Famous versions: Wintergatan's Marble Machine (hand-cranked wheel with programming
  pegs, vibraphone front) and Animusic's *Pipe Dream* (balls fired in arcs from tubes onto instruments). Ours: no
  crank, no peg wheel, no launching tubes; a gravity cascade with gates and a spiral lift, floating in a void.
- Void physics is pure fantasy (gravity only inside the machine; escaped marbles drift away weightless). No
  captions claim real physics.
- Same structure as #16 and #17 (layers lock in one by one → holdout → drop): it's the formula the user liked,
  but space these releases apart so they don't feel samey back to back.
- Brightness: keep the void vibrant, not black: saturated nebula colours at the playbook's lit level, marbles
  glowing.
- Hit density: ~30–40 marble hits/s once gates are shut, far more in the chaotic opening. Group dense hits so the
  parts stay clear.

**Feasibility:** no physics sim: each channel's marbles follow pre-made spline paths; overflow marbles are
pre-computed arcs with a few random bounces (no marble-to-marble collisions); ~1,000 instanced spheres; gates
are animated flaps; the lift is a rotating spiral. The void is cheap (gradient sky, dithered noise nebula, star
particles in parallax, floating low-poly shapes), its pulses driven by the same event list as the audio. The
machine is the main modelling job (tiers of bars, bowls, drum skins, pipes, the spiral ramp, the lift), mostly
repeated parts. Lock focus on the machine; let the void and nearby crystals go soft. Audio: one synth voice per
instrument type (mallet bars, glass bowls, drum skins, pipe bloops, cymbals); one event list drives marbles,
void pulses and sound.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01TqVLeD (2026-10-22) · Output: `marble_waterfall_organ.mp4`
- v1 delivered with caption + tags (early sheet sent first). Floating brass machine in a vibrant void; ~120 hits/s spray chaos -> gates shut on downbeats 6/10/14/18/20/24/32 s -> melody gate sticks 35-40 s (2 failed tries, half-beat silent gap) -> drop 40 s -> lever 57 s springs gates open into frame 0. Every note = a marble hit from one cached list (~33 hits/s in the full song). Checks: subject CoC 0 on all frames; audio -15.6 -> -10 dBFS build, marbles 13-30 dB over the bed; flash check 1 general flash (camera move), worst 1/s; seamless audio seam. Waiting for feedback
