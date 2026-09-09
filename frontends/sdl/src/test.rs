use image::{io::Reader as ImageReader, ImageBuffer, Rgb};

pub fn compare_images(source_pixels: &[u8], target_path: &str) -> bool {
    let image_buffer = ImageReader::open(target_path)
        .unwrap()
        .decode()
        .unwrap()
        .to_rgb8();
    let (width, _) = image_buffer.dimensions();

    for (x, y, pixel) in image_buffer.enumerate_pixels() {
        let base = ((y * width + x) * 3) as usize;
        if [
            source_pixels[base],
            source_pixels[base + 1],
            source_pixels[base + 2],
        ] != pixel.0
        {
            return false;
        }
    }

    true
}

pub fn save_image(pixels: &[u8], width: u32, height: u32, file_path: &str) {
    let mut image_buffer: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);

    for (x, y, pixel) in image_buffer.enumerate_pixels_mut() {
        let base = ((y * width + x) * 3) as usize;
        *pixel = Rgb([pixels[base], pixels[base + 1], pixels[base + 2]]);
    }

    image_buffer
        .save_with_format(file_path, image::ImageFormat::Png)
        .unwrap();
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        ffi::CString,
        fs::{create_dir, read, remove_dir, remove_file, write},
        sync::Mutex,
    };

    use boytacean::{
        gb::{GameBoy, GameBoyMode},
        gba::GameBoyAdvance,
        gba_test::{run_gba_image_test, GbaTestOptions},
        system::System,
        test::{run_image_test, TestOptions},
    };

    use super::{compare_images, save_image};
    use crate::{Emulator, EmulatorOptions, VOLUME, VOLUME_GBA};

    static EMULATOR_LOCK: Mutex<()> = Mutex::new(());

    fn make_emulator(system: System) -> Emulator {
        Emulator::new(
            system,
            EmulatorOptions {
                auto_mode: Some(true),
                unlimited: Some(false),
                opengl: Some(false),
                features: Some(vec![]),
            },
        )
    }

    #[test]
    fn test_emulator_stop_saves_ram() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        let path = temp_dir().join("boytacean_sdl_stop_test.sav");
        let mut gba = GameBoyAdvance::new();
        gba.cpu.bus.save.detect_save_type(b"FLASH1M_V");
        gba.cpu.bus.save.data[0x1FFFF] = 0x42;
        let mut emulator = make_emulator(System::Gba(gba));
        emulator.ram_path = path.to_str().unwrap().to_string();
        emulator.stop();
        let data = read(&path).unwrap();
        assert_eq!(data.len(), 0x20000);
        assert_eq!(data[0x1FFFF], 0x42);
        remove_file(path).unwrap();
        emulator.ram_path = temp_dir().to_str().unwrap().to_string();
        emulator.stop();
    }

    #[test]
    fn test_emulator_switch_rom() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        let mut emulator = make_emulator(System::Gb(GameBoy::new(None)));
        emulator.system.set_all_enabled(false);
        emulator
            .load_rom(Some("../../res/roms/demo/pocket.gb"))
            .unwrap();
        let gba_path = "../../res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba";
        emulator.switch_rom(gba_path).unwrap();
        assert!(emulator.system.is_gba());
        assert_eq!(emulator.system.display_width(), 240);
        assert_eq!(emulator.system.frame_buffer().len(), 240 * 160 * 3);
        assert_eq!(emulator.logic_frequency, GameBoyAdvance::CPU_FREQ);
        assert_eq!(emulator.visual_frequency, GameBoyAdvance::VISUAL_FREQ);
        assert_eq!(emulator.volume, VOLUME_GBA);
        assert_eq!(emulator.rom_path, gba_path);
        assert!(!emulator.system.ppu_enabled());
        assert!(!emulator.system.apu_enabled());
        assert!(!emulator.system.dma_enabled());
        assert!(!emulator.system.timer_enabled());
        emulator.switch_rom(gba_path).unwrap();
        assert!(emulator.system.is_gba());
        emulator.next_tick_time = 100.0;
        emulator.next_tick_time_i = 100;
        emulator
            .switch_rom("../../res/roms/test/cgb_acid2.gbc")
            .unwrap();
        assert!(emulator.system.is_gb());
        assert_eq!(emulator.system.display_width(), 160);
        assert_eq!(emulator.system.frame_buffer().len(), 160 * 144 * 3);
        assert_eq!(emulator.logic_frequency, GameBoy::CPU_FREQ);
        assert_eq!(emulator.volume, VOLUME);
        assert_eq!(emulator.next_tick_time, 0.0);
        assert_eq!(emulator.next_tick_time_i, 0);
        if let System::Gb(gb) = &emulator.system {
            assert_eq!(gb.mode(), GameBoyMode::Cgb);
        }
        emulator
            .switch_rom("../../res/roms/demo/pocket.gb")
            .unwrap();
        if let System::Gb(gb) = &emulator.system {
            assert_eq!(gb.mode(), GameBoyMode::Dmg);
        }
        emulator.stop();
    }

    #[test]
    fn test_emulator_switch_rom_explicit_mode() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        let mut emulator = make_emulator(System::Gba(GameBoyAdvance::new()));
        emulator.auto_mode = false;
        emulator.gb_mode = GameBoyMode::Cgb;
        emulator
            .switch_rom("../../res/roms/demo/pocket.gb")
            .unwrap();
        if let System::Gb(gb) = &emulator.system {
            assert_eq!(gb.mode(), GameBoyMode::Cgb);
        } else {
            panic!("Expected Game Boy system");
        }
        emulator
            .switch_rom("../../res/roms/demo/pocket.gb")
            .unwrap();
        if let System::Gb(gb) = &emulator.system {
            assert_eq!(gb.mode(), GameBoyMode::Cgb);
        }
        emulator.stop();
    }

    #[test]
    fn test_emulator_switch_rom_errors() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        let mut emulator = make_emulator(System::Gb(GameBoy::new(None)));
        emulator
            .load_rom(Some("../../res/roms/demo/pocket.gb"))
            .unwrap();
        assert!(emulator.switch_rom(temp_dir().to_str().unwrap()).is_err());
        let path = temp_dir().join("boytacean_sdl_invalid_rom_test.gba");
        write(&path, [0]).unwrap();
        assert!(emulator.switch_rom(path.to_str().unwrap()).is_err());
        assert!(emulator.system.is_gb());
        assert_eq!(emulator.rom_path, "../../res/roms/demo/pocket.gb");
        remove_file(path).unwrap();

        let path = temp_dir().join("boytacean_sdl_invalid_ram_test.gba");
        let ram_path = path.with_extension("sav");
        write(
            &path,
            include_bytes!("../../../res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba"),
        )
        .unwrap();
        create_dir(&ram_path).unwrap();
        assert!(emulator.switch_rom(path.to_str().unwrap()).is_err());
        assert!(emulator.system.is_gb());
        assert_eq!(emulator.rom_path, "../../res/roms/demo/pocket.gb");
        remove_dir(ram_path).unwrap();
        remove_file(path).unwrap();
        emulator.stop();
    }

    #[test]
    fn test_emulator_switch_rom_preserves_bios() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        let mut gba = GameBoyAdvance::new();
        gba.load_bios(&vec![0xAB; 0x4000]);
        let mut emulator = make_emulator(System::Gba(gba));
        emulator
            .switch_rom("../../res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba")
            .unwrap();
        if let System::Gba(gba) = &emulator.system {
            assert!(gba.cpu.bus.use_real_bios);
            assert_eq!(gba.cpu.pc(), 0);
            assert_eq!(gba.cpu.bus.bios.as_slice(), &[0xAB; 0x4000]);
        } else {
            panic!("Expected Game Boy Advance system");
        }
        emulator.stop();
    }

    #[test]
    fn test_emulator_run_switches_display_and_audio() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        let video_driver = sdl2::hint::get("SDL_VIDEODRIVER").unwrap_or_default();
        let audio_driver = sdl2::hint::get("SDL_AUDIODRIVER").unwrap_or_default();
        sdl2::hint::set("SDL_VIDEODRIVER", "dummy");
        sdl2::hint::set("SDL_AUDIODRIVER", "dummy");
        let sdl = sdl2::init().unwrap();
        let mut emulator = make_emulator(System::Gb(GameBoy::new(None)));
        emulator.features = vec!["video", "audio", "no-accelerated", "no-vsync"];
        emulator.start_graphics(&sdl, 1.0, false);
        emulator.start_audio(&sdl);
        let gba_path = "../../res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba";
        emulator.switch_rom(gba_path).unwrap();
        assert_eq!(
            emulator
                .sdl
                .as_ref()
                .unwrap()
                .canvas
                .as_ref()
                .unwrap()
                .logical_size(),
            (240, 160)
        );
        assert_eq!(emulator.audio.as_ref().unwrap().device.spec().freq, 32768);
        emulator
            .audio
            .as_ref()
            .unwrap()
            .device
            .queue_audio(&[0.5; 2048])
            .unwrap();
        emulator.switch_rom(gba_path).unwrap();
        assert_eq!(emulator.audio.as_ref().unwrap().device.size(), 0);

        let path = temp_dir().join("boytacean_sdl_periodic_test.sav");
        if let System::Gba(gba) = &mut emulator.system {
            gba.cpu.bus.save.detect_save_type(b"SRAM_V");
            gba.cpu.bus.save.data[0] = 0x42;
        }
        emulator.ram_path = path.to_str().unwrap().to_string();
        emulator.visual_frequency = 0.2;
        let events = sdl.event().unwrap();
        for filename in [
            "../../res/roms/demo/pocket.gb",
            temp_dir().to_str().unwrap(),
        ] {
            let filename = CString::new(filename).unwrap();
            // sdl owns the filename after the drop event is queued
            unsafe {
                let mut event = sdl2::sys::SDL_Event {
                    drop: sdl2::sys::SDL_DropEvent {
                        type_: sdl2::sys::SDL_EventType::SDL_DROPFILE as u32,
                        timestamp: 0,
                        file: sdl2::sys::SDL_strdup(filename.as_ptr()),
                        windowID: emulator.sdl.as_ref().unwrap().window().id(),
                    },
                };
                assert!(!event.drop.file.is_null());
                assert_eq!(sdl2::sys::SDL_PushEvent(&mut event), 1);
            }
        }
        events
            .push_event(sdl2::event::Event::Quit { timestamp: 0 })
            .unwrap();
        emulator.run();
        assert!(emulator.system.is_gb());
        assert_eq!(
            emulator
                .sdl
                .as_ref()
                .unwrap()
                .canvas
                .as_ref()
                .unwrap()
                .logical_size(),
            (160, 144)
        );
        assert_eq!(
            emulator.audio.as_ref().unwrap().device.spec().freq,
            emulator.system.audio_sampling_rate() as i32
        );
        assert_eq!(read(&path).unwrap()[0], 0x42);
        remove_file(path).unwrap();
        emulator.stop();
        drop(emulator);
        drop(events);
        drop(sdl);
        sdl2::hint::set("SDL_VIDEODRIVER", &video_driver);
        sdl2::hint::set("SDL_AUDIODRIVER", &audio_driver);
    }

    #[test]
    fn test_blargg_cpu_instrs() {
        let (result, _) = run_image_test(
            "../../res/roms/test/blargg/cpu/cpu_instrs.gb",
            Some(300000000),
            TestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/blargg/cpu/cpu_instrs.png");
        assert!(image_result);
    }

    #[test]
    fn test_blargg_instr_timing() {
        let (result, _) = run_image_test(
            "../../res/roms/test/blargg/instr_timing/instr_timing.gb",
            Some(50000000),
            TestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/blargg/instr_timing/instr_timing.png");
        assert!(image_result);
    }

    #[test]
    fn test_blargg_interrupt_time() {
        let (result, _) = run_image_test(
            "../../res/roms/test/blargg/interrupt_time/interrupt_time.gb",
            Some(20000000),
            TestOptions {
                mode: Some(GameBoyMode::Cgb),
                ..TestOptions::default()
            },
        )
        .unwrap();
        let image_result =
            compare_images(&result, "res/test/blargg/interrupt_time/interrupt_time.png");
        assert!(image_result);
    }

    #[test]
    fn test_blargg_dmg_sound() {
        let (result, _) = run_image_test(
            "../../res/roms/test/blargg/dmg_sound/01-registers.gb",
            Some(50000000),
            TestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/blargg/dmg_sound/01-registers.png");
        assert!(image_result);

        let (result, _) = run_image_test(
            "../../res/roms/test/blargg/dmg_sound/02-len ctr.gb",
            Some(50000000),
            TestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/blargg/dmg_sound/02-len ctr.png");
        assert!(image_result);

        let (result, _) = run_image_test(
            "../../res/roms/test/blargg/dmg_sound/03-trigger.gb",
            Some(100000000),
            TestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/blargg/dmg_sound/03-trigger.png");
        assert!(image_result);
    }

    #[test]
    fn test_dmg_acid2() {
        let (result, _) = run_image_test(
            "../../res/roms/test/dmg_acid2.gb",
            Some(50000000),
            TestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/dmg_acid2.png");
        assert!(image_result);
    }

    #[test]
    fn test_cgb_acid2() {
        let (result, _) = run_image_test(
            "../../res/roms/test/cgb_acid2.gbc",
            Some(50000000),
            TestOptions {
                mode: Some(GameBoyMode::Cgb),
                ..Default::default()
            },
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/cgb_acid2.png");
        assert!(image_result);
    }

    #[test]
    fn test_firstwhite() {
        let (result, _) = run_image_test(
            "../../res/roms/test/firstwhite.gb",
            Some(50000000),
            TestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/firstwhite.png");
        assert!(image_result);
    }

    #[test]
    fn test_gba_jsmolka_arm() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/jsmolka_gba-tests/arm.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/gba/jsmolka_gba-tests/arm.png");
        assert!(image_result);
    }

    #[test]
    fn test_gba_jsmolka_memory() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/jsmolka_gba-tests/memory.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/gba/jsmolka_gba-tests/memory.png");
        assert!(image_result);
    }

    #[test]
    fn test_gba_jsmolka_bios() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/jsmolka_gba-tests/bios.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/gba/jsmolka_gba-tests/bios.png");
        assert!(image_result);
    }

    #[test]
    fn test_gba_jsmolka_sram() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/jsmolka_gba-tests/sram.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/gba/jsmolka_gba-tests/sram.png");
        assert!(image_result);
    }

    #[test]
    fn test_gba_jsmolka_flash64() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/jsmolka_gba-tests/flash64.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/gba/jsmolka_gba-tests/flash64.png");
        assert!(image_result);
    }

    #[test]
    fn test_gba_jsmolka_nes() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/jsmolka_gba-tests/nes.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/gba/jsmolka_gba-tests/nes.png");
        assert!(image_result);
    }

    #[test]
    fn test_gba_jsmolka_thumb() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/jsmolka_gba-tests/thumb.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/gba/jsmolka_gba-tests/thumb.png");
        assert!(image_result);
    }

    #[test]
    fn test_gba_jsmolka_flash128() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/jsmolka_gba-tests/flash128.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/gba/jsmolka_gba-tests/flash128.png");
        assert!(image_result);
    }

    #[test]
    fn test_gba_jsmolka_none() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/jsmolka_gba-tests/none.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/gba/jsmolka_gba-tests/none.png");
        assert!(image_result);
    }

    #[test]
    fn test_gba_jsmolka_unsafe() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/jsmolka_gba-tests/unsafe.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/gba/jsmolka_gba-tests/unsafe.png");
        assert!(image_result);
    }

    #[test]
    fn test_gba_jsmolka_ppu_hello() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/jsmolka_gba-tests/ppu_hello.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/gba/jsmolka_gba-tests/ppu_hello.png");
        assert!(image_result);
    }

    #[test]
    fn test_gba_jsmolka_ppu_shades() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/jsmolka_gba-tests/ppu_shades.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(&result, "res/test/gba/jsmolka_gba-tests/ppu_shades.png");
        assert!(image_result);
    }

    #[test]
    fn test_gba_jsmolka_ppu_stripes() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result =
            compare_images(&result, "res/test/gba/jsmolka_gba-tests/ppu_stripes.png");
        assert!(image_result);
    }

    #[test]
    #[ignore]
    fn test_gba_alyosha_dma_rom_fixed() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/alyosha-tas_gba-tests/DMA/DMA_ROM_Fixed.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(
            &result,
            "res/test/gba/alyosha-tas_gba-tests/DMA/DMA_ROM_Fixed.png",
        );
        assert!(image_result);
    }

    #[test]
    #[ignore]
    fn test_gba_alyosha_dma_mode_change() {
        let (result, _) = run_gba_image_test(
            "../../res/roms.gba/test/alyosha-tas_gba-tests/DMA/DMA_Mode_Change.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        let image_result = compare_images(
            &result,
            "res/test/gba/alyosha-tas_gba-tests/DMA/DMA_Mode_Change.png",
        );
        assert!(image_result);
    }

    #[test]
    #[ignore]
    fn generate_gba_reference_images() {
        let tests: &[(&str, &str)] = &[
            (
                "../../res/roms.gba/test/jsmolka_gba-tests/arm.gba",
                "res/test/gba/jsmolka_gba-tests/arm.png",
            ),
            (
                "../../res/roms.gba/test/jsmolka_gba-tests/memory.gba",
                "res/test/gba/jsmolka_gba-tests/memory.png",
            ),
            (
                "../../res/roms.gba/test/jsmolka_gba-tests/bios.gba",
                "res/test/gba/jsmolka_gba-tests/bios.png",
            ),
            (
                "../../res/roms.gba/test/jsmolka_gba-tests/sram.gba",
                "res/test/gba/jsmolka_gba-tests/sram.png",
            ),
            (
                "../../res/roms.gba/test/jsmolka_gba-tests/flash64.gba",
                "res/test/gba/jsmolka_gba-tests/flash64.png",
            ),
            (
                "../../res/roms.gba/test/jsmolka_gba-tests/nes.gba",
                "res/test/gba/jsmolka_gba-tests/nes.png",
            ),
            (
                "../../res/roms.gba/test/jsmolka_gba-tests/thumb.gba",
                "res/test/gba/jsmolka_gba-tests/thumb.png",
            ),
            (
                "../../res/roms.gba/test/jsmolka_gba-tests/flash128.gba",
                "res/test/gba/jsmolka_gba-tests/flash128.png",
            ),
            (
                "../../res/roms.gba/test/jsmolka_gba-tests/none.gba",
                "res/test/gba/jsmolka_gba-tests/none.png",
            ),
            (
                "../../res/roms.gba/test/jsmolka_gba-tests/unsafe.gba",
                "res/test/gba/jsmolka_gba-tests/unsafe.png",
            ),
            (
                "../../res/roms.gba/test/jsmolka_gba-tests/ppu_hello.gba",
                "res/test/gba/jsmolka_gba-tests/ppu_hello.png",
            ),
            (
                "../../res/roms.gba/test/jsmolka_gba-tests/ppu_shades.gba",
                "res/test/gba/jsmolka_gba-tests/ppu_shades.png",
            ),
            (
                "../../res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba",
                "res/test/gba/jsmolka_gba-tests/ppu_stripes.png",
            ),
            (
                "../../res/roms.gba/test/alyosha-tas_gba-tests/DMA/DMA_ROM_Fixed.gba",
                "res/test/gba/alyosha-tas_gba-tests/DMA/DMA_ROM_Fixed.png",
            ),
            (
                "../../res/roms.gba/test/alyosha-tas_gba-tests/DMA/DMA_Mode_Change.gba",
                "res/test/gba/alyosha-tas_gba-tests/DMA/DMA_Mode_Change.png",
            ),
        ];
        for (rom, out) in tests {
            let (fb, _) =
                run_gba_image_test(rom, Some(100000000), GbaTestOptions::default()).unwrap();
            save_image(&fb, 240, 160, out);
            println!("Generated {}", out);
        }
    }
}
