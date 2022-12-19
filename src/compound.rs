use super::blockchain::HttpBlockchainReader;
use super::Loan;
use ethabi::Uint;
use ethabi::{Address, Contract, Token};
use std::error::Error;

const COMPOUND_ADDRESS: &str = "c3d688B66703497DAA19211EEdff47f25384cdc3";
const WETH_ADDRESS: &str = "C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
pub struct Compound<'a> {
    blockchain_reader: &'a HttpBlockchainReader,
    compound_address: Address,
    compound_contract: Contract,
}
impl<'a> Compound<'a> {
    pub async fn new(blockchain_reader: &'a HttpBlockchainReader ) -> Result<Compound<'a>, Box<dyn Error>> {
        let compound_address: Address = COMPOUND_ADDRESS.parse()?;
        let compound_abi: &[u8] = include_bytes!("abi/cUSDCv3.abi");
        let compound_contract: Contract = Contract::load(compound_abi)?;
        Ok(Self {
            blockchain_reader,
            compound_address,
            compound_contract,
        })
    }

    pub async fn get_eth_col(&self, owner_address: &str) -> Result<f64, Box<dyn Error>> {

        let tokens = self
            .blockchain_reader
            .call_function(
                &self.compound_contract,
                &self.compound_address,
                "userCollateral",
                &[Token::Address(owner_address.parse()?), Token::Address(WETH_ADDRESS.parse()?)],
                )
            .await?;

        let col = tokens[0].clone().into_uint();
        let col = col.unwrap();

        let eth_value =
            (col.as_u128() as f64) / Uint::exp10(18).as_u128() as f64;

        Ok(eth_value)
    }

    pub async fn get_eth_debt(&self, owner_address: &str, eth_price: f64) -> Result<f64, Box<dyn Error>> {

        let tokens = self
            .blockchain_reader
            .call_function(
                &self.compound_contract,
                &self.compound_address,
                "borrowBalanceOf",
                &[Token::Address(owner_address.parse()?)],
                )
            .await?;

        let debt = tokens[0].clone().into_uint();
        let debt = debt.unwrap();

        let eth_value =
            (debt.as_u128() as f64) / (Uint::exp10(6).as_u128() as f64 * eth_price);

        Ok(eth_value)
    }

    pub async fn get_loan(&self, owner_address: &str, eth_price: f64) -> Result<Loan, Box<dyn Error>> {
        let col =  self.get_eth_col(owner_address).await?;
        let debt = self.get_eth_debt(owner_address, eth_price).await?;
        Ok(Loan{collateral:col, debt})
    }

    pub async fn get_eth_value(&self, owner_address: &str, eth_price: f64) -> Result<f64, Box<dyn Error>> {
        let col = self.get_eth_col(owner_address).await?;
        let debt = self.get_eth_debt(owner_address, eth_price).await?;

        let eth_value = col - debt;

        Ok(eth_value)
    }
}
