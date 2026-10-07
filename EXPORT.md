<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: Copyright the Vortex contributors -->

# Initial source export

This record describes the October 6, 2026 export from Vortex base
`24a96cece436409dc4f60f94c6b846d6af017804` plus the uncommitted RowFn work. See
[status and verification](STATUS.md) for later standalone changes.

At export, the core, kernels, Arrow adapter, and portable function Rust sources were carried across
unchanged. The independent fixture package enables only Arrow and replaces Vortex-only fixture panic
helpers with standard `expect`. Full Vortex fixtures and the original function manifest are preserved
under `integrations/vortex`. No tests, builds, formatting, linting, or benchmarks ran for this export.
