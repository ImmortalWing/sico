#!/usr/bin/env python3
"""B3 cursorization: replace in-loop cgj_array_item with cgj_array_first/next.

Strict, verified transform. Each site is matched exactly against its
surrounding code; any ambiguity aborts without writing.

For each site:
  del_item : remove `let VAR = cgj_array_item(BA, ARR, IDX)` line.
  init     : if keep_idx: insert `let VAR = cgj_array_first(BA, ARR)` after
             the `let IDX = U64.literal(0)` init.
             else: replace that init line with the cursor declaration.
  tail     : if keep_idx: append `set VAR = cgj_array_next(BA, VAR)` after the
             `set IDX = cg_next(IDX)` line.
             else: replace that tail line with the cursor advance.
"""
import re
import sys

PATH = 'selfhost/codegen.sico'
lines = open(PATH, encoding='utf-8').read().split('\n')

# (site_line, arr_expr, idx, var, keep_idx)
SITES = [
    (1003, 'arr', 'cgpil_index', 'cgpil_item', True),
    (1024, 'arr', 'cgpfl_index', 'cgpfl_item', True),
    (1196, 'cgpr_functions', 'cgpr_index', 'cgpr_item', False),
    (1223, 'cgpr_parameters', 'cgpr_parameter_index', 'cgpr_parameter', False),
    (1264, 'cgpr_blocks', 'cgpr_block_index', 'cgpr_block', False),
    (1276, 'cgpr_instructions', 'cgpr_instruction_index', 'cgpr_instruction', False),
    (1355, 'cgpr_record_fields', 'cgpr_field_index', 'cgpr_field', False),
    (1414, 'cgsrf_functions', 'cgsrf_index', 'cgsrf_item', False),
    (1572, 'cgsrh_fields', 'cgsrh_index', 'cgsrh_field', False),
    (1674, 'cgslh_blocks', 'cgslh_block_index', 'cgslh_block', False),
    (1686, 'cgslh_instructions', 'cgslh_instruction_index', 'cgslh_instruction', False),
    (1714, 'cgslh_cells', 'cgslh_cell_index', 'cgslh_cell', False),
    (2177, 'ccsls_locals', 'ccsls_index', 'ccsls_cell', True),
    (2206, 'cds_params', 'cds_pindex', 'cds_param', False),
    (2226, 'cds_blocks', 'cds_bindex', 'cds_block', False),
    (2234, 'cds_instructions', 'cds_iindex', 'cds_instruction', False),
    (2261, 'cds_cells', 'cds_cell_index', 'cds_cell', True),
    (2298, 'ccsl_locals', 'ccsl_index', 'ccsl_cell', True),
    (2331, 'crfo_fields', 'crfo_index', 'crfo_item', False),
    (2365, 'crfs_fields', 'crfs_index', 'crfs_item', False),
    (2540, 'cch_fields', 'cch_index', 'cch_item', False),
    (2629, 'cvh_payload', 'cvhl_index', 'cvhl_item', False),
    (2671, 'cvh_payload', 'cvh_pindex', 'cvh_item', False),
    (2908, 'cdo_functions', 'cdo_findex', 'cdo_function', False),
    (2919, 'cdo_blocks', 'cdo_bindex', 'cdo_block', False),
    (2930, 'cdo_instructions', 'cdo_iindex', 'cdo_instruction', False),
    (3052, 'cc_arguments', 'cc_aindex', 'cc_item', False),
    (3116, 'cfc_functions', 'cfc_index', 'cfc_item', False),
    (3166, 'cbhi_instructions', 'cbhi_index', 'cbhi_instruction', False),
    (3191, 'cfhi_blocks', 'cfhi_index', 'cfhi_block', False),
    (3220, 'cmhi_functions', 'cmhi_index', 'cmhi_function', False),
    (3446, 'ccch_arguments', 'ccch_index', 'ccch_item', True),   # count loop
    (3506, 'ccch_arguments', 'ccch_arg', 'ccch_arg_item', True), # count loop
    (4284, 'cifv_blocks', 'cifv_bindex', 'cifv_block', False),
    (4292, 'cifv_instructions', 'cifv_iindex', 'cifv_instruction', False),
    (4831, 'cdh_functions', 'cdh_findex', 'cdh_function', False),
    (4842, 'cdh_blocks', 'cdh_bindex', 'cdh_block', False),
    (4853, 'cdh_instructions', 'cdh_iindex', 'cdh_instruction', False),
    (4945, 'cfp_functions', 'cfp_index', 'cfp_item', False),
    (4971, 'cgsri_functions', 'cgsri_index', 'cgsri_item', True),
    (5042, 'ccth_param_array', 'ccth_pindex', 'ccth_param', False),
    (6264, 'payload', 'cmpo_index', 'cmpo_item', False),
    (6424, 'cgmh_arms', 'cgmh_index', 'cgmh_arm', False),
]

