param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [int]$Port = 4322,
    [switch]$Build,
    [string]$NprGodot = ''
)
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if ($env:BEAVER_API_TOKEN -notmatch '^[A-Za-z0-9_-]{32,256}$') {
    throw 'Set BEAVER_API_TOKEN in the calling process; never put it in command arguments.'
}
$stopped = @()
foreach ($candidate in @(Get-Process -Name Beaver -ErrorAction SilentlyContinue)) {
    $old = Get-Process -Id $candidate.Id -ErrorAction SilentlyContinue
    if (!$old -or $old.HasExited) { continue }
    $oldPath = $old.Path
    if (!$oldPath -or [IO.Path]::GetFileName($oldPath) -ine 'Beaver.exe') {
        throw "Cannot verify Beaver process $($old.Id)"
    }
    $stopped += @{ pid = $old.Id; executable = $oldPath }
    if ($old.CloseMainWindow()) { [void]$old.WaitForExit(10000) }
    if (!$old.HasExited) {
        $current = Get-Process -Id $old.Id -ErrorAction SilentlyContinue
        if ($current -and $current.Path -eq $oldPath) {
            Stop-Process -Id $old.Id
            $current.WaitForExit()
        }
    }
}
if (@(Get-Process -Name Beaver -ErrorAction SilentlyContinue).Count) {
    throw 'Old Beaver processes remain; refusing to launch another version.'
}
Push-Location $workspace
try {
    if ($Build) {
        & rtk proxy npm run build:native
        if ($LASTEXITCODE -ne 0) { throw 'Native build failed' }
    }
    $exe = (Resolve-Path -LiteralPath $Executable).Path
    if ([IO.Path]::GetFileName($exe) -ine 'Beaver.exe') { throw 'Expected Beaver.exe' }
    $round = Join-Path $workspace ('output\validation\fresh-round-' + [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss') + '-' + [Guid]::NewGuid().ToString('N'))
    [void](New-Item -ItemType Directory -Path $round)
    $env:BEAVER_DATA_DIR = Join-Path $round 'data'
    $env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $round 'webview'
    $env:BEAVER_API_PORT = [string]$Port
    $process = Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe) -WindowStyle Hidden -PassThru
    $headers = @{ Authorization = 'Bearer ' + $env:BEAVER_API_TOKEN }
    function Invoke-Beaver([string]$Method, $InputData = @{}) {
        $body = @{ method = $Method; input = $InputData } | ConvertTo-Json -Depth 30 -Compress
        $response = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/v1/call" -Method Post -Headers $headers -ContentType 'application/json; charset=utf-8' -Body ([Text.Encoding]::UTF8.GetBytes($body)) -TimeoutSec 300
        if (!$response.ok) { throw "Beaver call failed: $Method" }
        return $response.result
    }
    $ready = $false
    for ($i = 0; $i -lt 60; $i++) {
        if ($process.HasExited) { throw 'New Beaver exited before API startup' }
        try { $state = Invoke-Beaver 'state'; $ready = $true; break } catch { Start-Sleep -Milliseconds 500 }
    }
    if (!$ready) { throw 'Beaver API did not start' }
    if (@($state.projects).Count -or @($state.tasks).Count) { throw 'Fresh round unexpectedly contains previous projects or tasks' }
    $project = Invoke-Beaver 'project.create' @{ parent = $round; name = 'Fresh game'; template = 'blank' }
    $projectPath = [IO.Path]::GetFullPath($project.path)
    if ($projectPath.StartsWith('\\?\UNC\')) { $projectPath = '\\' + $projectPath.Substring(8) }
    elseif ($projectPath.StartsWith('\\?\')) { $projectPath = $projectPath.Substring(4) }
    if (!$projectPath.StartsWith($round + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Project outside fresh round' }
    $running = @(Get-Process -Name Beaver -ErrorAction SilentlyContinue)
    if ($running.Count -ne 1 -or $running[0].Id -ne $process.Id -or $running[0].Path -ne $exe) { throw 'Unexpected active Beaver instance' }
    $proof = @{
        root = $round; pid = $process.Id; executable = $exe; port = $Port
        projectId = $project.id; projectPath = $projectPath; template = 'blank'
        priorProjects = 0; priorTasks = 0; stopped = $stopped
        initialFiles = @(Get-ChildItem -LiteralPath $projectPath -File -Recurse | ForEach-Object { $_.FullName.Substring($projectPath.Length + 1) })
        exeSha256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
    }
    $json = $proof | ConvertTo-Json -Depth 10
    [IO.File]::WriteAllText((Join-Path $round 'start-proof.json'), $json, [Text.UTF8Encoding]::new($false))
    $json
    if ($NprGodot) {
        $installed = Invoke-Beaver 'project.npr.install' @{ id = $project.id; godot = $NprGodot }
        [IO.File]::WriteAllText((Join-Path $round 'npr-install-proof.json'), ($installed | ConvertTo-Json -Depth 20), [Text.UTF8Encoding]::new($false))
        Write-Output 'NPR package ready in the newly-created blank project.'
    }
} finally { Pop-Location }
