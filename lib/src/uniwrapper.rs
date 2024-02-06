use std::sync::Arc;

use ethers::{
    abi::{Abi, Address},
    contract::Contract,
    providers::Middleware,
    types::U256,
};
use serde_json::from_str;

pub struct UniWrapper<T: Middleware + Clone> {
    pub address: Address,
    contract: Contract<T>,
}

impl<T: Middleware + Clone> UniWrapper<T> {
    pub fn new(provider: &Arc<T>, address: Address) -> Self {
        let file = include_str!("../abi/uniwrapper.json");
        let abi = from_str::<Abi>(file).unwrap();

        let contract = Contract::new(address, abi, Arc::clone(provider));

        Self { address, contract }
    }

    pub async fn total(&self, npfm: Address, token_id: U256, sqrt_price_x96: U256) -> (U256, U256) {
        return self
            .contract
            .method::<(Address, U256, U256), (U256, U256)>("total", (npfm, token_id, sqrt_price_x96))
            .unwrap()
            .call()
            .await
            .unwrap().into();
    }
}