<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: Copyright the Vortex contributors -->

# Status and verification

[Overview](README.md) · [Safety contracts](rowfn/SAFETY.md) · [Measurements](BENCHMARKS.md)

The API is experimental. All workspace crates are unpublished to crates.io.

## Implemented

| Package | Responsibility |
| --- | --- |
| `rowfn` | Strict row contracts, typed visitors, planning, execution, preparation, and sinks. |
| `rowfn-kernels` | Lane traversal, borrowed bitmaps, and Boolean packing. |
| `rowfn-arrow` | Arrow fields, decoding, validity, allocation, and publication. |
| `rowfn-datafusion` | Field-aware scalar UDF registration over the Arrow backend. |
| `rowfn-functions` | Portable definitions and optional Arrow domain mappings. |
| `rowfn-examples` | Arrow fixtures and comparison benchmarks. |

The Vortex adapter is a [reconstruction snapshot](integrations/vortex/README.md), outside the active
workspace. DuckDB and other native backends remain possible integrations.

## Verification

The standalone workspace and DataFusion integration have source review only. Offline Cargo
dependency resolution selected DataFusion 55.1.0 with Arrow 59.3.0 throughout the dependency graph.
The framework, kernels, and Arrow backend have no DataFusion or Vortex dependency.

No builds, tests, examples, formatting, linting, or benchmarks ran for the standalone export or
DataFusion integration. Commands in the guides are references for an explicitly requested run.
The [safety guide](rowfn/SAFETY.md) records the reviewed invariants, not a memory-safety certification.

## Historical evidence

[BENCHMARKS.md](BENCHMARKS.md) links earlier timings, test reports, compiler metadata, and captured
source. Those records predate the package split, concrete text dispatch, rename, standalone export,
and DataFusion integration. Their results do not verify the current workspace.

The [research archive](research/row-fn-engine/README.md) retains earlier designs and investigations.
Recorded measurements and captured source remain unchanged. A new performance claim needs a matched
baseline, the exact source state, and an authorized verification run.
