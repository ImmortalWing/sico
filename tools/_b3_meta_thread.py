#!/usr/bin/env python3
"""B3 第二步：把全模块走查改为每模块一次 meta 索引。

meta = List[U64]，13 项：entry0=count, entry1..12=12 helper 存在位（字典序）。
正确性依据：
- 后端 lib.rs:829-846 BTreeSet（名字字典序）+ helper_count+3+index。
- guest 12 名单即字典序，meta 位= module_has_intrinsic 同值，字节不变。
- function_position: id canonical 1-based == 1-based position（m22::function_index）。
"""
import sys

PATH = "selfhost/codegen.sico"

SIG_APPEND = [
    ("function cgs_intrinsic_hex(bytes: Bytes, run: U64, instr: U64, data: U64, result0: U64, vindex: List[U64]) returns Text:",
     "function cgs_intrinsic_hex(bytes: Bytes, run: U64, instr: U64, data: U64, result0: U64, vindex: List[U64], meta: List[U64]) returns Text:"),
    ("function cgs_op_hex(bytes: Bytes, run: U64, instr: U64, vindex: List[U64]) returns Text:",
     "function cgs_op_hex(bytes: Bytes, run: U64, instr: U64, vindex: List[U64], meta: List[U64]) returns Text:"),
    ("function cgs_run_instr_hex(bytes: Bytes) returns Text:",
     "function cgs_run_instr_hex(bytes: Bytes, meta: List[U64]) returns Text:"),
    ("function cgs_function_instr_hex(bytes: Bytes, fn: U64, terms: U64) returns Text:",
     "function cgs_function_instr_hex(bytes: Bytes, fn: U64, terms: U64, meta: List[U64]) returns Text:"),
    ("function cgs_core_types_hex(bytes: Bytes) returns Text:",
     "function cgs_core_types_hex(bytes: Bytes, meta: List[U64]) returns Text:"),
    ("function cgs_core_funcs_hex(bytes: Bytes) returns Text:",
     "function cgs_core_funcs_hex(bytes: Bytes, meta: List[U64]) returns Text:"),
    ("function cgs_bytes_equal_hex(bytes: Bytes, run: U64, instr: U64, data: U64, result0: U64, vindex: List[U64]) returns Text:",
     "function cgs_bytes_equal_hex(bytes: Bytes, run: U64, instr: U64, data: U64, result0: U64, vindex: List[U64], meta: List[U64]) returns Text:"),
    ("function cgs_starts_with_hex(bytes: Bytes, run: U64, instr: U64, data: U64, result0: U64, vindex: List[U64]) returns Text:",
     "function cgs_starts_with_hex(bytes: Bytes, run: U64, instr: U64, data: U64, result0: U64, vindex: List[U64], meta: List[U64]) returns Text:"),
    ("function cgs_u64_to_text_hex(bytes: Bytes, run: U64, instr: U64, data: U64, result0: U64, vindex: List[U64]) returns Text:",
     "function cgs_u64_to_text_hex(bytes: Bytes, run: U64, instr: U64, data: U64, result0: U64, vindex: List[U64], meta: List[U64]) returns Text:"),
    ("function cgs_collection_call_hex(bytes: Bytes, run: U64, instr: U64, data: U64, result0: U64, name: Text, vindex: List[U64]) returns Text:",
     "function cgs_collection_call_hex(bytes: Bytes, run: U64, instr: U64, data: U64, result0: U64, name: Text, vindex: List[U64], meta: List[U64]) returns Text:"),
    ("function cgs_call_hex(bytes: Bytes, run: U64, instr: U64, data: U64, result0: U64, vindex: List[U64]) returns Text:",
     "function cgs_call_hex(bytes: Bytes, run: U64, instr: U64, data: U64, result0: U64, vindex: List[U64], meta: List[U64]) returns Text:"),
    ("function cgs_function_body_hex(bytes: Bytes, index: U64) returns Text:",
     "function cgs_function_body_hex(bytes: Bytes, index: U64, meta: List[U64]) returns Text:"),
    ("function cgs_core_code_hex(bytes: Bytes, arena_base: U64) returns Text:",
     "function cgs_core_code_hex(bytes: Bytes, arena_base: U64, meta: List[U64]) returns Text:"),
    ("function cgs_run_body_hex(bytes: Bytes) returns Text:",
     "function cgs_run_body_hex(bytes: Bytes, meta: List[U64]) returns Text:"),
    ("function cg_run_body_hex_from_ir(ir: Bytes) returns Text:",
     "function cg_run_body_hex_from_ir(ir: Bytes, meta: List[U64]) returns Text:"),
]

