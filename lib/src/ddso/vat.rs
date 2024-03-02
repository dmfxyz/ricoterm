use ethers::{abi::{Abi, ParamType, Token}, prelude::*};
use serde_json::from_str;
use std::sync::Arc;

use crate::utils::string_to_bytes32;

#[derive(Debug, Clone)]
pub struct Ilk {
    pub tart: U256,
    pub rack: U256,
    pub line: U256,
    pub dust: U256,
    pub fee: U256,
    pub rho: U256,
    pub chop: U256,
    pub hook: Address,
    pub tink: Option<U256>,
    pub inkd: Option<U256>,
}
impl From<(U256, U256, U256, U256, U256, U256, U256, Address)> for Ilk {
    fn from(data: (U256, U256, U256, U256, U256, U256, U256, Address)) -> Self {
        Ilk {
            tart: data.0,
            rack: data.1,
            line: data.2,
            dust: data.3,
            fee: data.4,
            rho: data.5,
            chop: data.6,
            hook: data.7,
            tink: None,
            inkd: None,
        }
    }
}
pub struct Vat<T: Middleware + Clone> {
    pub address: Address,
    contract: Contract<T>,
}

impl<T: Middleware + Clone> Vat<T> {
    pub fn new(provider: &Arc<T>, address: Address) -> Self {
        let file = include_str!("./abi/vat.json");
        let abi = from_str::<Abi>(file).unwrap();

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

    pub async fn ink(&self, ilk: &str, urn: Address) -> Vec<U256> {
        let ilk = string_to_bytes32(ilk);
        match ilk.eq(&string_to_bytes32(":uninft")) {
            true => {
                let raw_ilk = self
                    .contract
                    .method::<(H256, Address), Bytes>("ink", (ilk, urn))
                    .unwrap()
                    .call()
                    .await
                    .unwrap();
                let decoded_tokens = ethers::abi::decode(&[ParamType::Array(Box::new(ParamType::Uint(256)))], &raw_ilk.0).unwrap();
                let mut token_ids: Vec<U256> = Vec::new();
                if let Token::Array(values) = &decoded_tokens[0] {
                    for token in values {
                        if let Token::Uint(value) = token {
                            token_ids.push(*value);
                        }
                    }
                } else {
                    println!("Unexpected token type")
                }
                return token_ids;
            }
            false => {
                let raw_ilk = self
                    .contract
                    .method::<(H256, Address), Bytes>("ink", (ilk, urn))
                    .unwrap()
                    .call()
                    .await
                    .unwrap();
                return vec![U256::from_big_endian(&raw_ilk)];
            }
        }
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
