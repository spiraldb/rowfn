<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: Copyright the Vortex contributors -->

# Vortex integration snapshot

The standalone workspace has no Vortex dependency. This directory preserves the Vortex adapter,
full cross-host fixtures, and original function manifest from the experimental checkout.

Vortex's [RowFn epic](https://github.com/vortex-data/vortex/issues/9128) and
[API tracking issue](https://github.com/vortex-data/vortex/issues/9129) provide the original project
context. This snapshot preserves one experiment, rather than tracking Vortex's current API.

| File | Contains |
| --- | --- |
| [adapter](adapter/mod.rs) | Native decoding, allocation, validity, and registry wrapper. |
| [rowfn-examples](rowfn-examples/README.md) | Cross-host tests, external registration, and complete benchmarks. |
| [rowfn-functions.Cargo.toml](rowfn-functions.Cargo.toml) | The original manifest with optional Vortex mappings. |
| [vortex.patch](vortex.patch) | Supporting Vortex changes and adapter source. |
| [BASE](BASE) | Exact Vortex commit on which the patch is based. |

The snapshot is not a standalone Cargo package and does not target stock Vortex. Rust function
implementations retain their Vortex feature guards. The root manifest recognizes those guards but
does not enable the unavailable host. The preserved manifest enables them inside the patched Vortex
workspace, keeping the domain traits and mappings in the same owning package.

## Reconstruct the experiment

From beside this RowFn checkout, prepare a separate Vortex checkout:

```sh
git clone https://github.com/vortex-data/vortex.git vortex-rowfn
cd vortex-rowfn
git checkout 24a96cece436409dc4f60f94c6b846d6af017804
git apply ../rowfn/integrations/vortex/vortex.patch
cp -R ../rowfn/rowfn ../rowfn/rowfn-kernels ../rowfn/rowfn-arrow ../rowfn/rowfn-functions .
cp ../rowfn/integrations/vortex/rowfn-functions.Cargo.toml rowfn-functions/Cargo.toml
cp -R ../rowfn/integrations/vortex/rowfn-examples .
```

The patch declares those package paths in the Vortex workspace and restores its recorded lockfile
edges. The independent crate sources are shared with this repository. Cross-host function mappings
remain in the function package to satisfy Rust's orphan rules.

This reconstruction command is a reference, not an executed check. See
[status and verification](../../STATUS.md) for the evidence boundary. Keep the original compiler,
target, CGU, and LTO settings when repeating a recorded comparison. Historical timings do not measure
later source.
