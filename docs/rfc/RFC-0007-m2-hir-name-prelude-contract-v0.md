# RFC-0007: M2 HIR, name and P0 prelude contract v0

> - status: accepted
> - created: 2026-07-15
> - accepted: 2026-07-15
> - owners: autonomous-agent
> - related-step: STEP-0022

## Summary

M2 lowering uses deterministic preorder HIR IDs and UTF-8 byte source maps. Every non-trivia B token is represented in a typed declaration/body line and balanced token tree before semantic interpretation. Any lexical/parser error blocks HIR production.

The minimum semantic environment is [`prelude-v0.json`](../../semantics/prelude-v0.json). It freezes only the names and arities required to execute the 54 P0 cases; it is not a public standard-library compatibility promise.

## Decisions

1. HIR identity is module-local preorder `u32`; source range is authoritative UTF-8 half-open `TextRange`.
2. HIR retains declaration, line/block kind and balanced token trees, but does not choose operator/type semantics before their M2 STEP.
3. Name lookup order is lexical local → parameter → module declaration → prelude; heuristic/string-similarity binding is forbidden.
4. Type and value symbols are kind-tagged. Whether equal spelling may coexist across public namespaces remains undecided because the corpus has no discriminating case; implementing collision behavior requires a new case/RFC.
5. `Float64.from_int`, `collect_tasks` and `input` are accepted only as P0 fixture environment members. Their public SDK names remain unstable.
6. `Bytes`, currency/unit names and revision/domain types declared by source are not silently replaced by prelude aliases.

## Rejected alternatives

- Reusing rowan node address as identity: not stable across parse/format runs.
- Lowering recovered trees: would allow missing/error syntax into type checking.
- Assigning expression precedence in STEP-0022: current M2 substeps own the relevant semantic evidence.
- Treating every capitalized identifier or call as a prelude member: hides unresolved-name bugs.

## Acceptance

- 54/54 B sources lower deterministically with contiguous IDs and valid ranges;
- every HIR semantic token text equals its source range;
- 54 structural snapshots are stable;
- 12/12 mutation sources cannot lower;
- prelude manifest schema/counts and unresolved decisions are validated offline.
