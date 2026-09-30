$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

$fixtures = @(
  @{name='const-i64'; source="function probe() returns I64:`n  return I64.literal(624485)`nend function`n"},
  @{name='const-u64'; source="function probe() returns U64:`n  return U64.literal(624485)`nend function`n"},
  @{name='const-bool-true'; source="function probe() returns Bool:`n  return true`nend function`n"},
  @{name='const-bool-false'; source="function probe() returns Bool:`n  return false`nend function`n"},
  @{name='identity-i64'; source="function probe(value: I64) returns I64:`n  return value`nend function`n"},
  @{name='identity-u64'; source="function probe(value: U64) returns U64:`n  return value`nend function`n"},
  @{name='identity-bool'; source="function probe(value: Bool) returns Bool:`n  return value`nend function`n"},
  @{name='identity-pair-first'; source="function probe(a: I64, b: I64) returns I64:`n  return a`nend function`n"},
  @{name='identity-pair-second'; source="function probe(a: I64, b: I64) returns I64:`n  return b`nend function`n"},
  @{name='identity-pair-bool'; source="function probe(a: Bool, b: Bool) returns Bool:`n  return b`nend function`n"},
  @{name='less-than-i64'; source="function probe(a: I64, b: I64) returns Bool:`n  return I64.less_than(a, b)`nend function`n"},
  @{name='less-than-u64'; source="function probe(a: U64, b: U64) returns Bool:`n  return U64.less_than(a, b)`nend function`n"},
  @{name='checked-add-i64'; source="function probe(a: I64, b: I64) returns Result[I64, NumericError]:`n  return I64.checked_add(a, b)`nend function`n"},
  @{name='checked-add-u64'; source="function probe(a: U64, b: U64) returns Result[U64, NumericError]:`n  return U64.checked_add(a, b)`nend function`n"},
  @{name='checked-sub-i64'; source="function probe(a: I64, b: I64) returns Result[I64, NumericError]:`n  return I64.checked_sub(a, b)`nend function`n"},
  @{name='checked-sub-u64'; source="function probe(a: U64, b: U64) returns Result[U64, NumericError]:`n  return U64.checked_sub(a, b)`nend function`n"}
)
$entries = @()
foreach ($f in $fixtures) {
  $sha = [BitConverter]::ToString([Security.Cryptography.SHA256]::Create().ComputeHash([Text.UTF8Encoding]::new($false).GetBytes($f.source))).Replace('-','').ToLower()
  $entries += [ordered]@{ name = $f.name; source_sha256 = $sha; expected = 'rust-codegen-byte-equal' }
}
$refusals = @(
  @{name='wrong-param-name'; source="function probe(x: I64) returns I64:`n  return x`nend function`n"},
  @{name='return-type-mismatch'; source="function probe(value: I64) returns U64:`n  return value`nend function`n"},
  @{name='three-params'; source="function probe(a: I64, b: I64, c: I64) returns I64:`n  return c`nend function`n"},
  @{name='undeclared-intrinsic'; source="function probe(a: I64, b: I64) returns Bool:`n  return I64.greater_than(a, b)`nend function`n"},
  @{name='checked-mul'; source="function probe(a: I64, b: I64) returns Result[I64, NumericError]:`n  return I64.checked_mul(a, b)`nend function`n"},
  @{name='u64-out-of-range'; source="function probe() returns U64:`n  return U64.literal(9223372036854775808)`nend function`n"},
  @{name='undeclared-i64-add'; source="function probe(value: I64) returns I64:`n  return I64.add(value, value)`nend function`n"}
)
$refusalEntries = @()
foreach ($f in $refusals) {
  $sha = [BitConverter]::ToString([Security.Cryptography.SHA256]::Create().ComputeHash([Text.UTF8Encoding]::new($false).GetBytes($f.source))).Replace('-','').ToLower()
  $refusalEntries += [ordered]@{ name = $f.name; source_sha256 = $sha; expected = 'typed-refusal-unsupported-codegen-source-shape' }
}
$doc = [ordered]@{
  schema = 'sico.m22.codegen-corpus.v0'
  created = '2026-09-29'
  step = 'STEP-0315+0322'
  backend = 'RFC-0011 compile() Direct Core-Wasm subset'
  byte_equal_entries = $entries
  typed_refusal_entries = $refusalEntries
}
($doc | ConvertTo-Json -Depth 5) | Set-Content -LiteralPath (Join-Path $root 'selfhost\codegen-corpus-v0.json') -Encoding ASCII
Write-Output 'codegen-corpus-v0.json written'
