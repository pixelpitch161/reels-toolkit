# Sets up BALLFIGHT Studio's GitHub Actions runner on the owner's PC. Needs "Run as administrator" (Windows shows
# a UAC prompt). Safe to run again: every step checks what is already there. See README.md in this folder.
#
#   -Step Account : create the bf-runner user (standard, background logons only, hidden from the sign-in screen).
#   -Step Folders : make F:\bf-runner, copy these scripts to F:\bf-runner\setup and set who may do what there.
#   -Step Copy    : copy the owner's MinGW and studio\reference\.cache into F:\bf-runner (bf-runner: read only).
#                   Run it again after the reference cache changes.
#   -Step Runner  : register the runner with GitHub as a Windows service running as bf-runner. Asks for the
#                   registration token (GitHub: the repository's Settings, Actions, Runners, New self-hosted runner).
#   -Step Lock    : keep bf-runner out of everything on D:, E: and F: (they let every signed-in user in by default).
#                   About 15 minutes on F:. Don't close the window.
#   -Step Tools   : install rustup, the pinned Rust and the scratch clone AS bf-runner.        -> probe\tools.log
#   -Step Smoke   : build and run the interface tests' smoke group once AS bf-runner.          -> probe\smoke.log
#   -Step All     : asks for the token first, then Account, Folders, Copy, Runner, Lock, Tools and Smoke.
#
# -ScratchBranch picks the branch the scratch clone is made from (default: the build branch).
#
# The bf-runner password is random, made fresh on every run, and handed straight to Windows (the user account, the
# scheduled task and the service). It is never written to disk or shown. The registration token is typed into this
# window (hidden), handed straight to the runner's config.cmd and expires in an hour.
#
# This script runs as administrator, so it never runs a program bf-runner could have changed, and it writes only to
# F:\bf-runner\admin and to folders it has just checked or made itself.
param(
    [ValidateSet('Account', 'Folders', 'Copy', 'Runner', 'Lock', 'Tools', 'Smoke', 'All')][string]$Step = 'All',
    [string]$ScratchBranch = '',
    # Get the registration token from the BallPhysicsEngine WSL test machine's `beat` GitHub login instead of asking.
    [switch]$TokenFromWsl
)

. "$PSScriptRoot\common.ps1"
if (-not (Test-Admin)) { throw 'Run this script as administrator.' }
New-Item -ItemType Directory -Force $AdminDir | Out-Null
Start-Transcript -Path "$AdminDir\setup-pc-runner-$Step-$(Get-Date -Format yyyyMMdd-HHmmss).log" | Out-Null

$ServiceName = "actions.runner.$($Repo -replace '/', '-').$RunnerName"
$Admins = '*S-1-5-32-544'   # the Administrators group, by its fixed id (its name differs between languages)
$System = '*S-1-5-18'
$Owner  = "$env:USERDOMAIN\$env:USERNAME"   # the owner's account (this script runs elevated as the owner)

# --- Windows account rights (Log on as a batch job, and so on) through the LSA API, which Windows Home has ----------
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class BfLsa {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    struct LSA_UNICODE_STRING { public ushort Length; public ushort MaximumLength; public string Buffer; }
    [StructLayout(LayoutKind.Sequential)]
    struct LSA_OBJECT_ATTRIBUTES { public int Length; public IntPtr RootDirectory; public IntPtr ObjectName;
        public uint Attributes; public IntPtr SecurityDescriptor; public IntPtr SecurityQualityOfService; }
    [DllImport("advapi32.dll")] static extern uint LsaOpenPolicy(IntPtr system, ref LSA_OBJECT_ATTRIBUTES attr,
        uint access, out IntPtr handle);
    [DllImport("advapi32.dll")] static extern uint LsaAddAccountRights(IntPtr handle, byte[] sid,
        LSA_UNICODE_STRING[] rights, uint count);
    [DllImport("advapi32.dll")] static extern uint LsaClose(IntPtr handle);
    [DllImport("advapi32.dll")] static extern int LsaNtStatusToWinError(uint status);
    public static void Grant(byte[] sid, string right) {
        var attr = new LSA_OBJECT_ATTRIBUTES();
        IntPtr h;
        uint s = LsaOpenPolicy(IntPtr.Zero, ref attr, 0x00000810, out h); // POLICY_CREATE_ACCOUNT | LOOKUP_NAMES
        if (s != 0) throw new System.ComponentModel.Win32Exception(LsaNtStatusToWinError(s));
        try {
            var r = new LSA_UNICODE_STRING[1];
            r[0].Buffer = right; r[0].Length = (ushort)(right.Length * 2); r[0].MaximumLength = (ushort)(right.Length * 2 + 2);
            s = LsaAddAccountRights(h, sid, r, 1);
            if (s != 0) throw new System.ComponentModel.Win32Exception(LsaNtStatusToWinError(s));
        } finally { LsaClose(h); }
    }
}
'@

