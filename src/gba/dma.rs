//! GBA DMA controller (4 channels with priority and trigger modes).

use crate::gba::consts::{
    DMA_TIMING_HBLANK, DMA_TIMING_IMMEDIATE, DMA_TIMING_SPECIAL, DMA_TIMING_VBLANK, REG_FIFO_A,
    REG_FIFO_B,
};

/// address control mode for DMA source/destination
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DmaAddrControl {
    Increment = 0,
    Decrement = 1,
    Fixed = 2,
    IncrementReload = 3,
}

impl DmaAddrControl {
    pub fn from_u16(value: u16) -> Self {
        match value & 0x03 {
            0 => DmaAddrControl::Increment,
            1 => DmaAddrControl::Decrement,
            2 => DmaAddrControl::Fixed,
            3 => DmaAddrControl::IncrementReload,
            _ => unreachable!(),
        }
    }
}

pub struct DmaChannel {
    /// source address register (written by CPU)
    src_reg: u32,

    /// destination address register (written by CPU)
    dst_reg: u32,

    /// word count register (written by CPU)
    count_reg: u16,

    /// control register (DMA*CNT_H)
    control: u16,

    /// internal latched source address
    src: u32,

    /// internal latched destination address
    dst: u32,

    /// internal latched word count
    count: u32,

    /// whether this channel is currently active/pending
    active: bool,
}

impl DmaChannel {
    pub fn new() -> Self {
        Self {
            src_reg: 0,
            dst_reg: 0,
            count_reg: 0,
            control: 0,
            src: 0,
            dst: 0,
            count: 0,
            active: false,
        }
    }

    pub fn src_reg(&self) -> u32 {
        self.src_reg
    }

    pub fn set_src_reg(&mut self, value: u32) {
        self.src_reg = value;
    }

    pub fn dst_reg(&self) -> u32 {
        self.dst_reg
    }

    pub fn set_dst_reg(&mut self, value: u32) {
        self.dst_reg = value;
    }

    pub fn count_reg(&self) -> u16 {
        self.count_reg
    }

    pub fn set_count_reg(&mut self, value: u16) {
        self.count_reg = value;
    }

    pub fn control(&self) -> u16 {
        self.control
    }

