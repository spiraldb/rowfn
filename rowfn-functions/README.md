<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: Copyright the Vortex contributors -->

# rowfn-functions

[Overview](../README.md) · [Author guide](../rowfn/AUTHORING.md)

Portable function definitions for RowFn. Each function keeps its semantic type dispatch and row
operation together. Backend mappings interpret native metadata. Callers can invoke the same
definitions directly on Arrow or through the DataFusion integration.

## Examples

| Definition | Demonstrates |
| --- | --- |
| `Add`, `Multiply` | Integer dispatch, constants, and deferred overflow errors. |
| `Divide` | Immediate errors and scalar output initialization. |
| `PositiveSum` | Boolean packing and nullable retry. |
| `Trim`, `Concat` | Borrowed strings and owned output. |
| `Scale` | List width, prepared constants, and row initialization. |
| `AdjustTicks` | A custom domain with preserved units and timezone. |
| `Seven` | Nullary execution with an explicit row count. |
| Comparisons, patterns, substring, and lengths | Storage-aware values and preparation. |

The default package has no backend dependency. The `arrow` feature enables mappings for the
included backend:

```sh
cargo run --locked -p rowfn-functions --features arrow --example arrow
```

The [DataFusion example](../rowfn-datafusion/examples/sql.rs) registers these definitions as scalar
UDFs through `rowfn-arrow`. It uses the same Arrow mappings. An earlier Vortex mapping and manifest
remain in the [integration snapshot](../integrations/vortex/README.md).

## Extend or compare

The package owns its domain traits and `TimestampTicks` row kind, so mappings can satisfy Rust's
orphan rules. Functions are not registered automatically. Registration belongs to the backend.

`Divide` and `Scale` use unsafe initialization tokens. The [author guide](../rowfn/AUTHORING.md)
also shows `ElementSink`, which supplies safe initialized rows. [Examples and benchmarks](../rowfn-examples/README.md)
exercise the included backend.

See [status and verification](../STATUS.md) for publication and verification status.
