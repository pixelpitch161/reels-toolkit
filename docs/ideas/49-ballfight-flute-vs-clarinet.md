# 49. BALLFIGHT: Flute vs Clarinet

Board row: `docs/IDEAS.md` (#49). Series rules: `docs/ballfight/RULES.md`. Make it with the episode kit
(`projects/ballfight/README.md`, "Making an episode").

## 49. BALLFIGHT: Flute vs Clarinet — APPROVED
**Direction (user, Oct 29):** "Draft Flute vs Clarinet"; approved with mood A (minor bossa noir). A crossover of two
fighters already in the kit: FLUTE (#32 / #41) and CLARINET (#37 / #46). All BALLFIGHT series rules (VS title card
without an episode tag, 3·2·1, ability-name captions, HUD ability list, colours, honest tuned fight, 61.0 s, loop point
= file boundary), the crossover rules (fighters exactly as in `fighters.py`; the only per-episode lever is a per-fighter
damage handicap, 40–60 % wins) and consistent loudness from frame 0.
**Rivalry:** the band-room classic: the two biggest sections in every school band, side by side, each sure it's the real
melody. Post caption: "Flutes or clarinets? Band kids, choose 😤 Comment the next match-up 👇"

| | FLUTE (silver, left) | CLARINET (lavender, right) |
|---|---|---|
| Signature | SNAKE CHARMER (baskets, up to 2; cobras strike on the beat) | TWELFTH JUMP (teleports beside the enemy on the beat; the zip line cuts) |
| Grows per landed hit | Cobra reach + speed | Jump distance |
| LV2 | FLUTTER TONGUE (cobras strike twice as fast) | SQUEAK (0.5 s stun) |
| LV3 | SNAKE DANCE (cobras chase for a bar) | CLARION (jumps twice as often) |
| Below 25 HP | PICCOLO (piercing ring-wide note) | RHAPSODY (five rising jump-cuts) |

- **Likely shape (only if the honest fight gives it):** teleporter vs trapper: jumps that land next to a basket get bitten,
  but the clarinet can jump past the cobras to the flute; SQUEAK stuns the flute while its cobras keep striking; SNAKE
  DANCE answers a target that won't stay put.
- **Sides:** FLUTE left, CLARINET right (both their usual sides).
- **Colours:** silver vs lavender are both pale (as silver vs mint in #41): check on the early sheet that they read
  apart; if not, a stronger edge glow on each ball, never a colour change.
- **Music, mood A (minor bossa noir):** new kit mood `kit/moods/bossa_noir.py`: a sly, smoky minor bossa groove (nylon
  guitar comping in the bossa rhythm, brushes + rim clicks, soft upright-style bass, a dark Rhodes pad); no woodwinds of
  its own. The **flute plays the high melody**, the **clarinet the liquid counter-line** below it, trading the lead as it
  changes hands; leader fuller and louder, loser thinner and dirtier; the winner's solo over the bossa. `loop_bpm(132)`
  = 133.77 BPM (34 bars; bar = 1.794 s, FIGHT! at 3.59 s). Consistent loudness from frame 0 (bossa is soft by nature:
  keep it at the series' level with density and the master, not by playing it quietly). (Not chosen: B klezmer; C minor
  afro-house.)
- **VS title card:** FLUTE / VS / CLARINET: a flute flurry, the thunder crack, a clarinet gliss.
- **Beat sheet:** series standard (bar 0 card · bar 1 count-in · FIGHT! at 3.59 s · unlocks 8–30 s · desperation
  32–44 s · K.O. ~49–53 s · solo + "COMMENT THE / NEXT MATCH-UP" · reassembly into frame 0).
- **Flags:** both fighters are in the kit; the only new piece is the mood. If the engine doesn't define a teleport landing
  inside a cobra's reach (struck on arrival or not), use what it does and note it; don't change either fighter. Kit
  `check` must pass; every earlier episode replays identically.
- **Effort:** ~25–35 min. Folder: `projects/ballfight_flute_vs_clarinet/`.

## Board history

- Approved Oct 29 (ideas chat).
