use super::blockchain::HttpBlockchainReader;
use ethabi::Uint;
use ethabi::{Address, Contract};
use std::error::Error;

const CHAINLINK_ADDRESS: &str = "773616e4d11a78f511299002da57a0a94577f1f4";
pub struct Chainlink<'a> {
    blockchain_reader: &'a HttpBlockchainReader,
    chainlink_address: Address,
    chainlink_contract: Contract,
}
impl<'a> Chainlink<'a> {
    pub fn new(blockchain_reader: &'a HttpBlockchainReader ) -> Result<Self, Box<dyn Error>> {
        let chainlink_address: Address = CHAINLINK_ADDRESS.parse()?;
        let chainlink_abi: &[u8] = include_bytes!("abi/chainlink.abi");
        let chainlink_contract: Contract = Contract::load(chainlink_abi)?;
        Ok(Self {
            blockchain_reader,
            chainlink_address,
            chainlink_contract,
        })
    }
    pub async fn get_eth_price(&self) -> Result<f64, Box<dyn Error>> {
        let tokens = self
            .blockchain_reader
            .call_function(
                &self.chainlink_contract,
                &self.chainlink_address,
                "latestAnswer",
                &[],
            )
            .await?;

        let price = tokens[0].clone().into_int();

        let price = price.unwrap();

        let eth_price = 1.0 / (price.as_u128() as f64 / Uint::exp10(18).as_u128() as f64);

        Ok(eth_price)
    }
}
