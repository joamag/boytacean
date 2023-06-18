use image::io::Reader as ImageReader;

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

#[cfg(test)]
mod tests {
    use boytacean::{
        gb::GameBoyMode,
        ppu::FRAME_BUFFER_SIZE,
        test::{run_image_test, TestOptions},
    };

    use super::compare_images;

    #[test]
    fn test_blargg_cpu_instrs() {
        let result: [u8; FRAME_BUFFER_SIZE] = run_image_test(
            "../../res/roms/test/blargg/cpu/cpu_instrs.gb",
            Some(300000000),
            TestOptions::default(),
        );
        let image_result = compare_images(&result, "res/test/blargg/cpu/cpu_instrs.png");
        assert_eq!(image_result, true);
    }

    #[test]
    fn test_dmg_acid2() {
        let result: [u8; FRAME_BUFFER_SIZE] = run_image_test(
            "../../res/roms/test/dmg_acid2.gb",
            Some(50000000),
            TestOptions::default(),
        );
        let image_result = compare_images(&result, "res/test/dmg_acid2.png");
        assert_eq!(image_result, true);
    }

    #[test]
    fn test_cgb_acid2() {
        let result: [u8; FRAME_BUFFER_SIZE] = run_image_test(
            "../../res/roms/test/cgb_acid2.gbc",
            Some(50000000),
            TestOptions {
                mode: Some(GameBoyMode::Cgb),
                ..Default::default()
            },
        );
        let image_result = compare_images(&result, "res/test/cgb_acid2.png");
        assert_eq!(image_result, true);
    }

    #[test]
    fn test_firstwhite() {
        let result: [u8; FRAME_BUFFER_SIZE] = run_image_test(
            "../../res/roms/test/firstwhite.gb",
            Some(50000000),
            TestOptions::default(),
        );
        let image_result = compare_images(&result, "res/test/firstwhite.png");
        assert_eq!(image_result, true);
    }
}
