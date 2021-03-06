use crate::hardware::Bank;
use crate::log;

use crc::{Crc, CRC_32_CKSUM};
#[cfg(feature = "defmt")]
use defmt::Format;
#[cfg(feature = "ram-state")]
use desse::{Desse, DesseSized};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Decision making states for the bootloader
// TODO: Hash + Signature? Should be done on download I think! This way, the algorithms can be
// exchanged via software updates easily
#[cfg_attr(feature = "use-defmt", derive(Format))]
#[cfg_attr(feature = "derive", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "ram-state", derive(Desse, DesseSized))]
#[derive(Debug, PartialEq, Eq)]
pub enum Update {
    // No update requested, just jump to the application
    None,
    // Exchange the current boot image with the one from image specified as index, a signature to
    // verify against and the size of the firmware image to make the firmware signature
    // verification succeed
    Request(Bank),
    // Revert the current boot image to the from image specified as index
    Revert(Bank),
    // An Exchange Operation is in Progress or was interrupted
    Exchanging(ExchangeProgress),
    // An Error during the update has occured!
    Error(UpdateError),
}

#[cfg_attr(feature = "use-defmt", derive(Format))]
#[cfg_attr(feature = "derive", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "ram-state", derive(Desse, DesseSized))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Errors that can occur during update
pub enum UpdateError {
    /// A wrong Image Index has been specified
    InvalidImageIndex,
    /// Failed to exchange the new image with the old one
    ImageExchangeFailed,
    /// Something f'ed up the internal state
    InvalidState,
    /// The Signature provided does not match the PublicKey or Image.
    InvalidSignature,
}

/// Store the progress of the current exchange operation
#[cfg_attr(feature = "use-defmt", derive(Format))]
#[cfg_attr(feature = "derive", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "ram-state", derive(Desse, DesseSized))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExchangeProgress {
    /// Bank the update is coming from
    pub(crate) a: Bank,
    /// Bank the update is going to
    pub(crate) b: Bank,
    /// Page the operation has last copied
    pub(crate) page_index: u32,
    /// Whether this exchange resulted from a Request (false) or a Revert (true)
    pub(crate) recovering: bool,
}

/// Struct used to store the state of the bootloader situation in NVM
#[cfg_attr(feature = "use-defmt", derive(Format))]
#[cfg_attr(feature = "derive", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "ram-state", derive(Desse, DesseSized))]
#[derive(Debug)]
pub struct MoonbootState {
    /// If set Request, an Update is requested. This will exchange the two images, set the update
    /// state to Revert and start the application. The application then has to set this state to
    /// None and store it. If something went wrong and the boot attempt results in a restart of
    /// the bootloader, the bootloader starts with this variable set to Revert and thus exchanges
    /// the two images again, doing a downgrade because of a failed boot
    pub update: Update,
}

/// Hardware abstraction for the state storage. Can for example be stored on a flash bank, or in
/// RAM. As long as you don't want to perform update download, power cycle the device, and then
/// apply the update, storing it in volatile memory is fine.
pub trait State {
    /// Read the shared state
    fn read(&mut self) -> MoonbootState;
    /// Write the new state to the shared state
    fn write(&mut self, data: MoonbootState) -> Result<(), ()>;
}

/// Size of the serialized state
pub const STATE_SERIALIZED_MAX_SIZE: usize = MoonbootState::SIZE;
/// Type used to store the shared state CRC
pub type StateCrcType = u32;
const CRC: Crc<StateCrcType> = Crc::<StateCrcType>::new(&CRC_32_CKSUM);

fn checksum(bytes: &[u8]) -> StateCrcType {
    CRC.checksum(bytes)
}

/// State stored in the RAM
/// TODO: Move to hardware folder together with state trait?
#[cfg(feature = "ram-state")]
pub mod ram {
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
        fn read(&mut self) -> MoonbootState {
            let crc = unsafe { _moonboot_state_crc_start };

            log::info!(
                "Reading data with len: {}, CRC: {}",
                STATE_SERIALIZED_MAX_SIZE,
                crc
            );

            let checksum = checksum(unsafe { &_moonboot_state_data_start });
            if crc == checksum {
                let data =
                    MoonbootState::deserialize_from(unsafe { &_moonboot_state_data_start });
                log::trace!("CRC Match! {}: {:?}", crc, data);
                return data;
            } else {
                log::trace!("CRC Mismatch! {} vs {}", crc, checksum);
            }

            MoonbootState {
                update: Update::None,
            }
        }

        fn write(&mut self, data: MoonbootState) -> Result<(), ()> {
            log::trace!("Writing data {:?}", data);

            unsafe { _moonboot_state_data_start = data.serialize() };
            log::trace!("Written data: {:?}", unsafe {
                &_moonboot_state_data_start
            });

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
}
