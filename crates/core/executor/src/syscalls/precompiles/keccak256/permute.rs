use crate::{
    events::KeccakPermuteEvent,
    syscalls::{Syscall, SyscallContext},
};

use tiny_keccak::keccakf;

pub(crate) const STATE_SIZE: usize = 25;

// The permutation state is 25 u64's.  Our word size is 32 bits, so it is 50 words.
pub const STATE_NUM_WORDS: usize = STATE_SIZE * 2;

pub(crate) struct Keccak256PermuteSyscall;

impl Syscall for Keccak256PermuteSyscall {
    fn num_extra_cycles(&self) -> u32 {
        1
    }

    fn execute(&self, rt: &mut SyscallContext, arg1: u32, arg2: u32) -> Option<u32> {
        let start_clk = rt.clk;
        let state_ptr = arg1;
        if arg2 != 0 {
            panic!("Expected arg2 to be 0, got {arg2}");
        }

        let mut state_read_records = Vec::new();
        let mut state_write_records = Vec::new();

        let mut state = Vec::new();

        for i in 0..STATE_NUM_WORDS {
            let addr = state_ptr + i as u32 * 4;
            let local_mem_access = rt.rt.local_memory_access.remove(&addr);

            if let Some(local_mem_access) = local_mem_access {
                rt.rt.record.local_memory_access.push(local_mem_access);
            }
        }

        let (state_records, state_values) = rt.mr_slice(state_ptr, STATE_NUM_WORDS);
        state_read_records.extend_from_slice(&state_records);

        for values in state_values.chunks_exact(2) {
            let least_sig = values[0];
            let most_sig = values[1];
            state.push(least_sig as u64 + ((most_sig as u64) << 32));
        }

        let saved_state = state.clone();

        let mut state = state.try_into().unwrap();
        keccakf(&mut state);

        // Increment the clk by 1 before writing because we read from memory at start_clk.
        rt.clk += 1;
        let mut values_to_write = Vec::new();
        for i in 0..STATE_SIZE {
            let most_sig = ((state[i] >> 32) & 0xFFFFFFFF) as u32;
            let least_sig = (state[i] & 0xFFFFFFFF) as u32;
            values_to_write.push(least_sig);
            values_to_write.push(most_sig);
        }

        let write_records = rt.mw_slice(state_ptr, values_to_write.as_slice());
        state_write_records.extend_from_slice(&write_records);

        let mut keccek_local_mem_access = Vec::new();
        for i in 0..STATE_NUM_WORDS {
            let addr = state_ptr + i as u32 * 4;
            let local_mem_access =
                rt.rt.local_memory_access.remove(&addr).expect("Expected local memory access");

            keccek_local_mem_access.push(local_mem_access);
        }

        // Push the Keccak permute event.
        let shard = rt.current_shard();
        let channel = rt.current_channel();
        let lookup_id = rt.syscall_lookup_id;
        rt.record_mut().keccak_permute_events.push(KeccakPermuteEvent {
            lookup_id,
            shard,
            channel,
            clk: start_clk,
            pre_state: saved_state.as_slice().try_into().unwrap(),
            post_state: state.as_slice().try_into().unwrap(),
            state_read_records,
            state_write_records,
            state_addr: state_ptr,
            local_mem_access: keccek_local_mem_access,
        });

        None
    }
}