function Grant-Right([string]$Right) {
    $sid = (Get-LocalUser $RunnerUser).SID
    $bytes = New-Object byte[] $sid.BinaryLength
    $sid.GetBinaryForm($bytes, 0)
    [BfLsa]::Grant($bytes, $Right)
    Write-Host "  $RunnerUser has $Right"
}

# icacls, stopping loudly if it fails.
function Invoke-Icacls {
    & icacls.exe @args | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "icacls $($args -join ' ') failed (exit $LASTEXITCODE)" }
}

# robocopy /MIR, stopping loudly if it fails (robocopy's codes below 8 all mean success).
function Copy-Mirror([string]$From, [string]$To) {
    & robocopy.exe $From $To /MIR /NFL /NDL /NJH /NP /R:3 /W:2 | Out-Null
    if ($LASTEXITCODE -ge 8) { throw "Copying $From to $To failed (robocopy $LASTEXITCODE)" }
}

# A fresh random password, set on the account and on the runner service if it exists. Kept only in memory.
function New-RunnerPassword {
    $chars = [char[]]'ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!#%+-=?@_'
    $bytes = New-Object byte[] 40
    [Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
    $plain = -join ($bytes | ForEach-Object { $chars[$_ % $chars.Length] })
    Set-LocalUser -Name $RunnerUser -Password (ConvertTo-SecureString $plain -AsPlainText -Force)
    if (Get-Service $ServiceName -ErrorAction SilentlyContinue) {
        & sc.exe config $ServiceName obj= ".\$RunnerUser" password= $plain | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "Could not give the runner service the new password (sc.exe $LASTEXITCODE)" }
    }
    return $plain
}

function Initialize-Account {
    Write-Step "Windows user $RunnerUser"
    if (-not (Get-LocalUser $RunnerUser -ErrorAction SilentlyContinue)) {
        # Created with a throwaway password; New-RunnerPassword sets the real one when it's first needed.
        $tmp = ConvertTo-SecureString ([guid]::NewGuid().ToString() + 'aA1!') -AsPlainText -Force
        New-LocalUser -Name $RunnerUser -Password $tmp -PasswordNeverExpires -UserMayNotChangePassword `
            -AccountNeverExpires -FullName 'BALLFIGHT test runner' `
            -Description 'BALLFIGHT Studio checks (GitHub runner)' | Out-Null
        Write-Host "  created $RunnerUser"
    } else { Write-Host "  $RunnerUser already exists" }
    $users = Get-LocalGroupMember -Group Users | Where-Object { $_.Name -like "*\$RunnerUser" }
    if (-not $users) { Add-LocalGroupMember -Group Users -Member $RunnerUser }
    # Never an administrator.
    if (Get-LocalGroupMember -Group Administrators | Where-Object { $_.Name -like "*\$RunnerUser" }) {
        throw "$RunnerUser is an administrator; it must not be"
    }
    # Background logons only: no sign-in at the screen or over Remote Desktop, and hidden from the sign-in screen.
    Grant-Right 'SeBatchLogonRight'
    Grant-Right 'SeServiceLogonRight'
    Grant-Right 'SeDenyInteractiveLogonRight'
    Grant-Right 'SeDenyRemoteInteractiveLogonRight'
    $hide = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon\SpecialAccounts\UserList'
    New-Item -Path $hide -Force | Out-Null
    New-ItemProperty -Path $hide -Name $RunnerUser -Value 0 -PropertyType DWord -Force | Out-Null
}

