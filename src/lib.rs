mod aave;
mod blockchain;
mod erc20;
mod chainlink;
mod smart_wallet;

use std::collections::HashMap;

pub use crate::blockchain::BlockchainReader;
pub use crate::blockchain::HttpBlockchainReader;
pub use crate::aave::Aave;
pub use crate::chainlink::Chainlink;
pub use crate::smart_wallet::SmartWallet;
pub use crate::erc20::ERC20;


use serde::{Serialize, Deserialize};
use tera::Value;
use tera::to_value;
use tera::Result;
use num_format::{Locale, ToFormattedString};

#[macro_use]
extern crate tera;

const EXP_FACTOR:f64 = 2.35;

pub fn humanize(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let num = try_get_value!("humanize", "value", f64, value);
    match num {
        x if x < 10.0 => Ok(to_value(format!("{:.2}", x)).unwrap()),
        x if x < 100.0 => Ok(to_value(format!("{:.1}", x)).unwrap()),
        x => Ok(to_value((x.round() as i64).to_formatted_string( &Locale::fr_BE)).unwrap()),
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct Prediction {
    pub price: f64,
    pub short: f64,
    pub long: f64,
}

#[derive(Serialize, Deserialize, Debug)]
enum Direction {
    Up,
    Down
}

pub fn predict_down(current: &Prediction, base_price:f64) -> Result<Prediction> {
    predict_next(current, base_price, Direction::Down)
}

pub fn predict_up(current: &Prediction, base_price:f64) -> Result<Prediction> {
    predict_next(current, base_price, Direction::Up)

}

pub fn predict(current: &Prediction, next_price: f64) -> Result<Prediction> {
    let exp_factor = EXP_FACTOR;
    let next_factor = (current.price / next_price).powf(exp_factor);
    Ok(Prediction{
        price: next_price, 
        short: current.short * next_factor, 
        long: current.long * current.price / (next_factor * next_price)
    })

}

fn predict_next(current: &Prediction, base_price:f64, dir:Direction) -> Result<Prediction> {
    let mut next_price = get_next_price(current.price, base_price, &dir);

    let prediction = predict(current, next_price)?;
    let next_ratio =  prediction.short / prediction.long;
    // If there is ~ 3x more short than long, this is probably the starting price
    if 2.0 < next_ratio && next_ratio < 5.0 {
        next_price = match dir {                                       
            Direction::Up => next_price*3.0, 
            Direction::Down => next_price/3.0, 
        };
        return predict(current, next_price)
    }
    Ok(prediction)
}

fn get_next_price(current_price:f64, base_price:f64, dir:&Direction) -> f64 {
    let mut price = base_price;  
    match dir {  
        Direction::Up => {  
            while price > current_price {  
                price /= 3.0;  
            }  
            while price <= current_price {  
                price *= 3.0;  
            }  
            price  
        }  
        Direction::Down => {  
            while price < current_price {  
                price *= 3.0;  
            }  
            while price >= current_price {  
                price /= 3.0;  
            }  
            price                                                                                        
        }                                                                                                
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case((1200.0, 100.0, 10.0), 1000.0, (2000.0, 26.0, 23.0) ; "up normal")]
    #[test_case((1200.0, 113.221, 6.624), 1000.0, (4000.0, 4.948, 45.471) ; "up must skip")]
    fn predict_up_tests(current: (f64,f64,f64), base_price:f64, expected: (f64,f64,f64)) {
        let current = Prediction{price: current.0, short: current.1, long: current.2};
        let next = predict_up(&current, base_price).unwrap();
        assert_eq!(next.price, expected.0);
        assert_eq!(next.short.round(), expected.1.round());
        assert_eq!(next.long.round(), expected.2.round());
    }

    #[test_case((1200.0, 100.0, 10.0), 1000.0, (1000.0, 161.0, 7.0) ; "down normal")]
    #[test_case((4000.0, 4.948, 45.471), 1000.0, (1000.0, 181.886, 4.948) ; "down must skip")]
    fn predict_down_tests(current: (f64,f64,f64), base_price:f64, expected: (f64,f64,f64)) {
        let current = Prediction{price: current.0, short: current.1, long: current.2};
        let next = predict_down(&current, base_price).unwrap();
        assert_eq!(next.price, expected.0);
        assert_eq!(next.short.round(), expected.1.round());
        assert_eq!(next.long.round(), expected.2.round());
    }
}
