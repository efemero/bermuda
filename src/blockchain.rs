use async_jsonrpc_client::HttpTransport;
use async_jsonrpc_client::{Params, Transport, Value};
use ethabi::{Address, Contract, Token, Uint};
use rustc_hex::FromHex;
use rustc_hex::FromHexError;
use serde_json::json;
use std::error::Error;
use std::fmt;
use web3::types::Bytes;
use web3::types::CallRequest;

use async_trait::async_trait;

#[async_trait]
pub trait BlockchainReader {
    async fn call_function(
        &self,
        contract: &Contract,
        contract_address: &Address,
        name: &str,
        params: &[Token],
    ) -> Result<Vec<Token>, Box<dyn Error>>;

    async fn get_storage_at(
        &self,
        address: &Address,
        position: Uint,
    ) -> Result<Vec<u8>, Box<dyn Error>>;
    async fn get_eth_balance(&self, address: &Address) -> Result<f64, Box<dyn Error>>;
}

pub struct HttpBlockchainReader {
    transport: HttpTransport,
}

impl HttpBlockchainReader {
    pub fn new(transport: HttpTransport) -> Result<Self, Box<dyn Error>> {
        Ok(Self { transport })
    }
}

#[async_trait]
impl BlockchainReader for HttpBlockchainReader {
    async fn call_function(
        &self,
        contract: &Contract,
        contract_address: &Address,
        name: &str,
        params: &[Token],
    ) -> Result<Vec<Token>, Box<dyn Error>> {
        let function = contract.function(name)?;
        let data = function.encode_input(params)?;

        let req = serde_json::to_value(CallRequest {
            from: None,
            to: Some(*contract_address),
            gas: None,
            gas_price: None,
            value: None,
            data: Some(Bytes(data)),
        })?;
        let params = Params::Array(vec![req, json!("latest")]);
        let response: Value = self.transport.send("eth_call", params).await?;
        let hex_str = &response.as_str().ok_or(BlockchainError(String::from(
            "cannot retrieve response from eth_call",
        )))?[2..];
        let data: Vec<u8> = hex_str.from_hex()?;
        let result = function.decode_output(&data)?;
        Ok(result)
    }

    async fn get_storage_at(
        &self,
        address: &Address,
        position: Uint,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let params = Params::Array(vec![
            Value::String(format!("{:#x}", address)),
            Value::String(format!("{:#x}", position)),
            Value::String("latest".to_string()),
        ]);
        let response: Value = self.transport.send("eth_getStorageAt", params).await?;
        let hex_str = &response.as_str().ok_or(BlockchainError(String::from(
            "cannot retrieve response from eth_call",
        )))?[2..];
        let data: Vec<u8> = hex_str.from_hex()?;
        let data: Vec<u8> = data.iter().rev().take(16).rev().cloned().collect();
        Ok(data)
    }

    async fn get_eth_balance(&self, address: &Address) -> Result<f64, Box<dyn Error>> {
        let params = Params::Array(vec![
            Value::String(format!("{:#x}", address)),
            Value::String("latest".to_string()),
        ]);
        let response: Value = self.transport.send("eth_getBalance", params).await?;
        let hex_str = &response.as_str().ok_or(BlockchainError(String::from(
            "cannot retrieve response from eth_call",
        )))?[2..];
        let result: Result<Vec<u8>, FromHexError> = hex_str.from_hex();
        let data = match result {
            Ok(data) => data,
            Err(_) => {
                let new_hex_str = ["0", hex_str].concat();
                new_hex_str.from_hex()?
            }
        };
        let data: Vec<u8> = data.iter().rev().take(16).rev().cloned().collect();
        let eth = Uint::from_big_endian(&data).as_u128() as f64 / Uint::exp10(18).as_u128() as f64;

        Ok(eth)
    }
}

#[derive(Debug, Clone)]
pub struct BlockchainError(pub String);

impl fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Error for BlockchainError {
    fn description(&self) -> &str {
        &self.0
    }
}
