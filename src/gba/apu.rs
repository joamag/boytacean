//! GBA APU (Audio Processing Unit) with 4 legacy GB channels
//! and 2 DirectSound PCM channels (FIFO A and FIFO B).

use std::collections::VecDeque;

use crate::warnln;

/// sampling rate for audio output
const SAMPLING_RATE: u32 = 32768;

/// duty cycle waveforms for square wave channels
const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [1, 0, 0, 0, 0, 0, 0, 1], // 25%
    [1, 0, 0, 0, 0, 1, 1, 1], // 50%
    [0, 1, 1, 1, 1, 1, 1, 0], // 75%
];

/// DirectSound FIFO channel
pub struct DirectSoundChannel {
    /// FIFO buffer (up to 32 bytes / 8 words)
    fifo: VecDeque<i8>,

    /// current sample being output
    current_sample: i8,

    /// timer index that drives this channel (0 or 1)
    pub timer_id: usize,

    /// volume: true = full, false = 50%
    volume_full: bool,

    /// enable left output
    enable_left: bool,

    /// enable right output
    enable_right: bool,
}

impl DirectSoundChannel {
    pub fn new() -> Self {
        Self {
            fifo: VecDeque::with_capacity(32),
            current_sample: 0,
            timer_id: 0,
            volume_full: true,
            enable_left: false,
            enable_right: false,
        }
    }

    /// pushes a 32-bit word (4 samples) into the FIFO
    pub fn write_fifo(&mut self, value: u32) {
        for i in 0..4 {
            let sample = ((value >> (i * 8)) & 0xFF) as i8;
            if self.fifo.len() < 32 {
                self.fifo.push_back(sample);
            }
        }
    }

    /// pops the next sample from the FIFO (called on timer overflow)
    pub fn timer_tick(&mut self) {
        if let Some(sample) = self.fifo.pop_front() {
            self.current_sample = sample;
        }
    }

    /// returns true if the FIFO needs refilling (fewer than 16 bytes)
    pub fn needs_refill(&self) -> bool {
        self.fifo.len() <= 16
    }

    /// resets the FIFO
    pub fn reset_fifo(&mut self) {
        self.fifo.clear();
        self.current_sample = 0;
    }

    /// returns the current output sample scaled to i16 range
    pub fn output(&self) -> i16 {
        let sample = self.current_sample as i16 * 256;
        if self.volume_full {
            sample
        } else {
            sample / 2
        }
    }

    pub fn output_left(&self) -> i16 {
        if self.enable_left {
            self.output()
        } else {
            0
        }
    }

    pub fn output_right(&self) -> i16 {
        if self.enable_right {
            self.output()
        } else {
            0
        }
    }
}

impl Default for DirectSoundChannel {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GbaApu {
    /// legacy sound control registers
    soundcnt_l: u16,
    soundcnt_h: u16,
    soundcnt_x: u16,
    soundbias: u16,

    /// wave RAM (16 bytes, shared with legacy channel 3)
    wave_ram: [u8; 16],

    /// DirectSound channels A and B
    pub direct_sound: [DirectSoundChannel; 2],

    /// legacy channel registers (simplified for initial implementation)
    /// channel 1: square with sweep
    ch1_sweep: u8,
    ch1_duty_length: u8,
    ch1_envelope: u8,
    ch1_freq: u16,

    /// channel 2: square
    ch2_duty_length: u8,
    ch2_envelope: u8,
    ch2_freq: u16,

    /// channel 3: wave
    ch3_enable: u8,
    ch3_length: u8,
    ch3_volume: u8,
    ch3_freq: u16,

    /// channel 4: noise
    ch4_length_envelope: u8,
    ch4_envelope: u8,
    ch4_poly: u8,
    ch4_control: u8,

    /// internal sequencer counter
    sequencer_counter: u32,
    sequencer_step: u8,

    /// internal timer counters for legacy channels
    ch1_timer: i32,
    ch1_sequence: u8,
    ch1_volume: u8,
    ch1_enabled: bool,

    ch2_timer: i32,
    ch2_sequence: u8,
    ch2_volume: u8,
    ch2_enabled: bool,

    ch3_timer: i32,
    ch3_position: u8,
    ch3_enabled: bool,

    ch4_timer: i32,
    ch4_lfsr: u16,
    ch4_volume: u8,
    ch4_enabled: bool,

    /// output audio buffer
    audio_buffer: VecDeque<i16>,

