<#
SYNOPSIS
  STEP-0562 fuel-wall profiler: attributes where the representative
  FuelExhausted sites burn their fuel, by driving the prebuilt runner test
  binary's existing #[ignore] fuel probes.

DISCIPLINE (AGENTS.md §10.2 + §9)
  - Read-only against guest sources: selfhost/parser.sico and
    selfhost/codegen.sico are NOT modified. crates/ production code is NOT
    modified. runner/sico-runner/tests/selfhost_compiler.rs is NOT modified
    (only the designated agent may write it).
  - No fuel / memory / timeout limit is raised. SICO_M22_PROBE_FUEL only caps
    the probe's bisection CEILING for measurement; it never changes the frozen
    800e9 ledger budget or any runner limit.
  - Measurement only. This tool closes no exit gate and flips no verdict.

ENVIRONMENT BLOCKER (recorded honestly)
  This host has no gcc.exe, and the runner workspace cannot be recompiled:
    cargo check --locked --offline --manifest-path runner/sico-runner/Cargo.toml --tests
  fails with `cc-rs: failed to find tool "gcc.exe"` (exit 101). Therefore no new
  test-side Rust arm and no wasmtime epoch/opcode sampling hook can be built
  here. Profiling is restricted to the probes already compiled into the prebuilt
  test binary, selected through environment variables. Opcode-level sampling
  remains an open gap that needs a host with a C toolchain.

METHOD
  Two probe families already in the prebuilt binary are used:
    A) emit-only injection probe
       the_component_emission_fuel_of_a_real_match_group_is_measured
       Feeds the site's canonical IR (built by the Rust oracle, no guest fuel)
       straight to the guest emitter, so it isolates EMIT fuel from PARSE fuel.
       Selects real ledger sites through SICO_M22_PROBE_GROUPS=path:function,....
    B) full-compiler (ledger-path) probe
       the_component_emission_fuel_of_a_real_match_group_through_the_compiler_is_measured
       Runs the whole guest compiler (--emit-component-hex) on the site's
       closure, paying guest PARSE + IR-build + EMIT. Comparing A and B on the
       same site splits the wall into parse vs emit.
    C) synthetic scaling probe (STEP-0544-style shape)
       the_emitter_fuel_scaling_in_read_local_count_is_measured
       Fixed synthetic families (read_local count N, module function count F,
       EMIT_ONLY / EMIT_SPLIT / EMIT_PADDED). Provides the controlled curve that
       real sites cannot.

OUTPUT
  <OutDir>/fuel-wall-raw.log        verbatim probe stdout
  <OutDir>/fuel-wall-sites.csv      per-site ranking (function, ir_bytes, fuel)
  <OutDir>/fuel-wall-parse-vs-emit.csv
  <OutDir>/fuel-wall-synthetic.log  scaling arm lines

USAGES
  powershell -NoProfile -File tools/profile-m22-fuel-wall.ps1 -Probe sites
  powershell -NoProfile -File tools/profile-m22-fuel-wall.ps1 -Probe parsevs
  powershell -NoProfile -File tools/profile-m22-fuel-wall.ps1 -Probe synthetic
#>
[CmdletBinding()]
param(
  [ValidateSet("sites","parsevs","synthetic","all")]
  [string]$Probe = "all",

  [string]$TestBinary = "",

  [string]$OutDir = "target/tmp/step0562",

  # Bisection ceiling for the probe search. This is a MEASUREMENT cap only; the
  # frozen ledger budget (800e9) and every runner limit stay untouched.
  [uint64]$Ceiling = 200000000000,

  # Comma-separated path:function list for the real-site probes. Defaults to one
  # representative per top FuelExhausted function family in
  # selfhost/match-component-coverage-v1.json.
  [string]$Groups = "",

  # Timeout per single probe invocation, seconds.
  [int]$ProbeTimeoutSec = 3000
)

$ErrorActionPreference = "Continue"

function Resolve-Toolchain {
  $env:RUSTUP_TOOLCHAIN = "1.98.0-x86_64-pc-windows-gnu"
}

