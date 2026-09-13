$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead($Archive)
try {
    $versions = @($zip.Entries | Where-Object { $_.FullName -ceq 'templates/version.txt' })
    if ($versions.Count -ne 1 -or $versions[0].Length -gt 128) { throw 'Invalid template version entry' }
    $reader = [IO.StreamReader]::new($versions[0].Open())
    try {
        $chars = New-Object char[] 129
        $count = $reader.ReadBlock($chars, 0, 129)
        if ($count -gt 128) { throw 'Template version is too large' }
        $version = [string]::new($chars, 0, $count).Trim()
    } finally { $reader.Dispose() }
    if ($version -cne $Expected) { throw 'Template version does not match engine' }
    foreach ($name in @('windows_release_x86_64.exe', 'windows_debug_x86_64.exe')) {
        $entries = @($zip.Entries | Where-Object { $_.FullName -ceq ('templates/' + $name) })
        if ($entries.Count -ne 1 -or $entries[0].Length -lt 1024 -or $entries[0].Length -gt 536870912) { throw 'Invalid Windows template entry' }
        $inputStream = $entries[0].Open()
        try {
            $outputStream = [IO.File]::Open([IO.Path]::Combine($Stage, $name), [IO.FileMode]::CreateNew, [IO.FileAccess]::Write)
            try {
                $buffer = New-Object byte[] 65536
                $total = 0L
                while (($count = $inputStream.Read($buffer, 0, $buffer.Length)) -gt 0) {
                    $total += $count
                    if ($total -gt $entries[0].Length -or $total -gt 536870912) { throw 'Template exceeds declared size' }
                    $outputStream.Write($buffer, 0, $count)
                }
                if ($total -ne $entries[0].Length) { throw 'Template entry is truncated' }
            } finally { $outputStream.Dispose() }
        } finally { $inputStream.Dispose() }
    }
    [IO.File]::WriteAllText([IO.Path]::Combine($Stage, 'version.txt'), $version, [Text.UTF8Encoding]::new($false))
} finally { $zip.Dispose() }