    /// sample counter for timing audio output
    sample_counter: u32,
    sample_period: u32,
}

impl GbaApu {
    pub fn new() -> Self {
        Self {
            soundcnt_l: 0,
            soundcnt_h: 0,
            soundcnt_x: 0,
            soundbias: 0x200,
            wave_ram: [0; 16],
            direct_sound: [DirectSoundChannel::new(), DirectSoundChannel::new()],
            ch1_sweep: 0,
            ch1_duty_length: 0,
            ch1_envelope: 0,
            ch1_freq: 0,
            ch2_duty_length: 0,
            ch2_envelope: 0,
            ch2_freq: 0,
            ch3_enable: 0,
            ch3_length: 0,
            ch3_volume: 0,
            ch3_freq: 0,
            ch4_length_envelope: 0,
            ch4_envelope: 0,
            ch4_poly: 0,
            ch4_control: 0,
            sequencer_counter: 0,
            sequencer_step: 0,
            ch1_timer: 0,
            ch1_sequence: 0,
            ch1_volume: 0,
            ch1_enabled: false,
            ch2_timer: 0,
            ch2_sequence: 0,
            ch2_volume: 0,
            ch2_enabled: false,
            ch3_timer: 0,
            ch3_position: 0,
            ch3_enabled: false,
            ch4_timer: 0,
            ch4_lfsr: 0x7FFF,
            ch4_volume: 0,
            ch4_enabled: false,
            audio_buffer: VecDeque::with_capacity(4096),
            sample_counter: 0,
            sample_period: super::consts::CPU_FREQ / SAMPLING_RATE,
        }
    }

    pub fn soundcnt_l(&self) -> u16 {
        self.soundcnt_l
    }

    pub fn set_soundcnt_l(&mut self, value: u16) {
        self.soundcnt_l = value;
    }

    pub fn soundcnt_h(&self) -> u16 {
        self.soundcnt_h
    }

    pub fn set_soundcnt_h(&mut self, value: u16) {
        self.soundcnt_h = value;

        // configure DirectSound channels from SOUNDCNT_H
        self.direct_sound[0].volume_full = value & (1 << 2) != 0;
        self.direct_sound[1].volume_full = value & (1 << 3) != 0;
        self.direct_sound[0].enable_right = value & (1 << 8) != 0;
        self.direct_sound[0].enable_left = value & (1 << 9) != 0;
        self.direct_sound[0].timer_id = ((value >> 10) & 1) as usize;
        self.direct_sound[1].enable_right = value & (1 << 12) != 0;
        self.direct_sound[1].enable_left = value & (1 << 13) != 0;
        self.direct_sound[1].timer_id = ((value >> 14) & 1) as usize;

        // reset FIFO if requested
        if value & (1 << 11) != 0 {
            self.direct_sound[0].reset_fifo();
        }
        if value & (1 << 15) != 0 {
            self.direct_sound[1].reset_fifo();
        }
    }

    pub fn soundcnt_x(&self) -> u16 {
        let mut value = self.soundcnt_x & 0x80;
        if self.ch1_enabled {
            value |= 1;
        }
        if self.ch2_enabled {
            value |= 2;
        }
        if self.ch3_enabled {
            value |= 4;
        }
        if self.ch4_enabled {
            value |= 8;
        }
        value
    }

    pub fn set_soundcnt_x(&mut self, value: u16) {
        self.soundcnt_x = value & 0x80;
        if value & 0x80 == 0 {
            // master disable - turn off all channels
            self.ch1_enabled = false;
            self.ch2_enabled = false;
            self.ch3_enabled = false;
            self.ch4_enabled = false;
        }
    }

    pub fn soundbias(&self) -> u16 {
        self.soundbias
    }

    pub fn set_soundbias(&mut self, value: u16) {
        self.soundbias = value;
    }

    pub fn wave_ram(&self) -> &[u8; 16] {
        &self.wave_ram
    }

    pub fn read_wave_ram(&self, offset: usize) -> u8 {
        if offset < 16 {
            self.wave_ram[offset]
        } else {
            0
        }
    }

    pub fn write_wave_ram(&mut self, offset: usize, value: u8) {
        if offset < 16 {
            self.wave_ram[offset] = value;
        }
    }

    pub fn audio_buffer(&self) -> &VecDeque<i16> {
        &self.audio_buffer
    }

    pub fn clear_audio_buffer(&mut self) {
        self.audio_buffer.clear();
    }

    /// called when a timer overflows; feeds DirectSound FIFO channels
    pub fn timer_overflow(&mut self, timer_id: usize) {
        for i in 0..2 {
            if self.direct_sound[i].timer_id == timer_id {
                self.direct_sound[i].timer_tick();
            }
        }
    }

