// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright the Vortex contributors

//! Invocation contracts at the field-aware DataFusion boundary.

use std::collections::HashSet;
use std::sync::Arc;

use arrow_array::Array;
use arrow_array::ArrayRef;
use arrow_array::DictionaryArray;
use arrow_array::Int8Array;
use arrow_array::Int64Array;
use arrow_array::LargeStringArray;
use arrow_array::StringArray;
use arrow_array::StringViewArray;
use arrow_array::types::Int8Type;
use arrow_buffer::NullBuffer;
use arrow_schema::ArrowError;
use arrow_schema::DataType;
use arrow_schema::Field;
use arrow_schema::FieldRef;
use arrow_schema::TimeUnit;
use datafusion_common::DataFusionError;
use datafusion_common::Result;
use datafusion_common::ScalarValue;
use datafusion_common::config::ConfigOptions;
use datafusion_expr::ColumnarValue;
use datafusion_expr::ReturnFieldArgs;
use datafusion_expr::ScalarFunctionArgs;
use datafusion_expr::ScalarUDF;
use datafusion_expr::ScalarUDFImpl;
use datafusion_expr::Volatility;
use rowfn::InputBinding;
use rowfn::RowFn;
use rowfn::RowKind;
use rowfn::RowVisitor;
use rowfn_arrow::ArrowHost;
use rowfn_datafusion::RowFnUdf;
use rowfn_functions::Add;
use rowfn_functions::AdjustTicks;
use rowfn_functions::Divide;
use rowfn_functions::NoOptions;
use rowfn_functions::Seven;
use rowfn_functions::Trim;
use rstest::rstest;

fn invocation(
    udf: &impl ScalarUDFImpl,
    args: Vec<ColumnarValue>,
    fields: Vec<FieldRef>,
    rows: usize,
) -> Result<ScalarFunctionArgs> {
    let scalars = args
        .iter()
        .map(|arg| match arg {
            ColumnarValue::Scalar(value) => Some(value),
            ColumnarValue::Array(_) => None,
        })
        .collect::<Vec<_>>();
    let return_field = udf.return_field_from_args(ReturnFieldArgs {
        arg_fields: &fields,
        scalar_arguments: &scalars,
    })?;

    Ok(ScalarFunctionArgs {
        args,
        arg_fields: fields,
        number_rows: rows,
        return_field,
        config_options: Arc::new(ConfigOptions::default()),
    })
}

fn run<F>(
    function: F,
    args: Vec<ColumnarValue>,
    fields: Vec<FieldRef>,
    rows: usize,
) -> Result<ColumnarValue>
where
    F: RowFn<ArrowHost, Options = NoOptions>,
{
    let udf = RowFnUdf::new("fixture", function, NoOptions, Volatility::Immutable)?;

    udf.invoke_with_args(invocation(&udf, args, fields, rows)?)
}

fn field(dtype: DataType, nullable: bool) -> FieldRef {
    Arc::new(Field::new("input", dtype, nullable))
}

#[track_caller]
fn assert_scalar(result: ColumnarValue, expected: ScalarValue) {
    let ColumnarValue::Scalar(value) = result else {
        panic!("scalar-only operands require a scalar result");
    };
    assert_eq!(value, expected);
}

