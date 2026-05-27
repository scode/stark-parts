# Ambiguity Log

NOTE: This file records decisions that were reasonable to make without blocking forward progress. It is not a replacement for `SPEC.md` or `IMPL_SPEC.md`; when the contract is clear enough to specify, update those files instead.

## 2026-05-27: Plan Granularity

Decision: split the implementation into eight stacked PRs: scaffold, schema/JSON5, fixture-backed crawler core, real HTTP CLI commands, real catalog build and crawler hardening, search/tree model, Leptos UI, and static build verification.

Why: this keeps the stack bottom-up. The data format and crawler can get useful tests before network code and UI are layered on top, and the browser behavior can be tested against a stable in-memory model before the full catalog is committed.

Other reasonable options: split the web app into more PRs, split the CLI command work from the real HTTP client, combine the first real catalog build with the crawler command PR, or postpone real catalog generation until final end-to-end verification. Those are all plausible, but this plan keeps each PR reviewable while making sure real Stark crawl problems are found before the GUI is built on top of the data.

Caveat: if the first real Stark crawl exposes enough API differences across bike variants, the crawler work may need an extra hardening PR before the search and UI layers build on the committed catalog.

## 2026-05-27: Search Index Timing

Decision: plan for the search data model to be implemented after the committed catalog schema, crawler transformation, and first real committed catalog build, but before the Leptos UI.

Why: the search behavior is central user-visible behavior and should have focused tests without UI complexity. The UI can then compose a tested model instead of embedding filtering logic in components.

Other reasonable options: precompute the full search index during catalog generation, derive it at app initialization, or use a hybrid. `IMPL_SPEC.md` allows either. The plan does not force that choice yet because the right answer depends on catalog size and Leptos build ergonomics.

Caveat: if the full committed catalog is large enough to make app initialization slow, this decision should be revisited before the UI PR.

## 2026-05-27: Live Network Tests

Decision: do not make live Stark API calls part of the normal test suite. Use trait-backed fixtures for automated tests and reserve live network access for explicit catalog init/update runs.

Why: the specs require static runtime behavior, deterministic output, and mockable upstream fetches. Live tests would be slower, less deterministic, and more likely to fail for reasons unrelated to the code under review.

Other reasonable options: add an ignored/manual live smoke test or a separate CI job gated behind credentials or an explicit flag.

Caveat: fixture-only coverage can drift away from Stark's current API. The catalog refresh PR should include enough sanity checking to catch drift when the real crawler is run.

## 2026-05-27: Plan Status Timing

Decision: mark a plan step `[x]` after the step's work, coverage, checks, `$pre-pr-review-swarm`, and unambiguous review fixes are complete, but before committing and creating that step's PR.

Why: the status update needs to be part of the step PR itself. If `[x]` meant "the GitHub PR already exists", every step would need an extra follow-up PR update just to mark the completed step.

Other reasonable options: mark `[x]` only after the PR exists, or add a second status marker for "ready for PR" versus "PR created". Those are more literal, but they make the status tracker awkward in a stacked-PR workflow.

Caveat: `[x]` means the local step is complete and ready to be submitted as its own PR. It does not mean that PR has merged.

## 2026-05-27: Stark Variant Discovery

Decision: keep `varg-ex` as the only concrete variant tag documented from the crawl report, but require the crawler work to discover the rest of Stark's public bike variant tags before the first real committed catalog build.

Why: the behavior spec now requires all Stark bike variants with public parts catalog data. The existing research only proves the VARG EX path, so pretending the variant set is already known would make the plan overconfident.

Other reasonable options: block until the full variant-tag list is researched now, or constrain the first implementation to `varg-ex` only. The user chose all bikes, and discovery can be implemented and tested as part of the crawler layer without blocking this planning PR.

Caveat: if Stark does not expose a clean variant listing surface, the crawler may need a manually maintained variant list. That should be recorded in `IMPL_SPEC.md` when discovered.
