use crate::log;

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
