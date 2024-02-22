use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    types::{Typing, TypingImplementation, TypingSpecification},
};

lazy_static! {
    pub static ref CLARITY_UINT: TypingSpecification = define_addon_type! {
        ClarityIntegerUnsigned => {
            name: "clarity_uint",
            documentation: "Clarity unsigned integer (128 bits)",
        }
    };
}

pub struct ClarityIntegerUnsigned;
impl TypingImplementation for ClarityIntegerUnsigned {
    fn check(_ctx: &TypingSpecification, lhs: &Typing, rhs: &Typing) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}
