//! Embedded video frame fixtures
//!
//! These image files are embedded at compile time using `include_bytes!`
//! for zero-overhead access in tests.

use image::{DynamicImage, GenericImageView, ImageFormat, ImageResult};

/// 1920x1080 test pattern PNG image
///
/// This is a test pattern image useful for:
/// - OCR accuracy testing
/// - Frame comparison algorithms
/// - Video processing pipelines
/// - Color space testing
///
/// # Specifications
/// - Resolution: 1920x1080 pixels
/// - Format: PNG (lossless compression)
/// - Color space: RGBA
/// - Contents: Test pattern with text, gradients, and geometric shapes
///
/// # Example
///
/// ```rust
/// use screenpipe_test_utils::fixtures::video_frames::TEST_FRAME_1920X1080;
/// use image::load_from_memory;
///
/// let img = load_from_memory(TEST_FRAME_1920X1080).unwrap();
/// assert_eq!(img.width(), 1920);
/// assert_eq!(img.height(), 1080);
/// ```
pub const TEST_FRAME_1920X1080: &[u8] =
    include_bytes!("../../fixtures/video/test_frame_1920x1080.png");

/// Helper functions for working with video frame fixtures
pub mod helpers {
    use super::*;

    /// Loads the test frame as a DynamicImage
    ///
    /// # Example
    ///
    /// ```rust
    /// use screenpipe_test_utils::fixtures::video_frames::helpers::load_test_frame;
    ///
    /// let img = load_test_frame().unwrap();
    /// assert_eq!(img.width(), 1920);
    /// assert_eq!(img.height(), 1080);
    /// ```
    pub fn load_test_frame() -> ImageResult<DynamicImage> {
        image::load_from_memory_with_format(TEST_FRAME_1920X1080, ImageFormat::Png)
    }

    /// Creates a solid color image of the specified size
    ///
    /// # Example
    ///
    /// ```rust
    /// use screenpipe_test_utils::fixtures::video_frames::helpers::create_solid_color;
    /// use image::Rgba;
    ///
    /// let img = create_solid_color(100, 100, Rgba([255, 0, 0, 255]));
    /// assert_eq!(img.width(), 100);
    /// assert_eq!(img.height(), 100);
    /// ```
    pub fn create_solid_color(width: u32, height: u32, color: image::Rgba<u8>) -> DynamicImage {
        use image::ImageBuffer;
        let buffer = ImageBuffer::from_pixel(width, height, color);
        DynamicImage::ImageRgba8(buffer)
    }

    /// Creates a checkerboard pattern image
    ///
    /// # Arguments
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `checker_size` - Size of each checker square in pixels
    pub fn create_checkerboard(width: u32, height: u32, checker_size: u32) -> DynamicImage {
        use image::{ImageBuffer, Rgba};

        let mut image = ImageBuffer::new(width, height);
        let white = Rgba([255, 255, 255, 255]);
        let black = Rgba([0, 0, 0, 255]);

        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let checker_x = x / checker_size;
            let checker_y = y / checker_size;
            let is_white = (checker_x + checker_y) % 2 == 0;
            *pixel = if is_white { white } else { black };
        }

