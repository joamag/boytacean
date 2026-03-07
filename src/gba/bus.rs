//! GBA memory bus with 32-bit address space.
//!
//! Dispatches read/write operations to the appropriate memory
//! region or I/O component based on the address.

use crate::gba::{
    apu::GbaApu,
    consts::{
        EWRAM_SIZE, IWRAM_SIZE, OAM_SIZE, PALETTE_SIZE, REG_BG0CNT, REG_BG0HOFS, REG_BG0VOFS,
        REG_BG1CNT, REG_BG1HOFS, REG_BG1VOFS, REG_BG2CNT, REG_BG2HOFS, REG_BG2PA, REG_BG2PB,
        REG_BG2PC, REG_BG2PD, REG_BG2VOFS, REG_BG2X, REG_BG2Y, REG_BG3CNT, REG_BG3HOFS, REG_BG3PA,
        REG_BG3PB, REG_BG3PC, REG_BG3PD, REG_BG3VOFS, REG_BG3X, REG_BG3Y, REG_BLDALPHA, REG_BLDCNT,
        REG_BLDY, REG_DISPCNT, REG_DISPSTAT, REG_DMA0CNT_H, REG_DMA0CNT_L, REG_DMA0DAD,
        REG_DMA0SAD, REG_DMA1CNT_H, REG_DMA1CNT_L, REG_DMA1DAD, REG_DMA1SAD, REG_DMA2CNT_H,
        REG_DMA2CNT_L, REG_DMA2DAD, REG_DMA2SAD, REG_DMA3CNT_H, REG_DMA3CNT_L, REG_DMA3DAD,
        REG_DMA3SAD, REG_FIFO_A, REG_FIFO_B, REG_HALTCNT, REG_IE, REG_IF, REG_IME, REG_KEYCNT,
        REG_KEYINPUT, REG_MOSAIC, REG_POSTFLG, REG_SOUNDBIAS, REG_SOUNDCNT_H, REG_SOUNDCNT_L,
        REG_SOUNDCNT_X, REG_TM0CNT_H, REG_TM0CNT_L, REG_TM1CNT_H, REG_TM1CNT_L, REG_TM2CNT_H,
        REG_TM2CNT_L, REG_TM3CNT_H, REG_TM3CNT_L, REG_VCOUNT, REG_WAITCNT, REG_WAVE_RAM, REG_WIN0H,
        REG_WIN0V, REG_WIN1H, REG_WIN1V, REG_WININ, REG_WINOUT, VRAM_SIZE,
    },
    dma::GbaDma,
    flash::SaveMedia,
    irq::IrqController,
    pad::GbaPad,
    ppu::GbaPpu,
    timer::GbaTimers,
};

/// Size of the BIOS stub (we only need a small region for HLE stubs)
const BIOS_SIZE: usize = 0x4000;

pub struct GbaBus {
    /// BIOS memory (16KB, contains HLE stubs for IRQ handler etc.)
    pub bios: Vec<u8>,

    /// external work RAM (256KB)
    pub ewram: Vec<u8>,

    /// internal work RAM (32KB)
    pub iwram: Vec<u8>,

    /// palette RAM (1KB)
    pub palette: Vec<u8>,

    /// video RAM (96KB)
    pub vram: Vec<u8>,

    /// OAM - object attribute memory (1KB)
    pub oam: Vec<u8>,

    /// cartridge ROM data
    pub rom: Vec<u8>,

    /// cartridge save media (SRAM/Flash)
    pub save: SaveMedia,

    /// PPU
    pub ppu: GbaPpu,

    /// APU
    pub apu: GbaApu,

    /// DMA controller
    pub dma: GbaDma,

    /// timers
    pub timers: GbaTimers,

    /// keypad input
    pub pad: GbaPad,

    /// interrupt controller
    pub irq: IrqController,

    /// wait state control register
    pub waitcnt: u16,

    /// post boot flag
    pub postflg: u8,

    /// halt flag (set via HALTCNT register)
    pub halt_requested: bool,

    /// last BIOS read value (for open bus emulation)
    bios_value: u32,

    /// Whether BIOS reads are currently allowed (true when CPU PC is in BIOS)
    pub bios_readable: bool,

    /// Whether a real BIOS ROM is loaded (disables HLE SWI handling)
    pub use_real_bios: bool,
}

impl GbaBus {
    pub fn new() -> Self {
        let mut bus = Self {
            bios: vec![0u8; BIOS_SIZE],
            ewram: vec![0u8; EWRAM_SIZE],
            iwram: vec![0u8; IWRAM_SIZE],
            palette: vec![0u8; PALETTE_SIZE],
            vram: vec![0u8; VRAM_SIZE],
            oam: vec![0u8; OAM_SIZE],
            rom: Vec::new(),
            save: SaveMedia::new(),
            ppu: GbaPpu::new(),
            apu: GbaApu::new(),
            dma: GbaDma::new(),
            timers: GbaTimers::new(),
            pad: GbaPad::new(),
            irq: IrqController::new(),
            waitcnt: 0,
            postflg: 0,
            halt_requested: false,
            bios_value: 0,
            bios_readable: false,
            use_real_bios: false,
        };
        bus.init_bios_stubs();
        bus
    }

