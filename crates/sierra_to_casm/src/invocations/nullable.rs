use sierra::extensions::nullable::NullableConcreteLibFunc;

use super::{CompiledInvocation, CompiledInvocationBuilder, InvocationError};
use crate::references::{try_unpack_deref, CellExpression, ReferenceExpression, ReferenceValue};

/// Builds instructions for Nullable operations.
pub fn build(
    libfunc: &NullableConcreteLibFunc,
    builder: CompiledInvocationBuilder<'_>,
) -> Result<CompiledInvocation, InvocationError> {
    match libfunc {
        NullableConcreteLibFunc::Null(_) => build_nullable_null(builder),
        NullableConcreteLibFunc::IntoNullable(_) => build_nullable_into_nullable(builder),
    }
}

fn build_nullable_null(
    builder: CompiledInvocationBuilder<'_>,
) -> Result<CompiledInvocation, InvocationError> {
    if !builder.refs.is_empty() {
        return Err(InvocationError::WrongNumberOfArguments {
            expected: 0,
            actual: builder.refs.len(),
        });
    }

    Ok(builder.build(
        vec![],
        vec![],
        [[ReferenceExpression { cells: vec![CellExpression::Immediate(0.into())] }].into_iter()]
            .into_iter(),
    ))
}

fn build_nullable_into_nullable(
    builder: CompiledInvocationBuilder<'_>,
) -> Result<CompiledInvocation, InvocationError> {
    let value = match builder.refs {
        [ReferenceValue { expression: expr_value, .. }] => try_unpack_deref(expr_value)?,
        refs => {
            return Err(InvocationError::WrongNumberOfArguments {
                expected: 1,
                actual: refs.len(),
            });
        }
    };

    // TODO(lior): Should we explicitly check that the input is not zero?
    //   The Cairo AIR guarantees no access to address 0. Is this enough?

    Ok(builder.build(
        vec![],
        vec![],
        [[ReferenceExpression { cells: vec![CellExpression::Deref(value)] }].into_iter()]
            .into_iter(),
    ))
}
