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
    pub static ref CLARITY_SEQUENCE: TypeSpecification = define_addon_type! {
        ClaritySequence => {
            name: "clarity_sequence",
            documentation: "Clarity sequence",
        }
    };
    pub static ref CLARITY_VALUE: TypeSpecification = define_addon_type! {
        ClarityValue => {
            name: "clarity_value",
            documentation: "Any Clarity value",
        }
    };
    // pub static ref CLARITY_CHAR_TYPE: TypeSpecification = define_addon_type! {
    //     ClaritySequence => {
    //         name: "clarity_sequence",
    //         documentation: "Clarity sequence",
    //     }
    // };
    pub static ref STACKS_SIGNED_TRANSACTION: TypeSpecification = define_addon_type! {
        StacksSignedTransaction => {
            name: "stacks_signed_transaction",
            documentation: "Stacks signed transaction",
        }
    };
    pub static ref STACKS_CONTRACT_CALL: TypeSpecification = define_addon_type! {
        StacksContractCall => {
            name: "stacks_contract_call",
            documentation: "Stacks contract call payload",
        }
    };
}

pub struct ClarityIntegerUnsigned;
impl TypeImplementation for ClarityIntegerUnsigned {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}

pub struct ClaritySequence;
impl TypeImplementation for ClaritySequence {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}

pub struct ClarityValue;
impl TypeImplementation for ClarityValue {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksSignedTransaction;
impl TypeImplementation for StacksSignedTransaction {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksContractCall;
impl TypeImplementation for StacksContractCall {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}
