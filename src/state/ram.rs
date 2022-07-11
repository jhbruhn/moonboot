use embedded_storage::Storage;

use crate::{log, Address};

use super::*;

/// State read and written to RAM. This assumes the device is never powered off / the ram is never
/// reset!
pub struct RamState;

extern "C" {
    static mut _moonboot_state_crc_start: StateCrcType;
    static mut _moonboot_state_data_start: [u8; STATE_SERIALIZED_MAX_SIZE];
    // TODO: Move these as normal variables to linker sections via #[link] macro?
}

impl State for RamState {
    type Error = void::Void;

    fn read(&mut self) -> Result<MoonbootState, void::Void> {
        let crc = unsafe { _moonboot_state_crc_start };

        log::info!(
            "Reading data with len: {}, CRC: {}",
            STATE_SERIALIZED_MAX_SIZE,
            crc
        );

        let checksum = checksum(unsafe { &_moonboot_state_data_start });
        if crc == checksum {
            let data = MoonbootState::deserialize_from(unsafe { &_moonboot_state_data_start });
            log::trace!("CRC Match! {}: {:?}", crc, data);
            Ok(data)
        } else {
            log::trace!("CRC Mismatch! {} vs {}", crc, checksum);
            Ok(MoonbootState {
                update: Update::None,
            })
        }
    }

    fn write(&mut self, data: &MoonbootState) -> Result<(), Self::Error> {
        log::trace!("Writing data {:?}", data);

        unsafe { _moonboot_state_data_start = data.serialize() };
        log::trace!("Written data: {:?}", unsafe { &_moonboot_state_data_start });

        unsafe {
            _moonboot_state_crc_start = checksum(&_moonboot_state_data_start);
        }
        log::info!(
            "Written len: {}, checksum: {}",
            STATE_SERIALIZED_MAX_SIZE,
            unsafe { _moonboot_state_crc_start }
        );

        Ok(())
    }
}

impl Exchange for RamState {
    type OtherError = void::Void;

    fn exchange<STORAGE: Storage, STATE: State, const INTERNAL_PAGE_SIZE: usize>(
        &mut self,
        storage: &mut STORAGE,
        state: &mut STATE,
        progress: ExchangeProgress,
    ) -> Result<(), ExchangeError<STORAGE::Error, STATE::Error, Self::OtherError>> {
        let ExchangeProgress {
            a,
            b,
            page_index,
            step,
            ..
        } = progress;

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
