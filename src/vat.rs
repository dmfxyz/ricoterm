use ethers::{abi::Abi, contract::Contract, prelude::*, types::Address};
use serde_json::from_str;
use std::{fs::File, io::Read, sync::Arc};

use crate::{utils::string_to_bytes32, Ilk};

pub struct Vat<T: Middleware + Clone> {
    pub address: Address,
    contract: Contract<T>,
}

impl<T: Middleware + Clone> Vat<T> {
    pub fn new(provider: &Arc<T>, address: Address) -> Self {
        let mut file = File::open("./abi/vat.json").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let abi = from_str::<Abi>(&contents).unwrap();

        let contract = Contract::new(address, abi, Arc::clone(provider));

        Self { address, contract }
    }

    pub async fn par(&self) -> U256 {
        return self
            .contract
            .method::<(), U256>("par", ())
            .unwrap()
            .call()
            .await
            .unwrap();
    }

    pub async fn ink(&self, ilk: &str, urn: Address) -> U256 {
        let ilk = string_to_bytes32(ilk);
        let raw_ilk = self
            .contract
            .method::<(H256, Address), Bytes>("ink", (ilk, urn))
            .unwrap()
            .call()
            .await
            .unwrap();
        U256::from_big_endian(&raw_ilk)
    }

    pub async fn urns(&self, ilk: &str, usr: Address) -> U256 {
        let ilk = string_to_bytes32(ilk);
        let urn = self
            .contract
            .method::<(H256, Address), U256>("urns", (ilk, usr))
            .unwrap()
            .call()
            .await
            .unwrap();
        urn
    }

    #[allow(dead_code)]
    pub async fn safe(&self, ilk: &str, usr: Address) -> (U256, U256, U256) {
        let ilk = string_to_bytes32(ilk);
        let safe = self
            .contract
            .method::<(H256, Address), (U256, U256, U256)>("safe", (ilk, usr))
            .unwrap()
            .call()
            .await
            .unwrap();
        safe
    }

    pub async fn geth<O: From<H256>>(&self, ilk: &str, char: &str, indexes: Vec<H256>) -> O {
        let ilk = string_to_bytes32(ilk);
        let char = string_to_bytes32(char);
        let geth = self
            .contract
            .method::<(H256, H256, Vec<H256>), H256>("geth", (ilk, char, indexes))
            .unwrap()
            .call()
            .await
            .unwrap();
        geth.into()
    }

    pub async fn ilks(&self, ilk: &str) -> Ilk {
        let ilk = string_to_bytes32(ilk);
        let _ilk = self
            .contract
            .method::<H256, (U256, U256, U256, U256, U256, U256, U256, Address)>("ilks", ilk)
            .unwrap()
            .call()
            .await
            .unwrap()
            .into();
        _ilk
    }
}
pub struct RU256(U256);
impl From<H256> for RU256 {
    fn from(h: H256) -> Self {
        RU256(U256::from_big_endian(&h.as_bytes()))
    }
}
impl From<RU256> for U256 {
    fn from(r: RU256) -> Self {
        r.0
    }
}
