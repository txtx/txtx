use solana_sdk::pubkey::Pubkey;

const MAINNET_USDC_PROGRAM_ID: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

pub fn get_token_by_name(network: &str, name: &str) -> Option<Pubkey> {
    match (network, name.to_ascii_lowercase().as_str()) {
        ("mainnet", "usdc") => Some(Pubkey::from_str_const(MAINNET_USDC_PROGRAM_ID)),
        _ => None,
    }
}
