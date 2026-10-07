<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- SPDX-FileCopyrightText: Copyright the Vortex contributors -->

# RowFn research archive

[Project overview](../../README.md) · [Current status](../../STATUS.md) · [Measurements](../../BENCHMARKS.md)

This tree preserves the research behind the RowFn extraction from Vortex. Reports describe the
source revisions they inspected or measured. API names, proposed integrations, and references to
current behavior belong to those revisions. Captured source and measurement files retain the
historical `rofl` name.

For the active workspace, use the [architecture](../../rowfn/ARCHITECTURE.md),
[author guide](../../rowfn/AUTHORING.md), and [backend guide](../../rowfn/BACKENDS.md).
The [DataFusion integration](../../rowfn-datafusion/README.md) now implements the Rust UDF wrapper
proposed in the earlier research.

The original Vortex work is tracked in the [RowFn epic](https://github.com/vortex-data/vortex/issues/9128)
and [API design issue](https://github.com/vortex-data/vortex/issues/9129).

## Measurements and implementation records

| Record | Contains |
| --- | --- |
| [Compute comparison](compute-options.md) | Recorded Arrow and Vortex comparisons, including regressions and baseline limits. |
| [Arrow investigation](arrow-performance-opportunities.md) | Native-kernel improvement candidates suggested by those measurements. |
| [Implementation record](implementation.md) | The earlier Vortex-based extraction and its verification history. |
| [Initial measurements](measurements/2026-09-25-rofl/README.md) | Timings, compiler output, tests, and source provenance. |
| [Scalar families](measurements/2026-09-29-families/final/README.md) | Additional arithmetic, text, and comparison fixtures. |
| [Earlier standalone comparison](rofl-repository-comparison.md) | A comparison with the earlier `rofl` repository. |
| [Evidence record](evidence.md) | Source findings and measurement qualifications. |

## Design investigations

| Topic | Read |
| --- | --- |
| Extraction and ownership | [Architecture](architecture.md), [dependency boundary](current-system/engine-boundary.md). |
| Type bindings | [Type analysis](type-system/README.md), [portable contract](type-system/portable-contract.md). |
| Host integration | [Arrow, DataFusion, and DuckDB](integrations/README.md), [storage and ownership](integrations/storage-and-ownership.md). |
| Row demand | [Demand and completion](definedness/README.md), [worked cases](definedness/worked-cases.md). |
| Compiler and runtime costs | [Performance investigation](performance/README.md), [optimization candidates](performance/optimization-candidates.md). |
| Earlier Vortex source | [Source inventory](current-system/README.md), [design history](current-system/design-history.md). |
| Other frameworks | [Prior art](prior-art.md), including Velox, Arrow, DuckDB, and DataFusion. |
| Further work | [Next steps](next-steps.md). |

Historical test reports and timings do not verify the current standalone packages. Preserve their
source snapshots and metadata when comparing a later change.
