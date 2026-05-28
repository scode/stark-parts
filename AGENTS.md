# Repository Instructions

## Specs

`SPEC.md` is the source of truth for user-observable behavior. All code, docs, tests, and generated artifacts must
adhere to it unless the intended change is to alter that behavior.

When intended behavior changes, update `SPEC.md` in the same change. Keep it strictly limited to user-observable
behavior: what a person can see, do, rely on, or be told by the site or command line tools. Do not put implementation
choices, library decisions, internal schema details, test strategy, or private architecture notes in `SPEC.md`.

`IMPL_SPEC.md` is the source of truth for implementation choices and implementation constraints. All implementation work
must adhere to it unless the intended change is to alter those choices or constraints.

When intended implementation choices change, update `IMPL_SPEC.md` in the same change. Keep it about implementation
details: architecture, tool choices, data format mechanics, API boundary decisions, testability constraints, and
internal contracts. Do not use it to specify user-visible behavior except when restating the minimum needed context to
explain an implementation constraint.

## Implementation Plan

`PLAN.md` is the source of truth for implementation order and status. Work through it from top to bottom. Each step is
intended to become its own reviewable `$jjstack` PR, stacked on the PR for the previous step.

When a plan step has passed its required gates and is ready to become its PR, update that step from `[ ]` to `[x]` in
that same reviewable change. Do not mark a step complete before its required coverage, checks, `$pre-pr-review-swarm`,
and fixes are done.

If the intended implementation order changes, update `PLAN.md` in the same change that makes the plan true again.

## Ambiguity Log

Use `AMBIGUITY.md` when a decision is unclear but forward progress is still reasonable. Record what was decided, why,
what reasonable alternatives existed, and any caveats that should be revisited later.

Do not use `AMBIGUITY.md` to avoid updating `SPEC.md` or `IMPL_SPEC.md`. If the user-visible behavior or implementation
contract is clear enough to specify, update the relevant spec. Stop and ask for help only when the decision is
existential enough that choosing either direction would likely waste the stack.

## Finish-Work Checks

Before claiming work is done or opening a PR, run the same commands CI runs:

```sh
cargo fmt --all -- --check
cargo clippy
cargo test
dprint check
```

If a command cannot be run locally, report why instead of omitting it.

## Conventional Commits

All commit messages and PR titles must use Conventional Commit format: `<type>: <short summary>`

Allowed types: `feat`, `fix`, `docs`, `perf`, `refactor`, `style`, `test`, `chore`, `ci`, `revert`.

Append `!` after the type for breaking changes, for example `feat!: remove legacy endpoint`. Scope is optional.

Rules:

- Type reflects the user-visible effect, not the implementation activity. A bug fix that requires heavy refactoring is
  `fix`, not `refactor`. A new CLI flag is `feat`, not `chore`.
- The summary after the colon is lowercase, imperative mood, no trailing period.
- Keep the first line under 72 characters.
