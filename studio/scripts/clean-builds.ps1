<#
.SYNOPSIS
  Finds BALLFIGHT Studio worktrees, clones and Rust build folders on this PC and frees the space of finished ones.

.DESCRIPTION
  Every build folder (a Rust `target`) of this repo grows to 30-95 GB. This script lists every checkout of the repo
  (the main folder, `.wt\<name>`, `.lanes\<name>`, `wt-*` and the other places threads have used) with its size and
  state, and with -Apply it frees space:

    * A checkout is FINISHED when its work tree is clean, its branch is pushed and the branch is merged into the
      build branch. A separate clone (a folder with a .git folder of its own) must also hold no other local branch
      with commits that are not on GitHub and no stash, since those live only in that folder. A finished worktree
      or clone is removed (git worktree remove / folder delete) with its build folder. Its branch stays on GitHub.
      Git-ignored files of a clone go with it; the report names the first few that are not build output.
    * A build folder is IN USE when a program runs from inside it or from its checkout (cargo/rustc/tests/an app).
      In-use folders are never touched.
    * Extra build folders nobody uses (`studio\target-*`, `.wt\<name>-target` with no worktree) are deleted.
    * -TrimMain also deletes the main folder's `studio\target\debug` and `studio\target\tmp` when nothing uses them
      (the next build remakes them). `studio\target\release` is kept so the install keeps working.

  It never touches projects\, reelkit\, series\, studio\reference\ (pinned test data) or anything outside the
  folders listed below.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File studio\scripts\clean-builds.ps1            # report only
  powershell -ExecutionPolicy Bypass -File studio\scripts\clean-builds.ps1 -Apply     # free finished ones
  powershell -ExecutionPolicy Bypass -File studio\scripts\clean-builds.ps1 -Apply -TrimMain
#>
param(
    [switch]$Apply,
    [switch]$TrimMain,
    [string]$BuildBranch = 'claude/project-thread-h102zt'
)

# Continue, not Stop: Windows PowerShell 5.1 turns git's normal stderr lines into terminating errors under Stop.
$ErrorActionPreference = 'Continue'
$Main = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path.TrimEnd('\')
$Parent = Split-Path $Main -Parent

function Get-SizeGB([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return 0 }
    $sum = 0
    try {
        foreach ($f in [System.IO.Directory]::EnumerateFiles($Path, '*', [System.IO.SearchOption]::AllDirectories)) {
            try { $sum += ([System.IO.FileInfo]$f).Length } catch {}
        }
    } catch {}
    [math]::Round($sum / 1GB, 1)
}

# Paths of every running program, used to tell whether a folder is in use.
$running = @(Get-Process -ErrorAction SilentlyContinue | ForEach-Object { try { $_.Path } catch {} } | Where-Object { $_ })
$cmdLines = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessId -ne $PID } |
    ForEach-Object { $_.CommandLine } | Where-Object { $_ -and $_ -notlike '*clean-builds.ps1*' })
