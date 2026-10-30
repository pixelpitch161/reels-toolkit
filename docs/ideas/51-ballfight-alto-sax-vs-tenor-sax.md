# 51. BALLFIGHT: Alto Sax vs Tenor Sax

Board row: `docs/IDEAS.md` (#51). Series rules: `docs/ballfight/RULES.md`. Make it with the episode kit
(`projects/ballfight/README.md`, "Making an episode").

## 51. BALLFIGHT: Alto Sax vs Tenor Sax — APPROVED
**Direction (user, Oct 29):** "Draft Alto Sax vs Tenor Sax"; approved with mood C (dark disco); "Have the session remake
the alto sax model or at least reference it against photos using chrome". Two new fighters with their own designs (the
retired generic SAX's attacks aren't inherited). All BALLFIGHT series rules (VS title card without an episode tag, 3·2·1,
ability-name captions, HUD ability list, colours, honest tuned fight, 61.0 s, loop point = file boundary) plus
consistent loudness from frame 0.
**Rivalry:** the sax family feud: the alto (most players' first sax, the 80s pop-solo star) vs the tenor (the jazz giant,
the original rock 'n' roll honker). Post caption: "Alto or tenor? Sax players, settle it 🎷 Comment the next match-up 👇"

| | ALTO SAX (orchid, left), new | TENOR SAX (burnt orange, right), new |
|---|---|---|
| Style | The only light-wielder | The only fighter that uses the crowd |
| Signature | **SPOTLIGHT SOLO**: every bar a spotlight beam swings down from the rafters and sweeps the floor where the alto aims it; anything caught in the beam burns (new kind `spotlight`) | **CROWD PLEASER**: every bar it strikes a pose and honks, a hype meter fills, and the crowd hurls stuff at the enemy from the stands (tomatoes, roses, foam fingers; arcs from the edges of the frame) (new kind `crowd`) |
| Grows per landed hit | Beam width + burn | Hype (more throws, thrown harder) |
| LV2 | OCTAVE KEY: two beams at once, an octave apart | WALK THE BAR: struts down a spotlit runway across the ring, honking |
| LV3 | ALTISSIMO: the beam narrows to a white-hot line and burns fast | SUBTONE: the crowd hushes and leans in, then throws twice as hard |
| Below 25 HP | ENCORE: the spotlight sweeps nonstop and every phone light in the stands joins in | STAGE DIVE: leaps into the crowd, which launches it back like a cannonball |

- **Likely shape (only if the honest fight gives it):** a show fight: one owns the lights, the other the crowd. The
  throws come from the edges, so the middle is the alto's safe zone, but its spotlight wants the tenor there. Finish:
  ENCORE vs STAGE DIVE.
- **Colours (in the table):** ALTO SAX orchid (200, 80, 240), TENOR SAX burnt orange (230, 110, 30); strong contrast
  (orchid between the VIOLIN's violet and the ACCORDION's magenta, burnt orange near the DRUMS' orange; none in this fight).
- **Music, mood C (dark disco):** new kit mood `kit/moods/dark_disco.py`: minor four-on-the-floor, octave-jumping bass,
  open hi-hats on the off-beats, string stabs and a filtered pad, a mirror-ball shimmer in the mix; no saxes of its own.
  The **alto plays the bright lead**, the **tenor the husky, honking low line**; the saxes are the disco leads; leader
  fuller and louder, loser thinner and dirtier; the winner's solo over the groove. `loop_bpm(118)` = 118.03 BPM (30 bars;
  bar = 2.034 s, FIGHT! at 4.07 s). Consistent loudness from frame 0. All original. (Not chosen: A dark synthwave; B dark
  R&B rock 'n' roll.)
- **Scene touch (optional, cheap):** a mirror ball over the ring throwing small light specks (local, slow; never a flash).
- **VS title card:** ALTO SAX / VS / TENOR SAX: a bright alto squeal, the thunder crack, a tenor honk.
- **Beat sheet:** series standard (bar 0 card · bar 1 count-in · FIGHT! at 4.07 s · unlocks 8–30 s · desperation 32–44 s ·
  K.O. ~49–53 s · solo + "COMMENT THE / NEXT MATCH-UP" · reassembly into frame 0).
- **Flags:**
  - **ALTO SAX model: the user wants it remade, or at least checked against photos using Claude in Chrome.** Look up
    real alto sax photos (front / side / 3/4; e.g. Wikimedia Commons), compare them with `altosax60_lowpoly`
    (proportions, neck curve, key cups, bell size and flare, the U-bend), and rebuild it as a new file if it's off;
    don't edit the old file (the retired SAX in #29 and #26 use it). Voice: the bridge's `SAX` voice is fine unless
    it sounds wrong next to the tenor. New roster entry `ALTO SAX`; the retired `SAX` entry stays untouched.
  - **TENOR SAX model: build it from reference photos found with Claude in Chrome** (bigger than the alto, the neck's
    characteristic hump, a bigger bell), as a new file, own commit; a husky, honking voice, own commit; bridge entry.
  - New kinds `spotlight`, `crowd` (+ the runway, the stage-dive launch) additive, own commits; every earlier episode
    replays identically. The crowd's throws must clearly come from the stands; the spotlight dims the arena only
    locally. (No preview approval: the user dropped that step on Oct 30; see RULES.) Re-tune for 40–60 % wins; honest
    seed search with `t_fight` = 2 bars. Kit `check` must pass.
- **Effort:** ~60–75 min (+ the alto model check / rebuild). Folder: `projects/ballfight_altosax_vs_tenorsax/`.

## Board history

- Approved Oct 29 (ideas chat).
- Oct 29: in production (session_01CzTfqcfepqv4wWSk4ryE2g).
- Oct 29: delivered for review (`ballfight_altosax_vs_tenorsax.mp4`). Alto model checked in Chrome against Commons' Amati family shot and a Selmer Series III alto: off (bell too small / high, neck too tall, keys as lines that vanish at ball size) -> rebuilt as `altosax_lowpoly.py` (altosax60 untouched); tenor built from the YTS-62 + Amati shots (`tenorsax_lowpoly.py`) with a husky voice. Previews sent before tuning. Tuned 53 % / 47 %, median K.O. 50.2 s (scale 0.75); seed 621 (ranked 2nd of 8 fits, picked for STAGE DIVE 41.2 s then ENCORE 45.2 s both captioned); ALTO SAX wins at 49.18 s on 2 HP. All checks + verify pass; every earlier episode replays identically.
