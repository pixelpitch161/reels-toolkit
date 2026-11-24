# BALLFIGHT Studio's test runner on the owner's PC

The checks run on the owner's PC instead of GitHub's machines or a cloud container: a GitHub Actions runner on this
PC, registered to `pixelpitch161/reels-toolkit`, running as its own locked-down Windows user. It mirrors the
BallPhysicsEngine GPU runner (`F:\bpe-gpu`, same design: its own user, its own folder, the owner's drives locked) and
shares nothing with it.

What runs there once it is switched on:

- **`.github/workflows/studio.yml`** (every push or pull request that changes `studio/`): both jobs, the same steps as
  on GitHub's machines. Build files and the pinned ffmpeg are kept between runs, so a warm run builds only what
  changed. A pull request from a fork always runs on GitHub's machines, never on the PC.
- **`.github/workflows/pc-tests.yml`**: the tests a change names, with the PC's copy of the reference cache (so
  bf-harness's fighter checks run) and DirectX 12 (so the interface tests run). Put a line in the last commit's message
  before pushing:

  ```
  PC-Tests: -p bf-sim --test violin_quick
  PC-Tests: -p bf-app --test ui matchup::
  ```

  Each line is what follows `cargo test --release --locked` in `studio/`. Or start it by hand: Actions, `pc-tests`,
  Run workflow, with the lines in `tests` separated by `;`. The result is on the run's page.

## The switch

The repository variable **`BF_PC_RUNNER`** (GitHub: Settings, Secrets and variables, Actions, Variables) decides:

- `on`: both workflows run on the PC. If the PC is off, the jobs wait in GitHub's queue until it is on again (GitHub
  gives up after 24 hours).
- anything else, or not set: `studio.yml` runs on GitHub's machines as before, and `pc-tests` fails with "the PC
  runner is switched off".

A manual run of `studio` can also choose GitHub's machines ("Run on GitHub's machines").

## Setting it up (once)

Claude runs this through a Remote Control session; the owner clicks Yes on the Windows prompt and pastes one token.

1. On github.com open `pixelpitch161/reels-toolkit`, **Settings, Actions, Runners, New self-hosted runner**, pick
   **Windows**. In the "Configure" box, copy the token after `--token` (the page shows it; it works for an hour).
2. In PowerShell, from the repository folder:

   ```powershell
   Start-Process powershell.exe -Verb RunAs -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-NoExit','-File','F:\!Ai Stuff\InstrumentBattleMachine\tools\pc-runner\setup-pc-runner.ps1','-Step','All'
   ```

   Click Yes on the Windows prompt, paste the token into the new window and press Enter. The rest needs nobody: it
   makes the user, copies the tools, registers the runner, locks the drives (about 15 minutes on F:), installs Rust
   and runs a trial build (the first build is long). Leave the window open until it says `=== Done ===`.
3. Set the repository variable `BF_PC_RUNNER` to `on` (above).
4. Recommended for a public repository with a runner on a PC: Settings, Actions, General, "Approval for running fork
   pull request workflows from contributors": **Require approval for all external contributors**.

Each step can be run on its own with `-Step <name>` and is safe to run again; see the top of `setup-pc-runner.ps1`.

## What it makes

| Thing | Where |
| --- | --- |
| Standard (not admin) Windows user `bf-runner`, hidden from the sign-in screen, background logons only | Windows accounts |
| Everything the runner uses or writes | `F:\bf-runner` |
| rustup and the Rust of `studio/rust-toolchain.toml` (GNU host, clippy, rustfmt) | `F:\bf-runner\rustup`, `F:\bf-runner\cargo` |
| A copy of the owner's WinLibs MinGW, and of `studio\reference\.cache` | `F:\bf-runner\mingw64`, `F:\bf-runner\reference-cache` |
| GitHub Actions runner `bf-pc-1`, labels `self-hosted, Windows, X64, bf-pc` | `F:\bf-runner\runner` |
| Windows service `actions.runner.pixelpitch161-reels-toolkit.bf-pc-1`, runs as bf-runner, starts with Windows | Services |
| Job checkouts / build files (one folder, 30 to 95 GB) / the pinned ffmpeg archive | `F:\bf-runner\work` / `target` / `ffmpeg-cache` |
| Administrator logs and the checked runner download | `F:\bf-runner\admin` |
| The trial run's logs | `F:\bf-runner\probe` |

