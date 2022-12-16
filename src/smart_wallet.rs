use super::blockchain::HttpBlockchainReader;
use ethabi::{Address, Contract};
use std::error::Error;

pub struct SmartWallet<'a> {
    blockchain_reader: &'a HttpBlockchainReader,
    address: Address,
    contract: Contract,
}
impl<'a> SmartWallet<'a> {
    pub fn new(blockchain_reader: &'a HttpBlockchainReader , smart_wallet_address:&str) -> Result<Self, Box<dyn Error>> {
        let address: Address = smart_wallet_address.parse()?;
        let smart_wallet_abi: &[u8] = include_bytes!("abi/smart_wallet.abi");
        let contract: Contract = Contract::load(smart_wallet_abi)?;
        Ok(Self {
            blockchain_reader,
            address,
            contract,
        })
    }
    pub async fn get_owner(&self) -> Result<Address, Box<dyn Error>> {
        let tokens = self
            .blockchain_reader
            .call_function(
                &self.contract,
                &self.address,
                "owner",
                &[],
            )
            .await?;

        let owner = tokens[0].clone().into_address();
        let owner = owner.unwrap();

        Ok(owner)
    }
}
