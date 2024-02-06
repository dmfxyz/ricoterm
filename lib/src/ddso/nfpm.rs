use std::sync::Arc;

use ethers::{
    abi::{Abi, Address},
    contract::Contract,
    providers::Middleware,
    types::U256,
};
use serde_json::from_str;

#[derive(Debug)]
pub struct PositionsData {
    pub nonce: U256,
    pub operator: Address,
    pub token0: Address,
    pub token1: Address,
    pub fee: U256,
    pub tick_lower: U256,
    pub tick_upper: U256,
    pub liquidity: U256,
    pub fee_growth_inside_0_last_x128: U256,
    pub fee_growth_inside_1_last_x128: U256,
    pub tokens_owed_0: U256,
    pub tokens_owed_1: U256,
}

impl From<(U256, Address, Address, Address, U256, U256, U256, U256, U256, U256, U256, U256)> for PositionsData {
    fn from(data: (U256, Address, Address, Address, U256, U256, U256, U256, U256, U256, U256, U256)) -> Self {
        PositionsData {
            nonce: data.0,
            operator: data.1,
            token0: data.2,
            token1: data.3,
            fee: data.4,
            tick_lower: data.5,
            tick_upper: data.6,
            liquidity: data.7,
            fee_growth_inside_0_last_x128: data.8,
            fee_growth_inside_1_last_x128: data.9,
            tokens_owed_0: data.10,
            tokens_owed_1: data.11,
        }
    }
}

pub struct NPFM<T: Middleware + Clone> {
    pub address: Address,
    contract: Contract<T>,
}

impl<T: Middleware + Clone> NPFM<T> {
    pub fn new(provider: &Arc<T>, address: Address) -> Self {
        let file = include_str!("./abi/npfm.json");
        let abi = from_str::<Abi>(file).unwrap();

        let contract = Contract::new(address, abi, Arc::clone(provider));

        Self { address, contract }
    }

    pub async fn positions(&self, token_id: U256) -> PositionsData {
        return self
            .contract
            .method::<U256, (U256, Address, Address, Address, U256, U256, U256, U256, U256, U256, U256, U256)>("positions", token_id)
            .unwrap()
            .call()
            .await
            .unwrap().into();
    }
}