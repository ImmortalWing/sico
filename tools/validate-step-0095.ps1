param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)
. (Join-Path $root 'tools\lib\native-command.ps1')

$maxSafeInteger = [int64]9007199254740991
$maxFrameBytes = 1048576
$maxStringBytes = 65536

function Read-Json([string]$RelativePath) {
    $path = Join-Path $root $RelativePath
    if (-not (Test-Path -LiteralPath $path)) { throw "missing-file|$RelativePath" }
    return Get-Content -LiteralPath $path -Raw -Encoding UTF8 | ConvertFrom-Json
}

function Fail([string]$Code, [string]$Message) {
    throw "$Code|$Message"
}

function Get-PropertyNames($Value) {
    return @($Value.PSObject.Properties | ForEach-Object { $_.Name })
}

function Assert-ExactProperties($Value, [string[]]$Required, [string]$Context) {
    $actual = @(Get-PropertyNames $Value)
    foreach ($name in $actual) {
        if ($name -notin $Required) { Fail 'unknown-field' "$Context contains $name" }
    }
    foreach ($name in $Required) {
        if ($name -notin $actual) { Fail 'missing-field' "$Context is missing $name" }
    }
}

function Assert-String([object]$Value, [string]$Context, [int]$MaxBytes = $maxStringBytes) {
    if ($Value -isnot [string] -or [string]::IsNullOrEmpty([string]$Value)) {
        Fail 'invalid-string' "$Context must be a non-empty string"
    }
    if ([Text.Encoding]::UTF8.GetByteCount([string]$Value) -gt $MaxBytes) {
        Fail 'over-limit' "$Context exceeds $MaxBytes bytes"
    }
}

function Assert-OptionalString([object]$Value, [string]$Context, [int]$MaxBytes = $maxStringBytes) {
    if ($null -eq $Value) { return }
    Assert-String $Value $Context $MaxBytes
}

function Assert-Id([object]$Value, [string]$Context) {
    Assert-String $Value $Context 256
    if ([string]$Value -notmatch '^[A-Za-z0-9][A-Za-z0-9._:/@-]*$') {
        Fail 'invalid-id' "$Context is not a stable bounded ID"
    }
}

function Assert-Sha256([object]$Value, [string]$Context) {
    if ($Value -isnot [string] -or [string]$Value -notmatch '^[0-9a-f]{64}$') {
        Fail 'invalid-digest' "$Context is not lowercase SHA-256"
    }
}

function Assert-SafeInteger([object]$Value, [string]$Context) {
    if ($Value -isnot [byte] -and $Value -isnot [int16] -and $Value -isnot [int32] -and
        $Value -isnot [int64] -and $Value -isnot [decimal]) {
        Fail 'unsafe-integer' "$Context is not an integer"
    }
    $number = [decimal]$Value
    if ($number -lt 0 -or $number -gt $maxSafeInteger -or [decimal]::Truncate($number) -ne $number) {
        Fail 'unsafe-integer' "$Context is outside interoperable integer range"
    }
}

function Assert-UniqueSortedStrings([object[]]$Values, [string]$Context, [int]$Maximum) {
    if ($Values.Count -gt $Maximum) { Fail 'over-limit' "$Context exceeds $Maximum entries" }
    $seen = @{}
    $previous = $null
    foreach ($value in $Values) {
        Assert-Id $value $Context
        $text = [string]$value
        if ($seen.ContainsKey($text)) { Fail 'duplicate-identity' "$Context duplicates $text" }
        if ($null -ne $previous -and [StringComparer]::Ordinal.Compare($previous, $text) -gt 0) {
            Fail 'non-canonical-order' "$Context is not ordinal sorted"
        }
        $seen[$text] = $true
        $previous = $text
    }
}

function Assert-Span($Span, [hashtable]$Documents, [string]$Context) {
    Assert-ExactProperties $Span @('document_id', 'start', 'end') $Context
    Assert-Id $Span.document_id "$Context.document_id"
    Assert-SafeInteger $Span.start "$Context.start"
    Assert-SafeInteger $Span.end "$Context.end"
    if ([int64]$Span.start -gt [int64]$Span.end) { Fail 'invalid-span' "$Context start is after end" }
    $documentId = [string]$Span.document_id
    if (-not $Documents.ContainsKey($documentId)) { Fail 'unknown-identity' "$Context references $documentId" }
    if ([int64]$Span.end -gt [int64]$Documents[$documentId]) { Fail 'invalid-span' "$Context exceeds document bytes" }
}

