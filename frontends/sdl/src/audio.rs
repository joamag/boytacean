use sdl2::{
    audio::{AudioCallback, AudioDevice, AudioSpecDesired},
    AudioSubsystem, Sdl,
};

pub struct AudioWave {
    phase_inc: f32,

    phase: f32,

    volume: f32,

    /// The relative amount of time (as a percentage decimal) the low level
    /// is going to be present during a period (cycle).
    /// From [Wikipedia](https://en.wikipedia.org/wiki/Duty_cycle).
    duty_cycle: f32,
}

impl AudioCallback for AudioWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = if self.phase < (1.0 - self.duty_cycle) {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

pub struct Audio {
    pub device: AudioDevice<AudioWave>,
    pub audio_subsystem: AudioSubsystem,
}

impl Audio {
    pub fn new(sdl: &Sdl) -> Self {
        let audio_subsystem = sdl.audio().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1),
            samples: None,
        };

        let device = audio_subsystem
            .open_playback(None, &desired_spec, |spec| AudioWave {
                phase_inc: 440.0 / spec.freq as f32,
                phase: 0.0,
                volume: 0.25,
                duty_cycle: 0.5,
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
