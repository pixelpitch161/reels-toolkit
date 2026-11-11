# Milestone 6 (packaging): measurements

Numbers measured on the owner's PC for the packaging milestone. Sections 1, 3 and 4 are written by the tasks that
measure them (Tasks 4, 12 and 19).

## 2. The package

Measured on 11 Nov 2026 (Task 11) with `ballfight-pack package --exe studio\target\release\ballfight-studio.exe
--out <a scratch folder>`, from `lane/app-c` at version 0.1.0, the release exe built from that checkout, and the
pinned ffmpeg 9.0.2 from the PC's cache (`%LOCALAPPDATA%\BALLFIGHT Studio\data\downloads\9.0.2-759d0a9831c4`).
Making the package took 5 s (it reads and checks every file, then writes and checks the copies). The tree has no
`studio/packaging/guide` yet (Task 17), so the package holds no `guide\` folder.

| File | Bytes |
|---|---:|
| `ballfight-studio.exe` | 141,795,173 |
| `ballfight.ico` | 7,887 |
| `ffmpeg\ABOUT.txt` | 1,027 |
| `ffmpeg\LICENSE.txt` | 35,147 |
| `ffmpeg\ffmpeg.exe` | 227,823,104 |
| `ffmpeg\ffprobe.exe` | 227,621,376 |
| **Total (6 files, plus `MANIFEST.txt`)** | **597,283,714** (about 570 MB) |

- `MANIFEST.txt` itself is 602 bytes; the install adds `update.toml` (under 1 KB).
- The exe is a release build with line tables (`[profile.release] debug = 1`); ffmpeg and ffprobe are 76 % of the
  package. Without ffmpeg (what a download route would fetch on each update, D1 B) the package is about 142 MB.
- A trial install of this package under a scratch folder (`install --root`, its source copy cloned from the lane's
  local `origin`, no network) took 14 s in all and left 722 MB there (the program folder plus the source copy).
