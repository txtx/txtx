use borsh::de::BorshDeserialize;
use kaigan::types::U8PrefixString;
use kaigan::types::U8PrefixVec;
use txtx_addon_kit::types::diagnostics::Diagnostic;

pub mod create_class;
pub mod create_record;

fn to_u8_prefix_string(s: &str) -> Result<U8PrefixString, Diagnostic> {
    if s.len() > u8::MAX as usize {
        return Err(diagnosed_error!("string must be less than 256 bytes"));
    }

    let mut buf = Vec::with_capacity(1 + s.len());
    buf.push(s.len() as u8);
    buf.extend_from_slice(s.as_bytes());

    U8PrefixString::try_from_slice(&buf).map_err(Into::into)
}

fn to_u8_prefix_vec(s: &str) -> Result<U8PrefixVec<u8>, Diagnostic> {
    if s.len() > u8::MAX as usize {
        return Err(diagnosed_error!("string must be less than 256 bytes"));
    }

    let mut buf = Vec::with_capacity(1 + s.len());
    buf.push(s.len() as u8);
    buf.extend_from_slice(s.as_bytes());

    U8PrefixVec::try_from_slice(&buf).map_err(Into::into)
}
