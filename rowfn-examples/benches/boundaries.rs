// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright the Vortex contributors

//! Compare direct collection, shared collection, and complete Arrow invocation.
//!
//! The workspace bench profile uses 16 CGUs without LTO. Keep compiler, target features, allocator,
//! and profile identical for every matched comparison.

#![allow(clippy::expect_used)] // Benchmark fixtures report invariant failures before timing.

use std::sync::Arc;

use arrow_array::Array;
use arrow_array::ArrayRef;
use arrow_array::Int64Array;
use arrow_array::LargeStringArray;
use arrow_array::StringArray;
use arrow_array::StringViewArray;
use arrow_schema::Field;
use divan::Bencher;
use divan::black_box;
use rowfn::InputBinding;
use rowfn::InputTuple;
use rowfn::OutputBinding;
use rowfn::OutputBuffer;
use rowfn::RowKind;
use rowfn::TextBinding;
use rowfn::TextLayout;
use rowfn::TextValue;
use rowfn::kernels::lane_kernels::LaneZip;
use rowfn_arrow::{ArrowHost, ArrowOperand};
use rowfn_examples::{Add, NoOptions};

const SIZES: &[usize] = &[0, 1, 64, 1024, 16_384];

fn main() {
    divan::main();
}

fn values(rows: usize) -> Vec<i64> {
    (0..rows).map(|row| i64::try_from(row % 1024).expect("fixture is below 1024")).collect()
}

fn operand(column: ArrayRef, scalar: bool) -> ArrowOperand {
    ArrowOperand {
        dtype: Field::new("input", column.data_type().clone(), column.null_count() != 0),
        column,
        scalar,
    }
}

#[divan::bench(consts = SIZES)]
fn direct_typed_collection<const ROWS: usize>(bencher: Bencher) {
    let lhs = values(ROWS);
    let rhs = values(ROWS);
    bencher.bench(|| {
        let mut output = <ArrowHost as OutputBinding<i64>>::allocate(ROWS, &mut ()).expect("benchmark fixtures must satisfy the selected signature");
        for ((slot, lhs), rhs) in output.slots().iter_mut().zip(black_box(&lhs)).zip(black_box(&rhs)) {
            slot.write(lhs.wrapping_add(*rhs));
        }
        // SAFETY: the fixed output allocation and both inputs contain exactly ROWS elements.
        unsafe { output.finish(ROWS) }
    });
}

#[divan::bench(consts = SIZES)]
fn shared_collection<const ROWS: usize>(bencher: Bencher) {
    let lhs = values(ROWS);
    let rhs = values(ROWS);
    bencher.bench(|| {
        let source = LaneZip::new(black_box(lhs.as_slice()), black_box(rhs.as_slice()));
        <ArrowHost as OutputBinding<i64>>::build_from(source, |(lhs, rhs)| lhs.wrapping_add(rhs), &mut ()).expect("benchmark fixtures must satisfy the selected signature")
    });
}

// These all-valid controls use the same output binding and allocation in both collections.
// Decoding is outside timing; shared collection includes borrowed-view length validation.
// arrow_families supplies the corresponding native-kernel and complete-invocation measurements.
#[divan::bench(consts = SIZES, args = [TextLayout::Offset32, TextLayout::Offset64, TextLayout::View])]
fn direct_text_length_collection<const ROWS: usize>(bencher: Bencher, layout: TextLayout) {
    let values = vec!["a long string outside an inline view"; ROWS];
    macro_rules! collect {
        ($native:ty, $array:expr, $lengths:expr) => {
            let baseline = arrow_string::length::length($array)
                .expect("the Arrow baseline must support the fixture layout");
            let collect = || {
                let mut output = <ArrowHost as OutputBinding<$native>>::allocate(ROWS, &mut ())
                    .expect("the benchmark output binding must allocate the fixed row count");
                for (slot, length) in output.slots()[..ROWS].iter_mut().zip($lengths) {
                    slot.write(length);
                }

                // SAFETY: each fixture array has ROWS elements. Its validated offset windows or
                // view headers yield exactly ROWS values, so the entire prefix is initialized.
                unsafe { output.finish(ROWS) }
            };
            assert_eq!(collect().to_data(), baseline.to_data());
            bencher.bench_local(collect);
        };
    }

    match layout {
        TextLayout::Offset32 => {
            let array = StringArray::from(values);
            collect!(
                i32,
                &array,
                black_box(array.value_offsets()).windows(2).map(|pair| pair[1] - pair[0])
            );
        }
        TextLayout::Offset64 => {
            let array = LargeStringArray::from(values);
            collect!(
                i64,
                &array,
                black_box(array.value_offsets()).windows(2).map(|pair| pair[1] - pair[0])
            );
        }
        TextLayout::View => {
            let array = StringViewArray::from(values);
            collect!(
                i32,
                &array,
                black_box(array.views()).iter().map(|view| *view as i32)
            );
        }
    }
}

