# bf-pack

Everything about files on disk outside the app's own data: the ffmpeg that BALLFIGHT Studio ships and its checked
download (`ffmpeg.rs`, `fetch.rs`), the package folder (`package.rs`), the install and its shortcuts
(`install.rs`), removal (`remove.rs`), `update.toml` (`config.rs`) and version numbers (`version.rs`).

## The package, the install and removal

```
ballfight-pack package --exe <ballfight-studio.exe> --out <new folder> [--from <checkout>] [--cache <folder>]
ballfight-pack install --exe <ballfight-studio.exe> --from <owner's clone> [--root <folder>] [--cache <folder>]
```

- **The package** is the program folder as installed: `ballfight-studio.exe`, `ballfight.ico` (from
  `studio/crates/bf-app/assets/icon`), `ffmpeg\` (the pinned files and ABOUT.txt, from the cache), `guide\` (from
  `studio/packaging/guide`, when there is one) and `MANIFEST.txt`, which lists every other file with its size and
  SHA-256 (first line `BALLFIGHT Studio package manifest 1`, then `version <v>`, then `<sha256> <bytes> <path>`).
  The out folder must be new or empty and outside any cargo build folder. Packaging never downloads: ffmpeg must
  already be in the cache (default `%LOCALAPPDATA%\BALLFIGHT Studio\data\downloads`; fill it with `fetch-ffmpeg`).
- **The install** (decision D2: a plain per-user folder, no installer, no admin rights, nothing in the registry)
  packages into a temp folder, makes the app's own copy of the source for updates (`git clone --no-checkout` of the
  clone's `origin` into `<data>\source`, through `run::run` with a 30-minute limit, after the line "Copying the source
  from GitHub; this can take a few minutes. A GitHub sign-in window may open."; an address with a user name or
  password is refused without repeating it; a folder there that is not a finished copy is named, never deleted),
  copies the package into `BALLFIGHT Studio.installing` beside the program folder, checks every file, and only
  then puts it in place: one rename for a fresh install; over an earlier install, `.previous` gets a copy of the
  earlier manifest, each file the earlier manifest lists moves into `.previous`, each new file moves in, and the last
  step replaces `MANIFEST.txt`, all undone if a step fails. The swap writes `.previous\INSTALLING` before its first
  change and removes it after its last, so an install stopped part-way (a power cut) is told from a finished one and
  is finished by running it again. It refuses while the app runs ("Close BALLFIGHT Studio first."), over a folder
  without our manifest, and when a new file would land on a file a person put there. Then it writes `update.toml`
  and the two shortcuts (one it cannot make is a note, not a failure). `--root <folder>` puts everything (program
  folder, Start menu, desktop, data) inside that folder instead.
- **Where** (`install::Layout::system()`): `%LOCALAPPDATA%\Programs\BALLFIGHT Studio`,
  `%APPDATA%\Microsoft\Windows\Start Menu\Programs\BALLFIGHT Studio.lnk`, `<Desktop>\BALLFIGHT Studio.lnk`
  (Windows' own desktop folder, which follows OneDrive), data in `%LOCALAPPDATA%\BALLFIGHT Studio\data`.
- **Shortcuts** are written by `install::lnk`, a small writer of the shell link format (MS-SHLLINK): the target's
  item id list comes from Windows itself (`SHParseDisplayName`, on a thread of its own), everything else is plain
  code. No PowerShell, no `cmd`, no COM interface tables. An existing shortcut of that name is replaced only when it
  already opens the installed exe.
- **Removal** (`remove::remove_app`, called by Settings inside the app; ruling M6-R2) deletes only what an install
  wrote, file by file from the manifest, each file only while its SHA-256 still matches: the program folder's listed
  files, `update.toml`, `MANIFEST.txt`, the same inside `.previous` and `.next`, what a stopped install left (by the
  staging folder's own manifest), the shortcuts that open the installed exe, and then each folder that is left
  empty (a folder's manifest stays while anything of ours in it could not go, so removal can be run again). It never
  deletes a folder with what is in it, never follows a link, and refuses a folder without our
  manifest, one not named `BALLFIGHT Studio`, a drive root, the user's own folder and the data folder. Files a person
  added stay. **Kept:** settings and the queue (`%APPDATA%\BALLFIGHT Studio`), the whole of `%LOCALAPPDATA%\BALLFIGHT
  Studio` (cache, logs and fault reports, and the rebuildable source copy, builds and downloads) and every video.
  Windows lets a running exe be renamed but not deleted, so the app's own exe is first moved to a new name in the
  temp folder (same drive only) and the words say where; if that cannot be done, the exe (and so its folder) stays
  and the words name it. No other program is started. Only the installed exe itself may call it ("This copy of
  BALLFIGHT Studio is not the installed one in {folder}, so nothing was removed.").

## The ffmpeg pin

`studio/packaging/ffmpeg.toml` names the archive (its https address, size and SHA-256), the version and build, the
licence, where the source code is, and each file the app needs from the archive with its SHA-256: `ffmpeg.exe`,
`ffprobe.exe` and `LICENSE.txt` (the archive's `LICENSE`, unchanged). The file says where each value was read.

While the archive's checksum has not been read from the real archive, `archive_sha256` holds the words
`not yet pinned` and `archive_bytes` is 0. In that state the fetcher refuses: nothing is downloaded, and no archive
already in a cache is used, since nothing could check it.

## The command

```
ballfight-pack fetch-ffmpeg --cache <folder> --out <folder>
```

- Exit 0 with one line on success; exit 1 with one plain sentence on the error output.
- The cache keeps the archive (`ffmpeg-<version>-full_build.zip`) and its checked files
  (`<version>-<first 12 digits of the archive's SHA-256>\`). An archive there with the right size and SHA-256 is not
  downloaded again; the unpacked files are checked again on every run, and unpacked again if any differs.
