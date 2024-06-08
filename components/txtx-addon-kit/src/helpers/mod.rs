pub mod fs;
pub mod hcl;

pub fn format_currency(value: u128, decimals: usize, currency: &str) -> String {
    let divisor = 10u128.pow(decimals as u32);
    let integer_part = (value / divisor) as f64;
    let decimal_part = (value % divisor) as f64 / divisor as f64;
    let formatted = format!("{:.6}", integer_part + decimal_part);
    format!("{} {}", formatted, currency)
}