function Assert-DebugIdentity($Document, [string]$Context) {
    Assert-ExactProperties $Document @('schema', 'source', 'compiler', 'language_semantics', 'ir_schema', 'component_code_sha256', 'component_sha256', 'debug_map_sha256', 'adapter_identities', 'wit_identities') $Context
    if ($Document.schema -ne 'sico.debug.identity.v0') { Fail 'unknown-schema' $Context }
    Assert-ExactProperties $Document.source @('document_id', 'sha256', 'byte_length', 'display_uri') "$Context.source"
    Assert-Id $Document.source.document_id "$Context.source.document_id"
    Assert-Sha256 $Document.source.sha256 "$Context.source.sha256"
    Assert-SafeInteger $Document.source.byte_length "$Context.source.byte_length"
    Assert-OptionalString $Document.source.display_uri "$Context.source.display_uri"
    if ($null -ne $Document.source.display_uri -and
        [string]$Document.source.display_uri -notmatch '^(workspace|sico-source)://') {
        Fail 'authority-bearing-path' "$Context display URI must not be an authority-bearing path"
    }
    Assert-ExactProperties $Document.compiler @('package', 'version', 'executable_sha256') "$Context.compiler"
    Assert-Id $Document.compiler.package "$Context.compiler.package"
    Assert-Id $Document.compiler.version "$Context.compiler.version"
    Assert-Sha256 $Document.compiler.executable_sha256 "$Context.compiler.executable_sha256"
    Assert-Id $Document.language_semantics "$Context.language_semantics"
    Assert-Id $Document.ir_schema "$Context.ir_schema"
    Assert-Sha256 $Document.component_code_sha256 "$Context.component_code_sha256"
    Assert-Sha256 $Document.component_sha256 "$Context.component_sha256"
    Assert-Sha256 $Document.debug_map_sha256 "$Context.debug_map_sha256"
    Assert-UniqueSortedStrings @($Document.adapter_identities) "$Context.adapter_identities" 64
    Assert-UniqueSortedStrings @($Document.wit_identities) "$Context.wit_identities" 64
}

