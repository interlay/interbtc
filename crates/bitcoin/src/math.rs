use sp_core::U256;

pub trait GetCompact {
    fn get_compact(self) -> Option<u32>;
}

// https://github.com/bitcoin/bitcoin/blob/7fcf53f7b4524572d1d0c9a5fdc388e87eb02416/src/arith_uint256.cpp#L223
impl GetCompact for U256 {
    fn get_compact(self) -> Option<u32> {
        let mut size = (self.bits() + 7) / 8;
        let mut compact = if size <= 3 {
            (self.low_u64() << (8 * (3 - size))) as u32
        } else {
            let bn = self >> (8 * (size - 3));
            bn.low_u32()
        };

        if (compact & 0x00800000) != 0 {
            compact >>= 8;
            size += 1;
        }

        if !(compact & !0x007fffff == 0) {
            None
        } else if !(size < 256) {
            None
        } else {
            Some(compact | (size << 24) as u32)
        }
    }
}

pub trait SetCompact {
    fn set_compact(value: u32) -> Option<Self>
    where
        Self: Sized;
}

// https://github.com/bitcoin/bitcoin/blob/7fcf53f7b4524572d1d0c9a5fdc388e87eb02416/src/arith_uint256.cpp#L203
impl SetCompact for U256 {
    fn set_compact(compact: u32) -> Option<Self>
    where
        Self: Sized,
    {
        let size = compact >> 24;
        let mut word = compact & 0x007fffff;

        let value = if size <= 3 {
            word >>= 8 * (3 - size);
            U256::from(word)
        } else {
            let word = U256::from(word);
            word << 8 * (size - 3)
        };

        if word != 0 && (compact & 0x00800000) != 0 {
            // negative
            None
        } else if word != 0 && ((size > 34) || (word > 0xff && size > 33) || (word > 0xffff && size > 32)) {
            // overflow
            None
        } else {
            Some(value)
        }
    }
}

// https://github.com/bitcoin/bitcoin/blob/7fcf53f7b4524572d1d0c9a5fdc388e87eb02416/src/test/arith_uint256_tests.cpp
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bignum_set_compact() {
        // make sure that we don't generate compacts with the 0x00800000 bit set
        assert_eq!(U256::from(0x80).get_compact(), Some(0x02008000));

        for (input, output) in [
            (0, Some(0)),
            (0x00123456, Some(0)),
            (0x01003456, Some(0)),
            (0x02000056, Some(0)),
            (0x03000000, Some(0)),
            (0x04000000, Some(0)),
            (0x00923456, Some(0)),
            (0x01803456, Some(0)),
            (0x02800056, Some(0)),
            (0x03800000, Some(0)),
            (0x04800000, Some(0)),
            (0x01123456, Some(0x01120000)),
            (0x01fedcba, None), // negative
            (0x02123456, Some(0x02123400)),
            (0x03123456, Some(0x03123456)),
            (0x04123456, Some(0x04123456)),
            (0x04923456, None), // negative
            (0x05009234, Some(0x05009234)),
            (0x20123456, Some(0x20123456)),
            (0xff123456, None), // overflow
        ] {
            assert_eq!(U256::set_compact(input).and_then(|num| num.get_compact()), output)
        }
    }
}
