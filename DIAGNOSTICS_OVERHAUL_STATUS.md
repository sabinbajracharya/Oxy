# Diagnostics Overhaul Status

Purpose: track the architecture-first diagnostics refactor so work can be resumed by another agent without guesswork.

## Scope

- Build a first-class diagnostics model (codes, labels, notes/help, fix-it suggestions).
- Keep `PipelineError` backward-compatible while adding structured diagnostics plumbing.
- Route CLI and LSP through the same structured diagnostics data.
- Improve snippet/caret rendering for spans and secondary labels.

## Step Plan

- [x] Step 1: Create this tracker and define phased architecture plan.
- [ ] Step 2: Add `oxy-core` diagnostics module (`Diagnostic`, labels, notes/help, fix-its, codes).
- [ ] Step 3: Wire `PipelineError` ↔ structured diagnostics conversion.
- [ ] Step 4: Switch CLI rendering to structured diagnostics (primary + secondary labels, notes/help, fix-its).
- [ ] Step 5: Switch LSP diagnostic mapping to structured diagnostics.
- [ ] Step 6: Add/adjust regression tests for diagnostics conversion and LSP mapping.
- [ ] Step 7: Run validation and mark completion.

## Progress Log

- ✅ Initialized tracking file and agreed phased implementation.