    /// clocks the APU by the given number of CPU cycles
    pub fn clock(&mut self, cycles: u32) {
        if self.soundcnt_x & 0x80 == 0 {
            return; // master sound disabled
        }

        // clock legacy channels
        self.clock_legacy_channels(cycles);

        // generate output samples at the configured rate
        self.sample_counter += cycles;
        while self.sample_counter >= self.sample_period {
            self.sample_counter -= self.sample_period;
            self.generate_sample();
        }
    }

    /// clocks the legacy GB sound channels
    fn clock_legacy_channels(&mut self, cycles: u32) {
        // simplified: just update timers for frequency generation
        let cycles = cycles as i32;

        // channel 1 (square with sweep)
        if self.ch1_enabled {
            self.ch1_timer -= cycles;
            while self.ch1_timer <= 0 {
                let freq = self.ch1_freq & 0x07FF;
                self.ch1_timer += (2048 - freq as i32) * 4;
                self.ch1_sequence = (self.ch1_sequence + 1) & 7;
            }
        }

        // channel 2 (square)
        if self.ch2_enabled {
            self.ch2_timer -= cycles;
            while self.ch2_timer <= 0 {
                let freq = self.ch2_freq & 0x07FF;
                self.ch2_timer += (2048 - freq as i32) * 4;
                self.ch2_sequence = (self.ch2_sequence + 1) & 7;
            }
        }

        // channel 3 (wave)
        if self.ch3_enabled {
            self.ch3_timer -= cycles;
            while self.ch3_timer <= 0 {
                let freq = self.ch3_freq & 0x07FF;
                self.ch3_timer += (2048 - freq as i32) * 2;
                self.ch3_position = (self.ch3_position + 1) & 31;
            }
        }

        // channel 4 (noise)
        if self.ch4_enabled {
            self.ch4_timer -= cycles;
            while self.ch4_timer <= 0 {
                let divisor = match self.ch4_poly & 0x07 {
                    0 => 8,
                    n => (n as i32) * 16,
                };
                let shift = ((self.ch4_poly >> 4) & 0x0F) as i32;
                self.ch4_timer += divisor << shift;

                let xor_bit = (self.ch4_lfsr ^ (self.ch4_lfsr >> 1)) & 1;
                self.ch4_lfsr = (self.ch4_lfsr >> 1) | (xor_bit << 14);
                if self.ch4_poly & (1 << 3) != 0 {
                    self.ch4_lfsr &= !(1 << 6);
                    self.ch4_lfsr |= xor_bit << 6;
                }
            }
        }

        // update frame sequencer (512 Hz)
        self.sequencer_counter += cycles as u32;
        let sequencer_period = super::consts::CPU_FREQ / 512;
        while self.sequencer_counter >= sequencer_period {
            self.sequencer_counter -= sequencer_period;
            self.clock_frame_sequencer();
        }
    }

    /// frame sequencer: handles length, envelope, and sweep at 512 Hz sub-steps
    fn clock_frame_sequencer(&mut self) {
        self.sequencer_step = (self.sequencer_step + 1) & 7;
        // simplified: just envelope on step 7
        // (full implementation would handle length on 0,2,4,6
        //  and sweep on 2,6)
    }

    /// generates a single stereo audio sample (left, right)
    fn generate_sample(&mut self) {
        // mix legacy channels
        let legacy_volume = ((self.soundcnt_h & 0x03) as i16).min(2);
        let legacy_shift = 2 - legacy_volume;

        let ch1_out = if self.ch1_enabled {
            let duty = ((self.ch1_duty_length >> 6) & 0x03) as usize;
            (DUTY_TABLE[duty][self.ch1_sequence as usize] as i16) * self.ch1_volume as i16
        } else {
            0
        };

        let ch2_out = if self.ch2_enabled {
            let duty = ((self.ch2_duty_length >> 6) & 0x03) as usize;
            (DUTY_TABLE[duty][self.ch2_sequence as usize] as i16) * self.ch2_volume as i16
        } else {
            0
        };

        let ch3_out = if self.ch3_enabled {
            let sample_index = self.ch3_position as usize / 2;
            let sample = if self.ch3_position & 1 == 0 {
                (self.wave_ram[sample_index] >> 4) & 0x0F
            } else {
                self.wave_ram[sample_index] & 0x0F
            };
            let volume_shift = match (self.ch3_volume >> 5) & 0x03 {
                0 => 4, // mute
                1 => 0, // 100%
                2 => 1, // 50%
                3 => 2, // 25%
                _ => 4,
            };
            (sample as i16) >> volume_shift
        } else {
            0
        };

        let ch4_out = if self.ch4_enabled {
            let output = if self.ch4_lfsr & 1 == 0 { 1 } else { 0 };
            output * self.ch4_volume as i16
        } else {
            0
        };

        let legacy_left = (ch1_out + ch2_out + ch3_out + ch4_out) >> legacy_shift;
        let legacy_right = legacy_left; // simplified: mono for now

        // mix DirectSound channels
        let ds_a_left = self.direct_sound[0].output_left();
        let ds_a_right = self.direct_sound[0].output_right();
        let ds_b_left = self.direct_sound[1].output_left();
        let ds_b_right = self.direct_sound[1].output_right();

        let left = (legacy_left + ds_a_left / 4 + ds_b_left / 4).clamp(-32768, 32767);
        let right = (legacy_right + ds_a_right / 4 + ds_b_right / 4).clamp(-32768, 32767);

        self.audio_buffer.push_back(left);
        self.audio_buffer.push_back(right);
    }

