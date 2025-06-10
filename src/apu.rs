//! APU (Audio Processing Unit) functions and structures.

use std::{collections::VecDeque, io::Cursor};

use boytacean_common::{
    data::{
        read_f32, read_i16, read_i32, read_into, read_u16, read_u8, write_bytes, write_f32,
        write_i16, write_i32, write_u16, write_u8,
    },
    error::Error,
};

use crate::{
    consts::{
        NR10_ADDR, NR11_ADDR, NR12_ADDR, NR13_ADDR, NR14_ADDR, NR20_ADDR, NR21_ADDR, NR22_ADDR,
        NR23_ADDR, NR24_ADDR, NR30_ADDR, NR31_ADDR, NR32_ADDR, NR33_ADDR, NR34_ADDR, NR40_ADDR,
        NR41_ADDR, NR42_ADDR, NR43_ADDR, NR44_ADDR, NR50_ADDR, NR51_ADDR, NR52_ADDR,
    },
    gb::GameBoy,
    mmu::BusComponent,
    state::{StateComponent, StateFormat},
    warnln,
};

const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];

const CH4_DIVISORS: [u8; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

/// The base rate for the filter, this is used to calculate the
/// filter rate based on the clock frequency and the sampling rate.
const FILTER_RATE_BASE: f64 = 0.999958;

pub enum Channel {
    Ch1,
    Ch2,
    Ch3,
    Ch4,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HighPassFilter {
    Preserve,
    Accurate,
    Disable,
}

impl HighPassFilter {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => HighPassFilter::Disable,
            2 => HighPassFilter::Preserve,
            3 => HighPassFilter::Accurate,
            _ => HighPassFilter::Disable,
        }
    }
}

impl From<u8> for HighPassFilter {
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

impl From<HighPassFilter> for u8 {
    fn from(val: HighPassFilter) -> Self {
        match val {
            HighPassFilter::Disable => 1,
            HighPassFilter::Preserve => 2,
            HighPassFilter::Accurate => 3,
        }
    }
}

pub struct Apu {
    ch1_timer: i16,
    ch1_sequence: u8,
    ch1_envelope_sequence: u8,
    ch1_envelope_enabled: bool,
    ch1_sweep_sequence: u8,
    ch1_output: u8,
    ch1_dac: bool,
    ch1_sweep_slope: u8,
    ch1_sweep_increase: bool,
    ch1_sweep_pace: u8,
    ch1_length_timer: u8,
    ch1_wave_duty: u8,
    ch1_pace: u8,
    ch1_direction: u8,
    ch1_volume: u8,
    ch1_wave_length: u16,
    ch1_length_enabled: bool,
    ch1_enabled: bool,

    ch2_timer: i16,
    ch2_sequence: u8,
    ch2_envelope_sequence: u8,
    ch2_envelope_enabled: bool,
    ch2_output: u8,
    ch2_dac: bool,
    ch2_length_timer: u8,
    ch2_wave_duty: u8,
    ch2_pace: u8,
    ch2_direction: u8,
    ch2_volume: u8,
    ch2_wave_length: u16,
    ch2_length_enabled: bool,
    ch2_enabled: bool,

    ch3_timer: i16,
    ch3_position: u8,
    ch3_output: u8,
    ch3_dac: bool,
    ch3_length_timer: u16,
    ch3_output_level: u8,
    ch3_wave_length: u16,
    ch3_length_enabled: bool,
    ch3_enabled: bool,

    ch4_timer: i32,
    ch4_envelope_sequence: u8,
    ch4_envelope_enabled: bool,
    ch4_output: u8,
    ch4_dac: bool,
    ch4_length_timer: u8,
    ch4_pace: u8,
    ch4_direction: u8,
    ch4_volume: u8,
    ch4_divisor: u8,
    ch4_width_mode: bool,
    ch4_clock_shift: u8,
    ch4_lfsr: u16,
    ch4_length_enabled: bool,
    ch4_enabled: bool,

    master: u8,
    glob_panning: u8,

    right_enabled: bool,
    left_enabled: bool,
    sound_enabled: bool,

    ch1_out_enabled: bool,
    ch2_out_enabled: bool,
    ch3_out_enabled: bool,
    ch4_out_enabled: bool,

    /// The RAM that is used to sore the wave information
    /// to be used in channel 3 audio
    wave_ram: [u8; 16],

    /// The rate at which audio samples are going to be
    /// taken, ideally this value should be aligned with
    /// the sampling rate of the output device. A typical
    /// sampling rate would be of 44.1kHz.
    sampling_rate: u16,

    /// The number of audion channels that are going to be
    /// outputted as part fo the audio buffer)
    channels: u8,

    /// Internal sequencer counter that runs at 512Hz
    /// used for the activation of the tick actions.
    sequencer: u16,
    sequencer_step: u8,
    output_timer: i16,

    /// The delta value that is used to calculate the
    /// output timer, this is the amount of cycles that
    /// should be used to create a new audio sample.
    output_timer_delta: i16,

    /// The audio buffer that is used to store the audio
    /// samples that are going to be outputted, uses an
    /// integer (i16) deque to store the audio samples.
    audio_buffer: VecDeque<i16>,
    audio_buffer_max: usize,

    filter_mode: HighPassFilter,
    filter_rate: f32,
    filter_diff: [f32; 2],

    clock_freq: u32,
}

impl Apu {
    pub fn new(sampling_rate: u16, channels: u8, buffer_size: f32, clock_freq: u32) -> Self {
        Self {
            ch1_timer: 0,
            ch1_sequence: 0,
            ch1_envelope_sequence: 0,
            ch1_envelope_enabled: false,
            ch1_sweep_sequence: 0,
            ch1_output: 0,
            ch1_dac: false,
            ch1_sweep_slope: 0x0,
            ch1_sweep_increase: true,
            ch1_sweep_pace: 0x0,
            ch1_length_timer: 0x0,
            ch1_wave_duty: 0x0,
            ch1_pace: 0x0,
            ch1_direction: 0x0,
            ch1_volume: 0x0,
            ch1_wave_length: 0x0,
            ch1_length_enabled: false,
            ch1_enabled: false,

            ch2_timer: 0,
            ch2_sequence: 0,
            ch2_envelope_sequence: 0,
            ch2_envelope_enabled: false,
            ch2_output: 0,
            ch2_dac: false,
            ch2_length_timer: 0x0,
            ch2_wave_duty: 0x0,
            ch2_pace: 0x0,
            ch2_direction: 0x0,
            ch2_volume: 0x0,
            ch2_wave_length: 0x0,
            ch2_length_enabled: false,
            ch2_enabled: false,

            ch3_timer: 0,
            ch3_position: 0,
            ch3_output: 0,
            ch3_dac: false,
            ch3_length_timer: 0x0,
            ch3_output_level: 0x0,
            ch3_wave_length: 0x0,
            ch3_length_enabled: false,
            ch3_enabled: false,

            ch4_timer: 0,
            ch4_envelope_sequence: 0,
            ch4_envelope_enabled: false,
            ch4_output: 0,
            ch4_dac: false,
            ch4_length_timer: 0x0,
            ch4_pace: 0x0,
            ch4_direction: 0x0,
            ch4_volume: 0x0,
            ch4_divisor: 0x0,
            ch4_width_mode: false,
            ch4_clock_shift: 0x0,
            ch4_lfsr: 0x0,
            ch4_length_enabled: false,
            ch4_enabled: false,

            master: 0x0,
            glob_panning: 0x0,

            left_enabled: true,
            right_enabled: true,
            sound_enabled: true,

            ch1_out_enabled: true,
            ch2_out_enabled: true,
            ch3_out_enabled: true,
            ch4_out_enabled: true,

            wave_ram: [0u8; 16],

            sampling_rate,
            channels,

            sequencer: 0,
            sequencer_step: 0,
            output_timer: 0,
            output_timer_delta: (clock_freq as f32 / sampling_rate as f32) as i16,
            audio_buffer: VecDeque::with_capacity(
                (sampling_rate as f32 * buffer_size) as usize * channels as usize,
            ),
            audio_buffer_max: (sampling_rate as f32 * buffer_size) as usize * channels as usize,
            filter_mode: HighPassFilter::Accurate,
            filter_rate: FILTER_RATE_BASE.powf(clock_freq as f64 / sampling_rate as f64) as f32,
            filter_diff: [0.0; 2],
            clock_freq,
        }
    }

