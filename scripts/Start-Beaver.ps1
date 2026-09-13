$ErrorActionPreference = 'Stop'
$systemDirectory = [Environment]::SystemDirectory
if (-not [Environment]::Is64BitProcess) {
    $systemDirectory = Join-Path $env:WINDIR 'Sysnative'
}
foreach ($dll in @('vcruntime140.dll', 'vcruntime140_1.dll')) {
    if (-not (Test-Path -LiteralPath (Join-Path $systemDirectory $dll) -PathType Leaf)) {
        Add-Type -AssemblyName PresentationFramework
        [System.Windows.MessageBox]::Show('Install Microsoft Visual C++ v14 Redistributable (x64), then start Beaver again. Download: https://aka.ms/vc14/vc_redist.x64.exe', 'Beaver - Visual C++ runtime required') | Out-Null
        exit 1
    }
}
$runtimeId = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
$keys = @(
    "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$runtimeId",
    "HKCU:\Software\Microsoft\EdgeUpdate\Clients\$runtimeId"
)
$ready = $false
foreach ($key in $keys) {
    $value = Get-ItemProperty -LiteralPath $key -Name pv -ErrorAction SilentlyContinue
    $version = $null
    if ($value -and [version]::TryParse([string]$value.pv, [ref]$version) -and $version -gt [version]'0.0.0.0') {
        $ready = $true
    }
}
if (-not $ready) {
    Add-Type -AssemblyName PresentationFramework
    [System.Windows.MessageBox]::Show('Install Microsoft Edge WebView2 Evergreen Runtime, then start Beaver again. Download: https://developer.microsoft.com/microsoft-edge/webview2/', 'Beaver - WebView2 required') | Out-Null
    exit 1
}
Start-Process -FilePath (Join-Path $PSScriptRoot 'Beaver.exe') -WorkingDirectory $PSScriptRoot
