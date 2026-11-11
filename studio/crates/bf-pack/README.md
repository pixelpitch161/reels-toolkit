# bf-pack

Everything about files on disk outside the app's own data: the ffmpeg that BALLFIGHT Studio ships, and its checked
download. Later tasks of milestone 6 add the package folder, the install, the shortcuts and removal here
(`package.rs`, `install.rs`, `remove.rs`, `config.rs`, `version.rs` are declared and still empty).

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
