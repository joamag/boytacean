//! GBA APU (Audio Processing Unit) with 4 legacy GB channels
//! and 2 DirectSound PCM channels (FIFO A and FIFO B).
//!
//! The legacy channels are identical to the Game Boy's sound hardware
//! (square wave with sweep, square wave, programmable wave, noise)
//! with GBA-specific extensions for volume control, stereo panning,
//! and dual-bank wave RAM.

use std::collections::VecDeque;

use crate::warnln;

/// Sampling rate for audio output in Hz.
const SAMPLING_RATE: u32 = 32768;

/// Duty cycle waveforms for square wave channels.
///
/// Each row represents one of 4 duty cycles (12.5%, 25%, 50%, 75%)
/// with 8 samples per cycle.
const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [1, 0, 0, 0, 0, 0, 0, 1], // 25%
    [1, 0, 0, 0, 0, 1, 1, 1], // 50%
    [0, 1, 1, 1, 1, 1, 1, 0], // 75%
];

/// Divisor values for channel 4 noise generation.
const CH4_DIVISORS: [u8; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

/// DirectSound FIFO channel for PCM audio playback.
pub struct DirectSoundChannel {
    /// FIFO buffer (up to 32 bytes / 8 words).
    fifo: VecDeque<i8>,

    /// Current sample being output (raw from FIFO).
    current_sample: i8,

    /// Timer index that drives this channel (0 or 1).
    pub timer_id: usize,

    /// Volume: true = full, false = 50%.
    volume_full: bool,

    /// Enable left output.
    enable_left: bool,

    /// Enable right output.
    enable_right: bool,

    // -- debug counters (reset per frame by diagnostic) --
    /// Number of timer_tick calls (FIFO pops).
    pub debug_pops: u32,
    /// Number of timer_tick calls with empty FIFO.
    pub debug_underflows: u32,
    /// Number of reset_fifo calls.
    pub debug_resets: u32,
    /// Number of samples written to FIFO.
    pub debug_writes: u32,
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
            debug_pops: 0,
            debug_underflows: 0,
            debug_resets: 0,
            debug_writes: 0,
        }
    }

    /// Pushes a 32-bit word (4 samples) into the FIFO.
    pub fn write_fifo(&mut self, value: u32) {
        for i in 0..4 {
            let sample = ((value >> (i * 8)) & 0xFF) as i8;
            if self.fifo.len() < 32 {
                self.fifo.push_back(sample);
                self.debug_writes += 1;
            }
        }
    }

    /// Pushes a 16-bit half-word (2 samples) into the FIFO.
    pub fn write_fifo_half(&mut self, value: u16) {
        for i in 0..2 {
            let sample = ((value >> (i * 8)) & 0xFF) as i8;
            if self.fifo.len() < 32 {
                self.fifo.push_back(sample);
                self.debug_writes += 1;
            }
        }
    }

    /// Pops the next sample from the FIFO (called on timer overflow).
    pub fn timer_tick(&mut self) {
        self.debug_pops += 1;
        if let Some(sample) = self.fifo.pop_front() {
            self.current_sample = sample;
        } else {
            self.debug_underflows += 1;
        }
    }

    /// Returns true if the FIFO needs refilling (fewer than 16 bytes).
    pub fn needs_refill(&self) -> bool {
        self.fifo.len() <= 16
    }

    /// Resets the FIFO and current sample.
    pub fn reset_fifo(&mut self) {
        self.fifo.clear();
        self.current_sample = 0;
        self.debug_resets += 1;
    }

    /// Returns the current output sample with hardware volume scaling.
    /// On real GBA, DirectSound samples are multiplied by 4 at full volume
    /// or by 2 at half volume (matching NanoBoyAdvance and mGBA).
    pub fn output(&self) -> i16 {
        let sample = self.current_sample as i16;
        if self.volume_full {
            sample * 4
        } else {
            sample * 2
        }
    }

    /// Returns the left output sample, or 0 if left is disabled.
    pub fn output_left(&self) -> i16 {
        if self.enable_left {
            self.output()
        } else {
            0
        }
    }

    /// Returns the right output sample, or 0 if right is disabled.
    pub fn output_right(&self) -> i16 {
        if self.enable_right {
            self.output()
        } else {
            0
        }
    }

    /// Returns the number of samples currently in the FIFO.
    pub fn fifo_len(&self) -> usize {
        self.fifo.len()
    }

    /// Returns the current sample value (last popped from FIFO).
    pub fn current_sample(&self) -> i8 {
        self.current_sample
    }

    /// Returns whether left output is enabled.
    pub fn enable_left(&self) -> bool {
        self.enable_left
    }

    /// Returns whether right output is enabled.
    pub fn enable_right(&self) -> bool {
        self.enable_right
    }

    /// Returns whether full volume is enabled.
    pub fn volume_full(&self) -> bool {
        self.volume_full
    }

    /// Resets debug counters (call at start of each diagnostic frame).
    pub fn reset_debug_counters(&mut self) {
        self.debug_pops = 0;
        self.debug_underflows = 0;
        self.debug_resets = 0;
        self.debug_writes = 0;
    }
}

impl Default for DirectSoundChannel {
    fn default() -> Self {
        Self::new()
    }
}

/// GBA Audio Processing Unit.
///
/// Contains 4 legacy Game Boy sound channels (square with sweep,
/// square, wave, noise) and 2 DirectSound PCM FIFO channels.
pub struct GbaApu {
    // -- control registers --
    /// SOUNDCNT_L: legacy channel panning and master volume.
    soundcnt_l: u16,

    /// SOUNDCNT_H: DirectSound volume, routing, timer select.
    soundcnt_h: u16,

    /// SOUNDCNT_X: master sound enable (bit 7).
    soundcnt_x: u16,

    /// SOUNDBIAS: DC bias and PWM resolution.
    soundbias: u16,

    // -- wave RAM --
    /// Wave RAM banks (2 x 16 bytes for dual-bank mode).
    wave_ram: [[u8; 16]; 2],

    /// Currently selected wave RAM bank for CPU access (0 or 1).
    wave_bank: u8,

    /// Wave RAM dimension (false = single bank, true = dual 64-sample).
    wave_dimension: bool,

    // -- DirectSound channels --
    /// DirectSound FIFO channels A and B.
    pub direct_sound: [DirectSoundChannel; 2],

    // -- channel 1: square with sweep --
    /// Sweep pace (0 = disabled, 1-7 = period in 128Hz ticks).
    ch1_sweep_pace: u8,

    /// Sweep direction (false = increase frequency, true = decrease).
    ch1_sweep_decrease: bool,

    /// Sweep shift amount (0-7).
    ch1_sweep_slope: u8,

    /// Sweep shadow frequency register.
    ch1_sweep_shadow: u16,

    /// Sweep sequence counter.
    ch1_sweep_sequence: u8,

    /// Sweep enabled flag.
    ch1_sweep_enabled: bool,

    /// Duty cycle index (0-3).
    ch1_wave_duty: u8,

    /// Length timer (counts down from 64).
    ch1_length_timer: u8,

    /// Whether length counter is enabled.
    ch1_length_enabled: bool,

    /// Envelope pace (0 = disabled, 1-7).
    ch1_envelope_pace: u8,

    /// Envelope direction (0 = decrease, 1 = increase).
    ch1_envelope_direction: u8,

    /// Envelope sequence counter.
    ch1_envelope_sequence: u8,

    /// Whether envelope is still active.
    ch1_envelope_enabled: bool,

    /// Current volume (0-15).
    ch1_volume: u8,

    /// Initial volume from envelope register.
    ch1_initial_volume: u8,

    /// Frequency (11-bit, 0-2047).
    ch1_wave_length: u16,

    /// DAC enabled (envelope register bits 3-7 != 0).
    ch1_dac: bool,

    /// Frequency timer.
    ch1_timer: i32,

    /// Waveform sequence position (0-7).
    ch1_sequence: u8,

    /// Current output sample value.
    ch1_output: u8,

    /// Channel enabled.
    ch1_enabled: bool,

    // -- channel 2: square --
    /// Duty cycle index (0-3).
    ch2_wave_duty: u8,

    /// Length timer (counts down from 64).
    ch2_length_timer: u8,

    /// Whether length counter is enabled.
    ch2_length_enabled: bool,

    /// Envelope pace (0 = disabled, 1-7).
    ch2_envelope_pace: u8,

    /// Envelope direction (0 = decrease, 1 = increase).
    ch2_envelope_direction: u8,

    /// Envelope sequence counter.
    ch2_envelope_sequence: u8,

    /// Whether envelope is still active.
    ch2_envelope_enabled: bool,

    /// Current volume (0-15).
    ch2_volume: u8,

