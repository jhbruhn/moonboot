use moonboot::{
    hardware::{Config, LinkerConfig},
    state::{StateCrcType, STATE_SERIALIZED_MAX_SIZE},
    Address,
};
fn generate_linker_script(
    flash_origin: Address,
    flash_length: Address,
    ram_origin: Address,
    ram_length: Address,
    with_ram_state: bool,
) -> String {
    let ram_length = ram_length as usize;
    if with_ram_state {
        let crc_length = core::mem::size_of::<StateCrcType>(); // 4 byte crc
        let data_len_length = core::mem::size_of::<u32>();
        let state_length = STATE_SERIALIZED_MAX_SIZE;
        let ram_length = ram_length as usize - state_length - crc_length - data_len_length;
        let state_origin = ram_origin as usize + ram_length;
        format!(
            "

    MEMORY {{
        FLASH : ORIGIN = 0x{flash_origin:08x}, LENGTH = {flash_length}
        RAM : ORIGIN = 0x{ram_origin:08x}, LENGTH = {ram_length}
        MOONBOOT_STATE: ORIGIN = 0x{state_origin:08x}, LENGTH = {state_section_length}
    }}

    _moonboot_state_crc_start = ORIGIN(MOONBOOT_STATE);
    _moonboot_state_len_start = ORIGIN(MOONBOOT_STATE) + {crc_length};
    _moonboot_state_data_start = ORIGIN(MOONBOOT_STATE) + {crc_length} + {data_len_length};
    PROVIDE(_moonboots_pre_jump = __moonboots_default_pre_jump);
",
            flash_origin = flash_origin,
            flash_length = flash_length,
            ram_origin = ram_origin,
            ram_length = ram_length,
            state_origin = state_origin,
            state_section_length = state_length + crc_length + data_len_length,
            data_len_length = data_len_length,
            crc_length = crc_length,
        )
    } else {
        format!(
            "
    MEMORY {{
        FLASH : ORIGIN = 0x{flash_origin:08x}, LENGTH = {flash_length}
        RAM : ORIGIN = 0x{ram_origin:08x}, LENGTH = {ram_length}
    }}
    
    PROVIDE(_moonboots_pre_jump = __moonboots_default_pre_jump);
",
            flash_origin = flash_origin,
            flash_length = flash_length,
            ram_origin = ram_origin,
            ram_length = ram_length,
        )
    }
}

pub fn generate_bootloader_script(config: Config, linker_config: LinkerConfig) -> String {
    generate_linker_script(
        linker_config.flash_origin + config.bootloader_bank.location,
        config.bootloader_bank.size,
        linker_config.ram_origin + config.ram_bank.location,
        config.ram_bank.size,
        linker_config.has_ram_state,
    )
}

pub fn generate_application_script(config: Config, linker_config: LinkerConfig) -> String {
    // find the bootable image
    let bootable_firmware = config.boot_bank;

    generate_linker_script(
        linker_config.flash_origin + bootable_firmware.location,
        bootable_firmware.size,
        linker_config.ram_origin + config.ram_bank.location,
        config.ram_bank.size,
        linker_config.has_ram_state,
    )
}