function Assert-DebugMap($Document, [string]$Context) {
    Assert-ExactProperties $Document @('schema', 'binding', 'coordinate_system', 'documents', 'functions', 'mappings') $Context
    if ($Document.schema -ne 'sico.debug-map.v0') { Fail 'unknown-schema' $Context }
    Assert-ExactProperties $Document.binding @('source_sha256', 'compiler_executable_sha256', 'component_code_sha256') "$Context.binding"
    Assert-Sha256 $Document.binding.source_sha256 "$Context.binding.source_sha256"
    Assert-Sha256 $Document.binding.compiler_executable_sha256 "$Context.binding.compiler_executable_sha256"
    Assert-Sha256 $Document.binding.component_code_sha256 "$Context.binding.component_code_sha256"
    if ($Document.coordinate_system -ne 'utf8-byte-half-open') { Fail 'invalid-coordinate-system' $Context }

    $documents = @($Document.documents)
    if ($documents.Count -lt 1 -or $documents.Count -gt 256) { Fail 'over-limit' "$Context documents" }
    $documentBytes = @{}
    $previousDocument = $null
    foreach ($entry in $documents) {
        Assert-ExactProperties $entry @('id', 'sha256', 'byte_length') "$Context.document"
        Assert-Id $entry.id "$Context.document.id"
        Assert-Sha256 $entry.sha256 "$Context.document.sha256"
        Assert-SafeInteger $entry.byte_length "$Context.document.byte_length"
        $id = [string]$entry.id
        if ($documentBytes.ContainsKey($id)) { Fail 'duplicate-identity' "$Context document $id" }
        if ($null -ne $previousDocument -and [StringComparer]::Ordinal.Compare($previousDocument, $id) -gt 0) {
            Fail 'non-canonical-order' "$Context documents"
        }
        $documentBytes[$id] = [int64]$entry.byte_length
        $previousDocument = $id
    }

    $functions = @($Document.functions)
    if ($functions.Count -gt 100000) { Fail 'over-limit' "$Context functions" }
    $functionIds = @{}
    $previousFunction = $null
    foreach ($entry in $functions) {
        Assert-ExactProperties $entry @('id', 'core_module', 'component_function') "$Context.function"
        Assert-Id $entry.id "$Context.function.id"
        Assert-Id $entry.core_module "$Context.function.core_module"
        Assert-SafeInteger $entry.component_function "$Context.function.component_function"
        $id = [string]$entry.id
        if ($functionIds.ContainsKey($id)) { Fail 'duplicate-identity' "$Context function $id" }
        if ($null -ne $previousFunction -and [StringComparer]::Ordinal.Compare($previousFunction, $id) -gt 0) {
            Fail 'non-canonical-order' "$Context functions"
        }
        $functionIds[$id] = "$($entry.core_module):$($entry.component_function)"
        $previousFunction = $id
    }

    $mappings = @($Document.mappings)
    if ($mappings.Count -gt 1000000) { Fail 'over-limit' "$Context mappings" }
    $mappingKeys = @{}
    $previousCoreModule = $null
    $previousComponentFunction = -1L
    $previousStart = -1L
    $previousEnd = -1L
    foreach ($entry in $mappings) {
        Assert-ExactProperties $entry @('core_module', 'component_function', 'instruction_start', 'instruction_end', 'function_id', 'source', 'call_site', 'inline_parent', 'generated') "$Context.mapping"
        Assert-Id $entry.core_module "$Context.mapping.core_module"
        Assert-SafeInteger $entry.component_function "$Context.mapping.component_function"
        Assert-SafeInteger $entry.instruction_start "$Context.mapping.instruction_start"
        Assert-SafeInteger $entry.instruction_end "$Context.mapping.instruction_end"
        if ([int64]$entry.instruction_start -ge [int64]$entry.instruction_end) { Fail 'invalid-span' "$Context instruction range" }
        Assert-Id $entry.function_id "$Context.mapping.function_id"
        if (-not $functionIds.ContainsKey([string]$entry.function_id)) { Fail 'unknown-identity' "$Context mapping function" }
        if ([string]$functionIds[[string]$entry.function_id] -ne "$($entry.core_module):$($entry.component_function)") { Fail 'identity-mismatch' "$Context mapping function index" }
        if ($null -ne $entry.source) { Assert-Span $entry.source $documentBytes "$Context.mapping.source" }
        if ($null -ne $entry.call_site) { Assert-Span $entry.call_site $documentBytes "$Context.mapping.call_site" }
        if ($null -ne $entry.inline_parent) {
            Assert-Id $entry.inline_parent "$Context.mapping.inline_parent"
            if (-not $functionIds.ContainsKey([string]$entry.inline_parent)) { Fail 'unknown-identity' "$Context inline parent" }
        }
        if ($entry.generated -isnot [bool]) { Fail 'invalid-boolean' "$Context.mapping.generated" }
        $key = "$($entry.core_module):$($entry.component_function):$($entry.instruction_start):$($entry.instruction_end):$($entry.function_id)"
        if ($mappingKeys.ContainsKey($key)) { Fail 'duplicate-mapping' "$Context mapping $key" }
        $mappingKeys[$key] = $true
        $coreModule = [string]$entry.core_module
        $componentFunction = [int64]$entry.component_function
        $start = [int64]$entry.instruction_start
        $end = [int64]$entry.instruction_end
        $moduleOrder = if ($null -eq $previousCoreModule) { 1 } else { [StringComparer]::Ordinal.Compare($coreModule, $previousCoreModule) }
        if ($moduleOrder -lt 0 -or
            ($moduleOrder -eq 0 -and $componentFunction -lt $previousComponentFunction) -or
            ($moduleOrder -eq 0 -and $componentFunction -eq $previousComponentFunction -and $start -lt $previousStart)) {
            Fail 'non-canonical-order' "$Context mappings"
        }
        if ($moduleOrder -eq 0 -and $componentFunction -eq $previousComponentFunction -and $start -lt $previousEnd) {
            Fail 'overlapping-mapping' "$Context mappings"
        }
        $previousCoreModule = $coreModule
        $previousComponentFunction = $componentFunction
        $previousStart = $start
        $previousEnd = $end
    }

    $canonicalBytes = [Text.Encoding]::UTF8.GetByteCount(($Document | ConvertTo-Json -Depth 20 -Compress))
    if ($canonicalBytes -gt 16777216) { Fail 'over-limit' "$Context canonical bytes" }
}

