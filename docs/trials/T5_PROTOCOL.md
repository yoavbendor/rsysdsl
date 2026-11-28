# Trial 5 protocol — real-world corpus → model, end to end (pre-registered)

Stage T5 of [`../SPRINT_CONSOLIDATE_PLAN.md`](../SPRINT_CONSOLIDATE_PLAN.md). Trial agents must not read this file or anything else under `docs/trials/`.

## Question
Can a Haiku-class agent go from a raw, unfamiliar multi-document corpus to a traceable, auditable `rsysdsl` model — through docmem indexing, `reconcile`, authoring, `audit`/`lint`/`tbd`, `budget`/`refute`, and an HTML review — **without anyone hand-writing entity names or aliases for it**?

## Corpus (owner's choice: PX4)
The 16-document PX4 + Pixhawk 6X (FMUv6X) corpus from `scripts/fetch-px4-corpus.sh`, **pinned**:
- `PX4/PX4-user_guide` @ `2fa62338099f18fc3092197e856615eca94d2975`
- `pixhawk/Pixhawk-Standards` @ `a97a915bfb6e20d6c8a41a86c5673350a3e51ceb`

Fetched over `raw.githubusercontent.com` at those SHAs (the `gh api` route is not available in every environment), with the existing SHA-256 manifest. PDF text is extracted with pypdf **layout mode** (changed during preparation: the T5.K key showed plain mode drops table cells, e.g. DS-010 p.6 `CAN 2 2 1 3` → `CAN`; layout mode keeps the row). Known at the pinned SHA: `obstacle_avoidance.md` is a 61-byte redirect stub (`<Redirect to="../computer_vision/path_planning_interface" />`), so the corpus has **15 documents with content**; nothing is expected from that file. It contains real PDF standards with tables (DS-012 FMUv6X, DS-010 Autopilot Bus) and Markdown docs whose diagrams are images.

## Contamination controls
This corpus was modelled by hand before (`BROWNFIELD_AB_RESULTS.md`). Trial agents must NOT read:
- `crates/sysml-emit/examples/px4_*.rs`, `crates/sysml-emit/tests/px4_*`
- `tools/corpus_kg/px4_docmem_kg.py` (contains the hand-written `CANONICAL_ALIASES`)
- `docs/BROWNFIELD_AB_RESULTS.md`, `docs/HAIKU_TRIAL.md`, `docs/DEVELOPMENT_PLAN.md`, `docs/SPRINT_CONSOLIDATE_PLAN.md`, `docs/trials/`
- other trial directories.
They may read `AGENTS.md`, `.agents/skills/`, `crates/` (except the files above), `examples/`, the `cppognee` docmem tool, and the corpus. All files they create stay inside their own run directory.

## Prerequisites (do not run before)
- C2 (IR expression evaluator) and C3 (`refute`/`trade-study` over real constraints) merged — otherwise `refute` has nothing general to check.
- C4 (SW/HW concepts in the macro path) merged — the corpus is mostly SW/HW architecture.
- Fetch script fixed to the pinned raw-URL route (T5.0).
- Answer key written (T5.K) and committed before the first run.

## Preparation tasks
- **T5.0 (Haiku):** make `scripts/fetch-px4-corpus.sh` fetch via `raw.githubusercontent.com` at the two pinned SHAs (keep `gh api` as an optional path), verify all 16 files download and the manifest is produced.
- **T5.K (Opus, separate context, before any run):** read the pinned corpus and write `docs/trials/T5_KEY.md`: compute units, buses and interfaces (incl. facts only present in PDF tables, with page numbers); numeric values with their exact source location; "shall"/requirement statements; any inconsistencies or conflicts between documents (or an explicit "none found" with what was checked); information carried only by images/diagrams (so it can't be expected from text extraction). The key builder must not read the contaminating files above either.

## Run
3 Haiku runs, each in its own directory, canonical trial rules from `HAIKU_TRIAL.md`. Steps: fetch → docmem scan (no hand-written alias table; aliases must come from the corpus or the tool) → `reconcile` → author the model with `rsysdsl new` + macros → `audit --stdlib --strict`, `lint --strict`, `tbd` → `budget`/`refute` where the corpus gives limits → `view` HTML. Soft cap: 40 invocations (bigger task than T1–T4). Log every invocation.

## Grading (separate Opus grader, from source, against T5_KEY.md)
1. **Coverage:** recall of key compute units, buses, interfaces; recall of PDF-table-only facts (measures the table-extraction problem directly).
2. **Provenance:** every number in the model traces to a corpus location (file + section/page); leakage = numbers with no source.
3. **Honesty:** gaps and inter-document conflicts surfaced as TBDs with true reasons; misuse as in T3 (TBD on a given value, TBD hiding an error).
4. **Aliases:** canonical names chosen without a hand-written table; count of duplicated entities (same thing modelled twice under two names).
5. **Friction:** failing invocations by pipeline stage (fetch, docmem, reconcile, authoring, audit, operators, view) — the main output is the list of sharp edges to fix.
6. **Operators:** whether `budget`/`refute` produced a meaningful result or an honest `not_applicable`/`incomplete`.
