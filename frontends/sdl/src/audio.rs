use boytacean::gb::{AudioProvider, GameBoy};
use sdl2::{
    audio::{AudioCallback, AudioDevice, AudioSpec, AudioSpecDesired},
    AudioSubsystem, Sdl,
};
use std::sync::{Arc, Mutex};

pub struct AudioWave {
    /// Specification of the audion settings that have been put in place
    /// for the playing of this audio wave.
    spec: AudioSpec,

    /// The object that is going to be used as the provider of the audio
    /// operation.
    audio_provider: Arc<Mutex<Box<GameBoy>>>,

    /// The number of audio ticks that have passed since the beginning
    /// of the audio playback, the value wraps around (avoids overflow).
    ticks: usize,
}

impl AudioCallback for AudioWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        self.ticks = self.ticks.wrapping_add(out.len() as usize);

        for x in out.iter_mut() {
            *x = match self.audio_provider.lock() {
                Ok(mut provider) => {
                    let value = provider.output_clock_apu(1, self.spec.freq as u32) as f32 / 7.0;
                    value
                }
                Err(_) => 0.0,
            }
        }
    }
}

pub struct Audio {
    pub device: AudioDevice<AudioWave>,
    pub audio_subsystem: AudioSubsystem,
}

impl Audio {
    pub fn new(sdl: &Sdl, audio_provider: Arc<Mutex<Box<GameBoy>>>) -> Self {
        let audio_subsystem = sdl.audio().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1),
            samples: Some(2),
        };

        let device = audio_subsystem
            .open_playback(None, &desired_spec, |spec| AudioWave {
                spec: spec,
                audio_provider: audio_provider,
                ticks: 0,
            })
            .unwrap();

        // starts the playback by resuming the audio
        // device's activity
        device.resume();

        Self {
            device,
            audio_subsystem,
        }
    }
}