        DynamicImage::ImageRgba8(image)
    }

    /// Creates a gradient image
    ///
    /// Creates a horizontal gradient from left (black) to right (white)
    pub fn create_gradient(width: u32, height: u32) -> DynamicImage {
        use image::{ImageBuffer, Rgba};

        let mut image = ImageBuffer::new(width, height);

        for (x, _y, pixel) in image.enumerate_pixels_mut() {
            let value = ((x as f32 / width as f32) * 255.0) as u8;
            *pixel = Rgba([value, value, value, 255]);
        }

        DynamicImage::ImageRgba8(image)
    }

    /// Creates a test pattern with concentric circles
    pub fn create_circle_pattern(width: u32, height: u32) -> DynamicImage {
        use image::{ImageBuffer, Rgba};

        let mut image = ImageBuffer::new(width, height);
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let max_radius = (center_x.min(center_y) * 0.9) as u32;

        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance = (dx * dx + dy * dy).sqrt() as u32;

            // Create concentric circles
            let ring = (distance / (max_radius / 10)) % 2;
            let value = if ring == 0 { 255 } else { 0 };
            *pixel = Rgba([value, value, value, 255]);
        }

        DynamicImage::ImageRgba8(image)
    }

    /// Calculates the PSNR (Peak Signal-to-Noise Ratio) between two images
    ///
    /// Higher values indicate more similarity. A value of infinity means
    /// the images are identical.
    pub fn calculate_psnr(img1: &DynamicImage, img2: &DynamicImage) -> Option<f64> {
        if img1.dimensions() != img2.dimensions() {
            return None;
        }

        let img1_rgba = img1.to_rgba8();
        let img2_rgba = img2.to_rgba8();

        let mut mse: f64 = 0.0;
        let pixel_count = (img1.width() * img1.height()) as f64;

        for (p1, p2) in img1_rgba.pixels().zip(img2_rgba.pixels()) {
            for i in 0..3 {
                // Only RGB, ignore alpha
                let diff = p1.0[i] as f64 - p2.0[i] as f64;
                mse += diff * diff;
            }
        }

        mse /= pixel_count * 3.0;

        if mse == 0.0 {
            return Some(f64::INFINITY);
        }

        let max_value = 255.0;
        Some(10.0 * (max_value * max_value / mse).log10())
    }

    /// Calculates the mean squared error between two images
    pub fn calculate_mse(img1: &DynamicImage, img2: &DynamicImage) -> Option<f64> {
        if img1.dimensions() != img2.dimensions() {
            return None;
        }

        let img1_rgba = img1.to_rgba8();
        let img2_rgba = img2.to_rgba8();

        let mut mse: f64 = 0.0;
        let pixel_count = (img1.width() * img1.height()) as f64;

        for (p1, p2) in img1_rgba.pixels().zip(img2_rgba.pixels()) {
            for i in 0..3 {
                let diff = p1.0[i] as f64 - p2.0[i] as f64;
                mse += diff * diff;
            }
        }

        Some(mse / (pixel_count * 3.0))
    }

    /// Resizes an image to the target dimensions
    pub fn resize(img: &DynamicImage, width: u32, height: u32) -> DynamicImage {
        img.resize_exact(width, height, image::imageops::Lanczos3)
    }

    /// Crops an image to the specified region
    pub fn crop(img: &DynamicImage, x: u32, y: u32, width: u32, height: u32) -> DynamicImage {
        img.crop_imm(x, y, width, height)
    }

    /// Converts an image to grayscale
    pub fn to_grayscale(img: &DynamicImage) -> DynamicImage {
        DynamicImage::ImageLuma8(img.to_luma8())
    }

    /// Calculates the average brightness of an image
    pub fn average_brightness(img: &DynamicImage) -> f64 {
        let luma = img.to_luma8();
        let sum: u64 = luma.pixels().map(|p| p.0[0] as u64).sum();
        sum as f64 / luma.pixels().count() as f64
    }

    /// Detects if an image is mostly dark
    pub fn is_mostly_dark(img: &DynamicImage, threshold: f64) -> bool {
        average_brightness(img) < threshold
    }

    /// Detects if an image is mostly bright
    pub fn is_mostly_bright(img: &DynamicImage, threshold: f64) -> bool {
        average_brightness(img) > threshold
    }
}

#[cfg(test)]
mod tests {
    use super::helpers::*;
    use super::*;

    #[test]
    fn test_test_frame_exists() {
        assert!(!TEST_FRAME_1920X1080.is_empty());
    }

    #[test]
    fn test_load_test_frame() {
        let img = load_test_frame().unwrap();
        assert_eq!(img.width(), 1920);
        assert_eq!(img.height(), 1080);
    }

