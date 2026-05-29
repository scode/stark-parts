# Ambiguity Log

NOTE: This file records decisions that were reasonable to make without blocking forward progress. It is not a
replacement for `SPEC.md` or `IMPL_SPEC.md`; when the contract is clear enough to specify, update those files instead.

## 2026-05-27: Plan Granularity

Decision: split the implementation into eight stacked PRs: scaffold, schema/JSON5, fixture-backed crawler core, real
HTTP CLI commands, real catalog build and crawler hardening, search/tree model, Leptos UI, and static build
verification.

Why: this keeps the stack bottom-up. The data format and crawler can get useful tests before network code and UI are
layered on top, and the browser behavior can be tested against a stable in-memory model before the full catalog is
committed.

Other reasonable options: split the web app into more PRs, split the CLI command work from the real HTTP client, combine
the first real catalog build with the crawler command PR, or postpone real catalog generation until final end-to-end
verification. Those are all plausible, but this plan keeps each PR reviewable while making sure real Stark crawl
problems are found before the GUI is built on top of the data.

Caveat: if the first real Stark crawl exposes enough API differences across bike variants, the crawler work may need an
extra hardening PR before the search and UI layers build on the committed catalog.

## 2026-05-27: Search Index Timing

Decision: plan for the search data model to be implemented after the committed catalog schema, crawler transformation,
and first real committed catalog build, but before the Leptos UI.

Why: the search behavior is central user-visible behavior and should have focused tests without UI complexity. The UI
can then compose a tested model instead of embedding filtering logic in components.

Other reasonable options: precompute the full search index during catalog generation, derive it at app initialization,
or use a hybrid. `IMPL_SPEC.md` allows either. The plan does not force that choice yet because the right answer depends
on catalog size and Leptos build ergonomics.

Caveat: if the full committed catalog is large enough to make app initialization slow, this decision should be revisited
before the UI PR.

## 2026-05-27: Live Network Tests

Decision: do not make live Stark API calls part of the normal test suite. Use trait-backed fixtures for automated tests
and reserve live network access for explicit catalog init/update runs.

Why: the specs require static runtime behavior, deterministic output, and mockable upstream fetches. Live tests would be
slower, less deterministic, and more likely to fail for reasons unrelated to the code under review.

Other reasonable options: add an ignored/manual live smoke test or a separate CI job gated behind credentials or an
explicit flag.

Caveat: fixture-only coverage can drift away from Stark's current API. The catalog refresh PR should include enough
sanity checking to catch drift when the real crawler is run.

## 2026-05-27: Plan Status Timing

Decision: mark a plan step `[x]` after the step's work, coverage, checks, `$pre-pr-review-swarm`, and unambiguous review
fixes are complete, but before committing and creating that step's PR.

Why: the status update needs to be part of the step PR itself. If `[x]` meant "the GitHub PR already exists", every step
would need an extra follow-up PR update just to mark the completed step.

Other reasonable options: mark `[x]` only after the PR exists, or add a second status marker for "ready for PR" versus
"PR created". Those are more literal, but they make the status tracker awkward in a stacked-PR workflow.

Caveat: `[x]` means the local step is complete and ready to be submitted as its own PR. It does not mean that PR has
merged.

## 2026-05-27: Stark Variant Discovery

Decision: keep `varg-ex` as the only concrete variant tag documented from the crawl report, but require the crawler work
to discover the rest of Stark's public bike variant tags before the first real committed catalog build.

Why: the behavior spec now requires all Stark bike variants with public parts catalog data. The existing research only
proves the VARG EX path, so pretending the variant set is already known would make the plan overconfident.

Other reasonable options: block until the full variant-tag list is researched now, or constrain the first implementation
to `varg-ex` only. The user chose all bikes, and discovery can be implemented and tested as part of the crawler layer
without blocking this planning PR.

Caveat: if Stark does not expose a clean variant listing surface, the crawler may need a manually maintained variant
list. That should be recorded in `IMPL_SPEC.md` when discovered.

## 2026-05-27: CI Action Pinning

Decision: keep GitHub Actions referenced by their standard version tags in the initial scaffold instead of pinning every
action to a commit SHA.

Why: the scaffold is intentionally following the requested Rust modernization baseline, which calls out the current
standard actions and `dprint/check` action shape. Tag refs are also easier to keep aligned with that baseline while the
project is still in early setup.

Other reasonable options: pin every action to a full SHA now, or use organization-level policy/tooling to enforce action
pinning later.

Caveat: mutable action tags are a supply-chain risk. If this repository starts handling secrets, release signing, or
privileged deployment in CI, revisit this and likely pin action SHAs.

## 2026-05-27: Leptos SSR Feature in Scaffold Tests

Decision: enable Leptos `ssr` alongside `csr` in the initial workspace dependency so the scaffold can assert rendered
HTML in unit tests.

Why: the first scaffold only has a minimal Leptos app shell, so construction-only tests would be too weak. Rendering the
component to HTML catches removal of the unofficial notice and search control. In Leptos 0.8, that render path requires
the `ssr` feature.

Other reasonable options: drop the render assertion and keep only `csr`, or move HTML rendering coverage to a later
browser-test PR.

Caveat: this does not mean the runtime app should depend on server rendering. The static runtime behavior remains
controlled by `SPEC.md`; this is a testability choice for the scaffold.

## 2026-05-28: Localization Extraction Timing

Decision: the first committed live catalog keeps Stark localization keys and stable codes, but does not block step 5 on
extracting English display strings from Stark's Next.js page payload.

Why: the live catalog build exposed API-shape and transport problems that needed to be fixed before later search and UI
work could build against real data. The generated catalog still has stable searchable identifiers and localization keys,
and `SPEC.md` already requires the UI to fall back to codes or localization keys when display strings are missing.
Adding page-payload localization extraction is still required by `IMPL_SPEC.md`, but it is a separate implementation
problem from proving the real catalog crawl works end to end.

Other reasonable options: implement localization extraction in this PR, relax `IMPL_SPEC.md`, or add a separate plan
step before the search-index work. Keeping the requirement in `IMPL_SPEC.md` while recording the timing decision makes
the remaining work visible without turning the live catalog build PR into a broader parser change.

Caveat: search and UI work should revisit this before relying on localization-key fallbacks as the final user-facing
experience.
