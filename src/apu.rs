use std::collections::VecDeque;

use crate::warnln;

const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];

pub enum Channel {
    Ch1,
    Ch2,
    Ch3,
    Ch4,
}

pub struct Apu {
    ch1_timer: i16,
    ch1_sequence: u8,
    ch1_envelope_sequence: u8,
    ch1_envelope_enabled: bool,
    ch1_sweep_sequence: u8,
    ch1_output: u8,
    ch1_sweep_slope: u8,
    ch1_sweep_increase: bool,
    ch1_sweep_pace: u8,
    ch1_length_timer: u8,
    ch1_wave_duty: u8,
    ch1_pace: u8,
    ch1_direction: u8,
    ch1_volume: u8,
    ch1_wave_length: u16,
    ch1_length_stop: bool,
    ch1_enabled: bool,

    ch2_timer: i16,
    ch2_sequence: u8,
    ch2_envelope_sequence: u8,
    ch2_envelope_enabled: bool,
    ch2_output: u8,
    ch2_length_timer: u8,
    ch2_wave_duty: u8,
    ch2_pace: u8,
    ch2_direction: u8,
    ch2_volume: u8,
    ch2_wave_length: u16,
    ch2_length_stop: bool,
    ch2_enabled: bool,

    ch3_timer: i16,
    ch3_position: u8,
    ch3_output: u8,
    ch3_dac: bool,
    ch3_length_timer: u8,
    ch3_output_level: u8,
    ch3_wave_length: u16,
    ch3_length_stop: bool,
    ch3_enabled: bool,

    ch4_timer: i16,
    ch4_output: u8,
    ch4_length_timer: u8,
    ch4_pace: u8,
    ch4_direction: u8,
    ch4_volume: u8,
    ch4_output_level: u8,
    ch4_wave_length: u16,
    ch4_length_stop: bool,
    ch4_enabled: bool,

    right_enabled: bool,
    left_enabled: bool,

    wave_ram: [u8; 16],

    sampling_rate: u16,
    sequencer: u16,
    sequencer_step: u8,
    output_timer: i16,
    audio_buffer: VecDeque<u8>,
    audio_buffer_max: usize,
}

impl Apu {
    pub fn new(sampling_rate: u16, buffer_size: f32) -> Self {
        Self {
            ch1_timer: 0,
            ch1_sequence: 0,
            ch1_envelope_sequence: 0,
            ch1_envelope_enabled: false,
            ch1_sweep_sequence: 0,
            ch1_output: 0,
            ch1_sweep_slope: 0x0,
            ch1_sweep_increase: false,
            ch1_sweep_pace: 0x0,
            ch1_length_timer: 0x0,
            ch1_wave_duty: 0x0,
            ch1_pace: 0x0,
            ch1_direction: 0x0,
            ch1_volume: 0x0,
            ch1_wave_length: 0x0,
            ch1_length_stop: false,
            ch1_enabled: false,

            ch2_timer: 0,
            ch2_sequence: 0,
            ch2_envelope_sequence: 0,
            ch2_envelope_enabled: false,
            ch2_output: 0,
            ch2_length_timer: 0x0,
            ch2_wave_duty: 0x0,
            ch2_pace: 0x0,
            ch2_direction: 0x0,
            ch2_volume: 0x0,
            ch2_wave_length: 0x0,
            ch2_length_stop: false,
            ch2_enabled: false,

            ch3_timer: 0,
            ch3_position: 0,
            ch3_output: 0,
            ch3_dac: false,
            ch3_length_timer: 0x0,
            ch3_output_level: 0x0,
            ch3_wave_length: 0x0,
            ch3_length_stop: false,
            ch3_enabled: false,

            ch4_timer: 0,
            ch4_output: 0,
            ch4_length_timer: 0x0,
            ch4_pace: 0x0,
            ch4_direction: 0x0,
            ch4_volume: 0x0,
            ch4_output_level: 0x0,
            ch4_wave_length: 0x0,
            ch4_length_stop: false,
            ch4_enabled: false,

            left_enabled: true,
            right_enabled: true,

            /// The RAM that is used to sore the wave information
            /// to be used in channel 3 audio
            wave_ram: [0u8; 16],

            /// The rate at which audio samples are going to be
            /// taken, ideally this value should be aligned with
            /// the sampling rate of the output device. A typical
            /// sampling rate would be of 44.1kHz.
            sampling_rate,

            /// Internal sequencer counter that runs at 512Hz
            /// used for the activation of the tick actions.
            sequencer: 0,
            sequencer_step: 0,
            output_timer: 0,
            audio_buffer: VecDeque::with_capacity(
                (sampling_rate as f32 * buffer_size) as usize * 2,
            ),
            audio_buffer_max: (sampling_rate as f32 * buffer_size) as usize * 2,
        }
    }

