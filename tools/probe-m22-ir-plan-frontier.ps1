<#
.SYNOPSIS
M22 S5 IR-ingestion frontier probe (STEP-0496).

.DESCRIPTION
Probes the guest IR-ingestion layer — the `run` emission-plan extraction in
selfhost/codegen.sico, driven by the `--emit-ir-plan` mode of
selfhost/compiler.sico — with the shapes the IR-driven component emitter
needs. Every probe is a source the guest must either plan (the plan text
comes back on stdout) or refuse with a typed identity (no silent fallback,
ADR-0016 decision 6).

This is the layer behind the frontend frontier probed by
tools/probe-m22-frontend-frontier.ps1: the frontend decides whether a shape
can be lowered at all, this probe decides whether the lowered IR can be
ingested into an emission plan.

Fuel: the byte-oriented IR scanner costs more fuel than the runner CLI
default (10M) allows — the plan mode needs between 10M and 20M on the
Script-profile fixture, while the plain IR mode needs between 5M and 10M. The
probe therefore drives `sico-runner` directly with an explicit budget
(the same 800B budget the selfhost fixture suite uses) instead of hiding the
cost behind the CLI default.

Exit codes: 0 = frontier unchanged (all probes match the recorded table),
1 = frontier moved (a probe changed verdict) or a probe produced an
unexpected outcome.

Usage:
  pwsh -File tools/probe-m22-ir-plan-frontier.ps1
  pwsh -File tools/probe-m22-ir-plan-frontier.ps1 -Json
#>
[CmdletBinding()]
param(
    [switch]$Json,
    [int64]$Fuel = 800000000000
)

$ErrorActionPreference = 'Continue'

$repoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$cli = Join-Path $repoRoot 'target\debug\sico.exe'
$runner = Join-Path $repoRoot 'runner\sico-runner\target\debug\sico-runner.exe'
$workDir = Join-Path $repoRoot 'target\tmp\ir-plan-frontier'
$component = Join-Path $workDir 'compiler.component.wasm'

if (-not (Test-Path $cli)) {
    Write-Error "sico CLI not found at $cli (build with: cargo build -p sico-cli)"
    exit 1
}
if (-not (Test-Path $runner)) {
    Write-Error "sico-runner not found at $runner (build with: cargo build --manifest-path runner/sico-runner/Cargo.toml --bins)"
    exit 1
}

New-Item -ItemType Directory -Force -Path $workDir | Out-Null

# The self-host compiler component, built through the Script profile.
if (Test-Path -LiteralPath $component) {
    & rm -- $component
}
& $cli build --profile script-v0 --output $component (Join-Path $repoRoot 'selfhost\compiler.sico')
if ($LASTEXITCODE -ne 0) {
    Write-Error "self-host compiler component build failed (exit $LASTEXITCODE)"
    exit 1
}

