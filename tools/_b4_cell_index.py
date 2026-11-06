#!/usr/bin/env python3
"""B4 candidate D: index the per-function local-cell layout.

`cgs_cell_slot` / `cgs_cell_width` currently rescan the whole function on every
read_local / write_local instruction (O(L*I) / O(I) each), so a function with K
instructions costs O(K*L*I).  B2 already threads `vindex` (value slots) through
`cgs_op_hex`; this change threads a per-function `cindex` (cell widths) and the
`dispatcher` slot alongside it, making each cell lookup O(1).

Output bytes must stay identical: codegen_guest asserts component byte equality.
"""
import re
import sys

PATH = "selfhost/codegen.sico"


def read_source():
    with open(PATH, "r", encoding="utf-8") as handle:
        return handle.read()


def write_source(text):
    with open(PATH, "w", encoding="utf-8") as handle:
        handle.write(text)


def find_function(source, name):
    start = source.index("function %s(" % name)
    end = source.index("\nend function", start) + len("\nend function")
    return start, end


def insert_after_function(source, name, addition):
    anchor = find_function(source, name)[1]
    return source[:anchor] + "\n\n" + addition + source[anchor:]


def replace_function(source, name, replacement):
    start, end = find_function(source, name)
    return source[:start] + replacement + source[end:]


CELL_INDEX = """\
function cgs_cell_index(bytes: Bytes, fn: U64) returns List[U64]:
  let ccix_total = U64.literal(0)
  let ccix_locals = cgj_member(bytes, fn, "locals")
  if U64.less_than(U64.literal(0), ccix_locals):
    let ccix_cell_a = cgj_array_first(bytes, ccix_locals)
    while U64.less_than(U64.literal(0), ccix_cell_a):
      set ccix_total = cgs_add(ccix_total, cgs_cell_flat_width(bytes, cgj_member(bytes, ccix_cell_a, "ty")))
      set ccix_cell_a = cgj_array_next(bytes, ccix_cell_a)
    end while
  end if
  let ccix_index = sico.list.empty[U64]()
  let ccix_before = U64.literal(0)
  if U64.less_than(U64.literal(0), ccix_locals):
    let ccix_cell_b = cgj_array_first(bytes, ccix_locals)
    while U64.less_than(U64.literal(0), ccix_cell_b):
      let ccix_w = cgs_cell_flat_width(bytes, cgj_member(bytes, ccix_cell_b, "ty"))
      set ccix_index = sico.list.append[U64](ccix_index, cgs_sub(ccix_total, ccix_before))
      set ccix_index = sico.list.append[U64](ccix_index, ccix_w)
      set ccix_before = cgs_add(ccix_before, ccix_w)
      set ccix_cell_b = cgj_array_next(bytes, ccix_cell_b)
    end while
  end if
  set ccix_index = sico.list.append[U64](ccix_index, U64.literal(0))
  set ccix_index = sico.list.append[U64](ccix_index, U64.literal(0))
  return ccix_index
end function"""


CELL_SLOT_FROM = """\
function cgs_cell_slot_from(dispatcher: U64, cindex: List[U64], index: U64) returns U64:
  let ccsf_at = cgs_mul(index, U64.literal(2))
  match sico.list.get[U64](cindex, ccsf_at):
    case ok(ccsf_before):
      return cgs_sub(dispatcher, ccsf_before)
    case error(_):
      return U64.literal(0)
  end match
end function"""


CELL_WIDTH_FROM = """\
function cgs_cell_width_from(cindex: List[U64], index: U64) returns U64:
  let ccwf_at = cgs_add(cgs_mul(index, U64.literal(2)), U64.literal(1))
  match sico.list.get[U64](cindex, ccwf_at):
    case ok(ccwf_width):
      return ccwf_width
    case error(_):
      return U64.literal(0)
  end match
end function"""


CELL_FLAT_WIDTH = """\
function cgs_cell_flat_width(bytes: Bytes, ty: U64) returns U64:
  let ccfw_hex = cgs_cell_flat_hex(bytes, ty)
  if U64.equal(sico.text.length(ccfw_hex), U64.literal(0)):
    return U64.literal(0)
  end if
  return U64.shr(sico.text.length(ccfw_hex), U64.literal(2))
end function"""