    /// Initial volume from envelope register.
    ch2_initial_volume: u8,

    /// Frequency (11-bit).
    ch2_wave_length: u16,

    /// DAC enabled.
    ch2_dac: bool,

    /// Frequency timer.
    ch2_timer: i32,

    /// Waveform sequence position (0-7).
    ch2_sequence: u8,

    /// Current output sample value.
    ch2_output: u8,

    /// Channel enabled.
    ch2_enabled: bool,

    // -- channel 3: wave --
    /// DAC enabled (SOUND3CNT_L bit 7).
    ch3_dac: bool,

    /// Length timer (counts down from 256).
    ch3_length_timer: u16,

    /// Whether length counter is enabled.
    ch3_length_enabled: bool,

    /// Output level (0-3 from bits 13-14, plus GBA force-75% from bit 15).
    ch3_output_level: u8,

    /// Force 75% volume (GBA extension, SOUND3CNT_H bit 15).
    ch3_force_volume: bool,

    /// Frequency (11-bit).
    ch3_wave_length: u16,

    /// Frequency timer.
    ch3_timer: i32,

    /// Current position in wave RAM (0-31 for single, 0-63 for dual).
    ch3_position: u8,

    /// Current output sample value.
    ch3_output: u8,

    /// Channel enabled.
    ch3_enabled: bool,

    // -- channel 4: noise --
    /// Length timer (counts down from 64).
    ch4_length_timer: u8,

    /// Whether length counter is enabled.
    ch4_length_enabled: bool,

    /// Envelope pace (0 = disabled, 1-7).
    ch4_envelope_pace: u8,

    /// Envelope direction (0 = decrease, 1 = increase).
    ch4_envelope_direction: u8,

    /// Envelope sequence counter.
    ch4_envelope_sequence: u8,

    /// Whether envelope is still active.
    ch4_envelope_enabled: bool,

    /// Current volume (0-15).
    ch4_volume: u8,

    /// Initial volume from envelope register.
    ch4_initial_volume: u8,

    /// Clock divisor index (0-7).
    ch4_divisor: u8,

    /// Width mode (false = 15-bit LFSR, true = 7-bit).
    ch4_width_mode: bool,

    /// Clock shift (0-15).
    ch4_clock_shift: u8,

    /// DAC enabled.
    ch4_dac: bool,

    /// Frequency timer.
    ch4_timer: i32,

    /// Linear feedback shift register.
    ch4_lfsr: u16,

    /// Current output sample value.
    ch4_output: u8,

    /// Channel enabled.
    ch4_enabled: bool,

    // -- frame sequencer --
    /// Internal sequencer counter (counts CPU cycles).
    sequencer_counter: u32,

    /// Current sequencer step (0-7).
    sequencer_step: u8,

    // -- output --
    /// Output audio buffer (interleaved stereo i16 samples).
    audio_buffer: VecDeque<i16>,

    /// Sample counter for timing audio output.
    sample_counter: u32,

    /// Number of CPU cycles per audio sample.
    sample_period: u32,
}

impl GbaApu {
    pub fn new() -> Self {
        Self {
            soundcnt_l: 0,
            soundcnt_h: 0,
            soundcnt_x: 0,
            soundbias: 0x200,
            wave_ram: [[0; 16]; 2],
            wave_bank: 0,
            wave_dimension: false,
            direct_sound: [DirectSoundChannel::new(), DirectSoundChannel::new()],

            ch1_sweep_pace: 0,
            ch1_sweep_decrease: false,
            ch1_sweep_slope: 0,
            ch1_sweep_shadow: 0,
            ch1_sweep_sequence: 0,
            ch1_sweep_enabled: false,
            ch1_wave_duty: 0,
            ch1_length_timer: 0,
            ch1_length_enabled: false,
            ch1_envelope_pace: 0,
            ch1_envelope_direction: 0,
            ch1_envelope_sequence: 0,
            ch1_envelope_enabled: false,
            ch1_volume: 0,
            ch1_initial_volume: 0,
            ch1_wave_length: 0,
            ch1_dac: false,
            ch1_timer: 0,
            ch1_sequence: 0,
            ch1_output: 0,
            ch1_enabled: false,

            ch2_wave_duty: 0,
            ch2_length_timer: 0,
            ch2_length_enabled: false,
            ch2_envelope_pace: 0,
            ch2_envelope_direction: 0,
            ch2_envelope_sequence: 0,
            ch2_envelope_enabled: false,
            ch2_volume: 0,
            ch2_initial_volume: 0,
            ch2_wave_length: 0,
            ch2_dac: false,
            ch2_timer: 0,
            ch2_sequence: 0,
            ch2_output: 0,
            ch2_enabled: false,

            ch3_dac: false,
            ch3_length_timer: 0,
            ch3_length_enabled: false,
            ch3_output_level: 0,
            ch3_force_volume: false,
            ch3_wave_length: 0,
            ch3_timer: 0,
            ch3_position: 0,
            ch3_output: 0,
            ch3_enabled: false,

            ch4_length_timer: 0,
            ch4_length_enabled: false,
            ch4_envelope_pace: 0,
            ch4_envelope_direction: 0,
            ch4_envelope_sequence: 0,
            ch4_envelope_enabled: false,
            ch4_volume: 0,
            ch4_initial_volume: 0,
            ch4_divisor: 0,
            ch4_width_mode: false,
            ch4_clock_shift: 0,
            ch4_dac: false,
            ch4_timer: 0,
            ch4_lfsr: 0x7FFF,
            ch4_output: 0,
            ch4_enabled: false,

            sequencer_counter: 0,
            sequencer_step: 0,

            audio_buffer: VecDeque::with_capacity(4096),
            sample_counter: 0,
            sample_period: super::consts::CPU_FREQ / SAMPLING_RATE,
        }
    }

    // -- control register accessors --

    pub fn soundcnt_l(&self) -> u16 {
        self.soundcnt_l
    }

    pub fn set_soundcnt_l(&mut self, value: u16) {
        self.soundcnt_l = value;
    }

    pub fn soundcnt_h(&self) -> u16 {
        // bits 11, 15 (FIFO reset) are write-only strobe bits
        self.soundcnt_h & !((1 << 11) | (1 << 15))
    }

    pub fn set_soundcnt_h(&mut self, value: u16) {
        // store without FIFO reset strobe bits (11, 15) — they are
        // write-only one-shot actions, not persistent state
        self.soundcnt_h = value & !((1 << 11) | (1 << 15));

        // configure DirectSound channels from SOUNDCNT_H
        self.direct_sound[0].volume_full = value & (1 << 2) != 0;
        self.direct_sound[1].volume_full = value & (1 << 3) != 0;
        self.direct_sound[0].enable_right = value & (1 << 8) != 0;
        self.direct_sound[0].enable_left = value & (1 << 9) != 0;
        self.direct_sound[0].timer_id = ((value >> 10) & 1) as usize;
        self.direct_sound[1].enable_right = value & (1 << 12) != 0;
        self.direct_sound[1].enable_left = value & (1 << 13) != 0;
        self.direct_sound[1].timer_id = ((value >> 14) & 1) as usize;

        // reset FIFO if requested (one-shot, bits not stored)
        if value & (1 << 11) != 0 {
            self.direct_sound[0].reset_fifo();
        }
        if value & (1 << 15) != 0 {
            self.direct_sound[1].reset_fifo();
        }
    }

