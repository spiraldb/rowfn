# Agent instructions

- Do not use em dashes or add an agent coauthor.
- All commits must include the committer's sign-off.
- Preserve user changes and historical measurement artifacts.
- Keep prose short and concrete. Use diagrams for component and execution relationships.
- Keep RowFn's strict null, callback, ownership, and initialization contracts intact.
- `rowfn`, `rowfn-kernels`, and `rowfn-arrow` must not depend on Vortex.
- The root workspace contains the independent crates and Arrow fixtures. Vortex files under
  `integrations/vortex` are a reconstruction snapshot, not root workspace packages.
- Tests, builds, formatting, linting, and benchmarks are opt-in. Run only checks that the user requests.
- Report source review separately from executed verification. Historical timings describe their
  recorded source, not later changes.
- Follow `STYLE.md` and applicable instructions before modifying code or documentation.