#[rstest]
#[case::scalars(true, true)]
#[case::arrays(false, false)]
#[case::array_scalar(false, true)]
#[case::scalar_array(true, false)]
fn addition_preserves_operand_shape(#[case] lhs_scalar: bool, #[case] rhs_scalar: bool) -> Result<()> {
    let lhs = if lhs_scalar {
        ColumnarValue::Scalar(ScalarValue::Int64(Some(10)))
    } else {
        ColumnarValue::Array(Arc::new(Int64Array::from(vec![10, 20])))
    };
    let rhs = if rhs_scalar {
        ColumnarValue::Scalar(ScalarValue::Int64(Some(1)))
    } else {
        ColumnarValue::Array(Arc::new(Int64Array::from(vec![1, 2])))
    };
    let result = run(
        Add::<true>,
        vec![lhs, rhs],
        vec![field(DataType::Int64, false); 2],
        2,
    )?;
    assert_eq!(matches!(result, ColumnarValue::Scalar(_)), lhs_scalar && rhs_scalar);
    let array = result.into_array(2)?;
    let last_lhs = if lhs_scalar { 10 } else { 20 };
    let last_rhs = if rhs_scalar { 1 } else { 2 };
    let expected = Int64Array::from(vec![11, last_lhs + last_rhs]);
    assert_eq!(array.as_any().downcast_ref::<Int64Array>().unwrap(), &expected);

    Ok(())
}

#[rstest]
#[case::i8(ScalarValue::Int8(Some(2)), ScalarValue::Int8(Some(4)))]
#[case::i16(ScalarValue::Int16(Some(2)), ScalarValue::Int16(Some(4)))]
#[case::i32(ScalarValue::Int32(Some(2)), ScalarValue::Int32(Some(4)))]
#[case::i64(ScalarValue::Int64(Some(2)), ScalarValue::Int64(Some(4)))]
#[case::u8(ScalarValue::UInt8(Some(2)), ScalarValue::UInt8(Some(4)))]
#[case::u16(ScalarValue::UInt16(Some(2)), ScalarValue::UInt16(Some(4)))]
#[case::u32(ScalarValue::UInt32(Some(2)), ScalarValue::UInt32(Some(4)))]
#[case::u64(ScalarValue::UInt64(Some(2)), ScalarValue::UInt64(Some(4)))]
fn addition_uses_shared_integer_dispatch(
    #[case] value: ScalarValue,
    #[case] expected: ScalarValue,
) -> Result<()> {
    let fields = vec![field(value.data_type(), false); 2];
    let result = run(
        Add::<true>,
        vec![ColumnarValue::Scalar(value.clone()), ColumnarValue::Scalar(value)],
        fields,
        3,
    )?;
    assert_scalar(result, expected);

    Ok(())
}

#[derive(Clone)]
struct ConstantMarkers;

impl RowFn<ArrowHost> for ConstantMarkers {
    type Options = NoOptions;
    const ARG_NAMES: &'static [&'static str] = &["lhs", "rhs"];
    const INFALLIBLE: bool = true;

    fn dispatch<V: RowVisitor<ArrowHost>>(
        &self,
        _: &NoOptions,
        _: &[Field],
        visitor: V,
    ) -> std::result::Result<V::VisitResult, ArrowError> {
        visitor.visit_prepared::<(i64, i64), i64, i64>(
            |(lhs, rhs)| i64::from(lhs.is_some()) + 2 * i64::from(rhs.is_some()),
            |markers, _| *markers,
        )
    }
}

#[test]
fn preparation_sees_a_scalar_and_a_length_one_array_as_distinct() -> Result<()> {
    let result = run(
        ConstantMarkers,
        vec![
            ColumnarValue::Scalar(ScalarValue::Int64(Some(10))),
            ColumnarValue::Array(Arc::new(Int64Array::from(vec![1]))),
        ],
        vec![field(DataType::Int64, false); 2],
        1,
    )?;
    let ColumnarValue::Array(array) = result else {
        panic!("an array operand requires an array result");
    };
    assert_eq!(
        array.as_any().downcast_ref::<Int64Array>().unwrap(),
        &Int64Array::from(vec![1]),
    );

    Ok(())
}

