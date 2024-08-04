//! MMU (Memory Management Unit) functions and structures.

use std::sync::Mutex;

use crate::{
    apu::Apu,
    assert_pedantic_gb,
    dma::Dma,
    gb::{Components, GameBoyConfig, GameBoyMode, GameBoySpeed},
    pad::Pad,
    panic_gb,
    ppu::Ppu,
    rom::Cartridge,
    serial::Serial,
    timer::Timer,
    util::SharedThread,
    warnln,
};

pub const BOOT_SIZE_DMG: usize = 256;
pub const BOOT_SIZE_CGB: usize = 2304;

pub const RAM_SIZE_DMG: usize = 8192;
pub const RAM_SIZE_CGB: usize = 32768;

pub trait BusComponent {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
    fn read_many(&self, addr: u16, count: usize) -> Vec<u8> {
        (0..count)
            .map(|offset| self.read(addr + offset as u16))
            .collect()
    }
    fn write_many(&mut self, addr: u16, values: &[u8]) {
        for (offset, &value) in values.iter().enumerate() {
            self.write(addr + offset as u16, value);
        }
    }
}

pub struct Mmu {
    /// Register that controls the interrupts that are considered
    /// to be enabled and should be triggered.
    pub ie: u8,

    /// Register that controls the compatibility mode in use, this
    /// value comes directly from 0x0143 (CGB flag). The possible (and
    /// valid) values are: 0x80 for games that support CGB enhancements
    /// and 0xC0 for games that are compatible only with a CGB device
    /// (CGB only).
    pub key0: u8,

    /// Flags that controls if the system is currently in the process
    /// of switching between the double and single speed modes.
    pub switching: bool,

    /// The speed (frequency) at which the system is currently running,
    /// it may be either normal (4.194304 MHz) or double (8.388608 MHz).
    speed: GameBoySpeed,

    /// Callback to be called when the speed of the system changes, it
    /// should provide visibility over the current speed of the system.
    speed_callback: fn(speed: GameBoySpeed),

    /// Reference to the PPU (Pixel Processing Unit) that is going
    /// to be used both for VRAM reading/writing and to forward
    /// some of the access operations.
    ppu: Ppu,

    /// Reference to the APU (Audio Processing Unit) that is going
    /// to be used both for register reading/writing and to forward
    /// some of the access operations.
    apu: Apu,

    /// Reference to the DMA (Direct Memory Access) controller that is going
    /// to be used for quick and CPU offloaded memory transfers.
    /// There are multiple execution modes for the DMA.
    dma: Dma,

    /// Reference to the Gamepad structure that is going to control
    /// the I/O access to this device.
    pad: Pad,

    /// The timer controller to be used as part of the I/O access
    /// that is memory mapped.
    timer: Timer,

    /// The serial data transfer controller to be used to control the
    /// link cable connection, this component is memory mapped.
    serial: Serial,

    /// The cartridge ROM that is currently loaded into the system,
    /// going to be used to access ROM and external RAM banks.
    rom: Cartridge,

    /// Flag that control the access to the boot section in the
    /// 0x0000-0x00FE memory area, this flag should be unset after
    /// the boot sequence has been finished.
    boot_active: bool,

    /// Buffer to be used to store the boot ROM, this is the code
    /// that is going to be executed at the beginning of the Game
    /// Boy execution. The buffer effectively used is of 256 bytes
    /// for the "normal" Game Boy (MGB) and 2308 bytes for the
    /// Game Boy Color (CGB). Note that in the case of the CGB
    /// the BIOS which is 2308 bytes long is in fact only 2048 bytes
    /// as the 256 bytes in range 0x0100-0x01FF are meant to be
    /// overwritten byte the cartridge header.
    boot: Vec<u8>,

    /// Buffer that is used to store the RAM of the system, this
    /// value varies between DMG and CGB emulation, being 8KB for
    /// the DMG and 32KB for the CGB. Mapped in range 0xC000-0xDFFF.
    ram: Vec<u8>,

    /// The RAM bank to be used in the read and write operation of
    /// the 0xD000-0xDFFF memory range (CGB only).
    ram_bank: u8,

    /// The offset to be used in the read and write operation of
    /// the RAM, this value should be consistent with the RAM bank
    /// that is currently selected (CGB only).
    ram_offset: u16,

    /// The current running mode of the emulator, this
    /// may affect many aspects of the emulation.
    mode: GameBoyMode,

    /// The pointer to the parent configuration of the running
    /// Game Boy emulator, that can be used to control the behaviour
    /// of Game Boy emulation.
    gbc: SharedThread<GameBoyConfig>,
}

