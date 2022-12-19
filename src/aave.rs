use super::blockchain::HttpBlockchainReader;
use super::Loan;
use ethabi::Uint;
use ethabi::{Address, Contract, Token};
use std::error::Error;

const AAVE_ADDRESS: &str = "7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9";
pub struct Aave<'a> {
    blockchain_reader: &'a HttpBlockchainReader,
    aave_address: Address,
    aave_contract: Contract,
}
impl<'a> Aave<'a> {
    pub fn new(blockchain_reader: &'a HttpBlockchainReader ) -> Result<Self, Box<dyn Error>> {
        let aave_address: Address = AAVE_ADDRESS.parse()?;
        let aave_abi: &[u8] = include_bytes!("abi/aave.abi");
        let aave_contract: Contract = Contract::load(aave_abi)?;
        Ok(Self {
            blockchain_reader,
            aave_address,
            aave_contract,
        })
    }

    pub async fn get_loan(&self, address: &str) -> Result<Loan, Box<dyn Error>> {
         let tokens = self
            .blockchain_reader
            .call_function(
                &self.aave_contract,
                &self.aave_address,
                "getUserAccountData",
                &[Token::Address(address.parse()?)],
                )
            .await?;

        let col = tokens[0].clone().into_uint();
        let col = col.unwrap();
        let col = (col.as_u128() as f64) / Uint::exp10(18).as_u128() as f64;

        let debt = tokens[1].clone().into_uint();
        let debt = debt.unwrap();
        let debt = (debt.as_u128() as f64) / Uint::exp10(18).as_u128() as f64;

        Ok(Loan{collateral:col, debt})

   }

    pub async fn get_eth_value(&self, address: &str) -> Result<f64, Box<dyn Error>> {
        let loan = self.get_loan(address).await?;
        Ok(loan.collateral-loan.debt)
    }
}
