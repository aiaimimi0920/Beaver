import fs from "node:fs/promises";
import path from "node:path";

export type DesktopTool = "godot" | "blender";

// Read only application registration, shortcut targets and matching executable names.
// Portable editors often have no installer entry; never crawl user disks to find them.
export const windowsToolHintsScript = String.raw`
$ErrorActionPreference='SilentlyContinue'
$ProgressPreference='SilentlyContinue'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$hints = [System.Collections.Generic.List[string]]::new()
function Add-Hint($value) { if ($value -is [string] -and $value.Trim()) { $hints.Add($value) } }
foreach ($root in @('HKCU:\Software\Microsoft\Windows\CurrentVersion\App Paths','HKLM:\Software\Microsoft\Windows\CurrentVersion\App Paths')) {
  Get-ChildItem -LiteralPath $root | Where-Object { $_.PSChildName -match '^(godot.*|blender)\.exe$' } | ForEach-Object { Add-Hint $_.GetValue('') }
}
foreach ($root in @('HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*')) {
  Get-ItemProperty $root | Where-Object { $_.DisplayName -match '^(Godot|Blender)(\s|$)' } | ForEach-Object { Add-Hint $_.InstallLocation; Add-Hint $_.DisplayIcon }
}
foreach ($prefix in @('', 'Applications\')) {
  $parent = [Microsoft.Win32.Registry]::ClassesRoot.OpenSubKey($prefix)
  if ($parent) {
    foreach ($name in $parent.GetSubKeyNames()) {
      if ($name -match '^(godot|blender)') {
        $command = $parent.OpenSubKey($name+'\shell\open\command')
        if ($command) { Add-Hint $command.GetValue(''); $command.Dispose() }
      }
    }
    $parent.Dispose()
  }
}
$shell = New-Object -ComObject WScript.Shell
foreach ($root in @([Environment]::GetFolderPath('StartMenu'),[Environment]::GetFolderPath('CommonStartMenu'),[Environment]::GetFolderPath('Desktop'),[Environment]::GetFolderPath('CommonDesktopDirectory'))) {
  if ($root) { Get-ChildItem -LiteralPath $root -Filter *.lnk -Recurse -Depth 5 | Where-Object { $_.BaseName -match 'Godot|Blender' } | ForEach-Object { Add-Hint ($shell.CreateShortcut($_.FullName)).TargetPath } }
}
foreach ($key in @('HKCU:\Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Compatibility Assistant\Store','HKCU:\Software\Classes\Local Settings\Software\Microsoft\Windows\Shell\MuiCache')) {
  $item=Get-Item -LiteralPath $key
  if ($item) { $item.GetValueNames() | Where-Object { $_ -match '(?i)(^|\\)(godot[^\\]*|blender)\.exe(\.(FriendlyAppName|ApplicationCompany))?$' } | ForEach-Object { Add-Hint ($_ -replace '\.(FriendlyAppName|ApplicationCompany)$','') } }
}
ConvertTo-Json -InputObject @($hints | Select-Object -Unique) -Compress
`;

export function isToolExecutable(name: DesktopTool, file: string): boolean {
  const base = path.win32.basename(file);
  return name === "blender"
    ? /^blender\.exe$/i.test(base)
    : /^godot(?:[_.-].*)?\.exe$/i.test(base) &&
        !/(?:^|[_.-])(?:template|headless|server)(?:[_.-]|$)/i.test(base);
}

export function executableHint(value: string): string {
  const quoted = value.trim().match(/^"([^"]+)"/);
  if (quoted?.[1]) return quoted[1];
  return value.trim().replace(/(\.exe)(?:,\s*-?\d+|\s+[-"%]).*$/i, "$1");
}

export function launcherTarget(
  name: DesktopTool,
  text: string,
  launcher: string,
  env: NodeJS.ProcessEnv,
): string | undefined {
  // Resolve simple launchers without running shell code or reading personal Codex config.
  for (const match of text.matchAll(
    /^\s*@?(?:call\s+)?"([^"\r\n]+)"(?:\s+%[*1-9]|\s*$)/gim,
  )) {
    const raw = match[1];
    if (!raw) continue;
    const target = raw
      .replace(/%~dp0/gi, path.win32.dirname(launcher) + "\\")
      .replace(/%([^%]+)%/g, (original, key: string) => {
        const entry = Object.entries(env).find(
          ([k]) => k.toLowerCase() === key.toLowerCase(),
        );
        return entry?.[1] ?? original;
      });
    if (
      !target.includes("%") &&
      path.win32.isAbsolute(target) &&
      isToolExecutable(name, target)
    )
      return target;
  }
}

export async function resolveToolHints(
  name: DesktopTool,
  hints: string[],
  env: NodeJS.ProcessEnv = process.env,
): Promise<string | undefined> {
  const seen = new Set<string>();
  for (const hint of hints) {
    const file = executableHint(hint);
    if (!file || seen.has(file.toLowerCase())) continue;
    seen.add(file.toLowerCase());
    try {
      const stat = await fs.stat(file);
      if (stat.isDirectory()) {
        // Registered install roots, including Steam libraries and versioned portable packages.
        for (const dir of [file, path.join(file, "bin")]) {
          const entries = await fs.readdir(dir).catch(() => []);
          const files = entries
            .filter((entry) => isToolExecutable(name, entry))
            .sort(
              (a, b) =>
                Number(/console/i.test(a)) - Number(/console/i.test(b)) ||
                b.localeCompare(a, undefined, { numeric: true }),
            )
            .map((entry) => path.join(dir, entry));
          const regular = await Promise.all(
            files.map(async (candidate) =>
              (await fs.stat(candidate).catch(() => undefined))?.isFile()
                ? candidate
                : "",
            ),
          );
          const found = await resolveToolHints(name, regular, env);
          if (found) return found;
        }
      } else if (stat.isFile()) {
        if (/\.(cmd|bat)$/i.test(file) && stat.size < 32768) {
          const target = launcherTarget(
            name,
            await fs.readFile(file, "utf8"),
            file,
            env,
          );
          if (
            target &&
            (await fs.stat(target).catch(() => undefined))?.isFile()
          )
            return target;
        } else if (isToolExecutable(name, file)) {
          const gui = file.replace(/(?:_console|\.console)(?=\.exe$)/i, "");
          if (
            gui !== file &&
            (await fs.stat(gui).catch(() => undefined))?.isFile()
          )
            return gui;
          return file;
        } else if (name === "blender" && /blender-launcher\.exe$/i.test(file)) {
          const target = path.join(path.dirname(file), "blender.exe");
          if ((await fs.stat(target).catch(() => undefined))?.isFile())
            return target;
        }
      }
    } catch {}
  }
}
