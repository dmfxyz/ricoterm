use std::{fs::File, io::Read, sync::Arc};

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
        let mut file = File::open("./abi/feedbase.json").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let abi = from_str::<Abi>(&contents).unwrap();

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
