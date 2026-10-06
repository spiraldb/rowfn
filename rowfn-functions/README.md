<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: Copyright the Vortex contributors -->

# rowfn-functions

[Overview](../README.md) · [Author guide](../rowfn/AUTHORING.md)

Backend-independent function definitions. Each function keeps its type dispatch and row operation
together. Backend mappings interpret native metadata without duplicating that function logic.

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

An earlier Vortex mapping and manifest are preserved in the
[integration snapshot](../integrations/vortex/README.md). Other backend mappings have yet to be added.

## Extend or compare

The package owns its domain traits and `TimestampTicks` row kind, so mappings can satisfy Rust's
orphan rules. Functions are not registered automatically. Registration belongs to the backend.

`Divide` and `Scale` use unsafe initialization tokens. The [author guide](../rowfn/AUTHORING.md)
also shows `ElementSink`, which supplies safe initialized rows. [Examples and benchmarks](../rowfn-examples/README.md)
exercise the included backend.

The package is unpublished. See [project status](../README.md#status) for the evidence boundary.
