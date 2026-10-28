# BALLFIGHT series rules

Read this for every BALLFIGHT episode, together with the episode's own file in `docs/ideas/` and `docs/PLAYBOOK.md`. Rules added later override earlier ones where they differ (each says when and why). Design history of the v2 attack redesign: `docs/ballfight/redesign-2026-10-24.md`.

**Making an episode (Oct 27): use the episode kit.** A new episode is one config file (`episode.py`) run step by step with `python -m ballfight <step> <episode>`; fighters' looks and sounds live in `projects/ballfight/kit/fighters/`, moods in `kit/moods/`. A new fighter gets a `preview` clip sent to the user before tuning and rendering. How-to, commands and what goes where: `projects/ballfight/README.md`, "Making an episode". The rules below still hold; the kit applies the shared ones (card, count-in, beat sheet, HUD list, captions, loudness, loop).

## Series rules (first written for #29–#35; standing rules since)
**Direction (user, Oct 23):** #27 Guitar vs Piano "turned out amazing and I think will be a winner". Develop match-ups in
advance in case comments are slow, **with only new instruments in each fight** (no Guitar, Piano, Drums or Violin, and
no instrument in two fights), "so that way I can render many at once without confusion. Then, it will be easier to
decide new matchups with existing instruments since we'll know exactly how they fight". User approved all seven
(#29–#35) with the moods listed. Each fighter gets a fighting style no one else has: after these, 18 fighters with 18
styles, ready for crossovers and a tournament bracket.
- **VS title card intro (series standard from Oct 24; user on the #30 test: "That turned out great, add it to the
  rules").** Every episode opens with a fighting-game versus screen on bar 0, then the 3 · 2 · 1 on bar 1, FIGHT! on
  the bar-2 downbeat. Reference build: #30's `projects/ballfight_bagpipes_vs_accordion/title.py` (move it into
  `projects/ballfight/` in its own small commit the first time another episode needs it, so all episodes share one).
  - Bar 0: the frame splits diagonally into two panels in the fighters' colours (left fighter top-left, right fighter
    bottom-right) over the dimmed arena, each ball big and close; no episode tag (user, Oct 27). Name A slams in
    on beat 1 (frame 0), VS on beat 2 (cracks the divider), name B on beat 3: each from huge to size with overshoot,
    motion streaks, camera shake, dust burst, local light, and its own hit (A's instrument blast / a thunder crack /
    B's instrument blast) over big unison band hits; beat 4 holds (only VS pulses), the panels slide apart like doors
    along the crack, with a pickup fill.
  - Words at the biggest pixel-font scales that fit the safe zone (names in fighter colours with a thick dark
    outline, VS largest in white-gold; long names keep left of the button column); panels and effects fill the screen.
  - No top caption "A / VS B" any more (the card replaces it). The HUD stays hidden while the card is shut.
  - Loop: the title slam is the frame-0 reset. The doors slide shut over the last ~0.30 s in small steps, a ghost of
    name A lands exactly on frame 0's word (snap to frame 0's exact size), the slam's light / streaks / shake start the
    frame after frame 0, and a riser + whoosh ramps to full into the seam (#30: seam 0.9x render / 1.5x mp4, audio
    step 3.9 dB).
  - Flash: panels are never white; one doors-shut / doors-open pair; `reelkit.flashcheck` must pass.
  - The fight starts at 2 bars. Search seeds with `t_fight` = 2 bars from the start: moving an existing seed's fight a
    bar later does **not** replay the same run (float drift, chaos from ~30 s; #30 lesson), so re-run the search.
- **Fighter versions:** PIANO v2 = PIANO ROLL (user, Oct 27), from the next piano episode; #27 keeps v1. Details and
  other redesigns: `docs/ballfight/redesign-2026-10-24.md`.
- **Retired fighters (user, Oct 27): GUITAR and SAX** are scrapped for new episodes, "since we will be using more specific
  versions moving forwards". Draft future match-ups with specific instruments instead: guitars = ACOUSTIC (exists),
  ELECTRIC GUITAR, BASS GUITAR; saxes = BARI SAX (exists), ALTO SAX, TENOR SAX, SOPRANO SAX. Each specific version is a
  new fighter with its own design (it doesn't inherit the old one's attacks). Their roster entries stay in
  `fighters.py` untouched (#27 and #29 must keep replaying; the replay guard covers them), but no new episode uses them.
  Their colours (GUITAR red, SAX teal) are free for the specific versions.
- **Ability list in the HUD (series standard from Oct 25; user: "It can be a little confusing to know all the abilities
  of the instruments since only one can pop up at the top at a time … the bar with the health and lv could be extended
  downward to list each ability. They could start grayed out and become colored once they have unlocked the ability.
  The text could light up each time they use that ability.").**
  - Under each fighter's HP bar, its three abilities in small scale-1 text, aligned to that fighter's outer edge:
    "LV2 <unlock 1>", "LV3 <unlock 2>", "LOW <desperation>" (e.g. "LV3 HIGHLAND FLING").
  - States: **locked** = grey · **unlocked** = the fighter's colour (a quick glow sweeps across the name the moment it
    unlocks) · **in use** = the name lights up white-hot on a smooth glowing pill in the fighter's colour, then eases
    back over ~0.5 s; abilities with a duration (shields, haste) stay lit while active. LOW stays grey until it fires,
    then lights and stays coloured.
  - The list replaces the LV pips and the old status line (it shows both jobs). Names, HP numbers, bars and VS stay.
  - Size and place: the HUD grows ~18–20 px (internal) taller, upward; it still ends at the bottom of the safe zone
    (y 500 internal) and left of the button column. Nudge the ring up slightly if the taller HUD covers the action.
    Hidden while the VS title card is shut (as before). `reelkit.safezone` must pass.
  - Top captions unchanged: the first use of each ability still gets its impact caption (4 s rule); the list lights on
    every use, including the ones the 4 s rule keeps out of the captions.
  - **Applies from the next newly drafted episode** (user, Oct 25: no re-renders of #27–#35 just for this; re-renders cost
    too many tokens). Built by #36 (`b09f547`). Exceptions the user asked for: **#34** (re-rendered anyway for the new
    HORN) and **#35** get the list in their re-renders.
  - Build: an optional `abilities` argument on `reelkit.ui.fight_hud` (per fighter: tag, name, state, glow 0..1), in
    its own small commit; without it the HUD draws exactly as before, so #27 / #28 and older renders don't change.
    Rough mock (Oct 25, ideas chat): bars + names on top, the three rows under each bar.
- **Everything from #27 holds** (see #27 / #28 and `projects/ballfight/README.md`, "Making the next episode"): the
  count-in (the 3 · 2 · 1 is the band's count-in, one bar of the episode's tempo, now on bar 1 after the title card);
  mid-fight captions = **ability names only** (first use of each, never two within
  4 s; repeats and close calls go in the HUD status line), then "K.O." → "(WINNER) WINS" and "COMMENT THE / NEXT
  MATCH-UP"; status in small text inside the safe zone (`reelkit.ui.fight_hud`, HP numbers in the balls); post caption
  "Comment the next match-up 👇" + the two instruments' emojis; honest fight, never hand-edited; 61.0 s; loop point =
  file boundary (loser reassembles, both glide to corners, the doors shut and the title slam lands at 61.0 = frame 0).
- **Beat sheet** (every episode; the real fight decides after the count-in): bar 0 VS title card · bar 1 count-in ·
  FIGHT! on bar 2 · launch, first clashes · 8–30 s
  the four unlocks (each named once) · 32–44 s one fighter below 25 HP fires its desperation move · 44–52 s final
  exchange · K.O. ~49–53 s (hit-stop, slow-mo shatter into the instrument's parts) · winner's solo under the spot +
  "COMMENT THE / NEXT MATCH-UP" · 58–61 s reassembly into frame 0.
- **Music:** every hit plays the attacker's instrument in key, snapped to the nearest 16th in the audio; the leader's
  part gets fuller and louder, the loser's thins and dirties as its HP drops; the winner's solo is an original motif in
  its own idiom. Tempos via `loop_bpm` (whole bars in 61 s). Each episode has its own mood; none sugary.
- **Fighter colours (user, Oct 23):** every instrument has its **own unique ball colour**, "so it's not just red vs
  blue every time" (only Guitar and Piano keep red and blue). The colour is the fighter's identity across every
  episode and crossover: glass ball tint + rim, its corner light and the LED barrier on its side, HUD bar + name,
  ability captions, weapon / projectile glow, shatter particles. Corners are just **left** (first named) and
  **right** (second named); nothing in the scene stays red / blue for other fighters. Use these exact values in
  `fighters.py` (`color`; `dark` = a deeper shade of it, as #27 did), and don't reuse a colour for a new fighter:

| Fighter | Colour | RGB | | Fighter | Colour | RGB |
|---|---|---|---|---|---|---|
| GUITAR (retired Oct 27) | red, now KAZOO's | (255, 84, 72) | | PIANO | blue | (84, 156, 255) |
| DRUMS | orange | (255, 146, 40) | | VIOLIN | violet | (176, 98, 255) |
| TRUMPET | yellow | (255, 214, 60) | | SAX (retired Oct 27) | teal, now VIOLA's | (36, 200, 176) |
| BAGPIPES | tartan green | (64, 196, 88) | | ACCORDION | magenta | (236, 82, 214) |
| TURNTABLES | lime | (190, 245, 60) | | SYNTH | cyan | (70, 228, 255) |
| TUBA | indigo | (104, 92, 236) | | FLUTE | silver (cool) | (222, 232, 246) |
| UKULELE | pink | (255, 138, 178) | | ACOUSTIC | wood tan | (214, 160, 100) |
| TROMBONE | mint | (120, 240, 184) | | HORN | copper | (200, 112, 64) |
| ORGAN | wine | (196, 52, 104) | | HARP | pearl (warm ivory) | (244, 228, 190) |

| CLARINET | lavender | (190, 168, 255) | | OBOE | olive | (156, 176, 52) |
| BASS CLARINET | slate blue | (110, 132, 170) | | EUPHONIUM | amber | (240, 172, 32) |
| BASSOON | forest green | (30, 130, 76) | | BARI SAX | coral | (255, 118, 92) |
| VIOLA | teal (from the retired SAX) | (36, 200, 176) | | HARMONICA | sky blue | (128, 204, 255) |
| KAZOO | red (from the retired GUITAR) | (255, 84, 72) | | ELECTRIC | neon green | (100, 255, 120) |
| CELLO | rust | (196, 84, 40) | | CONTRABASS | steel blue | (70, 110, 200) |

  Spares (Cello, later fighters): pick a colour clearly apart from all of these and add it here.
- **Fighters:** first named = left corner (screen left), second = right. Weapon scaling: every landed hit grows the
  signature weapon (the column "grows"). Abilities: unlock1 at LV2, unlock2 at LV3, `low` once below 25 HP; existing
  ability kinds (`shockwave`, `shield`, `haste`, `arc`, `wave`, `slam`) are reused wherever they fit.
- **Tuning:** re-tune each pair once (`tune.py`: 40–60 % wins, ~50 s median K.O.), freeze the rules, then the
  early-stopping seed search with #27's hard windows (K.O. 49–53 s, >= 2 lead changes, winner HP <= 30, >= 5 ability
  captions >= 4 s apart). Rules take the episode's tempo (beat, `t_fight` = 1 bar).
- **Parallel-production rule (important):** all seven add new weapon / ability kinds to the shared engine
  (`projects/ballfight/fight.py`, `fighters.py`) at the same time. Keep engine changes **additive** (new kinds as new
  functions / branches, new roster entries; no rewrites of shared code), commit them **alone and small**, `git pull
  --rebase` right before and push right after, and confirm #27 (seed 88) and #28's chosen seed still replay
  identically. Draw functions for new weapon styles live in the episode's own folder where possible. Work in
  `projects/ballfight_<a>_vs_<b>/`. If #28's engine additions (`pulse`, blade spin growth) aren't pushed yet, pull
  them first.
- **Voices:** Pixel Pitch voices + models via `projects/which_instrument_wins/pp_bridge.py` (read-only use of the series).
  Already in the bridge: TRUMPET, SAX, BAGPIPES, ACCORDION, TUBA, FLUTE, UKULELE, TROMBONE, HORN, HARP. Not yet (models
  exist in `series/instrument-history/`): TURNTABLES (`turntables60_lowpoly`), SYNTH (`synth60_lowpoly`), ACOUSTIC
  (`acoustic_lowpoly`), ORGAN (`organ60_lowpoly`): add a bridge entry in its own small commit (don't edit the series).
- **No episode number on screen (user, Oct 27: "From now on, don't have the Ballfight # in the video").** New episodes
  show no "BALLFIGHT #N" tag anywhere in the video (title card, HUD or captions); the number may stay in post captions
  and on this board for tracking. Videos already delivered keep theirs (no re-renders for this).
- **Effort:** ~45–60 min each (one or two new weapon kinds each; ring, HUD, count-in, audio pipeline reused).

## Crossover rules (from #36, Oct 25)

- **Crossover rules (new; for this and every later crossover):** both fighters come straight from `fighters.py` as last
  built (weapon, numbers, abilities, look, Pixel Pitch voice and idiom; the rebuilt trombone model from #34 v4). **The
  only per-episode lever is a damage handicap per fighter**, tuned to 40–60 % wins (add a per-fighter damage scale to
  the engine if it only has the shared one: additive, own commit). Mechanics, reach, timing and abilities never change
  per episode, so "how they fight" stays true everywhere. If damage alone can't balance a pair, flag it to the user
  instead of changing a fighter. All earlier episodes must still replay identically (`tools/ballfight_replay_hash.py`).
