use std::sync::Arc;

use ethers::{
    abi::{Abi, Address},
    contract::Contract,
    providers::Middleware,
    types::{H256, U256},
};
use serde_json::from_str;

pub struct Feedbase<T: Middleware + Clone> {
    pub address: Address,
    contract: Contract<T>,
}

impl<T: Middleware + Clone> Feedbase<T> {
    pub fn new(provider: &Arc<T>, address: Address) -> Self {
        let file = include_str!("../abi/feedbase.json");
        let abi = from_str::<Abi>(file).unwrap();

        let contract = Contract::new(address, abi, Arc::clone(provider));

        Self { address, contract }
    }

    pub async fn pull(&self, src: Address, tag: H256) -> (H256, U256) {
        self.contract
            .method::<(Address, H256), (H256, U256)>("pull", (src, tag))
            .unwrap()
            .call()
            .await
            .unwrap()
    }
}
