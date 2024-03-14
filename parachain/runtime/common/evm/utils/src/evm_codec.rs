use crate::{MayRevert, RevertReason};
use core::ops::Range;
use sp_core::{H160, H256, U256};
use sp_std::prelude::*;

pub struct Reader<'inner> {
    input: &'inner [u8],
    cursor: usize,
}

impl<'inner> Reader<'inner> {
    pub fn new(input: &'inner [u8]) -> Self {
        Self { input, cursor: 0 }
    }

    pub fn read_selector(&mut self) -> MayRevert<u32> {
        if self.cursor != 0 {
            return Err(RevertReason::NotStart);
        }

        let range = self.move_cursor(4)?;
        let data = self.input.get(range).ok_or_else(|| RevertReason::UnknownSelector)?;

        let mut buffer = [0u8; 4];
        buffer.copy_from_slice(data);
        Ok(u32::from_be_bytes(buffer))
    }

    pub fn read<T: EvmCodec>(&mut self) -> MayRevert<T> {
        T::read(self)
    }

    fn move_cursor(&mut self, len: usize) -> MayRevert<Range<usize>> {
        let start = self.cursor;
        let end = self
            .cursor
            .checked_add(len)
            .ok_or_else(|| RevertReason::CursorOverflow)?;
        self.cursor = end;
        Ok(start..end)
    }
}

pub struct Writer {
    data: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self { data: vec![] }
    }

    pub fn build(self) -> Vec<u8> {
        self.data
    }

    pub fn write_selector(mut self, selector: u32) -> Self {
        self.data.extend_from_slice(&selector.to_be_bytes().to_vec());
        self
    }

    pub fn write<T: EvmCodec>(mut self, value: T) -> Self {
        value.write(&mut self);
        self
    }
}

pub trait EvmCodec: Sized {
    fn read(reader: &mut Reader) -> MayRevert<Self>;
    fn write(self, writer: &mut Writer);
}

impl EvmCodec for H160 {
    fn read(reader: &mut Reader) -> MayRevert<Self> {
        let range = reader.move_cursor(32)?;
        let data = reader
            .input
            .get(range)
            .ok_or_else(|| RevertReason::read_out_of_bounds("address"))?;
        Ok(H160::from_slice(&data[12..32]).into())
    }

    fn write(self, writer: &mut Writer) {
        Into::<H256>::into(self).write(writer);
    }
}

impl EvmCodec for H256 {
    fn read(reader: &mut Reader) -> MayRevert<Self> {
        let range = reader.move_cursor(32)?;
        let data = reader
            .input
            .get(range)
            .ok_or_else(|| RevertReason::read_out_of_bounds("bytes32"))?;
        Ok(H256::from_slice(data))
    }

    fn write(self, writer: &mut Writer) {
        writer.data.extend_from_slice(self.as_bytes());
    }
}

impl EvmCodec for U256 {
    fn read(reader: &mut Reader) -> MayRevert<Self> {
        let range = reader.move_cursor(32)?;
        let data = reader
            .input
            .get(range)
            .ok_or_else(|| RevertReason::read_out_of_bounds("uint256"))?;
        Ok(U256::from_big_endian(data))
    }

    fn write(self, writer: &mut Writer) {
        let mut buffer = [0u8; 32];
        self.to_big_endian(&mut buffer);
        writer.data.extend_from_slice(&buffer);
    }
}

impl EvmCodec for bool {
    fn read(reader: &mut Reader) -> MayRevert<Self> {
        let h256 = H256::read(reader).map_err(|_| RevertReason::read_out_of_bounds("bool"))?;
        Ok(!h256.is_zero())
    }

    fn write(self, writer: &mut Writer) {
        let mut buffer = [0u8; 32];
        if self {
            buffer[31] = 1;
        }
        writer.data.extend_from_slice(&buffer);
    }
}

#[derive(Debug, PartialEq)]
pub struct EvmString(pub Vec<u8>);

impl EvmCodec for EvmString {
    // NOTE: we don't yet use this implementation in the precompiles
    // but it is useful for testing
    fn read(reader: &mut Reader) -> MayRevert<Self> {
        let offset: usize = U256::read(reader)
            .map_err(|_| RevertReason::read_out_of_bounds("pointer"))?
            .try_into()
            .unwrap();
        let mut inner_reader = Reader::new(reader.input.get(offset..).unwrap());

        let array_size: usize = U256::read(&mut inner_reader)
            .map_err(|_| RevertReason::read_out_of_bounds("length"))?
            .try_into()
            .unwrap();
        let range = inner_reader.move_cursor(array_size)?;

        let data = inner_reader
            .input
            .get(range)
            .ok_or_else(|| RevertReason::read_out_of_bounds("string"))?;

        Ok(Self(data.to_vec()))
    }

    fn write(self, writer: &mut Writer) {
        let value: Vec<_> = self.0.into();
        let length = value.len();

        let chunks = length / 32;
        let padded_size = match length % 32 {
            0 => chunks * 32,
            _ => (chunks + 1) * 32,
        };

        let mut value = value.to_vec();
        value.resize(padded_size, 0);

        // TODO: this won't work if encoding multiple arguments
        U256::from(32).write(writer);
        U256::from(length).write(writer);
        writer.data.extend_from_slice(&value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;

    #[test]
    fn decode_input() {
        let input = hex::decode("70a082310000000000000000000000005b38da6a701c568545dcfcb03fcb875f56beddc4").unwrap();
        let mut reader = Reader::new(&input);
        let selector = reader.read_selector().unwrap();
        assert_eq!(selector, 1889567281);
        let address = H160::read(&mut reader).unwrap();
        assert_eq!(
            address,
            H160::from_str("0x5b38da6a701c568545dcfcb03fcb875f56beddc4").unwrap()
        );
    }

    macro_rules! assert_encoding {
        ($codec:ty, $value:expr) => {{
            let data = Writer::new().write($value).build();
            let mut reader = Reader::new(&data);
            assert_eq!($value, <$codec>::read(&mut reader).unwrap());
        }};
    }

    #[test]
    fn test_encoding() {
        assert_encoding!(H160, H160([1; 20]));
        assert_encoding!(H256, H256([2; 32]));
        assert_encoding!(U256, U256::from(100));
        assert_encoding!(U256, U256::MAX);
        assert_encoding!(bool, true);
        assert_encoding!(bool, false);
        assert_encoding!(EvmString, EvmString(vec![1; 50]));
    }
}