#[divan::bench(consts = SIZES, args = [TextLayout::Offset32, TextLayout::Offset64, TextLayout::View])]
fn shared_text_length_collection<const ROWS: usize>(bencher: Bencher, layout: TextLayout) {
    let values = vec!["a long string outside an inline view"; ROWS];
    match layout {
        TextLayout::Offset32 => text_length_collection::<<ArrowHost as TextBinding>::Offset32, i32>(
            bencher,
            Arc::new(StringArray::from(values)),
            |value| value.byte_len() as i32,
        ),
        TextLayout::Offset64 => text_length_collection::<<ArrowHost as TextBinding>::Offset64, i64>(
            bencher,
            Arc::new(LargeStringArray::from(values)),
            |value| value.byte_len() as i64,
        ),
        TextLayout::View => text_length_collection::<<ArrowHost as TextBinding>::Text, i32>(
            bencher,
            Arc::new(StringViewArray::from(values)),
            |value| value.byte_len() as i32,
        ),
    }
}

fn text_length_collection<K, Out>(
    bencher: Bencher,
    array: ArrayRef,
    apply: impl Fn(K::Value<'_>) -> Out,
)
where
    K: RowKind,
    Out: Default + 'static,
    ArrowHost: InputBinding<K> + OutputBinding<Out>,
{
    let rows = array.len();
    let baseline = arrow_string::length::length(array.as_ref())
        .expect("the Arrow baseline must support the fixture layout");
    let columns = <(K,) as InputTuple<ArrowHost>>::decode(&[operand(array, false)], false, &mut ())
        .expect("the concrete text binding must decode its matching fixture layout");
    let collect = || {
        let source = <(K,) as InputTuple<ArrowHost>>::rows_source(black_box(&columns), rows)
            .expect("decoded fixture views must retain their original row count")
            .expect("the text collection fixture contains an array operand");
        <ArrowHost as OutputBinding<Out>>::build_from(source, |(value,)| apply(value), &mut ())
            .expect("the text collector must accept this infallible fixture")
    };
    assert_eq!(collect().to_data(), baseline.to_data());
    bencher.bench_local(collect);
}

#[divan::bench(consts = SIZES)]
fn shared_executor<const ROWS: usize>(bencher: Bencher) {
    let inputs = [
        operand(Arc::new(Int64Array::from(values(ROWS))), false),
        operand(Arc::new(Int64Array::from(values(ROWS))), false),
    ];
    let output = rowfn_arrow::plan(&Add::<false>, &NoOptions, &inputs.each_ref().map(|input| input.dtype.clone())).expect("benchmark fixtures must satisfy the selected signature");
    bencher.bench(|| {
        rowfn::execute::<ArrowHost, _>(&Add::<false>, &NoOptions, black_box(&inputs), ROWS, &output, &mut ()).expect("benchmark fixtures must satisfy the selected signature")
    });
}

#[divan::bench(consts = SIZES, args = [false, true])]
fn arrow_invocation<const ROWS: usize>(bencher: Bencher, scalar: bool) {
    let inputs = [
        operand(Arc::new(Int64Array::from(values(ROWS))), false),
        operand(Arc::new(Int64Array::from(if scalar { vec![1] } else { values(ROWS) })), scalar),
    ];
    let output = rowfn_arrow::plan(&Add::<false>, &NoOptions, &inputs.each_ref().map(|input| input.dtype.clone())).expect("benchmark fixtures must satisfy the selected signature");
    bencher.bench(|| rowfn_arrow::invoke(&Add::<false>, &NoOptions, black_box(&inputs), ROWS, output.clone()).expect("benchmark fixtures must satisfy the selected signature"));
}

#[divan::bench(consts = SIZES, args = [false, true])]
fn arrow_native_wrapping_add<const ROWS: usize>(bencher: Bencher, scalar: bool) {
    let lhs = Int64Array::from(values(ROWS));
    let rhs = Int64Array::from(values(ROWS));
    let constant = Int64Array::new_scalar(1);
    bencher.bench(|| {
        if scalar {
            arrow_arith::numeric::add_wrapping(black_box(&lhs), black_box(&constant)).expect("benchmark fixtures must satisfy the selected signature")
        } else {
            arrow_arith::numeric::add_wrapping(black_box(&lhs), black_box(&rhs)).expect("benchmark fixtures must satisfy the selected signature")
        }
    });
}

