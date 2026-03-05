//! GBA hardware constants, register addresses, and memory region sizes.

// clock and timing constants
pub const CPU_FREQ: u32 = 16_777_216;
pub const VISUAL_FREQ: f32 = 59.7275;
pub const CYCLES_PER_FRAME: u32 = 280_896;
pub const CYCLES_PER_SCANLINE: u32 = 1232;
pub const VISIBLE_DOTS: u32 = 960;
pub const HBLANK_DOTS: u32 = 272;
pub const VISIBLE_LINES: u32 = 160;
pub const VBLANK_LINES: u32 = 68;
pub const TOTAL_LINES: u32 = 228;

// display constants
pub const DISPLAY_WIDTH: usize = 240;
pub const DISPLAY_HEIGHT: usize = 160;
pub const FRAME_BUFFER_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT * 3;

// memory region sizes
pub const BIOS_SIZE: usize = 0x4000;
pub const EWRAM_SIZE: usize = 0x40000;
pub const IWRAM_SIZE: usize = 0x8000;
pub const IO_SIZE: usize = 0x400;
pub const PALETTE_SIZE: usize = 0x400;
pub const VRAM_SIZE: usize = 0x18000;
pub const OAM_SIZE: usize = 0x400;
pub const ROM_MAX_SIZE: usize = 0x2000000;
pub const SRAM_SIZE: usize = 0x10000;

// memory region base addresses
pub const BIOS_BASE: u32 = 0x0000_0000;
pub const EWRAM_BASE: u32 = 0x0200_0000;
pub const IWRAM_BASE: u32 = 0x0300_0000;
pub const IO_BASE: u32 = 0x0400_0000;
pub const PALETTE_BASE: u32 = 0x0500_0000;
pub const VRAM_BASE: u32 = 0x0600_0000;
pub const OAM_BASE: u32 = 0x0700_0000;
pub const ROM_BASE: u32 = 0x0800_0000;
pub const SRAM_BASE: u32 = 0x0E00_0000;

// I/O register addresses (display)
pub const REG_DISPCNT: u32 = 0x0400_0000;
pub const REG_DISPSTAT: u32 = 0x0400_0004;
pub const REG_VCOUNT: u32 = 0x0400_0006;
pub const REG_BG0CNT: u32 = 0x0400_0008;
pub const REG_BG1CNT: u32 = 0x0400_000A;
pub const REG_BG2CNT: u32 = 0x0400_000C;
pub const REG_BG3CNT: u32 = 0x0400_000E;
pub const REG_BG0HOFS: u32 = 0x0400_0010;
pub const REG_BG0VOFS: u32 = 0x0400_0012;
pub const REG_BG1HOFS: u32 = 0x0400_0014;
pub const REG_BG1VOFS: u32 = 0x0400_0016;
pub const REG_BG2HOFS: u32 = 0x0400_0018;
pub const REG_BG2VOFS: u32 = 0x0400_001A;
pub const REG_BG3HOFS: u32 = 0x0400_001C;
pub const REG_BG3VOFS: u32 = 0x0400_001E;
pub const REG_BG2PA: u32 = 0x0400_0020;
pub const REG_BG2PB: u32 = 0x0400_0022;
pub const REG_BG2PC: u32 = 0x0400_0024;
pub const REG_BG2PD: u32 = 0x0400_0026;
pub const REG_BG2X: u32 = 0x0400_0028;
pub const REG_BG2Y: u32 = 0x0400_002C;
pub const REG_BG3PA: u32 = 0x0400_0030;
pub const REG_BG3PB: u32 = 0x0400_0032;
pub const REG_BG3PC: u32 = 0x0400_0034;
pub const REG_BG3PD: u32 = 0x0400_0036;
pub const REG_BG3X: u32 = 0x0400_0038;
pub const REG_BG3Y: u32 = 0x0400_003C;
pub const REG_WIN0H: u32 = 0x0400_0040;
pub const REG_WIN1H: u32 = 0x0400_0042;
pub const REG_WIN0V: u32 = 0x0400_0044;
pub const REG_WIN1V: u32 = 0x0400_0046;
pub const REG_WININ: u32 = 0x0400_0048;
pub const REG_WINOUT: u32 = 0x0400_004A;
pub const REG_MOSAIC: u32 = 0x0400_004C;
pub const REG_BLDCNT: u32 = 0x0400_0050;
pub const REG_BLDALPHA: u32 = 0x0400_0052;
pub const REG_BLDY: u32 = 0x0400_0054;

// I/O register addresses (sound)
pub const REG_SOUND1CNT_L: u32 = 0x0400_0060;
pub const REG_SOUND1CNT_H: u32 = 0x0400_0062;
pub const REG_SOUND1CNT_X: u32 = 0x0400_0064;
pub const REG_SOUND2CNT_L: u32 = 0x0400_0068;
pub const REG_SOUND2CNT_H: u32 = 0x0400_006C;
pub const REG_SOUND3CNT_L: u32 = 0x0400_0070;
pub const REG_SOUND3CNT_H: u32 = 0x0400_0072;
pub const REG_SOUND3CNT_X: u32 = 0x0400_0074;
pub const REG_SOUND4CNT_L: u32 = 0x0400_0078;
pub const REG_SOUND4CNT_H: u32 = 0x0400_007C;
pub const REG_SOUNDCNT_L: u32 = 0x0400_0080;
pub const REG_SOUNDCNT_H: u32 = 0x0400_0082;
pub const REG_SOUNDCNT_X: u32 = 0x0400_0084;
pub const REG_SOUNDBIAS: u32 = 0x0400_0088;
pub const REG_WAVE_RAM: u32 = 0x0400_0090;
pub const REG_FIFO_A: u32 = 0x0400_00A0;
pub const REG_FIFO_B: u32 = 0x0400_00A4;