#[rstest]
#[case::empty(vec![], vec![])]
#[case::all_null(vec![None, None], vec![None, None])]
#[case::partially_valid(vec![Some(1), None, Some(41)], vec![Some(2), None, Some(42)])]
fn addition_keeps_strict_nulls(
    #[case] input: Vec<Option<i64>>,
    #[case] expected: Vec<Option<i64>>,
) -> Result<()> {
    let rows = input.len();
    let result = run(
        Add::<true>,
        vec![
            ColumnarValue::Array(Arc::new(Int64Array::from(input))),
            ColumnarValue::Scalar(ScalarValue::Int64(Some(1))),
        ],
        vec![field(DataType::Int64, true), field(DataType::Int64, false)],
        rows,
    )?
    .into_array(rows)?;
    assert_eq!(
        result.as_any().downcast_ref::<Int64Array>().unwrap(),
        &Int64Array::from(expected),
    );

    Ok(())
}

#[rstest]
#[case::scalar(false)]
#[case::array(true)]
fn a_null_operand_suppresses_a_failing_scalar(#[case] null_array: bool) -> Result<()> {
    let null = ScalarValue::Int64(None);
    let input = if null_array {
        ColumnarValue::Array(null.to_array_of_size(3)?)
    } else {
        ColumnarValue::Scalar(null)
    };
    let result = run(
        Divide,
        vec![input, ColumnarValue::Scalar(ScalarValue::Int64(Some(0)))],
        vec![field(DataType::Int64, true), field(DataType::Int64, false)],
        3,
    )?
    .into_array(3)?;
    assert_eq!(result.null_count(), 3);

    Ok(())
}

#[test]
fn deferred_overflow_in_a_null_payload_is_suppressed() -> Result<()> {
    let input = Int64Array::new(
        vec![4, i64::MAX, 40].into(),
        Some(NullBuffer::from(vec![true, false, true])),
    );
    let result = run(
        Add::<true>,
        vec![
            ColumnarValue::Array(Arc::new(input)),
            ColumnarValue::Scalar(ScalarValue::Int64(Some(1))),
        ],
        vec![field(DataType::Int64, true), field(DataType::Int64, false)],
        3,
    )?
    .into_array(3)?;
    assert_eq!(
        result.as_any().downcast_ref::<Int64Array>().unwrap(),
        &Int64Array::from(vec![Some(5), None, Some(41)]),
    );

    Ok(())
}

#[test]
fn immediate_division_errors_in_null_payloads_are_skipped() -> Result<()> {
    let lhs = Int64Array::new(vec![8, 8].into(), Some(NullBuffer::from(vec![true, false])));
    let result = run(
        Divide,
        vec![
            ColumnarValue::Array(Arc::new(lhs)),
            ColumnarValue::Array(Arc::new(Int64Array::from(vec![2, 0]))),
        ],
        vec![field(DataType::Int64, true), field(DataType::Int64, false)],
        2,
    )?
    .into_array(2)?;
    assert_eq!(
        result.as_any().downcast_ref::<Int64Array>().unwrap(),
        &Int64Array::from(vec![Some(4), None]),
    );

    Ok(())
}

#[test]
fn valid_row_errors_reach_datafusion() {
    let fields = vec![field(DataType::Int64, false); 2];
    let overflow = run(
        Add::<true>,
        vec![
            ColumnarValue::Scalar(ScalarValue::Int64(Some(i64::MAX))),
            ColumnarValue::Scalar(ScalarValue::Int64(Some(1))),
        ],
        fields.clone(),
        1,
    );
    assert!(matches!(overflow, Err(DataFusionError::Execution(message)) if message.contains("overflow")));
    let partial = run(
        Add::<true>,
        vec![
            ColumnarValue::Array(Arc::new(Int64Array::from(vec![Some(i64::MAX), None]))),
            ColumnarValue::Scalar(ScalarValue::Int64(Some(1))),
        ],
        vec![field(DataType::Int64, true), field(DataType::Int64, false)],
        2,
    );
    assert!(matches!(partial, Err(DataFusionError::Execution(message)) if message.contains("overflow")));
    let divide = run(
        Divide,
        vec![
            ColumnarValue::Scalar(ScalarValue::Int64(Some(8))),
            ColumnarValue::Scalar(ScalarValue::Int64(Some(0))),
        ],
        fields,
        1,
    );
    assert!(matches!(divide, Err(DataFusionError::Execution(message)) if message.contains("division")));
}