impl Mmu {
    pub fn new(
        components: Components,
        mode: GameBoyMode,
        gbc: SharedThread<GameBoyConfig>,
    ) -> Self {
        Self {
            ppu: components.ppu,
            apu: components.apu,
            dma: components.dma,
            pad: components.pad,
            timer: components.timer,
            serial: components.serial,
            rom: Cartridge::new(),
            boot_active: true,
            boot: vec![],
            ram: vec![],
            ram_bank: 0x1,
            ram_offset: 0x1000,
            ie: 0x0,
            key0: 0x0,
            speed: GameBoySpeed::Normal,
            switching: false,
            speed_callback: |_| {},
            mode,
            gbc,
        }
    }

    pub fn reset(&mut self) {
        self.rom = Cartridge::new();
        self.boot_active = true;
        self.boot = vec![];
        self.ram = vec![];
        self.ram_bank = 0x1;
        self.ram_offset = 0x1000;
        self.ie = 0x0;
        self.key0 = 0x0;
        self.speed = GameBoySpeed::Normal;
        self.switching = false;
    }

    pub fn allocate_default(&mut self) {
        self.allocate_dmg();
    }

    pub fn allocate_dmg(&mut self) {
        self.boot = vec![0x00; BOOT_SIZE_DMG];
        self.ram = vec![0x00; RAM_SIZE_DMG];
    }

    pub fn allocate_cgb(&mut self) {
        self.boot = vec![0x00; BOOT_SIZE_CGB];
        self.ram = vec![0x00; RAM_SIZE_CGB];
    }

    /// Notifies the system that a VBlank interrupt has been
    /// triggered, would usually be the perfect time to update
    /// some of the internal memory structures.
    pub fn vblank(&mut self) {
        let writes = self.rom.vblank();
        if let Some(writes) = writes {
            for (base_addr, addr, value) in writes {
                match base_addr {
                    0xa000 => self.rom.ram_data_mut()[addr as usize] = value,
                    0xc000 => self.ram[addr as usize] = value,
                    _ => panic_gb!("Invalid base address for write: 0x{:04x}", base_addr),
                }
            }
        }
    }

    /// Switches the current system's speed toggling between
    /// the normal and double speed modes.
    pub fn switch_speed(&mut self) {
        self.speed = self.speed.switch();
        self.switching = false;
        (self.speed_callback)(self.speed);
    }

    pub fn speed(&self) -> GameBoySpeed {
        self.speed
    }

    pub fn set_speed(&mut self, value: GameBoySpeed) {
        self.speed = value;
    }

    pub fn set_speed_callback(&mut self, callback: fn(speed: GameBoySpeed)) {
        self.speed_callback = callback;
    }

    pub fn ppu(&mut self) -> &mut Ppu {
        &mut self.ppu
    }

    pub fn ppu_i(&self) -> &Ppu {
        &self.ppu
    }

    pub fn apu(&mut self) -> &mut Apu {
        &mut self.apu
    }

    pub fn apu_i(&self) -> &Apu {
        &self.apu
    }

    pub fn dma(&mut self) -> &mut Dma {
        &mut self.dma
    }

    pub fn dma_i(&self) -> &Dma {
        &self.dma
    }

    pub fn pad(&mut self) -> &mut Pad {
        &mut self.pad
    }

    pub fn pad_i(&self) -> &Pad {
        &self.pad
    }

    pub fn timer(&mut self) -> &mut Timer {
        &mut self.timer
    }

    pub fn timer_i(&self) -> &Timer {
        &self.timer
    }

    pub fn serial(&mut self) -> &mut Serial {
        &mut self.serial
    }

    pub fn serial_i(&self) -> &Serial {
        &self.serial
    }

    pub fn boot_active(&self) -> bool {
        self.boot_active
    }

    pub fn set_boot_active(&mut self, value: bool) {
        self.boot_active = value;
    }