# Who may do what in F:\bf-runner. F:\bf-runner is owned by Administrators and takes nothing from the drive (F: lets
# every signed-in user change files, and the Lock step denies bf-runner everything below F:\). Everything in it gives
# full control to Administrators, SYSTEM and the owner. bf-runner gets:
#   F:\bf-runner itself                : look inside only (it can't delete, rename or add folders there)
#   setup, mingw64, reference-cache    : read and run only, so it can't change what the owner or an administrator runs
#   admin                              : nothing (administrator logs and the checked runner download)
#   the folders in $RunnerWritable     : full control (jobs write these)
function Set-Folders {
    Write-Step "Folder $Root"
    foreach ($d in @($Root, $AdminDir) + $RunnerReadOnly + $RunnerWritable) { New-Item -ItemType Directory -Force $d | Out-Null }
    Invoke-Icacls $Root /setowner $Admins /C /Q
    Invoke-Icacls $Root /remove:g $RunnerUser /C /Q
    Invoke-Icacls $Root /inheritance:r /grant:r "${Admins}:(OI)(CI)F" "${System}:(OI)(CI)F" "${Owner}:(OI)(CI)F" `
        "${RunnerUser}:(RX)" /C /Q
    # Clear every rule naming bf-runner below F:\bf-runner, then give back exactly what's listed above. /L works on
    # links themselves, never on what they point to.
    Invoke-Icacls "$Root\*" /remove:g $RunnerUser /T /L /C /Q
    foreach ($d in $RunnerReadOnly) { Invoke-Icacls $d /grant:r "${RunnerUser}:(OI)(CI)RX" /C /Q }
    foreach ($d in $RunnerWritable) { Invoke-Icacls $d /grant:r "${RunnerUser}:(OI)(CI)F" /C /Q }

    # These scripts, from the owner's checkout (bf-runner can't read it) into setup, which bf-runner can only read.
    Copy-Mirror $PSScriptRoot $SetupDir
    foreach ($d in @($SetupDir, $AdminDir, $MingwDir, $RefCacheDir)) { Invoke-Icacls $d /setowner $Admins /T /L /C /Q }
    foreach ($d in @($Root, $SetupDir, $AdminDir, $CargoHome)) {
        & icacls.exe $d | Select-String $RunnerUser, 'Administrators', 'SYSTEM', $env:USERNAME, 'Authenticated' |
            ForEach-Object { "  $($_.Line.Trim())" }
    }
}

# The owner's MinGW (winget's WinLibs) and the git-ignored reference cache, copied in; nothing is run from them here.
function Copy-Tools {
    Write-Step 'MinGW compiler'
    $wingetMingw = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Directory -Filter 'BrechtSanders.WinLibs*' |
        ForEach-Object { Join-Path $_.FullName 'mingw64' } | Where-Object { Test-Path "$_\bin\gcc.exe" } |
        Select-Object -First 1
    if (-not $wingetMingw) { throw 'WinLibs MinGW not found in the winget packages folder' }
    Copy-Mirror $wingetMingw $MingwDir
    Write-Host "  from $wingetMingw"

    Write-Step 'Reference cache (studio\reference\.cache, for bf-harness)'
    $cache = "$OwnerCheckout\studio\reference\.cache"
    if (Test-Path $cache) {
        Copy-Mirror $cache $RefCacheDir
        $mb = [int]((Get-ChildItem $RefCacheDir -Recurse -File | Measure-Object Length -Sum).Sum / 1MB)
        Write-Host "  $mb MB in $RefCacheDir"
    } else { Write-Warning "  no $cache; the fighter match checks will say their files are missing" }
    foreach ($d in @($MingwDir, $RefCacheDir)) {
        Invoke-Icacls $d /setowner $Admins /T /L /C /Q
        Invoke-Icacls $d /grant:r "${RunnerUser}:(OI)(CI)RX" /C /Q
    }
}

# Runs as-runner.ps1 once as bf-runner from a task set to "run whether user is logged on or not" (session 0), waits
# for it, then removes the task again.
function Invoke-AsRunner([string]$Mode, [int]$TimeoutMinutes) {
    $task = "BALLFIGHT runner $Mode"
    $pw = New-RunnerPassword
    $action = New-ScheduledTaskAction -Execute 'powershell.exe' `
        -Argument "-NoProfile -ExecutionPolicy Bypass -File `"$SetupDir\as-runner.ps1`" -Mode $Mode -ScratchBranch `"$ScratchBranch`""
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes $TimeoutMinutes) `
        -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -Priority 7
    Unregister-ScheduledTask -TaskName $task -Confirm:$false -ErrorAction SilentlyContinue
    Register-ScheduledTask -TaskName $task -Action $action -Settings $settings -User $RunnerUser -Password $pw `
        -RunLevel Limited | Out-Null
    $pw = $null
    Start-ScheduledTask -TaskName $task
    Write-Host "  started '$task' as $RunnerUser, waiting (up to $TimeoutMinutes minutes)"
    $deadline = (Get-Date).AddMinutes($TimeoutMinutes + 2)
    do { Start-Sleep -Seconds 10 } while ((Get-ScheduledTask -TaskName $task).State -eq 'Running' -and
        (Get-Date) -lt $deadline)
    $info = Get-ScheduledTaskInfo -TaskName $task
    Write-Host "  finished, result $($info.LastTaskResult)"
    Unregister-ScheduledTask -TaskName $task -Confirm:$false
}

