use crate::{
    hardware::processor::Processor,
    hardware::{Bank, Config},
    state::{ExchangeProgress, State, Update, UpdateError},
    Address,
};

use embedded_storage::Storage;

use crate::log;

#[cfg(feature = "defmt")]
use defmt::Format;


/// Error occured during nemory access
#[cfg_attr(feature = "use-defmt", derive(Format))]
#[derive(Debug)]
enum MemoryError {
    BankSizeNotEqual,
    BankSizeZero,
    ReadFailure,
    WriteFailure,
}

/// Use this from your bootloader application and call boot() to do the magic, reading the current
/// state via the State type and then jumping to the new image using the Jumper specified
pub struct MoonbootBoot<
    InternalMemory: Storage,
    HardwareState: State,
    CPU: Processor, // TODO: Wrap these into a context struct like rubble?
    const INTERNAL_PAGE_SIZE: usize,
> {
    config: Config,
    internal_memory: InternalMemory,
    state: HardwareState,
    processor: CPU,
}

impl<
        InternalMemory: Storage,
        HardwareState: State,
        CPU: Processor,
        const INTERNAL_PAGE_SIZE: usize,
    > MoonbootBoot<InternalMemory, HardwareState, CPU, INTERNAL_PAGE_SIZE>
{
    /// create a new instance of the bootloader
    pub fn new(
        config: Config,
        internal_memory: InternalMemory,
        state: HardwareState,
        processor: CPU,
    ) -> MoonbootBoot<InternalMemory, HardwareState, CPU, INTERNAL_PAGE_SIZE> {
        Self {
            config,
            internal_memory,
            state,
            processor,
        }
    }

    /// Destroy this instance of the bootloader and return access to the hardware peripheral
    pub fn destroy(self) -> (InternalMemory, HardwareState, CPU) {
        (self.internal_memory, self.state, self.processor)
    }

    /// Execute the update and boot logic of the bootloader
    pub fn boot(&mut self) -> Result<void::Void, ()> {
        // TODO: consider error handling
        log::info!("Booting with moonboot!");

        self.processor.setup(&self.config);

        let mut state = self.state.read();

        log::info!("Old State: {:?}", state);

        // Step 1: Do things according to update state
        state.update = match state.update {
            Update::None => self.handle_none(),
            Update::Request(bank) => self.handle_request(bank),
            Update::Revert(bank) => self.handle_revert(bank),
            Update::Exchanging(progress) => self.handle_exchanging(progress),
            Update::Error(err) => Update::Error(err),
        };

        // TODO: Handle Progress Variable in state to recover from power loss

        log::info!("New State: {:?}", state);

        // Step 2: Update state of Bootloader
        self.state.write(state)?;

        // Step 3: Jump to new or unchanged firmware
        self.jump_to_firmware();
    }

    // Handle a Update::None Request (no op effectively)
    fn handle_none(&mut self) -> Update {
        // No update requested -> Nothing to do
        log::info!("Nothing to do, jumping straight to firmware!");
        Update::None
    }

    // Handle an Update::Request state, replacing the old firmware with the new one
    fn handle_request(&mut self, new_firmware: Bank) -> Update {
        log::info!("Update to firmware image {:?} requested.", new_firmware);

        self.exchange_firmwares(new_firmware, true)
    }

    // Handle a revert request because booting of the new firmware failed
    fn handle_revert(&mut self, revert_firmware: Bank) -> Update {
        // Exchange failed update firmware with old firmware image, on success return None so
        // firmware and bootloader functions as usual
        log::warn!(
            "Firmware did not reset state from Revert to None, something went wrong after update!"
        );
        log::info!(
            "Reverting to previous firmware image {:?}.",
            revert_firmware
        );
        self.exchange_firmwares(revert_firmware, false)
    }

    // Handle a case of power interruption or similar, which lead to a exchange_banks being
    // interrupted.
    fn handle_exchanging(&mut self, progress: ExchangeProgress) -> Update {
        log::error!(
            "Firmware Update was interrupted! Trying to recover with exchange operation: {:?}",
            progress
        );

        let exchange_result =
            self.exchange_banks_with_start(progress.a, progress.b, progress.page_index);

        if exchange_result.is_ok() {
            let state = self.state.read().update;
            match state {
                Update::Exchanging(progress) => {
                    if progress.recovering {
                        Update::None
                    } else {
                        Update::Revert(progress.a)
                    }
                }
                _ => Update::Error(UpdateError::InvalidState),
            }
        } else {
            log::error!(
                "Could not recover from failed update, Error: {:?}",
                exchange_result
            );
            Update::Error(UpdateError::ImageExchangeFailed)
        }
    }

    // Revert the bootable image with the image in index new_firmware. Returns Revert on success if
    // with_failsafe_revert is true, returns None if with_failsafe_revert ist false
    fn exchange_firmwares(&mut self, new: Bank, with_failsafe_revert: bool) -> Update {
        let old = self.config.boot_bank;
        // An update from new_firmware to the bootable firmware was requested.
        if new != old
        // TODO: sanity check firmware is not updated with itself && new_firmware != self.hardware.bootable_image
        {
            log::info!(
                "Exchanging bootable firmware image slot (address: 0x{:x}, size: {}K) with image (address: 0x{:x}, size: {}K).",
                old.location,
                old.size / 1024,
                new.location,
                new.size / 1024
            );

            // Try to exchange the firmware images
            let exchange_result = self.exchange_banks(new, old);
            if exchange_result.is_ok() {
                if with_failsafe_revert {
                    // Update Firmware Update State to revert. The Application will set this to
                    // None on successful boot. If we go into the bootloader again and this is still
                    // set, something is wrong with the new application, so we will revert!
                    Update::Revert(new)
                } else {
                    // Reverting to the new firmware, boot as usual to let the firmware try an
                    // update again
                    Update::None
                }
            } else {
                log::error!(
                    "Failed to exchange firmware images due to a hardware error: {:?}",
                    exchange_result
                );
                Update::Error(UpdateError::ImageExchangeFailed)
            }
        } else {
            log::error!("An invalid image index has been specified during update or revert!");
            Update::Error(UpdateError::InvalidImageIndex)
        }
    }

    fn exchange_banks(&mut self, a: Bank, b: Bank) -> Result<(), MemoryError> {
        self.exchange_banks_with_start(a, b, 0)
    }

    fn exchange_banks_with_start(
        &mut self,
        a: Bank,
        b: Bank,
        start_index: u32,
    ) -> Result<(), MemoryError> {
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

        let mut page_a_buf = [0_u8; INTERNAL_PAGE_SIZE];
        let mut page_b_buf = [0_u8; INTERNAL_PAGE_SIZE];
        // can we reduce this to 1 buf and fancy operations?
        // probably not with the read/write API.
        // classic memory exchange problem :)

        // Set this in the exchanging part to know whether we are in a recovery process from a
        // failed update or on the initial update
        let recovering = matches!(self.state.read().update, Update::Revert(_));

        // TODO: Fix
        let a_location = a.location;
        let b_location = b.location;

        for page_index in start_index..full_pages {
            let offset = page_index * INTERNAL_PAGE_SIZE as u32;
            log::trace!(
                "Exchange: Page {}, from a ({}) to b ({})",
                page_index,
                a_location + offset,
                b_location + offset
            );
            self.internal_memory
                .read(a_location + offset, &mut page_a_buf)
                .map_err(|_| MemoryError::ReadFailure)?;
            self.internal_memory
                .read(b_location + offset, &mut page_b_buf)
                .map_err(|_| MemoryError::ReadFailure)?;
            self.internal_memory
                .write(a_location + offset, &page_b_buf)
                .map_err(|_| MemoryError::WriteFailure)?;
            self.internal_memory
                .write(b_location + offset, &page_a_buf)
                .map_err(|_| MemoryError::WriteFailure)?;

            // Store the exchange progress
            let mut state = self.state.read();
            state.update = Update::Exchanging(ExchangeProgress {
                a,
                b,
                recovering,
                page_index,
            });
            // TODO: Ignore the error here?
            let _ = self.state.write(state);
        }
        // TODO: Fit this into the while loop
        if remaining_page_length > 0 {
            let offset = full_pages * INTERNAL_PAGE_SIZE as u32;

            self.internal_memory
                .read(
                    a.location + offset,
                    &mut page_a_buf[0..remaining_page_length],
                )
                .map_err(|_| MemoryError::ReadFailure)?;
            self.internal_memory
                .read(
                    b.location + offset,
                    &mut page_b_buf[0..remaining_page_length],
                )
                .map_err(|_| MemoryError::ReadFailure)?;
            self.internal_memory
                .write(a.location + offset, &page_a_buf[0..remaining_page_length])
                .map_err(|_| MemoryError::WriteFailure)?;
            self.internal_memory
                .write(b.location + offset, &page_b_buf[0..remaining_page_length])
                .map_err(|_| MemoryError::WriteFailure)?;
        }

        Ok(())
    }
    // Jump to the firmware image marked as bootable
    fn jump_to_firmware(&mut self) -> ! {
        let app_exec_image = self.config.boot_bank;
        let app_address = app_exec_image.location;
        log::info!("Jumping to firmware at {:x}", app_address);

        log::info!("Executing pre jump handler.");
        extern "Rust" {
            fn _moonboots_pre_jump();
        }
        unsafe {
            _moonboots_pre_jump();
        }

        self.processor.do_jump(app_address)
    }
}