    pub fn clock_dma(&mut self, cycles: u16) {
        if !self.dma.active() {
            return;
        }

        if self.dma.active_dma() {
            let cycles_dma = self.dma.cycles_dma().saturating_sub(cycles);
            if cycles_dma == 0x0 {
                let data = self.read_many((self.dma.value_dma() as u16) << 8, 160);
                self.write_many(0xfe00, &data);
                self.dma.set_active_dma(false);
            }
            self.dma.set_cycles_dma(cycles_dma);
        }

        // @TODO: Implement DMA transfer in a better way, meaning that
        // the DMA transfer should respect the timings
        //
        // In both Normal Speed and Double Speed Mode it takes about 8 μs
        // to transfer a block of $10 bytes. That is, 8 M-cycles in Normal
        // Speed Mode [1], and 16 “fast” M-cycles in Double Speed Mode [2].
        // Older MBC controllers (like MBC1-3) and slower ROMs are not guaranteed
        // to support General Purpose or HBlank DMA, that’s because there are
        // always 2 bytes transferred per microsecond (even if the itself
        // program runs it Normal Speed Mode).
        //
        // we should also respect General-Purpose DMA vs HBlank DMA
        if self.dma.active_hdma() {
            // runs a series of pre-validation on the HDMA transfer in
            // pedantic mode is currently active (performance hit)
            assert_pedantic_gb!(
                (0x0000..=0x7ff0).contains(&self.dma.source())
                    || (0xa000..=0xdff0).contains(&self.dma.source()),
                "Invalid HDMA source start memory address 0x{:04x}",
                self.dma.source()
            );
            assert_pedantic_gb!(
                (0x8000..=0x9ff0).contains(&self.dma.destination()),
                "Invalid HDMA destination start memory address 0x{:04x}",
                self.dma.destination()
            );

            // only runs the DMA transfer if the system is in CGB mode
            // this avoids issues when writing to DMG unmapped registers
            // that would otherwise cause the system to crash
            if self.mode == GameBoyMode::Cgb {
                let data = self.read_many(self.dma.source(), self.dma.pending());
                self.write_many(self.dma.destination(), &data);
            }
            self.dma.set_pending(0);
            self.dma.set_active_hdma(false);
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            // 0x0000-0x0FFF - BOOT (256 B) + ROM0 (4 KB/16 KB)
            0x0000..=0x0fff => {
                // in case the boot mode is active and the
                // address is withing boot memory reads from it
                if self.boot_active && addr <= 0x00ff {
                    return self.boot[addr as usize];
                }
                if self.boot_active
                    && self.mode == GameBoyMode::Cgb
                    && (0x0200..=0x08ff).contains(&addr)
                {
                    return self.boot[addr as usize];
                }
                self.rom.read(addr)
            }

            // 0x1000-0x3FFF - ROM 0 (12 KB/16 KB)
            // 0x4000-0x7FFF - ROM 1 (Banked) (16 KB)
            0x1000..=0x7fff => self.rom.read(addr),

            // 0x8000-0x9FFF - Graphics: VRAM (8 KB)
            0x8000..=0x9fff => self.ppu.read(addr),

            // 0xA000-0xBFFF - External RAM (8 KB)
            0xa000..=0xbfff => self.rom.read(addr),

            // 0xC000-0xCFFF - Working RAM 0 (4 KB)
            0xc000..=0xcfff => self.ram[(addr & 0x0fff) as usize],

            // 0xD000..=0xDFFF - Working RAM 1 (Banked) (4KB)
            0xd000..=0xdfff => self.ram[(self.ram_offset + (addr & 0x0fff)) as usize],

            // 0xE000..=0xFDFF - Working RAM Shadow
            0xe000..=0xfdff => self.ram[(addr & 0x1fff) as usize],

            // 0xFE00-0xFE9F - Object attribute memory (OAM)
            0xfe00..=0xfe9f => self.ppu.read(addr),

            // 0xFEA0-0xFEFF - Not Usable
            0xfea0..=0xfeff => 0xff,

            // 0xFF00 - Joypad input
            0xff00 => self.pad.read(addr),

            // 0xFF01-0xFF02 - Serial data transfer
            0xff01..=0xff02 => self.serial.read(addr),

            // 0xFF04-0xFF07 - Timer and divider
            0xff04..=0xff07 => self.timer.read(addr),

            // 0xFF0F — IF: Interrupt flag
            0xff0f =>
            {
                #[allow(clippy::bool_to_int_with_if)]
                (if self.ppu.int_vblank() { 0x01 } else { 0x00 }
                    | if self.ppu.int_stat() { 0x02 } else { 0x00 }
                    | if self.timer.int_tima() { 0x04 } else { 0x00 }
                    | if self.serial.int_serial() { 0x08 } else { 0x00 }
                    | if self.pad.int_pad() { 0x10 } else { 0x00 }
                    | 0xe0)
            }

            // 0xFF10-0xFF26 — Audio
            // 0xFF10-0xFF26 — Wave pattern
            0xff10..=0xff26 | 0xff30..=0xff3f => self.apu.read(addr),

            // 0xFF40-0xFF45 - PPU registers
            // 0xFF47-0xFF4B - PPU registers
            0xff40..=0xff45 | 0xff47..=0xff4b => self.ppu.read(addr),

            // 0xFF46 — DMA: OAM DMA source address & start
            0xff46 => self.dma.read(addr),

            // 0xFF4C - KEY0: Compatibility flag (CGB only)
            0xff4c => self.key0,

            // 0xFF4D - KEY1: Speed switching (CGB only)
            0xff4d => (if self.switching { 0x01 } else { 0x00 }) | ((self.speed as u8) << 7) | 0x7e,

            // 0xFF4F - VRAM Bank Select (CGB only)
            0xff4f => self.ppu.read(addr),

            // 0xFF50 - Boot active flag
            0xff50 => u8::from(!self.boot_active),

            // 0xFF51-0xFF55 - VRAM DMA (HDMA) (CGB only)
            0xff51..=0xff55 => self.dma.read(addr),

            // 0xFF56 - RP: Infrared communications port (CGB only)
            0xff56 => 0xff,

            // 0xFF68-0xFF6B - BG / OBJ Palettes (CGB only)
            0xff68..=0xff6b => self.ppu.read(addr),

            // 0xFF70 - SVBK: WRAM bank (CGB only)
            0xff70 => (self.ram_bank & 0x07) | 0xf8,

            // 0xFF80-0xFFFE - High RAM (HRAM)
            0xff80..=0xfffe => self.ppu.read(addr),

            // 0xFFFF — IE: Interrupt enable
            0xffff => self.ie,

            addr => {
                warnln!("Reading from unknown location 0x{:04x}", addr);
                #[allow(unreachable_code)]
                0xff
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // 0x0000-0x0FFF - BOOT (256 B) + ROM0 (4 KB/16 KB)
            // 0x1000-0x3FFF - ROM 0 (12 KB/16 KB)
            // 0x4000-0x7FFF - ROM 1 (Banked) (16 KB)
            0x0000..=0x7fff => self.rom.write(addr, value),

            // 0x8000-0x9FFF - Graphics: VRAM (8 KB)
            0x8000..=0x9fff => self.ppu.write(addr, value),

            // 0xA000-0xBFFF - External RAM (8 KB)
            0xa000..=0xbfff => self.rom.write(addr, value),

            // 0xC000-0xCFFF - Working RAM 0 (4 KB)
            0xc000..=0xcfff => self.ram[(addr & 0x0fff) as usize] = value,

            // 0xD000..=0xDFFF - Working RAM 1 (Banked) (4KB)
            0xd000..=0xdfff => self.ram[(self.ram_offset + (addr & 0x0fff)) as usize] = value,

            // 0xE000..=0xFDFF - Working RAM Shadow
            0xe000..=0xfdff => self.ram[(addr & 0x1fff) as usize] = value,

            // 0xFE00-0xFE9F - Object attribute memory (OAM)
            0xfe00..=0xfe9f => self.ppu.write(addr, value),

            // 0xFEA0-0xFEFF - Not Usable
            0xfea0..=0xfeff => {}

            // 0xFF00 - Joypad input
            0xff00 => self.pad.write(addr, value),

            // 0xFF01-0xFF02 - Serial data transfer
            0xff01..=0xff02 => self.serial.write(addr, value),

            // 0xFF04-0xFF07 - Timer and divider
            0xff04..=0xff07 => self.timer.write(addr, value),

            // 0xFF0F — IF: Interrupt flag
            0xff0f => {
                self.ppu.set_int_vblank(value & 0x01 == 0x01);
                self.ppu.set_int_stat(value & 0x02 == 0x02);
                self.timer.set_int_tima(value & 0x04 == 0x04);
                self.serial.set_int_serial(value & 0x08 == 0x08);
                self.pad.set_int_pad(value & 0x10 == 0x10);
            }

            // 0xFF10-0xFF26 — Audio
            // 0xFF10-0xFF26 — Wave pattern
            0xff10..=0xff26 | 0xff30..=0xff3f => self.apu.write(addr, value),

            // 0xFF40-0xFF45 - PPU registers
            // 0xFF47-0xFF4B - PPU registers
            0xff40..=0xff45 | 0xff47..=0xff4b => self.ppu.write(addr, value),

            // 0xFF46 — DMA: OAM DMA source address & start
            0xff46 => self.dma.write(addr, value),

            // 0xFF4C - KEY0: Compatibility flag (CGB only)
            0xff4c => {
                self.key0 = value;
                if value == 0x04 {
                    self.ppu().set_dmg_compat(true);
                }
            }

            // 0xFF4D - KEY1: Speed switching (CGB only)
            0xff4d => self.switching = value & 0x01 == 0x01,

            // 0xFF4F - VRAM Bank Select (CGB only)
            0xff4f => self.ppu.write(addr, value),

            // 0xFF50 - Boot active flag
            0xff50 => self.boot_active = value == 0x00,

            // 0xFF51-0xFF55 - VRAM DMA (HDMA) (CGB only)
            0xff51..=0xff55 => self.dma.write(addr, value),

            // 0xFF56 - RP: Infrared communications port (CGB only)
            0xff56 => {}

            // 0xFF68-0xFF6B - BG / OBJ Palettes (CGB only)
            0xff68..=0xff6b => self.ppu.write(addr, value),

            // 0xFF70 - SVBK: WRAM bank (CGB only)
            0xff70 => {
                let mut ram_bank = value & 0x07;
                if ram_bank == 0x0 {
                    ram_bank = 0x1;
                }
                self.ram_bank = ram_bank;
                self.ram_offset = self.ram_bank as u16 * 0x1000;
            }

            // 0xFF80-0xFFFE - High RAM (HRAM)
            0xff80..=0xfffe => self.ppu.write(addr, value),

            // 0xFFFF — IE: Interrupt enable
            0xffff => self.ie = value,

            addr => warnln!("Writing to unknown location 0x{:04x}", addr),
        }
    }

    /// Reads a byte from a certain memory address, without the typical
    /// Game Boy verifications, allowing deep read of values.
    pub fn read_raw(&mut self, addr: u16) -> u8 {
        match addr {
            0xff10..=0xff3f => self.apu.read_raw(addr),
            _ => self.read(addr),
        }
    }

    /// Writes a byte to a certain memory address without the typical
    /// Game Boy verification process. This allows for faster memory
    /// access in registers and other memory areas that are typically
    /// inaccessible.
    pub fn write_raw(&mut self, addr: u16, value: u8) {
        match addr {
            0xff10..=0xff3f => self.apu.write_raw(addr, value),
            _ => self.write(addr, value),
        }
    }

    pub fn read_many(&mut self, addr: u16, count: u16) -> Vec<u8> {
        let mut data: Vec<u8> = vec![];

        for index in 0..count {
            let byte = self.read(addr + index);
            data.push(byte);
        }

        data
    }

    pub fn write_many(&mut self, addr: u16, data: &[u8]) {
        for (index, byte) in data.iter().enumerate() {
            self.write(addr + index as u16, *byte)
        }
    }

    pub fn read_many_unsafe(&mut self, addr: u16, count: u16) -> Vec<u8> {
        let mut data: Vec<u8> = vec![];

        for index in 0..count {
            let byte = self.read_raw(addr + index);
            data.push(byte);
        }

        data
    }

    pub fn write_many_unsafe(&mut self, addr: u16, data: &[u8]) {
        for (index, byte) in data.iter().enumerate() {
            self.write_raw(addr + index as u16, *byte)
        }
    }

    pub fn write_boot(&mut self, addr: u16, buffer: &[u8]) {
        self.boot[addr as usize..addr as usize + buffer.len()].clone_from_slice(buffer);
    }

    pub fn write_ram(&mut self, addr: u16, buffer: &[u8]) {
        self.ram[addr as usize..addr as usize + buffer.len()].clone_from_slice(buffer);
    }

    pub fn ram(&mut self) -> &mut Vec<u8> {
        &mut self.ram
    }

    pub fn ram_i(&self) -> &Vec<u8> {
        &self.ram
    }

    pub fn set_ram(&mut self, value: Vec<u8>) {
        self.ram = value;
    }

    pub fn rom(&mut self) -> &mut Cartridge {
        &mut self.rom
    }

    pub fn rom_i(&self) -> &Cartridge {
        &self.rom
    }

    pub fn set_rom(&mut self, rom: Cartridge) {
        self.rom = rom;
    }

    pub fn mode(&self) -> GameBoyMode {
        self.mode
    }

    pub fn set_mode(&mut self, value: GameBoyMode) {
        self.mode = value;
    }

    pub fn set_gbc(&mut self, value: SharedThread<GameBoyConfig>) {
        self.gbc = value;
    }
}

impl Default for Mmu {
    fn default() -> Self {
        let mode = GameBoyMode::Dmg;
        let gbc = SharedThread::new(Mutex::new(GameBoyConfig::default()));
        let components = Components {
            ppu: Ppu::new(mode, gbc.clone()),
            apu: Apu::default(),
            dma: Dma::default(),
            pad: Pad::default(),
            timer: Timer::default(),
            serial: Serial::default(),
        };
        Mmu::new(components, mode, gbc)
    }
}