function Test-InUse([string]$Path) {
    $p = $Path.TrimEnd('\') + '\'
    foreach ($r in $running) { if ($r.StartsWith($p, [StringComparison]::OrdinalIgnoreCase)) { return $true } }
    foreach ($c in $cmdLines) { if ($c.IndexOf($p.TrimEnd('\'), [StringComparison]::OrdinalIgnoreCase) -ge 0) { return $true } }
    # A cargo lock held right now also counts.
    foreach ($lock in @("$p.cargo-lock", "${p}debug\.cargo-lock", "${p}release\.cargo-lock")) {
        if (Test-Path -LiteralPath $lock) {
            try { $s = [System.IO.File]::Open($lock, 'Open', 'ReadWrite', 'None'); $s.Close() } catch { return $true }
        }
    }
    $false
}

function Remove-Folder([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return }
    cmd /c rd /s /q "`"$Path`"" | Out-Null
    if (Test-Path -LiteralPath $Path) { Write-Warning "Could not fully delete $Path (a file may be open)." }
}

git -C $Main fetch -q origin 2>$null

# Every checkout git knows about, plus stray clones in the usual places.
$checkouts = [System.Collections.Generic.List[string]]::new()
foreach ($line in (git -C $Main worktree list --porcelain)) {
    if ($line -like 'worktree *') { $checkouts.Add((($line.Substring(9)) -replace '/', '\').TrimEnd('\')) }
}
$strayRoots = @("$Main\.lanes", "$Main\.wt", "$Parent\InstrumentBattleMachine-wt")
foreach ($root in $strayRoots) {
    if (Test-Path -LiteralPath $root) {
        Get-ChildItem -LiteralPath $root -Directory -Force | Where-Object { Test-Path -LiteralPath "$($_.FullName)\.git" } |
            ForEach-Object { if (-not $checkouts.Contains($_.FullName)) { $checkouts.Add($_.FullName) } }
    }
}
# Clones made next to the main folder (wt-<name>, <name>-work\reels-toolkit).
foreach ($c in @(Get-ChildItem -LiteralPath $Parent -Directory -Filter 'wt-*' -ErrorAction SilentlyContinue) +
               @(Get-ChildItem -LiteralPath $Parent -Directory -Filter '*-work' -ErrorAction SilentlyContinue |
                 ForEach-Object { Get-Item -LiteralPath "$($_.FullName)\reels-toolkit" -ErrorAction SilentlyContinue })) {
    if ($c -and (Test-Path -LiteralPath "$($c.FullName)\.git") -and -not $checkouts.Contains($c.FullName)) {
        $checkouts.Add($c.FullName)
    }
}

$rows = @()
foreach ($co in $checkouts) {
    if (-not (Test-Path -LiteralPath $co)) { continue }
    $isMain = ($co -ieq $Main)
    # A separate clone has its own copy of GitHub's branches; refresh it so "merged" is judged on today's branch.
    if (-not $isMain -and (Test-Path -LiteralPath "$co\.git" -PathType Container)) { git -C $co fetch -q origin 2>$null }
    $branch = (git -C $co branch --show-current 2>$null)
    $dirty = [bool](git -C $co status --porcelain 2>$null)
    # A separate clone keeps its own branches and stashes: work there that is on no other branch and not on
    # GitHub would go with the folder. (A worktree's branches and stashes live in the main folder's repo.)
    $ownRepo = (-not $isMain) -and (Test-Path -LiteralPath "$co\.git" -PathType Container)
    $onlyHere = @()
    $ignored = @()
    if ($ownRepo) {
        $unpushed = @(git -C $co log --branches --not --remotes --format='%h %s' 2>$null)
        if ($LASTEXITCODE -ne 0) { $onlyHere += 'its branches could not be read' }
        elseif ($unpushed.Count -gt 0) { $onlyHere += "$($unpushed.Count) commit(s) on local branches that are not on GitHub" }
        $stashes = @(git -C $co stash list 2>$null)
        if ($LASTEXITCODE -ne 0) { $onlyHere += 'its stashes could not be read' }
        elseif ($stashes.Count -gt 0) { $onlyHere += "$($stashes.Count) stash(es)" }
        $ignored = @(git -C $co status --porcelain --ignored 2>$null | Where-Object { $_ -like '!! *' } |
            ForEach-Object { $_.Substring(3) } | Where-Object { $_ -notmatch '^studio/target(-[^/]*)?/' })
    }
    $pushed = $false; $merged = $false
    if ($branch) {
        $ahead = git -C $co rev-list --count "origin/$branch..HEAD" 2>$null
        $pushed = ($LASTEXITCODE -eq 0 -and $ahead -eq '0')
        $notMerged = git -C $co rev-list --count "origin/$BuildBranch..HEAD" 2>$null
        $merged = ($LASTEXITCODE -eq 0 -and $notMerged -eq '0')
    }
    $targets = @("$co\studio\target") + @(Get-ChildItem -LiteralPath "$co\studio" -Directory -Filter 'target-*' -ErrorAction SilentlyContinue | ForEach-Object FullName)
    $sibling = "$co-target"; if (Test-Path -LiteralPath $sibling) { $targets += $sibling }
    $targets = @($targets | Where-Object { Test-Path -LiteralPath $_ })
    # The main folder holds other worktrees, so only its build folders count towards its size.
    $outside = @($targets | Where-Object { $isMain -or -not $_.StartsWith("$co\") })
    $size = ($outside | ForEach-Object { Get-SizeGB $_ } | Measure-Object -Sum).Sum
    if (-not $isMain) { $size += Get-SizeGB $co }
    $inUse = (Test-InUse $co) -or [bool]($targets | Where-Object { Test-InUse $_ })
    $rows += [pscustomobject]@{
        Path = $co; Main = $isMain; Branch = $branch; Clean = -not $dirty; Pushed = $pushed; Merged = $merged
        InUse = $inUse; Targets = $targets; SizeGB = $size; OnlyHere = $onlyHere; Ignored = $ignored
    }
}

# Build folders left behind with no checkout (e.g. .wt\<name>-target after its worktree went).
$orphans = @()
if (Test-Path -LiteralPath "$Main\.wt") {
    Get-ChildItem -LiteralPath "$Main\.wt" -Directory -Filter '*-target' | ForEach-Object {
        $owner = $_.FullName.Substring(0, $_.FullName.Length - 7)
        if (-not (Test-Path -LiteralPath $owner)) { $orphans += $_.FullName }
    }
}

$freeBefore = (Get-PSDrive ($Main.Substring(0, 1))).Free
Write-Host ''
Write-Host 'Checkouts of the repo:'
$rows | Sort-Object SizeGB -Descending | Format-Table @{n='GB';e={$_.SizeGB}}, Branch, Clean, Pushed, Merged, InUse, Path -AutoSize | Out-Host
if ($orphans) { Write-Host 'Build folders with no checkout:'; $orphans | ForEach-Object { Write-Host "  $(Get-SizeGB $_) GB  $_" } }

if (-not $Apply) {
    Write-Host 'Report only. Run again with -Apply to free finished ones (and -TrimMain for the main debug build).'
    return
}

foreach ($r in $rows) {
    if ($r.Main) {
        # Extra build folders (studio\target-*) are checked one by one: the main folder itself is nearly always busy.
        foreach ($t in $r.Targets | Where-Object { $_ -ne "$Main\studio\target" }) {
            if (Test-InUse $t) { Write-Host "In use, skipped: $t" } else { Write-Host "Deleting $t"; Remove-Folder $t }
        }
        if ($TrimMain -and -not $r.InUse) {
            foreach ($t in "$Main\studio\target\debug", "$Main\studio\target\tmp") { Write-Host "Deleting $t"; Remove-Folder $t }
        }
        elseif ($TrimMain) { Write-Host "In use, debug build kept: $Main\studio\target" }
        continue
    }
    if ($r.InUse) { Write-Host "In use, skipped: $($r.Path)"; continue }
    if (-not ($r.Clean -and $r.Pushed -and $r.Merged)) {
        Write-Host "Not finished (uncommitted, unpushed or unmerged), kept: $($r.Path)"
        continue
    }
    if ($r.OnlyHere.Count -gt 0) {
        Write-Host "Holds work found nowhere else ($($r.OnlyHere -join ', ')), kept: $($r.Path)"
        continue
    }
    if ($r.Ignored.Count -gt 0) {
        $few = ($r.Ignored | Select-Object -First 5) -join ', '
        Write-Host "Git-ignored files in $($r.Path) go with it ($($r.Ignored.Count), for example $few)."
    }
    foreach ($t in $r.Targets) { Write-Host "Deleting $t"; Remove-Folder $t }
    $isWorktree = (git -C $Main worktree list --porcelain) -contains ('worktree ' + ($r.Path -replace '\\', '/'))
    if ($isWorktree) { Write-Host "Removing worktree $($r.Path)"; git -C $Main worktree remove --force $r.Path }
    if (Test-Path -LiteralPath $r.Path) { Remove-Folder $r.Path }
}
foreach ($o in $orphans) { if (-not (Test-InUse $o)) { Write-Host "Deleting $o"; Remove-Folder $o } }
git -C $Main worktree prune
$freed = ((Get-PSDrive ($Main.Substring(0, 1))).Free - $freeBefore) / 1GB
Write-Host ("Freed {0:N1} GB." -f $freed)
