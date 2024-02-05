use std::default;

use ethers::types::U256;
pub struct UrnData {
    pub ink_name: String,
    pub ink: U256,
    pub art: U256,
    pub loan: U256,
    pub value: U256,
    pub safety: f64,
}

impl UrnData {
    pub fn new() -> Self {
        Self {
            ink_name: String::from(""),
            ink: U256::zero(),
            art: U256::zero(),
            loan: U256::zero(),
            value: U256::zero(),
            safety: 0.0,
        }
    }
}
impl default::Default for UrnData {
    fn default() -> Self {
        Self::new()
    }
}