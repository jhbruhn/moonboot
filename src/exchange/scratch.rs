use core::{fmt::Debug, ops::Range};

use embedded_storage::Storage;

use crate::{
    exchange::Exchange,
    log,
    state::{ExchangeProgress, ExchangeStep, State, Update},
    Address,
};

pub struct Scratch<'a> {
    pub pages: &'a [Range<Address>],
}

pub enum ExchangeError<STORAGE, STATE> {
    Storage(STORAGE),
    State(STATE),
}

impl<STORAGE, STATE> Debug for ExchangeError<STORAGE, STATE>
where
    STORAGE: Debug,
    STATE: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Storage(arg0) => f.debug_tuple("Storage").field(arg0).finish(),
            Self::State(arg0) => f.debug_tuple("State").field(arg0).finish(),
        }
    }
}

impl<'a, STORAGE: Storage, STATE: State> Exchange<STORAGE, STATE> for Scratch<'a>
where
    STORAGE::Error: Debug,
    STATE::Error: Debug,
{
    type Error = ExchangeError<STORAGE::Error, STATE::Error>;

    fn exchange<const INTERNAL_PAGE_SIZE: usize>(
        &mut self,
        storage: &mut STORAGE,
        state: &mut STATE,
        progress: ExchangeProgress,
    ) -> Result<(), Self::Error> {
        let ExchangeProgress {
            a,
            b,
            page_index,
            mut step,
            ..
        } = progress;

        assert_eq!(a.size, b.size);
        assert_ne!(a.size, 0);
        assert_ne!(b.size, 0);

        let size = a.size; // Both are equal

        let full_pages = size / INTERNAL_PAGE_SIZE as Address;
        let remaining_page_length = size as usize % INTERNAL_PAGE_SIZE;

        assert_eq!(remaining_page_length, 0);

        let mut ram_buf = [0_u8; INTERNAL_PAGE_SIZE];

        let mut last_state = state.read().map_err(ExchangeError::State)?;

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
                    state.write(&last_state).map_err(ExchangeError::State)?;
                }

                match step {
                    ExchangeStep::AToScratch => {
                        storage
                            .read(a_location, &mut ram_buf)
                            .map_err(ExchangeError::Storage)?;
                        storage
                            .write(scratch_location, &ram_buf)
                            .map_err(ExchangeError::Storage)?;
                        step = ExchangeStep::BToA;
                    }
                    ExchangeStep::BToA => {
                        storage
                            .read(b_location, &mut ram_buf)
                            .map_err(ExchangeError::Storage)?;
                        storage
                            .write(a_location, &ram_buf)
                            .map_err(ExchangeError::Storage)?;
                        step = ExchangeStep::ScratchToB;
                    }
                    ExchangeStep::ScratchToB => {
                        storage
                            .read(scratch_location, &mut ram_buf)
                            .map_err(ExchangeError::Storage)?;
                        storage
                            .write(b_location, &ram_buf)
                            .map_err(ExchangeError::Storage)?;
                        step = ExchangeStep::AToScratch;
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}
