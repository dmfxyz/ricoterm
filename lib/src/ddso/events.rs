use ethers::{types::{Log, H160, H256, I256, U256, U64}, utils::keccak256};
extern crate lazy_static;



lazy_static::lazy_static ! {
    pub static ref NEW_PALM_2_SIG: H256 = H256::from(keccak256("NewPalm2(bytes32,bytes32,bytes32,bytes32)"));
}

pub struct NewPalm2 {
    pub block_number: U64,
    pub act: H256,
    pub ilk: H256,
    pub usr: H160,
    pub val: I256,
}

impl From<Log> for NewPalm2 {
    fn from(log: Log) -> Self {
        let block_number = log.block_number.unwrap();
        let act = log.topics[1];
        let ilk = log.topics[2];
        let usr = H160::from_slice(&log.topics[3].as_bytes()[0..20]);
        let val: I256 = I256::from_raw(U256::from_big_endian(&log.data[..]));
        Self { block_number, act, ilk, usr, val}
    }
}
pub trait IntoNewPalm2Vec {
    fn into_new_palm2_vec(self) -> Vec<NewPalm2>;
}

impl IntoNewPalm2Vec for Vec<Log> {
    fn into_new_palm2_vec(self) -> Vec<NewPalm2> {
        self.into_iter().map(|log| NewPalm2::from(log)).collect()
    }
}