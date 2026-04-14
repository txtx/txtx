use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_pubkey::Pubkey;

use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{diagnostics::Diagnostic, frontend::LogDispatcher, stores::ValueStore, types::Value},
};
use txtx_addon_network_svm_types::SvmValue;

use super::surfnet_update::SurfnetAccountUpdate;
use crate::constants::SET_PROGRAM_AUTHORITY;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfpoolSetProgramAuthority {
    #[serde(skip)]
    pub program_id: Pubkey,
    #[serde(skip)]
    pub authority: Option<Pubkey>,
}

impl SurfpoolSetProgramAuthority {
    pub fn new(program_id: Pubkey, authority: Option<Pubkey>) -> Self {
        Self { program_id, authority }
    }

    pub fn from_map(map: &mut IndexMap<String, Value>) -> Result<Self, Diagnostic> {
        let some_program_id = map.swap_remove("program_id");
        let program_id = some_program_id
            .map(|p| {
                SvmValue::to_pubkey(&p)
                    .map_err(|e| diagnosed_error!("invalid 'program_id' field: {e}"))
            })
            .ok_or_else(|| diagnosed_error!("missing required 'program_id'"))??;

        let some_authority = map.swap_remove("authority");
        let authority = some_authority
            .map(|p| {
                SvmValue::to_pubkey(&p)
                    .map_err(|e| diagnosed_error!("invalid 'authority' field: {e}"))
            })
            .transpose()?;

        Ok(Self::new(program_id, authority))
    }

    pub fn parse_value_store(values: &ValueStore) -> Result<Vec<Self>, Diagnostic> {
        let mut set_authorities = vec![];

        let set_program_authority_data = values
            .get_value(SET_PROGRAM_AUTHORITY)
            .map(|v| {
                v.as_map()
                    .ok_or_else(|| diagnosed_error!("'set_program_authority' must be a map type"))
            })
            .transpose()?;

        let Some(set_program_authority_data) = set_program_authority_data else {
            return Ok(vec![]);
        };

        let mut set_program_authority_data = set_program_authority_data
            .iter()
            .map(|i| {
                i.as_object()
                    .map(|o| o.clone())
                    .ok_or(diagnosed_error!("'set_program_authority' must be a map type"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        for (i, set_program_authority) in set_program_authority_data.iter_mut().enumerate() {
            let prefix = format!("failed to parse `set_program_authority` map #{}", i + 1);
            let account = SurfpoolSetProgramAuthority::from_map(set_program_authority)
                .map_err(|e| diagnosed_error!("{prefix}: {e}"))?;

            set_authorities.push(account);
        }

        Ok(set_authorities)
    }

}

impl SurfnetAccountUpdate for SurfpoolSetProgramAuthority {
    fn rpc_method() -> &'static str {
        "surfnet_setProgramAuthority"
    }

    fn to_request_params(&self) -> serde_json::Value {
        let program_id = json![self.program_id.to_string()];
        let authority = json![self.authority.map(|a| a.to_string())];
        json!(vec![program_id, authority])
    }

    fn update_status(&self, logger: &LogDispatcher, index: usize, total: usize) {
        logger.success_info(
            "Program Authority Set",
            &format!("Set program authority #{}/{}", index + 1, total,),
        );
    }
}
