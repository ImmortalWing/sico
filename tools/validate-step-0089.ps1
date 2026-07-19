param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)

cargo test --offline --locked -p sico-codegen-wasm -p sico-package -p sico-cli
if ($LASTEXITCODE -ne 0) { throw 'STEP-0089 workspace tests failed' }
cargo build --offline --locked -p sico-cli
if ($LASTEXITCODE -ne 0) { throw 'sico build failed' }

$vcvars = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
$runnerDir = Join-Path $root 'runner\sico-runner'
$build = "call `"$vcvars`" && set RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc&& cd /d `"$runnerDir`" && cargo test --release --offline && cargo build --release --offline"
$null = & cmd.exe /c $build
if ($LASTEXITCODE -ne 0) { throw 'sico-runner build failed' }

$sico = Join-Path $root 'target\debug\sico.exe'
$runner = Join-Path $runnerDir 'target\release\sico-runner.exe'
$source = Join-Path $root 'tests\end-to-end\script-http-request.sico'
$runId = [Guid]::NewGuid().ToString('N')
$work = Join-Path $root "target\evidence\step-0089\$runId"
New-Item -ItemType Directory -Force -Path $work | Out-Null
$component = Join-Path $work 'http-request.wasm'
& $sico build --profile script-v0 -o $component $source
if ($LASTEXITCODE -ne 0) { throw 'HTTP Component build failed' }

$sourceText = Get-Content $source -Raw -Encoding UTF8
$getSource = $sourceText.Replace('sico.http.request("POST"', 'sico.http.request("GET"')
$getPath = Join-Path $work 'http-get.sico'
[IO.File]::WriteAllText($getPath, $getSource, [Text.UTF8Encoding]::new($false))
$getComponent = Join-Path $work 'http-get.wasm'
& $sico build --profile script-v0 -o $getComponent $getPath
if ($LASTEXITCODE -ne 0) { throw 'HTTP GET Component build failed' }

$putSource = $sourceText.Replace('sico.http.request("POST"', 'sico.http.request("PUT"')
$putPath = Join-Path $work 'http-put.sico'
[IO.File]::WriteAllText($putPath, $putSource, [Text.UTF8Encoding]::new($false))
$putComponent = Join-Path $work 'http-put.wasm'
& $sico build --profile script-v0 -o $putComponent $putPath
if ($LASTEXITCODE -ne 0) { throw 'HTTP unsupported-method Component build failed' }

