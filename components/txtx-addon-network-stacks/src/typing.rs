use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    types::{Type, TypeImplementation, TypeSpecification},
};

lazy_static! {
    pub static ref CLARITY_UINT: TypeSpecification = define_addon_type! {
        ClarityIntegerUnsigned => {
            name: "clarity_uint",
            documentation: "Clarity unsigned integer (128 bits)",
        }
    };
}

pub struct ClarityIntegerUnsigned;
impl TypeImplementation for ClarityIntegerUnsigned {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}