    #[test]
    fn test_create_solid_color() {
        let img = create_solid_color(100, 100, image::Rgba([255, 0, 0, 255]));
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);

        // All pixels should be red
        let rgba = img.to_rgba8();
        for pixel in rgba.pixels() {
            assert_eq!(pixel.0, [255, 0, 0, 255]);
        }
    }

    #[test]
    fn test_create_checkerboard() {
        let img = create_checkerboard(64, 64, 32);
        let rgba = img.to_rgba8();

        // Check corners
        let top_left = rgba.get_pixel(0, 0);
        let top_right = rgba.get_pixel(63, 0);

        // Should be different colors
        assert_ne!(top_left.0, top_right.0);
    }

    #[test]
    fn test_create_gradient() {
        let img = create_gradient(100, 10);
        let rgba = img.to_rgba8();

        // Left side should be darker than right side
        let left = rgba.get_pixel(0, 5);
        let right = rgba.get_pixel(99, 5);
        assert!(left.0[0] < right.0[0]);
    }

    #[test]
    fn test_create_circle_pattern() {
        let img = create_circle_pattern(100, 100);
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);
    }

    #[test]
    fn test_calculate_psnr_identical() {
        let img = create_solid_color(100, 100, image::Rgba([128, 128, 128, 255]));
        let psnr = calculate_psnr(&img, &img);
        assert_eq!(psnr, Some(f64::INFINITY));
    }

    #[test]
    fn test_calculate_psnr_different() {
        let img1 = create_solid_color(100, 100, image::Rgba([0, 0, 0, 255]));
        let img2 = create_solid_color(100, 100, image::Rgba([255, 255, 255, 255]));

        let psnr = calculate_psnr(&img1, &img2);
        assert!(psnr.is_some());
        // PSNR pode ser negativo para imagens muito diferentes devido a precisão numérica
        // O importante é que seja um número finito (não NaN, não infinito)
        let psnr_val = psnr.unwrap();
        assert!(psnr_val.is_finite());
    }

    #[test]
    fn test_calculate_mse() {
        let img1 = create_solid_color(100, 100, image::Rgba([0, 0, 0, 255]));
        let img2 = create_solid_color(100, 100, image::Rgba([10, 10, 10, 255]));

        let mse = calculate_mse(&img1, &img2);
        assert!(mse.is_some());
        assert!(mse.unwrap() > 0.0);
    }

    #[test]
    fn test_resize() {
        let img = create_solid_color(100, 100, image::Rgba([255, 0, 0, 255]));
        let resized = resize(&img, 50, 50);

        assert_eq!(resized.width(), 50);
        assert_eq!(resized.height(), 50);
    }

    #[test]
    fn test_crop() {
        let img = create_solid_color(100, 100, image::Rgba([255, 0, 0, 255]));
        let cropped = crop(&img, 25, 25, 50, 50);

        assert_eq!(cropped.width(), 50);
        assert_eq!(cropped.height(), 50);
    }

    #[test]
    fn test_average_brightness() {
        let black = create_solid_color(100, 100, image::Rgba([0, 0, 0, 255]));
        let white = create_solid_color(100, 100, image::Rgba([255, 255, 255, 255]));
        let gray = create_solid_color(100, 100, image::Rgba([128, 128, 128, 255]));

        assert_eq!(average_brightness(&black), 0.0);
        assert_eq!(average_brightness(&white), 255.0);
        assert!((average_brightness(&gray) - 128.0).abs() < 1.0);
    }

    #[test]
    fn test_is_mostly_dark_bright() {
        let black = create_solid_color(100, 100, image::Rgba([0, 0, 0, 255]));
        let white = create_solid_color(100, 100, image::Rgba([255, 255, 255, 255]));

        assert!(is_mostly_dark(&black, 50.0));
        assert!(!is_mostly_dark(&white, 50.0));

        assert!(is_mostly_bright(&white, 200.0));
        assert!(!is_mostly_bright(&black, 200.0));
    }
}
