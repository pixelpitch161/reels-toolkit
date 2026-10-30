# #53 BALLFIGHT: Flute vs Sousaphone (session_01CzTfqa, Oct 30)

Episode-kit episode (`projects/ballfight_flute_vs_sousaphone/episode.py`), a viewer request. FLUTE from the kit as built;
SOUSAPHONE new.

- **Model:** `series/instrument-history/sousaphone_lowpoly.py`, from two Commons photos found with Claude in Chrome
  (Yamaha YSH-411, three-quarter front; Provo marching band, worn). Lessons: the wrap must END at the top (a 1.4-turn
  loop: the first try ended low and the bell sat on the wrap), the bell's lower rim overlaps the wrap's top, white
  fiberglass bell + brass wrap so it never reads as the TUBA. Bridge size x1.12, yaw 20.
- **Voice:** `sousa_tone`: the TUBA's brass synth made brighter (centroid ~820 Hz vs ~200), a 12 ms marcato attack, a
  tongued "tah", `blat` for the blasts.
- **Engine:** `drill` (+ `flash_spin`, `company_front`), recorded as DRILL. Fixes found in smoke tests: the drill
  target must be clipped inside the ropes (a 2 s prediction ran off the ring and pinned the marcher on a rope with
  zero-length sets); a sideways trample knock made the blast miss every time, so the trample knocks mostly forward
  (`trample_fwd`) and the blast is a cone (`spread`): blast hit rate 0 % -> ~38 %. Replay guard identical.
- **Kit:** `Sound.prepare()` + `music.line_override` (opt-in, for the planned obbligato; unused in the end; #49's
  audio bit-identical); DRILL in REC_KEYS; mood `dark_drumline` ported from #38 (bit-identical cadence / unison / roll).
- **Look fixes from the sheets:** the ghost sousaphones x1.9 (they read as red rings at ball size); the chalk line
  brighter; the blast drawn as a filled cone with arcs from its event over 0.45 s (the 0.25 s engine blast barely
  showed); the ghost wall fades through the K.O. instead of vanishing on that frame.
- **Tuning:** 200 seeds, scale 0.7, blast 12, trample 3.6: 52 % / 48 %, median K.O. 48.7 s. Search 511 runs, 8 fits;
  picked 146 (ranked 2nd) for COMPANY FRONT answering PICCOLO and landing the K.O.
- **Music:** C minor, the drumline bed; the sousaphone's footfalls are the bass line, the flute the melody; COMPANY
  FRONT quotes the Stars and Stripes trio (Mutopia's public-domain John Church piano score, bars 39-42) in E-flat.
  Obbligato dropped (user: IMSLP's download gate).
- **Checks / render:** all pre-render checks pass (one allowed dip at 58 s); render 330 s + encode 193 s; verify PASS.