SIG_LEAF = [
    ("function cgs_helper_index(bytes: Bytes, wanted: Text) returns U64:",
     "function cgs_helper_index(meta: List[U64], wanted: Text) returns U64:"),
    ("function cgs_function_position(bytes: Bytes, id: Text) returns U64:",
     "function cgs_function_position(meta: List[U64], id: Text) returns U64:"),
]

HELPER_HEAD = [
    ("function cgs_helper_index(meta: List[U64], wanted: Text) returns U64:\n  let cchi_count = cgs_function_count(bytes)",
     "function cgs_helper_index(meta: List[U64], wanted: Text) returns U64:\n  let cchi_count = cgs_meta_count(meta)"),
]

MODULE_HEAD = [
    ("  let cgmh_count = cgs_function_count(ir)\n  if U64.equal(cgmh_count, U64.literal(0)):\n    return \"\"\n  end if\n  let cgmh_data = cgs_data_hex(ir)",
     "  let cgmh_count = cgs_function_count(ir)\n  if U64.equal(cgmh_count, U64.literal(0)):\n    return \"\"\n  end if\n  let cgmh_meta = cgs_module_meta(ir)\n  let cgmh_data = cgs_data_hex(ir)"),
    ("  let cgmh_types = cgs_core_types_hex(ir)",
     "  let cgmh_types = cgs_core_types_hex(ir, cgmh_meta)"),
    ("  let cgmh_funcs = cgs_core_funcs_hex(ir)",
     "  let cgmh_funcs = cgs_core_funcs_hex(ir, cgmh_meta)"),
    ("  let cgmh_code = cgs_core_code_hex(ir, cgmh_arena)",
     "  let cgmh_code = cgs_core_code_hex(ir, cgmh_arena, cgmh_meta)"),
]

CALL_APPEND = [
    ("    return cgs_bytes_equal_hex(bytes, run, instr, data, result0, vindex)",
     "    return cgs_bytes_equal_hex(bytes, run, instr, data, result0, vindex, meta)", 1),
    ("    return cgs_starts_with_hex(bytes, run, instr, data, result0, vindex)",
     "    return cgs_starts_with_hex(bytes, run, instr, data, result0, vindex, meta)", 1),
    ("    return cgs_u64_to_text_hex(bytes, run, instr, data, result0, vindex)",
     "    return cgs_u64_to_text_hex(bytes, run, instr, data, result0, vindex, meta)", 1),
    ("    return cgs_collection_call_hex(bytes, run, instr, data, result0, cih_name, vindex)",
     "    return cgs_collection_call_hex(bytes, run, instr, data, result0, cih_name, vindex, meta)", 6),
    ("    return cgs_intrinsic_hex(bytes, run, instr, coph_data, coph_result0, vindex)",
     "    return cgs_intrinsic_hex(bytes, run, instr, coph_data, coph_result0, vindex, meta)", 1),
    ("    return cgs_call_hex(bytes, run, instr, coph_data, coph_result0, vindex)",
     "    return cgs_call_hex(bytes, run, instr, coph_data, coph_result0, vindex, meta)", 1),
    ("  let crih_instr = cgs_function_instr_hex(bytes, crih_run, U64.literal(0))",
     "  let crih_instr = cgs_function_instr_hex(bytes, crih_run, U64.literal(0), meta)", 1),
    ("      let crih_op = cgs_op_hex(bytes, crih_run, crih_instruction, vindex)",
     "      let crih_op = cgs_op_hex(bytes, crih_run, crih_instruction, vindex, meta)", 1),
    ("  let crbh_instr = cgs_run_instr_hex(bytes)",
     "  let crbh_instr = cgs_run_instr_hex(bytes, meta)", 1),
    ("    let cgfh_run_body = cg_run_body_hex_from_ir(bytes)",
     "    let cgfh_run_body = cg_run_body_hex_from_ir(bytes, meta)", 1),
    ("  let cgfh_code = cgs_function_instr_hex(bytes, cgfh_fn, U64.literal(1))",
     "  let cgfh_code = cgs_function_instr_hex(bytes, cgfh_fn, U64.literal(1), meta)", 1),
    ("    let cccsh_body = cgs_function_body_hex(bytes, cccsh_index)",
     "    let cccsh_body = cgs_function_body_hex(bytes, cccsh_index, meta)", 1),
]

