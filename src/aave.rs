use super::blockchain::BlockchainReader;
use ethabi::Uint;
use ethabi::{Address, Contract, Token};
use std::error::Error;

const AAVE_ADDRESS: &str = "7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9";
pub struct Aave<'a> {
    blockchain_reader: &'a dyn BlockchainReader,
    aave_address: Address,
    aave_contract: Contract,
}
impl<'a> Aave<'a> {
    pub fn new(blockchain_reader: &'a (dyn BlockchainReader + 'a)) -> Result<Self, Box<dyn Error>> {
        let aave_address: Address = AAVE_ADDRESS.parse()?;
        let aave_abi: &[u8] = include_bytes!("abi/aave.abi");
        let aave_contract: Contract = Contract::load(aave_abi)?;
        Ok(Self {
            blockchain_reader,
            aave_address,
            aave_contract,
        })
    }
    pub async fn get_eth_value(&self, address: &str) -> Result<f64, Box<dyn Error>> {
        let tokens = self
            .blockchain_reader
            .call_function(
                &self.aave_contract,
                &self.aave_address,
                "getUserAccountData",
                &[Token::Address(address.parse()?)],
            )
            .await?;

        let col = tokens[0].clone().to_uint();
        let col = col.unwrap();
        let debt = tokens[1].clone().to_uint();
        let debt = debt.unwrap();

        let eth_value =
            (col.as_u128() as f64 - debt.as_u128() as f64) / Uint::exp10(18).as_u128() as f64;

        Ok(eth_value)
    }
}
