use bitcoin::compat::rust_bitcoin::{
    blockdata::{
        opcodes,
        script::{self, Instruction, Instructions},
    },
    taproot::TAPROOT_ANNEX_PREFIX,
    ScriptBuf, Witness,
    Transaction as RustBitcoinTransaction
};
use ink::prelude::vec::Vec;
use core::iter::Peekable;
use ink::prelude::collections::BTreeMap;

const PROTOCOL_ID: [u8; 3] = *b"ord";
const BODY_TAG: [u8; 0] = [];
const CONTENT_TYPE_TAG: [u8; 1] = [1];


#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Inscription {
    body: Option<Vec<u8>>,
    content_type: Option<Vec<u8>>,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct TransactionInscription {
    pub(crate) inscription: Inscription,
    pub(crate) tx_in_index: u32,
    pub(crate) tx_in_offset: u32,
}

impl Inscription {
    pub(crate) fn from_transaction(tx: &RustBitcoinTransaction) -> Vec<TransactionInscription> {
        let mut result = Vec::new();
        for (index, tx_in) in tx.input.iter().enumerate() {
            let Ok(inscriptions) = InscriptionParser::parse(&tx_in.witness) else { continue };

            result.extend(
                inscriptions
                    .into_iter()
                    .enumerate()
                    .map(|(offset, inscription)| TransactionInscription {
                        inscription,
                        tx_in_index: u32::try_from(index).unwrap(),
                        tx_in_offset: u32::try_from(offset).unwrap(),
                    })
                    .collect::<Vec<TransactionInscription>>(),
            )
        }

        result
    }

    pub(crate) fn into_body(self) -> Option<Vec<u8>> {
        self.body
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum InscriptionError {
    EmptyWitness,
    InvalidInscription,
    KeyPathSpend,
    NoInscription,
    Script(script::Error),
    UnrecognizedEvenField,
}

type Result<T, E = InscriptionError> = core::result::Result<T, E>;

#[derive(Debug)]
struct InscriptionParser<'a> {
    instructions: Peekable<Instructions<'a>>,
}

impl<'a> InscriptionParser<'a> {
    fn parse(witness: &Witness) -> Result<Vec<Inscription>> {
        if witness.is_empty() {
            return Err(InscriptionError::EmptyWitness);
        }

        if witness.len() == 1 {
            return Err(InscriptionError::KeyPathSpend);
        }

        let annex = witness
            .last()
            .and_then(|element| element.first().map(|byte| *byte == TAPROOT_ANNEX_PREFIX))
            .unwrap_or(false);

        if witness.len() == 2 && annex {
            return Err(InscriptionError::KeyPathSpend);
        }

        let script = witness
            .iter()
            .nth(if annex { witness.len() - 1 } else { witness.len() - 2 })
            .unwrap();

        InscriptionParser {
            instructions: ScriptBuf::from(Vec::from(script)).instructions().peekable(),
        }
        .parse_inscriptions()
        .into_iter()
        .collect()
    }

    fn parse_inscriptions(&mut self) -> Vec<Result<Inscription>> {
        let mut inscriptions = Vec::new();
        loop {
            let current = self.parse_one_inscription();
            if current == Err(InscriptionError::NoInscription) {
                break;
            }
            inscriptions.push(current);
        }

        inscriptions
    }

    fn parse_one_inscription(&mut self) -> Result<Inscription> {
        self.advance_into_inscription_envelope()?;
        let mut fields = BTreeMap::new();

        loop {
            match self.advance()? {
                Instruction::PushBytes(tag) if tag.as_bytes() == BODY_TAG.as_slice() => {
                    let mut body = Vec::new();
                    while !self.accept(&Instruction::Op(opcodes::all::OP_ENDIF))? {
                        body.extend_from_slice(self.expect_push()?);
                    }
                    fields.insert(BODY_TAG.as_slice(), body);
                    break;
                }
                Instruction::PushBytes(tag) => {
                    if fields.contains_key(tag.as_bytes()) {
                        return Err(InscriptionError::InvalidInscription);
                    }
                    fields.insert(tag.as_bytes(), self.expect_push()?.to_vec());
                }
                Instruction::Op(opcodes::all::OP_ENDIF) => break,
                _ => return Err(InscriptionError::InvalidInscription),
            }
        }

        let body = fields.remove(BODY_TAG.as_slice());
        let content_type = fields.remove(CONTENT_TYPE_TAG.as_slice());

        for tag in fields.keys() {
            if let Some(lsb) = tag.first() {
                if lsb % 2 == 0 {
                    return Err(InscriptionError::UnrecognizedEvenField);
                }
            }
        }

        Ok(Inscription { body, content_type })
    }

    fn advance(&mut self) -> Result<Instruction<'a>> {
        self.instructions
            .next()
            .ok_or(InscriptionError::NoInscription)?
            .map_err(InscriptionError::Script)
    }

    fn advance_into_inscription_envelope(&mut self) -> Result<()> {
        loop {
            if self.match_instructions(&[
                Instruction::PushBytes((&[]).into()), // represents an OF_FALSE
                Instruction::Op(opcodes::all::OP_IF),
                Instruction::PushBytes((&PROTOCOL_ID).into()),
            ])? {
                break;
            }
        }

        Ok(())
    }

    fn match_instructions(&mut self, instructions: &[Instruction]) -> Result<bool> {
        for instruction in instructions {
            if &self.advance()? != instruction {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn expect_push(&mut self) -> Result<&'a [u8]> {
        match self.advance()? {
            Instruction::PushBytes(bytes) => Ok(bytes.as_bytes()),
            _ => Err(InscriptionError::InvalidInscription),
        }
    }

    fn accept(&mut self, instruction: &Instruction) -> Result<bool> {
        match self.instructions.peek() {
            Some(Ok(next)) => {
                if next == instruction {
                    self.advance()?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Some(Err(err)) => Err(InscriptionError::Script(*err)),
            None => Ok(false),
        }
    }
}
