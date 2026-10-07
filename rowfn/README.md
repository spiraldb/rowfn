<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: Copyright the Vortex contributors -->

# rowfn

[Overview](../README.md) · [Architecture](ARCHITECTURE.md) · [Rust interfaces](INTERFACES.md)

The framework for turning typed row operations into columnar execution. Functions use `RowVisitor`
to select input types, output storage, and callbacks. Planning validates that selection. Execution
handles constants, preparation, traversal, nulls, and eligible retries through backend bindings.

`rowfn` depends only on `rowfn-kernels`. Backends retain native types, decoded owners, allocation,
and publication. Registration, serialization, optimizer rules, and whole-batch functions belong to
their host interfaces.

## Write a function

Inside `RowFn::dispatch`, a wrapping addition selects signed 64-bit inputs and output:

```rust,ignore
visitor.visit::<(i64, i64), i64>(|(lhs, rhs)| lhs.wrapping_add(rhs))
```

The [author guide](AUTHORING.md) covers complete definitions, runtime type dispatch, preparation,
checked errors, and owned strings. The [adapter guide](ADAPTERS.md) covers native storage bindings.

Every function follows the same contract:

- Null inputs imply null output. Valid inputs cannot produce null.
- Callbacks cannot panic or have observable side effects outside their supplied output row.
- Constants and retries can change callback counts. Preparation belongs to one invocation.
- Only rejected deferred row evidence permits a null-based retry. Infrastructure errors are terminal.

`ElementSink` supplies safe initialized scalar rows. Uninitialized sinks require evidence for the
exact callback row. See the [safety contracts](SAFETY.md) before implementing a binding or sink.

## Further reading

| Read | For |
| --- | --- |
| [Backends](BACKENDS.md) | Arrow, DataFusion, and other integration boundaries. |
| [Comparison guide](COMPARING.md) | Matching semantics and measuring execution costs. |
| [Status and verification](../STATUS.md) | Current evidence and unrun checks. |

The authoring model originated in the Vortex [RowFn epic](https://github.com/vortex-data/vortex/issues/9128)
and [API tracking issue](https://github.com/vortex-data/vortex/issues/9129). The interfaces here use
backend-independent contracts. Rust traits do not define a stable binary ABI.
