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
    pub static ref CLARITY_INT: TypeSpecification = define_addon_type! {
        ClarityIntegerSigned => {
            name: "clarity_int",
            documentation: "Clarity signed integer (128 bits)",
        }
    };
    pub static ref CLARITY_PRINCIPAL: TypeSpecification = define_addon_type! {
        ClarityPrincipal => {
            name: "clarity_principal",
            documentation: "Clarity principal",
        }
    };
    pub static ref CLARITY_ASCII: TypeSpecification = define_addon_type! {
        ClarityAscii => {
            name: "clarity_ascii",
            documentation: "Clarity ASCII string",
        }
    };
    pub static ref CLARITY_UTF8: TypeSpecification = define_addon_type! {
        ClarityAscii => {
            name: "clarity_utf8",
            documentation: "Clarity UTF8 string",
        }
    };
    pub static ref CLARITY_TUPLE: TypeSpecification = define_addon_type! {
        ClarityTuple => {
            name: "clarity_tuple",
            documentation: "Clarity tuple",
        }
    };
    pub static ref CLARITY_LIST: TypeSpecification = define_addon_type! {
        ClarityList => {
            name: "clarity_list",
            documentation: "Clarity list",
        }
    };
    pub static ref CLARITY_BUFFER: TypeSpecification = define_addon_type! {
        ClarityBuffer => {
            name: "clarity_buffer",
            documentation: "Clarity buffer",
        }
    };
    pub static ref CLARITY_OK: TypeSpecification = define_addon_type! {
        ClarityBuffer => {
            name: "clarity_ok",
            documentation: "Clarity Result Ok",
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
    pub static ref STACKS_SIGNATURE: TypeSpecification = define_addon_type! {
        StacksSignature => {
            name: "stacks_signature",
            documentation: "Stacks signature",
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

pub struct ClarityIntegerSigned;
impl TypeImplementation for ClarityIntegerSigned {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}
pub struct ClarityPrincipal;
impl TypeImplementation for ClarityPrincipal {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}
pub struct ClarityAscii;
impl TypeImplementation for ClarityAscii {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}

pub struct ClarityTuple;
impl TypeImplementation for ClarityTuple {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}
pub struct ClarityList;
impl TypeImplementation for ClarityList {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}

pub struct ClarityBuffer;
impl TypeImplementation for ClarityBuffer {
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

pub struct StacksSignature;
impl TypeImplementation for StacksSignature {
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