function Resolve-TestBinary {
  param([string]$Explicit)
  if ($Explicit -ne "") { return $Explicit }
  $deps = "runner/sico-runner/target/debug/deps"
  $cands = Get-ChildItem -Path $deps -Filter "selfhost_compiler-*.exe" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending
  if (-not $cands -or $cands.Count -eq 0) {
    throw "no prebuilt selfhost_compiler-*.exe under $deps; the runner could not be recompiled on this host (no gcc.exe), so a prebuilt binary is required"
  }
  return $cands[0].FullName
}

function Default-Groups {
  # One representative site id per top FuelExhausted function family, read from
  # the frozen ledger. Ranking is over distinct guest compiler functions.
  $ledger = "selfhost/match-component-coverage-v1.json"
  $json = Get-Content $ledger -Raw | ConvertFrom-Json
  $ex = $json.sites | Where-Object { $_.verdict -match "FuelExhausted" }
  $fams = $ex | Group-Object function | Sort-Object Count -Descending | Select-Object -First 14
  $pairs = foreach ($fam in $fams) {
    $rep = $fam.Group | Select-Object -First 1
    "{0}:{1}" -f $rep.path, $rep.function
  }
  # Add the two named hot sites carried through STEP-0555/0556 for continuity.
  $pairs += "parser.sico:general_while_function_ir"
  $pairs += "parser.sico:fixed_literal_const_segment"
  # path:function pairs are unique per function; dedupe while keeping order.
  return ($pairs | Select-Object -Unique) -join ","
}

function Write-Header {
  param([string]$Path, [string]$Title)
  $stamp = (Get-Date).ToUniversalTime().ToString("o")
  Add-Content -Path $Path -Value ""
  Add-Content -Path $Path -Value "===== $Title :: $stamp :: ceiling=$Ceiling ====="
}

function Invoke-Probe {
  param(
    [string]$Binary,
    [string]$TestName,
    [string]$GroupsValue,
    [uint64]$FuelCeiling,
    [string]$RawLog,
    [string]$Title,
    [int]$TimeoutSec
  )
  Write-Header -Path $RawLog -Title $Title
  $env:SICO_M22_PROBE_GROUPS = $GroupsValue
  $env:SICO_M22_PROBE_FUEL = [string]$FuelCeiling
  $started = Get-Date
  # Run from the repo root so include_str!-relative frozen sources resolve and
  # the component cache directory is shared.
  $out = & $Binary $TestName --exact --ignored --nocapture 2>&1
  $code = $LASTEXITCODE
  $elapsed = ((Get-Date) - $started).TotalSeconds
  Add-Content -Path $RawLog -Value ($out | Out-String)
  Add-Content -Path $RawLog -Value "----- exit=$code wall_s=$([math]::Round($elapsed,1)) -----"
  return [pscustomobject]@{
    ExitCode = $code
    WallSec  = [math]::Round($elapsed,1)
    Lines    = $out
  }
}

function Parse-SiteLines {
  param($Lines, [string]$Prefix)
  # Lines look like (fields are space-separated EXCEPT the fuel value, which
  # carries an internal TAB between its low and high, so each field is pulled
  # with its own anchored regex instead of one greedy whole-line match):
  #   COMPONENT_FUEL <path>:<function> ir_bytes=<n> component_bytes=<n> fuel=<v>\t- search_ms=<n>
  #   COMPILER_COMPONENT_FUEL <path>:<function> source_bytes=<n> component_bytes=<n> fuel=<v>\t- search_ms=<n>
  $rows = @()
  foreach ($line in $Lines) {
    $s = "$line"
    if (-not $s.StartsWith($Prefix)) { continue }
    $ir = $null; $cb = $null; $fuel = $null; $ms = $null
    if ($s -match "(?:ir_bytes|source_bytes)=(\d+)") { $ir = [uint64]$Matches[1] }
    if ($s -match "component_bytes=(\d+)") { $cb = [uint64]$Matches[1] }
    if ($s -match "fuel=(.*?)\s+search_ms=") { $fuel = $Matches[1] }
    if ($s -match "search_ms=(\d+)") { $ms = [uint64]$Matches[1] }
    if ($null -eq $ir -or $null -eq $fuel) { continue }
    # site identity = the first "<path>:<function>" token after the prefix
    $head = $s.Substring($Prefix.Length).TrimStart()
    $siteTok = ($head -split "\s+")[0]
    $colon = $siteTok.IndexOf(":")
    if ($colon -lt 0) { continue }
    $rows += [pscustomobject]@{
      prefix          = $Prefix
      path            = $siteTok.Substring(0, $colon)
      function        = $siteTok.Substring($colon + 1)
      ir_bytes        = $ir
      component_bytes = $(if ($null -ne $cb) { $cb } else { [uint64]0 })
      fuel_raw        = $fuel
      search_ms       = $(if ($null -ne $ms) { $ms } else { [uint64]0 })
    }
  }
  return $rows
}

