#[macro_use]
extern crate clap;
extern crate tera;

use async_jsonrpc_client::HttpTransport;
use ethabi::Address;
use bermuda::{Aave, Compound, humanize, Prediction, predict, initialize_bermuda, Equalize};
use bermuda::{Chainlink, SmartWallet};
use bermuda::ERC20;
use bermuda::HttpBlockchainReader;
use std::error::Error;
use std::fs;
use tera::Context;
use tera::Tera;

const DAI_ADDRESS: &str = "6b175474e89094c44da98b954eedeac495271d0f";
const USDC_ADDRESS: &str = "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    
    let app_m = clap_app!(
        vault =>
        (version: "0.3.0")
        (author: "François Bastien <fmrbastien@gmail.com>")
        (about: "Get informations about your makerDAO vault.")
        (@subcommand show =>
         (@arg NODE: -n --node +takes_value default_value("http://localhost:8545") "Ethereum node to call" )
         (@arg SMART_WALLET: -s --sw +takes_value +required "The address of the smart wallet. This is not your ethereum address, but your smart wallet address in DefiSaver." )
        )
        (@subcommand html =>
          (@arg NODE: -n --node +takes_value default_value("http://localhost:8545") "Ethereum node to call" )
         (@arg SMART_WALLET: -s --sw +takes_value +required "The address of the smart wallet. This is not your ethereum address, but your smart wallet address in DefiSaver." )
          (@arg FILE: -f --file +takes_value default_value("index.html") "file name where to output the generated html" )
          (@arg EURUSD: -r --rate +takes_value default_value("1.06") "The price of 1€ in $" )
        ))
        .get_matches();

    match app_m.subcommand() {
        (sub_c, Some(sub_m)) => {
            let node = sub_m.value_of("NODE").unwrap();
            let transport = HttpTransport::new(node);
            let reader: HttpBlockchainReader = HttpBlockchainReader::new(transport)?;
            let aave = Aave::new(&reader)?;
            let compound = Compound::new(&reader).await?;
            let chainlink = Chainlink::new(&reader)?;
            let dai = ERC20::new(&reader, DAI_ADDRESS.parse()?)?;
            let usdc = ERC20::new(&reader, USDC_ADDRESS.parse()?)?;
            let price = chainlink.get_eth_price().await?;

            let smart_wallet = sub_m.value_of("SMART_WALLET").unwrap();
            let smart_wallet = smart_wallet.strip_prefix("0x").unwrap_or(smart_wallet);
            let smart_wallet_contract = SmartWallet::new(&reader, smart_wallet)?;
            let wallet: Address = smart_wallet_contract.get_owner().await?;
            let short = aave.get_eth_value(smart_wallet).await?;
            let long = compound.get_eth_value(smart_wallet, price).await?;
            
            let sl = aave.get_loan(smart_wallet).await?;
            let ll = compound.get_loan(smart_wallet, price).await?;
            let equalize = initialize_bermuda(sl, ll)?;

            let eth_value = reader.get_eth_balance(&wallet).await?;
            let dai_eth_value = dai.get_value(&wallet).await? / price;
            let dai_eth_value = dai_eth_value + (usdc.get_value(&wallet).await? / price);
            let total = eth_value + short + long + dai_eth_value;

            let current = Prediction{                          
                price,                                                                              
                short,                                
                long                                  
            };
            let mut predictions = Vec::new();
            for price in (500..1000).step_by(50).map(|x| x as f64){
                predictions.push(predict(&current, price)?);
            }
            for price in (1000..3000).step_by(100).map(|x| x as f64){
                predictions.push(predict(&current, price)?);
            }
            for price in (3000..5000).step_by(250).map(|x| x as f64){
                predictions.push(predict(&current, price)?);
            }
            for price in (5000..10000).step_by(500).map(|x| x as f64){
                predictions.push(predict(&current, price)?);
            }
            for price in (10000..=20000).step_by(1000).map(|x| x as f64){
                predictions.push(predict(&current, price)?);
            }
            
            let rebalance_up = match predictions.iter().find(|&p| (p.short*6.0 <= p.long)) {
                Some(prediction) => prediction.price,
                _ => 0.0,
            };
            let rebalance_down = match predictions.iter().rev().find(|&p| p.long*36.0 <= p.short) {
                Some(prediction) => prediction.price,
                _ => 0.0,
            };
            
            let mut dai_to_flash_borrow = 0.0;
            let mut eth_to_flash_borrow = 0.0;
            if equalize.short_debt_delta < 0.0 {
                eth_to_flash_borrow = -1.0 * equalize.short_debt_delta + equalize.long_col_delta;
            } else {
                dai_to_flash_borrow = -1.0 * equalize.long_debt_delta * price + equalize.short_col_delta * price;
            }

            match sub_c {
                "show" => {
                    println!("eth price: {:.2} $", price);
                    println!("");

                    println!("eth wallet: {:.2} eth ({:.2} $)", eth_value, eth_value * price);
                    println!("dai wallet: {:.2} eth ({:.2} $)", dai_eth_value, dai_eth_value * price);
                    println!("");

                    println!("Short: {:.2} eth ({:.2} $)", short, short * price);
                    println!("Long: {:.2} eth ({:.2} $)", long, long * price);
                    println!("Long + short: {:.2} eth ({:.2} $)", long+short, (long+short) * price);
                    println!("");
                    if dai_to_flash_borrow == 0.0 {
                            println!("Flash borrow {:.2} eth", eth_to_flash_borrow);
                            println!("Short (AAVE): Repay {:.2} eth of debt and withdraw {:.2} $ of collateral", (-1.0 * equalize.short_debt_delta), (-1.0 * equalize.short_col_delta * price));
                            println!("Long (Compound): Add {:.2} eth of collateral and borrow {:.2} $", equalize.long_col_delta, equalize.long_debt_delta * price);
                            println!("Sell ~ {:.2} $ for {:.2} eth", eth_to_flash_borrow * price, eth_to_flash_borrow);
                            println!("Flash repay {:.2} eth", eth_to_flash_borrow);
                    } else {
                            println!("Flash borrow {:.2} $", dai_to_flash_borrow);
                            println!("Long (Compound): Repay {:.2} $ of debt and withdraw {:.2} eth of collateral", (-1.0 *equalize.long_debt_delta * price), (-1.0 * equalize.long_col_delta));
                            println!("Short (AAVE): Add {:.2} $ of collateral and borrow {:.2} eth", equalize.short_col_delta * price, equalize.short_debt_delta);
                            let eth_to_sell = equalize.short_debt_delta - equalize.long_col_delta;
                            println!("Sell {:.2} eth for ~ {:.2} $", eth_to_sell, eth_to_sell*price);
                            println!("Flash repay {:.2} $", dai_to_flash_borrow);
                    }
                    println!("Keep ~ {:.2} $", equalize.keep * price);
                    println!("");

                    println!("Total: {:.2} eth ({:.2} $)", total, total * price);
                }
                "html" => {
                    let mut tera = match Tera::new("*.html") {
                        Ok(t) => t,
                        Err(e) => {
                            println!("Parsing error(s): {}", e);
                            ::std::process::exit(1);
                        }
                    };
                    tera.register_filter("humanize", humanize);
                    tera.add_raw_template("index.html", TEMPLATE)?;
                    let eur_usd_str = sub_m.value_of("EURUSD").unwrap();
                    let eur_usd = eur_usd_str.parse::<f64>().unwrap();
                    let usd_eur = 1.0 / eur_usd;
                    let mut context = Context::new();
                    context.insert("eth_price", &price);
                    context.insert("eth_value", &(eth_value));
                    context.insert("dai_eth_value", &(dai_eth_value));
                    context.insert("eth_short", &short);
                    context.insert("eth_long", &long);
                    context.insert("usd_eur", &usd_eur);
                    context.insert("total", &total);
                    context.insert("rebalance_down", &rebalance_down);
                    context.insert("current", &current);
                    context.insert("rebalance_up", &rebalance_up);
                    context.insert("predictions", &predictions);

                    let html = tera.render("index.html", &context)?;
                    let file_name = sub_m.value_of("FILE").unwrap();


                    fs::write(file_name, html).expect("Unable to write file");
                }
                _ => println!("{}", app_m.usage()),
            }
        }
        _ => println!("{}", app_m.usage()),
    }

    Ok(())
}



const TEMPLATE: &str = include_str!("templates/index.html");


