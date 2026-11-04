import io

p = 'runner/sico-runner/tests/selfhost_compiler.rs'
t = io.open(p, encoding='utf-8').read()

old = '        "Bytes" => "sico.text.encode(\\"\\")",\n        other => panic!("codegen function {name} has an unexpected return type {other}"),'
new = (
    '        "Bytes" => "sico.text.encode(\\"\\")",\n'
    '        // STEP-0551 (B2) added cgs_value_index, whose per-function value\n'
    '        // index is the stride-2 List[U64]. A stub only carries the real\n'
    '        // signature so the call lowers identically, so an empty list is the\n'
    '        // right trivial body.\n'
    '        "List[U64]" => "sico.list.empty[U64]()",\n'
    '        other => panic!("codegen function {name} has an unexpected return type {other}"),'
)

count = t.count(old)
assert count == 1, "anchor count=%d" % count
t = t.replace(old, new, 1)
io.open(p, 'w', encoding='utf-8', newline='').write(t)
print('stub List[U64] branch added')
