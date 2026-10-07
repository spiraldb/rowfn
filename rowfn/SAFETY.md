<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: Copyright the Vortex contributors -->

# Safety contracts

[Overview](../README.md) · [Adapter guide](ADAPTERS.md) · [Status and verification](../STATUS.md)

The executor relies on backend bindings to provide valid borrowed views, stable output slots, and
safe publication. This page maps those obligations to their enforcement. Source review does not
certify memory safety. Executed verification and historical evidence are listed in [STATUS.md](../STATUS.md).

| Boundary | Contract and enforcement |
| --- | --- |
| [Borrowed input](src/input.rs) | `RowView` is unsafe to implement. Decoded owners stay live through preparation and traversal. Sources validate lengths before unchecked access. Every in-range index must return a valid value, including null payloads after successful decoding. |
| [Concrete text layouts](../rowfn-arrow/src/input.rs) | Offset views borrow matching offsets and bytes. Their offset slice has one sentinel beyond the row domain. View rows borrow matching headers and backing buffers. Slices retain their original offsets and owners. |
| [Scalar input](src/tuple.rs) | A scalar decode retains exactly one addressable row. Source construction checks that count. Scalar-only execution can compute one valid result before broadcasting to the logical row count. |
| [Selection](src/host.rs) | The unsafe trait requires stable bounds, order, uniqueness, count, and immediate error propagation. Executors check input and output domains before selected or filtered traversal. |
| [Owned output](src/output.rs) | `OutputBuffer` preserves stable slots and permits safe partial abandonment. Owned traversal excludes values that require destruction. Publication reads only the initialized prefix. |
| [Initialized scalar output](src/sink/initialized.rs) | `ElementSink` writes defaults before lending mutable values. The initialized prefix covers every exposed slot, and safe mutation cannot remove initialization. |
| [Uninitialized output](src/sink/element.rs) | A token must prove initialization of the exact complete row supplied to the callback. Its unsafe constructor requires that initialization to remain intact until return. |
| [Fixed-size lists](src/sink/list.rs) | Allocation checks row-count multiplication. The explicit row count survives width zero. Each nonempty row lends its own bounded child range. Initialization evidence covers that entire range. |
| [Strings](../rowfn-arrow/src/string.rs) | Writers own output bytes, check representation limits, and retain initialized empty placeholders. Unchecked Arrow construction follows those checks. |
| [Boolean packing](src/output.rs) | The unsafe host contract bounds callback indices and order. No callback runs for zero rows. Shared kernels support sliced bitmaps without assuming padding. |
| [Deferred errors](src/execute/mod.rs) | Only rejected row evidence permits null-based suppression or retry. Decoder, allocation, validation, and publication failures return directly. |
| [DataFusion invocation](../rowfn-datafusion/src/udf.rs) | The wrapper preserves scalar markers, fields, and row count, then delegates to Arrow. It adds no unsafe code or row traversal. Infrastructure errors remain terminal. |

The low-level initialization tokens do not encode row identity. Making their constructors safe would
permit a callback to return evidence for an unrelated slot. `ElementSink` provides a safe alternative
by initializing scalar storage before traversal, with the cost of those default writes.

Borrowed views cannot escape their decoded owners. Preparation belongs to one invocation and can
run again during an eligible retry. Callbacks cannot panic or have observable side effects outside
their supplied output row. These requirements apply even when the invocation fails.

## Verification targets

For an explicitly requested run, focus on borrowed lifetimes, exact-row tokens, partial abandonment,
sliced validity, dictionary nulls, output ownership, and zero-row behavior. Compare fields as well as
values for list shape, timestamps, and semantic extensions.

Compiler-sensitive changes also need matched source, target, CGU, and LTO settings. In particular,
the Boolean implementation retains its combined-state borrow and separate terminal and retry loops.
A source refactor can change inlining or vectorization even when the row callback is unchanged.
See the [comparison guide](COMPARING.md) for benchmark boundaries and commands.
