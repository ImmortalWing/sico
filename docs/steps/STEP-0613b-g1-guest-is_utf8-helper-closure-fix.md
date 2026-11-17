# STEP-0613b — G1 guest `is_utf8 → utf8_decode` helper-dependency closure fix (FIX-FIRST)

- Date: 2026-11-09
- Track: M22 compiler self-host, goal **W3-a**
- Slice type: **apply one small, precisely-specified FIX to the guest emitter** — no commit/push, no cargo.
- Scope: `selfhost/codegen.sico` only (the hot file; I am the only writer). The worktree is
  `D:/SorftWare/sico-m22-w3a`, branch `codex/m22-w2-step0613`, HEAD `e93f59a`.
- Fixes the single blocking bug found by the static review
  [`STEP-0613-review-static-g1-bootstrap-deRisk.md`](./STEP-0613-review-static-g1-bootstrap-deRisk.md) §4.3/§5
  (verdict #4 SUSPECT → BUG on the guest assertion).
- **Summary (Chinese):** 给 guest 发射器补上 oracle O1 的单向 helper 依赖闭包
  `is_utf8 → utf8_decode`（镜像 `lib.rs:546`）。guest 原模型是「直接使用的 intrinsic helper」
  无此闭包，导致只用 `is_utf8` 的模块发射 `call 4` 但未定义 `utf8_decode` 的类型/函数/函数体。
  本片在 4 处把 `utf8_decode` 的「使用」判定改为「直接使用 `utf8_decode` 或使用 `is_utf8`」，
  使 helper 索引 `call 4` 命中真实定义的函数。仅静态修改 + 非 cargo 校验；cargo 钉仍在工具链可用后执行。

---

## 1. The bug (recap, from review §4.3)

The oracle's used-helper model (`helper_dependencies`, `lib.rs:510-562`) is a **dependency closure**:
`used_helpers` is the union of `helper_dependencies(intrinsic)` over every intrinsic in the module
(`lib.rs:835-846`). STEP-0613 O1 added the single edge `"sico.bytes.is_utf8" => &["sico.bytes.utf8_decode"]`
(`lib.rs:546`), so a module that uses only `is_utf8` registers `utf8_decode` as used.

The guest's used-helper model is **directly-used intrinsics only**: every helper-section gate is
`cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode")` — a walk for a *direct* `utf8_decode`
instruction. There is **no** `is_utf8 → utf8_decode` closure in the guest (verified: all `utf8_decode`
references are `:181/:713/:752/:3268/:3271/:3806/:3911/:5189/:5308/:6651`; none maps `is_utf8` to it).

For `IS_UTF8_GATE_FIXTURE_SOURCE` (selfhost_compiler.rs:3943): the module carries **no** direct
`sico.bytes.utf8_decode` instruction → all three section gates are 0 → the guest emits **no** `utf8_decode`
type entry, **no** function entry, **no** body. But the inline `is_utf8` arm emits `call <index>`:
`cgs_is_utf8_hex` (was codegen.sico:3911) computes `cgs_helper_index(bytes, "sico.bytes.utf8_decode")`,
which returns `base + rank` unconditionally for `wanted == "sico.bytes.utf8_decode"`
(codegen.sico:3268-3269). For the fixture (`cgs_function_count = 1`, no `equal`, no other helper):
`base = 1 + 3 = 4`, `rank = 0` → returns **4**. So the guest emits `... call 4 ...` (matching the oracle's
`call 4`) **but defines only functions 0–3** (`run` + alloc/realloc/post). Function 4 does not exist →
the component is shorter than the oracle's and invalid (`call 4` → undefined function index) → both new
STEP-0613 pins fail (`assert_eq!` at :3982 and `validate_all` at :3985).

## 2. The fix

Mirror the oracle's O1 edge exactly: the `utf8_decode` helper is "used" when the module **directly calls**
`sico.bytes.utf8_decode` **OR** uses the `sico.bytes.is_utf8` intrinsic (whose inline arm needs the
`utf8_decode` runtime helper). The guest language has no boolean `or` operator in a usable form
(verified: no ` or ` in any `if` condition anywhere in `codegen.sico`; `parser.sico` has no `or` token),
so the OR is expressed as an **in-place set-if-zero** — a 0/1 local that is set to the `is_utf8` flag when
the direct `utf8_decode` flag is 0. This keeps every downstream `== 1` gate and every comparison shape
unchanged, and computes exactly `used_helpers.contains("sico.bytes.utf8_decode")` as the oracle does.

Four sites, each a +3-line block; the only new local is `cchi_utf8` (all four names unique in the file).

### 2.1 `cgs_helper_index` — rank gate (codegen.sico:3271→3271-3277)

The `utf8_decode` **rank increment** (the gate that counts `utf8_decode` as used for the rank of every
helper that sorts after it in the vocabulary) was `cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode") == 1`.
Before (codegen.sico:3271):

```
  if U64.equal(cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode"), U64.literal(1)):
    set cchi_rank = cgs_add(cchi_rank, U64.literal(1))
  end if
```

After (codegen.sico:3271-3277):

```
  let cchi_utf8 = cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode")
  if U64.equal(cchi_utf8, U64.literal(0)):
    set cchi_utf8 = cgs_module_has_intrinsic(bytes, "sico.bytes.is_utf8")
  end if
  if U64.equal(cchi_utf8, U64.literal(1)):
    set cchi_rank = cgs_add(cchi_rank, U64.literal(1))
  end if
```

`cchi_utf8` is declared *after* the `wanted == "sico.bytes.utf8_decode"` early return
(codegen.sico:3268-3270), so a module asking for `utf8_decode`'s own index is unaffected — its rank is
still `base + (equal used ? 1 : 0)`, which is correct because the closure adds no helper that sorts
*before* `utf8_decode`.

### 2.2 `cgs_core_types_hex` — type-section gate (codegen.sico:5189→5193-5196)

`let cctsh_utf8` was `cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode")`. The value gates both the
type-count total (now codegen.sico:5202) and the `utf8_decode` type entry emission `"60027f7f017f"`
(now codegen.sico:5264). Before (codegen.sico:5189):

```
  let cctsh_utf8 = cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode")
```

After (codegen.sico:5193-5196):

```
  let cctsh_utf8 = cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode")
  if U64.equal(cctsh_utf8, U64.literal(0)):
    set cctsh_utf8 = cgs_module_has_intrinsic(bytes, "sico.bytes.is_utf8")
  end if
```

### 2.3 `cgs_core_funcs_hex` — function-section gate (codegen.sico:5308→5315-5318)

`let ccfsh_utf8` was the direct `utf8_decode` check; the value gates the function-count total
(now codegen.sico:5324), i.e. whether the `utf8_decode` function entry exists. Before (codegen.sico:5308):

```
  let ccfsh_utf8 = cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode")
```

After (codegen.sico:5315-5318):

```
  let ccfsh_utf8 = cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode")
  if U64.equal(ccfsh_utf8, U64.literal(0)):
    set ccfsh_utf8 = cgs_module_has_intrinsic(bytes, "sico.bytes.is_utf8")
  end if
```

### 2.4 `cgs_core_code_hex` — code-section gate (codegen.sico:6651→6661-6664)

`let cccsh_utf8` was the direct `utf8_decode` check; the value gates the body-count total
(now codegen.sico:6670) and the `utf8_decode` helper body emission (now codegen.sico:6764-6766,
`cg_utf8_validate_helper_body_hex`). Before (codegen.sico:6651):

```
  let cccsh_utf8 = cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode")
```

After (codegen.sico:6661-6664):

```
  let cccsh_utf8 = cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode")
  if U64.equal(cccsh_utf8, U64.literal(0)):
    set cccsh_utf8 = cgs_module_has_intrinsic(bytes, "sico.bytes.is_utf8")
  end if
```

## 3. How each mirrors the oracle's `lib.rs:546`

The oracle's `helper_dependencies("sico.bytes.is_utf8")` returns `&["sico.bytes.utf8_decode"]`
(lib.rs:546), and `used_helpers` is the union of that over all intrinsics (lib.rs:835-846). Therefore:

```
used_helpers.contains("sico.bytes.utf8_decode")
  ⇔ (module directly calls utf8_decode)  OR  (module calls is_utf8)
```

Each guest `xxx_utf8` local now computes exactly that disjunction:
- `cgs_helper_index` (`cchi_utf8`, §2.1): the value increments `cchi_rank` for every helper that sorts
  after `utf8_decode`, exactly as the oracle's sorted `used_helpers` set counts it.
- `cgs_core_types_hex` (`cctsh_utf8`, §2.2): gates the type-count and the `utf8_decode` type entry —
  the oracle emits the helper type iff `utf8_decode ∈ used_helpers`.
- `cgs_core_funcs_hex` (`ccfsh_utf8`, §2.3): gates the function-count — the oracle emits the helper
  function entry iff `utf8_decode ∈ used_helpers`.
- `cgs_core_code_hex` (`cccsh_utf8`, §2.4): gates the body-count and the helper body — the oracle emits
  the helper body iff `utf8_decode ∈ used_helpers`.

No mechanism was invented: the guest already models "helper used" as a per-section 0/1 flag; the fix
only widens the `utf8_decode` flag's definition to the same one-direction closure the oracle applies.
The three sections and `cgs_helper_index` all use the **same** vocabulary order
(`equal, utf8_decode, list.append, list.append[U64], list.empty[U64], list.get[U64], list.length[U64],
text.concat, text.join, text.length, text.split_lines, text.starts_with, u64.to_text`), so the type entry
at type-index `k`, the function at function-index `k`, and the body at body-index `k` stay aligned and
`cgs_helper_index` returns the matching `k`.

## 4. Why the helper index now defines `call 4`

For `IS_UTF8_GATE_FIXTURE_SOURCE` (one `run`, no `equal`, no direct `utf8_decode`, no other helper):

- `cgs_function_count = 1`; the oracle's helper index for `utf8_decode` is
  `import_count + fn_count + 3 + rank = 0 + 1 + 3 + 0 = 4` (lib.rs:797-800/852); the guest's
  `cgs_helper_index` returns `base + rank = (1 + 3) + 0 = 4` (codegen.sico:3260/3268-3269). Both sides
  agree on **`call 4`** regardless of the fix (the closure adds no helper sorting before `utf8_decode`).
- **With the fix**, `cctsh_utf8`/`ccfsh_utf8`/`cccsh_utf8` are all 1 for the fixture (the module uses
  `is_utf8`), so:
  - the type section emits `count + 3 + 1 = 5` types: the module function type (0), the three arena types
    (1-3), and the `utf8_decode` type `"60027f7f017f"` at **type-index 4**;
  - the function section emits `5` functions: `run`(0), alloc(1), realloc(2), post(3), `utf8_decode`(4);
  - the code section emits 5 bodies, the fifth being `cg_utf8_validate_helper_body_hex()` at **body-index 4**.
- Therefore `call 4` now targets an actually-defined function (type `(i32,i32)->i32`, matching the
  oracle's helper layout), the component is no longer short, and the bytes match the oracle's
  `call 4` + defined-helper entries. Both new STEP-0613 pins (`assert_eq!(actual, expected)` and
  `wasmparser::Validator`) are unblocked on the guest side.

For the general class (a module using `is_utf8` **and** a later helper such as `sico.list.append`, but no
direct `utf8_decode`): the §2.1 rank fix is load-bearing — without it `cgs_helper_index` would undercount
`utf8_decode` and return `base + 0` for `list.append` while the code section emits that body at
`base + 1`, so the call would target the wrong body. With the fix both compute `base + 1`.

## 5. Non-cargo validation results (all pass)

- `python tools/validate-selfhost-comment-discipline.py selfhost/codegen.sico`
  → `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2`, exit 0. (No `//` comments were added inside any function
  body; the 4 additions are pure statements.)
- `git diff --check` → exit 0 (no whitespace errors).
- Manual re-read of all four regions confirms the blocks are syntactically well-formed and placed after
  the corresponding `let xxx_utf8` / after the `utf8_decode` early return, before every downstream gate.
- Python interpreter: `python` resolves to Python 3.15.2 (not a WindowsApps stub).

## 6. Remaining to run once a toolchain exists (NOT provable statically)

This host has **no Rust toolchain**, so no cargo/build/test/download was run. The following remain:

1. `step0507` / `step0513` / `step0526` self-host pins stay green (bootstrap statement-frontier — the
   four added `if`/`set` blocks are strictly within the already-green envelope, but the frontier is
   measured, not derivable; review §3.2/§3.3).
2. The two new STEP-0613 G1 pins turn green after this fix:
   `sico_is_utf8_gated_run_lowers_byte_exactly` and
   `sico_run_emits_the_minimal_multiblock_run_component_byte_exactly` (helper-section parity for the
   fixture — the blocking bug this fix removes).
3. The single-block `run` pins (STEP-0504/0509) stay green (no regression).
4. `sico_compiler_lowers_full_formatter_byte_exactly` and the whole-`compile_guest()` bootstrap still
   build (a red here breaks every `run_guest*` test).
5. `wasmparser::Validator` accepts the guest fixture output.

## 7. Diff summary

`selfhost/codegen.sico`: the four hunks added by this fix are +3 lines each (12 lines added net). The
file also carries the pre-existing STEP-0613 G1 diff (multi-block `run` restructure), which this fix does
not touch. `git diff --stat -- selfhost/codegen.sico` reports
`1 file changed, 36 insertions(+), 70 deletions(-)` — of which **+12 insertions are this fix** (the four
`if`/`set` closure blocks) and the rest is the pre-existing G1 change.

Worktree SHA of `selfhost/codegen.sico`:
- before fix: `b298918df675ea2ef414823650a96375520b31a6624ae07451b4df9de05032ec`
- after fix:  `def9fe6ceaf354d1e442f340689c1a95d87d6227f3fbde0c37172fbea2fd8267`

No commit, no push. No gate flipped; M22 overall remains NO-GO.
