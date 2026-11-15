# Matchup history: design (short)

Date: 2026-11-15. Asked for and decided by Aedan in the project thread "Track matchups already rendered".

## Intent

BALLFIGHT Studio should remember every matchup it has made, including the episodes made before the app existed, so Aedan can see what has been made, what has been uploaded, and which pairings the Left fighter has already had on his channel. The record is the app's own memory, not a scan of the video folders: moving or deleting a video never changes it.

## Goals

- A permanent history of fights made, kept by the app.
- A new **Matchups** tab listing every fight made, with an **Uploaded** checkbox per row.
- On the **Match-up** screen, the fighter in the Left corner shows, on each other fighter's roster chip, how many times they have fought, counting **uploaded fights only**.
- The 29 earlier episodes (#27 to #56, from `/mnt/project-files/matchups/ballfight-matchup-tracker.xlsx`, sheet Episodes) are in the history from the first launch.

## Non-goals

- No syncing with the spreadsheet after the one-time import; the app's history is the record from now on.
- No reading of upload status from Instagram, TikTok or YouTube; the tick is manual.
- The Library tab is unchanged.

## Anti-goals

- Counts that drift because a video folder moved, was deleted, or the output folder changed.
- Test renders and redos inflating counts.

## Decisions (Aedan's)

- **One row per distinct fight.** Re-rendering the same fight (same fighters, same seed) updates that row's date instead of adding one; a new seed makes a new row. A selected row can be deleted (for stray test renders).
- **Chip counts use uploaded fights only.** The Matchups tab still lists every fight made.
- **Old matchups included**, imported from the spreadsheet.
- **Approach 1:** the history is a file in the app's data folder, beside its settings.

## Approaches considered

1. **History file in the app's data folder** (picked). Independent of the output folder; cheap; matches "the app remembers".
2. History file inside the output folder. Travels with the videos, but pointing the app at a new folder shows an empty history.
3. The spreadsheet as the record (app reads/writes .xlsx). Familiar file, but Excel locking the open file makes it fragile, and it adds a lot of code.

## Design

**The record.** `history.toml` (name is the builder's call) in the app's data folder (the same folder as the settings file, overridable by `--data-dir` like everything else there). One row per fight: left fighter, right fighter, seed (may be missing for old episodes), winner, date made, uploaded (true/false), and an optional note. Written with the same safe-save pattern the settings file uses. Pairing counts ignore sides: Flute vs Trombone and Trombone vs Flute are the same pairing.

**When a row is written.** When a render finishes successfully (the queue's job-done event). A cancelled or failed render writes nothing. If a row already has the same left, right and seed, its date is updated and its uploaded tick is kept.

**First launch / import.** The spreadsheet's 29 episodes are converted once, now, into a starting history that ships with the app and is copied in when no history file exists. All imported rows start **ticked as uploaded** (the spreadsheet's "review" status does not say otherwise); Aedan unticks any that did not go up. Rows with retired fighters (Guitar vs Piano, Trumpet vs Sax) stay in the list but never light up a chip, since those fighters are not on the roster. Fighter names in the spreadsheet (e.g. `TURNTABLES`) are mapped to the roster's fighter ids.

**Matchups tab.** A new tab next to Library, in the Ball Studio look:

```
Matchups              31 fights · 27 pairings · 4 not uploaded
Made         Left        Right       Winner     Seed   Uploaded
Nov 15       Flute       Trombone    Flute      2210   [ ]
Oct 24       Bagpipes    Accordion   Accordion  1442   [x]
...
```

Newest first. Fighter names carry their colour swatch (colour belongs to fighters). Clicking the checkbox saves at once. A footer for the selected row holds **Delete** (asks once before deleting).

**Match-up screen.** With a fighter in the Left corner, each roster chip of a fighter it has an uploaded fight with shows the count (e.g. "×2") at the chip's right edge, in `small` `MUTED` text; where the chip already shows "Left"/"Right", that word wins. Chips with no uploaded fight show nothing extra. No new colours for the state. The chip's accessible name gains ", fought 2 times".

## Left to the builder

- File format details and names, the conversion script for the spreadsheet, the exact tab widths and row height (follow the Library and Ball Studio tokens), the delete confirmation wording, tests.
- Calls I made that Aedan can overrule: swapped sides with the same seed count as a different fight (different row) but the same pairing; imported rows start ticked as uploaded; date made comes from the spreadsheet's "Date made" column.