    /// writes to a legacy channel register
    pub fn write_channel_reg(&mut self, addr: u32, value: u8) {
        let offset = addr & 0xFF;
        match offset {
            // channel 1
            0x60 => self.ch1_sweep = value,
            0x62 => self.ch1_duty_length = value,
            0x63 => {
                self.ch1_envelope = value;
                self.ch1_volume = value >> 4;
            }
            0x64 => self.ch1_freq = (self.ch1_freq & 0xFF00) | value as u16,
            0x65 => {
                self.ch1_freq = (self.ch1_freq & 0x00FF) | ((value as u16 & 0x07) << 8);
                if value & 0x80 != 0 {
                    self.ch1_enabled = true;
                    self.ch1_volume = self.ch1_envelope >> 4;
                }
            }
            // channel 2
            0x68 => self.ch2_duty_length = value,
            0x69 => {
                self.ch2_envelope = value;
                self.ch2_volume = value >> 4;
            }
            0x6C => self.ch2_freq = (self.ch2_freq & 0xFF00) | value as u16,
            0x6D => {
                self.ch2_freq = (self.ch2_freq & 0x00FF) | ((value as u16 & 0x07) << 8);
                if value & 0x80 != 0 {
                    self.ch2_enabled = true;
                    self.ch2_volume = self.ch2_envelope >> 4;
                }
            }
            // channel 3
            0x70 => {
                self.ch3_enable = value;
                if value & 0x80 == 0 {
                    self.ch3_enabled = false;
                }
            }
            0x72 => self.ch3_length = value,
            0x73 => self.ch3_volume = value,
            0x74 => self.ch3_freq = (self.ch3_freq & 0xFF00) | value as u16,
            0x75 => {
                self.ch3_freq = (self.ch3_freq & 0x00FF) | ((value as u16 & 0x07) << 8);
                if value & 0x80 != 0 && self.ch3_enable & 0x80 != 0 {
                    self.ch3_enabled = true;
                }
            }
            // channel 4
            0x78 => self.ch4_length_envelope = value,
            0x79 => {
                self.ch4_envelope = value;
                self.ch4_volume = value >> 4;
            }
            0x7C => self.ch4_poly = value,
            0x7D => {
                self.ch4_control = value;
                if value & 0x80 != 0 {
                    self.ch4_enabled = true;
                    self.ch4_volume = self.ch4_envelope >> 4;
                    self.ch4_lfsr = 0x7FFF;
                }
            }
            _ => {
                warnln!("Unhandled APU register write at offset 0x{:02X}", offset);
            }
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for GbaApu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{DirectSoundChannel, GbaApu};

    #[test]
    fn test_direct_sound_new() {
        let ch = DirectSoundChannel::new();
        assert!(ch.needs_refill());
        assert_eq!(ch.output(), 0);
    }

    #[test]
    fn test_fifo_write_and_tick() {
        let mut channel = DirectSoundChannel::new();
        channel.write_fifo(0x04030201);

        channel.timer_tick();
        assert_eq!(channel.current_sample, 0x01);

        channel.timer_tick();
        assert_eq!(channel.current_sample, 0x02);
    }

    #[test]
    fn test_fifo_needs_refill() {
        let mut channel = DirectSoundChannel::new();
        assert!(channel.needs_refill());

        // fill with 5 words = 20 samples
        for i in 0..5 {
            channel.write_fifo(i);
        }
        assert!(!channel.needs_refill());
    }

    #[test]
    fn test_fifo_max_capacity() {
        let mut channel = DirectSoundChannel::new();
        for i in 0..8 {
            channel.write_fifo(i);
        }
        // further writes should be ignored (capacity = 32)
        channel.write_fifo(0xFFFFFFFF);
        assert_eq!(channel.fifo.len(), 32);
    }

    #[test]
    fn test_fifo_reset() {
        let mut channel = DirectSoundChannel::new();
        channel.write_fifo(0x01020304);
        channel.timer_tick();
        channel.reset_fifo();
        assert!(channel.needs_refill());
        assert_eq!(channel.current_sample, 0);
    }

    #[test]
    fn test_direct_sound_output_volume() {
        let mut channel = DirectSoundChannel::new();
        channel.write_fifo(0x00000040); // sample = 0x40 = 64
        channel.timer_tick();

        channel.volume_full = true;
        let full = channel.output();

        channel.volume_full = false;
        let half = channel.output();

        assert_eq!(half, full / 2);
    }

    #[test]
    fn test_direct_sound_output_left_right() {
        let mut channel = DirectSoundChannel::new();
        channel.write_fifo(0x00000010);
        channel.timer_tick();

        assert_eq!(channel.output_left(), 0);
        assert_eq!(channel.output_right(), 0);

        channel.enable_left = true;
        assert_ne!(channel.output_left(), 0);
        assert_eq!(channel.output_right(), 0);

        channel.enable_right = true;
        assert_ne!(channel.output_right(), 0);
    }

    #[test]
    fn test_apu_new() {
        let apu = GbaApu::new();
        assert_eq!(apu.soundcnt_l(), 0);
        assert_eq!(apu.soundcnt_h(), 0);
        assert_eq!(apu.soundcnt_x(), 0);
        assert_eq!(apu.soundbias(), 0x200);
        assert!(apu.audio_buffer().is_empty());
    }

    #[test]
    fn test_apu_soundcnt_registers() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_l(0x1234);
        assert_eq!(apu.soundcnt_l(), 0x1234);

        apu.set_soundbias(0x300);
        assert_eq!(apu.soundbias(), 0x300);
    }

    #[test]
    fn test_apu_soundcnt_h_configures_direct_sound() {
        let mut apu = GbaApu::new();
        // enable A right(8), A left(9), A timer 1(10), A vol full(2)
        apu.set_soundcnt_h((1 << 8) | (1 << 9) | (1 << 10) | (1 << 2));
        assert!(apu.direct_sound[0].enable_right);
        assert!(apu.direct_sound[0].enable_left);
        assert_eq!(apu.direct_sound[0].timer_id, 1);
        assert!(apu.direct_sound[0].volume_full);
    }

    #[test]
    fn test_apu_soundcnt_h_reset_fifo() {
        let mut apu = GbaApu::new();
        apu.direct_sound[0].write_fifo(0x01020304);
        assert!(!apu.direct_sound[0].fifo.is_empty());

        apu.set_soundcnt_h(1 << 11);
        assert!(apu.direct_sound[0].fifo.is_empty());
    }

    #[test]
    fn test_apu_soundcnt_x_master_disable() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);
        assert_eq!(apu.soundcnt_x() & 0x80, 0x80);

