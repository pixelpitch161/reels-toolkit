# 50. BALLFIGHT: Trumpet vs Trombone

Board row: `docs/IDEAS.md` (#50). Series rules: `docs/ballfight/RULES.md`. Make it with the episode kit
(`projects/ballfight/README.md`, "Making an episode").

## 50. BALLFIGHT: Trumpet vs Trombone — APPROVED
**Direction (user, Oct 29):** "Draft Trumpet vs Trombone"; approved with mood A (big-band mambo). A crossover of two
fighters already in the kit: TRUMPET (#29 / #46) and TROMBONE (#34 / #41 / #48). All BALLFIGHT series rules (VS title
card without an episode tag, 3·2·1, ability-name captions, HUD ability list, colours, honest tuned fight, 61.0 s, loop
point = file boundary), the crossover rules (fighters exactly as in `fighters.py`; the only per-episode lever is a
per-fighter damage handicap, 40–60 % wins) and consistent loudness from frame 0.
**Rivalry:** the brass section's two loudest seats: the lead trumpet (the star: high notes, ego) vs the trombones (the
back-row clowns with the slides). Post caption: "Lead trumpet or trombone section? Choose wisely 🎺 Comment the next
match-up 👇"

| | TRUMPET (yellow, left) | TROMBONE (mint, right) |
|---|---|---|
| Signature | VALVE COMBO (three piston punches 1-2-3, close range) | SLIDE LANCE (thrust on every beat, long range; swats what it passes through) |
| Grows per landed hit | Piston reach | Lance reach |
| LV2 | FANFARE (shockwave) | SAD TROMBONE (shockwave + 1 s slow) |
| LV3 | DOUBLE TONGUE (combos twice as fast) | POWER SLIDE (thrusts on 8ths) |
| Below 25 HP | HIGH C (giant golden beam: its only long-range move) | PEDAL TONE (a giant bell drops and stomps) |

- **Likely shape (only if the honest fight gives it):** a pure reach battle: the lance outranges the pistons, so the
  trumpet must get inside; SAD TROMBONE keeps it out; once in, DOUBLE TONGUE is brutal; the finish flips it: HIGH C finally
  gives the trumpet range vs PEDAL TONE's stomp (the highest note in the section vs the lowest).
- **Sides:** TRUMPET left, TROMBONE right (both their usual sides). **Colours:** yellow vs mint.
- **Music, mood A (big-band mambo):** new kit mood `kit/moods/mambo.py`: a minor Latin big-band groove (piano montuno,
  congas + timbales + cowbell, bass tumbao, saxophone-section pad kept low; no trumpets or trombones of its own). The
  **trumpet plays the high lead** (its combos = shout-chorus stabs up top), the **trombone the punchy low riffs and smears**;
  leader fuller and louder, loser thinner and dirtier; timbale fills on the abilities; the winner's solo over the mambo.
  `loop_bpm(132)` = 133.77 BPM (34 bars; bar = 1.794 s, FIGHT! at 3.59 s). Consistent loudness from frame 0. All original
  (no quoting famous mambos). (Not chosen: B dark disco; C dark funk.)
- **VS title card:** TRUMPET / VS / TROMBONE: a trumpet blast, the thunder crack, a trombone smear.
- **Beat sheet:** series standard (bar 0 card · bar 1 count-in · FIGHT! at 3.59 s · unlocks 8–30 s · desperation
  32–44 s · K.O. ~49–53 s · solo + "COMMENT THE / NEXT MATCH-UP" · reassembly into frame 0).
- **Flags:** both fighters are in the kit; the only new piece is the mood. Kit `check` must pass; every earlier episode
  replays identically.
- **Effort:** ~25–35 min. Folder: `projects/ballfight_trumpet_vs_trombone/`.

## Board history

- Approved Oct 29 (ideas chat).
