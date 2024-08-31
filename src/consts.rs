//! Game Boy specific hardware constants.

// Timer registers
pub const DIV_ADDR: u16 = 0xff04;
pub const TIMA_ADDR: u16 = 0xff05;
pub const TMA_ADDR: u16 = 0xff06;
pub const TAC_ADDR: u16 = 0xff07;
pub const IF_ADDR: u16 = 0xff0f;

// PPU registers
pub const LCDC_ADDR: u16 = 0xff40;
pub const STAT_ADDR: u16 = 0xff41;
pub const SCY_ADDR: u16 = 0xff42;
pub const SCX_ADDR: u16 = 0xff43;
pub const LY_ADDR: u16 = 0xff44;
pub const LYC_ADDR: u16 = 0xff45;
pub const BGP_ADDR: u16 = 0xff47;
pub const OBP0_ADDR: u16 = 0xff48;
pub const OBP1_ADDR: u16 = 0xff49;
pub const WX_ADDR: u16 = 0xff4a;
pub const WY_ADDR: u16 = 0xff4b;

// APU registers
pub const NR10_ADDR: u16 = 0xff10;
pub const NR11_ADDR: u16 = 0xff11;
pub const NR12_ADDR: u16 = 0xff12;
pub const NR13_ADDR: u16 = 0xff13;
pub const NR14_ADDR: u16 = 0xff14;
pub const NR20_ADDR: u16 = 0xff15;
pub const NR21_ADDR: u16 = 0xff16;
pub const NR22_ADDR: u16 = 0xff17;
pub const NR23_ADDR: u16 = 0xff18;
pub const NR24_ADDR: u16 = 0xff19;
pub const NR30_ADDR: u16 = 0xff1a;
pub const NR31_ADDR: u16 = 0xff1b;
pub const NR32_ADDR: u16 = 0xff1c;
pub const NR33_ADDR: u16 = 0xff1d;
pub const NR34_ADDR: u16 = 0xff1e;
pub const NR40_ADDR: u16 = 0xff1f;
pub const NR41_ADDR: u16 = 0xff20;
pub const NR42_ADDR: u16 = 0xff21;
pub const NR43_ADDR: u16 = 0xff22;
pub const NR44_ADDR: u16 = 0xff23;
pub const NR50_ADDR: u16 = 0xff24;
pub const NR51_ADDR: u16 = 0xff25;
pub const NR52_ADDR: u16 = 0xff26;

// DMA registers
pub const DMA_ADDR: u16 = 0xff46;
pub const HDMA1_ADDR: u16 = 0xff51;
pub const HDMA2_ADDR: u16 = 0xff52;
pub const HDMA3_ADDR: u16 = 0xff53;
pub const HDMA4_ADDR: u16 = 0xff54;
pub const HDMA5_ADDR: u16 = 0xff55;

// Serial registers
pub const SB_ADDR: u16 = 0xff01;
pub const SC_ADDR: u16 = 0xff02;
