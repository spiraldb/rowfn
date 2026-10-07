// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright the Vortex contributors

//! Registration identity and field-aware delegation to the Arrow adapter.
//!
//! The UDF retains DataFusion fields, explicit scalar markers, and logical row counts.
//! [`rowfn_arrow`] supplies semantic planning and execution.

use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;

use arrow_schema::ArrowError;
use arrow_schema::DataType;
use arrow_schema::FieldRef;
use datafusion_common::DataFusionError;
use datafusion_common::Result;
use datafusion_common::ScalarValue;
use datafusion_expr::ColumnarValue;
use datafusion_expr::ReturnFieldArgs;
use datafusion_expr::ScalarFunctionArgs;
use datafusion_expr::ScalarUDFImpl;
use datafusion_expr::Signature;
use datafusion_expr::Volatility;
use rowfn::RowFn;
use rowfn_arrow::ArrowHost;
use rowfn_arrow::ArrowOperand;

/// A strict RowFn exposed through DataFusion's ordinary scalar UDF interface.
///
/// Clones share one registration identity. Separate constructors create distinct identities, even
/// with the same name and options. This prevents optimizer equality from merging function state
/// whose semantics have no equality contract in [`RowFn`].
///
/// Types pass through unchanged. The Arrow planner decides which types and semantic mappings the
/// function supports. Use explicit SQL casts when input types differ or a null literal is untyped.
pub struct RowFnUdf<F: RowFn<ArrowHost>> {
    registration: Arc<Registration<F>>,
}

struct Registration<F: RowFn<ArrowHost>> {
    name: String,
    signature: Signature,
    function: F,
    options: F::Options,
}

impl<F> RowFnUdf<F>
where
    F: RowFn<ArrowHost>,
    F::Options: Send + Sync + 'static,
{
    /// Bind a name, function, and fixed options to a DataFusion volatility declaration.
    ///
    /// The name must be nonempty.
    /// The caller must choose [`Volatility::Immutable`] for results fixed by arguments and options,
    /// or [`Volatility::Stable`] for results fixed within a query. The function and options must
    /// retain those semantics throughout planning and execution. Preparation remains per invocation.
    /// [`Volatility::Volatile`] is rejected because folding and retries can change callback counts.
    /// All callbacks must obey the strict [`RowFn`] contract, including its side-effect restrictions.
    ///
    /// Convert the result with [`datafusion_expr::ScalarUDF::from`] and register it with DataFusion.
    pub fn new(
        name: impl Into<String>,
        function: F,
        options: F::Options,
        volatility: Volatility,
    ) -> Result<Self> {
        let name = name.into();
        if name.is_empty() {
            return Err(DataFusionError::Plan("RowFn UDF name must be nonempty".into()));
        }
        if volatility == Volatility::Volatile {
            return Err(DataFusionError::Plan(
                "RowFn UDF volatility must be immutable or stable, got volatile".into(),
            ));
        }

        let signature = if F::ARG_NAMES.is_empty() {
            Signature::nullary(volatility)
        } else {
            Signature::any(F::ARG_NAMES.len(), volatility)
        }
        .with_parameter_names(F::ARG_NAMES.to_vec())?;

        Ok(Self {
            registration: Arc::new(Registration {
                name,
                signature,
                function,
                options,
            }),
        })
    }
}

impl<F: RowFn<ArrowHost>> Clone for RowFnUdf<F> {
    fn clone(&self) -> Self {
        Self {
            registration: Arc::clone(&self.registration),
        }
    }
}

impl<F: RowFn<ArrowHost>> fmt::Debug for RowFnUdf<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RowFnUdf")
            .field("name", &self.registration.name)
            .field("signature", &self.registration.signature)
            .finish_non_exhaustive()
    }
}

impl<F: RowFn<ArrowHost>> PartialEq for RowFnUdf<F> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.registration, &other.registration)
    }
}

impl<F: RowFn<ArrowHost>> Eq for RowFnUdf<F> {}

impl<F: RowFn<ArrowHost>> Hash for RowFnUdf<F> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.registration).hash(state);
    }
}

impl<F> ScalarUDFImpl for RowFnUdf<F>
where
    F: RowFn<ArrowHost>,
    F::Options: Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.registration.name
    }

    fn signature(&self) -> &Signature {
        &self.registration.signature
    }

    fn is_strict(&self) -> bool {
        true
    }

    fn return_type(&self, _: &[DataType]) -> Result<DataType> {
        Err(DataFusionError::Internal(
            "RowFn UDF planning requires return_field_from_args to retain field metadata".into(),
        ))
    }

    fn return_field_from_args(&self, args: ReturnFieldArgs<'_>) -> Result<FieldRef> {
        let fields = args
            .arg_fields
            .iter()
            .map(|field| field.as_ref().clone())
            .collect::<Vec<_>>();
        let registration = &self.registration;
        let output = rowfn_arrow::plan(&registration.function, &registration.options, &fields)
            .map_err(|error| match error {
                ArrowError::InvalidArgumentError(message) => {
                    DataFusionError::Plan(format!("{}: {message}", registration.name))
                }
                error => DataFusionError::from(error),
            })?;

        Ok(Arc::new(output))
    }

    fn invoke_with_args(&self, args: ScalarFunctionArgs) -> Result<ColumnarValue> {
        if args.arg_fields.len() != args.args.len() {
            return Err(DataFusionError::Internal(format!(
                "RowFn UDF requires one field per argument, got {} fields for {} arguments",
                args.arg_fields.len(),
                args.args.len(),
            )));
        }

        let scalar_result = args.number_rows != 0
            && !args.args.is_empty()
            && args.args.iter().all(|arg| matches!(arg, ColumnarValue::Scalar(_)));
        let inputs = args
            .args
            .into_iter()
            .zip(args.arg_fields)
            .map(|(value, field)| {
                let (column, scalar) = match value {
                    ColumnarValue::Scalar(value) => (value.to_array()?, true),
                    ColumnarValue::Array(array) => (array, false),
                };

                Ok(ArrowOperand {
                    column,
                    dtype: field.as_ref().clone(),
                    scalar,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let registration = &self.registration;
        let output = rowfn_arrow::invoke(
            &registration.function,
            &registration.options,
            &inputs,
            args.number_rows,
            args.return_field.as_ref().clone(),
        )
        .map_err(execution_error)?;

        // Retain the actual row domain at the Arrow boundary, including zero and nullary rows.
        // Scalar-only nonempty calls can return a scalar after Arrow validates and publishes them.
        if scalar_result {
            return ScalarValue::try_from_array(&output.array, 0).map(ColumnarValue::Scalar);
        }

        Ok(ColumnarValue::Array(output.array))
    }
}

fn execution_error(error: ArrowError) -> DataFusionError {
    match error {
        ArrowError::InvalidArgumentError(message) => DataFusionError::Execution(message),
        error => DataFusionError::from(error),
    }
}
