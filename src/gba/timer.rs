//! GBA timer hardware (TM0-TM3) with cascade and prescaler support.

use crate::gba::consts::TIMER_PRESCALERS;

pub struct GbaTimer {
    /// Current counter value.
    counter: u16,

    /// Reload value (written to TM*CNT_L).
    reload: u16,

    /// Control register (TM*CNT_H).
    control: u16,

    /// Internal prescaler cycle counter.
    prescaler_counter: u32,

    /// Derived: timer enabled.
    enabled: bool,

    /// Derived: interrupt on overflow.
    irq_enable: bool,

    /// Derived: cascade mode (count on previous timer overflow).
    cascade: bool,

    /// Derived: prescaler divisor (1, 64, 256, 1024).
    prescaler: u32,

    /// Derived: prescaler divisor as a shift amount (0, 6, 8, 10).
    prescaler_shift: u32,

    /// Overflow flag (set when timer overflows, consumed externally).
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
            prescaler_shift: 0,
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
        self.prescaler_shift = self.prescaler.trailing_zeros();

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

        // consumes whole prescaler periods in one step, wrapping the
        // counter through the reload value on overflow
        let ticks = self.prescaler_counter >> self.prescaler_shift;
        if ticks > 0 {
            self.prescaler_counter &= self.prescaler - 1;
            let total = self.counter as u32 + ticks;
            if total >= 0x10000 {
                let period = 0x10000 - self.reload as u32;
                self.counter = (self.reload as u32 + (total - 0x10000) % period) as u16;
                self.overflow = true;
            } else {
                self.counter = total as u16;
            }
        }

