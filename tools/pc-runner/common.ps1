# Shared settings and helpers for BALLFIGHT Studio's test runner on the owner's PC.
# Every other script in this folder loads this file first (". $PSScriptRoot\common.ps1").
# The scripts run from F:\bf-runner\setup (setup-pc-runner.ps1 copies this folder there), because the background
# steps run as bf-runner, which can't read the owner's checkout. Nothing secret lives here.
#
# Mirrors the BallPhysicsEngine GPU runner (F:\bpe-gpu) with its own user and folder, so the two never share files.

$ErrorActionPreference = 'Stop'

# --- Where everything goes ---------------------------------------------------------------------------------------
# bf-runner may only READ F:\bf-runner itself, setup, mingw64 and reference-cache, has NO access to admin, and may
# WRITE only the folders in $RunnerWritable. So nothing bf-runner can change is ever run by the owner or as
# administrator (see Set-Folders in setup-pc-runner.ps1).
$Root        = 'F:\bf-runner'
$SetupDir    = "$Root\setup"            # these scripts (bf-runner: read only)
$AdminDir    = "$Root\admin"            # administrator logs and the checked runner download (bf-runner: no access)
$MingwDir    = "$Root\mingw64"          # a copy of the owner's WinLibs MinGW compiler (bf-runner: read only)
$RefCacheDir = "$Root\reference-cache"  # a copy of studio\reference\.cache, for bf-harness (bf-runner: read only)
$LogDir      = "$Root\logs"             # what the bf-runner steps printed
$ProbeDir    = "$Root\probe"            # results of the trial run
$RustupHome  = "$Root\rustup"           # Rust toolchains (written only by bf-runner)
$CargoHome   = "$Root\cargo"            # cargo and rustup, plus downloaded crates (written only by bf-runner)
$RunnerDir   = "$Root\runner"           # the GitHub Actions runner (it updates itself, so bf-runner writes here)
$WorkDir     = "$Root\work"             # where the runner checks out the repository for each job
$TargetDir   = "$Root\target"           # build files, kept between runs (one folder: one runner, one job at a time)
$FfmpegCache = "$Root\ffmpeg-cache"     # the pinned ffmpeg archive, downloaded once by the studio checks
$ScratchDir  = "$Root\scratch"          # a clone used for the trial run
$DownloadDir = "$Root\downloads"        # rustup-init, downloaded by bf-runner
$RunnerWritable = @($LogDir, $ProbeDir, $RustupHome, $CargoHome, $RunnerDir, $WorkDir, $TargetDir, $FfmpegCache,
    $ScratchDir, $DownloadDir)
$RunnerReadOnly = @($SetupDir, $MingwDir, $RefCacheDir)

$RunnerUser   = 'bf-runner'              # standard (not admin) local Windows user the runner runs as
$RunnerName   = 'bf-pc-1'                # the runner's name on GitHub
$RunnerLabels = 'bf-pc'                  # GitHub adds self-hosted, Windows and X64 itself
$Repo         = 'pixelpitch161/reels-toolkit'
$OwnerCheckout = 'F:\!Ai Stuff\InstrumentBattleMachine'
# Builds leave 4 of the 16 logical processors free, so the PC stays usable while the checks run.
$BuildJobs    = '12'
# Branch for the scratch clone. A script that takes -ScratchBranch sets it before loading this file.
if (-not (Get-Variable ScratchBranch -ErrorAction SilentlyContinue) -or -not $ScratchBranch) {
    $ScratchBranch = 'claude/project-thread-h102zt'
}

$GitDir = 'C:\Program Files\Git\cmd'

# Network on this PC damages downloads now and then, so everything retries.
$env:RUSTUP_MAX_RETRIES = '10'
$env:CARGO_NET_RETRY    = '10'

function Write-Step([string]$Text) {
    Write-Host ''
    Write-Host "=== $Text ===" -ForegroundColor Cyan
}

# The Rust version pinned in studio\rust-toolchain.toml of the given checkout.
function Get-RustVersion([string]$Checkout) {
    $toml = Get-Content "$Checkout\studio\rust-toolchain.toml" -Raw
    if ($toml -notmatch 'channel\s*=\s*"([^"]+)"') { throw "No channel in $Checkout\studio\rust-toolchain.toml" }
    return $Matches[1]
}

# Downloads a file with curl (which ships with Windows), retrying through dropped or damaged connections.
# If a SHA-256 is given the file must match it, otherwise it is deleted and the download is tried again.
function Get-Download([string]$Url, [string]$OutFile, [string]$Sha256 = '') {
    New-Item -ItemType Directory -Force (Split-Path $OutFile) | Out-Null
    for ($try = 1; $try -le 5; $try++) {
        & curl.exe --fail --location --silent --show-error --retry 10 --retry-all-errors --retry-delay 3 `
            --output $OutFile $Url
        if ($LASTEXITCODE -eq 0 -and (Test-Path $OutFile)) {
            if (-not $Sha256) { return }
            $got = (Get-FileHash $OutFile -Algorithm SHA256).Hash
            if ($got -ieq $Sha256) { Write-Host "  checked: $(Split-Path $OutFile -Leaf) SHA-256 $got"; return }
            Write-Warning "  $(Split-Path $OutFile -Leaf) came through damaged (SHA-256 $got), downloading again"
        }
        Remove-Item $OutFile -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds (5 * $try)
    }
    throw "Could not download $Url intact after 5 tries"
}

# Runs a command again (up to $Tries times) until it exits with 0. For rustup, cargo and git, whose own retries
# sometimes give up on this network.
function Invoke-Retry([int]$Tries, [scriptblock]$Command) {
    for ($try = 1; $try -le $Tries; $try++) {
        & $Command
        if ($LASTEXITCODE -eq 0) { return }
        Write-Warning "  attempt $try of $Tries failed (exit $LASTEXITCODE), trying again"
        Start-Sleep -Seconds (5 * $try)
    }
    throw "Command failed $Tries times: $Command"
}

function Test-Admin {
    ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator)
}