function Fuel-LowerBound {
  # Return a comparable number: the bracket's high if measured, else the
  # ceiling if exhausted (a strict lower bound on true demand). The fuel value
  # may carry a literal tab before a trailing "-", so match with \s.
  param([string]$FuelRaw, [uint64]$CeilingValue)
  if ($FuelRaw -match "^(\d+)\s+(\d+)$") { return [uint64]$Matches[2] }
  if ($FuelRaw -match "^exhausted") { return $CeilingValue }
  # refused / trap have no fuel demand; report 0 so they sort last.
  return [uint64]0
}

function Fuel-Kind {
  param([string]$FuelRaw)
  if ($FuelRaw -match "^\d+\s+\d+$") { return "measured" }
  if ($FuelRaw -match "^exhausted") { return "exhausted" }
  if ($FuelRaw -match "^refused") { return "refused" }
  if ($FuelRaw -match "^trap") { return "trap" }
  return "other"
}

# --------------------------------------------------------------------------
Resolve-Toolchain
$bin = Resolve-TestBinary -Explicit $TestBinary
if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Force -Path $OutDir | Out-Null }
$rawLog = Join-Path $OutDir "fuel-wall-raw.log"
$sitesCsv = Join-Path $OutDir "fuel-wall-sites.csv"
$pveCsv = Join-Path $OutDir "fuel-wall-parse-vs-emit.csv"
$synLog = Join-Path $OutDir "fuel-wall-synthetic.log"

"binary=$bin" | Add-Content -Path $rawLog
"ceiling=$Ceiling" | Add-Content -Path $rawLog

$groups = if ($Groups -ne "") { $Groups } else { Default-Groups }
"profile groups=$groups" | Add-Content -Path $rawLog

$doSites = $Probe -in @("sites","all")
$doParse = $Probe -in @("parsevs","all")
$doSynth = $Probe -in @("synthetic","all")

