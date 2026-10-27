# 45. BALLFIGHT: Piano vs Organ

Board row: `docs/IDEAS.md` (#45). Series rules: `docs/ballfight/RULES.md`. Make it with the episode kit
(`projects/ballfight/README.md`, "Making an episode").

## 45. BALLFIGHT: Piano vs Organ — APPROVED
**Direction (user, Oct 27):** "Draft Piano vs Organ"; approved with mood A (prog-rock keyboard duel). The **debut of
PIANO v2** (PIANO ROLL, chosen the same day: `docs/ballfight/redesign-2026-10-24.md`, "PIANO v2"). All BALLFIGHT series
rules (VS title card without an episode tag, 3·2·1, ability-name captions, HUD ability list, colours, honest tuned
fight, 61.0 s, loop point = file boundary), the crossover rules for the ORGAN side (exactly as built in #35; the only
per-episode lever is a per-fighter damage handicap) and the PLAYBOOK's consistent loudness from frame 0.
**Rivalry:** the same keyboard, two different beasts: the living room vs the cathedral, hammers on strings vs air
through pipes; prog rock's "keyboard duel" as a fight. Post caption: "Same keys, different monster 🎹⛪ Comment the
next match-up 👇"

| | PIANO (blue, left), v2 | ORGAN (wine, right), from #35 |
|---|---|---|
| Style | Ranged pattern-shooter | Ground control |
| Signature | PIANO ROLL: a punched-paper roll scrolls along the edge of the ring; each row of holes fires note-shots in patterns that are the piano's melody | ERUPTING PIPES: on the beat a line of organ pipes bursts up from the floor toward the enemy |
| Grows per landed hit | More holes per row (denser, capped for readability) | Line length |
| LV2 | SUSTAIN: the next rows' notes hang in the air for a beat before flying (mines) | CATHEDRAL (echoing dome shield) |
| LV3 | GLISSANDO: a sweeping arc of notes | PULL ALL STOPS (twice as fast) |
| Below 25 HP | 88 KEYS: the roll runs off its spool; every hole fires at once | TOCCATA (a forest of pipes erupts) |

- **Likely shape (only if the honest fight gives it):** the piano fights from range, the organ's pipe lines chase it
  across the floor; CATHEDRAL soaks note-shots, but SUSTAIN's hanging mines wait out the shield. Finish: keys filling the
  air vs pipes filling the floor.
- **Sides:** first named = left: PIANO left, ORGAN right (the organ moves from its usual left corner for this episode).
- **Colours:** PIANO blue (84, 156, 255) vs ORGAN wine (196, 52, 104).
- **Music, mood A (prog-rock keyboard duel):** a new kit mood `kit/moods/prog_duel.py`: dramatic minor 70s-prog groove
  with odd accents (an occasional 7/8-feeling push inside 4/4 bars so the loop math holds), big drums and a driving
  bass guitar; the bed has no keyboards of its own. The **piano plays the melody** (the roll's holes ARE its line: what
  you see is what you hear), the **organ plays big chords and the bass pedals** (each pipe line = a chord stab + pedal
  note); the two trade lines like a keyboard battle, the leader fuller and louder, the loser thinner and dirtier.
  `loop_bpm(132)` = 133.77 BPM (34 bars; bar = 1.794 s, FIGHT! at 3.59 s). Consistent loudness from frame 0. The winner's
  solo over the groove. (Not chosen: B dark synthwave; C sinister ragtime.)
- **VS title card:** PIANO / VS / ORGAN: a hammered low chord, the thunder crack, a full-organ blast; panel patterns:
  keys + punched roll (blue-lit) vs organ pipes (reuse #35's, wine-lit).
- **Beat sheet (61 s):** bar 0 VS card · bar 1 count-in · FIGHT! at 3.59 s · 8–30 s the four unlocks (each named once;
  the HUD list lights on every use) · 32–44 s 88 KEYS or TOCCATA · 44–52 s final exchange · K.O. ~49–53 s (slow-mo
  shatter: keys / hammers vs pipes) · winner's solo + "COMMENT THE / NEXT MATCH-UP" · 58–61 s reassembly, doors shut,
  title slam = frame 0.
- **Flags:**
  - **PIANO v2:** the new roster version + the `roll` weapon kind + the changed abilities (SUSTAIN mines, 88 KEYS), own
    commits; #27 keeps v1 and must replay identically. New kit module `kit/fighters/piano.py` (look: cream punched paper,
    brass read bar, holes flashing blue as they fire, key-shaped note-shots with short trails, the roll curling off its
    spool in 88 KEYS; sound: the piano voice from the bridge). **Send `python -m ballfight preview PIANO --vs ORGAN`
    to the user before tuning.**
  - **ORGAN** exactly as built in #35, ported into `kit/fighters/organ.py` (check against #35 with the hash and a few
    frames); balance with the per-fighter handicap (40–60 % wins).
  - The roll is a thin strip at the ring's edge: never over the action, the pipes or the HUD; inside the safe zone if
    it carries anything readable.
  - Engine kinds additive, own commits; all earlier episodes replay identically. Honest seed search with `t_fight` =
    2 bars and the usual hard windows. No full-screen flashes; the kit's `check` (safe zone, focus, levels, band,
    flash, loop) must pass.
- **Effort:** ~60–75 min (two fighters into the kit, the roll mechanic, the new mood). Episode folder:
  `projects/ballfight_piano_vs_organ/` (`python -m ballfight new piano_vs_organ PIANO ORGAN ...`).

## Board history

- Approved Oct 27 (ideas chat).
