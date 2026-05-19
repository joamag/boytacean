//! GBA timer hardware (TM0-TM3) with cascade and prescaler support.

use crate::gba::consts::TIMER_PRESCALERS;

pub struct GbaTimer {
    /// current counter value
    counter: u16,

    /// reload value (written to TM*CNT_L)
    reload: u16,

    /// control register (TM*CNT_H)
    control: u16,

    /// internal prescaler cycle counter
    prescaler_counter: u32,

    /// derived: timer enabled
    enabled: bool,

    /// derived: interrupt on overflow
    irq_enable: bool,

    /// derived: cascade mode (count on previous timer overflow)
    cascade: bool,

    /// derived: prescaler divisor (1, 64, 256, 1024)
    prescaler: u32,

    /// overflow flag (set when timer overflows, consumed externally)
    overflow: bool,
}

impl GbaTimer {
    pub fn new() -> Self {
        Self {
            counter: 0,
            reload: 0,
            control: 0,
            prescaler_counter: 0,
            enabled: false,
            irq_enable: false,
            cascade: false,
            prescaler: 1,
            overflow: false,
        }
    }

    pub fn counter(&self) -> u16 {
        self.counter
    }

    pub fn reload(&self) -> u16 {
        self.reload
    }

    pub fn set_reload(&mut self, value: u16) {
        self.reload = value;
    }

    pub fn control(&self) -> u16 {
        self.control
    }

