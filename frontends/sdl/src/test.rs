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
    use boytacean::{
        gb::GameBoyMode,
        test::{run_image_test, TestOptions},
    };

    use super::compare_images;

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
}
