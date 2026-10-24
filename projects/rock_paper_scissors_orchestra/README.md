# Rock Paper Scissors Orchestra (IDEAS #22)

The three teams are the sections of an orchestra, chasing each other round a round opera stage. When a piece catches
the piece it beats, the caught piece joins the hunter's team, and the hunter's section plays. The twist: the moment one
team is wiped out, the winner is already decided, and it's never the team that did the wiping out (its own predator
is the only team left with it).

| team | beats | plays |
|---|---|---|
| ROCK (slate grey stones) | SCISSORS | timpani (+ taiko on every 4th step) |
| PAPER (ivory folded sheets) | ROCK | strings: pizzicato, with tremolo swelling in as the section grows |
| SCISSORS (steel blades, red handles) | PAPER | brass stabs |

## The engine (`rps.py`)

120 discs (40 per team) on a round stage (radius 3.4). Each piece steers toward the nearest piece it beats (a faint
pull from far away, strong up close) and away from the nearest piece that beats it (only inside `see_pred`; when it
is cornered it slides along the rim instead of into it), plus a smooth wander, damping and a speed cap. Discs collide
elastically with each other and the rim. A piece with a predator close by may sprint 30 % faster (the same rule for
everyone). A touch converts the caught piece; a short immunity after each catch keeps every catch readable. Catches
start at bar 1 (2.18 s), so the opening burst is only the three colour wedges fanning out.

Video time = sim time. Frame 0 is the clump at centre stage in three wedges, bursting outward. After the last catch
the winner swirls slowly round the stage; from 57.2 s every piece spirals into its frame-0 slot (assignment by
`linear_sum_assignment`), arriving at exactly 61.0 s and turning to face outward, so the last frame flows into the
burst at frame 0 (the reset is the boundary itself: only the colours change at the seam).

Balance (`episode.py`): over 102 logged runs the winners were SCISSORS 26, PAPER 23, ROCK 24. `seed_search.py`
ranks seeds by story (first wipe-out 32-42 s, last catch 46-54 s, a team crashing to <= 6 pieces and coming back,
stragglers, lead changes; `--stop N`, raw counts per seed in `check/raw_*.jsonl` for re-scoring). Fits are rare
(~3 %). Seed 60 was chosen: ROCK crashes to 5 pieces at 30 s, swells to 110 by eating SCISSORS, wipes them out at
38.5 s and so dooms itself; tiny PAPER eats the giant and wins at 46.6 s. Never edited.

## Build

    python sim.py                  # seed.txt -> sim.npz
    python audio.py                # -> audio.wav (+ section levels, sections-over-bed band check)
    python -m reelkit.sheet render.py test/sheet.png 0.6 1.7 5.5 14 24 31 38.7 42.5 47.2 52.5 58.6 60.9
    python -m reelkit.safezone render.py
    python focus_check.py          # stage + every token: CoC 0 on all frames
    python -m reelkit.loopcheck render.py audio.wav
    python -m reelkit.video render.py audio.wav rock_paper_scissors_orchestra.mp4

## Picture

A low-poly opera house seen from the upper balcony: round stage (plank boards, painted arena ring, gilded rim,
footlights), red velvet curtains with a gold-fringed valance, three tiers of gilded boxes with sconces and pawn
patrons, a crystal chandelier (the key light, real token shadows on the boards via a per-frame point mask), the
orchestra pit with music-stand lamps and a conductor beating time, and the stalls. Each catch: a scale punch and a
glow in the hunter's colour. The hook words light their team as they land; the winner does a "stadium wave" hop on
every bar and the chandelier blazes. UI: `reelkit.ui.share_bar` (names + leader lines) and impact captions (the hook
lands one word per beat, each in its team colour; the twist's last line lands on the low brass sting), plus a rule
legend (ROCK > SCISSORS > PAPER > ROCK) at 3.6-7.9 s.

## Sound

D minor at 110.16 BPM (`loop_bpm(110)`: 28 bars). Catches are quantised forward to the grid and step the hunter's
part (ROCK and SCISSORS on 8ths, PAPER on 16ths); section volume follows its headcount; a wiped-out section stops
dead; the doomed team stutters on 8ths, fewer and fewer. Bed: cello + bass drone (Dm | Bb | Gm | A), heartbeat timpani,
a low-string ostinato that tightens from 8ths to 16ths, dark pads. Story moments: wipe-out boom + held breath, low
brass sting (D A D Eb), ticking pulse and rising tremolo through the mop-up, full orchestra hit on the last catch, the
winner's dark fanfare (led by its section), a timpani roll and string swell into the burst at frame 0. Periodic buses
wrap tails across the seam.
