use crate::*;
use ink::prelude::{string::String, vec::Vec};

pub struct Mint {
    pub ticker: String,
    pub amount: u64,
    pub src: String,
}

/// Very rudimentary json parsing. Expects a strict ordering of the fields.
pub fn parse_mint(input: &str) -> Option<Mint> {
    let parsed = parse_json(&input).ok()?;
    let obj = parsed
        .as_object()?
        .into_iter()
        .filter_map(|(field, value)| {
            Some((
                field.iter().cloned().collect::<String>(),
                value.as_string()?.iter().cloned().collect::<String>(),
            ))
        })
        .collect::<Vec<_>>();

    if obj.len() != 5 {
        return None;
    }


    if &obj[0].0 != "p" || &obj[0].1 != "brc-21" {
        return None;
    }

    if &obj[1].0 != "op" || &obj[1].1 != "mint" {
        return None;
    }

    if &obj[2].0 != "tick" {
        return None;
    }
    let ticker = obj[2].1.clone();

    if &obj[3].0 != "amt" {
        return None;
    }
    let amount = parse_json(&obj[3].1).ok()?.as_number()?.integer;

    if &obj[4].0 != "src" {
        return None;
    }
    let src = obj[4].1.clone();

    Some(Mint { ticker, amount, src })
}

pub struct Redeem {
    pub ticker: String,
    pub amount: u64,
    pub dest: String,
    pub account: AccountId,
}

/// Very rudimentary json parsing. Expects a strict ordering of the fields
pub fn parse_redeem(input: &str) -> Option<Redeem> {
    let parsed = parse_json(&input).ok()?;
    let obj = parsed
        .as_object()?
        .into_iter()
        .filter_map(|(field, value)| {
            Some((
                field.iter().cloned().collect::<String>(),
                value.as_string()?.iter().cloned().collect::<String>(),
            ))
        })
        .collect::<Vec<_>>();

    if obj.len() != 6 {
        return None;
    }

    if &obj[0].0 != "p" || &obj[0].1 != "brc-21" {
        return None;
    }

    if &obj[1].0 != "op" || &obj[1].1 != "redeem" {
        return None;
    }

    if &obj[2].0 != "tick" {
        return None;
    }
    let ticker = obj[2].1.clone();

    if &obj[3].0 != "amt" {
        return None;
    }
    let amount = parse_json(&obj[3].1).ok()?.as_number()?.integer;

    if &obj[4].0 != "dest" {
        return None;
    }
    let dest = obj[4].1.clone();

    if &obj[5].0 != "acc" {
        return None;
    }
    let account_str = obj[5].1.clone();

    let mut account_bytes = [0u8; 32];
    hex::decode_to_slice(account_str, &mut account_bytes as &mut [u8]).ok()?;
    let account = TryFrom::try_from(account_bytes).ok()?;

    Some(Redeem {
        ticker,
        amount,
        dest,
        account,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mint() {
        parse_mint(
            r#"{
                "p": "brc-21",
                "op": "mint",
                "tick": "INTR",
                "amt": "100",
                "src": "INTERLAY"
            }"#,
        )
        .unwrap();
    }
    #[test]
    fn test_parse_redeem() {
        parse_redeem(
            r#"{
                    "p": "brc-21",
                    "op": "redeem",
                    "tick": "INTR",
                    "amt": "50",
                    "dest": "INTERLAY",
                    "acc": "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
                }"#,
        )
        .unwrap();
    }
}
