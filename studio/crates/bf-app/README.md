# bf-app

The BALLFIGHT Studio desktop app: the window, the interface, the live preview with sound and the render queue. It is
drawn with egui through wgpu (eframe). The interface is a state machine that talks to the engine only through plain
commands and events, so it never waits on a fight, a picture, a soundtrack or a file.

`bf-app` is not an engine crate. Every number that reaches a fight, a frame, a soundtrack or a fight file comes from
the engine crates.

## Run the app

From `studio/`:

```sh
cargo build --release -p bf-app
```

The program is `target/release/ballfight-studio.exe`. Started with no option it is the real app on the real engine
(`engine::spawn`), with the 20 kit fighters and the 14 kit themes (it opens on FLUTE against TROMBONE):

- Its settings and queue are in `%APPDATA%\BALLFIGHT Studio\config` and its shortlist cache in
  `%LOCALAPPDATA%\BALLFIGHT Studio\cache` (`store::Paths::system`). They are read once, before the first frame;
  a file that could not be read becomes a notice and is kept aside, never written over. Videos go to the output
  folder in Settings (`Videos\BALLFIGHT` at first).
- The engine runs with the settings' worker threads (the PC's threads minus one at first), every frame of a video
  and no limit on a search. The PC's sound device plays the preview; without one the app says so and plays silently.
- At start the app asks the engine for the fighters, for ffmpeg (shown in Settings, with where it was found) and
  about the output folder.
- The interface's thread runs above the app's other threads on Windows (`app::interface_first`), so the preview's
  drawing threads and a render, which together can keep every core busy, do not keep the window waiting for a core.
- Closing the window stops the engine first and waits for it, a few seconds at most (`BfApp::close`,
  `app::ENGINE_STOP`): a video being rendered is cancelled, its ffmpeg is ended and its temporary folder removed,
  and it starts again from the beginning next time (it is back in the queue, which is paused and says so).

The options:

- `--fixtures` runs the app on the sample engine (`seam::fake`, answering at a real engine's pace) with real sound
  out. Its settings, queue and cache go in a folder of its own under the system temp folder, removed when the window
  closes, and the status bar's right side reads "Sample data, not a real fight". Nothing it shows is a real fight
  and no video is saved.
- `--data-dir <folder>` keeps the settings, the queue and the cache in `<folder>` (`config` and `cache` under it)
  instead of the temp folder or the PC's own app folders, with the real engine or with `--fixtures`. The folder is
  created if missing; a folder that cannot be used stops the app with one sentence on stderr and exit code 2. Videos
  still go to the output folder named in that folder's settings.
- `--smoke` opens the window, paints 30 frames, closes it and exits with code 0, with no sound and its files in a
  temp folder of its own. `ballfight-studio.exe --smoke` is the quick check that the real app starts on this PC (it
  opens on Match-up with the title card); `--fixtures --smoke` does the same for the sample interface.

### Known limits

- **The folder dialog has no parent window.** "Change folder" opens the PC's folder dialog on an engine thread
  (`engine/files.rs`, `rfd`) without naming the app's window as its owner, so it may open behind the app or show as
  a separate item on the taskbar. While it is open a second "Change folder" does nothing. Giving it the window as
  its owner needs the window's handle on the engine thread, which eframe 0.37 and rfd only allow through unsafe
  code (`WindowHandle::borrow_raw`), so it was left as it is.

At start the app writes one line to stderr naming the graphics adapter its window draws with, for example
`BALLFIGHT Studio draws with AMD Radeon ... (Dx12, DiscreteGpu)`. A release build has no console, so redirect stderr
to a file to read it.

## Run its tests

```sh
cargo test -p bf-app --lib          # pure logic, tested inside each file
cargo test -p bf-app --test ui      # everything that needs egui, in one test program
cargo test -p bf-app --test ui smoke::   # one module of it
```

A note on a rare failure (Task 24d). About once in ten runs on the build PC, the UI test program printed that every
test passed and then exited with a non-zero code. **The cause was not proven**: 26 full runs under load, 230 shorter
runs and 30 `--smoke` runs all exited 0, and Windows logged no crash for it. The suspect is a third-party DLL
crashing while the process closes: wgpu's default backends started Vulkan and OpenGL in every harness, and the
Vulkan loader pulled the PC's overlay and capture hooks (Overwolf, OBS, Steam, Epic) and AMD's Vulkan and OpenGL
drivers into the test process. Ruled out by reading the code: no harness is dropped outside `gpu()`, no device or
app is leaked or kept in a static, and the scheduler and writer threads are joined. Changed: the harnesses
(`common::wgpu_setup`) start only DirectX 12 on Windows, and the app prefers DirectX 12 (`app::prefer_dx12`), so
those hooks are no longer loaded; `WGPU_BACKEND` set in the environment still overrides both. If it happens again,
capture it rather than rerun: build once with `cargo test -p bf-app --test ui --no-run`, then from `crates/bf-app`
run the test exe it names in a PowerShell loop that keeps each run's whole output and its exit code in hex, for
example

```powershell
$exe = (Get-ChildItem ..\..\target\debug\deps\ui-*.exe | Sort-Object LastWriteTime | Select-Object -Last 1).FullName
for ($i = 1; $i -le 60; $i++) {
  $p = Start-Process $exe -NoNewWindow -Wait -PassThru -RedirectStandardOutput "run$i.out.txt" -RedirectStandardError "run$i.err.txt"
  "$i 0x{0:X8}" -f $p.ExitCode | Add-Content exits.txt
}
```

and look up a failing run's time in the Windows Application event log (Application Error, event 1000: the faulting
module). 0xC0000005 is an access violation, 0xC0000374 heap corruption, 0xC0000409 a fast-fail or abort.

