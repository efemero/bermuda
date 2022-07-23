use super::blockchain::BlockchainReader;
use ethabi::Uint;
use ethabi::{Address, Contract, Token};
use std::error::Error;


pub struct ERC20<'a> {
    blockchain_reader: &'a dyn BlockchainReader,
    address: Address,
    contract: Contract,
    decimals: Option<usize>,
}
impl<'a> ERC20<'a> {
    pub fn new(blockchain_reader: &'a (dyn BlockchainReader + 'a), address: Address) -> Result<Self, Box<dyn Error>> {
        let abi: &[u8] = include_bytes!("abi/erc20.abi");
        let contract: Contract = Contract::load(abi)?;
        let decimals = None;
        let erc20 = Self {
            blockchain_reader,
            address,
            contract,
            decimals,
        };
        Ok(erc20)
    }

    pub async fn get_value(&self, &address: &Address) -> Result<f64, Box<dyn Error>> {
        let tokens = self
            .blockchain_reader
            .call_function(
                &self.contract,
                &self.address,
                "balanceOf",
                &[Token::Address(address)],
            )
            .await?;

        let token = tokens[0].clone().to_uint();
        let token = token.unwrap();

        let decimals = self.get_decimals().await?;
        let token_value =
            (token.as_u128() as f64) / Uint::exp10(decimals).as_u128() as f64;

        Ok(token_value)
    }

    async fn get_decimals(&self) -> Result<usize, Box<dyn Error>> {
        let decimals = match self.decimals {
            Some(decimals) => decimals,
            None =>{
                let tokens = self
                    .blockchain_reader
                    .call_function(
                        &self.contract,
                        &self.address,
                        "decimals",
                        &[],
                        )
                    .await?;

                let token = tokens[0].clone().to_uint();
                let value = token.unwrap();
                let decimals = value.as_u32() as usize;
                decimals
            } 
        };
        Ok(decimals)
    }
}