function Assert-RuntimeFault($Document, [string]$Context) {
    Assert-ExactProperties $Document @('schema', 'run_id', 'generation_id', 'class', 'code', 'key', 'message', 'provider_id', 'frames') $Context
    if ($Document.schema -ne 'sico.runtime-fault.v0') { Fail 'unknown-schema' $Context }
    Assert-Id $Document.run_id "$Context.run_id"
    Assert-SafeInteger $Document.generation_id "$Context.generation_id"
    if ($Document.class -notin @('domain-error', 'cancelled', 'timeout', 'resource-limit.fuel', 'resource-limit.memory', 'resource-limit.other', 'trap', 'host-provider-failure', 'incompatible-artifact', 'launch-failure', 'external-termination', 'internal-invariant')) {
        Fail 'invalid-fault-class' $Context
    }
    Assert-Id $Document.code "$Context.code"
    Assert-Id $Document.key "$Context.key"
    Assert-OptionalString $Document.message "$Context.message"
    if ($null -ne $Document.provider_id) { Assert-Id $Document.provider_id "$Context.provider_id" }
    $frames = @($Document.frames)
    if ($frames.Count -gt 256) { Fail 'over-limit' "$Context frames" }
    $emptyDocuments = @{}
    foreach ($frame in $frames) {
        Assert-ExactProperties $frame @('function_id', 'source', 'generated', 'unavailable_reason') "$Context.frame"
        Assert-Id $frame.function_id "$Context.frame.function_id"
        if ($frame.generated -isnot [bool]) { Fail 'invalid-boolean' "$Context.frame.generated" }
        Assert-OptionalString $frame.unavailable_reason "$Context.frame.unavailable_reason"
        if ($null -ne $frame.source) {
            Assert-ExactProperties $frame.source @('document_id', 'start', 'end') "$Context.frame.source"
            Assert-Id $frame.source.document_id "$Context.frame.source.document_id"
            Assert-SafeInteger $frame.source.start "$Context.frame.source.start"
            Assert-SafeInteger $frame.source.end "$Context.frame.source.end"
            if ([int64]$frame.source.start -gt [int64]$frame.source.end) { Fail 'invalid-span' "$Context.frame.source" }
        }
    }
}

function Assert-ExecutionEvent($Document, [string]$Context) {
    Assert-ExactProperties $Document @('schema', 'run_id', 'generation_id', 'sequence', 'kind', 'task_id', 'parent_task_id', 'scope_id', 'cause', 'payload') $Context
    if ($Document.schema -ne 'sico.execution-event.v0') { Fail 'unknown-schema' $Context }
    Assert-Id $Document.run_id "$Context.run_id"
    Assert-SafeInteger $Document.generation_id "$Context.generation_id"
    Assert-SafeInteger $Document.sequence "$Context.sequence"
    if ($Document.kind -notin @('accepted', 'started', 'generation-published', 'stdout', 'stderr', 'breakpoint', 'stopped', 'continued', 'cancellation-requested', 'terminal', 'fault', 'truncated', 'dropped')) { Fail 'invalid-event-kind' $Context }
    foreach ($name in @('task_id', 'parent_task_id', 'scope_id')) {
        if ($null -ne $Document.$name) { Assert-Id $Document.$name "$Context.$name" }
    }
    if ($Document.cause -notin @('launch', 'guest', 'host', 'timeout', 'signal', 'client', 'debugger', 'overflow', 'internal')) { Fail 'invalid-event-cause' $Context }
    $allowedPayload = @('bytes_base64', 'message', 'breakpoint_id', 'terminal_class', 'dropped_events')
    foreach ($name in @(Get-PropertyNames $Document.payload)) {
        if ($name -notin $allowedPayload) { Fail 'unknown-field' "$Context.payload contains $name" }
    }
    if ($null -ne $Document.payload.PSObject.Properties['bytes_base64']) {
        try { $bytes = [Convert]::FromBase64String([string]$Document.payload.bytes_base64) } catch { Fail 'invalid-base64' $Context }
        if ($bytes.Length -gt 65536) { Fail 'over-limit' "$Context output chunk" }
    }
    if ($null -ne $Document.payload.PSObject.Properties['message']) { Assert-OptionalString $Document.payload.message "$Context.payload.message" }
    if ($null -ne $Document.payload.PSObject.Properties['breakpoint_id'] -and $null -ne $Document.payload.breakpoint_id) { Assert-Id $Document.payload.breakpoint_id "$Context.payload.breakpoint_id" }
    if ($null -ne $Document.payload.PSObject.Properties['terminal_class']) { Assert-OptionalString $Document.payload.terminal_class "$Context.payload.terminal_class" 256 }
    if ($null -ne $Document.payload.PSObject.Properties['dropped_events']) { Assert-SafeInteger $Document.payload.dropped_events "$Context.payload.dropped_events" }
    $serializedBytes = [Text.Encoding]::UTF8.GetByteCount(($Document | ConvertTo-Json -Depth 20 -Compress))
    if ($serializedBytes -gt $maxFrameBytes) { Fail 'over-limit' "$Context serialized event" }
}