    pub fn reset(&mut self) {
        self.ch1_timer = 0;
        self.ch1_sequence = 0;
        self.ch1_envelope_sequence = 0;
        self.ch1_envelope_enabled = false;
        self.ch1_sweep_sequence = 0;
        self.ch1_output = 0;
        self.ch1_dac = false;
        self.ch1_sweep_slope = 0x0;
        self.ch1_sweep_increase = true;
        self.ch1_sweep_pace = 0x0;
        self.ch1_length_timer = 0x0;
        self.ch1_wave_duty = 0x0;
        self.ch1_pace = 0x0;
        self.ch1_direction = 0x0;
        self.ch1_volume = 0x0;
        self.ch1_wave_length = 0x0;
        self.ch1_length_enabled = false;
        self.ch1_enabled = false;

        self.ch2_timer = 0;
        self.ch2_sequence = 0;
        self.ch2_envelope_sequence = 0;
        self.ch2_envelope_enabled = false;
        self.ch2_output = 0;
        self.ch2_dac = false;
        self.ch2_length_timer = 0x0;
        self.ch2_wave_duty = 0x0;
        self.ch2_pace = 0x0;
        self.ch2_direction = 0x0;
        self.ch2_volume = 0x0;
        self.ch2_wave_length = 0x0;
        self.ch2_length_enabled = false;
        self.ch2_enabled = false;

        self.ch3_timer = 0;
        self.ch3_position = 0;
        self.ch3_output = 0;
        self.ch3_dac = false;
        self.ch3_length_timer = 0x0;
        self.ch3_output_level = 0x0;
        self.ch3_wave_length = 0x0;
        self.ch3_length_enabled = false;
        self.ch3_enabled = false;

        self.ch4_timer = 0;
        self.ch4_envelope_sequence = 0;
        self.ch4_envelope_enabled = false;
        self.ch4_output = 0;
        self.ch4_dac = false;
        self.ch4_length_timer = 0x0;
        self.ch4_pace = 0x0;
        self.ch4_direction = 0x0;
        self.ch4_volume = 0x0;
        self.ch4_divisor = 0x0;
        self.ch4_width_mode = false;
        self.ch4_clock_shift = 0x0;
        self.ch4_lfsr = 0x0;
        self.ch4_length_enabled = false;
        self.ch4_enabled = false;

        self.master = 0x0;
        self.glob_panning = 0x0;

        self.left_enabled = true;
        self.right_enabled = true;
        self.sound_enabled = true;

        self.sequencer = 0;
        self.sequencer_step = 0;
        self.output_timer = 0;

        self.filter_diff = [0.0; 2];

        self.clear_audio_buffer()
    }