def rewrite_cell_slot(source):
    return replace_function(
        source,
        "cgs_cell_slot",
        CELL_SLOT_FROM.replace("cgs_cell_slot_from", "cgs_cell_slot_from")
        + "\n",
    )


def main():
    source = read_source()
    original = source

    # 1. new index builder + O(1) accessors, inserted right before cgs_cell_slot.
    start, _ = find_function(source, "cgs_cell_slot")
    addition = (
        CELL_INDEX
        + "\n\n"
        + CELL_SLOT_FROM
        + "\n\n"
        + CELL_WIDTH_FROM
        + "\n\n"
        + CELL_FLAT_WIDTH
        + "\n"
    )
    source = source[:start] + addition + source[start:]

    # 2. thread dispatcher + cindex into op_hex / write_local / read_local.
    source = source.replace(
        "function cgs_op_hex(bytes: Bytes, run: U64, instr: U64, vindex: List[U64]) returns Text:",
        "function cgs_op_hex(bytes: Bytes, run: U64, instr: U64, vindex: List[U64], dispatcher: U64, cindex: List[U64]) returns Text:",
        1,
    )
    source = source.replace(
        "function cgs_write_local_hex(bytes: Bytes, fn: U64, instr: U64, data: U64, vindex: List[U64]) returns Text:",
        "function cgs_write_local_hex(bytes: Bytes, fn: U64, instr: U64, data: U64, vindex: List[U64], dispatcher: U64, cindex: List[U64]) returns Text:",
        1,
    )
    source = source.replace(
        "function cgs_read_local_hex(bytes: Bytes, fn: U64, instr: U64, data: U64, result0: U64) returns Text:",
        "function cgs_read_local_hex(bytes: Bytes, fn: U64, instr: U64, data: U64, result0: U64, vindex: List[U64], dispatcher: U64, cindex: List[U64]) returns Text:",
        1,
    )
    # call sites inside op_hex
    source = source.replace(
        "return cgs_write_local_hex(bytes, run, instr, coph_data, vindex)",
        "return cgs_write_local_hex(bytes, run, instr, coph_data, vindex, dispatcher, cindex)",
        1,
    )
    source = source.replace(
        "return cgs_read_local_hex(bytes, run, instr, coph_data, coph_result0)",
        "return cgs_read_local_hex(bytes, run, instr, coph_data, coph_result0, vindex, dispatcher, cindex)",
        1,
    )
    # cell lookups inside write_local / read_local
    source = source.replace(
        "let cwlh_cell = cgs_cell_slot(bytes, fn, cwlh_local)",
        "let cwlh_cell = cgs_cell_slot_from(dispatcher, cindex, cwlh_local)",
        1,
    )
    source = source.replace(
        "let cwlh_cell_width = cgs_cell_width(bytes, fn, cwlh_local)",
        "let cwlh_cell_width = cgs_cell_width_from(cindex, cwlh_local)",
        1,
    )
    source = source.replace(
        "let ccrlh_cell = cgs_cell_slot(bytes, fn, ccrlh_local)",
        "let ccrlh_cell = cgs_cell_slot_from(dispatcher, cindex, ccrlh_local)",
        1,
    )
    source = source.replace(
        "let ccrlh_cell_width = cgs_cell_width(bytes, fn, ccrlh_local)",
        "let ccrlh_cell_width = cgs_cell_width_from(cindex, ccrlh_local)",
        1,
    )
    # build cindex alongside vindex in instr_hex
    source = source.replace(
        "  let vindex = cgs_value_index(bytes, crih_run)\n"
        "  let crih_terms = terms",
        "  let vindex = cgs_value_index(bytes, crih_run)\n"
        "  let cindex = cgs_cell_index(bytes, crih_run)\n"
        "  let crih_terms = terms",
        1,
    )
    # thread dispatcher (already computed) + cindex into op_hex
    source = source.replace(
        "let crih_op = cgs_op_hex(bytes, crih_run, crih_instruction, vindex)",
        "let crih_op = cgs_op_hex(bytes, crih_run, crih_instruction, vindex, crih_dispatcher, cindex)",
        1,
    )

    if source == original:
        print("no change applied")
        sys.exit(1)
    write_source(source)
    print("patched")


if __name__ == "__main__":
    main()
