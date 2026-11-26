# Runs AS bf-runner from a one-off scheduled task set to "run whether user is logged on or not", which is the same
# hidden background session (session 0) the runner's Windows service uses. setup-pc-runner.ps1 starts it.
#
#   -Mode Tools : installs rustup and the pinned Rust (GNU host, clippy, rustfmt) and makes the scratch clone, with
#                 its crates fetched.                                                   -> probe\tools.log
#   -Mode Smoke : the trial run. Builds the app's interface tests and runs their smoke group, which draws through
#                 DirectX 12 (the software adapter) in session 0, as a job will.        -> probe\smoke.log
#                 Built in F:\bf-runner\target, so the first real job starts warm.
#
# bf-runner can change everything in cargo and rustup (jobs need that), so those programs are only ever run as
# bf-runner, never by the owner or as administrator.
param([ValidateSet('Tools', 'Smoke')][string]$Mode = 'Tools', [string]$ScratchBranch = '')

. "$PSScriptRoot\common.ps1"
if ((whoami) -notlike "*\$RunnerUser") { throw "as-runner.ps1 runs as $RunnerUser (setup-pc-runner.ps1 starts it)" }

# The same settings the runner's .env gives every job (see Write-RunnerEnv in setup-pc-runner.ps1).
$env:RUSTUP_HOME = $RustupHome
$env:CARGO_HOME  = $CargoHome
$env:PATH = "$CargoHome\bin;$MingwDir\bin;$GitDir;$env:PATH"
# The scratch clone may have been made by another account; git may use it anyway.
$env:GIT_CONFIG_COUNT = '1'; $env:GIT_CONFIG_KEY_0 = 'safe.directory'; $env:GIT_CONFIG_VALUE_0 = '*'

$whoami = whoami
$session = (Get-Process -Id $PID).SessionId

if ($Mode -eq 'Tools') {
    Start-Transcript -Path "$ProbeDir\tools.log" | Out-Null
    try {
        Set-Location $Root   # away from any rust-toolchain.toml
        Write-Host "user: $whoami   session: $session"

        Write-Step "Scratch clone of $ScratchBranch"
        # The repository is public, so bf-runner clones it without any login.
        if (-not (Test-Path "$ScratchDir\.git")) {
            Invoke-Retry 5 { & git clone --branch $ScratchBranch --single-branch "https://github.com/$Repo.git" $ScratchDir }
        } else {
            Invoke-Retry 5 { & git -C $ScratchDir fetch origin $ScratchBranch }
            & git -C $ScratchDir checkout --force FETCH_HEAD
        }
        & git -C $ScratchDir log -1 --oneline
        $rust = Get-RustVersion $ScratchDir

        Write-Step "rustup and Rust $rust"
        if (-not (Test-Path "$CargoHome\bin\rustup.exe")) {
            $init = "$DownloadDir\rustup-init.exe"
            $base = 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-gnu/rustup-init.exe'
            Get-Download "$base.sha256" "$DownloadDir\rustup-init.exe.sha256"
            $sha = ((Get-Content "$DownloadDir\rustup-init.exe.sha256" -Raw) -split '\s+')[0]
            Get-Download $base $init $sha
            Invoke-Retry 5 { & $init -y --no-modify-path --profile minimal --default-host x86_64-pc-windows-gnu `
                --default-toolchain none }
        }
        Invoke-Retry 8 { & rustup toolchain install $rust --profile minimal -c clippy -c rustfmt -t x86_64-pc-windows-gnu }
        Invoke-Retry 3 { & rustup default $rust }
        & rustc -vV

        Write-Step 'Crates for the scratch clone'
        Push-Location "$ScratchDir\studio"
        Invoke-Retry 8 { & cargo fetch --locked }
        Pop-Location
        Write-Host 'exit code: 0'
    } catch {
        Write-Host "ERROR: $_"
        Write-Host 'exit code: 1'
    } finally {
        Stop-Transcript | Out-Null
    }
    return
}

# Smoke
$ErrorActionPreference = 'Continue'   # record every problem in the log instead of stopping at the first
$log = "$ProbeDir\smoke.log"
"user: $whoami   session: $session   start: $(Get-Date -Format s)" | Set-Content $log -Encoding utf8
$env:CARGO_TARGET_DIR = $TargetDir
$env:CARGO_BUILD_JOBS = $BuildJobs
Set-Location "$ScratchDir\studio"
$sw = [Diagnostics.Stopwatch]::StartNew()
& cargo --version 2>&1 | Out-File $log -Append -Encoding utf8
& cargo test --release --locked -p bf-app --test ui smoke:: -- --nocapture 2>&1 | Out-File $log -Append -Encoding utf8
"exit code: $LASTEXITCODE   total minutes (build + tests): $([math]::Round($sw.Elapsed.TotalMinutes, 1))" |
    Out-File $log -Append -Encoding utf8
"end: $(Get-Date -Format s)" | Out-File $log -Append -Encoding utf8