The interface tests draw on a software graphics adapter (Microsoft Basic Render Driver on Windows, lavapipe on Linux:
`mesa-vulkan-drivers`). Two such devices drawing at once crash the test process on Windows, so every test that draws
takes the lock `common::gpu()` before anything else. They never open a window, a sound device or a dialog.

The rule for every new test: a test that builds a harness or reads pixels takes `common::gpu()` first and keeps the
guard to its end; `app_harness`, `widget_harness` and `pixels` refuse to run (they panic) without it.

Tests that build the whole app (`tests/ui/flows.rs` on the sample engine, `tests/ui/real.rs` on the real one) keep
its files in a temp folder of their own and use a `NoAudio` output, so they never touch the real `%APPDATA%`,
`%LOCALAPPDATA%` or Videos folders or open a sound device. They wait for the engine on a condition, bounded in
time, and never assert anything through timing. The real-engine tests pass every command through a relay of their
own (`real::Watched`), which holds back and fails the test on `PickFolder` or `RevealFolder`, so no test opens the
folder dialog or Explorer.

### Which tests run where

| Group | Command | On CI | Why |
|---|---|---|---|
| Unit tests inside each file, the engine's among them | `cargo test --release -p bf-app --lib` | yes | no window, no graphics adapter; the render tests use the ffmpeg CI installs |
| The command line | `cargo test --release -p bf-app --bins` | yes | parsing only |
| Saved files of older versions | `cargo test --release -p bf-app --test compat` | yes | files only |
| Everything that draws (`tests/ui`) | `cargo test --release -p bf-app --test ui` | no: the owner's PC | DirectX 12 only (ruling M2-R27), and a hosted runner is not known to offer a DirectX 12 adapter |
| Ignored, run by name: the whole video on the real engine, the preview budgets, the silent sound device | see `studio/README.md` | no: the owner's PC | minutes long, need ffmpeg, a sound device or a quiet PC |

The slow engine tests are ignored in a debug run only (`cfg_attr(debug_assertions, ignore)`); in release they run
with the rest of `--lib`.

### Blessing pictures

Reference pictures live in `tests/snapshots/` and are blessed on Windows. To bless, run the module whose pictures
changed with `UPDATE_SNAPSHOTS` set:

```sh
UPDATE_SNAPSHOTS=true cargo test -p bf-app --test ui small_window::
```

This rewrites only the pictures that differ. Without it, a failed comparison leaves `<name>.new.png` and
`<name>.diff.png` beside the reference, and a bless leaves the previous reference behind as `<name>.old.png`; git
ignores all three and none is committed. A picture is blessed only after it has been looked at next to its section
of `docs/studio/interface-reference.md`, and the differences from the mock-up are written in the task's report.
Each test builds one harness per picture, so a bless rewrites each picture on its own. The shell's picture,
`shell_1440x900.png`, shows the Match-up screen and is re-blessed only by Match-up work (widget rule 18).

On Linux the sound crate needs `libasound2-dev` and `pkg-config` to build.