    pub fn clock(&mut self, cycles: u16) {
        if !self.sound_enabled {
            return;
        }

        self.sequencer += cycles;
        if self.sequencer >= 8192 {
            // each of these steps runs at 512/8 Hz = 64Hz,
            // meaning a complete loop runs at 512 Hz
            match self.sequencer_step {
                0 => {
                    self.tick_length_all();
                }
                1 => (),
                2 => {
                    self.tick_ch1_sweep();
                    self.tick_length_all();
                }
                3 => (),
                4 => {
                    self.tick_length_all();
                }
                5 => (),
                6 => {
                    self.tick_ch1_sweep();
                    self.tick_length_all();
                }
                7 => {
                    self.tick_envelope_all();
                }
                _ => (),
            }

            self.sequencer -= 8192;
            self.sequencer_step = (self.sequencer_step + 1) & 7;
        }

        self.tick_ch_all(cycles);

        self.output_timer = self.output_timer.saturating_sub(cycles as i16);
        if self.output_timer <= 0 {
            // verifies if we've reached the maximum allowed size for the
            // audio buffer and if that's the case an item is removed from
            // the buffer (avoiding overflow)
            if self.audio_buffer.len() >= self.audio_buffer_max {
                for _ in 0..self.channels {
                    self.audio_buffer.pop_front();
                }
            }

            // obtains the output sample (uses the same for both channels)
            // and filters it based on the channel configuration
            let sample = self.output();
            if self.left_enabled {
                let value = self.filter_sample(sample, 0);
                self.audio_buffer.push_back(value);
            }
            if self.right_enabled && self.channels > 1 {
                let value = self.filter_sample(sample, 1);
                self.audio_buffer.push_back(value);
            }

            // calculates the rate at which a new audio sample should be
            // created based on the (base/CPU) clock frequency and the
            // sampling rate, this is basically the amount of APU clock
            // calls that should be performed until an audio sample is created
            self.output_timer += self.output_timer_delta;
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            // 0xFF10 — NR10: Channel 1 sweep
            NR10_ADDR => {
                (self.ch1_sweep_slope & 0x07)
                    | (if self.ch1_sweep_increase { 0x00 } else { 0x08 })
                    | ((self.ch1_sweep_pace & 0x07) << 4)
                    | 0x80
            }
            // 0xFF11 — NR11: Channel 1 length timer & duty cycle
            NR11_ADDR => ((self.ch1_wave_duty & 0x03) << 6) | 0x3f,
            // 0xFF12 — NR12: Channel 1 volume & envelope
            NR12_ADDR => {
                (self.ch1_pace & 0x07)
                    | ((self.ch1_direction & 0x01) << 3)
                    | ((self.ch1_volume & 0x0f) << 4)
            }
            // 0xFF13 — NR13: Channel 1 wavelength low
            NR13_ADDR => 0xff,
            // 0xFF14 — NR14: Channel 1 wavelength high & control
            NR14_ADDR => (if self.ch1_length_enabled { 0x40 } else { 0x00 }) | 0xbf,

            // 0xFF15 — NR20: Not used
            NR20_ADDR => 0xff,
            // 0xFF16 — NR21: Channel 2 length timer & duty cycle
            NR21_ADDR => ((self.ch2_wave_duty & 0x03) << 6) | 0x3f,
            // 0xFF17 — NR22: Channel 2 volume & envelope
            NR22_ADDR => {
                (self.ch2_pace & 0x07)
                    | ((self.ch2_direction & 0x01) << 3)
                    | ((self.ch2_volume & 0x0f) << 4)
            }
            // 0xFF18 — NR23: Channel 2 wavelength low
            NR23_ADDR => 0xff,
            // 0xFF19 — NR24: Channel 2 wavelength high & control
            NR24_ADDR => (if self.ch2_length_enabled { 0x40 } else { 0x00 }) | 0xbf,

            // 0xFF1A — NR30: Channel 3 DAC enable
            NR30_ADDR => (if self.ch3_dac { 0x80 } else { 0x00 }) | 0x7f,
            // 0xFF1B — NR31: Channel 3 length timer
            NR31_ADDR => 0xff,
            // 0xFF1C — NR32: Channel 3 output level
            NR32_ADDR => ((self.ch3_output_level & 0x03) << 5) | 0x9f,
            // 0xFF1D — NR33: Channel 3 wavelength low
            NR33_ADDR => 0xff,
            // 0xFF1E — NR34: Channel 3 wavelength high & control
            NR34_ADDR => (if self.ch3_length_enabled { 0x40 } else { 0x00 }) | 0xbf,

            // 0xFF1F — NR40: Not used
            NR40_ADDR => 0xff,
            // 0xFF20 — NR41: Channel 4 length timer
            NR41_ADDR => 0xff,
            // 0xFF21 — NR42: Channel 4 volume & envelope
            NR42_ADDR => {
                (self.ch4_pace & 0x07)
                    | ((self.ch4_direction & 0x01) << 3)
                    | ((self.ch4_volume & 0x0f) << 4)
            }
            // 0xFF22 — NR43: Channel 4 frequency & randomness
            NR43_ADDR => {
                (self.ch4_divisor & 0x07)
                    | if self.ch4_width_mode { 0x08 } else { 0x00 }
                    | ((self.ch4_clock_shift & 0x0f) << 4)
            }
            // 0xFF23 — NR44: Channel 4 control
            NR44_ADDR => (if self.ch4_length_enabled { 0x40 } else { 0x00 }) | 0xbf,

            // 0xFF24 — NR50: Master volume & VIN panning
            NR50_ADDR => self.master,
            // 0xFF25 — NR51: Sound panning
            NR51_ADDR => self.glob_panning,
            // 0xFF26 — NR52: Sound on/off
            NR52_ADDR =>
            {
                #[allow(clippy::bool_to_int_with_if)]
                ((if self.ch1_enabled && self.ch1_dac {
                    0x01
                } else {
                    0x00
                } | if self.ch2_enabled && self.ch2_dac {
                    0x02
                } else {
                    0x00
                } | if self.ch3_enabled && self.ch3_dac {
                    0x04
                } else {
                    0x00
                } | if self.ch4_enabled && self.ch4_dac {
                    0x08
                } else {
                    0x00
                } | if self.sound_enabled { 0x80 } else { 0x00 })
                    | 0x70)
            }

            // 0xFF30-0xFF3F — Wave pattern RAM
            0xff30..=0xff3f => self.wave_ram[addr as usize & 0x000f],

            _ => {
                warnln!("Reading from unknown APU location 0x{:04x}", addr);
                #[allow(unreachable_code)]
                0xff
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        // in case the sound is disabled then ignores writes
        // to any register aside from the sound on/off
        if !self.sound_enabled && addr != 0xff26 {
            return;
        }

        match addr {
            // 0xFF10 — NR10: Channel 1 sweep
            NR10_ADDR => {
                self.ch1_sweep_slope = value & 0x07;
                self.ch1_sweep_increase = value & 0x08 == 0x00;
                self.ch1_sweep_pace = (value & 0x70) >> 4;
                self.ch1_sweep_sequence = 0;
            }
            // 0xFF11 — NR11: Channel 1 length timer & duty cycle
            NR11_ADDR => {
                self.ch1_length_timer = 64 - (value & 0x3f);
                self.ch1_wave_duty = (value & 0xc0) >> 6;
            }
            // 0xFF12 — NR12: Channel 1 volume & envelope
            NR12_ADDR => {
                self.ch1_pace = value & 0x07;
                self.ch1_direction = (value & 0x08) >> 3;
                self.ch1_volume = (value & 0xf0) >> 4;
                self.ch1_envelope_enabled = self.ch1_pace > 0;
                self.ch1_envelope_sequence = 0;
                self.ch1_dac = value & 0xf8 != 0x00;
                if !self.ch1_dac {
                    self.ch1_enabled = false;
                }
            }
            // 0xFF13 — NR13: Channel 1 wavelength low
            NR13_ADDR => {
                self.ch1_wave_length = (self.ch1_wave_length & 0xff00) | value as u16;
            }
            // 0xFF14 — NR14: Channel 1 wavelength high & control
            NR14_ADDR => {
                let length_trigger = value & 0x40 == 0x40;
                let trigger = value & 0x80 == 0x80;
                let length_edge = length_trigger && !self.ch1_length_enabled;
                self.ch1_wave_length =
                    (self.ch1_wave_length & 0x00ff) | (((value & 0x07) as u16) << 8);
                self.ch1_length_enabled = value & 0x40 == 0x40;
                self.ch1_enabled |= trigger;
                if length_edge && self.sequencer_step % 2 == 1 {
                    self.tick_length(Channel::Ch1);
                }
                if trigger {
                    self.trigger_ch1();
                }
                if length_trigger && self.ch1_length_timer == 0 {
                    self.ch1_enabled = false;
                }
            }

            // 0xFF15 — NR20: Not used
            NR20_ADDR => (),
            // 0xFF16 — NR21: Channel 2 length timer & duty cycle
            NR21_ADDR => {
                self.ch2_length_timer = 64 - (value & 0x3f);
                self.ch2_wave_duty = (value & 0xc0) >> 6;
            }
            // 0xFF17 — NR22: Channel 2 volume & envelope
            NR22_ADDR => {
                self.ch2_pace = value & 0x07;
                self.ch2_direction = (value & 0x08) >> 3;
                self.ch2_volume = (value & 0xf0) >> 4;
                self.ch2_envelope_enabled = self.ch2_pace > 0;
                self.ch2_envelope_sequence = 0;
                self.ch2_dac = value & 0xf8 != 0x00;
                if !self.ch2_dac {
                    self.ch2_enabled = false;
                }
            }
            // 0xFF18 — NR23: Channel 2 wavelength low
            NR23_ADDR => {
                self.ch2_wave_length = (self.ch2_wave_length & 0xff00) | value as u16;
            }
            // 0xFF19 — NR24: Channel 2 wavelength high & control
            NR24_ADDR => {
                let length_trigger = value & 0x40 == 0x40;
                let trigger = value & 0x80 == 0x80;
                let length_edge = length_trigger && !self.ch2_length_enabled;
                self.ch2_wave_length =
                    (self.ch2_wave_length & 0x00ff) | (((value & 0x07) as u16) << 8);
                self.ch2_length_enabled = length_trigger;
                self.ch2_enabled |= trigger;
                if length_edge && self.sequencer_step % 2 == 1 {
                    self.tick_length(Channel::Ch2);
                }
                if trigger {
                    self.trigger_ch2();
                }
                if length_trigger && self.ch2_length_timer == 0 {
                    self.ch2_enabled = false;
                }
            }

            // 0xFF1A — NR30: Channel 3 DAC enable
            NR30_ADDR => {
                self.ch3_dac = value & 0x80 == 0x80;
                if !self.ch3_dac {
                    self.ch3_enabled = false;
                }
            }
            // 0xFF1B — NR31: Channel 3 length timer
            NR31_ADDR => {
                self.ch3_length_timer = 256 - (value as u16);
            }
            // 0xFF1C — NR32: Channel 3 output level
            NR32_ADDR => {
                self.ch3_output_level = (value & 0x60) >> 5;
            }
            // 0xFF1D — NR33: Channel 3 wavelength low
            NR33_ADDR => {
                self.ch3_wave_length = (self.ch3_wave_length & 0xff00) | value as u16;
            }
            // 0xFF1E — NR34: Channel 3 wavelength high & control
            NR34_ADDR => {
                let length_trigger = value & 0x40 == 0x40;
                let trigger = value & 0x80 == 0x80;
                let length_edge = length_trigger && !self.ch3_length_enabled;
                self.ch3_wave_length =
                    (self.ch3_wave_length & 0x00ff) | (((value & 0x07) as u16) << 8);
                self.ch3_length_enabled = length_trigger;
                self.ch3_enabled |= trigger;
                if length_edge && self.sequencer_step % 2 == 1 {
                    self.tick_length(Channel::Ch3);
                }
                if trigger {
                    self.trigger_ch3();
                }
                if length_trigger && self.ch3_length_timer == 0 {
                    self.ch3_enabled = false;
                }
            }

            // 0xFF1F — Not used
            NR40_ADDR => (),
            // 0xFF20 — NR41: Channel 4 length timer
            NR41_ADDR => {
                self.ch4_length_timer = 64 - (value & 0x3f);
            }
            // 0xFF21 — NR42: Channel 4 volume & envelope
            NR42_ADDR => {
                self.ch4_pace = value & 0x07;
                self.ch4_direction = (value & 0x08) >> 3;
                self.ch4_volume = (value & 0xf0) >> 4;
                self.ch4_envelope_enabled = self.ch4_pace > 0;
                self.ch4_envelope_sequence = 0;
                self.ch4_dac = value & 0xf8 != 0x00;
                if !self.ch4_dac {
                    self.ch4_enabled = false;
                }
            }
            // 0xFF22 — NR43: Channel 4 frequency & randomness
            NR43_ADDR => {
                self.ch4_divisor = value & 0x07;
                self.ch4_width_mode = value & 0x08 == 0x08;
                self.ch4_clock_shift = (value & 0xf0) >> 4;
            }
            // 0xFF23 — NR44: Channel 4 control
            NR44_ADDR => {
                let length_trigger = value & 0x40 == 0x40;
                let trigger = value & 0x80 == 0x80;
                let length_edge = length_trigger && !self.ch4_length_enabled;
                self.ch4_length_enabled = length_trigger;
                self.ch4_enabled |= trigger;
                if length_edge && self.sequencer_step % 2 == 1 {
                    self.tick_length(Channel::Ch4);
                }
                if trigger {
                    self.trigger_ch4();
                }
                if length_trigger && self.ch4_length_timer == 0 {
                    self.ch4_enabled = false;
                }
            }

            // 0xFF24 — NR50: Master volume & VIN panning
            NR50_ADDR => {
                self.master = value;
            }
            // 0xFF25 — NR51: Sound panning
            NR51_ADDR => {
                self.glob_panning = value;
            }
            // 0xFF26 — NR52: Sound on/off
            NR52_ADDR => {
                self.sound_enabled = value & 0x80 == 0x80;
                if !self.sound_enabled {
                    self.reset();
                    self.sound_enabled = false;
                }
            }

            // 0xFF30-0xFF3F — Wave pattern RAM
            0xff30..=0xff3f => self.wave_ram[addr as usize & 0x000f] = value,

            _ => warnln!("Writing in unknown APU location 0x{:04x}", addr),
        }
    }

    pub fn read_raw(&mut self, addr: u16) -> u8 {
        match addr {
            // 0xFF11 — NR11: Channel 1 length timer & duty cycle
            NR11_ADDR => ((64 - self.ch1_length_timer) & 0x3f) | ((self.ch1_wave_duty & 0x03) << 6),
            // 0xFF14 — NR14: Channel 1 wavelength high & control
            NR14_ADDR => {
                (if self.ch1_length_enabled { 0x40 } else { 0x00 })
                    | (if self.ch1_enabled { 0x80 } else { 0x00 })
                    | (((self.ch1_wave_length & (0x0700 >> 8)) as u8) << 3)
                    | 0x38
            }

            // 0xFF16 — NR21: Channel 2 length timer & duty cycle
            NR21_ADDR => ((64 - self.ch2_length_timer) & 0x3f) | ((self.ch2_wave_duty & 0x03) << 6),
            // 0xFF19 — NR24: Channel 2 wavelength high & control
            NR24_ADDR => {
                (if self.ch2_length_enabled { 0x40 } else { 0x00 })
                    | (if self.ch2_enabled { 0x80 } else { 0x00 })
                    | (((self.ch2_wave_length & (0x0700 >> 8)) as u8) << 3)
                    | 0x38
            }

            // 0xFF1B — NR31: Channel 3 length timer
            NR31_ADDR => (255 - self.ch3_length_timer as u8).saturating_add(1),
            // 0xFF1E — NR34: Channel 3 wavelength high & control
            NR34_ADDR => {
                (if self.ch3_length_enabled { 0x40 } else { 0x00 })
                    | (if self.ch3_enabled { 0x80 } else { 0x00 })
                    | (((self.ch3_wave_length & (0x0700 >> 8)) as u8) << 3)
                    | 0x38
            }

            // 0xFF20 — NR41: Channel 4 length timer
            NR41_ADDR => (64 - self.ch4_length_timer) & 0x3f,
            // 0xFF23 — NR44: Channel 4 control
            NR44_ADDR => {
                (if self.ch4_length_enabled { 0x40 } else { 0x00 })
                    | (if self.ch4_enabled { 0x80 } else { 0x00 })
                    | 0x3f
            }

            _ => self.read(addr),
        }
    }

    pub fn write_raw(&mut self, addr: u16, value: u8) {
        match addr {
            // 0xFF26 — NR52: Sound on/off
            NR52_ADDR => {
                self.ch1_enabled = value & 0x01 == 0x01;
                self.ch2_enabled = value & 0x02 == 0x02;
                self.ch3_enabled = value & 0x04 == 0x04;
                self.ch4_enabled = value & 0x08 == 0x08;
                self.sound_enabled = value & 0x80 == 0x80;
                self.ch1_dac = self.ch1_enabled;
                self.ch2_dac = self.ch2_enabled;
                self.ch3_dac = self.ch3_enabled;
                self.ch4_dac = self.ch4_enabled;
                if !self.sound_enabled {
                    self.reset();
                    self.sound_enabled = false;
                }
            }

            _ => self.write(addr, value),
        }
    }

    /// Returns the current output of the APU, this is the sum
    /// of the outputs of all channels, this is used to
    /// calculate the final audio output that is going to be
    /// sent to the audio buffer.
    ///
    /// This value is not filtered, it is the raw output
    /// of the channels, the filtering is done in the `filter_sample`
    /// method, which is called when the audio sample is created.
    #[inline(always)]
    pub fn output(&self) -> u16 {
        self.ch1_output() as u16
            + self.ch2_output() as u16
            + self.ch3_output() as u16
            + self.ch4_output() as u16
    }

    /// Filters the given sample based on the current filter mode
    /// and returns the filtered sample as an i16 value.
    ///
    /// The `channel` parameter is used to determine which channel
    /// the sample belongs to, this is used to apply the correct
    /// filtering based on the channel's configuration.
    #[inline(always)]
    fn filter_sample(&mut self, sample: u16, channel: usize) -> i16 {
        match self.filter_mode {
            HighPassFilter::Disable => sample as i16,
            HighPassFilter::Accurate => {
                let input = sample as f32;
                let output = input - self.filter_diff[channel];
                self.filter_diff[channel] =
                    input - (input - self.filter_diff[channel]) * self.filter_rate;
                output as i16
            }
            HighPassFilter::Preserve => {
                let input = sample as f32;
                let output = input - self.filter_diff[channel];
                let volume_bits = if channel == 0 {
                    ((self.master >> 4) & 0x07) as f32
                } else {
                    (self.master & 0x07) as f32
                };
                let volume = (volume_bits + 1.0) * 15.0;
                self.filter_diff[channel] = volume * (1.0 - self.filter_rate)
                    + self.filter_diff[channel] * self.filter_rate;
                output as i16
            }
        }
    }

    #[inline(always)]
    pub fn ch1_output(&self) -> u8 {
        if self.ch1_out_enabled {
            self.ch1_output
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn ch2_output(&self) -> u8 {
        if self.ch2_out_enabled {
            self.ch2_output
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn ch3_output(&self) -> u8 {
        if self.ch3_out_enabled {
            self.ch3_output
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn ch4_output(&self) -> u8 {
        if self.ch4_out_enabled {
            self.ch4_output
        } else {
            0
        }
    }

    pub fn ch1_out_enabled(&self) -> bool {
        self.ch1_out_enabled
    }

    pub fn set_ch1_out_enabled(&mut self, enabled: bool) {
        self.ch1_out_enabled = enabled;
    }

    pub fn ch2_out_enabled(&self) -> bool {
        self.ch2_out_enabled
    }

    pub fn set_ch2_out_enabled(&mut self, enabled: bool) {
        self.ch2_out_enabled = enabled;
    }

    pub fn ch3_out_enabled(&self) -> bool {
        self.ch3_out_enabled
    }

    pub fn set_ch3_out_enabled(&mut self, enabled: bool) {
        self.ch3_out_enabled = enabled;
    }

    pub fn ch4_out_enabled(&self) -> bool {
        self.ch4_out_enabled
    }

    pub fn set_ch4_out_enabled(&mut self, enabled: bool) {
        self.ch4_out_enabled = enabled;
    }

    pub fn sampling_rate(&self) -> u16 {
        self.sampling_rate
    }

    pub fn channels(&self) -> u8 {
        self.channels
    }

    pub fn filter_mode(&self) -> HighPassFilter {
        self.filter_mode
    }

    pub fn set_filter_mode(&mut self, mode: HighPassFilter) {
        self.filter_mode = mode;
        self.filter_diff = [0.0; 2];
    }

    pub fn audio_buffer(&self) -> &VecDeque<i16> {
        &self.audio_buffer
    }

    pub fn audio_buffer_mut(&mut self) -> &mut VecDeque<i16> {
        &mut self.audio_buffer
    }

    pub fn clear_audio_buffer(&mut self) {
        self.audio_buffer.clear();
    }

    pub fn audio_buffer_max(&self) -> usize {
        self.audio_buffer_max
    }

    pub fn clock_freq(&self) -> u32 {
        self.clock_freq
    }

    pub fn set_clock_freq(&mut self, value: u32) {
        self.clock_freq = value;
        self.filter_rate =
            FILTER_RATE_BASE.powf(self.clock_freq as f64 / self.sampling_rate as f64) as f32;
        self.output_timer_delta = (self.clock_freq as f32 / self.sampling_rate as f32) as i16;
    }

    #[inline(always)]
    fn tick_length_all(&mut self) {
        self.tick_length(Channel::Ch1);
        self.tick_length(Channel::Ch2);
        self.tick_length(Channel::Ch3);
        self.tick_length(Channel::Ch4);
    }

    #[inline(always)]
    fn tick_length(&mut self, channel: Channel) {
        match channel {
            Channel::Ch1 => {
                if !self.ch1_length_enabled || self.ch1_length_timer == 0 {
                    return;
                }
                self.ch1_length_timer = self.ch1_length_timer.saturating_sub(1);
                if self.ch1_length_timer == 0 {
                    self.ch1_enabled = false;
                }
            }
            Channel::Ch2 => {
                if !self.ch2_length_enabled || self.ch2_length_timer == 0 {
                    return;
                }
                self.ch2_length_timer = self.ch2_length_timer.saturating_sub(1);
                if self.ch2_length_timer == 0 {
                    self.ch2_enabled = false;
                }
            }
            Channel::Ch3 => {
                if !self.ch3_length_enabled || self.ch3_length_timer == 0 {
                    return;
                }
                self.ch3_length_timer = self.ch3_length_timer.saturating_sub(1);
                if self.ch3_length_timer == 0 {
                    self.ch3_enabled = false;
                }
            }
            Channel::Ch4 => {
                if !self.ch4_length_enabled || self.ch4_length_timer == 0 {
                    return;
                }
                self.ch4_length_timer = self.ch4_length_timer.saturating_sub(1);
                if self.ch4_length_timer == 0 {
                    self.ch4_enabled = false;
                }
            }
        }
    }

    #[inline(always)]
    fn tick_envelope_all(&mut self) {
        self.tick_envelope(Channel::Ch1);
        self.tick_envelope(Channel::Ch2);
        self.tick_envelope(Channel::Ch4);
    }

    #[inline(always)]
    fn tick_envelope(&mut self, channel: Channel) {
        match channel {
            Channel::Ch1 => {
                if !self.ch1_enabled || !self.ch1_envelope_enabled {
                    return;
                }
                self.ch1_envelope_sequence += 1;
                if self.ch1_envelope_sequence >= self.ch1_pace {
                    if self.ch1_direction == 0x01 {
                        self.ch1_volume = self.ch1_volume.saturating_add(1);
                    } else {
                        self.ch1_volume = self.ch1_volume.saturating_sub(1);
                    }
                    if self.ch1_volume == 0 || self.ch1_volume == 15 {
                        self.ch1_envelope_enabled = false;
                    }
                    self.ch1_envelope_sequence = 0;
                }
            }
            Channel::Ch2 => {
                if !self.ch2_enabled || !self.ch2_envelope_enabled {
                    return;
                }
                self.ch2_envelope_sequence += 1;
                if self.ch2_envelope_sequence >= self.ch2_pace {
                    if self.ch2_direction == 0x01 {
                        self.ch2_volume = self.ch2_volume.saturating_add(1);
                    } else {
                        self.ch2_volume = self.ch2_volume.saturating_sub(1);
                    }
                    if self.ch2_volume == 0 || self.ch2_volume == 15 {
                        self.ch2_envelope_enabled = false;
                    }
                    self.ch2_envelope_sequence = 0;
                }
            }
            Channel::Ch3 => (),
            Channel::Ch4 => {
                if !self.ch4_enabled || !self.ch4_envelope_enabled {
                    return;
                }
                self.ch4_envelope_sequence += 1;
                if self.ch4_envelope_sequence >= self.ch4_pace {
                    if self.ch4_direction == 0x01 {
                        self.ch4_volume = self.ch4_volume.saturating_add(1);
                    } else {
                        self.ch4_volume = self.ch4_volume.saturating_sub(1);
                    }
                    if self.ch4_volume == 0 || self.ch4_volume == 15 {
                        self.ch4_envelope_enabled = false;
                    }
                    self.ch4_envelope_sequence = 0;
                }
            }
        }
    }

    #[inline(always)]
    fn tick_ch1_sweep(&mut self) {
        if self.ch1_sweep_pace == 0x0 {
            return;
        }
        self.ch1_sweep_sequence += 1;
        if self.ch1_sweep_sequence >= self.ch1_sweep_pace {
            let divisor = 1u16 << self.ch1_sweep_slope as u16;
            let delta = (self.ch1_wave_length as f32 / divisor as f32) as u16;
            if self.ch1_sweep_increase {
                self.ch1_wave_length = self.ch1_wave_length.saturating_add(delta);
            } else {
                self.ch1_wave_length = self.ch1_wave_length.saturating_sub(delta);
            }
            if self.ch1_wave_length > 0x07ff {
                self.ch1_enabled = false;
                self.ch1_wave_length = 0x07ff;
            }
            self.ch1_sweep_sequence = 0;
        }
    }

    #[inline(always)]
    fn tick_ch_all(&mut self, cycles: u16) {
        self.tick_ch1(cycles);
        self.tick_ch2(cycles);
        self.tick_ch3(cycles);
        self.tick_ch4(cycles);
    }

    #[inline(always)]
    fn tick_ch1(&mut self, cycles: u16) {
        self.ch1_timer = self.ch1_timer.saturating_sub(cycles as i16);
        if self.ch1_timer > 0 {
            return;
        }

        if self.ch1_enabled {
            self.ch1_output =
                if DUTY_TABLE[self.ch1_wave_duty as usize][self.ch1_sequence as usize] == 1 {
                    self.ch1_volume
                } else {
                    0
                };
        } else {
            self.ch1_output = 0;
        }

        self.ch1_timer += ((2048 - self.ch1_wave_length) << 2) as i16;
        self.ch1_sequence = (self.ch1_sequence + 1) & 7;
    }

    #[inline(always)]
    fn tick_ch2(&mut self, cycles: u16) {
        self.ch2_timer = self.ch2_timer.saturating_sub(cycles as i16);
        if self.ch2_timer > 0 {
            return;
        }

        if self.ch2_enabled {
            self.ch2_output =
                if DUTY_TABLE[self.ch2_wave_duty as usize][self.ch2_sequence as usize] == 1 {
                    self.ch2_volume
                } else {
                    0
                };
        } else {
            self.ch2_output = 0;
        }

        self.ch2_timer += ((2048 - self.ch2_wave_length) << 2) as i16;
        self.ch2_sequence = (self.ch2_sequence + 1) & 7;
    }

    #[inline(always)]
    fn tick_ch3(&mut self, cycles: u16) {
        self.ch3_timer = self.ch3_timer.saturating_sub(cycles as i16);
        if self.ch3_timer > 0 {
            return;
        }

        if self.ch3_enabled && self.ch3_dac {
            let wave_index = self.ch3_position >> 1;
            let mut output = self.wave_ram[wave_index as usize];
            output = if (self.ch3_position & 0x01) == 0x01 {
                output & 0x0f
            } else {
                (output & 0xf0) >> 4
            };
            if self.ch3_output_level > 0 {
                output >>= self.ch3_output_level - 1;
            } else {
                output = 0;
            }
            self.ch3_output = output;
        } else {
            self.ch3_output = 0;
        }

        self.ch3_timer += ((2048 - self.ch3_wave_length) << 1) as i16;
        self.ch3_position = (self.ch3_position + 1) & 31;
    }

    #[inline(always)]
    fn tick_ch4(&mut self, cycles: u16) {
        self.ch4_timer = self.ch4_timer.saturating_sub(cycles as i32);
        if self.ch4_timer > 0 {
            return;
        }

        if self.ch4_enabled {
            // obtains the current value of the LFSR based as
            // the XOR of the 1st and 2nd bit of the LFSR
            let result = ((self.ch4_lfsr & 0x0001) ^ ((self.ch4_lfsr >> 1) & 0x0001)) == 0x0001;

            // shifts the LFSR to the right and in case the
            // value is positive sets the 15th bit to 1
            self.ch4_lfsr >>= 1;
            self.ch4_lfsr |= if result { 0x0001 << 14 } else { 0x0 };

            // in case the short width mode (7 bits) is set then
            // the 6th bit will be set to value of the 15th bit
            if self.ch4_width_mode {
                self.ch4_lfsr &= 0xbf;
                self.ch4_lfsr |= if result { 0x40 } else { 0x00 };
            }

            self.ch4_output = if result { self.ch4_volume } else { 0 };
        } else {
            self.ch4_output = 0;
        }

        self.ch4_timer +=
            ((CH4_DIVISORS[self.ch4_divisor as usize] as u16) << self.ch4_clock_shift) as i32;
    }

    #[inline(always)]
    fn trigger_ch1(&mut self) {
        self.ch1_timer = ((2048 - self.ch1_wave_length) << 2) as i16;
        self.ch1_envelope_sequence = 0;
        self.ch1_sweep_sequence = 0;

        if self.ch1_length_timer == 0 {
            self.ch1_length_timer = 64;
            if self.ch1_length_enabled && self.sequencer_step % 2 == 1 {
                self.tick_length(Channel::Ch1);
            }
        }
    }

    #[inline(always)]
    fn trigger_ch2(&mut self) {
        self.ch2_timer = ((2048 - self.ch2_wave_length) << 2) as i16;
        self.ch2_envelope_sequence = 0;

        if self.ch2_length_timer == 0 {
            self.ch2_length_timer = 64;
            if self.ch2_length_enabled && self.sequencer_step % 2 == 1 {
                self.tick_length(Channel::Ch2);
            }
        }
    }

    #[inline(always)]
    fn trigger_ch3(&mut self) {
        self.ch3_timer = 3;
        self.ch3_position = 0;

        if self.ch3_length_timer == 0 {
            self.ch3_length_timer = 256;
            if self.ch3_length_enabled && self.sequencer_step % 2 == 1 {
                self.tick_length(Channel::Ch3);
            }
        }
    }

    #[inline(always)]
    fn trigger_ch4(&mut self) {
        self.ch4_timer =
            ((CH4_DIVISORS[self.ch4_divisor as usize] as u16) << self.ch4_clock_shift) as i32;
        self.ch4_lfsr = 0x7ff1;
        self.ch4_envelope_sequence = 0;

        if self.ch4_length_timer == 0 {
            self.ch4_length_timer = 64;
            if self.ch4_length_enabled && self.sequencer_step % 2 == 1 {
                self.tick_length(Channel::Ch4);
            }
        }
    }
}

impl BusComponent for Apu {
    fn read(&self, addr: u16) -> u8 {
        self.read(addr)
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.write(addr, value);
    }
}

impl StateComponent for Apu {
    fn state(&self, _format: Option<StateFormat>) -> Result<Vec<u8>, Error> {
        let mut cursor = Cursor::new(vec![]);

        write_i16(&mut cursor, self.ch1_timer)?;
        write_u8(&mut cursor, self.ch1_sequence)?;
        write_u8(&mut cursor, self.ch1_envelope_sequence)?;
        write_u8(&mut cursor, self.ch1_envelope_enabled as u8)?;
        write_u8(&mut cursor, self.ch1_sweep_sequence)?;
        write_u8(&mut cursor, self.ch1_output)?;
        write_u8(&mut cursor, self.ch1_dac as u8)?;
        write_u8(&mut cursor, self.ch1_sweep_slope)?;
        write_u8(&mut cursor, self.ch1_sweep_increase as u8)?;
        write_u8(&mut cursor, self.ch1_sweep_pace)?;
        write_u8(&mut cursor, self.ch1_length_timer)?;
        write_u8(&mut cursor, self.ch1_wave_duty)?;
        write_u8(&mut cursor, self.ch1_pace)?;
        write_u8(&mut cursor, self.ch1_direction)?;
        write_u8(&mut cursor, self.ch1_volume)?;
        write_u16(&mut cursor, self.ch1_wave_length)?;
        write_u8(&mut cursor, self.ch1_length_enabled as u8)?;
        write_u8(&mut cursor, self.ch1_enabled as u8)?;

        write_i16(&mut cursor, self.ch2_timer)?;
        write_u8(&mut cursor, self.ch2_sequence)?;
        write_u8(&mut cursor, self.ch2_envelope_sequence)?;
        write_u8(&mut cursor, self.ch2_envelope_enabled as u8)?;
        write_u8(&mut cursor, self.ch2_output)?;
        write_u8(&mut cursor, self.ch2_dac as u8)?;
        write_u8(&mut cursor, self.ch2_length_timer)?;
        write_u8(&mut cursor, self.ch2_wave_duty)?;
        write_u8(&mut cursor, self.ch2_pace)?;
        write_u8(&mut cursor, self.ch2_direction)?;
        write_u8(&mut cursor, self.ch2_volume)?;
        write_u16(&mut cursor, self.ch2_wave_length)?;
        write_u8(&mut cursor, self.ch2_length_enabled as u8)?;
        write_u8(&mut cursor, self.ch2_enabled as u8)?;

        write_i16(&mut cursor, self.ch3_timer)?;
        write_u8(&mut cursor, self.ch3_position)?;
        write_u8(&mut cursor, self.ch3_output)?;
        write_u8(&mut cursor, self.ch3_dac as u8)?;
        write_u16(&mut cursor, self.ch3_length_timer)?;
        write_u8(&mut cursor, self.ch3_output_level)?;
        write_u16(&mut cursor, self.ch3_wave_length)?;
        write_u8(&mut cursor, self.ch3_length_enabled as u8)?;
        write_u8(&mut cursor, self.ch3_enabled as u8)?;

        write_i32(&mut cursor, self.ch4_timer)?;
        write_u8(&mut cursor, self.ch4_envelope_sequence)?;
        write_u8(&mut cursor, self.ch4_envelope_enabled as u8)?;
        write_u8(&mut cursor, self.ch4_output)?;
        write_u8(&mut cursor, self.ch4_dac as u8)?;
        write_u8(&mut cursor, self.ch4_length_timer)?;
        write_u8(&mut cursor, self.ch4_pace)?;
        write_u8(&mut cursor, self.ch4_direction)?;
        write_u8(&mut cursor, self.ch4_volume)?;
        write_u8(&mut cursor, self.ch4_divisor)?;
        write_u8(&mut cursor, self.ch4_width_mode as u8)?;
        write_u8(&mut cursor, self.ch4_clock_shift)?;
        write_u16(&mut cursor, self.ch4_lfsr)?;
        write_u8(&mut cursor, self.ch4_length_enabled as u8)?;
        write_u8(&mut cursor, self.ch4_enabled as u8)?;

        write_u8(&mut cursor, self.master)?;
        write_u8(&mut cursor, self.glob_panning)?;

        write_u8(&mut cursor, self.right_enabled as u8)?;
        write_u8(&mut cursor, self.left_enabled as u8)?;
        write_u8(&mut cursor, self.sound_enabled as u8)?;

        write_u8(&mut cursor, self.ch1_out_enabled as u8)?;
        write_u8(&mut cursor, self.ch2_out_enabled as u8)?;
        write_u8(&mut cursor, self.ch3_out_enabled as u8)?;
        write_u8(&mut cursor, self.ch4_out_enabled as u8)?;

        write_bytes(&mut cursor, &self.wave_ram)?;

        write_u16(&mut cursor, self.sampling_rate)?;
        write_u8(&mut cursor, self.channels)?;

        write_u16(&mut cursor, self.sequencer)?;
        write_u8(&mut cursor, self.sequencer_step)?;
        write_i16(&mut cursor, self.output_timer)?;
        write_u8(&mut cursor, self.filter_mode.into())?;
        write_f32(&mut cursor, self.filter_diff[0])?;
        write_f32(&mut cursor, self.filter_diff[1])?;

        Ok(cursor.into_inner())
    }

    fn set_state(&mut self, data: &[u8], _format: Option<StateFormat>) -> Result<(), Error> {
        let mut cursor = Cursor::new(data);

        self.ch1_timer = read_i16(&mut cursor)?;
        self.ch1_sequence = read_u8(&mut cursor)?;
        self.ch1_envelope_sequence = read_u8(&mut cursor)?;
        self.ch1_envelope_enabled = read_u8(&mut cursor)? != 0;
        self.ch1_sweep_sequence = read_u8(&mut cursor)?;
        self.ch1_output = read_u8(&mut cursor)?;
        self.ch1_dac = read_u8(&mut cursor)? != 0;
        self.ch1_sweep_slope = read_u8(&mut cursor)?;
        self.ch1_sweep_increase = read_u8(&mut cursor)? != 0;
        self.ch1_sweep_pace = read_u8(&mut cursor)?;
        self.ch1_length_timer = read_u8(&mut cursor)?;
        self.ch1_wave_duty = read_u8(&mut cursor)?;
        self.ch1_pace = read_u8(&mut cursor)?;
        self.ch1_direction = read_u8(&mut cursor)?;
        self.ch1_volume = read_u8(&mut cursor)?;
        self.ch1_wave_length = read_u16(&mut cursor)?;
        self.ch1_length_enabled = read_u8(&mut cursor)? != 0;
        self.ch1_enabled = read_u8(&mut cursor)? != 0;

        self.ch2_timer = read_i16(&mut cursor)?;
        self.ch2_sequence = read_u8(&mut cursor)?;
        self.ch2_envelope_sequence = read_u8(&mut cursor)?;
        self.ch2_envelope_enabled = read_u8(&mut cursor)? != 0;
        self.ch2_output = read_u8(&mut cursor)?;
        self.ch2_dac = read_u8(&mut cursor)? != 0;
        self.ch2_length_timer = read_u8(&mut cursor)?;
        self.ch2_wave_duty = read_u8(&mut cursor)?;
        self.ch2_pace = read_u8(&mut cursor)?;
        self.ch2_direction = read_u8(&mut cursor)?;
        self.ch2_volume = read_u8(&mut cursor)?;
        self.ch2_wave_length = read_u16(&mut cursor)?;
        self.ch2_length_enabled = read_u8(&mut cursor)? != 0;
        self.ch2_enabled = read_u8(&mut cursor)? != 0;

        self.ch3_timer = read_i16(&mut cursor)?;
        self.ch3_position = read_u8(&mut cursor)?;
        self.ch3_output = read_u8(&mut cursor)?;
        self.ch3_dac = read_u8(&mut cursor)? != 0;
        self.ch3_length_timer = read_u16(&mut cursor)?;
        self.ch3_output_level = read_u8(&mut cursor)?;
        self.ch3_wave_length = read_u16(&mut cursor)?;
        self.ch3_length_enabled = read_u8(&mut cursor)? != 0;
        self.ch3_enabled = read_u8(&mut cursor)? != 0;

        self.ch4_timer = read_i32(&mut cursor)?;
        self.ch4_envelope_sequence = read_u8(&mut cursor)?;
        self.ch4_envelope_enabled = read_u8(&mut cursor)? != 0;
        self.ch4_output = read_u8(&mut cursor)?;
        self.ch4_dac = read_u8(&mut cursor)? != 0;
        self.ch4_length_timer = read_u8(&mut cursor)?;
        self.ch4_pace = read_u8(&mut cursor)?;
        self.ch4_direction = read_u8(&mut cursor)?;
        self.ch4_volume = read_u8(&mut cursor)?;
        self.ch4_divisor = read_u8(&mut cursor)?;
        self.ch4_width_mode = read_u8(&mut cursor)? != 0;
        self.ch4_clock_shift = read_u8(&mut cursor)?;
        self.ch4_lfsr = read_u16(&mut cursor)?;
        self.ch4_length_enabled = read_u8(&mut cursor)? != 0;
        self.ch4_enabled = read_u8(&mut cursor)? != 0;

        self.master = read_u8(&mut cursor)?;
        self.glob_panning = read_u8(&mut cursor)?;

        self.right_enabled = read_u8(&mut cursor)? != 0;
        self.left_enabled = read_u8(&mut cursor)? != 0;
        self.sound_enabled = read_u8(&mut cursor)? != 0;

        self.ch1_out_enabled = read_u8(&mut cursor)? != 0;
        self.ch2_out_enabled = read_u8(&mut cursor)? != 0;
        self.ch3_out_enabled = read_u8(&mut cursor)? != 0;
        self.ch4_out_enabled = read_u8(&mut cursor)? != 0;

        read_into(&mut cursor, &mut self.wave_ram)?;

        self.sampling_rate = read_u16(&mut cursor)?;
        self.channels = read_u8(&mut cursor)?;

        self.sequencer = read_u16(&mut cursor)?;
        self.sequencer_step = read_u8(&mut cursor)?;
        self.output_timer = read_i16(&mut cursor)?;
        self.output_timer_delta = (self.clock_freq as f32 / self.sampling_rate as f32) as i16;
        self.filter_mode = read_u8(&mut cursor)?.into();
        self.filter_diff[0] = read_f32(&mut cursor)?;
        self.filter_diff[1] = read_f32(&mut cursor)?;
        self.filter_rate =
            FILTER_RATE_BASE.powf(self.clock_freq as f64 / self.sampling_rate as f64) as f32;

        Ok(())
    }
}

impl Default for Apu {
    fn default() -> Self {
        Self::new(44100, 2, 1.0, GameBoy::CPU_FREQ)
    }
}

#[cfg(test)]
mod tests {
    use super::{Apu, HighPassFilter};

    use crate::state::StateComponent;

    #[test]
    fn test_trigger_ch1() {
        let mut apu = Apu {
            ch1_wave_length: 1024,
            ..Default::default()
        };
        apu.trigger_ch1();

        assert_eq!(apu.ch1_timer, 4096);
        assert_eq!(apu.ch1_envelope_sequence, 0);
        assert_eq!(apu.ch1_sweep_sequence, 0);
    }

    #[test]
    fn test_trigger_ch2() {
        let mut apu = Apu {
            ch2_wave_length: 1024,
            ..Default::default()
        };
        apu.trigger_ch2();

        assert_eq!(apu.ch2_timer, 4096);
        assert_eq!(apu.ch2_envelope_sequence, 0);
    }

    #[test]
    fn test_trigger_ch3() {
        let mut apu = Apu {
            ch3_wave_length: 1024,
            ..Default::default()
        };
        apu.trigger_ch3();

        assert_eq!(apu.ch3_timer, 3);
        assert_eq!(apu.ch3_position, 0);
    }

    #[test]
    fn test_trigger_ch4() {
        let mut apu = Apu {
            ch4_divisor: 3,
            ch4_clock_shift: 2,
            ..Default::default()
        };
        apu.trigger_ch4();

        assert_eq!(apu.ch4_timer, 192);
        assert_eq!(apu.ch4_lfsr, 0x7ff1);
        assert_eq!(apu.ch4_envelope_sequence, 0);
    }

    #[test]
    fn test_state_and_set_state() {
        let apu = Apu {
            ch1_timer: 1234,
            ch1_sequence: 5,
            ch1_envelope_sequence: 3,
            ch1_envelope_enabled: true,
            ch1_sweep_sequence: 2,
            ch1_output: 10,
            ch1_dac: true,
            ch1_sweep_slope: 1,
            ch1_sweep_increase: true,
            ch1_sweep_pace: 4,
            ch1_length_timer: 20,
            ch1_wave_duty: 2,
            ch1_pace: 3,
            ch1_direction: 1,
            ch1_volume: 15,
            ch1_wave_length: 2048,
            ch1_length_enabled: true,
            ch1_enabled: true,
            ch2_timer: 5678,
            ch2_sequence: 6,
            ch2_envelope_sequence: 4,
            ch2_envelope_enabled: true,
            ch2_output: 20,
            ch2_dac: true,
            ch2_length_timer: 30,
            ch2_wave_duty: 3,
            ch2_pace: 5,
            ch2_direction: 0,
            ch2_volume: 10,
            ch2_wave_length: 1024,
            ch2_length_enabled: true,
            ch2_enabled: true,
            ch3_timer: 9111,
            ch3_position: 7,
            ch3_output: 30,
            ch3_dac: true,
            ch3_length_timer: 40,
            ch3_output_level: 2,
            ch3_wave_length: 512,
            ch3_length_enabled: true,
            ch3_enabled: true,
            ch4_timer: 121314,
            ch4_envelope_sequence: 5,
            ch4_envelope_enabled: true,
            ch4_output: 40,
            ch4_dac: true,
            ch4_length_timer: 50,
            ch4_pace: 6,
            ch4_direction: 1,
            ch4_volume: 5,
            ch4_divisor: 2,
            ch4_width_mode: true,
            ch4_clock_shift: 3,
            ch4_lfsr: 0x7ff1,
            ch4_length_enabled: true,
            ch4_enabled: true,
            master: 0x77,
            glob_panning: 0x88,
            right_enabled: true,
            left_enabled: true,
            sound_enabled: true,
            ch1_out_enabled: true,
            ch2_out_enabled: true,
            ch3_out_enabled: true,
            ch4_out_enabled: true,
            wave_ram: [0x12; 16],
            sampling_rate: 44100,
            channels: 2,
            sequencer: 12345,
            sequencer_step: 6,
            output_timer: 789,
            ..Default::default()
        };

        let state = apu.state(None).unwrap();
        assert_eq!(state.len(), 109);

        let mut new_apu = Apu::default();
        new_apu.set_state(&state, None).unwrap();

        assert_eq!(new_apu.ch1_timer, 1234);
        assert_eq!(new_apu.ch1_sequence, 5);
        assert_eq!(new_apu.ch1_envelope_sequence, 3);
        assert!(new_apu.ch1_envelope_enabled);
        assert_eq!(new_apu.ch1_sweep_sequence, 2);
        assert_eq!(new_apu.ch1_output, 10);
        assert!(new_apu.ch1_dac);
        assert_eq!(new_apu.ch1_sweep_slope, 1);
        assert!(new_apu.ch1_sweep_increase);
        assert_eq!(new_apu.ch1_sweep_pace, 4);
        assert_eq!(new_apu.ch1_length_timer, 20);
        assert_eq!(new_apu.ch1_wave_duty, 2);
        assert_eq!(new_apu.ch1_pace, 3);
        assert_eq!(new_apu.ch1_direction, 1);
        assert_eq!(new_apu.ch1_volume, 15);
        assert_eq!(new_apu.ch1_wave_length, 2048);
        assert!(new_apu.ch1_length_enabled);
        assert!(new_apu.ch1_enabled);

        assert_eq!(new_apu.ch2_timer, 5678);
        assert_eq!(new_apu.ch2_sequence, 6);
        assert_eq!(new_apu.ch2_envelope_sequence, 4);
        assert!(new_apu.ch2_envelope_enabled);
        assert_eq!(new_apu.ch2_output, 20);
        assert!(new_apu.ch2_dac);
        assert_eq!(new_apu.ch2_length_timer, 30);
        assert_eq!(new_apu.ch2_wave_duty, 3);
        assert_eq!(new_apu.ch2_pace, 5);
        assert_eq!(new_apu.ch2_direction, 0);
        assert_eq!(new_apu.ch2_volume, 10);
        assert_eq!(new_apu.ch2_wave_length, 1024);
        assert!(new_apu.ch2_length_enabled);
        assert!(new_apu.ch2_enabled);

        assert_eq!(new_apu.ch3_timer, 9111);
        assert_eq!(new_apu.ch3_position, 7);
        assert_eq!(new_apu.ch3_output, 30);
        assert!(new_apu.ch3_dac);
        assert_eq!(new_apu.ch3_length_timer, 40);
        assert_eq!(new_apu.ch3_output_level, 2);
        assert_eq!(new_apu.ch3_wave_length, 512);
        assert!(new_apu.ch3_length_enabled);
        assert!(new_apu.ch3_enabled);

        assert_eq!(new_apu.ch4_timer, 121314);
        assert_eq!(new_apu.ch4_envelope_sequence, 5);
        assert!(new_apu.ch4_envelope_enabled);
        assert_eq!(new_apu.ch4_output, 40);
        assert!(new_apu.ch4_dac);
        assert_eq!(new_apu.ch4_length_timer, 50);
        assert_eq!(new_apu.ch4_pace, 6);
        assert_eq!(new_apu.ch4_direction, 1);
        assert_eq!(new_apu.ch4_volume, 5);
        assert_eq!(new_apu.ch4_divisor, 2);
        assert!(new_apu.ch4_width_mode);
        assert_eq!(new_apu.ch4_clock_shift, 3);
        assert_eq!(new_apu.ch4_lfsr, 0x7ff1);
        assert!(new_apu.ch4_length_enabled);
        assert!(new_apu.ch4_enabled);

        assert_eq!(new_apu.master, 0x77);
        assert_eq!(new_apu.glob_panning, 0x88);

        assert!(new_apu.right_enabled);
        assert!(new_apu.left_enabled);
        assert!(new_apu.sound_enabled);

        assert!(new_apu.ch1_out_enabled);
        assert!(new_apu.ch2_out_enabled);
        assert!(new_apu.ch3_out_enabled);
        assert!(new_apu.ch4_out_enabled);

        assert_eq!(new_apu.wave_ram, [0x12; 16]);

        assert_eq!(new_apu.sampling_rate, 44100);
        assert_eq!(new_apu.channels, 2);

        assert_eq!(new_apu.sequencer, 12345);
        assert_eq!(new_apu.sequencer_step, 6);
        assert_eq!(new_apu.output_timer, 789);
        assert_eq!(new_apu.filter_mode as u8, 1);
    }

    #[test]
    fn test_filter_sample_disable() {
        let mut apu = Apu::default();
        let sample = 128;
        let value = apu.filter_sample(sample, 0);
        assert_eq!(value, sample as i16);
    }

    #[test]
    fn test_filter_sample_accurate() {
        let mut apu = Apu::default();
        apu.set_filter_mode(HighPassFilter::Accurate);
        let sample = 128;
        let first = apu.filter_sample(sample, 0);
        let second = apu.filter_sample(sample, 0);
        assert_eq!(first, 128);
        assert_eq!(second, 127);
    }

    #[test]
    fn test_filter_sample_preserve() {
        let mut apu = Apu::default();
        apu.set_filter_mode(HighPassFilter::Preserve);
        let sample = 128;
        let first = apu.filter_sample(sample, 0);
        let second = apu.filter_sample(sample, 0);
        assert_eq!(first, 128);
        assert_eq!(second, 127);
    }

    #[test]
    fn test_set_filter_mode_resets_diff() {
        let mut apu = Apu::default();
        apu.set_filter_mode(HighPassFilter::Preserve);
        let _ = apu.filter_sample(100, 0);
        apu.set_filter_mode(HighPassFilter::Accurate);
        let value = apu.filter_sample(100, 0);
        assert_eq!(value, 100);
    }

    #[test]
    fn test_state_preserves_filter() {
        let mut apu = Apu::default();
        apu.set_filter_mode(HighPassFilter::Accurate);
        apu.filter_diff = [2.5, 3.5];

        let state = apu.state(None).unwrap();
        let mut other = Apu::default();
        other.set_state(&state, None).unwrap();

        assert_eq!(other.filter_mode, HighPassFilter::Accurate);
        assert!((other.filter_diff[0] - 2.5).abs() < f32::EPSILON);
        assert!((other.filter_diff[1] - 3.5).abs() < f32::EPSILON);
    }
}
