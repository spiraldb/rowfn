// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright the Vortex contributors

//! SQL registration and execution of the shared portable definitions.

use std::sync::Arc;

use arrow_array::Array;
use arrow_array::Int8Array;
use arrow_array::Int16Array;
use arrow_array::Int64Array;
use arrow_array::RecordBatch;
use arrow_array::StringArray;
use arrow_array::StringViewArray;
use arrow_array::TimestampMicrosecondArray;
use arrow_array::UInt32Array;
use arrow_buffer::NullBuffer;
use arrow_schema::DataType;
use arrow_schema::Field;
use arrow_schema::Schema;
use arrow_schema::TimeUnit;
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

fn context() -> Result<SessionContext> {
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

    Ok(context)
}

#[tokio::test(flavor = "current_thread")]
async fn sql_dispatches_integer_widths_text_and_timestamp_ticks() -> Result<()> {
    let context = context()?;
    let timestamp_field = Field::new(
        "observed",
        DataType::Timestamp(TimeUnit::Microsecond, Some("Europe/London".into())),
        true,
    )
    .with_metadata([("device".into(), "clock-a".into())].into());
    let schema = Arc::new(Schema::new(vec![
        Field::new("small", DataType::Int16, true),
        Field::new("big", DataType::Int64, true),
        Field::new("unsigned", DataType::UInt32, false),
        Field::new("text", DataType::Utf8, true),
        timestamp_field.clone(),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Int16Array::from(vec![Some(1), None, Some(20)])),
            Arc::new(Int64Array::new(
                vec![2, 2, i64::MAX].into(),
                Some(NullBuffer::from(vec![true, true, false])),
            )),
            Arc::new(UInt32Array::from(vec![2, 3, 4])),
            Arc::new(StringArray::from(vec![
                Some("  hello  "), // This string uses ordinary spaces.
                None, // This row remains null.
                Some("\u{2003}a long accented é string\u{2003}"), // This string uses Unicode spaces.
            ])),
            Arc::new(
                TimestampMicrosecondArray::from(vec![Some(100), None, Some(200)])
                    .with_timezone("Europe/London"),
            ),
        ],
    )?;
    context.register_batch("input", batch)?;
    let batches = context.sql(
        "SELECT rowfn_add(small, CAST(1 AS SMALLINT)) AS small_sum,
                rowfn_add(big, CAST(40 AS BIGINT)) AS big_sum,
                rowfn_add(unsigned, unsigned) AS unsigned_sum,
                rowfn_trim(text) AS trimmed,
                rowfn_adjust_ticks(observed, CAST(5 AS BIGINT)) AS shifted,
                rowfn_seven() AS seven
         FROM input",
    )
    .await?
    .collect()
    .await?;
    assert_eq!(batches.len(), 1);
    let batch = &batches[0];
    assert_eq!(
        batch.column(0).as_any().downcast_ref::<Int16Array>().unwrap(),
        &Int16Array::from(vec![Some(2), None, Some(21)]),
    );
    assert_eq!(
        batch.column(1).as_any().downcast_ref::<Int64Array>().unwrap(),
        &Int64Array::from(vec![Some(42), Some(42), None]),
    );
    assert_eq!(
        batch.column(2).as_any().downcast_ref::<UInt32Array>().unwrap(),
        &UInt32Array::from(vec![4, 6, 8]),
    );
    assert_eq!(
        batch.column(3).as_any().downcast_ref::<StringViewArray>().unwrap(),
        &StringViewArray::from(vec![
            Some("hello"), // The ordinary spaces are removed.
            None, // Null inputs stay null.
            Some("a long accented é string"), // The Unicode spaces are removed.
        ]),
    );
    let expected = TimestampMicrosecondArray::from(vec![Some(105), None, Some(205)])
        .with_timezone("Europe/London");
    assert_eq!(batch.column(4).to_data(), expected.to_data());
    assert_eq!(batch.schema().field(4).metadata(), timestamp_field.metadata());
    assert_eq!(
        batch.column(5).as_any().downcast_ref::<Int64Array>().unwrap(),
        &Int64Array::from(vec![7, 7, 7]),
    );

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sql_invokes_scalar_and_nullary_definitions() -> Result<()> {
    let batches = context()?.sql(
        "SELECT rowfn_add(CAST(2 AS TINYINT), CAST(3 AS TINYINT)) AS sum,
                rowfn_trim('  a long scalar string  ') AS trimmed,
                rowfn_seven() AS seven",
    )
    .await?
    .collect()
    .await?;
    assert_eq!(batches.len(), 1);
    let batch = &batches[0];
    assert_eq!(
        batch.column(0).as_any().downcast_ref::<Int8Array>().unwrap(),
        &Int8Array::from(vec![5]),
    );
    assert_eq!(
        batch.column(1).as_any().downcast_ref::<StringViewArray>().unwrap(),
        &StringViewArray::from(vec!["a long scalar string"]),
    );
    assert_eq!(
        batch.column(2).as_any().downcast_ref::<Int64Array>().unwrap(),
        &Int64Array::from(vec![7]),
    );

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sql_reports_unsupported_types_and_valid_row_overflow() -> Result<()> {
    let context = context()?;
    assert!(context.sql("SELECT rowfn_add(CAST(1 AS DOUBLE), CAST(2 AS DOUBLE))")
        .await.is_err());
    context.register_batch(
        "input",
        RecordBatch::try_new(
            Arc::new(Schema::new(vec![Field::new("value", DataType::Int64, false)])),
            vec![Arc::new(Int64Array::from(vec![i64::MAX]))],
        )?,
    )?;
    let result = context.sql("SELECT rowfn_add(value, CAST(1 AS BIGINT)) FROM input")
        .await?
        .collect()
        .await;
    assert!(result.is_err_and(|error| error.to_string().contains("integer overflow in checked add")));

    Ok(())
}