// I/O register addresses (DMA)
pub const REG_DMA0SAD: u32 = 0x0400_00B0;
pub const REG_DMA0DAD: u32 = 0x0400_00B4;
pub const REG_DMA0CNT_L: u32 = 0x0400_00B8;
pub const REG_DMA0CNT_H: u32 = 0x0400_00BA;
pub const REG_DMA1SAD: u32 = 0x0400_00BC;
pub const REG_DMA1DAD: u32 = 0x0400_00C0;
pub const REG_DMA1CNT_L: u32 = 0x0400_00C4;
pub const REG_DMA1CNT_H: u32 = 0x0400_00C6;
pub const REG_DMA2SAD: u32 = 0x0400_00C8;
pub const REG_DMA2DAD: u32 = 0x0400_00CC;
pub const REG_DMA2CNT_L: u32 = 0x0400_00D0;
pub const REG_DMA2CNT_H: u32 = 0x0400_00D2;
pub const REG_DMA3SAD: u32 = 0x0400_00D4;
pub const REG_DMA3DAD: u32 = 0x0400_00D8;
pub const REG_DMA3CNT_L: u32 = 0x0400_00DC;
pub const REG_DMA3CNT_H: u32 = 0x0400_00DE;

// I/O register addresses (timer)
pub const REG_TM0CNT_L: u32 = 0x0400_0100;
pub const REG_TM0CNT_H: u32 = 0x0400_0102;
pub const REG_TM1CNT_L: u32 = 0x0400_0104;
pub const REG_TM1CNT_H: u32 = 0x0400_0106;
pub const REG_TM2CNT_L: u32 = 0x0400_0108;
pub const REG_TM2CNT_H: u32 = 0x0400_010A;
pub const REG_TM3CNT_L: u32 = 0x0400_010C;
pub const REG_TM3CNT_H: u32 = 0x0400_010E;

// I/O register addresses (serial / unused in this emulator)
pub const REG_SIOCNT: u32 = 0x0400_0128;
pub const REG_RCNT: u32 = 0x0400_0134;

// I/O register addresses (keypad)
pub const REG_KEYINPUT: u32 = 0x0400_0130;
pub const REG_KEYCNT: u32 = 0x0400_0132;

// I/O register addresses (interrupt)
pub const REG_IE: u32 = 0x0400_0200;
pub const REG_IF: u32 = 0x0400_0202;
pub const REG_WAITCNT: u32 = 0x0400_0204;
pub const REG_IME: u32 = 0x0400_0208;
pub const REG_POSTFLG: u32 = 0x0400_0300;
pub const REG_HALTCNT: u32 = 0x0400_0301;

// interrupt bit flags
pub const IRQ_VBLANK: u16 = 1 << 0;
pub const IRQ_HBLANK: u16 = 1 << 1;
pub const IRQ_VCOUNT: u16 = 1 << 2;
pub const IRQ_TIMER0: u16 = 1 << 3;
pub const IRQ_TIMER1: u16 = 1 << 4;
pub const IRQ_TIMER2: u16 = 1 << 5;
pub const IRQ_TIMER3: u16 = 1 << 6;
pub const IRQ_SERIAL: u16 = 1 << 7;
pub const IRQ_DMA0: u16 = 1 << 8;
pub const IRQ_DMA1: u16 = 1 << 9;
pub const IRQ_DMA2: u16 = 1 << 10;
pub const IRQ_DMA3: u16 = 1 << 11;
pub const IRQ_KEYPAD: u16 = 1 << 12;
pub const IRQ_GAMEPAK: u16 = 1 << 13;

// CPU mode constants
pub const MODE_USR: u32 = 0x10;
pub const MODE_FIQ: u32 = 0x11;
pub const MODE_IRQ: u32 = 0x12;
pub const MODE_SVC: u32 = 0x13;
pub const MODE_ABT: u32 = 0x17;
pub const MODE_UND: u32 = 0x1B;
pub const MODE_SYS: u32 = 0x1F;

// CPSR flag bits
pub const CPSR_N: u32 = 1 << 31;
pub const CPSR_Z: u32 = 1 << 30;
pub const CPSR_C: u32 = 1 << 29;
pub const CPSR_V: u32 = 1 << 28;
pub const CPSR_I: u32 = 1 << 7;
pub const CPSR_F: u32 = 1 << 6;
pub const CPSR_T: u32 = 1 << 5;
pub const CPSR_MODE_MASK: u32 = 0x1F;

// timer prescaler values
pub const TIMER_PRESCALERS: [u32; 4] = [1, 64, 256, 1024];

// DMA timing modes
pub const DMA_TIMING_IMMEDIATE: u16 = 0;
pub const DMA_TIMING_VBLANK: u16 = 1;
pub const DMA_TIMING_HBLANK: u16 = 2;
pub const DMA_TIMING_SPECIAL: u16 = 3;