#[derive(Clone, Copy)]
struct UnavailableValue(i64);

impl RowKind for UnavailableValue {
    type Value<'a> = Self;
}

impl InputBinding<UnavailableValue> for ArrowHost {
    type Decoded = Vec<UnavailableValue>;
    type View<'a> = &'a [UnavailableValue];
    const DENSE_SAFE: bool = true;
    // Resource errors do not change semantic decode infallibility.
    const DECODE_INFALLIBLE: bool = true;

    fn validate(dtype: &Field) -> std::result::Result<(), ArrowError> {
        <Self as InputBinding<i64>>::validate(dtype)
    }

    fn decode(_: &ArrayRef, _: bool, _: &mut ()) -> std::result::Result<Self::Decoded, ArrowError> {
        Err(ArrowError::MemoryError("decoder resource failure".into()))
    }

    fn can_decode_null_tolerant(_: &ArrayRef) -> std::result::Result<bool, ArrowError> {
        Ok(true)
    }

    fn view(decoded: &Self::Decoded) -> Self::View<'_> {
        decoded.as_slice()
    }
}

#[derive(Clone)]
struct DecodeFailure;

impl RowFn<ArrowHost> for DecodeFailure {
    type Options = NoOptions;
    const ARG_NAMES: &'static [&'static str] = &["input"];
    const INFALLIBLE: bool = false;

    fn dispatch<V: RowVisitor<ArrowHost>>(
        &self,
        _: &NoOptions,
        _: &[Field],
        visitor: V,
    ) -> std::result::Result<V::VisitResult, ArrowError> {
        visitor.visit_deferred::<(UnavailableValue,), i64, bool>(
            |(value,)| (value.0, false),
            |_| Ok(()),
        )
    }
}

#[test]
fn infrastructure_errors_cannot_be_suppressed_by_nulls() {
    let result = run(
        DecodeFailure,
        vec![ColumnarValue::Array(Arc::new(Int64Array::from(vec![Some(1), None])))],
        vec![field(DataType::Int64, true)],
        2,
    );
    let Err(DataFusionError::ArrowError(error, _)) = result else {
        panic!("decoder resource failures must retain the Arrow error family");
    };
    assert!(matches!(error.as_ref(), ArrowError::MemoryError(message)
        if message == "decoder resource failure"));
}

#[test]
fn zero_rows_do_not_evaluate_failing_scalars() -> Result<()> {
    let result = run(
        Add::<true>,
        vec![
            ColumnarValue::Scalar(ScalarValue::Int64(Some(i64::MAX))),
            ColumnarValue::Scalar(ScalarValue::Int64(Some(1))),
        ],
        vec![field(DataType::Int64, false); 2],
        0,
    )?;
    let ColumnarValue::Array(array) = result else {
        panic!("zero rows require an empty array result");
    };
    assert_eq!(array.len(), 0);
    assert_eq!(array.data_type(), &DataType::Int64);

    Ok(())
}

#[rstest]
#[case::empty(0)]
#[case::one(1)]
#[case::batch(7)]
fn nullary_calls_keep_the_logical_row_count(#[case] rows: usize) -> Result<()> {
    let result = run(Seven, vec![], vec![], rows)?;
    let ColumnarValue::Array(array) = result else {
        panic!("nullary calls retain the row domain");
    };
    assert_eq!(
        array.as_any().downcast_ref::<Int64Array>().unwrap(),
        &Int64Array::from(vec![7; rows]),
    );

    Ok(())
}