function Assert-DapDocument($Document, [string]$Context, [bool]$RequireExactClaims) {
    Assert-ExactProperties $Document @('schema', 'dap_protocol', 'unknown_request_policy', 'unclaimed_event_policy', 'claims') $Context
    if ($Document.schema -ne 'sico.dap-claimed-subset.v0' -or $Document.dap_protocol -ne 'base-protocol-v0' -or
        $Document.unknown_request_policy -ne 'typed-unsupported-request' -or $Document.unclaimed_event_policy -ne 'must-not-emit') {
        Fail 'unknown-schema' $Context
    }
    $claims = @($Document.claims)
    if ($claims.Count -lt 1 -or $claims.Count -gt 128) { Fail 'over-limit' "$Context claims" }
    $seenClaims = @{}
    $seenEvidence = @{}
    foreach ($claim in $claims) {
        Assert-ExactProperties $claim @('kind', 'name', 'direction', 'state', 'preconditions', 'max_payload_bytes', 'evidence_id', 'behavior') "$Context.claim"
        if ($claim.kind -notin @('request', 'event')) { Fail 'invalid-dap-kind' $Context }
        Assert-String $claim.name "$Context.claim.name" 128
        if ($claim.name -notmatch '^[A-Za-z][A-Za-z0-9]*$') { Fail 'invalid-dap-name' $Context }
        if (($claim.kind -eq 'request' -and $claim.direction -ne 'client-to-adapter') -or
            ($claim.kind -eq 'event' -and $claim.direction -ne 'adapter-to-client')) { Fail 'invalid-dap-direction' $Context }
        if ($claim.state -notin @('supported', 'refused') -or ($claim.kind -eq 'event' -and $claim.state -ne 'supported')) { Fail 'invalid-dap-state' $Context }
        $preconditions = @($claim.preconditions)
        if ($preconditions.Count -gt 16) { Fail 'over-limit' "$Context preconditions" }
        foreach ($precondition in $preconditions) { Assert-String $precondition "$Context precondition" 256 }
        Assert-SafeInteger $claim.max_payload_bytes "$Context.claim.max_payload_bytes"
        if ([int64]$claim.max_payload_bytes -lt 1 -or [int64]$claim.max_payload_bytes -gt $maxFrameBytes) { Fail 'over-limit' "$Context DAP payload" }
        Assert-String $claim.evidence_id "$Context.claim.evidence_id" 128
        if ($claim.evidence_id -notmatch '^dap-v0-[a-z0-9-]+$') { Fail 'invalid-evidence-id' $Context }
        Assert-String $claim.behavior "$Context.claim.behavior" 1024
        $key = "$($claim.kind):$($claim.name)"
        if ($seenClaims.ContainsKey($key)) { Fail 'duplicate-identity' "$Context claim $key" }
        if ($seenEvidence.ContainsKey([string]$claim.evidence_id)) { Fail 'duplicate-identity' "$Context evidence $($claim.evidence_id)" }
        $seenClaims[$key] = $claim
        $seenEvidence[[string]$claim.evidence_id] = $true
    }
    if (-not $RequireExactClaims) { return }

    $supportedRequests = @('initialize', 'launch', 'setBreakpoints', 'configurationDone', 'threads', 'stackTrace', 'scopes', 'variables', 'continue', 'pause', 'disconnect', 'terminate')
    $refusedRequests = @('attach', 'next', 'stepIn', 'stepOut', 'evaluate', 'setExpression', 'setVariable', 'readMemory', 'writeMemory', 'disassemble', 'setFunctionBreakpoints', 'setDataBreakpoints', 'setInstructionBreakpoints', 'setExceptionBreakpoints', 'restartFrame', 'goto', 'stepBack', 'reverseContinue', 'restart', 'terminateThreads')
    $supportedEvents = @('initialized', 'stopped', 'continued', 'output', 'terminated', 'exited')
    $actualSupportedRequests = @($claims | Where-Object { $_.kind -eq 'request' -and $_.state -eq 'supported' } | ForEach-Object { $_.name } | Sort-Object)
    $actualRefusedRequests = @($claims | Where-Object { $_.kind -eq 'request' -and $_.state -eq 'refused' } | ForEach-Object { $_.name } | Sort-Object)
    $actualSupportedEvents = @($claims | Where-Object { $_.kind -eq 'event' -and $_.state -eq 'supported' } | ForEach-Object { $_.name } | Sort-Object)
    if (($actualSupportedRequests -join ',') -ne (@($supportedRequests | Sort-Object) -join ',')) { Fail 'dap-claim-drift' 'supported requests differ' }
    if (($actualRefusedRequests -join ',') -ne (@($refusedRequests | Sort-Object) -join ',')) { Fail 'dap-claim-drift' 'refused requests differ' }
    if (($actualSupportedEvents -join ',') -ne (@($supportedEvents | Sort-Object) -join ',')) { Fail 'dap-claim-drift' 'supported events differ' }
}

