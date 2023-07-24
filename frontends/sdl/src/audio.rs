use sdl2::{
    audio::{AudioQueue, AudioSpecDesired},
    AudioSubsystem, Sdl,
};

pub struct Audio {
    pub device: AudioQueue<f32>,
    pub audio_subsystem: AudioSubsystem,
}

impl Audio {
    pub fn new(sdl: &Sdl, freq: i32, channels: u8, samples: Option<u16>) -> Self {
        let audio_subsystem = sdl.audio().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(freq),
            channels: Some(channels),
            samples: Some(samples.unwrap_or(4096)),
        };

        // creates the queue that is going to be used to update the
        // audio stream with new values during the main loop
        let device = audio_subsystem.open_queue(None, &desired_spec).unwrap();

        // starts the playback by resuming the audio
        // device's activity
        device.resume();

        Self {
            device,
            audio_subsystem,
        }
    }
}