        apu.set_soundcnt_x(0x00);
        assert_eq!(apu.soundcnt_x() & 0x80, 0);
    }

    #[test]
    fn test_apu_wave_ram() {
        let mut apu = GbaApu::new();
        apu.write_wave_ram(0, 0xAB);
        assert_eq!(apu.read_wave_ram(0), 0xAB);
        assert_eq!(apu.read_wave_ram(16), 0); // out of bounds
    }

    #[test]
    fn test_apu_timer_overflow() {
        let mut apu = GbaApu::new();
        apu.direct_sound[0].timer_id = 0;
        apu.direct_sound[0].write_fifo(0x00000042);

        apu.timer_overflow(0);
        assert_eq!(apu.direct_sound[0].current_sample, 0x42);
    }

    #[test]
    fn test_apu_clear_audio_buffer() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);
        apu.clock(1000);
        apu.clear_audio_buffer();
        assert!(apu.audio_buffer().is_empty());
    }

    #[test]
    fn test_apu_clock_disabled() {
        let mut apu = GbaApu::new();
        apu.clock(1000);
        assert!(apu.audio_buffer().is_empty());
    }

    #[test]
    fn test_apu_channel_trigger() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);
        apu.write_channel_reg(0x0400_0063, 0xF0);
        apu.write_channel_reg(0x0400_0065, 0x80);
        assert_eq!(apu.soundcnt_x() & 1, 1);
    }

    #[test]
    fn test_apu_reset() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_l(0x1234);
        apu.set_soundcnt_x(0x80);
        apu.reset();
        assert_eq!(apu.soundcnt_l(), 0);
        assert_eq!(apu.soundcnt_x(), 0);
    }
}
