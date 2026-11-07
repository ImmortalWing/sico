<#
STEP-0600 fuel-sensitivity analysis: aggregates SENS_FUEL probe output into a
per-function and per-site flip report for the D8 decision.

Inputs (all produced by the m22_fuel_sensitivity probe or the ledger):
  -Log1p6t    <path>  SENS_FUEL log for the 1.6T (2x) sweep (required)
  -Log3p2t    <path>  SENS_FUEL log for the 3.2T (4x) sweep (optional)
  -SitesMap   <path>  "path:function=siteCount,..." per-group site counts
                      (default target/tmp/m22-sens-sites-per-group.csv)

Output: a per-function table (function, source_bytes, outcome@1.6T, outcome@3.2T,
flip2x, flip4x) and per-site totals (sites flipped at 2x / 4x / neither).
Measurement only: flips no gate, raises no limit.
#>
[CmdletBinding()]
param(
  [string]$Log1p6t = "target/tmp/m22-sens-1p6t.log",
  [string]$Log3p2t = "",
  [string]$SitesMap = "target/tmp/m22-sens-sites-per-group.csv"
)

function Parse-FuelLog {
  param([string]$Path)
  $rows = @{}
  if (-not (Test-Path $Path)) { return $rows }
  Get-Content $Path | Where-Object { $_ -match '^SENS_FUEL ' } | ForEach-Object {
    if ($_ -match '^SENS_FUEL ([^ ]+:[^ ]+) fuel=(\d+) outcome=([^ ]+)( component_bytes=(\d+))?( component_equal=(\S+))?.* wall_ms=(\d+) source_bytes=(\d+)') {
      $key = $Matches[1]; $fuel = [uint64]$Matches[2]
      $rows[$key] = [pscustomobject]@{
        function  = $key
        fuel      = $fuel
        outcome   = $Matches[3]
        equal     = $Matches[7]
        wall_ms   = [uint64]$Matches[8]
        src       = [uint64]$Matches[9]
      }
    }
  }
  $rows
}

function Parse-SitesMap {
  param([string]$Path)
  $map = @{}
  if (-not (Test-Path $Path)) { return $map }
  # The file is a single comma-separated line of "key=count" pairs.
  (Get-Content $Path -Raw) -split ',' | ForEach-Object {
    $p = $_ -split '='
    if ($p.Count -eq 2) { $map[$p[0].Trim()] = [int]$p[1].Trim() }
  }
  $map
}

$r1 = Parse-FuelLog -Path $Log1p6t
$r3 = if ($Log3p2t -ne "") { Parse-FuelLog -Path $Log3p2t } else { @{} }
$map = Parse-SitesMap -Path $SitesMap

"=== per-function flip report (unit = distinct path:function) ==="
"function                                  source  outcome@1.6T          outcome@3.2T          flip2x flip4x sites"
$keys = ($r1.Keys + $r3.Keys) | Sort-Object -Unique
foreach ($k in $keys) {
  $a = $r1[$k]
  $b = $r3[$k]
  if ($null -eq $a) { continue }
  $src = $a.src
  $flip2 = ($a.outcome -eq 'component' -and $a.equal -eq 'yes')
  $flip4 = $false
  if ($flip2) { $flip4 = $true }
  elseif ($null -ne $b) { $flip4 = ($b.outcome -eq 'component' -and $b.equal -eq 'yes') }
  $o1 = if ($a.outcome -eq 'component') { if ($a.equal -eq 'yes') { 'COMPONENT=equal' } else { 'COMPONENT=mis' } } else { $a.outcome }
  $o3 = if ($null -ne $b) { if ($b.outcome -eq 'component') { if ($b.equal -eq 'yes') { 'COMPONENT=equal' } else { 'COMPONENT=mis' } } else { $b.outcome } } else { '-' }
  $n = 0; if ($map.ContainsKey($k)) { $n = $map[$k] }
  "{0,-41} {1,7} {2,-22} {3,-22} {4,-6} {5,-6} {6}" -f $k, $src, $o1, $o3, $(if ($flip2) {'Y'} else {'N'}), $(if ($flip4) {'Y'} else {'N'}), $n
}

# per-site totals (map function flip -> site count)
$sites2 = 0; $sites4 = 0; $sitesNone = 0; $sitesMis = 0
foreach ($k in $r1.Keys) {
  $n = 0; if ($map.ContainsKey($k)) { $n = $map[$k] }
  $a = $r1[$k]
  $flip2 = ($a.outcome -eq 'component' -and $a.equal -eq 'yes')
  $flip4 = $false
  if ($flip2) { $flip4 = $true }
  elseif ($null -ne $r3[$k]) { $flip4 = ($r3[$k].outcome -eq 'component' -and $r3[$k].equal -eq 'yes') }
  $anyComponent = ($a.outcome -eq 'component') -or ($null -ne $r3[$k] -and $r3[$k].outcome -eq 'component')
  if ($anyComponent -and -not $flip2 -and -not $flip4) { $sitesMis += $n }
  if ($flip2) { $sites2 += $n } elseif ($flip4) { $sites4 += $n } else { $sitesNone += $n }
}
""
"=== per-site totals (of the 331 FuelExhausted sites) ==="
"flip at 2x (1.6T):   $sites2"
"flip at 4x (3.2T):   $sites4"
"not flip at 4x:      $sitesNone"
"component-but-mismatch: $sitesMis"
"measured functions:  $($r1.Count)"
