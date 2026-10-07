<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: Copyright the Vortex contributors -->

# rowfn-kernels

[Overview](../README.md) · [Architecture](../rowfn/ARCHITECTURE.md)

Typed lane traversal, borrowed bitmaps, and Boolean packing. The crate has no backend dependency.
Callers retain storage ownership and supply slots or words to write.

| Interface | Purpose |
| --- | --- |
| `IndexedSource` | Stable typed access to rows. Unsafe implementations must preserve lengths and valid reads. |
| `BitmapView` | Borrowed bytes, bit offset, and logical length, without alignment or padding assumptions. |
| Packing kernels | Collect Boolean values into caller-owned words. |

The inlined tail and separate multiversioned entry points preserve compiler-sensitive source
structure. [Recorded evidence](../BENCHMARKS.md) describes earlier code.
See [status and verification](../STATUS.md) for the current evidence boundary.
