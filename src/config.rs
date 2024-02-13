use std::{collections::HashMap, fs::File, io::Read, path::Path};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct TermConfig {
    pub rpc: RpcConfig,
    pub urns: UrnsConfig,
    pub rico: Rico,
    pub ilks: IlkConfig,
}

#[derive(Deserialize, Debug)]
pub struct IlkConfig {
    pub key_mappings: HashMap<char, String>,
}

#[derive(Deserialize, Debug)]
pub struct RpcConfig {
    pub arb_rpc_url: String,
    pub refresh_seconds: u64,
}

#[derive(Deserialize, Debug)]
pub struct UrnsConfig {
    pub user_address: String,
    pub user_nickname: Option<String>,
    pub ilks: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Rico {
    pub diamond: String,
    pub feedbase: String,
    pub npfm: String,
    pub uniwrapper: String,
    pub chain_link_feed: String,
}

pub fn read_config<T: AsRef<Path>>(path: T) -> Result<TermConfig, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let config: TermConfig = toml::from_str(&contents)?;
    Ok(config)
}