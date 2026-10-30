# 52. BALLFIGHT: Oboe vs Bassoon

Board row: `docs/IDEAS.md` (#52). Series rules: `docs/ballfight/RULES.md`. Make it with the episode kit
(`projects/ballfight/README.md`, "Making an episode").

## 52. BALLFIGHT: Oboe vs Bassoon — APPROVED
**Direction (user, Oct 29):** "draft Oboe vs Bassoon"; approved with mood A (sneaky heist caper). A crossover: OBOE as
built in #37 (in the kit), BASSOON as built in #39. All BALLFIGHT series rules (VS title card without an episode tag,
3·2·1, ability-name captions, HUD ability list, colours, honest tuned fight, 61.0 s, loop point = file boundary), the
crossover rules (fighters exactly as in `fighters.py`; the only per-episode lever is a per-fighter damage handicap,
40–60 % wins) and consistent loudness from frame 0.
**Rivalry:** the double-reed family feud: the orchestra's only two double reeds, both forever carving reeds; the
bassoon is the oboe's big, sneaky older sibling (high and nasal vs low and comic). Post caption: "The double reeds
settle it 🪈 Comment the next match-up 👇"

| | OBOE (olive, left), from #37 | BASSOON (forest green, right), from #39 |
|---|---|---|
| Signature | TUNING A (tuning beam every bar; a hit knocks the enemy out of tune: weaker hits that audibly play flat; each 8th note stings) | TIPTOE (staccato sneak-hops, a stomp on each landing, double damage from behind) |
| Grows per landed hit | Detune depth | Hop distance + stomp size |
| LV2 | DOUBLE REED (snipping blades up close) | STACCATO (hops twice as fast) |
| LV3 | ENDLESS NOTE (the beam held for a whole bar) | BOCAL HOOK (hooks the enemy, spins it to face away) |
| Below 25 HP | ILL WIND (sour gust, fully out of tune) | MOUNTAIN KING (accelerating hops around the enemy, Grieg's theme) |

- **Likely shape (only if the honest fight gives it):** the beam must land on a stop-and-go hopper; once out of tune even
  a backstab hurts less; BOCAL HOOK also turns the oboe's aim away for a moment; a musical finish: MOUNTAIN KING played
  out of tune if ILL WIND lands first (the detune applies to the bassoon's part, including the Grieg quote).
- **Sides:** OBOE left, BASSOON right (both swapped from their own episodes; set for this episode only, don't edit the
  roster).
- **Colours:** olive vs forest green are both greens (olive lighter and yellower, forest dark and bluer): check on the
  early sheet that they read apart; if not, a stronger edge glow on each ball, never a colour change.
- **Music, mood A (sneaky heist caper):** new kit mood `kit/moods/heist.py`: pizzicato strings, brushed drums + rim
  clicks, a minor walking upright-style bass, muted vibes stabs, tiptoe tension; no woodwinds of its own. The **oboe
  plays the high nasal melody**, the **bassoon the low comic staccato line** (its hops = its notes), a double-reed duet
  that starts arguing; leader fuller and louder, loser thinner and sour (the detune is audible); the winner's solo over the
  caper. `loop_bpm(118)` = 118.03 BPM (30 bars; bar = 2.034 s, FIGHT! at 4.07 s). Consistent loudness from frame 0 (a
  sneaky mood still at the series' level: density and the master, not playing quietly). MOUNTAIN KING: Grieg's
  public-domain theme in the episode's key, accelerating; nothing else quoted. (Not chosen: B klezmer; C dark synthwave.)
- **VS title card:** OBOE / VS / BASSOON: the oboe's A, the thunder crack, a bassoon blat.
- **Beat sheet:** series standard (bar 0 card · bar 1 count-in · FIGHT! at 4.07 s · unlocks 8–30 s · desperation
  32–44 s · K.O. ~49–53 s · solo + "COMMENT THE / NEXT MATCH-UP" · reassembly into frame 0).
- **Flags:** OBOE is in the kit. BASSOON: if the porting session hasn't ported it yet, port it here from #39
  (`kit/fighters/bassoon.py`, checked against #39 with the hash and a few frames) and note it in
  `docs/log/ballfight_studio.md` so it isn't done twice. Engine changes additive; every earlier episode replays
  identically. Kit `check` must pass.
- **Effort:** ~30–45 min. Folder: `projects/ballfight_oboe_vs_bassoon/`.

## Board history

- Approved Oct 29 (ideas chat).