CALL_LEAF = [
    ("  let cc_position = cgs_function_position(bytes, cc_target)",
     "  let cc_position = cgs_function_position(meta, cc_target)", 1),
    ("  let ccch_helper = cgs_helper_index(bytes, name)",
     "  let ccch_helper = cgs_helper_index(meta, name)", 1),
    ("    let cihc_helper = cgs_helper_index(bytes, \"sico.text.concat\")",
     "    let cihc_helper = cgs_helper_index(meta, \"sico.text.concat\")", 1),
    ("    let cihl_helper = cgs_helper_index(bytes, \"sico.text.length\")",
     "    let cihl_helper = cgs_helper_index(meta, \"sico.text.length\")", 1),
    ("  let cbeh_helper = cgs_helper_index(bytes, \"sico.bytes.equal\")",
     "  let cbeh_helper = cgs_helper_index(meta, \"sico.bytes.equal\")", 1),
    ("  let cstwh_helper = cgs_helper_index(bytes, \"sico.text.starts_with\")",
     "  let cstwh_helper = cgs_helper_index(meta, \"sico.text.starts_with\")", 1),
    ("  let cguth_helper = cgs_helper_index(bytes, \"sico.u64.to_text\")",
     "  let cguth_helper = cgs_helper_index(meta, \"sico.u64.to_text\")", 1),
]

GLOBAL = [
    ("cgs_module_has_intrinsic(bytes, \"", "cgs_meta_flag(meta, \"", 48),
]

COUNT_BY_PREFIX = [
    ("  let cctsh_count = cgs_function_count(bytes)",
     "  let cctsh_count = cgs_meta_count(meta)", 1),
    ("  let ccfsh_count = cgs_function_count(bytes)",
     "  let ccfsh_count = cgs_meta_count(meta)", 1),
    ("  let cccsh_count = cgs_function_count(bytes)",
     "  let cccsh_count = cgs_meta_count(meta)", 1),
]

NEW_FUNCS = """\
// The per-module emission index: entry 0 is the module function count, entries
// 1..12 are the presence flags of the 12 guest-known helper intrinsics, in the
// same dictionary order the backend's helper numbering uses.
//
// The backend collects the used helper set over every module function, block
// and instruction (crates/sico-codegen-wasm/src/lib.rs); the guest walks the
// same three levels once here instead of re-walking them on every helper index
// or function position lookup. The flags are the same values the per-intrinsic
// `cgs_module_has_intrinsic` returns, so the emitted bytes are unchanged.
function cgs_module_meta(bytes: Bytes) returns List[U64]:
  let cmm_count = cgs_function_count(bytes)
  let cmm_meta = sico.list.empty[U64]()
  set cmm_meta = sico.list.append[U64](cmm_meta, cmm_count)
  set cmm_meta = sico.list.append[U64](cmm_meta, cgs_module_has_intrinsic(bytes, "sico.bytes.equal"))
  set cmm_meta = sico.list.append[U64](cmm_meta, cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode"))
  set cmm_meta = sico.list.append[U64](cmm_meta, cgs_module_has_intrinsic(bytes, "sico.list.append"))
  set cmm_meta = sico.list.append[U64](cmm_meta, cgs_module_has_intrinsic(bytes, "sico.list.append[U64]"))
  set cmm_meta = sico.list.append[U64](cmm_meta, cgs_module_has_intrinsic(bytes, "sico.list.empty[U64]"))
  set cmm_meta = sico.list.append[U64](cmm_meta, cgs_module_has_intrinsic(bytes, "sico.list.get[U64]"))
  set cmm_meta = sico.list.append[U64](cmm_meta, cgs_module_has_intrinsic(bytes, "sico.text.concat"))
  set cmm_meta = sico.list.append[U64](cmm_meta, cgs_module_has_intrinsic(bytes, "sico.text.join"))
  set cmm_meta = sico.list.append[U64](cmm_meta, cgs_module_has_intrinsic(bytes, "sico.text.length"))
  set cmm_meta = sico.list.append[U64](cmm_meta, cgs_module_has_intrinsic(bytes, "sico.text.split_lines"))
  set cmm_meta = sico.list.append[U64](cmm_meta, cgs_module_has_intrinsic(bytes, "sico.text.starts_with"))
  set cmm_meta = sico.list.append[U64](cmm_meta, cgs_module_has_intrinsic(bytes, "sico.u64.to_text"))
  return cmm_meta
end function

// The module function count carried by the emission index (entry 0).
function cgs_meta_count(meta: List[U64]) returns U64:
  match sico.list.get[U64](meta, U64.literal(0)):
    case ok(cgsme_count):
      return cgsme_count
    case error(_):
      return U64.literal(0)
  end match
end function

// The presence flag of the helper named `wanted` in the emission index; 0 when
// the helper is outside the guest's 12-name vocabulary.
function cgs_meta_flag(meta: List[U64], wanted: Text) returns U64:
  let cmf_position = U64.literal(0)
  if cg_is_word(wanted, "sico.bytes.equal"):
    set cmf_position = U64.literal(1)
  end if
  if cg_is_word(wanted, "sico.bytes.utf8_decode"):
    set cmf_position = U64.literal(2)
  end if
  if cg_is_word(wanted, "sico.list.append"):
    set cmf_position = U64.literal(3)
  end if
  if cg_is_word(wanted, "sico.list.append[U64]"):
    set cmf_position = U64.literal(4)
  end if
  if cg_is_word(wanted, "sico.list.empty[U64]"):
    set cmf_position = U64.literal(5)
  end if
  if cg_is_word(wanted, "sico.list.get[U64]"):
    set cmf_position = U64.literal(6)
  end if
  if cg_is_word(wanted, "sico.text.concat"):
    set cmf_position = U64.literal(7)
  end if
  if cg_is_word(wanted, "sico.text.join"):
    set cmf_position = U64.literal(8)
  end if
  if cg_is_word(wanted, "sico.text.length"):
    set cmf_position = U64.literal(9)
  end if
  if cg_is_word(wanted, "sico.text.split_lines"):
    set cmf_position = U64.literal(10)
  end if
  if cg_is_word(wanted, "sico.text.starts_with"):
    set cmf_position = U64.literal(11)
  end if
  if cg_is_word(wanted, "sico.u64.to_text"):
    set cmf_position = U64.literal(12)
  end if
  if U64.equal(cmf_position, U64.literal(0)):
    return U64.literal(0)
  end if
  match sico.list.get[U64](meta, cmf_position):
    case ok(cmf_flag):
      return cmf_flag
    case error(_):
      return U64.literal(0)
  end match
end function


"""