    pub fn set_control(&mut self, value: u16) {
        let was_enabled = self.enabled;
        self.control = value;
        self.enabled = value & (1 << 7) != 0;
        self.irq_enable = value & (1 << 6) != 0;
        self.cascade = value & (1 << 2) != 0;
        self.prescaler = TIMER_PRESCALERS[(value & 0x03) as usize];

        // reload counter when transitioning from disabled to enabled
        if !was_enabled && self.enabled {
            self.counter = self.reload;
            self.prescaler_counter = 0;
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn cascade(&self) -> bool {
        self.cascade
    }

    pub fn irq_enable(&self) -> bool {
        self.irq_enable
    }

    pub fn overflow(&self) -> bool {
        self.overflow
    }

    pub fn clear_overflow(&mut self) {
        self.overflow = false;
    }

    /// Clocks this timer by the given number of CPU cycles.
    ///
    /// Returns if an overflow occurred.
    pub fn clock(&mut self, cycles: u32) -> bool {
        if !self.enabled || self.cascade {
            return false;
        }

        self.overflow = false;
        self.prescaler_counter += cycles;

        while self.prescaler_counter >= self.prescaler {
            self.prescaler_counter -= self.prescaler;
            let (new_counter, overflow) = self.counter.overflowing_add(1);
            if overflow {
                self.counter = self.reload;
                self.overflow = true;
            } else {
                self.counter = new_counter;
            }
        }

        self.overflow
    }

    /// handles a cascade tick from the previous timer's overflow.
    /// returns true if this timer also overflows
    pub fn cascade_tick(&mut self) -> bool {
        if !self.enabled || !self.cascade {
            return false;
        }

        self.overflow = false;
        let (new_counter, overflow) = self.counter.overflowing_add(1);
        if overflow {
            self.counter = self.reload;
            self.overflow = true;
        } else {
            self.counter = new_counter;
        }

        self.overflow
    }
}

impl Default for GbaTimer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GbaTimers {
    pub timers: [GbaTimer; 4],
}

impl GbaTimers {
    pub fn new() -> Self {
        Self {
            timers: [
                GbaTimer::new(),
                GbaTimer::new(),
                GbaTimer::new(),
                GbaTimer::new(),
            ],
        }
    }

    /// clocks all 4 timers, handling cascade chains.
    /// returns a bitmask of which timers overflowed (bit 0 = TM0, etc)
    pub fn clock(&mut self, cycles: u32) -> u8 {
        let mut overflows = 0u8;

        // clock timer 0 (never cascade)
        if self.timers[0].clock(cycles) {
            overflows |= 1 << 0;
        }

        // clock timers 1-3 with cascade support
        for i in 1..4 {
            let prev_overflow = overflows & (1 << (i - 1)) != 0;
            if self.timers[i].cascade() {
                if prev_overflow && self.timers[i].cascade_tick() {
                    overflows |= 1 << i;
                }
            } else if self.timers[i].clock(cycles) {
                overflows |= 1 << i;
            }
        }

        overflows
    }
}

impl Default for GbaTimers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{GbaTimer, GbaTimers};

    #[test]
    fn test_timer_new() {
        let timer = GbaTimer::new();
        assert_eq!(timer.counter(), 0);
        assert_eq!(timer.reload(), 0);
        assert_eq!(timer.control(), 0);
        assert!(!timer.enabled());
        assert!(!timer.cascade());
        assert!(!timer.irq_enable());
        assert!(!timer.overflow());
    }

    #[test]
    fn test_timer_reload() {
        let mut timer = GbaTimer::new();
        timer.set_reload(0x8000);
        assert_eq!(timer.reload(), 0x8000);
    }

    #[test]
    fn test_timer_control() {
        let mut timer = GbaTimer::new();
        // enable, IRQ enable, cascade, prescaler 64
        timer.set_control((1 << 7) | (1 << 6) | (1 << 2) | 0x01);
        assert!(timer.enabled());
        assert!(timer.irq_enable());
        assert!(timer.cascade());
    }

    #[test]
    fn test_timer_enable_reloads_counter() {
        let mut timer = GbaTimer::new();
        timer.set_reload(0x1234);
        timer.set_control(1 << 7); // enable
        assert_eq!(timer.counter(), 0x1234);
    }

    #[test]
    fn test_timer_clock_no_overflow() {
        let mut timer = GbaTimer::new();
        timer.set_reload(0);
        timer.set_control(1 << 7); // enable, prescaler 1
        assert!(!timer.clock(1)); // 0 -> 1, no overflow
        assert_eq!(timer.counter(), 1);
    }

    #[test]
    fn test_timer_clock_disabled() {
        let mut timer = GbaTimer::new();
        // not enabled
        assert!(!timer.clock(10));
    }

    #[test]
    fn test_timer_clock_cascade_skips() {
        let mut timer = GbaTimer::new();
        timer.set_reload(0xFFFF);
        timer.set_control((1 << 7) | (1 << 2)); // enable + cascade
                                                // cascade timers are not clocked by CPU cycles
        assert!(!timer.clock(1));
    }

    #[test]
    fn test_timer_cascade_tick() {
        let mut timer = GbaTimer::new();
        timer.set_reload(0xFFFE);
        timer.set_control((1 << 7) | (1 << 2)); // enable + cascade
        assert!(!timer.cascade_tick()); // 0xFFFE -> 0xFFFF
        assert!(timer.cascade_tick()); // 0xFFFF -> overflow
        assert_eq!(timer.counter(), 0xFFFE); // reloaded
    }

    #[test]
    fn test_timer_cascade_tick_disabled() {
        let mut timer = GbaTimer::new();
        // not enabled, not cascade
        assert!(!timer.cascade_tick());
    }

    #[test]
    fn test_timer_overflow_flag() {
        let mut timer = GbaTimer::new();
        timer.set_reload(0xFFFF);
        timer.set_control(1 << 7);
        timer.clock(1);
        assert!(timer.overflow());
        timer.clear_overflow();
        assert!(!timer.overflow());
    }

    #[test]
    fn test_timer_overflow() {
        let mut timers = GbaTimers::new();
        timers.timers[0].set_reload(0xFFFF);

        // enable timer 0 with prescaler 1
        timers.timers[0].set_control(1 << 7);

        // should overflow after 1 tick from 0xFFFF
        let overflows = timers.clock(1);
        assert_eq!(overflows & 1, 1);
    }

    #[test]
    fn test_timer_cascade() {
        let mut timers = GbaTimers::new();

        // timer 0: reload 0xFFFF, prescaler 1
        timers.timers[0].set_reload(0xFFFF);
        timers.timers[0].set_control(1 << 7);

        // timer 1: reload 0xFFFE, cascade mode
        timers.timers[1].set_reload(0xFFFE);
        timers.timers[1].set_control((1 << 7) | (1 << 2));

        // timer 0 overflows, timer 1 increments from 0xFFFE to 0xFFFF
        let overflows = timers.clock(1);
        assert_eq!(overflows & 1, 1); // timer 0 overflows
        assert_eq!(overflows & 2, 0); // timer 1 does not overflow yet
    }

    #[test]
    fn test_timer_cascade_chain_overflow() {
        let mut timers = GbaTimers::new();

        // timer 0: reload 0xFFFF
        timers.timers[0].set_reload(0xFFFF);
        timers.timers[0].set_control(1 << 7);

        // timer 1: reload 0xFFFF, cascade
        timers.timers[1].set_reload(0xFFFF);
        timers.timers[1].set_control((1 << 7) | (1 << 2));

        // both should overflow
        let overflows = timers.clock(1);
        assert_eq!(overflows & 1, 1);
        assert_eq!(overflows & 2, 2);
    }

    #[test]
    fn test_timers_new() {
        let timers = GbaTimers::new();
        for timer in &timers.timers {
            assert!(!timer.enabled());
        }
    }

    #[test]
    fn test_timers_independent() {
        let mut timers = GbaTimers::new();

        // only enable timer 2
        timers.timers[2].set_reload(0xFFFF);
        timers.timers[2].set_control(1 << 7);

        let overflows = timers.clock(1);
        assert_eq!(overflows & 1, 0); // timer 0 not enabled
        assert_eq!(overflows & 4, 4); // timer 2 overflowed
    }
}