    /// Writes ARM instruction word into BIOS memory at given address
    fn bios_write32(&mut self, addr: u32, value: u32) {
        let offset = addr as usize;
        if offset + 3 < self.bios.len() {
            self.bios[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
    }

    /// Initializes BIOS memory with HLE stubs for exception vectors
    /// and the IRQ handler that the real BIOS provides.
    fn init_bios_stubs(&mut self) {
        // IRQ vector at 0x18: branch to handler at 0x128
        // offset = (0x128 - 0x18 - 8) / 4 = 0x42
        self.bios_write32(0x18, 0xEA000042); // B #0x128

        // BIOS IRQ handler at 0x128 (matches real GBA BIOS):
        // Saves state, calls game handler via [0x03FFFFFC], restores
        // state, and returns from IRQ. The IntrCheck update at
        // 0x03007FF8 is the game's IRQ handler responsibility.
        //
        // 0x128: STMFD SP!, {R0-R3, R12, LR}
        // 0x12C: MOV R0, #0x04000000        ; I/O base
        // 0x130: ADD LR, PC, #0             ; LR = 0x138 (return point)
        // 0x134: LDR PC, [R0, #-4]          ; PC = [0x03FFFFFC] (game handler)
        // --- game handler returns here ---
        // 0x138: LDMFD SP!, {R0-R3, R12, LR}
        // 0x13C: SUBS PC, LR, #4            ; return from IRQ, restore CPSR

        self.bios_write32(0x128, 0xE92D500F); // STMFD SP!, {R0-R3, R12, LR}
        self.bios_write32(0x12C, 0xE3A00301); // MOV R0, #0x04000000
        self.bios_write32(0x130, 0xE28FE000); // ADD LR, PC, #0  (LR = 0x138)
        self.bios_write32(0x134, 0xE510F004); // LDR PC, [R0, #-4]
        self.bios_write32(0x138, 0xE8BD500F); // LDMFD SP!, {R0-R3, R12, LR}
        self.bios_write32(0x13C, 0xE25EF004); // SUBS PC, LR, #4
        self.bios_write32(0x140, 0xE55EC002); // Real BIOS value at 0x140 (for BIOS protection after IRQ)
        self.bios_write32(0x144, 0xE55EC002); // Real BIOS value at 0x144 (for BIOS protection after IRQ)

        // Write real BIOS values at addresses used by BIOS protection tests.
        // SWI handler return area (real BIOS address 0x190)
        self.bios_write32(0x190, 0xE3A02004);

        // Write real BIOS values at addresses used for BIOS protection after startup.
        // The real BIOS startup ends near 0xDC, where PC = 0xE4.
        self.bios_write32(0xDC, 0xE129F000); // Real BIOS value at 0xDC
        self.bios_write32(0xE0, 0xE129F000); // Real BIOS value at 0xE0
        self.bios_write32(0xE4, 0xE129F000); // Real BIOS value at 0xE4

        // After startup, set bios_value to what real BIOS leaves
        self.bios_value = 0xE129F000;
    }

    /// Loads a real BIOS ROM, replacing the HLE stubs.
    /// When a real BIOS is loaded, SWI instructions will execute
    /// through the actual BIOS code instead of HLE handlers.
    pub fn load_bios(&mut self, data: &[u8]) {
        let len = data.len().min(BIOS_SIZE);
        self.bios[..len].copy_from_slice(&data[..len]);
        self.use_real_bios = true;
        self.postflg = 0;
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        self.rom = data.to_vec();
        self.save.detect_save_type(data);
    }

    // 8-bit read
    pub fn read8(&mut self, addr: u32) -> u8 {
        match addr >> 24 {
            0x00 => {
                let offset = (addr & 0x3FFF) as usize;
                if self.bios_readable && offset < self.bios.len() {
                    self.bios[offset]
                } else {
                    (self.bios_value >> ((addr & 3) * 8)) as u8
                }
            }
            0x02 => self.ewram[(addr & 0x3FFFF) as usize],
            0x03 => self.iwram[(addr & 0x7FFF) as usize],
            0x04 => self.read_io8(addr),
            0x05 => self.palette[(addr & 0x3FF) as usize],
            0x06 => {
                let offset = self.mirror_vram(addr);
                self.vram[offset]
            }
            0x07 => self.oam[(addr & 0x3FF) as usize],
            0x08..=0x0D => {
                if self.save.is_eeprom_addr(addr) {
                    self.save.eeprom_read() as u8
                } else {
                    let offset = (addr & 0x01FFFFFF) as usize;
                    if offset < self.rom.len() {
                        self.rom[offset]
                    } else {
                        // open bus: 16-bit ROM bus returns halfword address
                        let halfword = ((addr / 2) & 0xFFFF) as u16;
                        let bytes = halfword.to_le_bytes();
                        bytes[(addr & 1) as usize]
                    }
                }
            }
            0x0E..=0x0F => self.save.read8(addr),
            _ => 0,
        }
    }

    // 16-bit read (aligned)
    pub fn read16(&mut self, addr: u32) -> u16 {
        let addr = addr & !1;
        match addr >> 24 {
            0x00 => {
                let offset = (addr & 0x3FFF) as usize;
                if self.bios_readable && offset + 1 < self.bios.len() {
                    u16::from_le_bytes([self.bios[offset], self.bios[offset + 1]])
                } else {
                    (self.bios_value >> ((addr & 2) * 8)) as u16
                }
            }
            0x02 => {
                let offset = (addr & 0x3FFFF) as usize;
                u16::from_le_bytes([self.ewram[offset], self.ewram[offset + 1]])
            }
            0x03 => {
                let offset = (addr & 0x7FFF) as usize;
                u16::from_le_bytes([self.iwram[offset], self.iwram[offset + 1]])
            }
            0x04 => self.read_io16(addr),
            0x05 => {
                let offset = (addr & 0x3FF) as usize;
                u16::from_le_bytes([self.palette[offset], self.palette[offset + 1]])
            }
            0x06 => {
                let offset = self.mirror_vram(addr);
                u16::from_le_bytes([self.vram[offset], self.vram[offset + 1]])
            }
            0x07 => {
                let offset = (addr & 0x3FF) as usize;
                u16::from_le_bytes([self.oam[offset], self.oam[offset + 1]])
            }
            0x08..=0x0D => {
                if self.save.is_eeprom_addr(addr) {
                    self.save.eeprom_read()
                } else {
                    let offset = (addr & 0x01FFFFFF) as usize;
                    if offset + 1 < self.rom.len() {
                        u16::from_le_bytes([self.rom[offset], self.rom[offset + 1]])
                    } else {
                        // open bus: 16-bit ROM bus returns halfword address
                        ((addr / 2) & 0xFFFF) as u16
                    }
                }
            }
            0x0E..=0x0F => {
                // 8-bit bus; 16-bit reads duplicate the byte
                let b = self.save.read8(addr) as u16;
                b | (b << 8)
            }
            _ => 0,
        }
    }

    // 32-bit read (aligned)
    pub fn read32(&mut self, addr: u32) -> u32 {
        let addr = addr & !3;
        match addr >> 24 {
            0x00 => {
                let offset = (addr & 0x3FFF) as usize;
                if self.bios_readable && offset + 3 < self.bios.len() {
                    u32::from_le_bytes([
                        self.bios[offset],
                        self.bios[offset + 1],
                        self.bios[offset + 2],
                        self.bios[offset + 3],
                    ])
                } else {
                    self.bios_value
                }
            }
            0x02 => {
                let offset = (addr & 0x3FFFF) as usize;
                u32::from_le_bytes([
                    self.ewram[offset],
                    self.ewram[offset + 1],
                    self.ewram[offset + 2],
                    self.ewram[offset + 3],
                ])
            }
            0x03 => {
                let offset = (addr & 0x7FFF) as usize;
                u32::from_le_bytes([
                    self.iwram[offset],
                    self.iwram[offset + 1],
                    self.iwram[offset + 2],
                    self.iwram[offset + 3],
                ])
            }
            0x04 => {
                let lo = self.read_io16(addr) as u32;
                let hi = self.read_io16(addr + 2) as u32;
                lo | (hi << 16)
            }
            0x05 => {
                let offset = (addr & 0x3FF) as usize;
                u32::from_le_bytes([
                    self.palette[offset],
                    self.palette[offset + 1],
                    self.palette[offset + 2],
                    self.palette[offset + 3],
                ])
            }
            0x06 => {
                let offset = self.mirror_vram(addr);
                u32::from_le_bytes([
                    self.vram[offset],
                    self.vram[offset + 1],
                    self.vram[offset + 2],
                    self.vram[offset + 3],
                ])
            }
            0x07 => {
                let offset = (addr & 0x3FF) as usize;
                u32::from_le_bytes([
                    self.oam[offset],
                    self.oam[offset + 1],
                    self.oam[offset + 2],
                    self.oam[offset + 3],
                ])
            }
            0x08..=0x0D => {
                if self.save.is_eeprom_addr(addr) {
                    self.save.eeprom_read() as u32
                } else {
                    let offset = (addr & 0x01FFFFFF) as usize;
                    if offset + 3 < self.rom.len() {
                        u32::from_le_bytes([
                            self.rom[offset],
                            self.rom[offset + 1],
                            self.rom[offset + 2],
                            self.rom[offset + 3],
                        ])
                    } else {
                        // open bus: two consecutive halfword addresses
                        let hw0 = (addr / 2) & 0xFFFF;
                        let hw1 = ((addr + 2) / 2) & 0xFFFF;
                        hw0 | (hw1 << 16)
                    }
                }
            }
            0x0E..=0x0F => {
                // 8-bit bus; 32-bit reads duplicate the byte
                let b = self.save.read8(addr) as u32;
                b * 0x01010101
            }
            _ => 0,
        }
    }

    // 8-bit write
    pub fn write8(&mut self, addr: u32, value: u8) {
        match addr >> 24 {
            0x02 => self.ewram[(addr & 0x3FFFF) as usize] = value,
            0x03 => self.iwram[(addr & 0x7FFF) as usize] = value,
            0x04 => self.write_io8(addr, value),
            0x05 => {
                // palette: 8-bit writes are mirrored to both bytes of the halfword
                let offset = (addr & 0x3FE) as usize;
                self.palette[offset] = value;
                self.palette[offset + 1] = value;
            }
            0x06 => {
                // VRAM: 8-bit writes to BG area are mirrored
                let offset = self.mirror_vram(addr);
                let aligned = offset & !1;
                self.vram[aligned] = value;
                self.vram[aligned + 1] = value;
            }
            0x0E..=0x0F => self.save.write8(addr, value),
            _ => {}
        }
    }

    // 16-bit write (aligned)
    pub fn write16(&mut self, addr: u32, value: u16) {
        let raw_addr = addr;
        let addr = addr & !1;
        let bytes = value.to_le_bytes();
        match addr >> 24 {
            0x02 => {
                let offset = (addr & 0x3FFFF) as usize;
                self.ewram[offset] = bytes[0];
                self.ewram[offset + 1] = bytes[1];
            }
            0x03 => {
                let offset = (addr & 0x7FFF) as usize;
                self.iwram[offset] = bytes[0];
                self.iwram[offset + 1] = bytes[1];
            }
            0x04 => self.write_io16(addr, value),
            0x05 => {
                let offset = (addr & 0x3FF) as usize;
                self.palette[offset] = bytes[0];
                self.palette[offset + 1] = bytes[1];
            }
            0x06 => {
                let offset = self.mirror_vram(addr);
                self.vram[offset] = bytes[0];
                self.vram[offset + 1] = bytes[1];
            }
            0x07 => {
                let offset = (addr & 0x3FF) as usize;
                self.oam[offset] = bytes[0];
                self.oam[offset + 1] = bytes[1];
            }
            0x08..=0x0D if self.save.is_eeprom_addr(raw_addr) => {
                self.save.eeprom_write(value);
            }
            0x0E..=0x0F => {
                // 8-bit bus; byte lane selected by original addr bit 0
                self.save.write8(raw_addr, bytes[(raw_addr & 1) as usize]);
            }
            _ => {}
        }
    }

    // 32-bit write (aligned)
    pub fn write32(&mut self, addr: u32, value: u32) {
        let raw_addr = addr;
        let addr = addr & !3;
        let bytes = value.to_le_bytes();
        match addr >> 24 {
            0x02 => {
                let offset = (addr & 0x3FFFF) as usize;
                self.ewram[offset..offset + 4].copy_from_slice(&bytes);
            }
            0x03 => {
                let offset = (addr & 0x7FFF) as usize;
                self.iwram[offset..offset + 4].copy_from_slice(&bytes);
            }
            0x04 => {
                self.write_io16(addr, value as u16);
                self.write_io16(addr + 2, (value >> 16) as u16);
            }
            0x05 => {
                let offset = (addr & 0x3FF) as usize;
                self.palette[offset..offset + 4].copy_from_slice(&bytes);
            }
            0x06 => {
                let offset = self.mirror_vram(addr);
                self.vram[offset..offset + 4].copy_from_slice(&bytes);
            }
            0x07 => {
                let offset = (addr & 0x3FF) as usize;
                self.oam[offset..offset + 4].copy_from_slice(&bytes);
            }
            0x0E..=0x0F => {
                // 8-bit bus; byte lane selected by original addr bits 0-1
                self.save.write8(raw_addr, bytes[(raw_addr & 3) as usize]);
            }
            _ => {}
        }
    }

    /// mirrors VRAM address (96KB total: 64KB + 32KB mirrored)
    fn mirror_vram(&self, addr: u32) -> usize {
        let offset = (addr & 0x1FFFF) as usize;
        if offset >= VRAM_SIZE {
            offset - 0x8000 // mirror the last 32KB
        } else {
            offset
        }
    }

    // I/O register reads

    fn read_io8(&self, addr: u32) -> u8 {
        let value = self.read_io16(addr & !1);
        if addr & 1 == 0 {
            value as u8
        } else {
            (value >> 8) as u8
        }
    }

    fn read_io16(&self, addr: u32) -> u16 {
        match addr {
            REG_DISPCNT => self.ppu.dispcnt(),
            REG_DISPSTAT => self.ppu.dispstat(),
            REG_VCOUNT => self.ppu.vcount(),
            REG_BG0CNT => self.ppu.bgcnt(0),
            REG_BG1CNT => self.ppu.bgcnt(1),
            REG_BG2CNT => self.ppu.bgcnt(2),
            REG_BG3CNT => self.ppu.bgcnt(3),
            REG_KEYINPUT => self.pad.keyinput(),
            REG_KEYCNT => self.pad.keycnt(),
            REG_IE => self.irq.ie(),
            REG_IF => self.irq.if_(),
            REG_IME => self.irq.ime() as u16,
            REG_WAITCNT => self.waitcnt,
            REG_SOUNDCNT_L => self.apu.soundcnt_l(),
            REG_SOUNDCNT_H => self.apu.soundcnt_h(),
            REG_SOUNDCNT_X => self.apu.soundcnt_x(),
            REG_SOUNDBIAS => self.apu.soundbias(),
            REG_TM0CNT_L => self.timers.timers[0].counter(),
            REG_TM0CNT_H => self.timers.timers[0].control(),
            REG_TM1CNT_L => self.timers.timers[1].counter(),
            REG_TM1CNT_H => self.timers.timers[1].control(),
            REG_TM2CNT_L => self.timers.timers[2].counter(),
            REG_TM2CNT_H => self.timers.timers[2].control(),
            REG_TM3CNT_L => self.timers.timers[3].counter(),
            REG_TM3CNT_H => self.timers.timers[3].control(),
            REG_DMA0CNT_H => self.dma.channels[0].control(),
            REG_DMA1CNT_H => self.dma.channels[1].control(),
            REG_DMA2CNT_H => self.dma.channels[2].control(),
            REG_DMA3CNT_H => self.dma.channels[3].control(),
            REG_POSTFLG => self.postflg as u16,
            _ => {
                // wave RAM
                if (REG_WAVE_RAM..REG_WAVE_RAM + 16).contains(&addr) {
                    let offset = (addr - REG_WAVE_RAM) as usize;
                    let lo = self.apu.read_wave_ram(offset) as u16;
                    let hi = self.apu.read_wave_ram(offset + 1) as u16;
                    lo | (hi << 8)
                } else {
                    0
                }
            }
        }
    }

    // I/O register writes

    fn write_io8(&mut self, addr: u32, value: u8) {
        // for sound channel registers, write directly
        let io_offset = addr & 0x3FF;
        if (0x60..=0x7D).contains(&io_offset) {
            self.apu.write_channel_reg(addr, value);
            return;
        }

        // HALTCNT is a write-only 8-bit register at 0x04000301
        // that shares the 16-bit address space with POSTFLG (0x04000300)
        if addr == REG_HALTCNT {
            self.halt_requested = true;
            return;
        }

        // for other registers, do a read-modify-write of the 16-bit register
        let aligned = addr & !1;
        let old = self.read_io16(aligned);
        let new_value = if addr & 1 == 0 {
            (old & 0xFF00) | value as u16
        } else {
            (old & 0x00FF) | ((value as u16) << 8)
        };
        self.write_io16(aligned, new_value);
    }

    fn write_io16(&mut self, addr: u32, value: u16) {
        match addr {
            REG_DISPCNT => self.ppu.set_dispcnt(value),
            REG_DISPSTAT => self.ppu.set_dispstat(value),
            REG_BG0CNT => self.ppu.set_bgcnt(0, value),
            REG_BG1CNT => self.ppu.set_bgcnt(1, value),
            REG_BG2CNT => self.ppu.set_bgcnt(2, value),
            REG_BG3CNT => self.ppu.set_bgcnt(3, value),
            REG_BG0HOFS => self.ppu.set_bg_hofs(0, value),
            REG_BG0VOFS => self.ppu.set_bg_vofs(0, value),
            REG_BG1HOFS => self.ppu.set_bg_hofs(1, value),
            REG_BG1VOFS => self.ppu.set_bg_vofs(1, value),
            REG_BG2HOFS => self.ppu.set_bg_hofs(2, value),
            REG_BG2VOFS => self.ppu.set_bg_vofs(2, value),
            REG_BG3HOFS => self.ppu.set_bg_hofs(3, value),
            REG_BG3VOFS => self.ppu.set_bg_vofs(3, value),
            REG_BG2PA => self.ppu.set_bg_pa(0, value),
            REG_BG2PB => self.ppu.set_bg_pb(0, value),
            REG_BG2PC => self.ppu.set_bg_pc(0, value),
            REG_BG2PD => self.ppu.set_bg_pd(0, value),
            REG_BG3PA => self.ppu.set_bg_pa(1, value),
            REG_BG3PB => self.ppu.set_bg_pb(1, value),
            REG_BG3PC => self.ppu.set_bg_pc(1, value),
            REG_BG3PD => self.ppu.set_bg_pd(1, value),
            REG_WIN0H => self.ppu.set_winh(0, value),
            REG_WIN1H => self.ppu.set_winh(1, value),
            REG_WIN0V => self.ppu.set_winv(0, value),
            REG_WIN1V => self.ppu.set_winv(1, value),
            REG_WININ => self.ppu.set_winin(value),
            REG_WINOUT => self.ppu.set_winout(value),
            REG_MOSAIC => self.ppu.set_mosaic(value),
            REG_BLDCNT => self.ppu.set_bldcnt(value),
            REG_BLDALPHA => self.ppu.set_bldalpha(value),
            REG_BLDY => self.ppu.set_bldy(value),
            REG_SOUNDCNT_L => self.apu.set_soundcnt_l(value),
            REG_SOUNDCNT_H => self.apu.set_soundcnt_h(value),
            REG_SOUNDCNT_X => self.apu.set_soundcnt_x(value),
            REG_SOUNDBIAS => self.apu.set_soundbias(value),
            REG_TM0CNT_L => self.timers.timers[0].set_reload(value),
            REG_TM0CNT_H => self.timers.timers[0].set_control(value),
            REG_TM1CNT_L => self.timers.timers[1].set_reload(value),
            REG_TM1CNT_H => self.timers.timers[1].set_control(value),
            REG_TM2CNT_L => self.timers.timers[2].set_reload(value),
            REG_TM2CNT_H => self.timers.timers[2].set_control(value),
            REG_TM3CNT_L => self.timers.timers[3].set_reload(value),
            REG_TM3CNT_H => self.timers.timers[3].set_control(value),
            REG_DMA0CNT_L => self.dma.channels[0].set_count_reg(value),
            REG_DMA0CNT_H => self.dma.channels[0].set_control(value, 0),
            REG_DMA1CNT_L => self.dma.channels[1].set_count_reg(value),
            REG_DMA1CNT_H => self.dma.channels[1].set_control(value, 1),
            REG_DMA2CNT_L => self.dma.channels[2].set_count_reg(value),
            REG_DMA2CNT_H => self.dma.channels[2].set_control(value, 2),
            REG_DMA3CNT_L => self.dma.channels[3].set_count_reg(value),
            REG_DMA3CNT_H => self.dma.channels[3].set_control(value, 3),
            REG_KEYCNT => self.pad.set_keycnt(value),
            REG_IE => self.irq.set_ie(value),
            REG_IF => {
                // Before acknowledging, compute which interrupts were both
                // enabled and pending — these are the ones being serviced.
                // Update IntrCheck at 0x03007FF8 (IWRAM mirror) so that
                // games using VBlankIntrWait / IntrWait can detect them.
                let serviced = self.irq.ie() & self.irq.if_() & value;
                if serviced != 0 {
                    let offset = (0x03007FF8u32 & 0x7FFF) as usize;
                    let old = u16::from_le_bytes([self.iwram[offset], self.iwram[offset + 1]]);
                    let new_val = old | serviced;
                    let bytes = new_val.to_le_bytes();
                    self.iwram[offset] = bytes[0];
                    self.iwram[offset + 1] = bytes[1];
                }
                self.irq.ack_if(value);
            }
            REG_IME => self.irq.set_ime(value & 1 != 0),
            REG_WAITCNT => self.waitcnt = value,
            REG_POSTFLG => self.postflg = value as u8,
            REG_HALTCNT => self.halt_requested = true,
            _ => {
                // handle 32-bit DMA address registers
                // (these are written as two 16-bit writes)
                self.write_dma_addr(addr, value);

                // handle 32-bit BG reference point registers
                self.write_bg_ref(addr, value);

                // handle FIFO writes
                if addr == REG_FIFO_A || addr == REG_FIFO_A + 2 {
                    self.apu.direct_sound[0].write_fifo(value as u32);
                } else if addr == REG_FIFO_B || addr == REG_FIFO_B + 2 {
                    self.apu.direct_sound[1].write_fifo(value as u32);
                }

                // wave RAM
                if (REG_WAVE_RAM..REG_WAVE_RAM + 16).contains(&addr) {
                    let offset = (addr - REG_WAVE_RAM) as usize;
                    self.apu.write_wave_ram(offset, value as u8);
                    self.apu.write_wave_ram(offset + 1, (value >> 8) as u8);
                }
            }
        }
    }

    /// handles 32-bit DMA address register writes (split across two 16-bit writes)
    fn write_dma_addr(&mut self, addr: u32, value: u16) {
        match addr {
            REG_DMA0SAD => {
                let old = self.dma.channels[0].src_reg();
                self.dma.channels[0].set_src_reg((old & 0xFFFF0000) | value as u32);
            }
            a if a == REG_DMA0SAD + 2 => {
                let old = self.dma.channels[0].src_reg();
                self.dma.channels[0].set_src_reg((old & 0x0000FFFF) | ((value as u32) << 16));
            }
            REG_DMA0DAD => {
                let old = self.dma.channels[0].dst_reg();
                self.dma.channels[0].set_dst_reg((old & 0xFFFF0000) | value as u32);
            }
            a if a == REG_DMA0DAD + 2 => {
                let old = self.dma.channels[0].dst_reg();
                self.dma.channels[0].set_dst_reg((old & 0x0000FFFF) | ((value as u32) << 16));
            }
            REG_DMA1SAD => {
                let old = self.dma.channels[1].src_reg();
                self.dma.channels[1].set_src_reg((old & 0xFFFF0000) | value as u32);
            }
            a if a == REG_DMA1SAD + 2 => {
                let old = self.dma.channels[1].src_reg();
                self.dma.channels[1].set_src_reg((old & 0x0000FFFF) | ((value as u32) << 16));
            }
            REG_DMA1DAD => {
                let old = self.dma.channels[1].dst_reg();
                self.dma.channels[1].set_dst_reg((old & 0xFFFF0000) | value as u32);
            }
            a if a == REG_DMA1DAD + 2 => {
                let old = self.dma.channels[1].dst_reg();
                self.dma.channels[1].set_dst_reg((old & 0x0000FFFF) | ((value as u32) << 16));
            }
            REG_DMA2SAD => {
                let old = self.dma.channels[2].src_reg();
                self.dma.channels[2].set_src_reg((old & 0xFFFF0000) | value as u32);
            }
            a if a == REG_DMA2SAD + 2 => {
                let old = self.dma.channels[2].src_reg();
                self.dma.channels[2].set_src_reg((old & 0x0000FFFF) | ((value as u32) << 16));
            }
            REG_DMA2DAD => {
                let old = self.dma.channels[2].dst_reg();
                self.dma.channels[2].set_dst_reg((old & 0xFFFF0000) | value as u32);
            }
            a if a == REG_DMA2DAD + 2 => {
                let old = self.dma.channels[2].dst_reg();
                self.dma.channels[2].set_dst_reg((old & 0x0000FFFF) | ((value as u32) << 16));
            }
            REG_DMA3SAD => {
                let old = self.dma.channels[3].src_reg();
                self.dma.channels[3].set_src_reg((old & 0xFFFF0000) | value as u32);
            }
            a if a == REG_DMA3SAD + 2 => {
                let old = self.dma.channels[3].src_reg();
                self.dma.channels[3].set_src_reg((old & 0x0000FFFF) | ((value as u32) << 16));
            }
            REG_DMA3DAD => {
                let old = self.dma.channels[3].dst_reg();
                self.dma.channels[3].set_dst_reg((old & 0xFFFF0000) | value as u32);
            }
            a if a == REG_DMA3DAD + 2 => {
                let old = self.dma.channels[3].dst_reg();
                self.dma.channels[3].set_dst_reg((old & 0x0000FFFF) | ((value as u32) << 16));
            }
            _ => {}
        }
    }

    /// handles 32-bit BG reference point register writes
    fn write_bg_ref(&mut self, addr: u32, value: u16) {
        match addr {
            REG_BG2X => self.ppu.set_bg_ref_x_lo(0, value),
            a if a == REG_BG2X + 2 => self.ppu.set_bg_ref_x_hi(0, value),
            REG_BG2Y => self.ppu.set_bg_ref_y_lo(0, value),
            a if a == REG_BG2Y + 2 => self.ppu.set_bg_ref_y_hi(0, value),
            REG_BG3X => self.ppu.set_bg_ref_x_lo(1, value),
            a if a == REG_BG3X + 2 => self.ppu.set_bg_ref_x_hi(1, value),
            REG_BG3Y => self.ppu.set_bg_ref_y_lo(1, value),
            a if a == REG_BG3Y + 2 => self.ppu.set_bg_ref_y_hi(1, value),
            _ => {}
        }
    }

    /// returns the number of cycles for a 16-bit (or 8-bit) access to
    /// the given address, accounting for WAITCNT settings.
    /// `sequential` indicates whether this is a sequential access.
    pub fn access_cycles_16(&self, addr: u32, sequential: bool) -> u32 {
        let region = (addr >> 24) as u8;
        match region {
            0x00 => 1, // BIOS
            0x01 => 1, // unused
            0x02 => 3, // EWRAM (16-bit bus, 3 cycles)
            0x03 => 1, // IWRAM
            0x04 => 1, // IO
            0x05 => 1, // Palette
            0x06 => 1, // VRAM
            0x07 => 1, // OAM
            0x08 | 0x09 => {
                // ROM WS0
                if sequential {
                    self.ws0_seq()
                } else {
                    self.ws0_nonseq()
                }
            }
            0x0A | 0x0B => {
                // ROM WS1
                if sequential {
                    self.ws1_seq()
                } else {
                    self.ws1_nonseq()
                }
            }
            0x0C | 0x0D => {
                // ROM WS2
                if sequential {
                    self.ws2_seq()
                } else {
                    self.ws2_nonseq()
                }
            }
            0x0E | 0x0F => self.sram_wait(), // SRAM
            _ => 1,
        }
    }

    /// returns the number of cycles for a 32-bit access to the given address.
    /// 32-bit regions with a 16-bit bus require two sequential accesses.
    pub fn access_cycles_32(&self, addr: u32, sequential: bool) -> u32 {
        let region = (addr >> 24) as u8;
        match region {
            0x00 => 1, // BIOS (32-bit bus)
            0x02 => 6, // EWRAM (16-bit bus: 2x3)
            0x03 => 1, // IWRAM (32-bit bus)
            0x04 => 1, // IO (32-bit bus)
            0x05 => 2, // Palette (16-bit bus)
            0x06 => 2, // VRAM (16-bit bus)
            0x07 => 1, // OAM (32-bit bus)
            0x08 | 0x09 => {
                // ROM WS0 (16-bit bus: N+S or S+S)
                if sequential {
                    self.ws0_seq() + self.ws0_seq()
                } else {
                    self.ws0_nonseq() + self.ws0_seq()
                }
            }
            0x0A | 0x0B => {
                // ROM WS1
                if sequential {
                    self.ws1_seq() + self.ws1_seq()
                } else {
                    self.ws1_nonseq() + self.ws1_seq()
                }
            }
            0x0C | 0x0D => {
                // ROM WS2
                if sequential {
                    self.ws2_seq() + self.ws2_seq()
                } else {
                    self.ws2_nonseq() + self.ws2_seq()
                }
            }
            0x0E | 0x0F => self.sram_wait(), // SRAM (8-bit bus)
            _ => 1,
        }
    }

    // WAITCNT decoding helpers
    // Values include the base 1-cycle access cost (total = 1 + wait_states)

    fn sram_wait(&self) -> u32 {
        [5, 4, 3, 9][(self.waitcnt & 0x03) as usize]
    }

    fn ws0_nonseq(&self) -> u32 {
        [5, 4, 3, 9][((self.waitcnt >> 2) & 0x03) as usize]
    }

    fn ws0_seq(&self) -> u32 {
        if self.waitcnt & (1 << 4) != 0 {
            2
        } else {
            3
        }
    }

    fn ws1_nonseq(&self) -> u32 {
        [5, 4, 3, 9][((self.waitcnt >> 5) & 0x03) as usize]
    }

    fn ws1_seq(&self) -> u32 {
        if self.waitcnt & (1 << 7) != 0 {
            2
        } else {
            5
        }
    }

    fn ws2_nonseq(&self) -> u32 {
        [5, 4, 3, 9][((self.waitcnt >> 8) & 0x03) as usize]
    }

    fn ws2_seq(&self) -> u32 {
        if self.waitcnt & (1 << 10) != 0 {
            2
        } else {
            9
        }
    }

    pub fn reset(&mut self) {
        self.ewram.fill(0);
        self.iwram.fill(0);
        self.palette.fill(0);
        self.vram.fill(0);
        self.oam.fill(0);
        self.save.reset();
        self.ppu.reset();
        self.apu.reset();
        self.dma = GbaDma::new();
        self.timers = GbaTimers::new();
        self.pad = GbaPad::new();
        self.irq = IrqController::new();
        self.waitcnt = 0;
        self.postflg = 0;
        self.halt_requested = false;
        self.bios_value = 0;
        self.bios_readable = false;
    }

    pub fn update_bios_value(&mut self, value: u32) {
        self.bios_value = value;
    }
}

impl Default for GbaBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::GbaBus;
    use crate::gba::consts::{REG_DISPCNT, REG_IE, REG_IME, REG_KEYINPUT};

    #[test]
    fn test_new() {
        let bus = GbaBus::new();
        assert!(bus.rom.is_empty());
        assert_eq!(bus.ewram.len(), 0x40000);
        assert_eq!(bus.iwram.len(), 0x8000);
        assert!(!bus.halt_requested);
    }

    #[test]
    fn test_ewram_read_write() {
        let mut bus = GbaBus::new();
        bus.write32(0x0200_0000, 0xDEADBEEF);
        assert_eq!(bus.read32(0x0200_0000), 0xDEADBEEF);
    }

    #[test]
    fn test_ewram_8bit() {
        let mut bus = GbaBus::new();
        bus.write8(0x0200_0000, 0x42);
        assert_eq!(bus.read8(0x0200_0000), 0x42);
    }

    #[test]
    fn test_iwram_read_write() {
        let mut bus = GbaBus::new();
        bus.write16(0x0300_0000, 0x1234);
        assert_eq!(bus.read16(0x0300_0000), 0x1234);
    }

    #[test]
    fn test_iwram_32bit() {
        let mut bus = GbaBus::new();
        bus.write32(0x0300_0000, 0xCAFEBABE);
        assert_eq!(bus.read32(0x0300_0000), 0xCAFEBABE);
    }

    #[test]
    fn test_ewram_mirror() {
        let mut bus = GbaBus::new();
        bus.write8(0x0200_0000, 0x42);
        assert_eq!(bus.read8(0x0204_0000), 0x42);
    }

    #[test]
    fn test_rom_read() {
        let mut bus = GbaBus::new();
        bus.load_rom(&[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(bus.read32(0x0800_0000), 0x78563412);
    }

    #[test]
    fn test_rom_out_of_bounds() {
        let mut bus = GbaBus::new();
        bus.load_rom(&[0x12, 0x34]);
        assert_eq!(bus.read32(0x0800_0000), 0x00010000);
        assert_eq!(bus.read8(0x0800_0000), 0x12);
        assert_eq!(bus.read8(0x0800_0010), ((0x0800_0010 / 2) & 0xFF) as u8);
    }

    #[test]
    fn test_palette_read_write() {
        let mut bus = GbaBus::new();
        bus.write16(0x0500_0000, 0x7FFF);
        assert_eq!(bus.read16(0x0500_0000), 0x7FFF);
    }

    #[test]
    fn test_palette_8bit_mirror() {
        let mut bus = GbaBus::new();
        bus.write8(0x0500_0000, 0x42);
        assert_eq!(bus.read8(0x0500_0000), 0x42);
        assert_eq!(bus.read8(0x0500_0001), 0x42);
    }

    #[test]
    fn test_vram_read_write() {
        let mut bus = GbaBus::new();
        bus.write16(0x0600_0000, 0x1234);
        assert_eq!(bus.read16(0x0600_0000), 0x1234);
    }

    #[test]
    fn test_oam_read_write() {
        let mut bus = GbaBus::new();
        bus.write32(0x0700_0000, 0x12345678);
        assert_eq!(bus.read32(0x0700_0000), 0x12345678);
    }

    #[test]
    fn test_sram_read_write() {
        let mut bus = GbaBus::new();
        let mut rom = vec![0u8; 512];
        rom[0x100..0x106].copy_from_slice(b"SRAM_V");
        bus.load_rom(&rom);
        bus.write8(0x0E00_0000, 0x42);
        assert_eq!(bus.read8(0x0E00_0000), 0x42);
    }

    #[test]
    fn test_sram_mirror() {
        let mut bus = GbaBus::new();
        let mut rom = vec![0u8; 512];
        rom[0x100..0x106].copy_from_slice(b"SRAM_V");
        bus.load_rom(&rom);
        bus.write8(0x0E00_0000, 0x01);
        // 0x0F region mirrors 0x0E (addr & 0xFFFF)
        assert_eq!(bus.read8(0x0F00_0000), 0x01);
    }

    #[test]
    fn test_sram_16bit_read_duplicates_byte() {
        let mut bus = GbaBus::new();
        let mut rom = vec![0u8; 512];
        rom[0x100..0x106].copy_from_slice(b"SRAM_V");
        bus.load_rom(&rom);
        bus.write8(0x0E00_0000, 0x42);
        assert_eq!(bus.read16(0x0E00_0000), 0x4242);
    }

    #[test]
    fn test_sram_32bit_read_duplicates_byte() {
        let mut bus = GbaBus::new();
        let mut rom = vec![0u8; 512];
        rom[0x100..0x106].copy_from_slice(b"SRAM_V");
        bus.load_rom(&rom);
        bus.write8(0x0E00_0000, 0xAB);
        assert_eq!(bus.read32(0x0E00_0000), 0xABABABAB);
    }

    #[test]
    fn test_sram_16bit_write_selects_byte_lane() {
        let mut bus = GbaBus::new();
        let mut rom = vec![0u8; 512];
        rom[0x100..0x106].copy_from_slice(b"SRAM_V");
        bus.load_rom(&rom);
        bus.write16(0x0E00_0000, 0xAABB);
        // byte lane 0 (even addr) -> low byte 0xBB
        assert_eq!(bus.read8(0x0E00_0000), 0xBB);
        // odd address -> high byte
        bus.write16(0x0E00_0001, 0xAABB);
        assert_eq!(bus.read8(0x0E00_0001), 0xAA);
    }

    #[test]
    fn test_sram_32bit_write_selects_byte_lane() {
        let mut bus = GbaBus::new();
        let mut rom = vec![0u8; 512];
        rom[0x100..0x106].copy_from_slice(b"SRAM_V");
        bus.load_rom(&rom);
        bus.write32(0x0E00_0000, 0xAABBCCDD);
        assert_eq!(bus.read8(0x0E00_0000), 0xDD);
        bus.write32(0x0E00_0001, 0xAABBCCDD);
        assert_eq!(bus.read8(0x0E00_0001), 0xCC);
        bus.write32(0x0E00_0002, 0xAABBCCDD);
        assert_eq!(bus.read8(0x0E00_0002), 0xBB);
        bus.write32(0x0E00_0003, 0xAABBCCDD);
        assert_eq!(bus.read8(0x0E00_0003), 0xAA);
    }

    #[test]
    fn test_flash64_via_bus() {
        let mut bus = GbaBus::new();
        let mut rom = vec![0u8; 512];
        rom[0x100..0x107].copy_from_slice(b"FLASH_V");
        bus.load_rom(&rom);
        // raw write should not work
        bus.write8(0x0E00_0000, 0x42);
        assert_eq!(bus.read8(0x0E00_0000), 0xFF);
        // command sequence should work
        bus.write8(0x0E00_5555, 0xAA);
        bus.write8(0x0E00_2AAA, 0x55);
        bus.write8(0x0E00_5555, 0xA0);
        bus.write8(0x0E00_0000, 0x42);
        assert_eq!(bus.read8(0x0E00_0000), 0x42);
    }

    #[test]
    fn test_flash64_chip_erase_via_bus() {
        let mut bus = GbaBus::new();
        let mut rom = vec![0u8; 512];
        rom[0x100..0x107].copy_from_slice(b"FLASH_V");
        bus.load_rom(&rom);
        // write a byte
        bus.write8(0x0E00_5555, 0xAA);
        bus.write8(0x0E00_2AAA, 0x55);
        bus.write8(0x0E00_5555, 0xA0);
        bus.write8(0x0E00_0000, 0x00);
        assert_eq!(bus.read8(0x0E00_0000), 0x00);
        // chip erase
        bus.write8(0x0E00_5555, 0xAA);
        bus.write8(0x0E00_2AAA, 0x55);
        bus.write8(0x0E00_5555, 0x80);
        bus.write8(0x0E00_5555, 0xAA);
        bus.write8(0x0E00_2AAA, 0x55);
        bus.write8(0x0E00_5555, 0x10);
        assert_eq!(bus.read8(0x0E00_0000), 0xFF);
    }

    #[test]
    fn test_flash64_16bit_read_duplicates_byte() {
        let mut bus = GbaBus::new();
        let mut rom = vec![0u8; 512];
        rom[0x100..0x107].copy_from_slice(b"FLASH_V");
        bus.load_rom(&rom);
        bus.write8(0x0E00_5555, 0xAA);
        bus.write8(0x0E00_2AAA, 0x55);
        bus.write8(0x0E00_5555, 0xA0);
        bus.write8(0x0E00_0000, 0x42);
        assert_eq!(bus.read16(0x0E00_0000), 0x4242);
    }

    #[test]
    fn test_no_save_type_returns_ff() {
        let mut bus = GbaBus::new();
        assert_eq!(bus.read8(0x0E00_0000), 0xFF);
    }

    #[test]
    fn test_load_rom_detects_save_type() {
        let mut bus = GbaBus::new();
        let mut rom = vec![0u8; 512];
        rom[0x100..0x106].copy_from_slice(b"SRAM_V");
        bus.load_rom(&rom);
        assert_eq!(bus.save.save_type(), crate::gba::flash::SaveType::Sram);
    }

    #[test]
    fn test_io_dispcnt() {
        let mut bus = GbaBus::new();
        bus.write16(REG_DISPCNT, 0x0403);
        assert_eq!(bus.read16(REG_DISPCNT), 0x0403);
    }

    #[test]
    fn test_io_irq_registers() {
        let mut bus = GbaBus::new();
        bus.write16(REG_IME, 0x0001);
        assert_eq!(bus.read16(REG_IME), 1);

        bus.write16(REG_IE, 0x0001);
        assert_eq!(bus.read16(REG_IE), 0x0001);
    }

    #[test]
    fn test_io_keyinput() {
        let mut bus = GbaBus::new();
        assert_eq!(bus.read16(REG_KEYINPUT), 0x03FF);
    }

    #[test]
    fn test_io_8bit_read() {
        let mut bus = GbaBus::new();
        bus.write16(REG_DISPCNT, 0x1234);
        assert_eq!(bus.read8(REG_DISPCNT), 0x34);
        assert_eq!(bus.read8(REG_DISPCNT + 1), 0x12);
    }

    #[test]
    fn test_io_32bit_read() {
        let mut bus = GbaBus::new();
        bus.write16(REG_DISPCNT, 0x1234);
        let val = bus.read32(REG_DISPCNT);
        assert_eq!(val & 0xFFFF, 0x1234);
    }

    #[test]
    fn test_unmapped_read() {
        let mut bus = GbaBus::new();
        assert_eq!(bus.read8(0x1000_0000), 0);
    }

    #[test]
    fn test_reset() {
        let mut bus = GbaBus::new();
        bus.write32(0x0200_0000, 0xDEADBEEF);
        bus.halt_requested = true;
        bus.reset();
        assert_eq!(bus.read32(0x0200_0000), 0);
        assert!(!bus.halt_requested);
    }

    #[test]
    fn test_bios_protection_read32_returns_bios_value() {
        let mut bus = GbaBus::new();
        // bios_readable is false by default; reading BIOS should return bios_value
        assert_eq!(bus.read32(0x0000_0000), 0xE129F000);
    }

    #[test]
    fn test_bios_protection_read16_returns_bios_value() {
        let mut bus = GbaBus::new();
        // low halfword of bios_value (0xE129F000)
        assert_eq!(bus.read16(0x0000_0000), 0xF000);
        // high halfword (addr & 2 = 2, shift by 16)
        assert_eq!(bus.read16(0x0000_0002), 0xE129);
    }

    #[test]
    fn test_bios_protection_read8_returns_bios_value() {
        let mut bus = GbaBus::new();
        assert_eq!(bus.read8(0x0000_0000), 0x00); // byte 0 of 0xE129F000
        assert_eq!(bus.read8(0x0000_0001), 0xF0); // byte 1
        assert_eq!(bus.read8(0x0000_0002), 0x29); // byte 2
        assert_eq!(bus.read8(0x0000_0003), 0xE1); // byte 3
    }

    #[test]
    fn test_bios_readable_allows_direct_access() {
        let mut bus = GbaBus::new();
        bus.bios_readable = true;
        // IRQ vector at 0x18 = 0xEA000042 (branch to 0x128)
        assert_eq!(bus.read32(0x0000_0018), 0xEA000042);
        bus.bios_readable = false;
        // with protection, returns bios_value instead
        assert_eq!(bus.read32(0x0000_0018), 0xE129F000);
    }

    #[test]
    fn test_bios_init_startup_value() {
        let mut bus = GbaBus::new();
        // after init, bios_value matches real GBA post-boot value
        assert_eq!(bus.read32(0x0000_0000), 0xE129F000);
    }

    #[test]
    fn test_bios_update_bios_value() {
        let mut bus = GbaBus::new();
        bus.update_bios_value(0xDEADBEEF);
        assert_eq!(bus.read32(0x0000_0000), 0xDEADBEEF);
    }

    #[test]
    fn test_bios_irq_handler_stubs() {
        let mut bus = GbaBus::new();
        bus.bios_readable = true;
        // verify IRQ handler stub at 0x128-0x13C
        assert_eq!(bus.read32(0x0000_0128), 0xE92D500F); // STMFD SP!, {R0-R3, R12, LR}
        assert_eq!(bus.read32(0x0000_012C), 0xE3A00301); // MOV R0, #0x04000000
        assert_eq!(bus.read32(0x0000_0130), 0xE28FE000); // ADD LR, PC, #0
        assert_eq!(bus.read32(0x0000_0134), 0xE510F004); // LDR PC, [R0, #-4]
        assert_eq!(bus.read32(0x0000_0138), 0xE8BD500F); // LDMFD SP!, {R0-R3, R12, LR}
        assert_eq!(bus.read32(0x0000_013C), 0xE25EF004); // SUBS PC, LR, #4
    }

    #[test]
    fn test_bios_swi_protection_value() {
        let mut bus = GbaBus::new();
        bus.bios_readable = true;
        // real BIOS value at SWI return area
        assert_eq!(bus.read32(0x0000_0190), 0xE3A02004);
    }

    #[test]
    fn test_iwram_mirror_0x03fffffc() {
        let mut bus = GbaBus::new();
        // write to 0x03007FFC, read from 0x03FFFFFC (IWRAM mirror)
        bus.write32(0x0300_7FFC, 0x12345678);
        assert_eq!(bus.read32(0x03FF_FFFC), 0x12345678);
    }

    #[test]
    fn test_eeprom_detection_via_load_rom() {
        let mut bus = GbaBus::new();
        let mut rom = vec![0u8; 512];
        rom[0x100..0x108].copy_from_slice(b"EEPROM_V");
        bus.load_rom(&rom);
        assert_eq!(bus.save.save_type(), crate::gba::flash::SaveType::Eeprom);
    }

    #[test]
    fn test_eeprom_read16_at_0x0d_returns_eeprom() {
        let mut bus = GbaBus::new();
        let mut rom = vec![0u8; 4 * 1024 * 1024];
        rom[0x100..0x108].copy_from_slice(b"EEPROM_V");
        bus.load_rom(&rom);
        // reading from 0x0D region should go through EEPROM (returns 1 when idle)
        let val = bus.read16(0x0D00_0000);
        assert_eq!(val & 1, 1);
    }

    #[test]
    fn test_eeprom_write16_at_0x0d_reaches_eeprom() {
        let mut bus = GbaBus::new();
        let mut rom = vec![0u8; 4 * 1024 * 1024]; // 4MB -> 14-bit addressing
        rom[0x100..0x108].copy_from_slice(b"EEPROM_V");
        bus.load_rom(&rom);
        // send read command: "11" + 14-bit addr (0) + 1 stop bit
        bus.write16(0x0D00_0000, 1); // bit 1
        bus.write16(0x0D00_0000, 1); // bit 1
                                     // send 14 address bits (0) + 1 stop bit
        for _ in 0..15 {
            bus.write16(0x0D00_0000, 0);
        }
        // should now be in ReadingData state, reads return dummy/data bits
        let val = bus.read16(0x0D00_0000);
        // first read is dummy bit (0)
        assert_eq!(val & 1, 0);
    }

    #[test]
    fn test_non_eeprom_rom_read_at_0x0d() {
        let mut bus = GbaBus::new();
        let mut rom = vec![0u8; 512];
        rom[0x100..0x106].copy_from_slice(b"SRAM_V");
        bus.load_rom(&rom);
        // 0x0D region for non-EEPROM games reads ROM (returns 0 past end)
        assert_eq!(bus.read16(0x0D00_0000), 0);
    }

    #[test]
    fn test_load_bios() {
        let mut bus = GbaBus::new();
        assert!(!bus.use_real_bios);
        let bios = vec![0x42u8; 0x4000];
        bus.load_bios(&bios);
        assert!(bus.use_real_bios);
        bus.bios_readable = true;
        assert_eq!(bus.read8(0x0000_0000), 0x42);
    }

    #[test]
    fn test_load_bios_replaces_stubs() {
        let mut bus = GbaBus::new();
        bus.bios_readable = true;
        // HLE stubs place IRQ vector at 0x18
        assert_eq!(bus.read32(0x0000_0018), 0xEA000042);

        // loading a real BIOS overwrites stubs
        let mut bios = vec![0u8; 0x4000];
        bios[0x18] = 0xAA;
        bios[0x19] = 0xBB;
        bios[0x1A] = 0xCC;
        bios[0x1B] = 0xDD;
        bus.load_bios(&bios);
        assert_eq!(bus.read32(0x0000_0018), 0xDDCCBBAA);
    }

    #[test]
    fn test_load_bios_use_real_bios_default_false() {
        let bus = GbaBus::new();
        assert!(!bus.use_real_bios);
    }
}
