param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) { $cargo = (Get-Command cargo -ErrorAction Stop).Source }
. (Join-Path $root 'tools\lib\native-command.ps1')

Invoke-NativeChecked $cargo @(
    'test', '--offline', '--locked',
    '-p', 'sico-parser', '-p', 'sico-diagnostics', '-p', 'sico-format',
    '-p', 'sico-hir', '-p', 'sico-language-server', '-p', 'sico-ai-tools',
    '-p', 'sico-cli'
) 'STEP-0092 frontend/editor/AI tests failed'
Invoke-NativeChecked $cargo @('build', '--offline', '--locked', '-p', 'sico-cli') 'sico build failed'
& (Join-Path $root 'tools\validate-diagnostics.ps1') -RepositoryRoot $root
& (Join-Path $root 'tools\validate-error-taxonomy.ps1') -RepositoryRoot $root

$sico = Join-Path $root 'target\debug\sico.exe'

function Invoke-Sico([string]$arguments, [string]$stdinText) {
    $psi = [Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $sico
    $psi.Arguments = $arguments
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($psi)
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $process.StandardInput.Write($stdinText)
    $process.StandardInput.Close()
    if (-not $process.WaitForExit(10000)) {
        & taskkill.exe /PID $process.Id /T /F | Out-Null
        $process.WaitForExit()
        throw "sico command timed out: $arguments"
    }
    return [pscustomobject]@{
        Exit = $process.ExitCode
        Stdout = $stdoutTask.GetAwaiter().GetResult()
        Stderr = $stderrTask.GetAwaiter().GetResult()
    }
}

$explicitMain = "function  main ( ) returns Int :`nreturn 1`nend function`n"
$accepted = Invoke-Sico 'check --json -' $explicitMain
if ($accepted.Exit -ne 0 -or ($accepted.Stdout | ConvertFrom-Json).summary.errors -ne 0) {
    throw 'explicit main was not accepted'
}
$formatted = Invoke-Sico 'format -' $explicitMain
$canonical = "function main() returns Int:`n  return 1`nend function`n"
if ($formatted.Exit -ne 0 -or $formatted.Stdout -cne $canonical) {
    throw 'explicit main did not format canonically'
}

$candidates = @(
    "stdout.write(stdin.read_all())`n",
    "script:`n  return input.stdin`nend script`n"
)
foreach ($candidate in $candidates) {
    $checked = Invoke-Sico 'check --json -' $candidate
    $diagnostics = $checked.Stdout | ConvertFrom-Json
    if ($checked.Exit -ne 1 -or $diagnostics.diagnostics.Count -ne 1 -or $diagnostics.diagnostics[0].code -ne 'E1013') {
        throw "top-level candidate did not fail with one E1013: $($checked.Stdout)"
    }
    if ($diagnostics.status.semantic_checks_performed -ne $false) {
        throw 'rejected top-level candidate reached semantic analysis'
    }
    $format = Invoke-Sico 'format -' $candidate
    if ($format.Exit -ne 1 -or $format.Stdout.Length -ne 0 -or $format.Stderr -notmatch 'E1013') {
        throw 'formatter did not refuse rejected top-level candidate'
    }
}

Write-Output 'STEP_0092_OK decision=explicit-main candidates=B,C-rejected diagnostic=E1013 parser=recover formatter=refuse hir=blocked lsp=typed ai=typed'