#[rstest]
#[case::utf8(DataType::Utf8)]
#[case::large_utf8(DataType::LargeUtf8)]
#[case::utf8_view(DataType::Utf8View)]
fn trimmed_output_owns_its_bytes(#[case] dtype: DataType) -> Result<()> {
    let values = vec![
        Some("unused"), // This prefix is discarded.
        Some("\u{2003}a long string outside an inline view\u{2003}"), // This output needs owned bytes.
        None, // This row remains null.
        Some(" é "), // This multibyte string fits inline.
    ];
    let input: ArrayRef = match dtype {
        DataType::Utf8 => Arc::new(StringArray::from(values)),
        DataType::LargeUtf8 => Arc::new(LargeStringArray::from(values)),
        DataType::Utf8View => Arc::new(StringViewArray::from(values)),
        _ => unreachable!("the cases specify supported string storage"),
    };
    let input_buffers = input
        .to_data()
        .buffers()
        .iter()
        .filter(|buffer| !buffer.is_empty())
        .map(|buffer| buffer.as_ptr())
        .collect::<Vec<_>>();
    let result = run(
        Trim,
        vec![ColumnarValue::Array(input.slice(1, 3))],
        vec![field(dtype, true)],
        3,
    )?
    .into_array(3)?;
    let strings = result.as_any().downcast_ref::<StringViewArray>().unwrap();
    assert!(strings.data_buffers().iter().all(|buffer| !input_buffers.contains(&buffer.as_ptr())));
    drop(input);
    assert_eq!(strings, &StringViewArray::from(vec![
        Some("a long string outside an inline view"), // Output storage owns these bytes.
        None, // Null inputs stay null.
        Some("é"), // This output fits inline.
    ]));
    assert!(!strings.data_buffers().is_empty());

    Ok(())
}

#[test]
fn scalar_strings_return_owned_scalar_output() -> Result<()> {
    let result = run(
        Trim,
        vec![ColumnarValue::Scalar(ScalarValue::Utf8(Some(
            "  an independently owned long string  ".into(),
        )))],
        vec![field(DataType::Utf8, false)],
        4,
    )?;
    assert_scalar(
        result,
        ScalarValue::Utf8View(Some("an independently owned long string".into())),
    );

    Ok(())
}

fn timestamp(unit: &TimeUnit, value: Option<i64>, timezone: Option<Arc<str>>) -> ScalarValue {
    match unit {
        TimeUnit::Second => ScalarValue::TimestampSecond(value, timezone),
        TimeUnit::Millisecond => ScalarValue::TimestampMillisecond(value, timezone),
        TimeUnit::Microsecond => ScalarValue::TimestampMicrosecond(value, timezone),
        TimeUnit::Nanosecond => ScalarValue::TimestampNanosecond(value, timezone),
    }
}

#[rstest]
#[case::seconds(TimeUnit::Second)]
#[case::milliseconds(TimeUnit::Millisecond)]
#[case::microseconds(TimeUnit::Microsecond)]
#[case::nanoseconds(TimeUnit::Nanosecond)]
fn timestamps_keep_units_timezone_and_planned_metadata(
    #[case] unit: TimeUnit,
    #[values(None, Some("Europe/London"))] timezone: Option<&str>,
    #[values(0, 2)] rows: usize,
) -> Result<()> {
    let timezone = timezone.map(Arc::<str>::from);
    let values = ScalarValue::iter_to_array([
        timestamp(&unit, Some(100), timezone.clone()),
        timestamp(&unit, None, timezone.clone()),
    ])?;
    let input = Field::new("observed", values.data_type().clone(), true)
        .with_metadata([("device".into(), "clock-a".into())].into());
    let udf = RowFnUdf::new("ticks", AdjustTicks, NoOptions, Volatility::Immutable)?;
    let args = invocation(
        &udf,
        vec![
            ColumnarValue::Array(values.slice(0, rows)),
            ColumnarValue::Scalar(ScalarValue::Int64(Some(10))),
        ],
        vec![Arc::new(input.clone()), field(DataType::Int64, false)],
        rows,
    )?;
    assert_eq!(args.return_field.data_type(), input.data_type());
    assert_eq!(args.return_field.metadata(), input.metadata());
    assert!(args.return_field.is_nullable());
    let result = udf.invoke_with_args(args)?.into_array(rows)?;
    let expected = ScalarValue::iter_to_array([
        timestamp(&unit, Some(110), timezone.clone()),
        timestamp(&unit, None, timezone.clone()),
    ])?
    .slice(0, rows);
    assert_eq!(result.to_data(), expected.to_data());

    Ok(())
}