    /// Reads SOUNDCNT_X with synthesized channel status bits.
    pub fn soundcnt_x(&self) -> u16 {
        let mut value = self.soundcnt_x & 0x80;
        if self.ch1_enabled && self.ch1_dac {
            value |= 1;
        }
        if self.ch2_enabled && self.ch2_dac {
            value |= 2;
        }
        if self.ch3_enabled && self.ch3_dac {
            value |= 4;
        }
        if self.ch4_enabled && self.ch4_dac {
            value |= 8;
        }
        value | 0x70
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

    // -- wave RAM --

    pub fn wave_ram(&self) -> &[u8; 16] {
        // CPU accesses the bank NOT currently being played
        let bank = if self.wave_dimension {
            1 - self.wave_bank
        } else {
            self.wave_bank
        };
        &self.wave_ram[bank as usize]
    }

    pub fn read_wave_ram(&self, offset: usize) -> u8 {
        if offset < 16 {
            let bank = if self.wave_dimension {
                1 - self.wave_bank
            } else {
                self.wave_bank
            };
            self.wave_ram[bank as usize][offset]
        } else {
            0
        }
    }

    pub fn write_wave_ram(&mut self, offset: usize, value: u8) {
        if offset < 16 {
            let bank = if self.wave_dimension {
                1 - self.wave_bank
            } else {
                self.wave_bank
            };
            self.wave_ram[bank as usize][offset] = value;
        }
    }

    // -- audio buffer --

    pub fn audio_buffer(&self) -> &VecDeque<i16> {
        &self.audio_buffer
    }

    pub fn clear_audio_buffer(&mut self) {
        self.audio_buffer.clear();
    }

    // -- timer integration --

    /// Called when a timer overflows; feeds DirectSound FIFO channels.
    pub fn timer_overflow(&mut self, timer_id: usize) {
        for i in 0..2 {
            if self.direct_sound[i].timer_id == timer_id {
                self.direct_sound[i].timer_tick();
            }
        }
    }

    // -- main clock --

    /// Clocks the APU by the given number of CPU cycles.
    pub fn clock(&mut self, cycles: u32) {
        if self.soundcnt_x & 0x80 == 0 {
            return;
        }

        // clock legacy channel frequency timers
        self.tick_channels(cycles);

        // update frame sequencer (512 Hz)
        self.sequencer_counter += cycles;
        let sequencer_period = super::consts::CPU_FREQ / 512;
        while self.sequencer_counter >= sequencer_period {
            self.sequencer_counter -= sequencer_period;
            self.clock_frame_sequencer();
        }

        // generate output samples at the configured rate
        self.sample_counter += cycles;
        while self.sample_counter >= self.sample_period {
            self.sample_counter -= self.sample_period;
            self.generate_sample();
        }
    }

    // -- frame sequencer --

    /// Dispatches length, envelope, and sweep at 512 Hz sub-steps.
    ///
    /// The frame sequencer runs an 8-step cycle:
    /// - Steps 0, 2, 4, 6: tick length counters.
    /// - Steps 2, 6: tick channel 1 frequency sweep.
    /// - Step 7: tick volume envelopes.
    fn clock_frame_sequencer(&mut self) {
        match self.sequencer_step {
            0 => self.tick_length_all(),
            1 => (),
            2 => {
                self.tick_ch1_sweep();
                self.tick_length_all();
            }
            3 => (),
            4 => self.tick_length_all(),
            5 => (),
            6 => {
                self.tick_ch1_sweep();
                self.tick_length_all();
            }
            7 => self.tick_envelope_all(),
            _ => (),
        }
        self.sequencer_step = (self.sequencer_step + 1) & 7;
    }

    // -- length counters --

    /// Ticks length counters for all channels.
    #[inline(always)]
    fn tick_length_all(&mut self) {
        // channel 1
        if self.ch1_length_enabled && self.ch1_length_timer > 0 {
            self.ch1_length_timer = self.ch1_length_timer.saturating_sub(1);
            if self.ch1_length_timer == 0 {
                self.ch1_enabled = false;
            }
        }

        // channel 2
        if self.ch2_length_enabled && self.ch2_length_timer > 0 {
            self.ch2_length_timer = self.ch2_length_timer.saturating_sub(1);
            if self.ch2_length_timer == 0 {
                self.ch2_enabled = false;
            }
        }

        // channel 3
        if self.ch3_length_enabled && self.ch3_length_timer > 0 {
            self.ch3_length_timer = self.ch3_length_timer.saturating_sub(1);
            if self.ch3_length_timer == 0 {
                self.ch3_enabled = false;
            }
        }

        // channel 4
        if self.ch4_length_enabled && self.ch4_length_timer > 0 {
            self.ch4_length_timer = self.ch4_length_timer.saturating_sub(1);
            if self.ch4_length_timer == 0 {
                self.ch4_enabled = false;
            }
        }
    }

    // -- volume envelopes --

    /// Ticks volume envelopes for channels 1, 2, and 4.
    #[inline(always)]
    fn tick_envelope_all(&mut self) {
        // channel 1
        if self.ch1_enabled && self.ch1_envelope_enabled && self.ch1_envelope_pace > 0 {
            self.ch1_envelope_sequence += 1;
            if self.ch1_envelope_sequence >= self.ch1_envelope_pace {
                if self.ch1_envelope_direction == 1 {
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

        // channel 2
        if self.ch2_enabled && self.ch2_envelope_enabled && self.ch2_envelope_pace > 0 {
            self.ch2_envelope_sequence += 1;
            if self.ch2_envelope_sequence >= self.ch2_envelope_pace {
                if self.ch2_envelope_direction == 1 {
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

        // channel 4
        if self.ch4_enabled && self.ch4_envelope_enabled && self.ch4_envelope_pace > 0 {
            self.ch4_envelope_sequence += 1;
            if self.ch4_envelope_sequence >= self.ch4_envelope_pace {
                if self.ch4_envelope_direction == 1 {
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

    // -- frequency sweep (channel 1 only) --

    /// Ticks channel 1 frequency sweep at 128 Hz.
    #[inline(always)]
    fn tick_ch1_sweep(&mut self) {
        if self.ch1_sweep_pace == 0 {
            return;
        }
        self.ch1_sweep_sequence += 1;
        if self.ch1_sweep_sequence >= self.ch1_sweep_pace {
            let delta = self.ch1_sweep_shadow >> self.ch1_sweep_slope as u16;
            if self.ch1_sweep_decrease {
                self.ch1_sweep_shadow = self.ch1_sweep_shadow.saturating_sub(delta);
            } else {
                self.ch1_sweep_shadow = self.ch1_sweep_shadow.saturating_add(delta);
            }
            if self.ch1_sweep_shadow > 0x07FF {
                self.ch1_enabled = false;
                self.ch1_sweep_shadow = 0x07FF;
            } else {
                self.ch1_wave_length = self.ch1_sweep_shadow;
            }
            self.ch1_sweep_sequence = 0;
        }
    }

    // -- channel frequency timers --

    /// Ticks all channel frequency timers by the given CPU cycles.
    fn tick_channels(&mut self, cycles: u32) {
        let cycles_i32 = cycles as i32;

        // channel 1 (square with sweep)
        self.ch1_timer -= cycles_i32;
        while self.ch1_timer <= 0 {
            self.ch1_timer += (2048 - self.ch1_wave_length as i32) * 16;
            if self.ch1_enabled && self.ch1_dac {
                self.ch1_output =
                    if DUTY_TABLE[self.ch1_wave_duty as usize][self.ch1_sequence as usize] == 1 {
                        self.ch1_volume
                    } else {
                        0
                    };
            } else {
                self.ch1_output = 0;
            }
            self.ch1_sequence = (self.ch1_sequence + 1) & 7;
        }

        // channel 2 (square)
        self.ch2_timer -= cycles_i32;
        while self.ch2_timer <= 0 {
            self.ch2_timer += (2048 - self.ch2_wave_length as i32) * 16;
            if self.ch2_enabled && self.ch2_dac {
                self.ch2_output =
                    if DUTY_TABLE[self.ch2_wave_duty as usize][self.ch2_sequence as usize] == 1 {
                        self.ch2_volume
                    } else {
                        0
                    };
            } else {
                self.ch2_output = 0;
            }
            self.ch2_sequence = (self.ch2_sequence + 1) & 7;
        }

        // channel 3 (wave)
        self.ch3_timer -= cycles_i32;
        while self.ch3_timer <= 0 {
            self.ch3_timer += (2048 - self.ch3_wave_length as i32) * 8;
            if self.ch3_enabled && self.ch3_dac {
                let max_pos = if self.wave_dimension { 63 } else { 31 };
                let bank = if self.wave_dimension {
                    (self.ch3_position >> 5) as usize
                } else {
                    self.wave_bank as usize
                };
                let pos_in_bank = (self.ch3_position & 31) as usize;
                let byte_index = pos_in_bank >> 1;
                let mut sample = self.wave_ram[bank][byte_index];
                sample = if pos_in_bank & 1 == 0 {
                    (sample >> 4) & 0x0F
                } else {
                    sample & 0x0F
                };

                // apply volume level
                if self.ch3_force_volume {
                    // GBA extension: 75% volume (shift right 1, then add quarter)
                    sample = (sample >> 1) + (sample >> 2);
                } else if self.ch3_output_level > 0 {
                    sample >>= self.ch3_output_level - 1;
                } else {
                    sample = 0;
                }

                self.ch3_output = sample;
                self.ch3_position = if self.ch3_position >= max_pos {
                    0
                } else {
                    self.ch3_position + 1
                };
            } else {
                self.ch3_output = 0;
                self.ch3_position = (self.ch3_position + 1) & 31;
            }
        }

        // channel 4 (noise)
        self.ch4_timer -= cycles_i32;
        while self.ch4_timer <= 0 {
            self.ch4_timer +=
                ((CH4_DIVISORS[self.ch4_divisor as usize] as i32) << self.ch4_clock_shift) * 4;
            if self.ch4_enabled && self.ch4_dac {
                let xor_result =
                    ((self.ch4_lfsr & 0x0001) ^ ((self.ch4_lfsr >> 1) & 0x0001)) == 0x0001;
                self.ch4_lfsr >>= 1;
                self.ch4_lfsr |= if xor_result { 1 << 14 } else { 0 };
                if self.ch4_width_mode {
                    self.ch4_lfsr &= 0xFFBF;
                    self.ch4_lfsr |= if xor_result { 0x40 } else { 0 };
                }
                self.ch4_output = if xor_result { self.ch4_volume } else { 0 };
            } else {
                self.ch4_output = 0;
            }
        }
    }

    // -- output generation --

    /// Generates a single stereo audio sample with proper mixing.
    ///
    /// Mixes legacy channels with stereo panning and master volume
    /// from SOUNDCNT_L, then adds DirectSound channels from SOUNDCNT_H.
    fn generate_sample(&mut self) {
        // legacy channel outputs (0-15 each)
        let ch1 = self.ch1_output as i32;
        let ch2 = self.ch2_output as i32;
        let ch3 = self.ch3_output as i32;
        let ch4 = self.ch4_output as i32;

        // stereo panning from SOUNDCNT_L bits 8-15
        let panning = self.soundcnt_l;
        let mut legacy_right: i32 = 0;
        let mut legacy_left: i32 = 0;

        if panning & (1 << 8) != 0 {
            legacy_right += ch1;
        }
        if panning & (1 << 9) != 0 {
            legacy_right += ch2;
        }
        if panning & (1 << 10) != 0 {
            legacy_right += ch3;
        }
        if panning & (1 << 11) != 0 {
            legacy_right += ch4;
        }
        if panning & (1 << 12) != 0 {
            legacy_left += ch1;
        }
        if panning & (1 << 13) != 0 {
            legacy_left += ch2;
        }
        if panning & (1 << 14) != 0 {
            legacy_left += ch3;
        }
        if panning & (1 << 15) != 0 {
            legacy_left += ch4;
        }

        // Scale PSG sum by 8 (matches mGBA's <<= 3 and NanoBoyAdvance's
        // ×8 waveform amplitude). Without this, PSG is 16x too quiet
        // relative to DirectSound channels.
        legacy_right <<= 3;
        legacy_left <<= 3;

        // master volume from SOUNDCNT_L bits 0-2 (right) and 4-6 (left)
        let right_vol = (self.soundcnt_l & 0x07) as i32 + 1;
        let left_vol = ((self.soundcnt_l >> 4) & 0x07) as i32 + 1;
        legacy_right *= right_vol;
        legacy_left *= left_vol;

        // SOUNDCNT_H bits 0-1: legacy volume ratio (0=25%, 1=50%, 2=100%)
        // mGBA uses shift = 4 - volume (not 2 - volume)
        let legacy_volume = ((self.soundcnt_h & 0x03) as i32).min(2);
        let legacy_shift = 4 - legacy_volume;
        legacy_right >>= legacy_shift;
        legacy_left >>= legacy_shift;

        // DirectSound channels (scaled: full=i8*4, half=i8*2)
        let ds_a_left = self.direct_sound[0].output_left() as i32;
        let ds_a_right = self.direct_sound[0].output_right() as i32;
        let ds_b_left = self.direct_sound[1].output_left() as i32;
        let ds_b_right = self.direct_sound[1].output_right() as i32;

        // GBA hardware mixer: add SOUNDBIAS, clamp to 10-bit DAC, subtract bias.
        // This matches NanoBoyAdvance and mGBA's mixing pipeline.
        let bias = (self.soundbias & 0x3FF) as i32;
        let raw_left = legacy_left + ds_a_left + ds_b_left + bias;
        let raw_right = legacy_right + ds_a_right + ds_b_right + bias;
        let left = raw_left.clamp(0, 0x3FF) - bias;
        let right = raw_right.clamp(0, 0x3FF) - bias;

        // Scale from 10-bit signed (±512) to i16 range (×64)
        let left = (left * 64).clamp(-32768, 32767) as i16;
        let right = (right * 64).clamp(-32768, 32767) as i16;

        self.audio_buffer.push_back(left);
        self.audio_buffer.push_back(right);
    }

    // -- trigger functions --

    /// Triggers channel 1 (resets timers, envelope, sweep).
    fn trigger_ch1(&mut self) {
        self.ch1_timer = (2048 - self.ch1_wave_length as i32) * 16;
        self.ch1_envelope_sequence = 0;
        self.ch1_envelope_enabled = self.ch1_envelope_pace > 0;
        self.ch1_volume = self.ch1_initial_volume;
        self.ch1_sweep_shadow = self.ch1_wave_length;
        self.ch1_sweep_sequence = 0;
        self.ch1_sweep_enabled = self.ch1_sweep_pace > 0 || self.ch1_sweep_slope > 0;

        if self.ch1_length_timer == 0 {
            self.ch1_length_timer = 64;
            if self.ch1_length_enabled && self.sequencer_step % 2 == 1 {
                self.ch1_length_timer = self.ch1_length_timer.saturating_sub(1);
                if self.ch1_length_timer == 0 {
                    self.ch1_enabled = false;
                }
            }
        }

        // overflow check on sweep
        if self.ch1_sweep_slope > 0 {
            let delta = self.ch1_sweep_shadow >> self.ch1_sweep_slope;
            let new_freq = if self.ch1_sweep_decrease {
                self.ch1_sweep_shadow.saturating_sub(delta)
            } else {
                self.ch1_sweep_shadow.saturating_add(delta)
            };
            if new_freq > 0x07FF {
                self.ch1_enabled = false;
            }
        }
    }

    /// Triggers channel 2 (resets timer and envelope).
    fn trigger_ch2(&mut self) {
        self.ch2_timer = (2048 - self.ch2_wave_length as i32) * 16;
        self.ch2_envelope_sequence = 0;
        self.ch2_envelope_enabled = self.ch2_envelope_pace > 0;
        self.ch2_volume = self.ch2_initial_volume;

        if self.ch2_length_timer == 0 {
            self.ch2_length_timer = 64;
            if self.ch2_length_enabled && self.sequencer_step % 2 == 1 {
                self.ch2_length_timer = self.ch2_length_timer.saturating_sub(1);
                if self.ch2_length_timer == 0 {
                    self.ch2_enabled = false;
                }
            }
        }
    }

    /// Triggers channel 3 (resets timer and wave position).
    fn trigger_ch3(&mut self) {
        self.ch3_timer = (2048 - self.ch3_wave_length as i32) * 8;
        self.ch3_position = 0;

        if self.ch3_length_timer == 0 {
            self.ch3_length_timer = 256;
            if self.ch3_length_enabled && self.sequencer_step % 2 == 1 {
                self.ch3_length_timer = self.ch3_length_timer.saturating_sub(1);
                if self.ch3_length_timer == 0 {
                    self.ch3_enabled = false;
                }
            }
        }
    }

    /// Triggers channel 4 (resets timer, LFSR, and envelope).
    fn trigger_ch4(&mut self) {
        self.ch4_timer =
            ((CH4_DIVISORS[self.ch4_divisor as usize] as i32) << self.ch4_clock_shift) * 4;
        self.ch4_lfsr = 0x7FFF;
        self.ch4_envelope_sequence = 0;
        self.ch4_envelope_enabled = self.ch4_envelope_pace > 0;
        self.ch4_volume = self.ch4_initial_volume;

        if self.ch4_length_timer == 0 {
            self.ch4_length_timer = 64;
            if self.ch4_length_enabled && self.sequencer_step % 2 == 1 {
                self.ch4_length_timer = self.ch4_length_timer.saturating_sub(1);
                if self.ch4_length_timer == 0 {
                    self.ch4_enabled = false;
                }
            }
        }
    }

    // -- register I/O --

    /// Reads a legacy channel register.
    ///
    /// Some bits are write-only and return fixed values. The register
    /// layout follows the GBA memory map at 0x04000060-0x0400007D.
    pub fn read_channel_reg(&self, addr: u32) -> u8 {
        let offset = addr & 0xFF;
        match offset {
            // SOUND1CNT_L (0x60): sweep
            0x60 => {
                (self.ch1_sweep_slope & 0x07)
                    | (if self.ch1_sweep_decrease { 0x08 } else { 0x00 })
                    | ((self.ch1_sweep_pace & 0x07) << 4)
                    | 0x80
            }
            0x61 => 0,
            // SOUND1CNT_H (0x62): duty/length
            0x62 => (self.ch1_wave_duty << 6) | 0x3F,
            // SOUND1CNT_H (0x63): envelope
            0x63 => {
                (self.ch1_envelope_pace & 0x07)
                    | ((self.ch1_envelope_direction & 0x01) << 3)
                    | ((self.ch1_initial_volume & 0x0F) << 4)
            }
            // SOUND1CNT_X (0x64): frequency low (write-only)
            0x64 => 0xFF,
            // SOUND1CNT_X (0x65): frequency high + control
            0x65 => (if self.ch1_length_enabled { 0x40 } else { 0x00 }) | 0xBF,

            // SOUND2CNT_L (0x68): duty/length
            0x68 => (self.ch2_wave_duty << 6) | 0x3F,
            // SOUND2CNT_L (0x69): envelope
            0x69 => {
                (self.ch2_envelope_pace & 0x07)
                    | ((self.ch2_envelope_direction & 0x01) << 3)
                    | ((self.ch2_initial_volume & 0x0F) << 4)
            }
            0x6A | 0x6B => 0,
            // SOUND2CNT_H (0x6C): frequency low (write-only)
            0x6C => 0xFF,
            // SOUND2CNT_H (0x6D): frequency high + control
            0x6D => (if self.ch2_length_enabled { 0x40 } else { 0x00 }) | 0xBF,

            // SOUND3CNT_L (0x70): DAC enable + wave bank
            0x70 => {
                (if self.wave_dimension { 0x20 } else { 0x00 })
                    | ((self.wave_bank & 1) << 6)
                    | (if self.ch3_dac { 0x80 } else { 0x00 })
                    | 0x1F
            }
            0x71 => 0,
            // SOUND3CNT_H (0x72): length (write-only)
            0x72 => 0xFF,
            // SOUND3CNT_H (0x73): volume
            0x73 => {
                ((self.ch3_output_level & 0x03) << 5)
                    | (if self.ch3_force_volume { 0x80 } else { 0x00 })
                    | 0x1F
            }
            // SOUND3CNT_X (0x74): frequency low (write-only)
            0x74 => 0xFF,
            // SOUND3CNT_X (0x75): frequency high + control
            0x75 => (if self.ch3_length_enabled { 0x40 } else { 0x00 }) | 0xBF,

            // SOUND4CNT_L (0x78): length (write-only)
            0x78 => 0xFF,
            // SOUND4CNT_L (0x79): envelope
            0x79 => {
                (self.ch4_envelope_pace & 0x07)
                    | ((self.ch4_envelope_direction & 0x01) << 3)
                    | ((self.ch4_initial_volume & 0x0F) << 4)
            }
            0x7A | 0x7B => 0,
            // SOUND4CNT_H (0x7C): polynomial counter
            0x7C => {
                (self.ch4_divisor & 0x07)
                    | (if self.ch4_width_mode { 0x08 } else { 0x00 })
                    | ((self.ch4_clock_shift & 0x0F) << 4)
            }
            // SOUND4CNT_H (0x7D): control
            0x7D => (if self.ch4_length_enabled { 0x40 } else { 0x00 }) | 0xBF,

            _ => 0,
        }
    }

    /// Writes to a legacy channel register.
    ///
    /// Handles the GBA register layout at 0x04000060-0x0400007D,
    /// including trigger logic, DAC gating, length enable edge cases,
    /// and envelope/sweep parameter loading.
    pub fn write_channel_reg(&mut self, addr: u32, value: u8) {
        let offset = addr & 0xFF;
        match offset {
            // -- channel 1: square with sweep --

            // SOUND1CNT_L (0x60): sweep parameters
            0x60 => {
                self.ch1_sweep_slope = value & 0x07;
                self.ch1_sweep_decrease = value & 0x08 != 0;
                self.ch1_sweep_pace = (value >> 4) & 0x07;
                self.ch1_sweep_sequence = 0;
            }

            // SOUND1CNT_H low (0x62): duty cycle + length
            0x62 => {
                self.ch1_length_timer = 64 - (value & 0x3F);
                self.ch1_wave_duty = (value >> 6) & 0x03;
            }

            // SOUND1CNT_H high (0x63): envelope parameters
            0x63 => {
                self.ch1_envelope_pace = value & 0x07;
                self.ch1_envelope_direction = (value >> 3) & 0x01;
                self.ch1_initial_volume = (value >> 4) & 0x0F;
                self.ch1_volume = self.ch1_initial_volume;
                self.ch1_envelope_enabled = self.ch1_envelope_pace > 0;
                self.ch1_envelope_sequence = 0;

                // DAC check: bits 3-7 must not all be zero
                self.ch1_dac = value & 0xF8 != 0;
                if !self.ch1_dac {
                    self.ch1_enabled = false;
                }
            }

            // SOUND1CNT_X low (0x64): frequency low byte
            0x64 => {
                self.ch1_wave_length = (self.ch1_wave_length & 0xFF00) | value as u16;
            }

            // SOUND1CNT_X high (0x65): frequency high + length enable + trigger
            0x65 => {
                let length_trigger = value & 0x40 != 0;
                let trigger = value & 0x80 != 0;
                let length_edge = length_trigger && !self.ch1_length_enabled;

                self.ch1_wave_length =
                    (self.ch1_wave_length & 0x00FF) | (((value & 0x07) as u16) << 8);
                self.ch1_length_enabled = length_trigger;

                // length enable edge: tick immediately on odd sequencer step
                if length_edge && self.sequencer_step % 2 == 1 && self.ch1_length_timer > 0 {
                    self.ch1_length_timer = self.ch1_length_timer.saturating_sub(1);
                    if self.ch1_length_timer == 0 {
                        self.ch1_enabled = false;
                    }
                }

                if trigger && self.ch1_dac {
                    self.ch1_enabled = true;
                    self.trigger_ch1();
                }

                if length_trigger && self.ch1_length_timer == 0 {
                    self.ch1_enabled = false;
                }
            }

            // -- channel 2: square --

            // SOUND2CNT_L low (0x68): duty cycle + length
            0x68 => {
                self.ch2_length_timer = 64 - (value & 0x3F);
                self.ch2_wave_duty = (value >> 6) & 0x03;
            }

            // SOUND2CNT_L high (0x69): envelope parameters
            0x69 => {
                self.ch2_envelope_pace = value & 0x07;
                self.ch2_envelope_direction = (value >> 3) & 0x01;
                self.ch2_initial_volume = (value >> 4) & 0x0F;
                self.ch2_volume = self.ch2_initial_volume;
                self.ch2_envelope_enabled = self.ch2_envelope_pace > 0;
                self.ch2_envelope_sequence = 0;

                self.ch2_dac = value & 0xF8 != 0;
                if !self.ch2_dac {
                    self.ch2_enabled = false;
                }
            }

            // SOUND2CNT_H low (0x6C): frequency low byte
            0x6C => {
                self.ch2_wave_length = (self.ch2_wave_length & 0xFF00) | value as u16;
            }

            // SOUND2CNT_H high (0x6D): frequency high + length enable + trigger
            0x6D => {
                let length_trigger = value & 0x40 != 0;
                let trigger = value & 0x80 != 0;
                let length_edge = length_trigger && !self.ch2_length_enabled;

                self.ch2_wave_length =
                    (self.ch2_wave_length & 0x00FF) | (((value & 0x07) as u16) << 8);
                self.ch2_length_enabled = length_trigger;

                if length_edge && self.sequencer_step % 2 == 1 && self.ch2_length_timer > 0 {
                    self.ch2_length_timer = self.ch2_length_timer.saturating_sub(1);
                    if self.ch2_length_timer == 0 {
                        self.ch2_enabled = false;
                    }
                }

                if trigger && self.ch2_dac {
                    self.ch2_enabled = true;
                    self.trigger_ch2();
                }

                if length_trigger && self.ch2_length_timer == 0 {
                    self.ch2_enabled = false;
                }
            }

            // -- channel 3: wave --

            // SOUND3CNT_L low (0x70): DAC enable + wave bank control
            0x70 => {
                self.wave_dimension = value & 0x20 != 0;
                self.wave_bank = (value >> 6) & 1;
                self.ch3_dac = value & 0x80 != 0;
                if !self.ch3_dac {
                    self.ch3_enabled = false;
                }
            }

            // SOUND3CNT_H low (0x72): length
            0x72 => {
                self.ch3_length_timer = 256 - value as u16;
            }

            // SOUND3CNT_H high (0x73): volume + force 75%
            0x73 => {
                self.ch3_output_level = (value >> 5) & 0x03;
                self.ch3_force_volume = value & 0x80 != 0;
            }

            // SOUND3CNT_X low (0x74): frequency low byte
            0x74 => {
                self.ch3_wave_length = (self.ch3_wave_length & 0xFF00) | value as u16;
            }

            // SOUND3CNT_X high (0x75): frequency high + length enable + trigger
            0x75 => {
                let length_trigger = value & 0x40 != 0;
                let trigger = value & 0x80 != 0;
                let length_edge = length_trigger && !self.ch3_length_enabled;

                self.ch3_wave_length =
                    (self.ch3_wave_length & 0x00FF) | (((value & 0x07) as u16) << 8);
                self.ch3_length_enabled = length_trigger;

                if length_edge && self.sequencer_step % 2 == 1 && self.ch3_length_timer > 0 {
                    self.ch3_length_timer = self.ch3_length_timer.saturating_sub(1);
                    if self.ch3_length_timer == 0 {
                        self.ch3_enabled = false;
                    }
                }

                if trigger && self.ch3_dac {
                    self.ch3_enabled = true;
                    self.trigger_ch3();
                }

                if length_trigger && self.ch3_length_timer == 0 {
                    self.ch3_enabled = false;
                }
            }

            // -- channel 4: noise --

            // SOUND4CNT_L low (0x78): length
            0x78 => {
                self.ch4_length_timer = 64 - (value & 0x3F);
            }

            // SOUND4CNT_L high (0x79): envelope parameters
            0x79 => {
                self.ch4_envelope_pace = value & 0x07;
                self.ch4_envelope_direction = (value >> 3) & 0x01;
                self.ch4_initial_volume = (value >> 4) & 0x0F;
                self.ch4_volume = self.ch4_initial_volume;
                self.ch4_envelope_enabled = self.ch4_envelope_pace > 0;
                self.ch4_envelope_sequence = 0;

                self.ch4_dac = value & 0xF8 != 0;
                if !self.ch4_dac {
                    self.ch4_enabled = false;
                }
            }

            // SOUND4CNT_H low (0x7C): polynomial counter
            0x7C => {
                self.ch4_divisor = value & 0x07;
                self.ch4_width_mode = value & 0x08 != 0;
                self.ch4_clock_shift = (value >> 4) & 0x0F;
            }

            // SOUND4CNT_H high (0x7D): control + trigger
            0x7D => {
                let length_trigger = value & 0x40 != 0;
                let trigger = value & 0x80 != 0;
                let length_edge = length_trigger && !self.ch4_length_enabled;

                self.ch4_length_enabled = length_trigger;

                if length_edge && self.sequencer_step % 2 == 1 && self.ch4_length_timer > 0 {
                    self.ch4_length_timer = self.ch4_length_timer.saturating_sub(1);
                    if self.ch4_length_timer == 0 {
                        self.ch4_enabled = false;
                    }
                }

                if trigger && self.ch4_dac {
                    self.ch4_enabled = true;
                    self.trigger_ch4();
                }

                if length_trigger && self.ch4_length_timer == 0 {
                    self.ch4_enabled = false;
                }
            }

            _ => {
                warnln!("Unhandled APU register write at offset 0x{:02X}", offset);
            }
        }
    }

    /// Resets the APU to its initial state.
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
    fn test_fifo_write_half() {
        let mut channel = DirectSoundChannel::new();

        // write_fifo_half should push exactly 2 samples
        channel.write_fifo_half(0x0201);
        channel.timer_tick();
        assert_eq!(channel.current_sample, 0x01);
        channel.timer_tick();
        assert_eq!(channel.current_sample, 0x02);

        // two half-writes should give 4 samples total (same as one full write)
        let mut ch2 = DirectSoundChannel::new();
        ch2.write_fifo_half(0x0201);
        ch2.write_fifo_half(0x0403);

        let mut ch3 = DirectSoundChannel::new();
        ch3.write_fifo(0x04030201);

        for _ in 0..4 {
            ch2.timer_tick();
            ch3.timer_tick();
            assert_eq!(ch2.current_sample, ch3.current_sample);
        }
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
        assert_eq!(apu.soundcnt_x() & 0x80, 0);
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
        // set envelope with DAC on (volume=0xF, direction=0, pace=0)
        apu.write_channel_reg(0x0400_0063, 0xF0);
        // trigger channel 1
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
        assert_eq!(apu.soundcnt_x() & 0x80, 0);
    }

    #[test]
    fn test_length_counter_disables_channel() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // set envelope with DAC on
        apu.write_channel_reg(0x0400_0063, 0xF0);
        // set length = 63 (timer will be 64-63=1)
        apu.write_channel_reg(0x0400_0062, 0x3F);
        // trigger with length enabled
        apu.write_channel_reg(0x0400_0065, 0xC0);

        // channel should be enabled
        assert!(apu.ch1_enabled);
        assert_eq!(apu.ch1_length_timer, 1);

        // clock enough to tick the frame sequencer at step 0 (length tick)
        // 512 Hz = CPU_FREQ/512 cycles per tick
        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        // advance to a length step (step 0)
        // first we need to reach step 0 in the sequencer
        apu.sequencer_step = 0; // step 0 dispatches length tick
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        // length counter should have ticked, disabling the channel
        assert!(!apu.ch1_enabled);
    }

    #[test]
    fn test_envelope_decreases_volume() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // set envelope: volume=15, direction=decrease(0), pace=1
        apu.write_channel_reg(0x0400_0063, 0xF1);
        // trigger channel 1
        apu.write_channel_reg(0x0400_0065, 0x80);

        assert_eq!(apu.ch1_volume, 15);

        // advance to step 7 (envelope tick)
        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        apu.sequencer_step = 7; // step 7 dispatches envelope tick
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        assert_eq!(apu.ch1_volume, 14);
    }

    #[test]
    fn test_envelope_increases_volume() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // set envelope: volume=5, direction=increase(1), pace=1
        apu.write_channel_reg(0x0400_0063, 0x59);
        // trigger channel 1
        apu.write_channel_reg(0x0400_0065, 0x80);

        assert_eq!(apu.ch1_volume, 5);

        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        apu.sequencer_step = 7; // step 7 dispatches envelope tick
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        assert_eq!(apu.ch1_volume, 6);
    }

    #[test]
    fn test_sweep_increases_frequency() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // sweep: pace=1, increase, slope=1
        apu.write_channel_reg(0x0400_0060, 0x11);
        // envelope with DAC on
        apu.write_channel_reg(0x0400_0063, 0xF0);
        // set frequency = 0x100
        apu.write_channel_reg(0x0400_0064, 0x00);
        apu.write_channel_reg(0x0400_0065, 0x81);

        assert_eq!(apu.ch1_sweep_shadow, 0x100);

        // sweep ticks at steps 2 and 6
        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        apu.sequencer_step = 2; // step 2 dispatches sweep + length
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        // frequency should have increased: 0x100 + (0x100 >> 1) = 0x100 + 0x80 = 0x180
        assert_eq!(apu.ch1_sweep_shadow, 0x180);
        assert_eq!(apu.ch1_wave_length, 0x180);
    }

    #[test]
    fn test_sweep_overflow_disables_channel() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // sweep: pace=1, increase, slope=0 (delta = shadow >> 0 = shadow)
        apu.write_channel_reg(0x0400_0060, 0x10);
        // envelope with DAC on
        apu.write_channel_reg(0x0400_0063, 0xF0);
        // set frequency near max = 0x700
        apu.write_channel_reg(0x0400_0064, 0x00);
        apu.write_channel_reg(0x0400_0065, 0x87);

        assert!(apu.ch1_enabled);

        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        apu.sequencer_step = 2; // step 2 dispatches sweep + length
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        // 0x700 + 0x700 = 0xE00 > 0x7FF, channel should be disabled
        assert!(!apu.ch1_enabled);
    }

    #[test]
    fn test_dac_gating_ch1() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // set envelope with DAC on, trigger
        apu.write_channel_reg(0x0400_0063, 0xF0);
        apu.write_channel_reg(0x0400_0065, 0x80);
        assert!(apu.ch1_enabled);

        // write envelope with DAC off (bits 3-7 = 0)
        apu.write_channel_reg(0x0400_0063, 0x00);
        assert!(!apu.ch1_enabled);
        assert!(!apu.ch1_dac);
    }

    #[test]
    fn test_dac_gating_ch3() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // enable DAC
        apu.write_channel_reg(0x0400_0070, 0x80);
        // trigger
        apu.write_channel_reg(0x0400_0075, 0x80);
        assert!(apu.ch3_enabled);

        // disable DAC
        apu.write_channel_reg(0x0400_0070, 0x00);
        assert!(!apu.ch3_enabled);
        assert!(!apu.ch3_dac);
    }

    #[test]
    fn test_stereo_panning() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // enable CH1 on left only (bit 12), full master volume
        apu.set_soundcnt_l(0x1077);
        // SOUNDCNT_H: legacy volume = 2 (100%)
        apu.set_soundcnt_h(0x0002);

        // set envelope with DAC on and max volume
        apu.write_channel_reg(0x0400_0063, 0xF0);
        // trigger channel 1 with a frequency
        apu.write_channel_reg(0x0400_0064, 0x00);
        apu.write_channel_reg(0x0400_0065, 0x83);

        // clock enough to generate a sample
        let sample_period = super::super::consts::CPU_FREQ / 32768;
        apu.clock(sample_period + 1);

        // should have at least 2 samples (left, right)
        assert!(apu.audio_buffer().len() >= 2);
    }

    #[test]
    fn test_channel_register_readback() {
        let mut apu = GbaApu::new();

        // write sweep: pace=3, decrease, slope=5
        apu.write_channel_reg(0x0400_0060, 0x3D);
        let readback = apu.read_channel_reg(0x0400_0060);
        assert_eq!(readback & 0x07, 5); // slope
        assert_ne!(readback & 0x08, 0); // decrease
        assert_eq!((readback >> 4) & 0x07, 3); // pace

        // write CH1 envelope: volume=0xA, increase, pace=3
        apu.write_channel_reg(0x0400_0063, 0xAB);
        let readback = apu.read_channel_reg(0x0400_0063);
        assert_eq!(readback & 0x07, 3); // pace
        assert_ne!(readback & 0x08, 0); // direction = increase
        assert_eq!((readback >> 4) & 0x0F, 0x0A); // volume
    }

    #[test]
    fn test_ch3_force_volume() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // enable CH3 DAC
        apu.write_channel_reg(0x0400_0070, 0x80);
        // set force 75% volume (bit 7 of 0x73)
        apu.write_channel_reg(0x0400_0073, 0x80);

        assert!(apu.ch3_force_volume);
        assert_eq!(apu.ch3_output_level, 0);

        // verify readback
        let readback = apu.read_channel_reg(0x0400_0073);
        assert_ne!(readback & 0x80, 0);
    }

    #[test]
    fn test_ch4_noise_trigger() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // set envelope with DAC on
        apu.write_channel_reg(0x0400_0079, 0xF0);
        // set polynomial: divisor=1, width=15-bit, shift=2
        apu.write_channel_reg(0x0400_007C, 0x21);
        // trigger
        apu.write_channel_reg(0x0400_007D, 0x80);

        assert!(apu.ch4_enabled);
        assert_eq!(apu.ch4_lfsr, 0x7FFF);
        assert_eq!(apu.ch4_divisor, 1);
        assert_eq!(apu.ch4_clock_shift, 2);
        assert!(!apu.ch4_width_mode);
    }

    #[test]
    fn test_wave_bank_selection() {
        let mut apu = GbaApu::new();

        // select bank 1 for CPU access
        apu.write_channel_reg(0x0400_0070, 0xC0);
        apu.write_wave_ram(0, 0xAB);

        // switch to bank 0 for CPU access
        apu.write_channel_reg(0x0400_0070, 0x80);
        apu.write_wave_ram(0, 0xCD);

        // verify different data in each bank
        apu.write_channel_reg(0x0400_0070, 0xC0);
        assert_eq!(apu.read_wave_ram(0), 0xAB);
        apu.write_channel_reg(0x0400_0070, 0x80);
        assert_eq!(apu.read_wave_ram(0), 0xCD);
    }

    // -- trigger tests for each channel --

    #[test]
    fn test_ch2_trigger_and_length() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // set envelope with DAC on
        apu.write_channel_reg(0x0400_0069, 0xF0);
        // set length = 63 (timer = 64-63 = 1)
        apu.write_channel_reg(0x0400_0068, 0x3F);
        // trigger with length enabled
        apu.write_channel_reg(0x0400_006D, 0xC0);

        assert!(apu.ch2_enabled);
        assert_eq!(apu.ch2_length_timer, 1);

        // tick length to disable channel
        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        apu.sequencer_step = 0;
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        assert!(!apu.ch2_enabled);
    }

