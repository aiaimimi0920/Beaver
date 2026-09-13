param(
  [Parameter(Mandatory = $true)][string]$ExecutablePath,
  [Parameter(Mandatory = $true)][string]$OutputPath,
  [Parameter(Mandatory = $true)][long]$WindowHandle
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class BeaverIconProbe {
  [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
  public static extern uint ExtractIconEx(string file, int index, IntPtr[] large, IntPtr[] small, uint count);
  [DllImport("user32.dll")]
  public static extern bool DestroyIcon(IntPtr icon);
  [DllImport("user32.dll")]
  public static extern IntPtr SendMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll", EntryPoint = "GetClassLongPtrW")]
  public static extern IntPtr GetClassLongPtr(IntPtr window, int index);
}
'@

function Measure-BrandIcon([IntPtr]$Handle, [string]$Name) {
  if ($Handle -eq [IntPtr]::Zero) { throw "Missing native icon: $Name" }
  $icon = [Drawing.Icon]::FromHandle($Handle)
  $bitmap = $null
  try {
    $bitmap = $icon.ToBitmap()
    $green = 0
    $yellow = 0
    for ($y = 0; $y -lt $bitmap.Height; $y++) {
      for ($x = 0; $x -lt $bitmap.Width; $x++) {
        $pixel = $bitmap.GetPixel($x, $y)
        if ($pixel.A -ne 255) { continue }
        if ($pixel.R -eq 34 -and $pixel.G -eq 197 -and $pixel.B -eq 94) { $green++ }
        if ($pixel.R -eq 217 -and $pixel.G -eq 255 -and $pixel.B -eq 56) { $yellow++ }
      }
    }
    $bitmap.Save((Join-Path $OutputPath ("00-" + $Name + '.png')), [Drawing.Imaging.ImageFormat]::Png)
    if ($green -eq 0 -or $yellow -eq 0) { throw "Native icon does not contain both approved colors: $Name" }
    [ordered]@{ name = $Name; width = $bitmap.Width; height = $bitmap.Height; green = $green; yellow = $yellow }
  } finally {
    if ($null -ne $bitmap) { $bitmap.Dispose() }
    $icon.Dispose()
  }
}

$ExecutablePath = (Resolve-Path -LiteralPath $ExecutablePath).ProviderPath
$OutputPath = (Resolve-Path -LiteralPath $OutputPath).ProviderPath
$results = @()
foreach ($source in @(
  @{ path = $ExecutablePath; name = 'exe' }
)) {
  $large = New-Object IntPtr[] 1
  $small = New-Object IntPtr[] 1
  try {
    $count = [BeaverIconProbe]::ExtractIconEx($source.path, 0, $large, $small, 1)
    if ($count -eq 0) { throw "Windows cannot extract icons: $($source.path)" }
    $results += Measure-BrandIcon $large[0] ($source.name + '-large')
    $results += Measure-BrandIcon $small[0] ($source.name + '-small')
  } finally {
    foreach ($handle in @($large[0], $small[0])) {
      if ($handle -ne [IntPtr]::Zero) { [void][BeaverIconProbe]::DestroyIcon($handle) }
    }
  }
}

# WM_GETICON returns borrowed handles owned by the running BrowserWindow.
foreach ($size in @(0, 1)) {
  $handle = [BeaverIconProbe]::SendMessage([IntPtr]$WindowHandle, 0x7f, [IntPtr]$size, [IntPtr]::Zero)
  $source = 'window'
  if ($handle -eq [IntPtr]::Zero) {
    $index = if ($size -eq 1) { -14 } else { -34 }
    $handle = [BeaverIconProbe]::GetClassLongPtr([IntPtr]$WindowHandle, $index)
    $source = 'class'
  }
  if ($handle -eq [IntPtr]::Zero -and $size -eq 1) {
    # Tao sets ICON_SMALL; the shell can use that when no separate large icon exists.
    $handle = [BeaverIconProbe]::SendMessage([IntPtr]$WindowHandle, 0x7f, [IntPtr]::Zero, [IntPtr]::Zero)
    $source = 'small-fallback'
  }
  $measured = Measure-BrandIcon $handle ("window-" + $size + '-' + $source)
  if ($measured.width -lt 32) { throw "Runtime window icon is too small: $($measured.width)" }
  $results += $measured
}
[ordered]@{ executablePath = $ExecutablePath; icons = $results } | ConvertTo-Json -Depth 4 -Compress
