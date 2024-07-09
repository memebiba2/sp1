pub mod air;
pub mod columns;
pub mod event;
pub mod opcode;
pub mod trace;
pub mod utils;

pub use event::ByteLookupEvent;
pub use opcode::*;

use core::borrow::BorrowMut;
use std::marker::PhantomData;

use itertools::Itertools;
use p3_field::Field;
use p3_matrix::dense::RowMajorMatrix;

use self::{
    columns::{BytePreprocessedCols, NUM_BYTE_PREPROCESSED_COLS},
    utils::shr_carry,
};
use crate::bytes::trace::NUM_ROWS;

/// The number of different byte operations.
pub const NUM_BYTE_OPS: usize = 9;

/// The number of different byte lookup channels.
pub const NUM_BYTE_LOOKUP_CHANNELS: u32 = 16;

/// A chip for computing byte operations.
///
/// The chip contains a preprocessed table of all possible byte operations. Other chips can then
/// use lookups into this table to compute their own operations.
#[derive(Debug, Clone, Copy, Default)]
pub struct ByteChip<F>(PhantomData<F>);

impl<F: Field> ByteChip<F> {
    /// Creates the preprocessed byte trace.
    ///
    /// This function returns a `trace` which is a matrix containing all possible byte operations.
    pub fn trace() -> RowMajorMatrix<F> {
        // The trace containing all values, with all multiplicities set to zero.
        let mut initial_trace = RowMajorMatrix::new(
            vec![F::zero(); NUM_ROWS * NUM_BYTE_PREPROCESSED_COLS],
            NUM_BYTE_PREPROCESSED_COLS,
        );

        // Record all the necessary operations for each byte lookup.
        let opcodes = ByteOpcode::all();

        // Iterate over all options for pairs of bytes `a` and `b`.
        for (row_index, (b, c)) in (0..=u8::MAX).cartesian_product(0..=u8::MAX).enumerate() {
            let b = b as u8;
            let c = c as u8;
            let col: &mut BytePreprocessedCols<F> = initial_trace.row_mut(row_index).borrow_mut();

            // Set the values of `b` and `c`.
            col.b = F::from_canonical_u8(b);
            col.c = F::from_canonical_u8(c);

            // Iterate over all operations for results and updating the table map.
            let shard = 0;
            for channel in 0..NUM_BYTE_LOOKUP_CHANNELS {
                for opcode in opcodes.iter() {
                    match opcode {
                        ByteOpcode::AND => {
                            let and = b & c;
                            col.and = F::from_canonical_u8(and);
                            ByteLookupEvent::new(
                                shard, channel, *opcode, and as u32, 0, b as u32, c as u32,
                            )
                        }
                        ByteOpcode::OR => {
                            let or = b | c;
                            col.or = F::from_canonical_u8(or);
                            ByteLookupEvent::new(
                                shard, channel, *opcode, or as u32, 0, b as u32, c as u32,
                            )
                        }
                        ByteOpcode::XOR => {
                            let xor = b ^ c;
                            col.xor = F::from_canonical_u8(xor);
                            ByteLookupEvent::new(
                                shard, channel, *opcode, xor as u32, 0, b as u32, c as u32,
                            )
                        }
                        ByteOpcode::SLL => {
                            let sll = b << (c & 7);
                            col.sll = F::from_canonical_u8(sll);
                            ByteLookupEvent::new(
                                shard, channel, *opcode, sll as u32, 0, b as u32, c as u32,
                            )
                        }
                        ByteOpcode::U8Range => {
                            ByteLookupEvent::new(shard, channel, *opcode, 0, 0, b as u32, c as u32)
                        }
                        ByteOpcode::ShrCarry => {
                            let (res, carry) = shr_carry(b, c);
                            col.shr = F::from_canonical_u8(res);
                            col.shr_carry = F::from_canonical_u8(carry);
                            ByteLookupEvent::new(
                                shard,
                                channel,
                                *opcode,
                                res as u32,
                                carry as u32,
                                b as u32,
                                c as u32,
                            )
                        }
                        ByteOpcode::LTU => {
                            let ltu = b < c;
                            col.ltu = F::from_bool(ltu);
                            ByteLookupEvent::new(
                                shard, channel, *opcode, ltu as u32, 0, b as u32, c as u32,
                            )
                        }
                        ByteOpcode::MSB => {
                            let msb = (b & 0b1000_0000) != 0;
                            col.msb = F::from_bool(msb);
                            ByteLookupEvent::new(
                                shard, channel, *opcode, msb as u32, 0, b as u32, 0 as u32,
                            )
                        }
                        ByteOpcode::U16Range => {
                            let v = ((b as u32) << 8) + c as u32;
                            col.value_u16 = F::from_canonical_u32(v);
                            ByteLookupEvent::new(shard, channel, *opcode, v, 0, 0, 0)
                        }
                    };
                }
            }
        }

        initial_trace
    }
}

#[cfg(test)]
mod tests {
    use p3_baby_bear::BabyBear;
    use std::time::Instant;

    use super::*;

    #[test]
    pub fn test_trace_and_map() {
        let start = Instant::now();
        ByteChip::<BabyBear>::trace();
        println!("trace and map: {:?}", start.elapsed());
    }
}
