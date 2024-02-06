use ethers::types::U256;


const BLN: U256 = U256([10_u64.pow(9), 0, 0, 0]);
const BLN_F64 : f64 = 10_u64.pow(9) as f64;

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
pub struct units {
    pub BLN: U256,
    pub BLN_F64: f64,
    pub WAD: U256,
    pub RAY: U256,
    pub RAD: U256,
    pub BANKYEAR: f64,
    pub X96: U256,
}

impl units {
    pub fn new() -> Self {
        units {
            BLN: BLN,
            BLN_F64: BLN_F64,
            WAD: BLN * BLN,
            RAY: BLN * BLN * BLN,
            RAD: BLN * BLN * BLN * BLN * BLN,
            BANKYEAR: ((24.0 * 365.0) + 6.0) * 3600.0,
            X96: U256::from(2).pow(U256::from(96)),
        }
    }
}
