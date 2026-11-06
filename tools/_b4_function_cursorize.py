import re
p = "selfhost/codegen.sico"
src = open(p, encoding="utf-8").read()

# --- Edit 1: cgs_function_body_hex takes the function NODE instead of index ---
old_sig = '''function cgs_function_body_hex(bytes: Bytes, index: U64) returns Text:
  let cgfh_fn = cgs_function_at_index(bytes, index)
  if U64.equal(cgfh_fn, U64.literal(0)):
    return ""
  end if
  let cgfh_name = cgj_ident_text(bytes, cgj_member(bytes, cgfh_fn, "name"))'''
new_sig = '''function cgs_function_body_hex(bytes: Bytes, fn: U64) returns Text:
  let cgfh_fn = fn
  if U64.equal(cgfh_fn, U64.literal(0)):
    return ""
  end if
  let cgfh_name = cgj_ident_text(bytes, cgj_member(bytes, cgfh_fn, "name"))'''
assert src.count(old_sig) == 1, f"edit1 anchor count {src.count(old_sig)}"
src = src.replace(old_sig, new_sig, 1)

# --- Edit 2: cgs_core_code_hex iterates by cursor, not index ---
old_loop = '''  let cccsh_index = U64.literal(0)
  while U64.less_than(cccsh_index, cccsh_count):
    let cccsh_body = cgs_function_body_hex(bytes, cccsh_index)'''
new_loop = '''  let cccsh_functions = cgj_member(bytes, U64.literal(0), "functions")
  if U64.equal(cccsh_functions, U64.literal(0)):
    return ""
  end if
  let cccsh_fn = cgj_array_first(bytes, cccsh_functions)
  while U64.less_than(U64.literal(0), cccsh_fn):
    let cccsh_body = cgs_function_body_hex(bytes, cccsh_fn)'''
assert src.count(old_loop) == 1, f"edit2 anchor count {src.count(old_loop)}"
src = src.replace(old_loop, new_loop, 1)

old_incr = '''    set cccsh_parts = sico.list.append(cccsh_parts, cccsh_body)
    set cccsh_index = cg_next(cccsh_index)
  end while
  let cccsh_alloc = cg_alloc_body_hex()'''
new_incr = '''    set cccsh_parts = sico.list.append(cccsh_parts, cccsh_body)
    set cccsh_fn = cgj_array_next(bytes, cccsh_fn)
  end while
  let cccsh_alloc = cg_alloc_body_hex()'''
assert src.count(old_incr) == 1, f"edit3 anchor count {src.count(old_incr)}"
src = src.replace(old_incr, new_incr, 1)

open(p, "w", encoding="utf-8").write(src)
print("EDITED")
