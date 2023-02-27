use crate::warnln;

const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];

pub struct Apu {
    ch1_timer: u16,
    ch1_sequence: u8,
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
    ch1_sound_length: bool,
    ch1_enabled: bool,

    ch2_timer: u16,
    ch2_sequence: u8,
    ch2_output: u8,
    ch2_length_timer: u8,
    ch2_wave_duty: u8,
    ch2_pace: u8,
    ch2_direction: u8,
    ch2_volume: u8,
    ch2_wave_length: u16,
    ch2_sound_length: bool,
    ch2_enabled: bool,

    ch3_timer: u16,
    ch3_sequence: u8,
    ch3_output: u8,
    ch3_dac: bool,
    ch3_length_timer: u8,
    ch3_output_level: u8,
    ch3_wave_length: u16,
    ch3_sound_length: bool,
    ch3_enabled: bool,

    wave_ram: [u8; 16],

    output_timer: u16,
    output_buffer: Vec<u8>
}

impl Apu {
    pub fn new() -> Self {
        Self {
            ch1_timer: 0,
            ch1_sequence: 0,
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
            ch1_sound_length: false,
            ch1_enabled: false,

            ch2_timer: 0,
            ch2_sequence: 0,
            ch2_output: 0,
            ch2_length_timer: 0x0,
            ch2_wave_duty: 0x0,
            ch2_pace: 0x0,
            ch2_direction: 0x0,
            ch2_volume: 0x0,
            ch2_wave_length: 0x0,
            ch2_sound_length: false,
            ch2_enabled: false,

            ch3_timer: 0,
            ch3_sequence: 0,
            ch3_output: 0,
            ch3_dac: false,
            ch3_length_timer: 0x0,
            ch3_output_level: 0x0,
            ch3_wave_length: 0x0,
            ch3_sound_length: false,
            ch3_enabled: false,

            wave_ram: [0u8; 16],

            output_timer: 0,
            output_buffer: Vec::new()
        }
    }

    pub fn clock(&mut self, cycles: u8) {
        self.clock_f(cycles, 4194304);
    }

    pub fn clock_f(&mut self, cycles: u8, freq: u32) {
        // @todo the performance here requires improvement
        for _ in 0..cycles {
            self.cycle(freq);
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            _ => {
                warnln!("Reading from unknown APU location 0x{:04x}", addr);
                0xff
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // 0xFF10 — NR10: Channel 1 sweep
            0xff10 => {
                self.ch1_sweep_slope = value & 0x03;
                self.ch1_sweep_increase = value & 0x04 == 0x04;
                self.ch1_sweep_pace = (value & 0x70) >> 4;
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
            }
            // 0xFF13 — NR13: Channel 1 wavelength low
            0xff13 => {
                self.ch1_wave_length = (self.ch1_wave_length & 0xff00) | value as u16;
            }
            // 0xFF14 — NR14: Channel 1 wavelength high & control
            0xff14 => {
                self.ch1_wave_length =
                    (self.ch1_wave_length & 0x00ff) | (((value & 0x07) as u16) << 8);
                self.ch1_sound_length |= value & 0x40 == 0x40;
                self.ch1_enabled |= value & 0x80 == 0x80;
                //println!("CH1 Enabled {}", self.ch1_enabled);
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
                self.ch2_sound_length |= value & 0x40 == 0x40;
                self.ch2_enabled |= value & 0x80 == 0x80;
                if value & 0x80 == 0x80 {
                    //self.ch2_timer = 0;
                    //self.ch2_sequence = 0;
                    //@todo improve this reset operation
                }
                //println!("CH2 Enabled {}", self.ch2_enabled);
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
                self.ch3_output_level = value & 0x60 >> 5;
            }
            // 0xFF1D — NR33: Channel 3 wavelength low [write-only]
            0xff1d => {
                self.ch3_wave_length = (self.ch3_wave_length & 0xff00) | value as u16;
            }
            // 0xFF1E — NR34: Channel 3 wavelength high & control
            0xff1e => {
                self.ch3_wave_length =
                    (self.ch3_wave_length & 0x00ff) | (((value & 0x07) as u16) << 8);
                self.ch3_sound_length |= value & 0x40 == 0x40;
                self.ch3_enabled |= value & 0x80 == 0x80;
                //println!("CH3 Enabled {}", self.ch3_enabled);
            }

            // 0xFF30-0xFF3F — Wave pattern RAM
            0xff30..=0xff3f => {
                self.wave_ram[addr as usize - 0xff30] = value;
            }

            _ => warnln!("Writing in unknown APU location 0x{:04x}", addr),
        }
    }

    #[inline(always)]
    pub fn cycle(&mut self, freq: u32) {
        self.ch1_timer = self.ch1_timer.saturating_sub(1);
        if self.ch1_timer == 0 {
            let target_freq = 1048576.0 / (2048.0 - self.ch1_wave_length as f32);
            self.ch1_timer = (freq as f32 / target_freq) as u16;
            self.ch1_sequence = (self.ch1_sequence + 1) & 7;

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
        }

        self.ch2_timer = self.ch2_timer.saturating_sub(1);
        if self.ch2_timer == 0 {
            let target_freq = 1048576.0 / (2048.0 - self.ch2_wave_length as f32);
            self.ch2_timer = (freq as f32 / target_freq) as u16;
            self.ch2_sequence = (self.ch2_sequence + 1) & 7;

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
        }

        self.output_timer = self.output_timer.saturating_sub(1);
        if self.output_timer == 0 {
            self.output_buffer.push(self.output());
            self.output_timer = (freq as f32 / 44100.0) as u16; // @todo target sampling rate is hardcoded
        }
    }

    pub fn output(&self) -> u8 {
        self.ch1_output + self.ch2_output
    }

    pub fn output_buffer(&self) -> &Vec<u8> {
        &self.output_buffer
    }

    pub fn clear_buffer(&mut self) {
        self.output_buffer.clear();
    }
}