function Invoke-Tools {
    Write-Step "rustup, Rust and the scratch clone, installed as $RunnerUser"
    if (-not (Test-Path "$MingwDir\bin\gcc.exe")) { throw "No MinGW in $MingwDir; run -Step Copy first" }
    Invoke-AsRunner -Mode Tools -TimeoutMinutes 90
    Get-Content "$ProbeDir\tools.log" -Tail 12
}

function Invoke-Smoke {
    Write-Step "Trial run of the interface tests' smoke group as $RunnerUser (the first build takes a while)"
    if (-not (Test-Path "$ScratchDir\studio\Cargo.toml")) { throw "No scratch clone at $ScratchDir; run -Step Tools first" }
    Invoke-AsRunner -Mode Smoke -TimeoutMinutes 150
    Get-Content "$ProbeDir\smoke.log" -Tail 15
}

# The environment every job gets. The runner folder is bf-runner's, so refuse to write through a link it planted.
function Write-RunnerEnv {
    $envFile = "$RunnerDir\.env"
    if (Test-Path $envFile) {
        if ((Get-Item $envFile -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "$envFile is a link; not writing it" }
        Remove-Item $envFile -Force
    }
    $machinePath = [Environment]::GetEnvironmentVariable('PATH', 'Machine')
    @(
        "RUSTUP_HOME=$RustupHome"
        "CARGO_HOME=$CargoHome"
        'CARGO_NET_RETRY=10'
        'RUSTUP_MAX_RETRIES=10'
        "CARGO_BUILD_JOBS=$BuildJobs"
        "BF_PC_TARGET=$TargetDir"
        "BF_PC_FFMPEG_CACHE=$FfmpegCache"
        "BF_PC_REFERENCE_CACHE=$RefCacheDir"
        "PATH=$CargoHome\bin;$MingwDir\bin;$GitDir;C:\Program Files\Git\bin;C:\Program Files\Git\usr\bin;$machinePath"
    ) | Set-Content $envFile -Encoding ascii
}

# The registration token, typed (hidden) into this window. Kept only in memory.
function Read-Token {
    if ($TokenFromWsl) {
        # The owner agreed (Nov 23, 2026) to use the BallPhysicsEngine WSL test machine's `beat` GitHub login for this
        # one call; nothing in WSL is changed.
        $t = (& wsl.exe -d bpe-ci -u beat -- gh api -X POST "repos/$Repo/actions/runners/registration-token" --jq .token)
        if (-not $t) { throw 'Could not get a registration token from the WSL beat login' }
        return "$t".Trim()
    }
    Write-Host ''
    Write-Host 'GitHub registration token: on github.com open pixelpitch161/reels-toolkit, Settings, Actions, Runners,'
    Write-Host '"New self-hosted runner", Windows. Copy the token from the line that starts with ./config.cmd'
    Write-Host '(the text after --token) and paste it here. It is not shown, and it expires in an hour.'
    $secure = Read-Host -AsSecureString 'Token'
    $plain = [Runtime.InteropServices.Marshal]::PtrToStringAuto([Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure))
    if (-not $plain -or $plain.Trim().Length -lt 20) { throw 'No token was pasted' }
    return $plain.Trim()
}

function Install-Runner([string]$Token) {
    Write-Step "GitHub Actions runner $RunnerName as a Windows service running as $RunnerUser"
    $svc = Get-Service $ServiceName -ErrorAction SilentlyContinue
    $registered = (Test-Path "$RunnerDir\.runner") -and $svc
    if (-not $registered) {
        if (-not $Token) { $Token = Read-Token }
        # A fresh copy of the latest runner, checked against the SHA-256 in its release notes, unpacked into a new
        # folder bf-runner can't touch until config.cmd (run here as administrator) has finished with it.
        if ($svc) { Stop-Service $ServiceName -Force; & sc.exe delete $ServiceName | Out-Null }
        $release = Invoke-RestMethod 'https://api.github.com/repos/actions/runner/releases/latest' -Headers @{ 'User-Agent' = 'bf-setup' }
        $version = $release.tag_name.TrimStart('v')
        if ($release.body -notmatch '<!-- BEGIN SHA win-x64 -->([0-9a-fA-F]{64})<!-- END SHA win-x64 -->') {
            throw 'The runner release notes have no SHA-256 for win-x64'
        }
        $zip = "$AdminDir\actions-runner-win-x64-$version.zip"
        Get-Download "https://github.com/actions/runner/releases/download/v$version/actions-runner-win-x64-$version.zip" $zip $Matches[1]
        if (Test-Path $RunnerDir) { & cmd.exe /c rmdir /s /q "$RunnerDir" }   # rmdir never follows links
        New-Item -ItemType Directory $RunnerDir | Out-Null   # inherits only Administrators, SYSTEM and the owner
        Expand-Archive $zip -DestinationPath $RunnerDir
        Write-RunnerEnv

        $pw = New-RunnerPassword
        Push-Location $RunnerDir
        & .\config.cmd --unattended --url "https://github.com/$Repo" --token $Token --name $RunnerName `
            --labels $RunnerLabels --work $WorkDir --replace --runasservice `
            --windowslogonaccount ".\$RunnerUser" --windowslogonpassword $pw | Out-Host
        $code = $LASTEXITCODE
        Pop-Location
        $Token = $null; $pw = $null
        if ($code -ne 0) { throw "config.cmd failed ($code). A token works once and for an hour: get a new one and run -Step Runner again." }
        # Now the runner is bf-runner's: it writes its logs here and updates itself.
        Invoke-Icacls $RunnerDir /grant:r "${RunnerUser}:(OI)(CI)F" /C /Q
    } else {
        Write-RunnerEnv
        $null = New-RunnerPassword
        Write-Host '  already registered; refreshed its password and .env'
    }
    # Normal priority for the runner itself, on purpose: the BallPhysicsEngine runner at below-normal missed GitHub's
    # 30-second handoff while builds ran. The builds leave CPU free instead (CARGO_BUILD_JOBS in the .env).
    Set-Service $ServiceName -StartupType Automatic
    Restart-Service $ServiceName
    Get-Service $ServiceName | Format-List Name, Status, StartType
    (Get-CimInstance Win32_Service -Filter "Name='$ServiceName'").StartName
}

# D:, E: and F: let every signed-in Windows user read and change files (Authenticated Users: Modify), so bf-runner
# could reach the owner's music, video and project folders. This adds one "deny everything to bf-runner" rule that
# applies to everything inside each drive (not the drive's top level itself, so tools can still look at the path).
# Nobody else's access changes. F:\bf-runner has its own rules and takes nothing from the drive (Set-Folders).
function Lock-Drives {
    Write-Step "Keep $RunnerUser out of the owner's drives"
    $rule = "\\${RunnerUser}:\(OI\)\(CI\)\(IO\)\(N\)"
    foreach ($drive in 'D:\', 'E:\', 'F:\') {
        if (-not (Test-Path $drive)) { continue }
        if ((& icacls.exe $drive) -match $rule) { Write-Host "  $drive already locked"; continue }
        Write-Host "  locking $drive (Windows copies the rule onto every folder, this can take a while)"
        & icacls.exe $drive /deny "${RunnerUser}:(OI)(CI)(IO)(F)" /C /Q | Out-Null
        $code = $LASTEXITCODE
        if (-not ((& icacls.exe $drive) -match $rule)) { throw "Locking $drive didn't take (icacls exit $code)" }
        if ($code -ne 0) { Write-Warning "  $drive locked, but icacls reported $code (some protected system folders can't be changed)" }
        Write-Host "  $drive done"
    }
    foreach ($p in @($env:USERPROFILE, $OwnerCheckout, 'F:\', $Root)) {
        if (-not (Test-Path $p)) { continue }
        Write-Host "  $p :"; & icacls.exe $p | Select-String $RunnerUser, 'Authenticated|Users:', $env:USERNAME | Out-Host
    }
}

switch ($Step) {
    'Account' { Initialize-Account }
    'Folders' { Set-Folders }
    'Copy'    { Copy-Tools }
    'Runner'  { Install-Runner }
    'Lock'    { Lock-Drives }
    'Tools'   { Invoke-Tools }
    'Smoke'   { Invoke-Smoke }
    'All'     {
        # The token first, so the owner can paste it and walk away; the rest needs nobody.
        $token = $null
        if (-not ((Test-Path "$RunnerDir\.runner") -and (Get-Service $ServiceName -ErrorAction SilentlyContinue))) { $token = Read-Token }
        Initialize-Account; Set-Folders; Copy-Tools; Install-Runner $token; $token = $null
        Lock-Drives; Invoke-Tools; Invoke-Smoke
    }
}
Write-Step 'Done'
Stop-Transcript | Out-Null