function Assert-CancellationDocument($Document, [string]$Context) {
    Assert-ExactProperties $Document @('schema', 'states', 'winner_rule', 'typed_cancellation_exit', 'cases', 'races') $Context
    if ($Document.schema -ne 'sico.cancellation-race.v0' -or (@($Document.states) -join ',') -ne 'running,cancellation-requested,terminal' -or
        $Document.winner_rule -ne 'first-runtime-observed-committed-terminal-wins' -or $Document.typed_cancellation_exit -ne 123) {
        Fail 'invalid-cancellation-contract' $Context
    }
    $expectedInputs = @('guest-completion', 'guest-failure', 'timeout', 'signal', 'client-request', 'host-failure', 'external-kill-fallback')
    $seenInputs = @{}
    $caseByInput = @{}
    foreach ($case in @($Document.cases)) {
        Assert-ExactProperties $case @('id', 'input', 'runner_observed', 'terminal_class', 'exit_code', 'typed_cancel') "$Context.case"
        Assert-String $case.id "$Context.case.id" 128
        if ($case.id -notmatch '^cancel-v0-[a-z0-9-]+$' -or $case.input -notin $expectedInputs) { Fail 'invalid-cancellation-case' $Context }
        if ($seenInputs.ContainsKey([string]$case.input)) { Fail 'duplicate-identity' "$Context input $($case.input)" }
        if ($case.runner_observed -isnot [bool] -or $case.typed_cancel -isnot [bool]) { Fail 'invalid-boolean' $Context }
        if ($null -ne $case.exit_code) { Assert-SafeInteger $case.exit_code "$Context.case.exit_code" }
        $seenInputs[[string]$case.input] = $true
        $caseByInput[[string]$case.input] = $case
    }
    if ((@($seenInputs.Keys | Sort-Object) -join ',') -ne (@($expectedInputs | Sort-Object) -join ',')) { Fail 'cancellation-case-drift' $Context }
    foreach ($input in @('signal', 'client-request')) {
        $case = $caseByInput[$input]
        if (-not $case.runner_observed -or -not $case.typed_cancel -or $case.terminal_class -ne 'cancelled' -or $case.exit_code -ne 123) { Fail 'typed-cancellation-drift' $input }
    }
    $external = $caseByInput['external-kill-fallback']
    if ($external.runner_observed -or $external.typed_cancel -or $external.terminal_class -ne 'external-termination' -or $null -ne $external.exit_code) { Fail 'external-termination-drift' $Context }

    $seenRaces = @{}
    $racePairs = @{}
    foreach ($race in @($Document.races)) {
        Assert-ExactProperties $race @('id', 'first_observed', 'second_observed', 'expected_winner', 'terminal_count') "$Context.race"
        Assert-String $race.id "$Context.race.id" 128
        if ($race.id -notmatch '^race-v0-[a-z0-9-]+$' -or $seenRaces.ContainsKey([string]$race.id)) { Fail 'duplicate-identity' "$Context race" }
        if ($race.first_observed -notin $expectedInputs -or $race.second_observed -notin $expectedInputs -or $race.expected_winner -ne $race.first_observed -or $race.terminal_count -ne 1) { Fail 'terminal-winner-drift' "$Context race $($race.id)" }
        $seenRaces[[string]$race.id] = $true
        $racePairs["$($race.first_observed)>$($race.second_observed)"] = $true
    }
    foreach ($pair in @('guest-completion>client-request', 'client-request>guest-completion', 'timeout>signal', 'signal>timeout', 'host-failure>client-request', 'client-request>host-failure', 'guest-failure>timeout', 'timeout>guest-failure')) {
        if (-not $racePairs.ContainsKey($pair)) { Fail 'missing-race-order' $pair }
    }
}

