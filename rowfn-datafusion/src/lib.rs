// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright the Vortex contributors

//! DataFusion scalar UDFs backed by strict RowFn definitions and Arrow execution.
//!
//! Register a [`RowFnUdf`] through DataFusion's [`ScalarUDF`]. The wrapper owns registration identity
//! and volatility. [`rowfn_arrow`] owns decoding, traversal, null propagation, and output storage.
//! Input fields and the planned result field cross the boundary without metadata reconstruction.
//!
//! [`ScalarUDF`]: datafusion_expr::ScalarUDF

#![deny(missing_docs)]

mod udf;
pub use udf::RowFnUdf;
