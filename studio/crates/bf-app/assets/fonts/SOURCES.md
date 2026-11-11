# Where the built-in fonts come from

Every file here was downloaded from the font's official repository at a pinned commit and checked with
`git hash-object <file>` against the blob id below. All five fonts are under the SIL Open Font License 1.1; its text
travels with them: `OFL-IBM-Plex.txt` for the four IBM Plex files, `OFL-Noto-Emoji.txt` for Noto Emoji.

Noto Emoji is built in but not used by any font family: the pinned file has no harp (U+1FA89), so the app leaves
emoji to the PC's own fonts.

| File | Licence | Bytes | Git blob | SHA-256 |
|---|---|---|---|---|
| `IBMPlexSans-Regular.ttf` | SIL Open Font License 1.1 | 200,500 | `bd6817d5202895da5ac4fad88de3da71e652881a` | `975dcda37d80f038dcd143c22e33ca2d97a0cc5a929aace1c749153b0fe1afa5` |
| `IBMPlexSans-Medium.ttf` | SIL Open Font License 1.1 | 202,460 | `a3826c4e9ab3f2d6e64e0c9aea7122ea3431e174` | `331c8639d7598b2cde62a911a71db195e30cb655cd6bdf2e324a7e984955f907` |
| `IBMPlexSans-SemiBold.ttf` | SIL Open Font License 1.1 | 202,632 | `09ec8cdae8208c26165db0c87da409cda8c26ae4` | `a20caf8286023a6a7a85e40b1d2a4ae9fc3e3b1f9eda8f4c542dd4986af67bb1` |
| `IBMPlexMono-Regular.ttf` | SIL Open Font License 1.1 | 173,052 | `4254c37f20ba57e829a3315fdc91cda4e63589f5` | `7c6fbddca4b700be918f5f6183d9bd4464fa427fe435f0b480d77fe2bb8c5a43` |
| `NotoEmoji-Variable.ttf` | SIL Open Font License 1.1 | 1,982,596 | `c2c26ab612a88a8610ff9cfbb89299bf2aea6c7a` | `de6c18832938afc99caf132b39d6a30a19bac7f2e812e28db2535b4608d27551` |
| `OFL-IBM-Plex.txt` | the licence text | 4,360 | `f72f76504cd73cf2c00b140b2743c138320597b6` | `91c25c350d3cac39da2736d74f7ba37ef648f5237a4e330a240615bc8d8c4360` |
| `OFL-Noto-Emoji.txt` | the licence text | 4,330 | `d09d3d0e09169f6747754e6b25faf5ba5ef6402d` | `500bb1ccf43df7bbb522112f9133a52b16e1c35e809632f5d8609b179152de5b` |

URLs:

- `IBMPlexSans-Regular.ttf`:
  <https://raw.githubusercontent.com/IBM/plex/1da12f02587b630c07e92692d21492d722f53614/packages/plex-sans/fonts/complete/ttf/IBMPlexSans-Regular.ttf>
- `IBMPlexSans-Medium.ttf`:
  <https://raw.githubusercontent.com/IBM/plex/1da12f02587b630c07e92692d21492d722f53614/packages/plex-sans/fonts/complete/ttf/IBMPlexSans-Medium.ttf>
- `IBMPlexSans-SemiBold.ttf`:
  <https://raw.githubusercontent.com/IBM/plex/1da12f02587b630c07e92692d21492d722f53614/packages/plex-sans/fonts/complete/ttf/IBMPlexSans-SemiBold.ttf>
- `OFL-IBM-Plex.txt` (saved from `license.txt`):
  <https://raw.githubusercontent.com/IBM/plex/1da12f02587b630c07e92692d21492d722f53614/packages/plex-sans/fonts/complete/ttf/license.txt>
- `IBMPlexMono-Regular.ttf`:
  <https://raw.githubusercontent.com/IBM/plex/2f9ba1b25957d958db71a849e85d72e3ecfb845a/packages/plex-mono/fonts/complete/ttf/IBMPlexMono-Regular.ttf>
- `NotoEmoji-Variable.ttf` (saved from `NotoEmoji[wght].ttf`):
  <https://raw.githubusercontent.com/google/fonts/8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5/ofl/notoemoji/NotoEmoji%5Bwght%5D.ttf>
- `OFL-Noto-Emoji.txt` (saved from `OFL.txt`):
  <https://raw.githubusercontent.com/google/fonts/8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5/ofl/notoemoji/OFL.txt>
