// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright the Vortex contributors

//! Register portable functions and invoke their runtime dispatch through DataFusion SQL.

use datafusion::prelude::SessionContext;
use datafusion_common::Result;
use datafusion_expr::ScalarUDF;
use datafusion_expr::Volatility;
use rowfn_datafusion::RowFnUdf;
use rowfn_functions::Add;
use rowfn_functions::AdjustTicks;
use rowfn_functions::NoOptions;
use rowfn_functions::Seven;
use rowfn_functions::Trim;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let context = SessionContext::new();
    context.register_udf(ScalarUDF::from(RowFnUdf::new(
        "rowfn_add",
        Add::<true>,
        NoOptions,
        Volatility::Immutable,
    )?));
    context.register_udf(ScalarUDF::from(RowFnUdf::new(
        "rowfn_trim",
        Trim,
        NoOptions,
        Volatility::Immutable,
    )?));
    context.register_udf(ScalarUDF::from(RowFnUdf::new(
        "rowfn_adjust_ticks",
        AdjustTicks,
        NoOptions,
        Volatility::Immutable,
    )?));
    context.register_udf(ScalarUDF::from(RowFnUdf::new(
        "rowfn_seven",
        Seven,
        NoOptions,
        Volatility::Immutable,
    )?));

    context.sql(
        "SELECT rowfn_add(CAST(value AS SMALLINT), CAST(2 AS SMALLINT)) AS small_sum,
                rowfn_add(CAST(value AS BIGINT), CAST(40 AS BIGINT)) AS big_sum,
                rowfn_trim(text) AS trimmed,
                rowfn_adjust_ticks(CAST('2026-10-07 12:00:00' AS TIMESTAMP),
                                   CAST(value AS BIGINT)) AS shifted,
                rowfn_seven() AS seven
         FROM (VALUES (1, '  hello  '), (2, '  a longer string  '), (NULL, NULL))
              AS input(value, text)",
    )
    .await?
    .show()
    .await?;

    Ok(())
}
