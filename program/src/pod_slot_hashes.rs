//! Temporary reimplementation of the `PodSlotHashes` API from Solana v2.1.
//!
//! Can be removed in favor of the API itself when the program is upgraded to
//! use Solana v2.1.

#[cfg(target_os = "solana")]
use solana_program::{
    pubkey::Pubkey,
    slot_hashes::SlotHashes,
    sysvar::{Sysvar, SysvarId},
};
use {
    bytemuck::{Pod, Zeroable},
    solana_program::{clock::Slot, hash::Hash, program_error::ProgramError},
};

#[cfg(target_os = "solana")]
const U64_SIZE: usize = std::mem::size_of::<u64>();

/// A bytemuck-compatible (plain old data) version of `SlotHash`.
#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
pub struct PodSlotHash {
    pub slot: Slot,
    pub hash: Hash,
}

/// API for querying of the `SlotHashes` sysvar by on-chain programs.
///
/// Hangs onto the allocated raw buffer from the account data, which can be
/// queried or accessed directly as a slice of `PodSlotHash`.
#[derive(Default)]
pub struct PodSlotHashes {
    data: Vec<u8>,
    slot_hashes_start: usize,
    slot_hashes_end: usize,
}

impl PodSlotHashes {
    /// Fetch all of the raw sysvar data using the `sol_get_sysvar` syscall.
    pub fn fetch() -> Result<Self, ProgramError> {
        #[cfg(target_os = "solana")]
        {
            // Allocate an uninitialized buffer for the raw sysvar data.
            let sysvar_len = SlotHashes::size_of();
            let mut data = vec![0; sysvar_len];

            // Ensure the created buffer is aligned to 8.
            if data.as_ptr().align_offset(8) != 0 {
                return Err(ProgramError::InvalidAccountData);
            }

            // Populate the buffer by fetching all sysvar data using the
            // `sol_get_sysvar` syscall.
            get_sysvar(
                &mut data,
                &SlotHashes::id(),
                /* offset */ 0,
                /* length */ sysvar_len as u64,
            )?;

            // Get the number of slot hashes present in the data by reading the
            // `u64` length at the beginning of the data, then use that count to
            // calculate the length of the slot hashes data.
            //
            // The rest of the buffer is uninitialized and should not be accessed.
            let length = data
                .get(..U64_SIZE)
                .and_then(|bytes| bytes.try_into().ok())
                .map(u64::from_le_bytes)
                .and_then(|length| length.checked_mul(std::mem::size_of::<PodSlotHash>() as u64))
                .ok_or(ProgramError::InvalidAccountData)?;

            let slot_hashes_start = U64_SIZE;
            let slot_hashes_end = slot_hashes_start.saturating_add(length as usize);

            return Ok(Self {
                data,
                slot_hashes_start,
                slot_hashes_end,
            });
        }

        #[cfg(not(target_os = "solana"))]
        Err(ProgramError::UnsupportedSysvar)
    }

    /// Return the `SlotHashes` sysvar data as a slice of `PodSlotHash`.
    /// Returns a slice of only the initialized sysvar data.
    pub fn as_slice(&self) -> Result<&[PodSlotHash], ProgramError> {
        self.data
            .get(self.slot_hashes_start..self.slot_hashes_end)
            .and_then(|data| bytemuck::try_cast_slice(data).ok())
            .ok_or(ProgramError::InvalidAccountData)
    }

    /// Given a slot, get its corresponding hash in the `SlotHashes` sysvar
    /// data. Returns `None` if the slot is not found.
    pub fn get(&self, slot: &Slot) -> Result<Option<Hash>, ProgramError> {
        self.as_slice().map(|pod_hashes| {
            pod_hashes
                .binary_search_by(|PodSlotHash { slot: this, .. }| slot.cmp(this))
                .map(|idx| pod_hashes[idx].hash)
                .ok()
        })
    }

    /// Given a slot, get its position in the `SlotHashes` sysvar data. Returns
    /// `None` if the slot is not found.
    pub fn position(&self, slot: &Slot) -> Result<Option<usize>, ProgramError> {
        self.as_slice().map(|pod_hashes| {
            pod_hashes
                .binary_search_by(|PodSlotHash { slot: this, .. }| slot.cmp(this))
                .ok()
        })
    }
}

/// Handler for retrieving a slice of sysvar data from the `sol_get_sysvar`
/// syscall.
#[cfg(target_os = "solana")]
fn get_sysvar(
    dst: &mut [u8],
    sysvar_id: &Pubkey,
    offset: u64,
    length: u64,
) -> Result<(), ProgramError> {
    // Check that the provided destination buffer is large enough to hold the
    // requested data.
    if dst.len() < length as usize {
        return Err(ProgramError::InvalidArgument);
    }

    let sysvar_id = sysvar_id as *const _ as *const u8;
    let var_addr = dst as *mut _ as *mut u8;

    let result =
        unsafe { solana_program::syscalls::sol_get_sysvar(sysvar_id, var_addr, offset, length) };

    match result {
        solana_program::entrypoint::SUCCESS => Ok(()),
        e => Err(e.into()),
    }
}