function Assert-Document($Document, [string]$Context, [bool]$RequireExactDap) {
    if ($null -eq $Document.PSObject.Properties['schema']) { Fail 'missing-field' "$Context.schema" }
    switch ([string]$Document.schema) {
        'sico.debug.identity.v0' { Assert-DebugIdentity $Document $Context }
        'sico.debug-map.v0' { Assert-DebugMap $Document $Context }
        'sico.runtime-fault.v0' { Assert-RuntimeFault $Document $Context }
        'sico.execution-event.v0' { Assert-ExecutionEvent $Document $Context }
        'sico.dap-claimed-subset.v0' { Assert-DapDocument $Document $Context $RequireExactDap }
        default { Fail 'unknown-schema' "$Context $($Document.schema)" }
    }
}

function Assert-CrossBindings([object[]]$Documents, [string]$Context) {
    $identity = @($Documents | Where-Object { $_.schema -eq 'sico.debug.identity.v0' }) | Select-Object -First 1
    $map = @($Documents | Where-Object { $_.schema -eq 'sico.debug-map.v0' }) | Select-Object -First 1
    if ($null -eq $identity -or $null -eq $map) { return }
    if ($identity.source.sha256 -ne $map.binding.source_sha256 -or
        $identity.compiler.executable_sha256 -ne $map.binding.compiler_executable_sha256 -or
        $identity.component_code_sha256 -ne $map.binding.component_code_sha256) {
        Fail 'stale-digest' "$Context identity/debug-map binding differs"
    }
    $primaryDocument = @($map.documents | Where-Object { $_.id -eq $identity.source.document_id })
    if ($primaryDocument.Count -ne 1 -or $primaryDocument[0].sha256 -ne $identity.source.sha256 -or $primaryDocument[0].byte_length -ne $identity.source.byte_length) {
        Fail 'stale-digest' "$Context source binding differs"
    }
}

# Schema files are themselves strict machine contracts; pin their identities and hard bounds.
$observabilitySchema = Read-Json 'observability\schema\observability-contract-v0.schema.json'
$dapSchema = Read-Json 'observability\schema\dap-claimed-subset-v0.schema.json'
$cancellationSchema = Read-Json 'observability\schema\cancellation-race-v0.schema.json'
if ($observabilitySchema.'$id' -ne 'https://sico.dev/schema/observability-contract-v0.schema.json' -or
    $observabilitySchema.'$defs'.debugMap.properties.documents.maxItems -ne 256 -or
    $observabilitySchema.'$defs'.debugMap.properties.functions.maxItems -ne 100000 -or
    $observabilitySchema.'$defs'.debugMap.properties.mappings.maxItems -ne 1000000 -or
    $observabilitySchema.'$defs'.runtimeFault.properties.frames.maxItems -ne 256) {
    Fail 'schema-drift' 'observability hard limits differ'
}
foreach ($definitionName in @('span', 'debugIdentity', 'debugMap', 'frame', 'runtimeFault', 'executionEvent')) {
    if ($observabilitySchema.'$defs'.$definitionName.additionalProperties -ne $false) { Fail 'schema-drift' "$definitionName is not strict" }
}
if ($dapSchema.additionalProperties -ne $false -or $dapSchema.properties.claims.maxItems -ne 128 -or
    $dapSchema.properties.claims.items.properties.max_payload_bytes.maximum -ne $maxFrameBytes -or
    $cancellationSchema.additionalProperties -ne $false -or $cancellationSchema.properties.typed_cancellation_exit.const -ne 123) {
    Fail 'schema-drift' 'DAP/cancellation schema differs'
}

$dap = Read-Json 'observability\contracts\dap-claimed-subset-v0.json'
Assert-DapDocument $dap 'dap-claimed-subset-v0' $true
$cancellation = Read-Json 'observability\contracts\cancellation-race-v0.json'
Assert-CancellationDocument $cancellation 'cancellation-race-v0'

