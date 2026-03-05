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
        REG_WIN0V, REG_WIN1H, REG_WIN1V, REG_WININ, REG_WINOUT, SRAM_SIZE, VRAM_SIZE,
    },
    dma::GbaDma,
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

    /// SRAM (64KB, optional)
    pub sram: Vec<u8>,

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
            sram: vec![0xFFu8; SRAM_SIZE],
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

        // BIOS IRQ handler at 0x128:
        // Saves state, calls game handler, updates IntrCheck at 0x03007FF8,
        // restores state, and returns from IRQ.
        //
        // 0x128: STMFD SP!, {R0-R3, R12, LR}
        // 0x12C: MOV R0, #0x04000000        ; I/O base
        // 0x130: ADD LR, PC, #0             ; LR = 0x138 (return point)
        // 0x134: LDR PC, [R0, #-4]          ; PC = [0x03FFFFFC] (game handler)
        // --- game handler returns here ---
        // 0x138: MOV R0, #0x04000000        ; reload I/O base
        // 0x13C: LDR R1, [R0, #0x200]       ; R1 = IE (low 16) | IF (high 16)
        // 0x140: AND R1, R1, R1, LSR #16    ; R1 = IE & IF (acknowledged bits)
        // 0x144: LDR R0, [PC, #16]          ; R0 = 0x03007FF8 (from literal pool at 0x15C)
        // 0x148: LDRH R2, [R0]              ; R2 = current IntrCheck
        // 0x14C: ORR R2, R2, R1             ; R2 |= acknowledged interrupts
        // 0x150: STRH R2, [R0]              ; store updated IntrCheck
        // 0x154: LDMFD SP!, {R0-R3, R12, LR}
        // 0x158: SUBS PC, LR, #4            ; return from IRQ, restore CPSR
        // 0x15C: .word 0x03007FF8            ; literal pool

        self.bios_write32(0x128, 0xE92D500F); // STMFD SP!, {R0-R3, R12, LR}
        self.bios_write32(0x12C, 0xE3A00301); // MOV R0, #0x04000000
        self.bios_write32(0x130, 0xE28FE000); // ADD LR, PC, #0  (LR = 0x138)
        self.bios_write32(0x134, 0xE510F004); // LDR PC, [R0, #-4]

        // After game handler returns:
        self.bios_write32(0x138, 0xE3A00301); // MOV R0, #0x04000000
        self.bios_write32(0x13C, 0xE5901200); // LDR R1, [R0, #0x200]  (IE|IF as 32-bit)
        self.bios_write32(0x140, 0xE0011821); // AND R1, R1, R1, LSR #16  (IE & IF)
        self.bios_write32(0x144, 0xE59F0010); // LDR R0, [PC, #16]  (load 0x03007FF8 from 0x15C)
        self.bios_write32(0x148, 0xE1D020B0); // LDRH R2, [R0, #0]
        self.bios_write32(0x14C, 0xE1822001); // ORR R2, R2, R1
        self.bios_write32(0x150, 0xE1C020B0); // STRH R2, [R0, #0]
        self.bios_write32(0x154, 0xE8BD500F); // LDMFD SP!, {R0-R3, R12, LR}
        self.bios_write32(0x158, 0xE25EF004); // SUBS PC, LR, #4
        self.bios_write32(0x15C, 0x03007FF8); // literal: IntrCheck address
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        self.rom = data.to_vec();
    }

    // 8-bit read
    pub fn read8(&self, addr: u32) -> u8 {
        match addr >> 24 {
            0x00 => {
                let offset = (addr & 0x3FFF) as usize;
                if offset < self.bios.len() {
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
                let offset = (addr & 0x01FFFFFF) as usize;
                if offset < self.rom.len() {
                    self.rom[offset]
                } else {
                    0
                }
            }
            0x0E..=0x0F => {
                let offset = (addr & 0xFFFF) as usize;
                if offset < self.sram.len() {
                    self.sram[offset]
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    // 16-bit read (aligned)
    pub fn read16(&self, addr: u32) -> u16 {
        let addr = addr & !1;
        match addr >> 24 {
            0x00 => {
                let offset = (addr & 0x3FFF) as usize;
                if offset + 1 < self.bios.len() {
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
                let offset = (addr & 0x01FFFFFF) as usize;
                if offset + 1 < self.rom.len() {
                    u16::from_le_bytes([self.rom[offset], self.rom[offset + 1]])
                } else {
                    0
                }
            }
            0x0E..=0x0F => {
                // sram is 8-bit bus; 16-bit reads duplicate the byte
                let offset = (addr & 0xFFFF) as usize;
                if offset < self.sram.len() {
                    let b = self.sram[offset] as u16;
                    b | (b << 8)
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    // 32-bit read (aligned)
    pub fn read32(&self, addr: u32) -> u32 {
        let addr = addr & !3;
        match addr >> 24 {
            0x00 => {
                let offset = (addr & 0x3FFF) as usize;
                if offset + 3 < self.bios.len() {
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
                let offset = (addr & 0x01FFFFFF) as usize;
                if offset + 3 < self.rom.len() {
                    u32::from_le_bytes([
                        self.rom[offset],
                        self.rom[offset + 1],
                        self.rom[offset + 2],
                        self.rom[offset + 3],
                    ])
                } else {
                    0
                }
            }
            0x0E..=0x0F => {
                // sram is 8-bit bus; 32-bit reads duplicate the byte
                let offset = (addr & 0xFFFF) as usize;
                if offset < self.sram.len() {
                    let b = self.sram[offset] as u32;
                    b * 0x01010101
                } else {
                    0
                }
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
            0x0E..=0x0F => {
                let offset = (addr & 0xFFFF) as usize;
                if offset < self.sram.len() {
                    self.sram[offset] = value;
                }
            }
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
            0x0E..=0x0F => {
                // sram is 8-bit bus; byte lane selected by original addr bit 0
                let offset = (raw_addr & 0xFFFF) as usize;
                if offset < self.sram.len() {
                    self.sram[offset] = bytes[(raw_addr & 1) as usize];
                }
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
                // sram is 8-bit bus; byte lane selected by original addr bits 0-1
                let offset = (raw_addr & 0xFFFF) as usize;
                if offset < self.sram.len() {
                    self.sram[offset] = bytes[(raw_addr & 3) as usize];
                }
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

    pub fn reset(&mut self) {
        self.ewram.fill(0);
        self.iwram.fill(0);
        self.palette.fill(0);
        self.vram.fill(0);
        self.oam.fill(0);
        self.sram.fill(0xFF);
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
        assert_eq!(bus.read32(0x0800_0000), 0);
        assert_eq!(bus.read8(0x0800_0000), 0x12);
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
        bus.write8(0x0E00_0000, 0x42);
        assert_eq!(bus.read8(0x0E00_0000), 0x42);
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
        let bus = GbaBus::new();
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
        let bus = GbaBus::new();
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
}