# Each case: name, source, expected verdict ('planned' | 'refused:<CODE>').
$cases = @(
    @{
        Name = 'plan-script-profile-run'
        Source = "record ScriptInput:`n  field arguments: List[Text]`n  field stdin: Bytes`nend record`n`nrecord ScriptOutput:`n  field stdout: Bytes`n  field stderr: Bytes`n  field exit_code: I64`nend record`n`nenum ScriptErrorCode:`n  case InvalidInput`nend enum`n`nrecord ScriptError:`n  field code: ScriptErrorCode`n  field message: Text`nend record`n`nfunction run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:`n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))`nend function`n"
        Expected = 'planned'
    }
    @{
        Name = 'plan-variant-scalar'
        Source = "enum NumericError:`n  case Invalid`nend enum`n`nfunction run() returns Result[I64, NumericError]:`n  return ok(I64.literal(1))`nend function`n"
        Expected = 'planned'
    }
    @{
        Name = 'plan-variant-record'
        Source = "record ScriptError:`n  field code: I64`n  field message: Text`nend record`n`nfunction run() returns Result[I64, ScriptError]:`n  return error(ScriptError(code: I64.literal(2), message: `"bad`"))`nend function`n"
        Expected = 'planned'
    }
    @{
        Name = 'plan-call'
        Source = "function helper() returns I64:`n  return I64.literal(1)`nend function`n`nfunction run() returns I64:`n  return helper()`nend function`n"
        Expected = 'planned'
    }
    @{
        Name = 'plan-missing-run'
        Source = "function probe() returns I64:`n  return I64.literal(0)`nend function`n"
        Expected = 'refused:ERR:E-SH-CG-IR-PLAN'
    }
    @{
        Name = 'plan-unsupported-op'
        Source = "function run() returns I64:`n  let value = I64.literal(1)`n  set value = I64.literal(2)`n  return value`nend function`n"
        Expected = 'refused:ERR:E-SH-CG-IR-OP'
    }
    @{
        Name = 'plan-unsupported-type'
        Source = "function run(input: Map[Text, Text]) returns I64:`n  return I64.literal(0)`nend function`n"
        Expected = 'refused:ERR:E-SH-CG-IR-TYPE'
    }
    @{
        Name = 'plan-unsupported-terminator'
        Source = "function run(flag: Bool) returns I64:`n  if flag:`n    return I64.literal(1)`n  end if`n  return I64.literal(2)`nend function`n"
        Expected = 'refused:ERR:E-SH-CG-IR-TERMINATOR'
    }
)

function Invoke-PlanProbe([string]$sourcePath) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $runner
    $psi.Arguments = '--fuel ' + $Fuel + ' "' + $component + '" -- --emit-ir-plan'
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = $psi
    $null = $proc.Start()
    $bytes = [System.IO.File]::ReadAllBytes($sourcePath)
    $proc.StandardInput.BaseStream.Write($bytes, 0, $bytes.Length)
    $proc.StandardInput.BaseStream.Close()
    $stdout = $proc.StandardOutput.ReadToEnd()
    $stderr = $proc.StandardError.ReadToEnd()
    $proc.WaitForExit()
    return [pscustomobject]@{
        Exit = $proc.ExitCode
        Stdout = $stdout
        Stderr = $stderr
    }
}

$results = @()
foreach ($case in $cases) {
    $path = Join-Path $workDir ($case.Name + '.sico')
    [System.IO.File]::WriteAllText($path, ($case.Source -replace "`r`n", "`n"), (New-Object System.Text.UTF8Encoding($false)))
    $run = Invoke-PlanProbe $path
    $flat = (($run.Stdout + ' ' + $run.Stderr) -replace "`r?`n", ' ')
    $marker = $flat.IndexOf('ERR:')
    if ($marker -ge 0) {
        $code = $flat.Substring($marker)
        $end = $code.IndexOf('"')
        if ($end -gt 0) { $code = $code.Substring(0, $end) }
        $code = $code.TrimEnd('\', '"', ' ')
        $verdict = 'refused:' + $code
    } elseif ($run.Exit -eq 0 -and $run.Stdout -match '(?m)^(param|inst|op|term|record) ') {
        $verdict = 'planned'
    } else {
        $verdict = 'failed:exit=' + $run.Exit + ' ' + $flat.Substring(0, [Math]::Min(80, $flat.Length))
    }
    $results += [pscustomobject]@{
        Name = $case.Name
        Expected = $case.Expected
        Actual = $verdict
        Match = ($case.Expected -eq $verdict)
    }
}

if ($Json) {
    $results | ConvertTo-Json -Depth 4
} else {
    foreach ($result in $results) {
        $status = if ($result.Match) { 'OK  ' } else { 'MOVED' }
        Write-Output ("{0} {1,-30} expected={2,-34} actual={3}" -f $status, $result.Name, $result.Expected, $result.Actual)
    }
    $moved = @($results | Where-Object { -not $_.Match })
    if ($moved.Count -eq 0) {
        Write-Output 'PLAN_FRONTIER_UNCHANGED'
        exit 0
    }
    Write-Output ('PLAN_FRONTIER_MOVED count=' + $moved.Count)
    exit 1
}