    pub fn set_control(&mut self, value: u16, channel_index: usize) {
        let was_enabled = self.control & (1 << 15) != 0;
        self.control = value;
        let now_enabled = value & (1 << 15) != 0;

        // latch registers when transitioning from disabled to enabled
        if !was_enabled && now_enabled {
            self.src = self.src_reg;
            self.dst = self.dst_reg;
            self.count = if self.count_reg == 0 {
                // 0 means max count (0x4000 for DMA0-2, 0x10000 for DMA3)
                if channel_index == 3 {
                    0x10000
                } else {
                    0x4000
                }
            } else {
                self.count_reg as u32
            };

            // start immediately if timing mode is immediate
            if self.timing() == DMA_TIMING_IMMEDIATE {
                self.active = true;
            }
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn set_active(&mut self, value: bool) {
        self.active = value;
    }

    pub fn enabled(&self) -> bool {
        self.control & (1 << 15) != 0
    }

    pub fn timing(&self) -> u16 {
        (self.control >> 12) & 0x03
    }

    pub fn word_size(&self) -> bool {
        // false = 16-bit, true = 32-bit
        self.control & (1 << 10) != 0
    }

    pub fn repeat(&self) -> bool {
        self.control & (1 << 9) != 0
    }

    pub fn irq_enable(&self) -> bool {
        self.control & (1 << 14) != 0
    }

    pub fn src_control(&self) -> DmaAddrControl {
        DmaAddrControl::from_u16((self.control >> 7) & 0x03)
    }

    pub fn dst_control(&self) -> DmaAddrControl {
        DmaAddrControl::from_u16((self.control >> 5) & 0x03)
    }

    /// returns the internal source address
    pub fn src(&self) -> u32 {
        self.src
    }

    /// returns the internal destination address
    pub fn dst(&self) -> u32 {
        self.dst
    }

    /// returns the remaining transfer count
    pub fn remaining(&self) -> u32 {
        self.count
    }

    /// Advances the DMA transfer by one unit,
    /// updating internal addresses and count.
    ///
    /// returns (src_addr, dst_addr, is_complete)
    pub fn step(&mut self) -> (u32, u32, bool) {
        let src_addr = self.src;
        let dst_addr = self.dst;
        let step = if self.word_size() { 4 } else { 2 };

        // updates source address
        match self.src_control() {
            DmaAddrControl::Increment => self.src = self.src.wrapping_add(step),
            DmaAddrControl::Decrement => self.src = self.src.wrapping_sub(step),
            DmaAddrControl::Fixed => {}
            DmaAddrControl::IncrementReload => self.src = self.src.wrapping_add(step),
        }

        // updates destination address (sound DMA forces Fixed destination)
        let is_sound_dma = self.timing() == DMA_TIMING_SPECIAL;
        if !is_sound_dma {
            match self.dst_control() {
                DmaAddrControl::Increment => self.dst = self.dst.wrapping_add(step),
                DmaAddrControl::Decrement => self.dst = self.dst.wrapping_sub(step),
                DmaAddrControl::Fixed => {}
                DmaAddrControl::IncrementReload => self.dst = self.dst.wrapping_add(step),
            }
        }

        self.count -= 1;
        let complete = self.count == 0;

        if complete {
            self.active = false;
            if self.repeat() && self.timing() != DMA_TIMING_IMMEDIATE {
                // re-latch count (and dst if IncrementReload)
                self.count = if self.count_reg == 0 {
                    0x4000 // simplified; DMA3 would be 0x10000
                } else {
                    self.count_reg as u32
                };
                if self.dst_control() == DmaAddrControl::IncrementReload {
                    self.dst = self.dst_reg;
                }
            } else {
                // disable the channel
                self.control &= !(1 << 15);
            }
        }

        (src_addr, dst_addr, complete)
    }
}

impl Default for DmaChannel {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GbaDma {
    pub channels: [DmaChannel; 4],
}

impl GbaDma {
    pub fn new() -> Self {
        Self {
            channels: [
                DmaChannel::new(),
                DmaChannel::new(),
                DmaChannel::new(),
                DmaChannel::new(),
            ],
        }
    }

    /// triggers DMA channels matching the given timing mode
    pub fn trigger(&mut self, timing: u16) {
        for channel in &mut self.channels {
            if channel.enabled() && channel.timing() == timing {
                channel.set_active(true);
            }
        }
    }

    /// triggers vblank DMA channels
    pub fn trigger_vblank(&mut self) {
        self.trigger(DMA_TIMING_VBLANK);
    }

    /// triggers hblank DMA channels
    pub fn trigger_hblank(&mut self) {
        self.trigger(DMA_TIMING_HBLANK);
    }

    /// Triggers sound FIFO DMA (special timing, channels 1-2).
    ///
    /// On real hardware, sound DMA forces 4-word (32-bit) transfers
    /// to the FIFO register. Only channels whose latched destination
    /// matches the requested FIFO address are triggered.
    pub fn trigger_sound_fifo(&mut self, fifo_index: usize) {
        let fifo_addr = if fifo_index == 0 {
            REG_FIFO_A
        } else {
            REG_FIFO_B
        };

        // only DMA channels 1 and 2 support sound FIFO (SPECIAL timing)
        for i in 1..=2 {
            let channel = &mut self.channels[i];
            if channel.enabled()
                && channel.timing() == DMA_TIMING_SPECIAL
                && channel.dst_reg == fifo_addr
            {
                // hardware forces 4-word transfer
                channel.count = 4;
                channel.set_active(true);
            }
        }
    }

    /// returns the index of the highest priority active DMA channel,
    /// or None if no channels are active
    pub fn highest_active(&self) -> Option<usize> {
        for (i, channel) in self.channels.iter().enumerate() {
            if channel.active() {
                return Some(i);
            }
        }
        None
    }
}

impl Default for GbaDma {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{DmaAddrControl, DmaChannel, GbaDma};
    use crate::gba::consts::{REG_FIFO_A, REG_FIFO_B};

    #[test]
    fn test_dma_channel_new() {
        let ch = DmaChannel::new();
        assert_eq!(ch.src_reg(), 0);
        assert_eq!(ch.dst_reg(), 0);
        assert_eq!(ch.count_reg(), 0);
        assert_eq!(ch.control(), 0);
        assert!(!ch.active());
        assert!(!ch.enabled());
    }

    #[test]
    fn test_dma_channel_registers() {
        let mut ch = DmaChannel::new();
        ch.set_src_reg(0x0800_0000);
        ch.set_dst_reg(0x0600_0000);
        ch.set_count_reg(256);
        assert_eq!(ch.src_reg(), 0x0800_0000);
        assert_eq!(ch.dst_reg(), 0x0600_0000);
        assert_eq!(ch.count_reg(), 256);
    }

    #[test]
    fn test_dma_control_fields() {
        let mut ch = DmaChannel::new();
        ch.set_count_reg(1);
        // enable(15), IRQ(14), 32bit(10), repeat(9), timing=vblank(12)
        ch.set_control((1 << 15) | (1 << 14) | (1 << 10) | (1 << 9) | (1 << 12), 0);
        assert!(ch.enabled());
        assert!(ch.irq_enable());
        assert!(ch.word_size()); // 32-bit
        assert!(ch.repeat());
        assert_eq!(ch.timing(), 1); // vblank
    }

    #[test]
    fn test_dma_addr_control() {
        assert_eq!(DmaAddrControl::from_u16(0), DmaAddrControl::Increment);
        assert_eq!(DmaAddrControl::from_u16(1), DmaAddrControl::Decrement);
        assert_eq!(DmaAddrControl::from_u16(2), DmaAddrControl::Fixed);
        assert_eq!(DmaAddrControl::from_u16(3), DmaAddrControl::IncrementReload);
    }

    #[test]
    fn test_dma_step_increment_16bit() {
        let mut ch = DmaChannel::new();
        ch.set_src_reg(0x0200_0000);
        ch.set_dst_reg(0x0600_0000);
        ch.set_count_reg(2);
        // enable, immediate, 16-bit, src/dst increment
        ch.set_control(1 << 15, 0);

        let (src, dst, complete) = ch.step();
        assert_eq!(src, 0x0200_0000);
        assert_eq!(dst, 0x0600_0000);
        assert!(!complete);

        let (src, dst, complete) = ch.step();
        assert_eq!(src, 0x0200_0002);
        assert_eq!(dst, 0x0600_0002);
        assert!(complete);
    }

    #[test]
    fn test_dma_step_32bit() {
        let mut ch = DmaChannel::new();
        ch.set_src_reg(0x0200_0000);
        ch.set_dst_reg(0x0600_0000);
        ch.set_count_reg(1);
        // enable, immediate, 32-bit
        ch.set_control((1 << 15) | (1 << 10), 0);

        let (src, dst, complete) = ch.step();
        assert_eq!(src, 0x0200_0000);
        assert_eq!(dst, 0x0600_0000);
        assert!(complete);
        // next addresses incremented by 4
        assert_eq!(ch.src(), 0x0200_0004);
        assert_eq!(ch.dst(), 0x0600_0004);
    }

    #[test]
    fn test_dma_step_fixed_addr() {
        let mut ch = DmaChannel::new();
        ch.set_src_reg(0x0200_0000);
        ch.set_dst_reg(0x0600_0000);
        ch.set_count_reg(2);
        // enable, immediate, 16-bit, dst fixed (bits 5-6 = 2)
        ch.set_control((1 << 15) | (2 << 5), 0);

        ch.step();
        assert_eq!(ch.dst(), 0x0600_0000); // dst unchanged
        assert_eq!(ch.src(), 0x0200_0002); // src incremented
    }

    #[test]
    fn test_dma_zero_count_max() {
        let mut ch = DmaChannel::new();
        ch.set_src_reg(0x0200_0000);
        ch.set_dst_reg(0x0600_0000);
        ch.set_count_reg(0); // 0 = max count
                             // enable, immediate
        ch.set_control(1 << 15, 0);
        assert_eq!(ch.remaining(), 0x4000); // channel 0

        let mut ch3 = DmaChannel::new();
        ch3.set_src_reg(0x0200_0000);
        ch3.set_dst_reg(0x0600_0000);
        ch3.set_count_reg(0);
        ch3.set_control(1 << 15, 3); // channel 3
        assert_eq!(ch3.remaining(), 0x10000);
    }

    #[test]
    fn test_dma_disable_on_complete() {
        let mut ch = DmaChannel::new();
        ch.set_src_reg(0x0200_0000);
        ch.set_dst_reg(0x0600_0000);
        ch.set_count_reg(1);
        ch.set_control(1 << 15, 0); // immediate, no repeat
        assert!(ch.enabled());

        ch.step();
        assert!(!ch.active());
        assert!(!ch.enabled()); // disabled after non-repeat completion
    }

    #[test]
    fn test_dma_immediate_trigger() {
        let mut dma = GbaDma::new();
        dma.channels[0].set_src_reg(0x0800_0000);
        dma.channels[0].set_dst_reg(0x0600_0000);
        dma.channels[0].set_count_reg(16);

        // enable with immediate timing
        dma.channels[0].set_control(1 << 15, 0);
        assert!(dma.channels[0].active());
        assert_eq!(dma.highest_active(), Some(0));
    }

    #[test]
    fn test_dma_vblank_trigger() {
        let mut dma = GbaDma::new();
        dma.channels[1].set_src_reg(0x0200_0000);
        dma.channels[1].set_dst_reg(0x0600_0000);
        dma.channels[1].set_count_reg(32);

        // enable with vblank timing
        dma.channels[1].set_control((1 << 15) | (1 << 12), 1);
        assert!(!dma.channels[1].active()); // not active until vblank

        dma.trigger_vblank();
        assert!(dma.channels[1].active());
    }

    #[test]
    fn test_dma_hblank_trigger() {
        let mut dma = GbaDma::new();
        dma.channels[2].set_src_reg(0x0200_0000);
        dma.channels[2].set_dst_reg(0x0600_0000);
        dma.channels[2].set_count_reg(16);

        // enable with hblank timing (timing = 2)
        dma.channels[2].set_control((1 << 15) | (2 << 12), 2);
        assert!(!dma.channels[2].active());

        dma.trigger_hblank();
        assert!(dma.channels[2].active());
    }

    #[test]
    fn test_dma_sound_fifo_trigger() {
        let mut dma = GbaDma::new();
        // channel 1 for FIFO A (fifo_index=0 -> channel 1)
        dma.channels[1].set_src_reg(0x0200_0000);
        dma.channels[1].set_dst_reg(REG_FIFO_A);
        dma.channels[1].set_count_reg(4);

        // enable with special timing (timing = 3)
        dma.channels[1].set_control((1 << 15) | (3 << 12), 1);
        assert!(!dma.channels[1].active());

        dma.trigger_sound_fifo(0);
        assert!(dma.channels[1].active());
    }

    #[test]
    fn test_dma_sound_fifo_trigger_fifo_b() {
        let mut dma = GbaDma::new();
        // channel 2 for FIFO B
        dma.channels[2].set_src_reg(0x0200_1000);
        dma.channels[2].set_dst_reg(REG_FIFO_B);
        dma.channels[2].set_count_reg(4);

        // enable with special timing
        dma.channels[2].set_control((1 << 15) | (3 << 12), 2);
        assert!(!dma.channels[2].active());

        dma.trigger_sound_fifo(1);
        assert!(dma.channels[2].active());
    }

    #[test]
    fn test_dma_sound_fifo_only_triggers_matching_dst() {
        // regression: trigger_sound_fifo must only activate channels whose
        // dst_reg matches the requested FIFO address
        let mut dma = GbaDma::new();

        // DMA1 → FIFO A
        dma.channels[1].set_src_reg(0x0300_5000);
        dma.channels[1].set_dst_reg(REG_FIFO_A);
        dma.channels[1].set_count_reg(4);
        dma.channels[1].set_control((1 << 15) | (3 << 12) | (1 << 9) | (1 << 10), 1);

        // DMA2 → FIFO B
        dma.channels[2].set_src_reg(0x0300_6000);
        dma.channels[2].set_dst_reg(REG_FIFO_B);
        dma.channels[2].set_count_reg(4);
        dma.channels[2].set_control((1 << 15) | (3 << 12) | (1 << 9) | (1 << 10), 2);

        // triggering FIFO A should only activate DMA1
        dma.trigger_sound_fifo(0);
        assert!(
            dma.channels[1].active(),
            "DMA1 should be active (dst = FIFO A)"
        );
        assert!(
            !dma.channels[2].active(),
            "DMA2 should NOT be active (dst = FIFO B)"
        );
    }

    #[test]
    fn test_dma_sound_fifo_b_does_not_trigger_fifo_a_channel() {
        // regression: triggering FIFO B must not activate a channel assigned to FIFO A
        let mut dma = GbaDma::new();

        // DMA1 → FIFO A
        dma.channels[1].set_src_reg(0x0300_5000);
        dma.channels[1].set_dst_reg(REG_FIFO_A);
        dma.channels[1].set_count_reg(4);
        dma.channels[1].set_control((1 << 15) | (3 << 12) | (1 << 9) | (1 << 10), 1);

        // DMA2 → FIFO B
        dma.channels[2].set_src_reg(0x0300_6000);
        dma.channels[2].set_dst_reg(REG_FIFO_B);
        dma.channels[2].set_count_reg(4);
        dma.channels[2].set_control((1 << 15) | (3 << 12) | (1 << 9) | (1 << 10), 2);

        // triggering FIFO B should only activate DMA2
        dma.trigger_sound_fifo(1);
        assert!(
            !dma.channels[1].active(),
            "DMA1 should NOT be active (dst = FIFO A)"
        );
        assert!(
            dma.channels[2].active(),
            "DMA2 should be active (dst = FIFO B)"
        );
    }

    #[test]
    fn test_dma_sound_fifo_both_fifos_independent() {
        // both FIFOs can be triggered independently without cross-contamination
        let mut dma = GbaDma::new();

        // DMA1 → FIFO A, DMA2 → FIFO B
        dma.channels[1].set_src_reg(0x0300_5000);
        dma.channels[1].set_dst_reg(REG_FIFO_A);
        dma.channels[1].set_count_reg(4);
        dma.channels[1].set_control((1 << 15) | (3 << 12) | (1 << 9) | (1 << 10), 1);

        dma.channels[2].set_src_reg(0x0300_6000);
        dma.channels[2].set_dst_reg(REG_FIFO_B);
        dma.channels[2].set_count_reg(4);
        dma.channels[2].set_control((1 << 15) | (3 << 12) | (1 << 9) | (1 << 10), 2);

        // trigger FIFO A, run DMA1 to completion
        dma.trigger_sound_fifo(0);
        assert!(dma.channels[1].active());
        assert!(!dma.channels[2].active());
        for _ in 0..4 {
            dma.channels[1].step();
        }
        assert!(!dma.channels[1].active());

        // trigger FIFO B, only DMA2 activates
        dma.trigger_sound_fifo(1);
        assert!(
            !dma.channels[1].active(),
            "DMA1 should stay inactive after FIFO B trigger"
        );
        assert!(
            dma.channels[2].active(),
            "DMA2 should be active after FIFO B trigger"
        );
    }

    #[test]
    fn test_dma_sound_fifo_forces_count_4() {
        // sound DMA always forces count to 4 regardless of count_reg
        let mut dma = GbaDma::new();
        dma.channels[1].set_src_reg(0x0200_0000);
        dma.channels[1].set_dst_reg(REG_FIFO_A);
        dma.channels[1].set_count_reg(16); // game sets 16, but hardware forces 4
        dma.channels[1].set_control((1 << 15) | (3 << 12) | (1 << 9) | (1 << 10), 1);

        dma.trigger_sound_fifo(0);
        assert_eq!(dma.channels[1].remaining(), 4);
    }

    #[test]
    fn test_dma_sound_fifo_src_advances_correctly() {
        // verify DMA source advances by 4 per step (32-bit words)
        let mut dma = GbaDma::new();
        dma.channels[1].set_src_reg(0x0300_5FF0);
        dma.channels[1].set_dst_reg(REG_FIFO_A);
        dma.channels[1].set_count_reg(4);
        dma.channels[1].set_control((1 << 15) | (3 << 12) | (1 << 9) | (1 << 10), 1);

        dma.trigger_sound_fifo(0);

        // 4 steps, each advancing src by 4 (32-bit)
        for i in 0u32..4 {
            let (src, dst, _) = dma.channels[1].step();
            assert_eq!(src, 0x0300_5FF0 + i * 4);
            assert_eq!(dst, REG_FIFO_A); // dst stays fixed for sound DMA
        }
        // after 4 steps, src should have advanced by 16 bytes total
        assert_eq!(dma.channels[1].src(), 0x0300_5FF0 + 16);
    }

    #[test]
    fn test_dma_sound_fifo_dst_fixed_for_special_timing() {
        // sound DMA (SPECIAL timing) keeps dst fixed regardless of dst_control
        let mut dma = GbaDma::new();
        dma.channels[1].set_src_reg(0x0200_0000);
        dma.channels[1].set_dst_reg(REG_FIFO_A);
        dma.channels[1].set_count_reg(4);
        // dst_control = Increment (0), but SPECIAL timing overrides to Fixed
        dma.channels[1].set_control((1 << 15) | (3 << 12) | (1 << 9) | (1 << 10), 1);

        dma.trigger_sound_fifo(0);
        for _ in 0..4 {
            let (_, dst, _) = dma.channels[1].step();
            assert_eq!(dst, REG_FIFO_A, "dst should remain fixed at FIFO address");
        }
    }

    #[test]
    fn test_dma_sound_fifo_repeat_reloads_count() {
        // after completion with repeat, count is re-latched for the next trigger
        let mut dma = GbaDma::new();
        dma.channels[1].set_src_reg(0x0200_0000);
        dma.channels[1].set_dst_reg(REG_FIFO_A);
        dma.channels[1].set_count_reg(4);
        dma.channels[1].set_control((1 << 15) | (3 << 12) | (1 << 9) | (1 << 10), 1);

        // first trigger + complete
        dma.trigger_sound_fifo(0);
        for _ in 0..4 {
            dma.channels[1].step();
        }
        assert!(!dma.channels[1].active());
        assert!(
            dma.channels[1].enabled(),
            "channel stays enabled with repeat"
        );

        // second trigger should work (count re-latched)
        dma.trigger_sound_fifo(0);
        assert!(dma.channels[1].active());
        assert_eq!(dma.channels[1].remaining(), 4);
    }

    #[test]
    fn test_dma_sound_fifo_src_continues_across_triggers() {
        // source address is NOT re-latched on repeat — it continues advancing
        let mut dma = GbaDma::new();
        dma.channels[1].set_src_reg(0x0300_5000);
        dma.channels[1].set_dst_reg(REG_FIFO_A);
        dma.channels[1].set_count_reg(4);
        dma.channels[1].set_control((1 << 15) | (3 << 12) | (1 << 9) | (1 << 10), 1);

        // first trigger: src starts at 0x0300_5000
        dma.trigger_sound_fifo(0);
        for _ in 0..4 {
            dma.channels[1].step();
        }
        let src_after_first = dma.channels[1].src();
        assert_eq!(src_after_first, 0x0300_5000 + 16);

        // second trigger: src continues from where it left off
        dma.trigger_sound_fifo(0);
        let (src, _, _) = dma.channels[1].step();
        assert_eq!(
            src, src_after_first,
            "src should continue from previous position"
        );
    }

    #[test]
    fn test_dma_sound_fifo_ignores_non_special_timing() {
        // channels with non-SPECIAL timing are not triggered by sound FIFO
        let mut dma = GbaDma::new();
        dma.channels[1].set_src_reg(0x0200_0000);
        dma.channels[1].set_dst_reg(REG_FIFO_A);
        dma.channels[1].set_count_reg(4);
        // timing = VBLANK (1), not SPECIAL (3)
        dma.channels[1].set_control((1 << 15) | (1 << 12), 1);

        dma.trigger_sound_fifo(0);
        assert!(!dma.channels[1].active());
    }

    #[test]
    fn test_dma_sound_fifo_ignores_disabled_channel() {
        // disabled channels are not triggered even with correct dst and timing
        let mut dma = GbaDma::new();
        dma.channels[1].set_src_reg(0x0200_0000);
        dma.channels[1].set_dst_reg(REG_FIFO_A);
        dma.channels[1].set_count_reg(4);
        // NOT enabled (bit 15 = 0)
        dma.channels[1].set_control(3 << 12, 1);

        dma.trigger_sound_fifo(0);
        assert!(!dma.channels[1].active());
    }

    #[test]
    fn test_dma_sound_fifo_does_not_override_dst() {
        // trigger_sound_fifo must NOT override the channel's latched dst
        let mut dma = GbaDma::new();
        dma.channels[1].set_src_reg(0x0200_0000);
        dma.channels[1].set_dst_reg(REG_FIFO_A);
        dma.channels[1].set_count_reg(4);
        dma.channels[1].set_control((1 << 15) | (3 << 12) | (1 << 9) | (1 << 10), 1);

        let dst_before = dma.channels[1].dst();
        dma.trigger_sound_fifo(0);
        assert_eq!(
            dma.channels[1].dst(),
            dst_before,
            "dst should not be overridden"
        );
    }

    #[test]
    fn test_dma_highest_active_priority() {
        let mut dma = GbaDma::new();
        // activate channels 1 and 2
        for i in 1..=2 {
            dma.channels[i].set_src_reg(0x0200_0000);
            dma.channels[i].set_dst_reg(0x0600_0000);
            dma.channels[i].set_count_reg(1);
            dma.channels[i].set_control(1 << 15, i);
        }
        // channel 1 has higher priority than channel 2
        assert_eq!(dma.highest_active(), Some(1));
    }

    #[test]
    fn test_dma_highest_active_none() {
        let dma = GbaDma::new();
        assert_eq!(dma.highest_active(), None);
    }
}
