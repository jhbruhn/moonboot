use core::ops::Range;

use embedded_storage::Storage;

use crate::{
    log,
    state::{ExchangeProgress, ExchangeStep, State, Update},
    swap::{MemoryError, Swap},
    Address,
};

pub struct Scratch<'a> {
    pub pages: &'a [Range<Address>],
}

impl<'a> Swap for Scratch<'a> {
    fn exchange<InternalMemory: Storage, HardwareState: State, const INTERNAL_PAGE_SIZE: usize>(
        &mut self,
        internal_memory: &mut InternalMemory,
        state: &mut HardwareState,
        exchange: ExchangeProgress,
    ) -> Result<(), MemoryError> {
        let ExchangeProgress {
            a,
            b,
            page_index,
            mut step,
            ..
        } = exchange;

        // TODO: Sanity Check start_index
        if a.size != b.size {
            return Err(MemoryError::BankSizeNotEqual);
        }

        if a.size == 0 || b.size == 0 {
            return Err(MemoryError::BankSizeZero);
        }

        let size = a.size; // Both are equal

        let full_pages = size / INTERNAL_PAGE_SIZE as Address;
        let remaining_page_length = size as usize % INTERNAL_PAGE_SIZE;

        assert_eq!(remaining_page_length, 0);

        let mut ram_buf = [0_u8; INTERNAL_PAGE_SIZE];

        let mut last_state = state.read();

        // Set this in the exchanging part to know whether we are in a recovery process from a
        // failed update or on the initial update
        let recovering = matches!(last_state.update, Update::Revert(_));

        let a_location = a.location;
        let b_location = b.location;

        let mut first = true;
        for page_index in page_index..full_pages {
            let offset = page_index * INTERNAL_PAGE_SIZE as u32;

            let a_location = a_location + offset;
            let b_location = b_location + offset;

            let scratch_index = page_index as usize % self.pages.len();
            let scratch_location = self.pages[scratch_index].start;

            log::trace!(
                "Exchange: Page {}, from a ({}) to b ({}) using scratch ({})",
                page_index,
                a_location,
                b_location,
                scratch_location
            );

            loop {
                if first {
                    // Do not write the state to flash, as it is already recent.
                    first = false;
                } else {
                    last_state.update = Update::Exchanging(ExchangeProgress {
                        a,
                        b,
                        recovering,
                        page_index,
                        step,
                    });
                    state
                        .write(&last_state)
                        .map_err(|_| MemoryError::WriteFailure)?;
                }

                match step {
                    ExchangeStep::AToScratch => {
                        internal_memory
                            .read(a_location, &mut ram_buf)
                            .map_err(|_| MemoryError::ReadFailure)?;
                        internal_memory
                            .write(scratch_location, &ram_buf)
                            .map_err(|_| MemoryError::WriteFailure)?;
                        step = ExchangeStep::BToA;
                    }
                    ExchangeStep::BToA => {
                        internal_memory
                            .read(b_location, &mut ram_buf)
                            .map_err(|_| MemoryError::ReadFailure)?;
                        internal_memory
                            .write(a_location, &ram_buf)
                            .map_err(|_| MemoryError::WriteFailure)?;
                        step = ExchangeStep::ScratchToB;
                    }
                    ExchangeStep::ScratchToB => {
                        internal_memory
                            .read(scratch_location, &mut ram_buf)
                            .map_err(|_| MemoryError::ReadFailure)?;
                        internal_memory
                            .write(b_location, &ram_buf)
                            .map_err(|_| MemoryError::WriteFailure)?;
                        step = ExchangeStep::AToScratch;
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}
