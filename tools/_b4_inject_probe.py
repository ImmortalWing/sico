p = "runner/sico-runner/tests/selfhost_compiler.rs"
src = open(p, encoding="utf-8").read()

probe = r'''#[test]
#[ignore = "a synthetic scaling probe for the emitter's fuel cost; run it explicitly"]
fn the_emitter_fuel_scaling_in_read_local_count_is_measured() {
    let ceiling = std::env::var("SICO_M22_PROBE_FUEL")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(200_000_000_000);
    // The quadratic theory: each `read_local` emission calls `cgs_cell_slot`,
    // which rescans the enclosing function's JSON (`cgs_dispatcher_slot` +
    // `cgs_cell_slots`).  N such instructions in one function should therefore
    // cost O(N^2), so doubling N should roughly quadruple the minimal fuel.
    // This probe brackets the minimal fuel for N = 4, 8, 16, 32 and prints the
    // growth ratio so the hypothesis is judged from data, not design.
    for count in [4usize, 8, 16, 32] {
        let mut body = String::from("function probe_closure(data: U64) returns U64:\n");
        body.push_str("  let n0 = data\n");
        for i in 1..count {
            body.push_str(&format!("  let n{i} = n{}\n", i - 1));
        }
        body.push_str(&format!("  return n{}\nend function\n", count - 1));
        let source = format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}");
        let mut low = 0u64;
        let mut budget = 100_000_000u64;
        let verdict = loop {
            match guest_component_with_fuel(&source, budget) {
                GuestFuelEmission::Component(_) => break format!("{low}\t{budget}"),
                GuestFuelEmission::FuelExhausted => low = budget,
                GuestFuelEmission::Refusal(identity) => break format!("refused:{identity}\t-"),
                GuestFuelEmission::Trap { reason } => break format!("trap:{reason}\t-"),
            }
            if budget >= ceiling {
                break "exhausted\t-".to_owned();
            }
            budget = budget.saturating_mul(2).min(ceiling);
        };
        println!("EMITTER_SCALING read_locals={count} fuel={verdict}");
    }
}

'''

anchor = '''#[test]
#[ignore = "a real-closure component-emission fuel probe; run it explicitly"]
fn the_component_emission_fuel_of_a_real_match_group_is_measured() {'''

if anchor not in src:
    print("ANCHOR_NOT_FOUND")
elif "the_emitter_fuel_scaling_in_read_local_count_is_measured" in src:
    print("ALREADY_PRESENT")
else:
    src = src.replace(anchor, probe + anchor, 1)
    open(p, "w", encoding="utf-8").write(src)
    print("INSERTED")
