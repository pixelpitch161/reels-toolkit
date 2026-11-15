# Theme categories and recommendations (BALLFIGHT Studio)

Short design, 2026-11-15. Requested by Aedan in the project thread.

## Intent

The theme picker is one flat list of 26 themes. Aedan wants the themes grouped into
categories, and wants Studio to recommend themes and a tempo for whichever two fighters are
on screen, **based on the instruments**, so a good fit is quick to find.

## Goals

- Group the themes into categories in the theme picker. The picker appears on the Match-up
  screen's bottom strip and on the Music screen, and both use the same dropdown.
- Mark the themes recommended for the current pair.
- Show a recommended tempo for the pair and the chosen theme.
- New fighters and themes slot in by giving them their own traits, with no need to revisit
  every other entry.

## Non-goals

- No in-app trait editor. Traits live in the content data.
- No learning from past episodes.
- Recommendations never hide a theme. The pair-rule filter (ruling M3-R2) stays the only
  thing that removes themes.

## Anti-goals

- Picking a theme or a pair must never silently change the tempo or the fight. A
  recommendation is a suggestion, and Aedan applies it.
- No colour used for the "recommended" state (Ball Studio: colour belongs to things, not
  states).

## Approaches considered

1. **Hand-made fit list.** Each theme lists the instruments it suits. Exact, but every new
   fighter means editing all 26 themes.
2. **Tags with a score.** Fighters and themes share a small trait vocabulary, and Studio
   scores the themes for the pair. **Chosen by Aedan** (decision card, 2026-11-15) because it
   grows with the roster.
3. **Learn from past episodes.** Too few episodes to stand alone.

## Design

### Categories (draft, Aedan to adjust)

| Category | Themes |
|---|---|
| Jazz & Latin | Hot jazz, Noir jazz, Bossa noir, Mambo, Dark tango, Second line |
| Folk & World | Balkan fire, Sea shanty, Spaghetti western, Big top |
| Cinematic & Dark | Baroque storm, Orchestral drill, Brass trailer, Villain march, Gothic horror, Heist, Dark drumline |
| Rock & Punk | Stadium rock, Prog duel, Ska punk |
| Electronic & Club | Brass house, Dark disco, Dark breakbeat, Drift phonk, Horror trap, Industrial |

Each `ThemeCard` gets a `category` field.

### Traits (shared vocabulary)

The new fields are named `traits`. Fighter cards already have caption `tags`, which are a
different thing.

- **Sound:** brassy, reedy, airy, bowed, plucked, keys, electric, percussive
- **Register:** high, low
- **Flavour:** folk, jazz, classical, rock, club, sinister

Each fighter card gets 3 to 5 traits and an **energy** from 1 to 3. 1 is heavy and stately
(tuba, contrabass) and 3 is fast and bright (flute, violin). Each theme card lists the traits
it likes, each weighted 1 or 2, plus a **tempo range** in bpm, seeded from its preset tempos.
The builder drafts all of these. "Close enough for the eye test" is fine.

### Scoring

- A fighter's fit for a theme is the sum of the weights of the traits they share.
- A pair's score is twice the weaker fighter's fit plus the stronger fighter's fit. This
  favours themes that suit both instruments over themes that suit one very well.
- A theme that is a fighter's home theme (its series episode) gets a bonus big enough to put
  it in the top three.
- The **top three** playable themes are "Recommended". Ties go to category order.

### Recommended tempo

Average the two fighters' energy and place the result in the chosen theme's tempo range:
energy 1 lands at the low end and energy 3 at the high end. Then snap to the nearest whole bar
count in the theme's `bars` range, using the existing `nearest_bars`.

### What Aedan sees

Theme dropdown:

```
Theme  [ Dark tango            v ]
        ★ Recommended
          Dark tango
          Noir jazz
          Gothic horror
        Jazz & Latin
          Hot jazz
          ...
        Folk & World
          ...
```

The recommended themes appear twice: in the Recommended group and under their own category.
Headings are muted and can't be selected. The ★ is a glyph, not a colour.

Tempo row:

```
Tempo  [====|====o======]  128.00 bpm   Recommended 132 bpm  [Use]
```

There is a small tick on the slider at the recommended bar count. "Use" sets the tempo to that
bar count, and the button is hidden when the tempo already matches.

## Left to the builder

- Exact trait lists, weights, energies and tempo ranges for the 30 fighters and 26 themes.
- The home bonus size, and how ties break within a category.
- One shared grouped-dropdown widget, used by both screens.
- Unit tests: every theme has a category, every fighter has traits, the top three are always
  playable for the pair, and the recommended tempo is inside the `bars` range.