    pub fn reset(&mut self) {
        self.ch1_timer = 0;
        self.ch1_sequence = 0;
        self.ch1_envelope_sequence = 0;
        self.ch1_envelope_enabled = false;
        self.ch1_sweep_sequence = 0;
        self.ch1_output = 0;
        self.ch1_sweep_slope = 0x0;
        self.ch1_sweep_increase = false;
        self.ch1_sweep_pace = 0x0;
        self.ch1_length_timer = 0x0;
        self.ch1_wave_duty = 0x0;
        self.ch1_pace = 0x0;
        self.ch1_direction = 0x0;
        self.ch1_volume = 0x0;
        self.ch1_wave_length = 0x0;
        self.ch1_length_stop = false;
        self.ch1_enabled = false;

        self.ch2_timer = 0;
        self.ch2_sequence = 0;
        self.ch2_envelope_sequence = 0;
        self.ch2_envelope_enabled = false;
        self.ch2_output = 0;
        self.ch2_length_timer = 0x0;
        self.ch2_wave_duty = 0x0;
        self.ch2_pace = 0x0;
        self.ch2_direction = 0x0;
        self.ch2_volume = 0x0;
        self.ch2_wave_length = 0x0;
        self.ch2_length_stop = false;
        self.ch2_enabled = false;

        self.ch3_timer = 0;
        self.ch3_position = 0;
        self.ch3_output = 0;
        self.ch3_dac = false;
        self.ch3_length_timer = 0x0;
        self.ch3_output_level = 0x0;
        self.ch3_wave_length = 0x0;
        self.ch3_length_stop = false;
        self.ch3_enabled = false;

        self.ch4_timer = 0;
        self.ch4_output = 0;
        self.ch4_length_timer = 0x0;
        self.ch4_output_level = 0x0;
        self.ch4_wave_length = 0x0;
        self.ch4_length_stop = false;
        self.ch4_enabled = false;

        self.left_enabled = true;
        self.right_enabled = true;

        self.sequencer = 0;
        self.sequencer_step = 0;
        self.output_timer = 0;

        self.clear_audio_buffer()
    }

    pub fn clock(&mut self, cycles: u8) {
        self.sequencer += cycles as u16;
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
            // the buffer (avoiding overflow) and then then the new audio
            // volume item is added to the queue
            if self.audio_buffer.len() >= self.audio_buffer_max {
                self.audio_buffer.pop_front();
                self.audio_buffer.pop_front();
            }
            if self.left_enabled {
                self.audio_buffer.push_back(self.output());
            }
            if self.right_enabled {
                self.audio_buffer.push_back(self.output());
            }

            // @TODO the CPU clock is hardcoded here, we must handle situations
            // where there's some kind of overclock
            self.output_timer += (4194304.0 / self.sampling_rate as f32) as i16;
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        {
            warnln!("Reading from unknown APU location 0x{:04x}", addr);
            0xff
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // 0xFF10 — NR10: Channel 1 sweep
            0xff10 => {
                self.ch1_sweep_slope = value & 0x07;
                self.ch1_sweep_increase = value & 0x08 == 0x00;
                self.ch1_sweep_pace = (value & 0x70) >> 4;
                self.ch1_sweep_sequence = 0;
            }
            // 0xFF11 — NR11: Channel 1 length timer & duty cycle
            0xff11 => {
                self.ch1_length_timer = value & 0x3f;
                self.ch1_wave_duty = (value & 0xc0) >> 6;
            }
            // 0xFF12 — NR12: Channel 1 volume & envelope
            0xff12 => {
                self.ch1_pace = value & 0x07;
                self.ch1_direction = (value & 0x08) >> 3;
                self.ch1_volume = (value & 0xf0) >> 4;
                self.ch1_envelope_enabled = self.ch1_pace > 0;
                self.ch1_envelope_sequence = 0;
            }
            // 0xFF13 — NR13: Channel 1 wavelength low
            0xff13 => {
                self.ch1_wave_length = (self.ch1_wave_length & 0xff00) | value as u16;
            }
            // 0xFF14 — NR14: Channel 1 wavelength high & control
            0xff14 => {
                self.ch1_wave_length =
                    (self.ch1_wave_length & 0x00ff) | (((value & 0x07) as u16) << 8);
                self.ch1_length_stop |= value & 0x40 == 0x40;
                self.ch1_enabled |= value & 0x80 == 0x80;
            }

            // 0xFF16 — NR21: Channel 2 length timer & duty cycle
            0xff16 => {
                self.ch2_length_timer = value & 0x3f;
                self.ch2_wave_duty = (value & 0xc0) >> 6;
            }
            // 0xFF17 — NR22: Channel 2 volume & envelope
            0xff17 => {
                self.ch2_pace = value & 0x07;
                self.ch2_direction = (value & 0x08) >> 3;
                self.ch2_volume = (value & 0xf0) >> 4;
            }
            // 0xFF18 — NR23: Channel 2 wavelength low
            0xff18 => {
                self.ch2_wave_length = (self.ch2_wave_length & 0xff00) | value as u16;
            }
            // 0xFF19 — NR24: Channel 2 wavelength high & control
            0xff19 => {
                self.ch2_wave_length =
                    (self.ch2_wave_length & 0x00ff) | (((value & 0x07) as u16) << 8);
                self.ch2_length_stop |= value & 0x40 == 0x40;
                self.ch2_enabled |= value & 0x80 == 0x80;
            }

            // 0xFF1A — NR30: Channel 3 DAC enable
            0xff1a => {
                self.ch3_dac = value & 0x80 == 0x80;
            }
            // 0xFF1B — NR31: Channel 3 length timer
            0xff1b => {
                self.ch3_length_timer = value;
            }
            // 0xFF1C — NR32: Channel 3 output level
            0xff1c => {
                self.ch3_output_level = (value & 0x60) >> 5;
            }
            // 0xFF1D — NR33: Channel 3 wavelength low [write-only]
            0xff1d => {
                self.ch3_wave_length = (self.ch3_wave_length & 0xff00) | value as u16;
            }
            // 0xFF1E — NR34: Channel 3 wavelength high & control
            0xff1e => {
                self.ch3_wave_length =
                    (self.ch3_wave_length & 0x00ff) | (((value & 0x07) as u16) << 8);
                self.ch3_length_stop |= value & 0x40 == 0x40;
                self.ch3_enabled |= value & 0x80 == 0x80;
            }

            // 0xFF20 — NR41: Channel 4 length timer
            0xff20 => {
                self.ch4_length_timer = value & 0x3f;
            }
            // 0xFF21 — NR42: Channel 4 volume & envelope
            0xff21 => {
                self.ch4_pace = value & 0x07;
                self.ch4_direction = (value & 0x08) >> 3;
                self.ch4_volume = (value & 0xf0) >> 4;
            }
            // 0xFF22 — NR43: Channel 4 frequency & randomness
            0xff22 => {
                //@TODO need to implement this one!
            }
            // 0xFF23 — NR44: Channel 4 control
            0xff23 => {
                self.ch4_length_stop |= value & 0x40 == 0x40;
                self.ch4_enabled |= value & 0x80 == 0x80;
            }

            // 0xFF30-0xFF3F — Wave pattern RAM
            0xff30..=0xff3f => {
                self.wave_ram[addr as usize & 0x000f] = value;
            }

            _ => warnln!("Writing in unknown APU location 0x{:04x}", addr),
        }
    }