#[rstest]
#[case::valid(Some(100), Some(105))]
#[case::all_null(None, None)]
fn scalar_timestamp_output_retains_its_timezone(
    #[case] value: Option<i64>,
    #[case] expected: Option<i64>,
) -> Result<()> {
    let timezone = Some(Arc::<str>::from("UTC"));
    let input = ScalarValue::TimestampNanosecond(value, timezone.clone());
    let result = run(
        AdjustTicks,
        vec![
            ColumnarValue::Scalar(input.clone()),
            ColumnarValue::Scalar(ScalarValue::Int64(Some(5))),
        ],
        vec![field(input.data_type(), true), field(DataType::Int64, false)],
        3,
    )?;
    assert_scalar(result, ScalarValue::TimestampNanosecond(expected, timezone));

    Ok(())
}

#[test]
fn dictionary_invocation_uses_referenced_logical_values() -> Result<()> {
    let dictionary = DictionaryArray::<Int8Type>::try_new(
        Int8Array::from(vec![Some(0), Some(1), None, Some(0)]),
        Arc::new(Int64Array::from(vec![Some(10), None, Some(i64::MAX)])),
    )?;
    let dtype = dictionary.data_type().clone();
    let result = run(
        Add::<true>,
        vec![
            ColumnarValue::Array(Arc::new(dictionary)),
            ColumnarValue::Scalar(ScalarValue::Int64(Some(1))),
        ],
        vec![field(dtype, true), field(DataType::Int64, false)],
        4,
    )?
    .into_array(4)?;
    assert_eq!(
        result.as_any().downcast_ref::<Int64Array>().unwrap(),
        &Int64Array::from(vec![Some(11), None, None, Some(11)]),
    );

    Ok(())
}

#[test]
fn unsupported_types_and_extensions_fail_during_planning() -> Result<()> {
    let udf = RowFnUdf::new("add", Add::<true>, NoOptions, Volatility::Immutable)?;
    let extension = Field::new("input", DataType::Int64, true)
        .with_metadata([("ARROW:extension:name".into(), "example.counter".into())].into());
    let cases = [
        vec![field(DataType::Float64, false); 2],
        vec![field(DataType::Int32, false), field(DataType::Int64, false)],
        vec![Arc::new(extension), field(DataType::Int64, false)],
        vec![
            field(DataType::Dictionary(Box::new(DataType::Int8), Box::new(DataType::Boolean)), true),
            field(DataType::Int64, false),
        ],
        vec![field(DataType::Int64, false)],
    ];

    for fields in cases {
        let scalars = vec![None; fields.len()];
        assert!(matches!(
            udf.return_field_from_args(ReturnFieldArgs {
                arg_fields: &fields,
                scalar_arguments: &scalars,
            }),
            Err(DataFusionError::Plan(_)),
        ));
    }
    assert!(matches!(
        udf.return_type(&[DataType::Int64, DataType::Int64]),
        Err(DataFusionError::Internal(_)),
    ));

    Ok(())
}