- A download goes to `<name>.part` and gets its name only after its size and SHA-256 match; anything else removes
  the `.part` file and is an error. Unpacking works the same way, in `<folder>.part`.
- The out folder receives `ffmpeg.exe`, `ffprobe.exe`, `LICENSE.txt` and `ABOUT.txt` (version, build, the archive's
  address and SHA-256, the licence, where the source is, and that the Studio runs ffmpeg as a separate program). It
  is the folder that becomes the app's `ffmpeg\`. Every checked file is checked again before the out folder is
  touched. A file already there with the right SHA-256 is kept; one with another SHA-256 is replaced, and the command
  prints "Replaced an older <name>." for it. If a copy fails part-way, the files placed before it stay (each checked),
  no `.part` file is left, and `ABOUT.txt`, written last, is not written.
- A bad pin built into the tool is a plain sentence and exit 1, never a crash.
- Nothing is written outside the two folders it is given.

Where the cache lives: on the owner's PC `%LOCALAPPDATA%\BALLFIGHT Studio\data\downloads`; in CI the runner's temp
folder, kept between runs by `actions/cache`. **Nothing this crate downloads is ever committed** (not the archive,
not the exes).

## Downloads and programs

- Downloads go through Windows' own curl, `<drive>:\Windows\System32\curl.exe` (from the `SystemRoot` variable,
  which must name a drive's `Windows` folder; never from `PATH`), with fixed arguments:
  `-q -fsSL --retry 3 --proto =https --proto-redir =https --connect-timeout 30 --speed-limit 1024 --speed-time 120
  --max-filesize <archive bytes> -o <file> --url <address>`. `-q` comes first so that curl reads no `.curlrc` or
  `_curlrc`: a user's curl settings can add nothing (no credentials, headers, proxy, `-k` or other output).
- Only a public https address is taken: one holding a user name or password is refused without repeating it, and so
  is one naming this PC or a private network (`localhost`, `127.0.0.0/8`, `[::1]`, `10/8`, `172.16/12`,
  `192.168/16`, link-local, unique-local IPv6, or a number curl would read as an address). No sign-in is ever sent.
- Each file unpacked from the archive may be at most 1 GB (`ffmpeg::UNPACK_CAP`), by what the archive declares and
  by what is actually read.
- Every child program is started through `run::run` (`run::quiet` builds the command): its arguments one by one, no
  console window, no input, its error output read on a thread, always waited for, stopped after a time limit or on
  cancel. `tests/scan.rs` fails on any other `Command::new`, `process::Command` or `Command as` in this crate and in
  the app's packaging modules.

## Tests

`cargo test -p bf-pack`. The tests never download: a fake download copies an archive the test builds in a temp
folder. One real-curl test points curl at a plain-http server on this PC itself and checks that curl refuses it
without connecting. `the_real_archive_fetches` is ignored and downloads the real archive; run it by name only with
the owner's go-ahead (`cargo test -p bf-pack -- --ignored the_real_archive_fetches`).