Every job gets `RUSTUP_HOME`, `CARGO_HOME`, `CARGO_BUILD_JOBS=12` (4 of the 16 logical processors stay free for the
owner), `BF_PC_TARGET`, `BF_PC_FFMPEG_CACHE`, `BF_PC_REFERENCE_CACHE` and a `PATH` of cargo, MinGW and Git's tools
(the `.env` file in `F:\bf-runner\runner`, written by `-Step Runner`). One runner runs one job at a time.

## Who may change what

A job runs as bf-runner, so anything bf-runner can change must never be run by the owner or as administrator.

| Folder | bf-runner may |
| --- | --- |
| `F:\bf-runner` itself (owned by Administrators, takes nothing from the drive) | look inside; not delete, rename or add folders |
| `setup`, `mingw64`, `reference-cache` (owned by Administrators) | read and run only |
| `admin` | nothing |
| `logs`, `probe`, `rustup`, `cargo`, `runner`, `work`, `target`, `ffmpeg-cache`, `scratch`, `downloads` | everything |
| everything else on D:, E: and F: (the owner's checkout, music, videos, `F:\bpe-gpu`) | nothing |

## Secrets

None are stored on disk by these scripts. The bf-runner password is random, made fresh on every administrator run
and handed straight to Windows; nobody needs to know it. The registration token is typed into the administrator
window (hidden), used once, and expires in an hour. The runner keeps its own GitHub credentials in
`F:\bf-runner\runner` (`.credentials`, `.credentials_rsaparams`), as every runner does.

## Known differences on the runner

- `bf-pack`'s `install::tests::a_stopped_update_is_taken_back_while_the_new_exe_runs_at_its_place` can fail while
  the PC is busy ("Close BALLFIGHT Studio first." at one of its 28 stop points: a just-killed test program's file is
  still held for a moment). It failed twice in the first runs, both slow ones (27 and 36 s), then passed seven times
  in a row on the runner (about 18 s each, Nov 23, 2026). Rerun it alone before calling it a failure.
- bf-harness's fighter checks for bagpipes, bari_sax, synth, turntables and viola fail here as in the owner's
  checkout: their git-ignored cache files are missing (docs/studio/HANDOFF.md).
- Timing tests (the `bf-app` update, search and preview ones) can miss their deadline while the PC is busy. On the
  PC, the `studio` checks run up to 5 failed app tests again alone and pass with a warning naming each when they pass
  alone; elsewhere, rerun them alone with a `PC-Tests:` line before calling them failures. `engine::search::tests::a_filter_change_then_a_new_fight_ends_the_old_search`
  timed out in the first full run and passed alone.

## Common fixes

- **The reference cache changed** (a new export): run `-Step Copy` again.
- **Runner shows offline on GitHub:** as administrator, `Restart-Service actions.runner.pixelpitch161-reels-toolkit.bf-pc-1`,
  or run `-Step Runner` again.
- **GitHub deleted the runner after 14 days offline:** delete `F:\bf-runner\runner\.runner`, get a new token and run
  `-Step Runner` again.
- **Disk:** the build folder is `F:\bf-runner\target`; deleting it (when the runner is idle) only makes the next run
  slow. Keep at least 150 GB free on F: (docs/studio/HANDOFF.md).
- **Switch it off for a while:** set `BF_PC_RUNNER` to `off`. To stop the PC taking jobs at all, stop the service.
- **Remove everything:** as administrator, `F:\bf-runner\runner\config.cmd remove --token <removal token>` (GitHub's
  runner page gives one), then `Remove-LocalUser bf-runner` and delete `F:\bf-runner`. To take the drive lock off:
  `icacls D:\ /remove:d bf-runner` (and E:\, F:\).
