# RFC-0033: Top-level Script syntax decision

> - status: accepted
> - created: 2026-07-19
> - phase: M9 / STEP-0092

## 1. Decision

M9 retains Candidate A: an explicit typed `main` function. Candidate B, unrestricted top-level statements, and Candidate C, a labeled `script:` block, are rejected for the current language and Script profiles.

```sico
function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  return ok(ScriptOutput(stdout: input.stdin, stderr: Bytes.empty(), exit_code: 0))
end function
```

This is an identity lowering to the existing normal function entry. There is no module initializer and no source-level wrapper or hidden capability scope. `sico eval` and the bounded expression REPL may synthesize an entry in memory under their own versioned contracts; that does not change source grammar.

## 2. Candidate evidence

| Surface | Parser/recovery | Formatter | HIR and source maps | LSP and AI tools | Decision |
|---|---|---|---|---|---|
| explicit typed `main` | existing declaration grammar and recovery | canonical and idempotent across the accepted corpus | existing function declaration and normal Script ABI entry | existing symbols, completion, diagnostics and generation examples | accepted |
| unrestricted top-level statements | previously parsed as success but silently absent from AST/HIR | no ownership for root execution order | requires module initialization, capability visibility and new source-map ownership | ambiguous root completion and misleading success | rejected with E1013 |
| labeled `script:` block | would reserve or contextually reinterpret the valid identifier `script` | needs a new block/close policy | needs synthetic declaration identity and diagnostic remapping | adds a second entry spelling and repair target | rejected with E1013 |

The parser now emits stable `E1013 / SYNTAX_UNEXPECTED_TOP_LEVEL` before semantic analysis. A rejected `script:` block produces one root diagnostic and recovery skips to `end script` or the next declaration. Formatter emits no output, HIR lowering is blocked, LSP publishes E1013, and AI inspect returns the same identity without echoing source.

## 3. Safety and compatibility

- top level contains declarations only;
- `script` remains an ordinary identifier and is not reserved;
- no statement can be silently discarded while `check` reports success;
- no implicit args, stdio, environment, storage, HTTP or process authority is introduced;
- existing explicit-main source, Script packages, caches and source maps are unchanged.

## 4. Reconsideration gate

A future syntax RFC may reconsider a shorthand only with a versioned grammar, explicit desugaring/source-map identity, module-initialization ordering, capability scoping, formatter recovery, LSP semantics, AI comparison evidence and a migration story. It cannot silently reinterpret RFC-0033 source.
