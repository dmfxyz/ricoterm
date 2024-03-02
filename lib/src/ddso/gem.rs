use std::sync::Arc;

use ethers::{
    abi::{Abi, Address},
    contract::Contract,
    providers::Middleware, types:: U256,
};
use serde_json::from_str;

pub struct Gem<T: Middleware + Clone> {
    pub address: Address,
    contract: Contract<T>,
}

impl<T: Middleware + Clone> Gem<T> {
    pub fn new(provider: &Arc<T>, address: Address) -> Self {
        let file = include_str!("./abi/gem.json");
        let abi = from_str::<Abi>(file).unwrap();

        let contract = Contract::new(address, abi, Arc::clone(provider));

        Self { address, contract }
    }

    pub async fn balance_of(&self, who: Address) -> U256 {
        self.contract.method::<Address, U256>("balanceOf", who).unwrap().call().await.unwrap()
    }

    pub async fn decimals(&self) -> U256 {
        self.contract.method::<(), U256>("decimals", ()).unwrap().call().await.unwrap()
    }
}