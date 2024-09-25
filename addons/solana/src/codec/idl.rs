use anchor_lang_idl::types::{Idl, IdlInstruction, IdlType};
use serde::de::value;
use txtx_addon_kit::{helpers::fs::FileLocation, types::types::Value};

pub struct IdlRef {
    pub idl: Idl,
    pub location: FileLocation,
}

impl IdlRef {
    pub fn new(location: FileLocation) -> Result<Self, String> {
        let idl_str =
            location.read_content_as_utf8().map_err(|e| format!("unable to read idl: {e}"))?;
        let idl = serde_json::from_str(&idl_str).map_err(|e| format!("invalid idl: {e}"))?;
        Ok(Self { idl, location })
    }

    pub fn get_discriminator(&self, instruction_name: &str) -> Result<Vec<u8>, String> {
        self.get_instruction(instruction_name).map(|i| i.discriminator.clone())
    }

    pub fn get_instruction(&self, instruction_name: &str) -> Result<&IdlInstruction, String> {
        self.idl
            .instructions
            .iter()
            .find(|i| i.name == instruction_name)
            .ok_or_else(|| format!("instruction not found: {instruction_name}"))
    }
    pub fn get_encoded_args(
        &self,
        instruction_name: &str,
        args: Vec<Value>,
    ) -> Result<Vec<u8>, String> {
        let instruction = self.get_instruction(instruction_name)?;
        if args.len() != instruction.args.len() {
            return Err(format!(
                "{} arguments provided for instruction {}, which expects {} arguments",
                args.len(),
                instruction_name,
                instruction.args.len()
            ));
        }
        if args.is_empty() {
            return Ok(vec![]);
        }
        let mut encoded_args = vec![];
        for (user_arg_idx, arg) in args.iter().enumerate() {
            let idl_arg = instruction.args.get(user_arg_idx).unwrap();
            let mut encoded_arg = encode_value_to_idl_type(arg, &idl_arg.ty)?;
            encoded_args.append(&mut encoded_arg);
        }
        Ok(encoded_args)
    }
}

pub fn encode_value_to_idl_type(value: &Value, idl_type: &IdlType) -> Result<Vec<u8>, String> {
    match idl_type {
        IdlType::Bool => {
            value.as_bool().and_then(|b| Some(borsh::to_vec(&b).unwrap())).ok_or(format!(""))
        }
        IdlType::U8 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as u8)).unwrap()))
            .ok_or(format!("")),
        IdlType::I8 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as i8)).unwrap()))
            .ok_or(format!("")),
        IdlType::U16 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as u16)).unwrap()))
            .ok_or(format!("")),
        IdlType::I16 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as i16)).unwrap()))
            .ok_or(format!("")),
        IdlType::U32 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as u32)).unwrap()))
            .ok_or(format!("")),
        IdlType::I32 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as i32)).unwrap()))
            .ok_or(format!("")),
        IdlType::F32 => value
            .as_float()
            .and_then(|i| Some(borsh::to_vec(&(i as f32)).unwrap()))
            .ok_or(format!("")),
        IdlType::U64 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as u64)).unwrap()))
            .ok_or(format!("")),
        IdlType::I64 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as i64)).unwrap()))
            .ok_or(format!("")),
        IdlType::F64 => value
            .as_float()
            .and_then(|i| Some(borsh::to_vec(&(i as f64)).unwrap()))
            .ok_or(format!("")),
        IdlType::U128 => todo!(),
        IdlType::I128 => todo!(),
        IdlType::U256 => todo!(),
        IdlType::I256 => todo!(),
        IdlType::Bytes => todo!(),
        IdlType::String => {
            value.as_string().and_then(|s| Some(borsh::to_vec(&s).unwrap())).ok_or(format!(""))
        }
        IdlType::Pubkey => todo!(),
        IdlType::Option(idl_type) => todo!(),
        IdlType::Vec(idl_type) => todo!(),
        IdlType::Array(idl_type, idl_array_len) => todo!(),
        IdlType::Defined { name, generics } => todo!(),
        IdlType::Generic(_) => todo!(),
        _ => todo!(),
    }
}
