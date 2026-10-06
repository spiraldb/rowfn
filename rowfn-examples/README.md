<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: Copyright the Vortex contributors -->

# rowfn-examples

[Overview](../README.md) · [Comparison guide](../rowfn/COMPARING.md)

Examples and benchmarks for the shared function definitions. The current harness uses Arrow because
it is the first included backend. Future backend harnesses can use the same functions and fixtures.

| Harness | Compares |
| --- | --- |
| `boundaries` | Direct loops, shared collection, and complete invocation. |
| `arrow_workloads` | Arithmetic, strings, dictionaries, lists, timestamps, and degenerate batches. |
| `arrow_scalar` | Multiplication and string predicates. |
| `arrow_families` | Arithmetic, comparisons, patterns, substrings, and lengths. |

When run, fixtures compare results before timing. Some references compose native kernels or use
different output layouts. The [comparison guide](../rowfn/COMPARING.md) explains those limits.

The standalone export has not been built, tested, or benchmarked. [Recorded measurements](../BENCHMARKS.md)
describe earlier source. Full cross-host fixtures are preserved with the
[Vortex prototype](../integrations/vortex/README.md).
