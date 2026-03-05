//! GBA keypad input handling (KEYINPUT / KEYCNT registers).

use crate::pad::PadKey;

/// GBA keypad state. The KEYINPUT register is active-low
/// (0 = pressed, 1 = released) so the default value is 0x03FF
/// (all 10 buttons released).
pub struct GbaPad {
    /// KEYINPUT register value (active-low, 10 bits)
    /// bit 0: A, bit 1: B, bit 2: Select, bit 3: Start,
    /// bit 4: Right, bit 5: Left, bit 6: Up, bit 7: Down,
    /// bit 8: R, bit 9: L
    keyinput: u16,

    /// KEYCNT register (interrupt control)
    keycnt: u16,

    /// interrupt pending flag
    int_keypad: bool,
}

impl GbaPad {
    pub fn new() -> Self {
        Self {
            keyinput: 0x03FF,
            keycnt: 0,
            int_keypad: false,
        }
    }

    pub fn keyinput(&self) -> u16 {
        self.keyinput
    }

    pub fn keycnt(&self) -> u16 {
        self.keycnt
    }

    pub fn set_keycnt(&mut self, value: u16) {
        self.keycnt = value;
    }

    pub fn int_keypad(&self) -> bool {
        self.int_keypad
    }

    pub fn ack_keypad(&mut self) {
        self.int_keypad = false;
    }

    pub fn key_press(&mut self, key: PadKey) {
        let bit = Self::key_bit(key);
        self.keyinput &= !bit;
        self.check_interrupt();
    }

    pub fn key_lift(&mut self, key: PadKey) {
        let bit = Self::key_bit(key);
        self.keyinput |= bit;
    }

    fn key_bit(key: PadKey) -> u16 {
        match key {
            PadKey::A => 1 << 0,
            PadKey::B => 1 << 1,
            PadKey::Select => 1 << 2,
            PadKey::Start => 1 << 3,
            PadKey::Right => 1 << 4,
            PadKey::Left => 1 << 5,
            PadKey::Up => 1 << 6,
            PadKey::Down => 1 << 7,
            PadKey::R => 1 << 8,
            PadKey::L => 1 << 9,
        }
    }

    /// checks if the keypad interrupt condition is met
    /// based on KEYCNT settings
    fn check_interrupt(&mut self) {
        let irq_enable = self.keycnt & (1 << 14) != 0;
        if !irq_enable {
            return;
        }

        let key_mask = self.keycnt & 0x03FF;
        let pressed = !self.keyinput & 0x03FF;
        let irq_and = self.keycnt & (1 << 15) != 0;

        let triggered = if irq_and {
            // logical AND mode: all specified keys must be pressed
            (pressed & key_mask) == key_mask
        } else {
            // logical OR mode: any specified key pressed
            (pressed & key_mask) != 0
        };

        if triggered {
            self.int_keypad = true;
        }
    }
}

impl Default for GbaPad {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::GbaPad;
    use crate::pad::PadKey;

    #[test]
    fn test_new() {
        let pad = GbaPad::new();
        assert_eq!(pad.keyinput(), 0x03FF);
        assert_eq!(pad.keycnt(), 0);
        assert!(!pad.int_keypad());
    }

    #[test]
    fn test_key_press_and_lift() {
        let mut pad = GbaPad::new();
        assert_eq!(pad.keyinput(), 0x03FF);

        pad.key_press(PadKey::A);
        assert_eq!(pad.keyinput() & 1, 0); // bit 0 cleared

        pad.key_lift(PadKey::A);
        assert_eq!(pad.keyinput() & 1, 1); // bit 0 set
    }

    #[test]
    fn test_shoulder_buttons() {
        let mut pad = GbaPad::new();

        pad.key_press(PadKey::L);
        assert_eq!(pad.keyinput() & (1 << 9), 0);

        pad.key_press(PadKey::R);
        assert_eq!(pad.keyinput() & (1 << 8), 0);
    }

    #[test]
    fn test_all_buttons() {
        let mut pad = GbaPad::new();
        let keys = [
            PadKey::A,
            PadKey::B,
            PadKey::Select,
            PadKey::Start,
            PadKey::Right,
            PadKey::Left,
            PadKey::Up,
            PadKey::Down,
            PadKey::R,
            PadKey::L,
        ];

        for key in &keys {
            pad.key_press(*key);
        }
        assert_eq!(pad.keyinput(), 0x0000);

        for key in &keys {
            pad.key_lift(*key);
        }
        assert_eq!(pad.keyinput(), 0x03FF);
    }

    #[test]
    fn test_keycnt() {
        let mut pad = GbaPad::new();
        pad.set_keycnt(0x1234);
        assert_eq!(pad.keycnt(), 0x1234);
    }

    #[test]
    fn test_interrupt_or_mode() {
        let mut pad = GbaPad::new();
        // enable IRQ, OR mode, mask A button (bit 0)
        pad.set_keycnt((1 << 14) | 0x0001);
        assert!(!pad.int_keypad());

        pad.key_press(PadKey::A);
        assert!(pad.int_keypad());
    }

    #[test]
    fn test_interrupt_and_mode() {
        let mut pad = GbaPad::new();
        // enable IRQ, AND mode, mask A+B (bits 0,1)
        pad.set_keycnt((1 << 14) | (1 << 15) | 0x0003);

        pad.key_press(PadKey::A);
        assert!(!pad.int_keypad()); // only A pressed, need A+B

        pad.key_press(PadKey::B);
        assert!(pad.int_keypad()); // both pressed
    }

    #[test]
    fn test_ack_keypad() {
        let mut pad = GbaPad::new();
        pad.set_keycnt((1 << 14) | 0x0001);
        pad.key_press(PadKey::A);
        assert!(pad.int_keypad());

        pad.ack_keypad();
        assert!(!pad.int_keypad());
    }

    #[test]
    fn test_no_interrupt_without_enable() {
        let mut pad = GbaPad::new();
        // mask A but IRQ not enabled (bit 14 = 0)
        pad.set_keycnt(0x0001);
        pad.key_press(PadKey::A);
        assert!(!pad.int_keypad());
    }

    #[test]
    fn test_default() {
        let pad = GbaPad::default();
        assert_eq!(pad.keyinput(), 0x03FF);
    }
}
