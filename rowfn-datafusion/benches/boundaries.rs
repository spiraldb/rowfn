// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright the Vortex contributors

//! Direct Arrow invocation, UDF wrapper cost, and matched native DataFusion controls.

use std::sync::Arc;

use arrow_array::Int64Array;
use arrow_array::RecordBatch;
use arrow_array::StringViewArray;
use arrow_schema::DataType;
use arrow_schema::Field;
use arrow_schema::Schema;
use datafusion::physical_expr::PhysicalExpr;
use datafusion::physical_expr::expressions::BinaryExpr;
use datafusion::physical_expr::expressions::col;
use datafusion::physical_expr::expressions::lit;
use datafusion_common::ScalarValue;
use datafusion_common::config::ConfigOptions;
use datafusion_expr::ColumnarValue;
use datafusion_expr::Operator;
use datafusion_expr::ReturnFieldArgs;
use datafusion_expr::ScalarFunctionArgs;
use datafusion_expr::ScalarUDFImpl;
use datafusion_expr::Volatility;
use divan::Bencher;
use divan::black_box;
use rowfn_arrow::ArrowOperand;
use rowfn_datafusion::RowFnUdf;
use rowfn_functions::Add;
use rowfn_functions::NoOptions;
use rowfn_functions::Trim;

const SIZES: &[usize] = &[0, 1, 64, 16_384];

fn main() {
    divan::main();
}

#[derive(Clone, Copy, Debug)]
enum Boundary {
    Arrow,
    Wrapper,
    Native,
}

const BOUNDARIES: &[Boundary] = &[Boundary::Arrow, Boundary::Wrapper, Boundary::Native];

fn arguments(
    udf: &(impl ScalarUDFImpl + ?Sized),
    inputs: &[ArrowOperand],
    rows: usize,
) -> ScalarFunctionArgs {
    let args = inputs
        .iter()
        .map(|input| {
            if input.scalar {
                ColumnarValue::Scalar(
                    ScalarValue::try_from_array(&input.column, 0)
                        .expect("scalar fixtures contain one supported row"),
                )
            } else {
                ColumnarValue::Array(Arc::clone(&input.column))
            }
        })
        .collect::<Vec<_>>();
    let fields = inputs.iter().map(|input| Arc::new(input.dtype.clone())).collect::<Vec<_>>();
    let scalars = args
        .iter()
        .map(|arg| match arg {
            ColumnarValue::Scalar(value) => Some(value),
            ColumnarValue::Array(_) => None,
        })
        .collect::<Vec<_>>();
    let return_field = udf
        .return_field_from_args(ReturnFieldArgs {
            arg_fields: &fields,
            scalar_arguments: &scalars,
        })
        .expect("fixture fields satisfy the shared planner");

    ScalarFunctionArgs {
        args,
        arg_fields: fields,
        number_rows: rows,
        return_field,
        config_options: Arc::new(ConfigOptions::default()),
    }
}

#[divan::bench(consts = SIZES, args = BOUNDARIES)]
fn checked_add_arrays<const ROWS: usize>(bencher: Bencher, boundary: Boundary) {
    addition::<ROWS>(bencher, boundary, false, false);
}

#[divan::bench(consts = SIZES, args = BOUNDARIES)]
fn checked_add_scalar_rhs<const ROWS: usize>(bencher: Bencher, boundary: Boundary) {
    addition::<ROWS>(bencher, boundary, false, true);
}

#[divan::bench(consts = SIZES, args = BOUNDARIES)]
fn checked_add_scalars<const ROWS: usize>(bencher: Bencher, boundary: Boundary) {
    addition::<ROWS>(bencher, boundary, true, true);
}