function Start-CapturedProcess([string]$file, [string]$arguments, [string]$stdin, [hashtable]$environment = @{}) {
    $psi = [Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $file
    $psi.Arguments = $arguments
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    foreach ($entry in $environment.GetEnumerator()) {
        $psi.EnvironmentVariables[$entry.Key] = $entry.Value
    }
    $process = [Diagnostics.Process]::Start($psi)
    $process.StandardInput.Write($stdin)
    $process.StandardInput.Close()
    return $process
}

function Finish-Process([Diagnostics.Process]$process, [int]$timeoutMs) {
    if (-not $process.WaitForExit($timeoutMs)) {
        $process.Kill()
        $process.WaitForExit()
        throw "process did not exit within ${timeoutMs}ms"
    }
    return [pscustomobject]@{
        Exit = $process.ExitCode
        Stdout = $process.StandardOutput.ReadToEnd()
        Stderr = $process.StandardError.ReadToEnd()
    }
}

function Invoke-NoServer([string]$program, [string]$arguments, [string]$stdin = '') {
    return Finish-Process (Start-CapturedProcess $program $arguments $stdin) 3000
}

function Read-Request([Net.Sockets.NetworkStream]$stream) {
    $stream.ReadTimeout = 2000
    $memory = [IO.MemoryStream]::new()
    $expected = $null
    while ($memory.Length -lt 131072) {
        $buffer = [byte[]]::new(4096)
        $count = $stream.Read($buffer, 0, $buffer.Length)
        if ($count -eq 0) { break }
        $memory.Write($buffer, 0, $count)
        $text = [Text.Encoding]::ASCII.GetString($memory.ToArray())
        $headerEnd = $text.IndexOf("`r`n`r`n", [StringComparison]::Ordinal)
        if ($headerEnd -ge 0 -and $null -eq $expected) {
            $match = [regex]::Match($text.Substring(0, $headerEnd), '(?im)^Content-Length:\s*(\d+)\s*$')
            $expected = if ($match.Success) { $headerEnd + 4 + [int]$match.Groups[1].Value } else { $headerEnd + 4 }
        }
        if ($null -ne $expected -and $memory.Length -ge $expected) { break }
    }
    return [Text.Encoding]::ASCII.GetString($memory.ToArray())
}

function Invoke-ServerCase(
    [string]$program,
    [string]$argumentTemplate,
    [string]$stdin,
    [byte[]]$response,
    [int]$timeoutMs = 10000,
    [hashtable]$environment = @{}
) {
    $listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $port = ([Net.IPEndPoint]$listener.LocalEndpoint).Port
    $accept = $listener.AcceptTcpClientAsync()
    $arguments = $argumentTemplate.Replace('{PORT}', [string]$port)
    $process = Start-CapturedProcess $program $arguments $stdin $environment
    if (-not $accept.Wait(5000)) {
        $result = Finish-Process $process 1000
        $listener.Stop()
        throw "expected HTTP connection, got exit=$($result.Exit) $($result.Stderr)"
    }
    $client = $accept.Result
    $stream = $client.GetStream()
    $request = Read-Request $stream
    if ($response.Length -gt 0) {
        $stream.Write($response, 0, $response.Length)
        $stream.Flush()
    }
    $result = Finish-Process $process $timeoutMs
    $client.Close()
    $listener.Stop()
    return [pscustomobject]@{
        Port = $port
        Request = $request
        Exit = $result.Exit
        Stdout = $result.Stdout
        Stderr = $result.Stderr
    }
}

# Exact CLI grant parsing and pre-socket default denial.
$invalid = Invoke-NoServer $runner "--allow-net *:80 `"$component`" -- http://127.0.0.1:80/"
if ($invalid.Exit -ne 121 -or $invalid.Stderr -notmatch 'invalid --allow-net') { throw 'invalid wildcard grant was not rejected by the CLI' }
$denied = Invoke-NoServer $runner "`"$component`" -- http://127.0.0.1:9/"
if ($denied.Exit -ne 122 -or $denied.Stderr -notmatch 'denied') { throw 'default network denial failed' }
$wrongPort = Invoke-NoServer $runner "--allow-net 127.0.0.1:10 `"$component`" -- http://127.0.0.1:9/"
if ($wrongPort.Exit -ne 122 -or $wrongPort.Stderr -notmatch 'denied') { throw 'wrong-port denial failed' }
$https = Invoke-NoServer $runner "--allow-net 127.0.0.1:443 `"$component`" -- https://127.0.0.1/"
if ($https.Exit -ne 122 -or $https.Stderr -notmatch 'denied') { throw 'HTTPS must fail closed in v0' }
$put = Invoke-NoServer $runner "--allow-net 127.0.0.1:9 `"$putComponent`" -- http://127.0.0.1:9/"
if ($put.Exit -ne 122 -or $put.Stderr -notmatch 'protocol') { throw 'unsupported method was not refused before connect' }

# Real `sico run` -> compiler/cache -> runner -> Component -> loopback server.
$okResponse = [Text.Encoding]::ASCII.GetBytes("HTTP/1.1 202 Accepted`r`nContent-Length: 4`r`nConnection: close`r`n`r`npong")
$cliArgs = "run --allow-net 127.0.0.1:{PORT} `"$source`" -- http://127.0.0.1:{PORT}/echo?q=1"
$cli = Invoke-ServerCase $sico $cliArgs 'cli-body' $okResponse 10000 @{
    SICO_RUNNER = $runner
    SICO_CACHE_DIR = (Join-Path $work 'cli-cache')
}
if ($cli.Exit -ne 0 -or $cli.Stdout -cne '202:pong' -or $cli.Stderr) { throw "CLI HTTP roundtrip failed: $($cli.Exit) $($cli.Stdout) $($cli.Stderr)" }
if ($cli.Request -notmatch '^POST /echo\?q=1 HTTP/1\.1' -or $cli.Request -notmatch "Content-Length: 8`r`n" -or -not $cli.Request.EndsWith('cli-body')) {
    throw "request framing mismatch: $($cli.Request)"
}

# GET uses the same exact endpoint grant and preserves its query target.
$getResponse = [Text.Encoding]::ASCII.GetBytes("HTTP/1.1 200 OK`r`nContent-Length: 3`r`nConnection: close`r`n`r`nget")
$getArgs = "--allow-net 127.0.0.1:{PORT} `"$getComponent`" -- http://127.0.0.1:{PORT}/read?q=2"
$get = Invoke-ServerCase $runner $getArgs '' $getResponse
if ($get.Exit -ne 0 -or $get.Stdout -cne '200:get' -or $get.Request -notmatch '^GET /read\?q=2 HTTP/1\.1') {
    throw 'GET roundtrip failed'
}

# Redirect is surfaced and never followed.
$redirectResponse = [Text.Encoding]::ASCII.GetBytes("HTTP/1.1 302 Found`r`nLocation: http://127.0.0.1:1/outside`r`nContent-Length: 8`r`nConnection: close`r`n`r`nredirect")
$redirectArgs = "--allow-net 127.0.0.1:{PORT} `"$component`" -- http://127.0.0.1:{PORT}/redirect"
$redirect = Invoke-ServerCase $runner $redirectArgs '' $redirectResponse
if ($redirect.Exit -ne 0 -or $redirect.Stdout -cne '302:redirect') { throw 'redirect was not surfaced as a normal 3xx response' }

# Response header/body bounds are typed and never silently truncated.
$largeHeader = "HTTP/1.1 200 OK`r`nX-Large: " + ('a' * 65536) + "`r`nContent-Length: 0`r`n`r`n"
$headerLimit = Invoke-ServerCase $runner $redirectArgs '' ([Text.Encoding]::ASCII.GetBytes($largeHeader))
if ($headerLimit.Exit -ne 122 -or $headerLimit.Stderr -notmatch 'resource-limit') { throw 'response header bound failed' }
$bodyLimitResponse = [Text.Encoding]::ASCII.GetBytes("HTTP/1.1 200 OK`r`nContent-Length: 8388609`r`nConnection: close`r`n`r`n")
$bodyLimit = Invoke-ServerCase $runner $redirectArgs '' $bodyLimitResponse
if ($bodyLimit.Exit -ne 122 -or $bodyLimit.Stderr -notmatch 'resource-limit') { throw 'response body bound failed' }

# Total timeout and explicit cancellation abandon a blocked read promptly.
$empty = [byte[]]::new(0)
$timeoutWatch = [Diagnostics.Stopwatch]::StartNew()
$timeout = Invoke-ServerCase $runner $redirectArgs '' $empty 7000
if ($timeout.Exit -ne 122 -or $timeout.Stderr -notmatch 'timeout' -or $timeoutWatch.Elapsed.TotalSeconds -gt 6.5) { throw 'HTTP total timeout failed' }
$cancelArgs = "--cancel-after-ms 100 --allow-net 127.0.0.1:{PORT} `"$component`" -- http://127.0.0.1:{PORT}/slow"
$cancelWatch = [Diagnostics.Stopwatch]::StartNew()
$cancel = Invoke-ServerCase $runner $cancelArgs '' $empty 1000
if ($cancel.Exit -ne 123 -or $cancel.Stderr -notmatch 'cancelled' -or $cancelWatch.Elapsed.TotalMilliseconds -gt 500) { throw 'HTTP cancellation failed' }

Write-Output ("STEP_0089_OK cli=POST status=202 body=exact GET=200 default-deny/wrong-port/https/method=typed redirect=not-followed header/body=bounded timeout={0:N0}ms cancel={1:N0}ms capability=network.connect work={2}" -f $timeoutWatch.Elapsed.TotalMilliseconds, $cancelWatch.Elapsed.TotalMilliseconds, $work)