def find_while_body(ln):
    depth = 0
    for j in range(ln - 1, 0, -1):
        s = lines[j - 1].strip()
        if s.endswith('end while') or s.endswith('end while:'):
            depth += 1
        elif s.startswith('while ') and not s.endswith('end while'):
            if depth == 0:
                d = 0
                for k in range(j - 1, len(lines)):
                    t = lines[k].strip()
                    if t.endswith('end while') or t.endswith('end while:'):
                        d -= 1
                        if d == 0:
                            return j, k + 1
                    elif t.startswith('while ') and not t.endswith('end while'):
                        d += 1
                return j, None
            depth -= 1
    return None, None

def byte_arg(ln):
    m = re.search(r'cgj_array_item\(([^,]+),\s*([^,]+),', lines[ln - 1])
    return m.group(1).strip()

edits = []  # (line_index_1based, kind, payload)

for (ln, arr, idx, var, keep_idx) in SITES:
    m = re.search(r'let\s+' + re.escape(var) + r'\s*=\s*cgj_array_item\(([^,]+),\s*' +
                  re.escape(arr) + r',\s*' + re.escape(idx) + r'\)\s*$', lines[ln - 1])
    if not m:
        print(f"ABORT: site {ln} item-call mismatch: {lines[ln-1].strip()!r}")
        sys.exit(1)
    ba = byte_arg(ln)
    wl, we = find_while_body(ln)
    if wl is None:
        print(f"ABORT: site {ln} no enclosing while")
        sys.exit(1)
    # init line nearest above the while
    init = None
    for j in range(wl - 1, max(0, wl - 12), -1):
        if re.match(r'\s*let\s+' + re.escape(idx) + r'\s*=\s*U64\.literal\(0\)\s*$', lines[j - 1]):
            init = j
            break
    if init is None:
        print(f"ABORT: site {ln} no init for {idx}")
        sys.exit(1)
    # tail line inside the while body
    tail = None
    for j in range(wl, we):
        if re.match(r'\s*set\s+' + re.escape(idx) + r'\s*=\s*cg_next\(' + re.escape(idx) + r'\)\s*$', lines[j - 1]):
            tail = j
            break
    if tail is None:
        print(f"ABORT: site {ln} no tail increment for {idx}")
        sys.exit(1)
    # idx must not be used after the tail
    uses = [k for k in range(wl, we) if re.search(r'\b' + re.escape(idx) + r'\b', lines[k - 1])]
    if max(uses) != tail:
        print(f"ABORT: site {ln} idx used after tail at {max(uses)}")
        sys.exit(1)
    # keep_idx is fixed by the catalogue; verify idx is NOT used after the tail
    edits.append((ln, 'del_item', var))
    edits.append((init, 'hoist', var, ba, arr, idx, keep_idx))
    edits.append((tail, 'advance', var, ba, idx, keep_idx))

# ---- cmch special (two cursors, shared index, MORE-form) ----
ln1, ln2 = 6373, 6374
m1 = re.search(r'let\s+(cmch_value)\s*=\s*cgj_array_item\(([^,]+),\s*values,\s*cmch_index\)\s*$', lines[ln1 - 1])
m2 = re.search(r'let\s+(cmch_pattern)\s*=\s*cgj_array_item\(([^,]+),\s*patterns,\s*cmch_index\)\s*$', lines[ln2 - 1])
if not (m1 and m2):
    print("ABORT: cmch site mismatch")
    sys.exit(1)
