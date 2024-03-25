use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultBalance {
    pub amount: u64,
    pub ui_amount: f64,
    pub withheld_amount: u64,
    pub ui_withheld_amount: f64,
    pub decimals: u8,
}
