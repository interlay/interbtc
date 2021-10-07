use sp_core::U256;

pub trait ToCompact {
    fn to_compact(self) -> u32;
}

// https://github.com/bitcoin/bitcoin/blob/7fcf53f7b4524572d1d0c9a5fdc388e87eb02416/src/arith_uint256.cpp#L223
impl ToCompact for U256 {
    fn to_compact(self) -> u32 {
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

        compact | (size << 24) as u32
    }
}

pub trait FromCompact {
    fn from_compact(value: u32) -> Self;
}

// https://github.com/bitcoin/bitcoin/blob/7fcf53f7b4524572d1d0c9a5fdc388e87eb02416/src/arith_uint256.cpp#L203
impl FromCompact for U256 {
    fn from_compact(bits: u32) -> Self {
        let (mant, expt) = {
            let size = bits >> 24;
            if size <= 3 {
                ((bits & 0xFFFFFF) >> (8 * (3 - size as usize)), 0)
            } else {
                (bits & 0xFFFFFF, 8 * ((bits >> 24) - 3))
            }
        };

        if mant > 0x7FFFFF {
            Default::default()
        } else {
            U256::from(mant) << (expt as usize)
        }
    }
}
