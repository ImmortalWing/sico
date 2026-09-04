# STEP-0124: Application-ready language and AI application platform roadmap

> - status: complete-planning
> - phase: M14–M18 roadmap
> - started: 2026-09-04
> - completed: 2026-09-04
> - owners: repository-owner, autonomous-agent

## 1. Objective

Turn the owner decision that visual/native automation is a representative AI workload into a dependency-correct roadmap without widening M12 or prematurely implementing platform APIs.

## 2. Context and evidence

- M12 is a secure HTTP/API provider track, not a GUI automation authority.
- Current Sico source checking covers more constructs than the executable application profile.
- M5 UI remains a companion model without compiler-facing Sico binding.
- The block-game case needs ordinary search code, capture, input and vision as four distinct layers.
- Former M14 Web/UI had direction approval but no reserved STEP numbers, so it can be renumbered without invalidating implementation history.

## 3. Scope

This step changes planning and documentation only. It defines M14 application readiness, moves Web/UI to M15, adds M16 Native Automation Host, M17 vision/model packages and M18 representative applications/pilots. It does not add language syntax, Host authority, native adapters, model dependencies or support claims.

## 4. Options and decision

- **Add GUI/vision to M12:** rejected; endpoint/secret HTTP policy and native observation/input have different threat and capability models.
- **Wait for an undefined “fully complete language”:** rejected; the gate would be unmeasurable and applications would provide feedback too late.
- **Chosen:** freeze a measurable application-ready language baseline, then split Web/UI and native automation into platform tracks, place vision/model work in the package/provider ecosystem, and validate all layers through representative applications.

## 5. Plan

1. Finish M12 and M13 under their existing contracts.
2. Execute M14 only after a machine support matrix and acceptance corpus are frozen.
3. Allow M15 and M16 to proceed independently after M14 GO and shared Host contracts.
4. Start M17 only after image ownership and package/provider trust boundaries exist.
5. Use M18 for real applications and evidence classes; do not let infrastructure self-certify product readiness.

No implementation STEP numbers are reserved. Each future milestone begins with RFC/ADR/threat/corpus work.

## 6. Changes

- `docs/ROADMAP.md`: M14–M18 definitions and dependency graph.
- `docs/plans/M14-application-ready-language.md` through `M18-ai-application-pilots.md`: entry gates, workstreams, non-goals and exit gates.
- `docs/plans/README.md`, `docs/README.md`, `docs/STATUS.md`: indexes and current decision.
- `DIRECTION.md`, `DEVELOPMENT.md`, `AGENT_GOAL.md`: product direction and autonomous-agent constraints.
- block-game case README: milestone acceptance path.

## 7. Validation

`tools/validate-step-0124.ps1` verifies all five plan files, exact roadmap ordering, dependency/gate phrases, plan/index links, owner-decision text, the block-game acceptance chain and absence of reserved M14–M18 STEP ranges.

## 8. Metrics

- 5 future milestones with separate plans.
- 0 implementation STEP numbers reserved.
- 4 distinct layers for the block-game case: Sico solver, native capture/input, vision package, application pilot.
- No runtime, compiler, provider or capability implementation changed.

## 9. Risks and follow-ups

- M14 scope must remain a bounded application profile; it cannot become an endless “finish every language feature” phase.
- M16 authority is high-risk and must not begin with a permissive desktop API.
- M17 native/model providers can reintroduce ambient authority unless Component and package gates remain mandatory.
- M18 external evidence still depends on real users, devices, accounts and legal authority.

## 10. Audit links

- [`ROADMAP`](../ROADMAP.md)
- [`M14 plan`](../plans/M14-application-ready-language.md)
- [`M15 plan`](../plans/M15-web-ui-platform.md)
- [`M16 plan`](../plans/M16-native-automation-host.md)
- [`M17 plan`](../plans/M17-vision-ml-ecosystem.md)
- [`M18 plan`](../plans/M18-ai-application-pilots.md)
