use std::sync::Arc;

use ethers::{
    abi::{Abi, Address}, contract::Contract, providers::Middleware, types::{H256, U256},
};
use serde_json::from_str;

pub struct Vox<T: Middleware + Clone> {
    pub address: Address,
    contract: Contract<T>,
}

impl<T: Middleware + Clone> Vox<T> {
    pub fn new(provider: &Arc<T>, address: Address) -> Self {
        let file = include_str!("./abi/vox.json");
        let abi = from_str::<Abi>(file).unwrap();

        let contract = Contract::new(address, abi, Arc::clone(provider));

        Self { address, contract }
    }

    pub async fn tip(&self) -> (Address, H256) {
        let (src, tag) = self
            .contract
            .method::<(), (Address, H256)>("tip", ())
            .unwrap()
            .call()
            .await
            .unwrap();
        (src, tag)
    }

    pub async fn way(&self) -> U256 {
        let way = self
            .contract
            .method::<(), U256>("way", ())
            .unwrap()
            .call()
            .await
            .unwrap();
        way
    }

    pub async fn tau(&self) -> U256 {
        let tau = self
            .contract
            .method::<(), U256>("tau", ())
            .unwrap()
            .call()
            .await
            .unwrap();
        tau
    }

    pub async fn how(&self) -> U256 {
        let how = self
            .contract
            .method::<(), U256>("how", ())
            .unwrap()
            .call()
            .await
            .unwrap();
        how
    }
}
