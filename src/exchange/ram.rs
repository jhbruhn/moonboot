use embedded_storage::Storage;

use crate::{
    exchange::Exchange,
    hardware::Config,
    log,
    state::{ExchangeProgress, State, Update},
    Address,
};

pub struct Ram;

pub enum ExchangeError<STORAGE, STATE> {
    Storage(STORAGE),
    State(STATE),
}

impl<STORAGE, STATE> core::fmt::Debug for ExchangeError<STORAGE, STATE>
where
    STORAGE: core::fmt::Debug,
    STATE: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Storage(arg0) => f.debug_tuple("Storage").field(arg0).finish(),
            Self::State(arg0) => f.debug_tuple("State").field(arg0).finish(),
        }
    }
}

impl<STORAGE: Storage, STATE: State> Exchange<STORAGE, STATE> for Ram
where
    STORAGE::Error: core::fmt::Debug,
    STATE::Error: core::fmt::Debug,
{
    type Error = ExchangeError<STORAGE::Error, STATE::Error>;

    fn exchange<const INTERNAL_PAGE_SIZE: usize>(
        &mut self,
        _config: &Config,
        storage: &mut STORAGE,
        state: &mut STATE,
        progress: ExchangeProgress,
    ) -> Result<(), Self::Error> {
        let ExchangeProgress {
            a,
            b,
            page_index,
            step,
            ..
        } = progress;

        assert_eq!(a.size, b.size);
        assert_ne!(a.size, 0);
        assert_ne!(b.size, 0);

        let size = a.size; // Both are equal

        let full_pages = size / INTERNAL_PAGE_SIZE as Address;
        let remaining_page_length = size as usize % INTERNAL_PAGE_SIZE;

        let mut page_a_buf = [0_u8; INTERNAL_PAGE_SIZE];
        let mut page_b_buf = [0_u8; INTERNAL_PAGE_SIZE];

        let mut last_state = state.read().map_err(ExchangeError::State)?;

        // Set this in the exchanging part to know whether we are in a recovery process from a
        // failed update or on the initial update
        let recovering = matches!(last_state.update, Update::Revert(_));

        for page_index in page_index..full_pages {
            let offset = page_index * INTERNAL_PAGE_SIZE as u32;
            let a_location = a.location + offset;
            let b_location = b.location + offset;
            log::trace!(
                "Exchange: Page {}, from a ({}) to b ({})",
                page_index,
                a_location,
                b_location
            );

            storage
                .read(a_location, &mut page_a_buf)
                .map_err(ExchangeError::Storage)?;
            storage
                .read(b_location, &mut page_b_buf)
                .map_err(ExchangeError::Storage)?;
            storage
                .write(a_location, &page_b_buf)
                .map_err(ExchangeError::Storage)?;
            storage
                .write(b_location, &page_a_buf)
                .map_err(ExchangeError::Storage)?;

            // Store the exchange progress

            last_state.update = Update::Exchanging(ExchangeProgress {
                a,
                b,
                recovering,
                page_index,
                step,
            });

            state.write(&last_state).map_err(ExchangeError::State)?;
        }
        // TODO: Fit this into the while loop
        if remaining_page_length > 0 {
            let offset = full_pages * INTERNAL_PAGE_SIZE as u32;
            let a_location = a.location + offset;
            let b_location = b.location + offset;

            storage
                .read(a_location, &mut page_a_buf[0..remaining_page_length])
                .map_err(ExchangeError::Storage)?;
            storage
                .read(b_location, &mut page_b_buf[0..remaining_page_length])
                .map_err(ExchangeError::Storage)?;
            storage
                .write(a_location, &page_a_buf[0..remaining_page_length])
                .map_err(ExchangeError::Storage)?;
            storage
                .write(b_location + offset, &page_b_buf[0..remaining_page_length])
                .map_err(ExchangeError::Storage)?;
        }

        Ok(())
    }
}