fn addition<const ROWS: usize>(
    bencher: Bencher,
    boundary: Boundary,
    scalar_lhs: bool,
    scalar_rhs: bool,
) {
    let lhs = Arc::new(if scalar_lhs {
        Int64Array::from(vec![5])
    } else {
        Int64Array::from_iter_values(
            (0..ROWS).map(|row| i64::try_from(row % 1024).expect("fixture values are below 1024")),
        )
    });
    let rhs = Arc::new(Int64Array::from(vec![3; if scalar_rhs { 1 } else { ROWS }]));
    let inputs = [
        ArrowOperand {
            column: lhs.clone(),
            dtype: Field::new("lhs", DataType::Int64, false),
            scalar: scalar_lhs,
        },
        ArrowOperand {
            column: rhs.clone(),
            dtype: Field::new("rhs", DataType::Int64, false),
            scalar: scalar_rhs,
        },
    ];
    let udf = RowFnUdf::new("add", Add::<true>, NoOptions, Volatility::Immutable)
        .expect("the fixture uses immutable registration");
    let args = arguments(&udf, &inputs, ROWS);
    let output = args.return_field.as_ref().clone();
    let schema = Schema::new(vec![inputs[0].dtype.clone(), inputs[1].dtype.clone()]);
    let batch = RecordBatch::try_new(
        Arc::new(schema.clone()),
        vec![
            if scalar_lhs { Arc::new(Int64Array::from(vec![5; ROWS])) } else { lhs },
            if scalar_rhs { Arc::new(Int64Array::from(vec![3; ROWS])) } else { rhs },
        ],
    )
    .expect("native fixture columns contain the logical row count");
    let lhs = if scalar_lhs {
        lit(ScalarValue::Int64(Some(5)))
    } else {
        col("lhs", &schema).expect("the fixture schema contains lhs")
    };
    let rhs = if scalar_rhs {
        lit(ScalarValue::Int64(Some(3)))
    } else {
        col("rhs", &schema).expect("the fixture schema contains rhs")
    };
    // DataFusion defaults to wrapping addition. The control must use checked arithmetic.
    let native = BinaryExpr::new(lhs, Operator::Plus, rhs).with_fail_on_overflow(true);
    let direct = || {
        rowfn_arrow::invoke(&Add::<true>, &NoOptions, black_box(&inputs), ROWS, output.clone())
            .expect("the bounded fixture cannot overflow")
            .array
    };
    let wrapped = || {
        udf.invoke_with_args(black_box(args.clone()))
            .expect("the bounded fixture cannot overflow")
    };
    let native = || native.evaluate(black_box(&batch)).expect("the bounded fixture cannot overflow");
    let expected = direct();
    assert_eq!(wrapped().into_array(ROWS).unwrap().to_data(), expected.to_data());
    assert_eq!(native().into_array(ROWS).unwrap().to_data(), expected.to_data());

    match boundary {
        Boundary::Arrow => bencher.bench_local(direct),
        Boundary::Wrapper => bencher.bench_local(wrapped),
        Boundary::Native => bencher.bench_local(native),
    }
}

#[divan::bench(consts = SIZES, args = BOUNDARIES)]
fn trim_owned_output<const ROWS: usize>(bencher: Bencher, boundary: Boundary) {
    let input = ArrowOperand {
        column: Arc::new(StringViewArray::from(vec!["  a long string outside inline storage  "; ROWS])),
        dtype: Field::new("text", DataType::Utf8View, false),
        scalar: false,
    };
    let inputs = [input];
    let udf = RowFnUdf::new("trim", Trim, NoOptions, Volatility::Immutable)
        .expect("the fixture uses immutable registration");
    let args = arguments(&udf, &inputs, ROWS);
    let native_udf = datafusion::functions::string::btrim();
    let native_args = arguments(native_udf.inner().as_ref(), &inputs, ROWS);
    let output = args.return_field.as_ref().clone();
    let direct = || {
        rowfn_arrow::invoke(&Trim, &NoOptions, black_box(&inputs), ROWS, output.clone())
            .expect("the string fixture satisfies the shared binding")
            .array
    };
    let wrapped = || {
        udf.invoke_with_args(black_box(args.clone()))
            .expect("the string fixture satisfies the shared binding")
    };
    let native = || {
        native_udf.invoke_with_args(black_box(native_args.clone()))
            .expect("native trim supports Utf8View")
    };
    let expected = direct();
    let expected = expected.as_any().downcast_ref::<StringViewArray>().unwrap();
    let wrapped_output = wrapped().into_array(ROWS).unwrap();
    let native_output = native().into_array(ROWS).unwrap();
    assert_eq!(wrapped_output.as_any().downcast_ref::<StringViewArray>().unwrap(), expected);
    assert_eq!(native_output.as_any().downcast_ref::<StringViewArray>().unwrap(), expected);

    match boundary {
        Boundary::Arrow => bencher.bench_local(direct),
        Boundary::Wrapper => bencher.bench_local(wrapped),
        Boundary::Native => bencher.bench_local(native),
    }
}
