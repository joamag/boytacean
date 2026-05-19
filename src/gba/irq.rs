//! GBA interrupt controller (IE, IF, IME registers).

use crate::gba::consts::{
    IRQ_DMA0, IRQ_DMA1, IRQ_DMA2, IRQ_DMA3, IRQ_HBLANK, IRQ_KEYPAD, IRQ_TIMER0, IRQ_TIMER1,
    IRQ_TIMER2, IRQ_TIMER3, IRQ_VBLANK, IRQ_VCOUNT,
};

pub struct IrqController {
    /// Interrupt Master Enable (IME) - global interrupt toggle
    ime: bool,

    /// Interrupt Enable register (IE) - per-source enable bits
    ie: u16,

    /// Interrupt Request Flags register (IF) - pending interrupt bits
    if_: u16,
}

impl IrqController {
    pub fn new() -> Self {
        Self {
            ime: false,
            ie: 0,
            if_: 0,
        }
    }

    pub fn ime(&self) -> bool {
        self.ime
    }

    pub fn set_ime(&mut self, value: bool) {
        self.ime = value;
    }

    pub fn ie(&self) -> u16 {
        self.ie
    }

    pub fn set_ie(&mut self, value: u16) {
        self.ie = value;
    }

    pub fn if_(&self) -> u16 {
        self.if_
    }

    /// acknowledges interrupts by writing 1 bits to IF
    /// (writing 1 clears the corresponding bit)
    pub fn ack_if(&mut self, value: u16) {
        self.if_ &= !value;
    }

    /// raises an interrupt by setting the corresponding bit in IF
    pub fn raise(&mut self, irq: u16) {
        self.if_ |= irq;
    }

    /// returns true if there are any pending interrupts that
    /// are both enabled (IE) and requested (IF) with IME on
    pub fn pending(&self) -> bool {
        self.ime && (self.ie & self.if_) != 0
    }

    /// returns the highest priority pending interrupt bit,
    /// or 0 if no interrupts are pending
    pub fn highest_pending(&self) -> u16 {
        let pending = self.ie & self.if_;
        if pending == 0 {
            return 0;
        }
        // lowest bit set is highest priority
        pending & pending.wrapping_neg()
    }

    pub fn raise_vblank(&mut self) {
        self.raise(IRQ_VBLANK);
    }

    pub fn raise_hblank(&mut self) {
        self.raise(IRQ_HBLANK);
    }

    pub fn raise_vcount(&mut self) {
        self.raise(IRQ_VCOUNT);
    }

    pub fn raise_timer(&mut self, index: usize) {
        let irq = match index {
            0 => IRQ_TIMER0,
            1 => IRQ_TIMER1,
            2 => IRQ_TIMER2,
            3 => IRQ_TIMER3,
            _ => return,
        };
        self.raise(irq);
    }

    pub fn raise_dma(&mut self, index: usize) {
        let irq = match index {
            0 => IRQ_DMA0,
            1 => IRQ_DMA1,
            2 => IRQ_DMA2,
            3 => IRQ_DMA3,
            _ => return,
        };
        self.raise(irq);
    }

    pub fn raise_keypad(&mut self) {
        self.raise(IRQ_KEYPAD);
    }
}

impl Default for IrqController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::IrqController;
    use crate::gba::consts::{
        IRQ_DMA0, IRQ_DMA3, IRQ_HBLANK, IRQ_KEYPAD, IRQ_TIMER0, IRQ_TIMER3, IRQ_VBLANK, IRQ_VCOUNT,
    };

    #[test]
    fn test_new() {
        let irq = IrqController::new();
        assert!(!irq.ime());
        assert_eq!(irq.ie(), 0);
        assert_eq!(irq.if_(), 0);
        assert!(!irq.pending());
    }

    #[test]
    fn test_ime() {
        let mut irq = IrqController::new();
        assert!(!irq.ime());

        irq.set_ime(true);
        assert!(irq.ime());

        irq.set_ime(false);
        assert!(!irq.ime());
    }

    #[test]
    fn test_ie() {
        let mut irq = IrqController::new();
        irq.set_ie(IRQ_VBLANK | IRQ_HBLANK);
        assert_eq!(irq.ie(), IRQ_VBLANK | IRQ_HBLANK);
    }

    #[test]
    fn test_raise() {
        let mut irq = IrqController::new();
        irq.raise(IRQ_VBLANK);
        assert_eq!(irq.if_(), IRQ_VBLANK);

        irq.raise(IRQ_HBLANK);
        assert_eq!(irq.if_(), IRQ_VBLANK | IRQ_HBLANK);
    }

    #[test]
    fn test_irq_pending() {
        let mut irq = IrqController::new();
        assert!(!irq.pending());

        irq.raise_vblank();
        assert!(!irq.pending()); // IME off

        irq.set_ime(true);
        assert!(!irq.pending()); // IE doesn't enable vblank

        irq.set_ie(IRQ_VBLANK);
        assert!(irq.pending());
    }

    #[test]
    fn test_irq_acknowledge() {
        let mut irq = IrqController::new();
        irq.set_ime(true);
        irq.set_ie(IRQ_VBLANK | IRQ_HBLANK);
        irq.raise_vblank();
        irq.raise_hblank();
        assert_eq!(irq.highest_pending(), IRQ_VBLANK);

        irq.ack_if(IRQ_VBLANK);
        assert_eq!(irq.highest_pending(), IRQ_HBLANK);
    }

    #[test]
    fn test_highest_pending_none() {
        let irq = IrqController::new();
        assert_eq!(irq.highest_pending(), 0);
    }

    #[test]
    fn test_raise_hblank() {
        let mut irq = IrqController::new();
        irq.raise_hblank();
        assert_eq!(irq.if_(), IRQ_HBLANK);
    }

    #[test]
    fn test_raise_vcount() {
        let mut irq = IrqController::new();
        irq.raise_vcount();
        assert_eq!(irq.if_(), IRQ_VCOUNT);
    }

    #[test]
    fn test_raise_timer() {
        let mut irq = IrqController::new();
        irq.raise_timer(0);
        assert_eq!(irq.if_(), IRQ_TIMER0);

        irq.raise_timer(3);
        assert_eq!(irq.if_(), IRQ_TIMER0 | IRQ_TIMER3);

        // invalid index should be ignored
        irq.raise_timer(4);
        assert_eq!(irq.if_(), IRQ_TIMER0 | IRQ_TIMER3);
    }

    #[test]
    fn test_raise_dma() {
        let mut irq = IrqController::new();
        irq.raise_dma(0);
        assert_eq!(irq.if_(), IRQ_DMA0);

        irq.raise_dma(3);
        assert_eq!(irq.if_(), IRQ_DMA0 | IRQ_DMA3);

        // invalid index should be ignored
        irq.raise_dma(5);
        assert_eq!(irq.if_(), IRQ_DMA0 | IRQ_DMA3);
    }

    #[test]
    fn test_raise_keypad() {
        let mut irq = IrqController::new();
        irq.raise_keypad();
        assert_eq!(irq.if_(), IRQ_KEYPAD);
    }

    #[test]
    fn test_default() {
        let irq = IrqController::default();
        assert!(!irq.ime());
        assert_eq!(irq.ie(), 0);
        assert_eq!(irq.if_(), 0);
    }
}