$fixtures = Read-Json 'observability\fixtures\contract-cases-v0.json'
Assert-ExactProperties $fixtures @('schema', 'cases') 'fixtures'
if ($fixtures.schema -ne 'sico.observability-fixtures.v0') { Fail 'unknown-schema' 'fixtures' }
$fixtureIds = @{}
$accepted = 0
$rejected = 0
foreach ($case in @($fixtures.cases)) {
    Assert-ExactProperties $case @('id', 'expected', 'error', 'documents') 'fixture-case'
    Assert-Id $case.id 'fixture-case.id'
    if ($fixtureIds.ContainsKey([string]$case.id)) { Fail 'duplicate-identity' "fixture $($case.id)" }
    $fixtureIds[[string]$case.id] = $true
    $actual = 'accept'
    $actualError = $null
    try {
        $documents = @($case.documents)
        foreach ($document in $documents) { Assert-Document $document "fixture $($case.id)" $false }
        Assert-CrossBindings $documents "fixture $($case.id)"
    } catch {
        $actual = 'reject'
        $actualError = ($_.Exception.Message -split '\|', 2)[0]
    }
    if ($actual -ne $case.expected) { Fail 'fixture-result-mismatch' "$($case.id) expected $($case.expected), got $actual/$actualError" }
    if ($actual -eq 'reject' -and $actualError -ne $case.error) { Fail 'fixture-error-mismatch' "$($case.id) expected $($case.error), got $actualError" }
    if ($actual -eq 'accept') { $accepted++ } else { $rejected++ }
}

# Real compiler identity evidence: two different sources must create distinct
# Components, while rebuilding the same source stays byte-identical. This does
# not claim that STEP-0096 debug-map emission exists.
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) { $cargo = (Get-Command cargo -ErrorAction Stop).Source }
Invoke-NativeChecked $cargo @('build', '-q', '--offline', '--locked', '-p', 'sico-cli') 'component-build-failed|sico-cli build failed'
$sico = Join-Path $root 'target\debug\sico.exe'
$work = Join-Path $root 'target\evidence\step-0095'
New-Item -ItemType Directory -Force -Path $work | Out-Null
$sourceA = Join-Path $root 'tests\end-to-end\script-echo.sico'
$sourceB = Join-Path $root 'tests\end-to-end\script-reject.sico'
$componentA = Join-Path $work 'echo.component.wasm'
$componentARepeat = Join-Path $work 'echo-repeat.component.wasm'
$componentB = Join-Path $work 'reject.component.wasm'
foreach ($artifact in @($componentA, $componentARepeat, $componentB)) {
    if (Test-Path -LiteralPath $artifact) { Remove-Item -LiteralPath $artifact -Force }
}
Invoke-NativeChecked $sico @('build', '--profile', 'script-v0', '--output', $componentA, $sourceA) 'component-build-failed|echo Component build failed' | Out-Null
Invoke-NativeChecked $sico @('build', '--profile', 'script-v0', '--output', $componentARepeat, $sourceA) 'component-build-failed|echo repeat Component build failed' | Out-Null
Invoke-NativeChecked $sico @('build', '--profile', 'script-v0', '--output', $componentB, $sourceB) 'component-build-failed|reject Component build failed' | Out-Null
$sourceAHash = (Get-FileHash -LiteralPath $sourceA -Algorithm SHA256).Hash.ToLowerInvariant()
$sourceBHash = (Get-FileHash -LiteralPath $sourceB -Algorithm SHA256).Hash.ToLowerInvariant()
$componentAHash = (Get-FileHash -LiteralPath $componentA -Algorithm SHA256).Hash.ToLowerInvariant()
$componentARepeatHash = (Get-FileHash -LiteralPath $componentARepeat -Algorithm SHA256).Hash.ToLowerInvariant()
$componentBHash = (Get-FileHash -LiteralPath $componentB -Algorithm SHA256).Hash.ToLowerInvariant()
$compilerHash = (Get-FileHash -LiteralPath $sico -Algorithm SHA256).Hash.ToLowerInvariant()
if ($sourceAHash -eq $sourceBHash -or $componentAHash -eq $componentBHash) { throw 'identity-collision|different sources/components share an identity' }
if ($componentAHash -ne $componentARepeatHash) { throw 'identity-nondeterministic|same source rebuilt to different Component bytes' }

Write-Output "STEP_0095_OK schemas=3 identities=4 fixtures=$($fixtureIds.Count) accepted=$accepted rejected=$rejected dap_supported_requests=12 dap_refused_requests=20 dap_events=6 cancellation_cases=$(@($cancellation.cases).Count) races=$(@($cancellation.races).Count) components=2 deterministic=true source_a=$($sourceAHash.Substring(0,16)) source_b=$($sourceBHash.Substring(0,16)) component_a=$($componentAHash.Substring(0,16)) component_b=$($componentBHash.Substring(0,16)) compiler=$($compilerHash.Substring(0,16)) authority=unchanged implementation=not-claimed next=STEP-0096"
