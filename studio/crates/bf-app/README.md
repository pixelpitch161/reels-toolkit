# bf-app

The BALLFIGHT Studio desktop app: the window, the interface, the live preview with sound and the render queue. It is
drawn with egui through wgpu (eframe). The interface is a state machine that talks to the engine only through plain
commands and events, so it never waits on a fight, a picture, a soundtrack or a file.

`bf-app` is not an engine crate. Every number that reaches a fight, a frame, a soundtrack or a fight file comes from
the engine crates.

## Run the app

From `studio/`:

```sh
cargo run --release -p bf-app
```

The program is `target/release/ballfight-studio.exe`. `--smoke` opens the window, paints 30 frames, closes it and
exits with code 0: a quick check that the app starts on this PC.

## Run its tests

```sh
cargo test -p bf-app --lib          # pure logic, tested inside each file
cargo test -p bf-app --test ui      # everything that needs egui, in one test program
cargo test -p bf-app --test ui smoke::   # one module of it
```

The interface tests draw on a software graphics adapter (Microsoft Basic Render Driver on Windows, lavapipe on Linux:
`mesa-vulkan-drivers`). Two such devices drawing at once crash the test process on Windows, so every test that draws
takes the lock `common::gpu()` before anything else. They never open a window, a sound device or a dialog.

Reference pictures live in `tests/snapshots/` and are blessed on Windows. To bless a changed picture, run the test
with `UPDATE_SNAPSHOTS=1` and look at the new picture before you commit it. A failing comparison leaves
`*.new.png` and `*.diff.png` beside the reference; git ignores them.

On Linux the sound crate needs `libasound2-dev` and `pkg-config` to build.
