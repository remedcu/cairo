use super::as_single_type;
use crate::define_libfunc_hierarchy;
use crate::extensions::lib_func::{
    DeferredOutputKind, LibFuncSignature, OutputVarInfo, SierraApChange,
    SignatureOnlyGenericLibFunc, SignatureSpecializationContext,
};
use crate::extensions::type_specialization_context::TypeSpecializationContext;
use crate::extensions::types::TypeInfo;
use crate::extensions::{ConcreteType, NamedType, OutputVarReferenceInfo, SpecializationError};
use crate::ids::{ConcreteTypeId, GenericLibFuncId, GenericTypeId};
use crate::program::GenericArg;

/// A type that holds a possibly-null pointer to an object.
#[derive(Default)]
pub struct NullableType {}
impl NamedType for NullableType {
    type Concrete = NullableConcreteType;
    const ID: GenericTypeId = GenericTypeId::new_inline("Nullable");

    fn specialize(
        &self,
        context: &dyn TypeSpecializationContext,
        args: &[GenericArg],
    ) -> Result<Self::Concrete, SpecializationError> {
        let ty = as_single_type(args)?;
        Ok(NullableConcreteType { info: context.get_type_info(ty.clone())?, ty })
    }
}

pub struct NullableConcreteType {
    pub info: TypeInfo,
    pub ty: ConcreteTypeId,
}
impl ConcreteType for NullableConcreteType {
    fn info(&self) -> &TypeInfo {
        &self.info
    }
}

define_libfunc_hierarchy! {
    pub enum NullableLibFunc {
        Null(NullLibFunc),
    }, NullableConcreteLibFunc
}

/// LibFunc for creating a null object of type Nullable<T>.
#[derive(Default)]
pub struct NullLibFunc {}
impl SignatureOnlyGenericLibFunc for NullLibFunc {
    const ID: GenericLibFuncId = GenericLibFuncId::new_inline("null");

    fn specialize_signature(
        &self,
        context: &dyn SignatureSpecializationContext,
        args: &[GenericArg],
    ) -> Result<LibFuncSignature, SpecializationError> {
        let ty = as_single_type(args)?;
        Ok(LibFuncSignature::new_non_branch(
            vec![],
            vec![OutputVarInfo {
                ty: context.get_wrapped_concrete_type(NullableType::id(), ty)?,
                ref_info: OutputVarReferenceInfo::Deferred(DeferredOutputKind::Generic),
            }],
            SierraApChange::Known { new_vars_only: true },
        ))
    }
}
