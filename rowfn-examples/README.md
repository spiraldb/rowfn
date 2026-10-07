<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: Copyright the Vortex contributors -->

# rowfn-examples

[Overview](../README.md) · [Comparison guide](../rowfn/COMPARING.md)

Arrow fixtures and benchmarks for the shared function definitions. The
[DataFusion integration](../rowfn-datafusion/README.md#benchmark-boundaries) has separate fixtures
for SQL execution and UDF wrapper costs.

| Harness | Compares |
| --- | --- |
| `boundaries` | Direct loops, shared collection, and complete invocation. |
| `arrow_workloads` | Arithmetic, strings, dictionaries, lists, timestamps, and degenerate batches. |
| `arrow_scalar` | Multiplication and string predicates. |
| `arrow_families` | Arithmetic, comparisons, patterns, substrings, and lengths. |

When run, fixtures compare results before timing. Some references compose native kernels or use
different output layouts. The [comparison guide](../rowfn/COMPARING.md) explains those limits.

[Recorded measurements](../BENCHMARKS.md) describe earlier source. See
[status and verification](../STATUS.md) for the current evidence boundary. Full cross-host fixtures
remain in the [Vortex snapshot](../integrations/vortex/README.md).