ba1, ba2 = m1.group(2).strip(), m2.group(2).strip()
wl, we = find_while_body(ln1)
init = None
for j in range(wl - 1, max(0, wl - 12), -1):
    if re.match(r'\s*let\s+cmch_index\s*=\s*U64\.literal\(0\)\s*$', lines[j - 1]):
        init = j
        break
if init is None:
    print("ABORT: cmch no init")
    sys.exit(1)
tail = None
for j in range(wl, we):
    if re.match(r'\s*set\s+cmch_index\s*=\s*cg_next\(cmch_index\)\s*$', lines[j - 1]):
        tail = j
        break
if tail is None:
    print("ABORT: cmch no tail")
    sys.exit(1)
edits.append((ln1, 'del_item', 'cmch_value'))
edits.append((ln2, 'del_item', 'cmch_pattern'))
edits.append((init, 'hoist2', 'cmch_value', ba1, 'values', 'cmch_pattern', ba2, 'patterns'))
edits.append((tail, 'advance2', 'cmch_value', ba1, 'cmch_pattern', ba2))

# ---- Apply bottom-up ----
edits.sort(key=lambda e: e[0], reverse=True)
for e in edits:
    kind, ln = e[1], e[0]
    if kind == 'del_item':
        var = e[2]
        assert re.search(r'let\s+' + re.escape(var) + r'\s*=\s*cgj_array_item\(', lines[ln - 1])
        lines[ln - 1] = None
    elif kind == 'hoist':
        _, _, var, ba, arr, idx, keep = e
        indent = re.match(r'\s*', lines[ln - 1]).group(0)
        if keep:
            assert re.match(r'\s*let\s+' + re.escape(idx) + r'\s*=\s*U64\.literal\(0\)\s*$', lines[ln - 1])
            lines.insert(ln, indent + f'let {var} = cgj_array_first({ba}, {arr})')
        else:
            assert re.match(r'\s*let\s+' + re.escape(idx) + r'\s*=\s*U64\.literal\(0\)\s*$', lines[ln - 1])
            lines[ln - 1] = indent + f'let {var} = cgj_array_first({ba}, {arr})'
    elif kind == 'advance':
        _, _, var, ba, idx, keep = e
        indent = re.match(r'\s*', lines[ln - 1]).group(0)
        assert re.match(r'\s*set\s+' + re.escape(idx) + r'\s*=\s*cg_next\(' + re.escape(idx) + r'\)\s*$', lines[ln - 1])
        if keep:
            lines[ln - 1] = (indent + f'set {idx} = cg_next({idx})\n' +
                             indent + f'set {var} = cgj_array_next({ba}, {var})')
        else:
            lines[ln - 1] = indent + f'set {var} = cgj_array_next({ba}, {var})'
    elif kind == 'hoist2':
        _, _, v1, b1, a1, v2, b2, a2 = e
        indent = re.match(r'\s*', lines[ln - 1]).group(0)
        assert re.match(r'\s*let\s+cmch_index\s*=\s*U64\.literal\(0\)\s*$', lines[ln - 1])
        lines[ln - 1] = (indent + f'let {v1} = cgj_array_first({b1}, {a1})\n' +
                         indent + f'let {v2} = cgj_array_first({b2}, {a2})')
    elif kind == 'advance2':
        _, _, v1, b1, v2, b2 = e
        indent = re.match(r'\s*', lines[ln - 1]).group(0)
        assert re.match(r'\s*set\s+cmch_index\s*=\s*cg_next\(cmch_index\)\s*$', lines[ln - 1])
        lines[ln - 1] = (indent + f'set {v1} = cgj_array_next({b1}, {v1})\n' +
                         indent + f'set {v2} = cgj_array_next({b2}, {v2})')
    else:
        print(f"ABORT: unknown edit kind {kind}")
        sys.exit(1)

out = [l for l in lines if l is not None]
open(PATH, 'w', encoding='utf-8').write('\n'.join(out))
print(f"APPLIED. sites={len(SITES) + 1} edits={len(edits)}")
