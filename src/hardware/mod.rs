pub mod processor;

use crate::Address;

#[cfg(feature = "defmt")]
use defmt::Format;
#[cfg(feature = "ram-state")]
use desse::{Desse, DesseSized};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Identifier for multiple memory instances. Currently only Internal memory is supported
#[cfg_attr(feature = "defmt", derive(Format))]
#[cfg_attr(feature = "derive", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "ram-state", derive(Desse, DesseSized))]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum MemoryUnit {
    /// On-chip memory of your SoC
    Internal,
    // External(usize) // nth external unit
}

/// Description of a memory bank in a specific memory unit
#[cfg_attr(feature = "defmt", derive(Format))]
#[cfg_attr(feature = "derive", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "ram-state", derive(Desse, DesseSized))]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Bank {
    // TODO: Hide members?
    /// Starting address of this Bank
    pub location: Address,
    /// Size of this Bank
    pub size: Address, // TODO: Use NonZeroU32 to remove checks?
    /// In which memory unit this bank is stored
    pub memory_unit: MemoryUnit,
}

/// Configuration of your SoCs partitioning
#[cfg_attr(feature = "defmt", derive(Format))]
#[cfg_attr(feature = "derive", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "ram-state", derive(Desse, DesseSized))]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Config {
    /// bank this bootloader jumps to, holds your main firmware
    pub boot_bank: Bank,
    /// alternative bank we write the new firmware image to, used as a memory only
    pub update_bank: Bank,
    /// bank the bootloader is contained in, switching between banks
    pub bootloader_bank: Bank,
    /// bank the pages are temporarily stored when using the `Scratch` exchange method
    pub scratch_bank: Bank,
    // Initial Image is stored to this bank after first update, restore on failure
    // pub golden_bank: Bank,
    /// section of RAM of this device
    pub ram_bank: Bank,
}

/// Configuration for linker scripts
#[cfg_attr(feature = "defmt", derive(Format))]
#[cfg_attr(feature = "derive", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "ram-state", derive(Desse, DesseSized))]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct LinkerConfig {
    /// Origin address of the internal non-volatile memory
    pub flash_origin: Address,
    /// Origin address of the internal RAM bank
    pub ram_origin: Address,
    // TODO enable via feature flag?
    /// Whether to store the shared state in RAM and thus reserve some RAM memory for that
    pub has_ram_state: bool,
}
