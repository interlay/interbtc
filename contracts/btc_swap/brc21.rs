use crate::*;

use serde::Deserializer;
use serde_json::Value;
use serde::Deserialize;
fn deserialize_quoted_integer<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    match Value::deserialize(deserializer)? {
        Value::String(s) => {
            serde_json::from_str::<u64>(&s).map_err(|_| serde::de::Error::custom("wrong type"))
        }
        _ => Err(serde::de::Error::custom("wrong type")),
    }
}

#[derive(serde::Deserialize, Debug, PartialEq)]
#[serde(tag="p", rename = "brc-21")]
pub struct Brc21<'a> {
    #[serde(flatten)]
    pub op: Brc21Operation<'a>,
    pub tick: &'a str,
}

#[derive(serde::Deserialize, Debug, PartialEq)]
#[serde(tag="op")]
#[serde(rename_all = "camelCase")]
pub enum Brc21Operation<'a> {
    Deploy {
        #[serde(deserialize_with = "deserialize_quoted_integer")]
        max: u64,
        src: &'a str,
        id: &'a str,
    },
    Mint {
        #[serde(deserialize_with = "deserialize_quoted_integer")]
        amt: u64,
        src: &'a str,
    },
    Transfer{
        #[serde(deserialize_with = "deserialize_quoted_integer")]
        amt: u64,
    },
    Redeem {
        #[serde(deserialize_with = "deserialize_quoted_integer")]
        amt: u64,
        dest:&'a str,
        acc: &'a str,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::parser::parse_transaction;
    use crate::ord::Inscription;

    #[test]
    fn test_inscription() {
        // txid b61b0172d95e266c18aea0c624db987e971a5d6d4ebc2aaed85da4642d635735
        let raw_tx = "01000000000101ace8423f874c95f5f9042d7cda6b9f0727251f3059ef827f373a56831cc621a30000000000fdffffff01102700000000000022512037679ea62eab55ebfd442c53c4ad46b6b75e45d8a8fa9cb31a87d0df268b029a03406c00eb3c4d35fedd257051333b4ca81d1a25a37a9af4891f1fec2869edd56b14180eafbda8851d63138a724c9b15384bc5f0536de658bd294d426a36212e6f08a5209e2849b90a2353691fccedd467215c88eec89a5d0dcf468e6cf37abed344d746ac0063036f7264010118746578742f706c61696e3b636861727365743d7574662d38004c5e7b200a20202270223a20226272632d3230222c0a2020226f70223a20226465706c6f79222c0a2020227469636b223a20226f726469222c0a2020226d6178223a20223231303030303030222c0a2020226c696d223a202231303030220a7d6821c19e2849b90a2353691fccedd467215c88eec89a5d0dcf468e6cf37abed344d74600000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let interlay_transaction = parse_transaction(&tx_bytes).unwrap();

        let rust_bitcoin_transaction = interlay_transaction.to_rust_bitcoin().unwrap();
        let inscriptions = Inscription::from_transaction(&rust_bitcoin_transaction);
        let expected = "{ \n  \"p\": \"brc-20\",\n  \"op\": \"deploy\",\n  \"tick\": \"ordi\",\n  \"max\": \"21000000\",\n  \"lim\": \"1000\"\n}";
        
        assert_eq!(inscriptions.len(), 1);
        let body_bytes = inscriptions[0].clone().inscription.into_body().unwrap();
        let body = std::str::from_utf8(&body_bytes).unwrap();
        assert_eq!(body, expected);
    }

    #[test]
    fn test_parse_transfer() {
        let s = r#"{"p": "brc-21", "a": "12", "tick": "ticker", "op": "transfer", "amt": "25"}"#;
        let parsed: Brc21 = serde_json::from_str(s).unwrap();
        let expected = Brc21{
            op: Brc21Operation::Transfer { amt: 25 },
            tick: "ticker"
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_redeem() {
        let s = r#"{"p": "brc-21", "a": "12", "tick": "ticker", "op": "redeem", "acc": "someAccount", "amt": "10", "dest": "someDest"}"#;
        let parsed: Brc21 = serde_json::from_str(s).unwrap();
        let expected = Brc21{
            op: Brc21Operation::Redeem { acc: "someAccount", amt: 10, dest: "someDest" },
            tick: "ticker"
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_mint() {
        let s = r#"{"p": "brc-21", "a": "12", "tick": "ticker", "op": "mint", "src": "someSource", "amt": "10"}"#;
        let parsed: Brc21 = serde_json::from_str(s).unwrap();
        let expected = Brc21{
            op: Brc21Operation::Mint { amt: 10, src: "someSource" },
            tick: "ticker"
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_deploy() {
        let s = r#"{"p": "brc-21", "a": "12", "tick": "ticker", "op": "deploy","id":"myId", "src": "someSource", "max": "10"}"#;
        let parsed: Brc21 = serde_json::from_str(s).unwrap();
        let expected = Brc21{
            op: Brc21Operation::Deploy { id: "myId", max: 10, src: "someSource" },
            tick: "ticker"
        };
        assert_eq!(parsed, expected);
    }
}
