<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: Copyright the Vortex contributors -->

# Source export

Exported from Vortex base `24a96cece436409dc4f60f94c6b846d6af017804` plus the uncommitted RowFn work on October 6, 2026.

The core, kernels, Arrow adapter, and portable function Rust sources are unchanged. The independent
fixture package enables only Arrow and replaces Vortex-only fixture panic helpers with standard
`expect`. Full Vortex fixtures and the original function manifest are preserved under
`integrations/vortex`. No tests, builds, formatting, linting, or benchmarks ran for this export.