    pub fn output(&self) -> u8 {
        self.ch1_output + self.ch2_output + self.ch3_output + self.ch4_output
    }

    pub fn audio_buffer(&self) -> &VecDeque<u8> {
        &self.audio_buffer
    }

    pub fn audio_buffer_mut(&mut self) -> &mut VecDeque<u8> {
        &mut self.audio_buffer
    }

    pub fn clear_audio_buffer(&mut self) {
        self.audio_buffer.clear();
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
                if !self.ch1_enabled {
                    return;
                }
                self.ch1_length_timer = self.ch1_length_timer.saturating_add(1);
                if self.ch1_length_timer >= 64 {
                    self.ch1_enabled = !self.ch1_length_stop;
                    self.ch1_length_timer = 0;
                }
            }
            Channel::Ch2 => {
                self.ch2_length_timer = self.ch2_length_timer.saturating_add(1);
                if self.ch2_length_timer >= 64 {
                    self.ch2_enabled = !self.ch2_length_stop;
                    self.ch2_length_timer = 0;
                }
            }
            Channel::Ch3 => {
                self.ch3_length_timer = self.ch3_length_timer.saturating_add(1);
                if self.ch3_length_timer >= 64 {
                    self.ch3_enabled = !self.ch3_length_stop;
                    self.ch3_length_timer = 0;
                }
            }
            Channel::Ch4 => (),
        }
    }

    #[inline(always)]
    fn tick_envelope_all(&mut self) {
        self.tick_envelope(Channel::Ch1);
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
            Channel::Ch4 => (),
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
    fn tick_ch_all(&mut self, cycles: u8) {
        self.tick_ch1(cycles);
        self.tick_ch2(cycles);
        self.tick_ch3(cycles);
    }

    #[inline(always)]
    fn tick_ch1(&mut self, cycles: u8) {
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
    fn tick_ch2(&mut self, cycles: u8) {
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
    fn tick_ch3(&mut self, cycles: u8) {
        self.ch3_timer = self.ch3_timer.saturating_sub(cycles as i16);
        if self.ch3_timer > 0 {
            return;
        }

        if self.ch3_enabled {
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
}

impl Default for Apu {
    fn default() -> Self {
        Self::new(44100, 1.0)
    }
}
