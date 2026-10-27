# 12. Prime-Number Cicadas

Board row: `docs/IDEAS.md` (#12). Series rules: none.

## 12. Prime-Number Cicadas — APPROVED
- **Hook:** "13 × 17 = 221 YRS" (17 chars, in the first 2 s). Alt: "LAST TIME: 1803".
- **Science:** periodical cicadas spend 13 or 17 years underground, then emerge together in their billions.
  In 2024 Brood XIX (13-year) and Brood XIII (17-year) emerged in the same year for the first time since
  1803 (next: 2245). They only overlap in a thin strip of central Illinois (around Springfield). 13 and 17
  are prime, so they line up only every 13 × 17 = 221 years; non-prime cycles meet far more often (12 & 18
  every 36 years, 12 & 16 every 48; calc). From 1803 to 2024, XIX emerges 17 times and XIII 13 times; the
  closest near misses are 1854/1855 and 1972/1973, one year apart (calc). Why prime? Leading ideas: avoiding
  hybrids with other cycles and, more loosely, predators. Unresolved.
  Sources: [Wikipedia: Brood XIX](https://en.wikipedia.org/wiki/Brood_XIX),
  [UW–La Crosse](https://www.uwlax.edu/currents/cicadas-2024/),
  [KING 5 fact-check](https://rssfeeds.king5.com/~/881604386/0/king5/home~No-this-isn%E2%80%99t-the-first-time-since-that-two-broods-of-cicadas-emerged-at-once),
  [Toivonen & Fromhage 2020](https://pmc.ncbi.nlm.nih.gov/articles/PMC7319174),
  [Wikipedia: Decim cicadas](https://en.wikipedia.org/wiki/Decim_periodical_cicadas),
  [CBS Chicago](https://cbsnews.com/amp/chicago/news/chicago-periodical-cicadas-sound-loud).
- **Music:** 1 year = 1 sixteenth at 120 BPM, so 1803 → 2024 = exactly 221 sixteenths = 27.6 s. XIX hits
  every 13 sixteenths, XIII every 17: a genuine 13-against-17 polyrhythm over an original 4/4 groove, landing
  together only on 2024 (the climax). Voices: synthesised "Pharaoh" calls (pure-tone buzz with a pitch drop).
  The 13-year M. tredecim sings slightly lower (~1.0–1.25 kHz), so XIX = lower voice, XIII = higher. Chorus
  sits above the music in its band (same carve as crickets / dice).
- **Scene:** low-poly Illinois woodland at golden hour as a cut-away diorama: a soil cross-section below the
  trees with nymphs, roots and tunnels. Left half = XIX (warm orange), right half = XIII (teal). Low sun = key
  light, farmhouse porch lamp = warm local light; birds snatching cicadas and shed shells on bark for small
  life. Each emergence makes its side's trees erupt with red-eyed cicadas.
- **Payoff / ending:** 2024: both halves erupt at once, the sky fills, both choruses + groove hit together.
  Then eggs drop from twigs, nymphs burrow back into the soil, and it loops to the opening.

**Beat sheet (60 s):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–4 | Soil cut-away, two nymphs side by side, counter | 13 × 17 = 221 YRS | Soil rumble, one buzz |
| 4–8 | Both burst out, trees erupt | 2024: THESE 2 / LAST MET 1803 | Double chorus swell |
| 8–12 | Split view: left and right nymphs underground | 13 YRS vs 17 YRS | Two buzz pitches (low vs high) |
| 12–16 | 13 and 17 glow as numbers | BOTH PRIME | Ping |
| 16–20 | Ghost 12- and 18-year broods meet over and over | 12 & 18: MEET / EVERY 36 YRS | Muted, frequent clashes |
| 20–24 | Ghosts fade; 13 & 17 return | 13 & 17: / EVERY 221 | — |
| 24–35 | Year race from 1803; each side bursts on its cycle | 1803 → … | Groove + 13:17 polyrhythm |
| 35–40 | 1854/1855 near miss | SO CLOSE… / 1 YEAR APART | Hits flam a sixteenth apart |
| 40–46 | Counter runs on; bursts interlock | XIX: 17 TIMES / XIII: 13 TIMES | Polyrhythm thickens |
| 46–50 | 1972/1973 near miss | 1 YEAR… AGAIN | Flam |
| 50–52 | Climax: 2024, both halves erupt | 2024 | Both choruses + groove together |
| 52–60 | Eggs drop, nymphs burrow, loop to opening | TRY IT: CLAP 13 / TAP 17 | Thins to one buzz, then soil |

**Flags:**
- Wording: "these two broods: first time since 1803", NEVER "first time two broods emerged together" (some
  pair co-emerges every ~5–6 years).
- Skip the hype ("a trillion cicadas", "cicada apocalypse"): the broods barely overlap.
- "Why prime" is debated: caption "WHY PRIME? STILL DEBATED" or say nothing. No periodic predator is known,
  so don't state the predator theory as fact.
- 221 applies to any 13/17 pair, not just XIII and XIX; "13 × 17 = 221" is safe as stated.
- Pitch claim is soft: M. tredecim is documented as slightly lower, but each brood has several species. Treat
  lower/higher as a voicing choice grounded in that difference.

**Feasibility:** easy: low-poly cicadas (body, wings, red eyes) instanced by the thousand, trees, soil
cut-away with tunnels, year counter HUD. Medium: mass emergence as particles; precompute burst times from the
year list so one list drives visuals and audio (as on #2 and #11). Audio: synthesise the Pharaoh call (buzz +
pitch drop) and cassini clicks; hit density at 120 BPM sixteenths is fine. Fallback: one tree with two
colour-coded broods and the soil cut-away below, if the split woodland is too busy.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01UkvSDH (2026-10-21) · Output: `prime_cicadas.mp4`
- v1 delivered with caption + tags (early sheet sent first). Exact grid kept (1 yr = 1/16 at 120 BPM, 2024 on the 51 s downbeat), so near-miss captions moved to 29.75 s / 44.5 s. Waiting for feedback
