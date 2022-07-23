#[macro_use]
extern crate clap;
extern crate tera;

use async_jsonrpc_client::HttpTransport;
use ethabi::Address;
use bermuda::{Aave, humanize, Prediction, predict_up, predict_down, predict};
use bermuda::BlockchainReader;
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
         (@arg SHORT_SW: -s --short +takes_value +required "The address of the short smart wallet. This is not your ethereum address, but your smart wallet address in DefiSaver." )
         (@arg LONG_SW: -l --long +takes_value +required "The address of the long smart wallet. This is not your ethereum address, but your smart wallet address in DefiSaver." )
        )
        (@subcommand html =>
          (@arg NODE: -n --node +takes_value default_value("http://localhost:8545") "Ethereum node to call" )
         (@arg SHORT_SW: -s --short +takes_value +required "The address of the short smart wallet. This is not your ethereum address, but your smart wallet address in DefiSaver." )
         (@arg LONG_SW: -l --long +takes_value +required +required "The address of the long smart wallet. This is not your ethereum address, but your smart wallet address in DefiSaver." )
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
            let chainlink = Chainlink::new(&reader)?;
            let dai = ERC20::new(&reader, DAI_ADDRESS.parse()?)?;
            let usdc = ERC20::new(&reader, USDC_ADDRESS.parse()?)?;
            let price = chainlink.get_eth_price().await?;

            let smart_wallet_short = sub_m.value_of("SHORT_SW").unwrap();
            let smart_wallet_short = smart_wallet_short.strip_prefix("0x").unwrap_or(smart_wallet_short);
            let smart_wallet_short_contract = SmartWallet::new(&reader, smart_wallet_short)?;
            let wallet_short: Address = smart_wallet_short_contract.get_owner().await?;
            let short = aave.get_eth_value(smart_wallet_short).await?;

            let smart_wallet_long = sub_m.value_of("LONG_SW").unwrap();
            let smart_wallet_long_contract = SmartWallet::new(&reader, smart_wallet_short)?;
            let smart_wallet_long = smart_wallet_long.strip_prefix("0x").unwrap_or(smart_wallet_long);
            let wallet_long: Address = smart_wallet_long_contract.get_owner().await?;
            let long = aave.get_eth_value(smart_wallet_long).await?;
            
            let eth_value_short = reader.get_eth_balance(&wallet_short).await?;
            let dai_eth_value_short = dai.get_value(&wallet_short).await? / price;
            let dai_eth_value_short = dai_eth_value_short + (usdc.get_value(&wallet_short).await? / price);
            let eth_value_long = reader.get_eth_balance(&wallet_long).await?;
            let dai_eth_value_long = dai.get_value(&wallet_long).await? / price;
            let dai_eth_value_long = dai_eth_value_long + (usdc.get_value(&wallet_long).await? / price);
            let total = eth_value_short + eth_value_long + short + long + dai_eth_value_short + dai_eth_value_long;

            let current = Prediction{                          
                price,                                                                              
                short,                                
                long                                  
            };
            let prediction_up = predict_up(&current, 2800.0)?;
            let prediction_down = predict_down(&current, 2800.0)?;
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

            match sub_c {
                "show" => {
                    println!("eth price: {:.2} $", price);
                    println!("eth wallet short: {:.2} eth ({:.2} $)", eth_value_short, eth_value_short * price);
                    println!("dai wallet short: {:.2} eth ({:.2} $)", dai_eth_value_short, dai_eth_value_short * price);
                    println!("eth wallet long: {:.2} eth ({:.2} $)", eth_value_long, eth_value_long * price);
                    println!("dai wallet long: {:.2} eth ({:.2} $)", dai_eth_value_long, dai_eth_value_long * price);
                    println!("Short: {:.2} eth ({:.2} $)", short, short * price);
                    println!("Long: {:.2} eth ({:.2} $)", long, long * price);
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
                    context.insert("eth_value", &(eth_value_short + eth_value_long));
                    context.insert("dai_eth_value", &(dai_eth_value_short + dai_eth_value_long));
                    context.insert("eth_short", &short);
                    context.insert("eth_long", &long);
                    context.insert("usd_eur", &usd_eur);
                    context.insert("total", &total);
                    context.insert("prediction_down", &prediction_down);
                    context.insert("current", &current);
                    context.insert("prediction_up", &prediction_up);
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


