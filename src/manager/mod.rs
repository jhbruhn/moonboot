use crate::{
    hardware::{processor::Processor, Config},
    state::{State, Update},
};

use embedded_storage::Storage;

use crate::log;

/// Instantiate this in your application to enable mutation of the State specified in this and jump
/// to the bootloader to apply any updates.
pub struct MoonbootManager<
    STORAGE: Storage,
    STATE: State,
    PROCESSOR: Processor,
    const INTERNAL_PAGE_SIZE: usize,
> {
    config: Config,
    storage: STORAGE,
    state: STATE,
    processor: PROCESSOR,
}

pub struct InitError;

pub enum MarkError<E> {
    UpdateQueuedButNotInstalled,
    State(E),
}

impl<STORAGE: Storage, STATE: State, PROCESSOR: Processor, const INTERNAL_PAGE_SIZE: usize>
    MoonbootManager<STORAGE, STATE, PROCESSOR, INTERNAL_PAGE_SIZE>
{
    pub fn new(
        config: Config,
        storage: STORAGE,
        state: STATE,
        processor: PROCESSOR,
    ) -> Result<MoonbootManager<STORAGE, STATE, PROCESSOR, INTERNAL_PAGE_SIZE>, InitError> {
        if config.update_bank.size > config.boot_bank.size {
            log::error!(
                "Requested update bank {:?} is larger than boot bank {:?}",
                bank,
                self.config.boot_bank
            );
            return Err(InitError);
        }

        Ok(Self {
            config,
            storage,
            state,
            processor,
        })
    }

    /// Destroy this instance of the boot manager and return access to the hardware peripheral
    pub fn destroy(self) -> (STORAGE, STATE, PROCESSOR) {
        (self.storage, self.state, self.processor)
    }

    /// Run this immediately after booting your new image successfully to mark the boot as
    /// succesful. If you do not do this, any reset will cause the bootloader to restore to the
    /// previous firmware image.
    pub fn mark_boot_successful(&mut self) -> Result<(), MarkError<STATE::Error>> {
        let mut current_state = self.state.read().map_err(MarkError::State)?;

        log::info!(
            "Application running, marking boot as successful. Current state: {:?}",
            current_state
        );

        current_state.update = match current_state.update {
            Update::None => {
                log::info!("No Update was done.");
                Update::None
            }
            Update::Revert(_) => {
                log::info!("Software was updated, marking as successful.");
                Update::None
            }
            _ => {
                log::error!("There is an update queued, but it has not been installed yet. Did you skip the bootloader?");
                return Err(MarkError::UpdateQueuedButNotInstalled);
            }
        };

        log::trace!("New state: {:?}", current_state);

        self.state.write(&current_state).map_err(MarkError::State)
    }

    // Upgrade firmware verifiying the given signature over the size of size.
    // Can only return an error or diverge (!, represented by Void while ! is not a type yet)
    pub fn update(&mut self) -> Result<void::Void, STATE::Error> {
        // Apply the update stored in the update bank
        let bank = self.config.update_bank;

        log::info!("Update requested on slot {:?}", bank);

        let mut current_state = self.state.read()?;

        if current_state.update != Update::None {
            log::warn!(
                "There is already an update in progress or queued: {:?}",
                current_state.update
            );
        }

        current_state.update = Update::Request(bank);

        self.state.write(&current_state)?;

        log::info!("Stored update request, jumping to bootloader! Geronimo!");

        let bootloader_address = self.config.bootloader_bank.location;

        log::info!("Executing pre jump handler.");
        extern "Rust" {
            fn _moonboots_pre_jump();
        }
        unsafe {
            _moonboots_pre_jump();
        }

        self.processor.do_jump(bootloader_address)
    }
}

// /// Easily get read access to the update bank
// impl<
//         InternalMemory: Storage,
//         HardwareState: State,
//         PROCESSOR: Processor,
//         const INTERNAL_PAGE_SIZE: usize,
//     > core::convert::AsRef<[u8]>
//     for MoonbootManager<InternalMemory, HardwareState, CPU, INTERNAL_PAGE_SIZE>
// {
//     #[inline]
//     fn as_ref(&self) -> &[u8] {
//         unsafe {
//             core::slice::from_raw_parts(
//                 self.config.update_bank.location as *const u8,
//                 self.config.update_bank.size as usize,
//             )
//         }
//     }
// }

// /// Read Access to the current update target slot
// impl<
//         InternalMemory: Storage,
//         HardwareState: State,
//         CPU: Processor,
//         const INTERNAL_PAGE_SIZE: usize,
//     > ReadStorage for MoonbootManager<InternalMemory, HardwareState, CPU, INTERNAL_PAGE_SIZE>
// {
//     type Error = (); // TODO

//     fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
//         let bank = self.config.update_bank; // For now we always write updates to this bank.
//         if offset > bank.size || offset + bytes.len() as u32 > bank.size {
//             Err(()) // TODO: We want better error types!
//         } else {
//             // TODO! fix
//             let bank_start = bank.location;
//             log::info!("Writing at {:x}[{:x}]", bank_start, offset);
//             match bank.memory_unit {
//                 crate::hardware::MemoryUnit::Internal => {
//                     { self.storage.read(bank_start + offset, bytes) }.map_err(|_| ())
//                 }
//             }
//         }
//     }

//     fn capacity(&self) -> usize {
//         self.config.update_bank.size as usize
//     }
// }

// /// Write Access to the current update target slot
// impl<
//         InternalMemory: Storage,
//         HardwareState: State,
//         CPU: Processor,
//         const INTERNAL_PAGE_SIZE: usize,
//     > Storage for MoonbootManager<InternalMemory, HardwareState, CPU, INTERNAL_PAGE_SIZE>
// {
//     fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
//         let bank = self.config.update_bank; // For now we always write updates to this bank.
//         if offset > bank.size || offset + bytes.len() as u32 > bank.size {
//             Err(()) // TODO: We want better error types!
//         } else {
//             // TODO! fix
//             let bank_start = bank.location;
//             log::info!("Writing at {:x}[{:x}]", bank_start, offset);
//             match bank.memory_unit {
//                 crate::hardware::MemoryUnit::Internal => {
//                     { self.internal_memory.write(bank_start + offset, bytes) }.map_err(|_| ())
//                 }
//             }
//         }
//     }
// }