    #[test]
    fn test_ch3_trigger_and_length() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // enable DAC
        apu.write_channel_reg(0x0400_0070, 0x80);
        // set length = 255 (timer = 256-255 = 1)
        apu.write_channel_reg(0x0400_0072, 0xFF);
        // trigger with length enabled
        apu.write_channel_reg(0x0400_0075, 0xC0);

        assert!(apu.ch3_enabled);
        assert_eq!(apu.ch3_length_timer, 1);

        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        apu.sequencer_step = 0;
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        assert!(!apu.ch3_enabled);
    }

    #[test]
    fn test_ch4_trigger_and_length() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // set envelope with DAC on
        apu.write_channel_reg(0x0400_0079, 0xF0);
        // set length = 63 (timer = 64-63 = 1)
        apu.write_channel_reg(0x0400_0078, 0x3F);
        // trigger with length enabled
        apu.write_channel_reg(0x0400_007D, 0xC0);

        assert!(apu.ch4_enabled);
        assert_eq!(apu.ch4_length_timer, 1);

        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        apu.sequencer_step = 0;
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        assert!(!apu.ch4_enabled);
    }

    // -- length counter edge cases --

    #[test]
    fn test_length_counter_does_not_tick_when_disabled() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // set envelope with DAC on
        apu.write_channel_reg(0x0400_0063, 0xF0);
        // set length = 63 (timer = 1)
        apu.write_channel_reg(0x0400_0062, 0x3F);
        // trigger WITHOUT length enabled (no bit 6)
        apu.write_channel_reg(0x0400_0065, 0x80);

        assert!(apu.ch1_enabled);
        assert!(!apu.ch1_length_enabled);

        // tick length step — should NOT disable channel
        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        apu.sequencer_step = 0;
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        assert!(apu.ch1_enabled);
    }

    #[test]
    fn test_length_counter_reloads_on_trigger() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // set envelope with DAC on
        apu.write_channel_reg(0x0400_0063, 0xF0);
        // set length = 0 (timer will be 64)
        apu.write_channel_reg(0x0400_0062, 0x00);
        // trigger with length enabled
        apu.write_channel_reg(0x0400_0065, 0xC0);

        // trigger should have reloaded timer to 64
        assert!(apu.ch1_enabled);
        assert!(apu.ch1_length_timer > 0);
    }

    // -- envelope edge cases --

    #[test]
    fn test_envelope_stops_at_zero() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // set envelope: volume=1, direction=decrease(0), pace=1
        apu.write_channel_reg(0x0400_0063, 0x11);
        // trigger
        apu.write_channel_reg(0x0400_0065, 0x80);

        assert_eq!(apu.ch1_volume, 1);

        // tick envelope
        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        apu.sequencer_step = 7;
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        assert_eq!(apu.ch1_volume, 0);
        assert!(!apu.ch1_envelope_enabled);

        // tick again — should stay at 0
        apu.sequencer_step = 7;
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        assert_eq!(apu.ch1_volume, 0);
    }

    #[test]
    fn test_envelope_stops_at_fifteen() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // set envelope: volume=14, direction=increase(1), pace=1
        apu.write_channel_reg(0x0400_0063, 0xE9);
        // trigger
        apu.write_channel_reg(0x0400_0065, 0x80);

        assert_eq!(apu.ch1_volume, 14);

        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        apu.sequencer_step = 7;
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        assert_eq!(apu.ch1_volume, 15);
        assert!(!apu.ch1_envelope_enabled);
    }

    #[test]
    fn test_envelope_pace_zero_disables() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // set envelope: volume=10, direction=decrease, pace=0 (disabled)
        apu.write_channel_reg(0x0400_0063, 0xA0);
        // trigger
        apu.write_channel_reg(0x0400_0065, 0x80);

        assert_eq!(apu.ch1_volume, 10);

        // tick envelope — should not change volume
        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        apu.sequencer_step = 7;
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        assert_eq!(apu.ch1_volume, 10);
    }

    #[test]
    fn test_ch4_envelope() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // set envelope: volume=8, direction=decrease, pace=1
        apu.write_channel_reg(0x0400_0079, 0x81);
        // trigger
        apu.write_channel_reg(0x0400_007D, 0x80);

        assert_eq!(apu.ch4_volume, 8);

        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        apu.sequencer_step = 7;
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        assert_eq!(apu.ch4_volume, 7);
    }

    // -- sweep edge cases --

    #[test]
    fn test_sweep_decrease() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // sweep: pace=1, decrease, slope=2
        apu.write_channel_reg(0x0400_0060, 0x1A);
        // envelope with DAC on
        apu.write_channel_reg(0x0400_0063, 0xF0);
        // set frequency = 0x200
        apu.write_channel_reg(0x0400_0064, 0x00);
        apu.write_channel_reg(0x0400_0065, 0x82);

        assert_eq!(apu.ch1_sweep_shadow, 0x200);

        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        apu.sequencer_step = 2;
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        // 0x200 - (0x200 >> 2) = 0x200 - 0x80 = 0x180
        assert_eq!(apu.ch1_sweep_shadow, 0x180);
    }

    #[test]
    fn test_sweep_pace_zero_no_sweep() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // sweep: pace=0 (disabled), increase, slope=1
        apu.write_channel_reg(0x0400_0060, 0x01);
        apu.write_channel_reg(0x0400_0063, 0xF0);
        apu.write_channel_reg(0x0400_0064, 0x00);
        apu.write_channel_reg(0x0400_0065, 0x82);

        let original = apu.ch1_sweep_shadow;

        let sequencer_period = super::super::consts::CPU_FREQ / 512;
        apu.sequencer_step = 2;
        apu.sequencer_counter = sequencer_period - 1;
        apu.clock(1);

        // frequency should not change with pace=0
        assert_eq!(apu.ch1_sweep_shadow, original);
    }

    // -- DAC gating for remaining channels --

    #[test]
    fn test_dac_gating_ch2() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        apu.write_channel_reg(0x0400_0069, 0xF0);
        apu.write_channel_reg(0x0400_006D, 0x80);
        assert!(apu.ch2_enabled);

        apu.write_channel_reg(0x0400_0069, 0x00);
        assert!(!apu.ch2_enabled);
        assert!(!apu.ch2_dac);
    }

    #[test]
    fn test_dac_gating_ch4() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        apu.write_channel_reg(0x0400_0079, 0xF0);
        apu.write_channel_reg(0x0400_007D, 0x80);
        assert!(apu.ch4_enabled);

        apu.write_channel_reg(0x0400_0079, 0x00);
        assert!(!apu.ch4_enabled);
        assert!(!apu.ch4_dac);
    }

    #[test]
    fn test_trigger_with_dac_off_does_not_enable() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // DAC off (envelope bits 3-7 = 0)
        apu.write_channel_reg(0x0400_0063, 0x00);
        // try to trigger
        apu.write_channel_reg(0x0400_0065, 0x80);

        // channel should not be enabled with DAC off
        assert!(!apu.ch1_enabled);
    }

    // -- register readback for all channels --

    #[test]
    fn test_ch2_register_readback() {
        let mut apu = GbaApu::new();

        // write CH2 envelope: volume=0xC, decrease, pace=5
        apu.write_channel_reg(0x0400_0069, 0xC5);
        let readback = apu.read_channel_reg(0x0400_0069);
        assert_eq!(readback & 0x07, 5);
        assert_eq!(readback & 0x08, 0); // decrease
        assert_eq!((readback >> 4) & 0x0F, 0x0C);
    }

    #[test]
    fn test_ch3_register_readback() {
        let mut apu = GbaApu::new();

        // enable DAC + dimension + bank 1
        apu.write_channel_reg(0x0400_0070, 0xE0);
        let readback = apu.read_channel_reg(0x0400_0070);
        assert_ne!(readback & 0x80, 0); // DAC on
        assert_ne!(readback & 0x40, 0); // bank 1
        assert_ne!(readback & 0x20, 0); // dimension
    }

    #[test]
    fn test_ch4_register_readback() {
        let mut apu = GbaApu::new();

        // set polynomial: divisor=3, width=7-bit, shift=5
        apu.write_channel_reg(0x0400_007C, 0x5B);
        let readback = apu.read_channel_reg(0x0400_007C);
        assert_eq!(readback & 0x07, 3); // divisor
        assert_ne!(readback & 0x08, 0); // width mode
        assert_eq!((readback >> 4) & 0x0F, 5); // shift
    }

    #[test]
    fn test_frequency_write_only_readback() {
        let apu = GbaApu::new();

        // frequency low bytes are write-only, read as 0xFF
        assert_eq!(apu.read_channel_reg(0x0400_0064), 0xFF);
        assert_eq!(apu.read_channel_reg(0x0400_006C), 0xFF);
        assert_eq!(apu.read_channel_reg(0x0400_0074), 0xFF);
        assert_eq!(apu.read_channel_reg(0x0400_0078), 0xFF);
    }

    // -- CH4 width mode --

    #[test]
    fn test_ch4_width_mode_7bit() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        apu.write_channel_reg(0x0400_0079, 0xF0);
        // width=7-bit (bit 3), divisor=0, shift=0
        apu.write_channel_reg(0x0400_007C, 0x08);
        apu.write_channel_reg(0x0400_007D, 0x80);

        assert!(apu.ch4_width_mode);
        assert_eq!(apu.ch4_lfsr, 0x7FFF);
    }

    // -- master disable --

    #[test]
    fn test_master_disable_stops_all_channels() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // enable all channels
        apu.write_channel_reg(0x0400_0063, 0xF0);
        apu.write_channel_reg(0x0400_0065, 0x80);
        apu.write_channel_reg(0x0400_0069, 0xF0);
        apu.write_channel_reg(0x0400_006D, 0x80);
        apu.write_channel_reg(0x0400_0070, 0x80);
        apu.write_channel_reg(0x0400_0075, 0x80);
        apu.write_channel_reg(0x0400_0079, 0xF0);
        apu.write_channel_reg(0x0400_007D, 0x80);

        assert!(apu.ch1_enabled);
        assert!(apu.ch2_enabled);
        assert!(apu.ch3_enabled);
        assert!(apu.ch4_enabled);

        // master disable
        apu.set_soundcnt_x(0x00);

        assert!(!apu.ch1_enabled);
        assert!(!apu.ch2_enabled);
        assert!(!apu.ch3_enabled);
        assert!(!apu.ch4_enabled);
    }

    // -- soundcnt_x status bits --

    #[test]
    fn test_soundcnt_x_status_bits() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // no channels enabled
        assert_eq!(apu.soundcnt_x() & 0x0F, 0);

        // enable CH1
        apu.write_channel_reg(0x0400_0063, 0xF0);
        apu.write_channel_reg(0x0400_0065, 0x80);
        assert_eq!(apu.soundcnt_x() & 0x01, 1);

        // enable CH2
        apu.write_channel_reg(0x0400_0069, 0xF0);
        apu.write_channel_reg(0x0400_006D, 0x80);
        assert_eq!(apu.soundcnt_x() & 0x02, 2);

        // enable CH3
        apu.write_channel_reg(0x0400_0070, 0x80);
        apu.write_channel_reg(0x0400_0075, 0x80);
        assert_eq!(apu.soundcnt_x() & 0x04, 4);

        // enable CH4
        apu.write_channel_reg(0x0400_0079, 0xF0);
        apu.write_channel_reg(0x0400_007D, 0x80);
        assert_eq!(apu.soundcnt_x() & 0x08, 8);

        // all four + master + unused bits
        assert_eq!(apu.soundcnt_x(), 0x8F | 0x70);
    }

    // -- wave dimension mode --

    #[test]
    fn test_wave_dimension_mode() {
        let mut apu = GbaApu::new();

        // enable dimension mode (bit 5)
        apu.write_channel_reg(0x0400_0070, 0xA0);
        assert!(apu.wave_dimension);

        // in dimension mode, CPU accesses the non-playing bank
        // bank=0, so CPU accesses bank 1
        apu.write_channel_reg(0x0400_0070, 0xA0); // bank=0, dim=1
        apu.write_wave_ram(0, 0x12);

        apu.write_channel_reg(0x0400_0070, 0xE0); // bank=1, dim=1
        apu.write_wave_ram(0, 0x34);

        // verify they are in different banks
        apu.write_channel_reg(0x0400_0070, 0xA0);
        assert_eq!(apu.read_wave_ram(0), 0x12);
        apu.write_channel_reg(0x0400_0070, 0xE0);
        assert_eq!(apu.read_wave_ram(0), 0x34);
    }

    // -- high-pass filter --

    // -- FIFO empty behavior --

    #[test]
    fn test_fifo_empty_keeps_last_sample() {
        let mut channel = DirectSoundChannel::new();
        channel.write_fifo(0x42424242);

        // consume all 4 samples
        for _ in 0..4 {
            channel.timer_tick();
            assert_eq!(channel.current_sample, 0x42);
        }

        // tick again with empty FIFO — should keep last sample
        channel.timer_tick();
        assert_eq!(channel.current_sample, 0x42);
    }

    // -- clock generates correct number of samples --

    #[test]
    fn test_clock_sample_generation_rate() {
        let mut apu = GbaApu::new();
        apu.set_soundcnt_x(0x80);

        // clock for exactly one frame (280896 cycles)
        // at 32768 Hz and 16.78 MHz, expect ~547 samples per frame
        // each sample is 2 entries (stereo)
        apu.clock(super::super::consts::CYCLES_PER_FRAME);

        let sample_count = apu.audio_buffer().len() / 2;
        // should be approximately 547 stereo samples
        assert!(sample_count > 540);
        assert!(sample_count < 560);
    }
}
