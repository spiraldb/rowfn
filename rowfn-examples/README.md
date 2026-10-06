<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: Copyright the Vortex contributors -->

# rowfn-examples

Arrow-only fixtures and comparison benchmarks for the shared function definitions. This workspace
has no Vortex dependency. Full cross-host fixtures remain in
[integrations/vortex](../integrations/vortex/README.md).

| Harness | Compares |
| --- | --- |
| `boundaries` | Direct collection, shared collection, and complete Arrow invocation. |
| `arrow_workloads` | Arithmetic, strings, dictionaries, lists, timestamps, and degenerate batches. |
| `arrow_scalar` | Multiplication and literal string predicates. |
| `arrow_families` | Arithmetic, bitwise operations, comparisons, patterns, substrings, and lengths. |

Fixture checks run before timing when a benchmark is executed. Some references compose native
kernels or use different output layouts. See the [comparison guide](../rowfn/COMPARING.md) for
those qualifications and focused commands.

The export retains Arrow benchmark operations and replaces Vortex-only fixture panic helpers with
standard `expect`. It has not been built, tested, or benchmarked. Historical reports describe their
recorded source and compiler settings, not this exported workspace.