#[test]
fn timestamp_and_text_extensions_require_an_explicit_semantic_mapping() -> Result<()> {
    let timestamp = Field::new("time", DataType::Timestamp(TimeUnit::Second, None), true)
        .with_metadata([("ARROW:extension:name".into(), "example.time".into())].into());
    let text = Field::new("text", DataType::Utf8, true)
        .with_metadata([("ARROW:extension:name".into(), "example.text".into())].into());
    let ticks = RowFnUdf::new("ticks", AdjustTicks, NoOptions, Volatility::Immutable)?;
    let trim = RowFnUdf::new("trim", Trim, NoOptions, Volatility::Immutable)?;
    assert!(matches!(
        ticks.return_field_from_args(ReturnFieldArgs {
            arg_fields: &[Arc::new(timestamp), field(DataType::Int64, false)],
            scalar_arguments: &[None, None],
        }),
        Err(DataFusionError::Plan(_)),
    ));
    assert!(matches!(
        trim.return_field_from_args(ReturnFieldArgs {
            arg_fields: &[Arc::new(text)],
            scalar_arguments: &[None],
        }),
        Err(DataFusionError::Plan(_)),
    ));

    Ok(())
}

#[test]
fn invalid_invocations_remain_terminal_even_for_null_inputs() -> Result<()> {
    let udf = RowFnUdf::new("add", Add::<true>, NoOptions, Volatility::Immutable)?;
    let args = vec![
        ColumnarValue::Array(Arc::new(Int64Array::from(vec![None]))),
        ColumnarValue::Scalar(ScalarValue::Int64(Some(1))),
    ];
    let fields = vec![field(DataType::Int64, true), field(DataType::Int64, false)];
    let mismatched_length = invocation(&udf, args.clone(), fields.clone(), 2)?;
    assert!(matches!(udf.invoke_with_args(mismatched_length), Err(DataFusionError::Execution(_))));

    let mut missing_field = invocation(&udf, args.clone(), fields.clone(), 1)?;
    missing_field.arg_fields.pop();
    assert!(matches!(udf.invoke_with_args(missing_field), Err(DataFusionError::Internal(_))));

    let mut changed_metadata = invocation(&udf, args, fields, 1)?;
    changed_metadata.return_field = Arc::new(
        changed_metadata.return_field.as_ref().clone()
            .with_metadata([("changed".into(), "after planning".into())].into()),
    );
    assert!(matches!(udf.invoke_with_args(changed_metadata), Err(DataFusionError::Execution(_))));

    let non_nullable = run(
        Add::<true>,
        vec![
            ColumnarValue::Scalar(ScalarValue::Int64(None)),
            ColumnarValue::Scalar(ScalarValue::Int64(Some(1))),
        ],
        vec![field(DataType::Int64, false); 2],
        1,
    );
    assert!(matches!(non_nullable, Err(DataFusionError::Execution(_))));

    Ok(())
}

#[test]
fn planned_nullability_does_not_depend_on_current_values() -> Result<()> {
    let udf = RowFnUdf::new("add", Add::<true>, NoOptions, Volatility::Immutable)?;
    let args = invocation(
        &udf,
        vec![ColumnarValue::Scalar(ScalarValue::Int64(Some(1))); 2],
        vec![field(DataType::Int64, true); 2],
        3,
    )?;
    assert!(args.return_field.is_nullable());
    assert_scalar(udf.invoke_with_args(args)?, ScalarValue::Int64(Some(2)));

    Ok(())
}

#[test]
fn identity_volatility_and_strictness_belong_to_registration() -> Result<()> {
    let udf = RowFnUdf::new("add", Add::<true>, NoOptions, Volatility::Immutable)?;
    assert!(udf.is_strict());
    let scalar = ScalarUDF::from(udf);
    let separate = ScalarUDF::from(RowFnUdf::new("add", Add::<true>, NoOptions, Volatility::Immutable)?);
    let mut identities = HashSet::new();
    identities.insert(scalar.clone());
    identities.insert(scalar);
    identities.insert(separate);
    assert_eq!(identities.len(), 2);

    let stable = RowFnUdf::new("stable", Seven, NoOptions, Volatility::Stable)?;
    assert_eq!(stable.signature().volatility, Volatility::Stable);
    assert!(matches!(
        RowFnUdf::new("volatile", Seven, NoOptions, Volatility::Volatile),
        Err(DataFusionError::Plan(_)),
    ));

    Ok(())
}