# ---- A) emit-only, per real function family -> top-N ranking -------------
$siteRows = @()
if ($doSites) {
  $res = Invoke-Probe -Binary $bin `
    -TestName the_component_emission_fuel_of_a_real_match_group_is_measured `
    -GroupsValue $groups -FuelCeiling $Ceiling -RawLog $rawLog `
    -Title "A emit-only real-site fuel bracket" -TimeoutSec $ProbeTimeoutSec
  $siteRows = Parse-SiteLines -Lines $res.Lines -Prefix "COMPONENT_FUEL"
  $siteRows | ForEach-Object {
    $_ | Add-Member -NotePropertyName fuel_lb -NotePropertyValue (Fuel-LowerBound -FuelRaw $_.fuel_raw -CeilingValue $Ceiling)
    $_ | Add-Member -NotePropertyName kind -NotePropertyValue (Fuel-Kind -FuelRaw $_.fuel_raw)
    $ratio = if ($_.ir_bytes -gt 0) { [math]::Round($_.fuel_lb / $_.ir_bytes, 1) } else { 0 }
    # fuel / IR^2, expressed stably as (fuel/1e9) / (IR/1e3)^2  ==  G per kb^2.
    $ratio2 = 0.0
    if ($_.ir_bytes -gt 0) {
      $kb = [double]$_.ir_bytes / 1000.0
      $g = [double]$_.fuel_lb / 1e9
      $ratio2 = [math]::Round($g / ($kb * $kb), 4)
    }
    $_ | Add-Member -NotePropertyName fuel_per_ir_byte -NotePropertyValue $ratio
    $_ | Add-Member -NotePropertyName g_per_kb2 -NotePropertyValue $ratio2
  }
  $siteRows | Sort-Object @{Expression="ir_bytes";Descending=$true}, @{Expression="fuel_lb";Descending=$true} |
    Export-Csv -Path $sitesCsv -NoTypeInformation -Encoding UTF8
  "A: wrote $($siteRows.Count) site rows -> $sitesCsv" | Add-Content -Path $rawLog
}

# ---- B) full-compiler (ledger path) on the same sites -> parse vs emit ---
$parseRows = @()
if ($doParse) {
  # A subset keeps wall time bounded; the full-compiler path is the slow one.
  $subset = ($groups -split "," | Select-Object -First 6) -join ","
  $res = Invoke-Probe -Binary $bin `
    -TestName the_component_emission_fuel_of_a_real_match_group_through_the_compiler_is_measured `
    -GroupsValue $subset -FuelCeiling $Ceiling -RawLog $rawLog `
    -Title "B full-compiler (parse+emit) real-site fuel bracket" -TimeoutSec $ProbeTimeoutSec
  $parseRows = Parse-SiteLines -Lines $res.Lines -Prefix "COMPILER_COMPONENT_FUEL"
  $parseRows | ForEach-Object {
    $_ | Add-Member -NotePropertyName fuel_lb -NotePropertyValue (Fuel-LowerBound -FuelRaw $_.fuel_raw -CeilingValue $Ceiling)
    $_ | Add-Member -NotePropertyName kind -NotePropertyValue (Fuel-Kind -FuelRaw $_.fuel_raw)
  }
  $parseRows | Sort-Object fuel_lb -Descending |
    Export-Csv -Path $pveCsv -NoTypeInformation -Encoding UTF8
  "B: wrote $($parseRows.Count) rows -> $pveCsv" | Add-Content -Path $rawLog
}

# ---- C) synthetic scaling (STEP-0544-style) ------------------------------
if ($doSynth) {
  Write-Header -Path $synLog -Title "C synthetic emitter fuel scaling"
  $env:SICO_M22_PROBE_GROUPS = ""
  $env:SICO_M22_PROBE_FUEL = [string]$Ceiling
  $started = Get-Date
  $out = & $bin the_emitter_fuel_scaling_in_read_local_count_is_measured --exact --ignored --nocapture 2>&1
  $code = $LASTEXITCODE
  $elapsed = ((Get-Date) - $started).TotalSeconds
  Add-Content -Path $synLog -Value ($out | Out-String)
  Add-Content -Path $synLog -Value "----- exit=$code wall_s=$([math]::Round($elapsed,1)) -----"
  "C: synthetic scaling -> $synLog (exit=$code)" | Add-Content -Path $rawLog
}

# ---- Console summary -----------------------------------------------------
if ($siteRows.Count -gt 0) {
  "`n=== TOP-N emit-only fuel-wall ranking (IR-size driven; ceiling=$Ceiling) ==="
  $siteRows | Sort-Object @{Expression="ir_bytes";Descending=$true} | Select-Object -First 15 |
    Format-Table function, path, ir_bytes, kind, fuel_lb, fuel_per_ir_byte, g_per_kb2, search_ms -AutoSize
}
if ($parseRows.Count -gt 0) {
  "`n=== parse+emit (full compiler) vs emit-only ==="
  foreach ($p in $parseRows) {
    $emit = $siteRows | Where-Object { $_.function -eq $p.function -and $_.path -eq $p.path } | Select-Object -First 1
    "{0,-40} parse+emit_lb={1,-14} emit_only_lb={2}" -f $p.function, $p.fuel_lb, $(if ($emit) { $emit.fuel_lb } else { "n/a" })
  }
}