FUNC_POS_OLD_START = "function cgs_function_position(meta: List[U64], id: Text) returns U64:"
FUNC_POS_NEW = """\
// The 1-based position of the module function whose id text is `id`; 0 when
// the module carries no such function.
//
// A module function id is canonical 1-based decimal and equals the function's
// 1-based position (the Rust verifier's m22::function_index maps
// `function.id == index + 1`), so the guest parses the id and range-checks it
// against the emission index's function count instead of re-walking the whole
// function list on every call instruction. A non-canonical or out-of-range id
// fails closed to 0.
function cgs_function_position(meta: List[U64], id: Text) returns U64:
  if cgs_value_id_canonical(id):
    let cfp_value = cgs_decimal_u64(id)
    if U64.equal(cfp_value, U64.literal(0)):
      return U64.literal(0)
    end if
    if U64.less_than(cgs_meta_count(meta), cfp_value):
      return U64.literal(0)
    end if
    return cfp_value
  end if
  return U64.literal(0)
end function"""


def main():
    with open(PATH, encoding="utf-8") as handle:
        text = handle.read()
    original = text

    def replace(old, new, expected, label):
        nonlocal text
        count = text.count(old)
        if count != expected:
            print("ABORT %s: expected %d got %d: %r" % (label, expected, count, old[:70]))
            sys.exit(1)
        text = text.replace(old, new)

    print("== signatures (append meta) ==")
    for old, new in SIG_APPEND:
        replace(old, new, 1, "sig")
    print("== signatures (leaf -> meta) ==")
    for old, new in SIG_LEAF:
        replace(old, new, 1, "sig-leaf")
    print("== helper_index head ==")
    for old, new in HELPER_HEAD:
        replace(old, new, 1, "helper-head")
    print("== module head (build meta) ==")
    for old, new in MODULE_HEAD:
        replace(old, new, 1, "module-head")
    print("== call sites (append meta) ==")
    for old, new, n in CALL_APPEND:
        replace(old, new, n, "call-append")
    print("== call sites (leaf -> meta) ==")
    for old, new, n in CALL_LEAF:
        replace(old, new, n, "call-leaf")
    print("== global module_has_intrinsic -> meta_flag ==")
    for old, new, n in GLOBAL:
        replace(old, new, n, "global")
    print("== core count -> meta_count ==")
    for old, new, n in COUNT_BY_PREFIX:
        replace(old, new, n, "count")

    anchor = "// The core function index of the standard-library helper named `wanted`."
    if text.count(anchor) != 1:
        print("ABORT anchor: %d" % text.count(anchor))
        sys.exit(1)
    text = text.replace(anchor, NEW_FUNCS + anchor)

    old_body_start = text.index(FUNC_POS_OLD_START)
    old_body_end = text.index("end function", old_body_start) + len("end function")
    text = text[:old_body_start] + FUNC_POS_NEW + text[old_body_end:]

    if text == original:
        print("ABORT: no changes")
        sys.exit(1)
    with open(PATH, "w", encoding="utf-8") as handle:
        handle.write(text)
    print("APPLIED ok")


if __name__ == "__main__":
    main()