        self.overflow
    }

    /// Returns the number of cycles until this timer overflows, or
    /// `u32::MAX` when the timer is disabled or in cascade mode.
    pub fn cycles_to_overflow(&self) -> u32 {
        if !self.enabled || self.cascade {
            return u32::MAX;
        }
        let ticks = 0x10000 - self.counter as u32;
        ((ticks << self.prescaler_shift) - self.prescaler_counter).max(1)
    }

    /// Handles a cascade tick from the previous timer's overflow.
    ///
    /// Returns true if this timer also overflows.
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

    /// Cycles accumulated since the last batch run.
    pending_cycles: u32,

    /// Cycles from the last batch run until the earliest overflow,
    /// `u32::MAX` when no timer can overflow on its own.
    next_overflow: u32,
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
            pending_cycles: 0,
            next_overflow: u32::MAX,
        }
    }

    /// Writes a timer reload value, flushing pending cycles so the
    /// change applies from the current CPU clock position.
    pub fn write_reload(&mut self, index: usize, value: u16) {
        self.flush();
        self.timers[index].set_reload(value);
        self.next_overflow = self.compute_next_overflow();
    }

    /// Writes a timer control register, flushing pending cycles so the
    /// change applies from the current CPU clock position.
    pub fn write_control(&mut self, index: usize, value: u16) {
        self.flush();
        self.timers[index].set_control(value);
        self.next_overflow = self.compute_next_overflow();
    }

    /// Reads a timer counter, flushing pending cycles so the value
    /// reflects the current CPU clock position.
    pub fn read_counter(&mut self, index: usize) -> u16 {
        self.flush();
        self.timers[index].counter()
    }

    /// Returns the number of cycles until the earliest enabled timer
    /// overflows, or `u32::MAX` when no timer can overflow on its own.
    ///
    /// Accounts for cycles accumulated since the last batch run, so
    /// the boundary is exact from the current CPU clock position.
    pub fn cycles_to_next_overflow(&self) -> u32 {
        if self.next_overflow == u32::MAX {
            return u32::MAX;
        }
        (self.next_overflow - self.pending_cycles).max(1)
    }

    /// Computes the batch boundary from the current timer state.
    ///
    /// Cascade timers are excluded, they only tick when their driving
    /// timer overflows, which is already an event boundary.
    fn compute_next_overflow(&self) -> u32 {
        let mut next = u32::MAX;
        for timer in &self.timers {
            next = next.min(timer.cycles_to_overflow());
        }
        next
    }

    /// Clocks all 4 timers, handling cascade chains.
    ///
    /// Cycles are accumulated and processed in batches bounded by the
    /// earliest overflow, keeping the per instruction cost low while
    /// overflow timing stays exact; [`Self::flush`] runs any pending
    /// cycles before timer state is observed.
    ///
    /// Returns a bitmask of which timers overflowed (bit 0 = TM0, etc).
    #[inline(always)]
    pub fn clock(&mut self, cycles: u32) -> u8 {
        self.pending_cycles = self.pending_cycles.saturating_add(cycles);
        if self.pending_cycles < self.next_overflow {
            return 0;
        }
        self.run_pending()
    }

    /// Processes any accumulated cycles, bringing the timer counters
    /// up to date with the CPU clock.
    ///
    /// Flushing never crosses the overflow boundary (the batched clock
    /// runs as soon as it is reached), so no overflow events are lost.
    pub fn flush(&mut self) {
        if self.pending_cycles > 0 {
            self.run_pending();
        }
    }

    /// Runs the accumulated pending cycles through the timers and
    /// recomputes the next overflow boundary.
    fn run_pending(&mut self) -> u8 {
        let cycles = self.pending_cycles;
        self.pending_cycles = 0;

        // fast path: no enabled timer means nothing can tick or overflow
        if !self.timers[0].enabled()
            && !self.timers[1].enabled()
            && !self.timers[2].enabled()
            && !self.timers[3].enabled()
        {
            return 0;
        }

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

        self.next_overflow = self.compute_next_overflow();

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
    fn test_timer_cycles_to_overflow() {
        let mut timer = GbaTimer::new();
        // disabled timers can never overflow on their own
        assert_eq!(timer.cycles_to_overflow(), u32::MAX);

        // enabled with prescaler 1, one tick away from overflow
        timer.set_reload(0xFFFF);
        timer.set_control(1 << 7);
        assert_eq!(timer.cycles_to_overflow(), 1);

        // prescaler 64 multiplies the tick distance
        timer.set_control(0);
        timer.set_control((1 << 7) | 1);
        assert_eq!(timer.cycles_to_overflow(), 64);

        // partially elapsed prescaler cycles shorten the distance
        timer.clock(10);
        assert_eq!(timer.cycles_to_overflow(), 54);

        // cascade timers are not clocked by CPU cycles
        timer.set_control((1 << 7) | (1 << 2));
        assert_eq!(timer.cycles_to_overflow(), u32::MAX);
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
    fn test_timer_clock_batched_ticks() {
        let mut timer = GbaTimer::new();
        timer.set_control((1 << 7) | 0x01); // enabled, prescaler 64

        // 129 cycles are two whole prescaler periods plus a remainder
        timer.clock(129);
        assert_eq!(timer.counter(), 2);

        // the remainder carries over into the next clock
        timer.clock(63);
        assert_eq!(timer.counter(), 3);
    }

    #[test]
    fn test_timer_clock_multi_overflow_wrap() {
        let mut timer = GbaTimer::new();
        timer.set_reload(0xFFFE);
        timer.set_control(1 << 7);

        // 5 ticks from 0xFFFE wrap through the reload value twice
        assert!(timer.clock(5));
        assert_eq!(timer.counter(), 0xFFFF);
    }

    #[test]
    fn test_timers_cycles_to_next_overflow() {
        let mut timers = GbaTimers::new();
        // no enabled timer means no overflow can happen
        assert_eq!(timers.cycles_to_next_overflow(), u32::MAX);

        // the earliest overflowing timer bounds the distance
        timers.write_reload(0, 0xFF00);
        timers.write_control(0, 1 << 7);
        timers.write_reload(1, 0xFFFF);
        timers.write_control(1, 1 << 7);
        assert_eq!(timers.cycles_to_next_overflow(), 1);
    }

    #[test]
    fn test_timers_cycles_to_next_overflow_pending() {
        let mut timers = GbaTimers::new();
        timers.write_reload(0, 0xFF00);
        timers.write_control(0, 1 << 7);
        assert_eq!(timers.cycles_to_next_overflow(), 0x100);

        // accumulated cycles shorten the distance to the boundary
        timers.clock(0x40);
        assert_eq!(timers.cycles_to_next_overflow(), 0xC0);
    }

    #[test]
    fn test_timers_clock_batches_until_overflow() {
        let mut timers = GbaTimers::new();
        timers.write_reload(0, 0xFF00);
        timers.write_control(0, 1 << 7);

        // clocking below the overflow boundary reports nothing
        assert_eq!(timers.clock(0xFF), 0);

        // the boundary cycle itself delivers the overflow
        assert_eq!(timers.clock(1), 1);
    }

    #[test]
    fn test_timers_flush_applies_pending() {
        let mut timers = GbaTimers::new();
        timers.write_control(0, 1 << 7);
        timers.clock(100);

        timers.flush();
        assert_eq!(timers.timers[0].counter(), 100);
    }

    #[test]
    fn test_timers_write_control_applies_pending() {
        let mut timers = GbaTimers::new();
        timers.write_control(0, 1 << 7);
        timers.clock(50);

        // re-writing control flushes, the elapsed time is not lost
        timers.write_control(0, 1 << 7);
        assert_eq!(timers.timers[0].counter(), 50);
    }

    #[test]
    fn test_timers_read_counter_flushes() {
        let mut timers = GbaTimers::new();
        timers.write_control(0, 1 << 7);

        // pending cycles are only applied when the counter is observed
        timers.clock(100);
        assert_eq!(timers.read_counter(0), 100);
    }

    #[test]
    fn test_timers_clock_all_disabled() {
        let mut timers = GbaTimers::new();

        // with no timer enabled the clock is a no-op
        let overflows = timers.clock(0x10000);
        assert_eq!(overflows, 0);
        assert_eq!(timers.timers[0].counter(), 0);
        assert_eq!(timers.timers[3].counter(), 0);
    }

    #[test]
    fn test_timer_overflow() {
        let mut timers = GbaTimers::new();
        timers.write_reload(0, 0xFFFF);

        // enable timer 0 with prescaler 1
        timers.write_control(0, 1 << 7);

        // should overflow after 1 tick from 0xFFFF
        let overflows = timers.clock(1);
        assert_eq!(overflows & 1, 1);
    }

    #[test]
    fn test_timer_cascade() {
        let mut timers = GbaTimers::new();

        // timer 0: reload 0xFFFF, prescaler 1
        timers.write_reload(0, 0xFFFF);
        timers.write_control(0, 1 << 7);

        // timer 1: reload 0xFFFE, cascade mode
        timers.write_reload(1, 0xFFFE);
        timers.write_control(1, (1 << 7) | (1 << 2));

        // timer 0 overflows, timer 1 increments from 0xFFFE to 0xFFFF
        let overflows = timers.clock(1);
        assert_eq!(overflows & 1, 1); // timer 0 overflows
        assert_eq!(overflows & 2, 0); // timer 1 does not overflow yet
    }

    #[test]
    fn test_timer_cascade_chain_overflow() {
        let mut timers = GbaTimers::new();

        // timer 0: reload 0xFFFF
        timers.write_reload(0, 0xFFFF);
        timers.write_control(0, 1 << 7);

        // timer 1: reload 0xFFFF, cascade
        timers.write_reload(1, 0xFFFF);
        timers.write_control(1, (1 << 7) | (1 << 2));

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
        timers.write_reload(2, 0xFFFF);
        timers.write_control(2, 1 << 7);

        let overflows = timers.clock(1);
        assert_eq!(overflows & 1, 0); // timer 0 not enabled
        assert_eq!(overflows & 4, 4); // timer 2 overflowed
    }
}
