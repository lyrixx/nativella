//! The launcher icon of an application: the icon of the project, or
//! Nativella's, drawn for each screen density of the template as `tauri icon`
//! draws the launcher icons of Android: 48 dp, the image inside a margin of a
//! twelfth, its corners rounded as much. Linux packages take it whole (deb.rs).

use std::fs;
use std::io;

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::imageops::{self, FilterType as Filter};
use image::{
    ExtendedColorType, ImageEncoder, ImageFormat, ImageResult, Rgba, Rgba32FImage, RgbaImage,
};

use crate::project::Project;

/// Nativella's icon, which the desktop shows too when the project has none:
/// the `bundle.icon` of tauri.conf.json.
const NATIVELLA: &[u8] = include_bytes!("../../icons/icon.png");

/// The size of a launcher icon, in density-independent pixels: as many
/// pixels at 160 dpi, the density of mdpi screens.
const LAUNCHER_DP: u32 = 48;
const MDPI: u32 = 160;

/// The launcher icon of the densest screens, xxxhdpi, at 640 dpi, in pixels:
/// an image smaller than that blurs there.
const LARGEST: u32 = LAUNCHER_DP * 640 / MDPI;

/// The margin around the image, and the radius of its corners, in parts of
/// the size of the icon.
const MARGIN: f32 = 0.0833;

/// An icon, ready to draw at any size.
pub struct Icon {
    /// A square, its colors premultiplied by their alpha: resizing does not
    /// darken the edges then.
    image: Rgba32FImage,
}

impl Icon {
    /// The icon of `project`, or Nativella's if it has none.
    pub fn of(project: &Project) -> io::Result<Self> {
        let Some(path) = project.icon() else {
            return Ok(Self::new(
                decode(NATIVELLA).expect("Nativella's icon is a PNG."),
            ));
        };

        let png = fs::read(&path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("Cannot read the icon {}: {error}.", path.display()),
            )
        })?;
        let image = decode(&png).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("The icon {} is not a PNG file: {error}.", path.display()),
            )
        })?;
        if image.width().max(image.height()) < LARGEST {
            eprintln!(
                "Warning: the icon {} is {}×{} pixels, and phones and desktops show it at up to {LARGEST} pixels, blurred: give it 512 pixels or more.",
                path.display(),
                image.width(),
                image.height()
            );
        }

        Ok(Self::new(image))
    }

    /// The icon of `image`, centered on a transparent square if it is not
    /// one, as `tauri icon --fit contain` does.
    fn new(image: Rgba32FImage) -> Self {
        let side = image.width().max(image.height());
        let mut square = Rgba32FImage::new(side, side);
        imageops::replace(
            &mut square,
            &image,
            i64::from((side - image.width()) / 2),
            i64::from((side - image.height()) / 2),
        );
        for pixel in square.pixels_mut() {
            let alpha = pixel[3];
            for channel in &mut pixel.0[..3] {
                *channel *= alpha;
            }
        }

        Self { image: square }
    }

    /// The launcher icon of screens of `dpi` dots per inch, as a PNG file.
    pub fn launcher(&self, dpi: u16) -> Vec<u8> {
        let size = (LAUNCHER_DP * u32::from(dpi) + MDPI / 2) / MDPI;
        let margin = (size as f32 * MARGIN).round() as u32;
        let inner = size - 2 * margin;
        let resized = imageops::resize(&self.image, inner, inner, Filter::Lanczos3);

        let mut icon = RgbaImage::new(size, size);
        for (x, y, &pixel) in resized.enumerate_pixels() {
            let opacity = pixel[3].clamp(0.0, 1.0) * coverage(x, y, inner, margin as f32);
            if opacity > 0.0 {
                icon.put_pixel(x + margin, y + margin, straight(pixel, opacity));
            }
        }

        encode(&icon)
    }

    /// The whole icon, `size` pixels square, as a PNG file.
    #[cfg(target_os = "linux")]
    pub fn png(&self, size: u32) -> Vec<u8> {
        let resized = imageops::resize(&self.image, size, size, Filter::Lanczos3);

        let mut icon = RgbaImage::new(size, size);
        for (x, y, &pixel) in resized.enumerate_pixels() {
            if pixel[3] > 0.0 {
                icon.put_pixel(x, y, straight(pixel, pixel[3]));
            }
        }

        encode(&icon)
    }
}

/// A premultiplied pixel, with the colors not premultiplied anymore and
/// `opacity`, in bytes.
fn straight(Rgba([red, green, blue, alpha]): Rgba<f32>, opacity: f32) -> Rgba<u8> {
    let to_byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;

    Rgba([
        to_byte(red / alpha),
        to_byte(green / alpha),
        to_byte(blue / alpha),
        to_byte(opacity),
    ])
}

fn encode(icon: &RgbaImage) -> Vec<u8> {
    let mut png = Vec::new();
    PngEncoder::new_with_quality(&mut png, CompressionType::Best, FilterType::Adaptive)
        .write_image(
            icon.as_raw(),
            icon.width(),
            icon.height(),
            ExtendedColorType::Rgba8,
        )
        .expect("An image of its own size encodes in memory.");

    png
}

/// The image of the PNG file `png`.
fn decode(png: &[u8]) -> ImageResult<Rgba32FImage> {
    Ok(image::load_from_memory_with_format(png, ImageFormat::Png)?.into_rgba32f())
}

/// How much of the pixel `x`, `y` of a square of `side` pixels, its corners
/// rounded with `radius`, is inside: 1, 0 outside, a part on the edge of a
/// corner, which smooths it.
fn coverage(x: u32, y: u32, side: u32, radius: f32) -> f32 {
    // From the center of the pixel to the band where the corners do not cut.
    let outside = |position: u32| {
        let position = position as f32 + 0.5;
        position - position.clamp(radius, side as f32 - radius)
    };
    let distance = outside(x).hypot(outside(y));

    (radius - distance + 0.5).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::Path;

    use super::*;

    const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
    const TRANSPARENT: Rgba<u8> = Rgba([0; 4]);

    fn png(image: &RgbaImage) -> Vec<u8> {
        let mut png = Cursor::new(Vec::new());
        image.write_to(&mut png, ImageFormat::Png).unwrap();
        png.into_inner()
    }

    fn icon(image: &RgbaImage) -> Icon {
        Icon::new(decode(&png(image)).unwrap())
    }

    fn pixels(png: &[u8]) -> RgbaImage {
        image::load_from_memory_with_format(png, ImageFormat::Png)
            .unwrap()
            .into_rgba8()
    }

    #[test]
    fn a_launcher_icon_for_each_density() {
        let icon = icon(&RgbaImage::from_pixel(512, 512, RED));

        for (dpi, size) in [
            (120, 36),
            (160, 48),
            (240, 72),
            (320, 96),
            (480, 144),
            (640, 192),
        ] {
            let launcher = pixels(&icon.launcher(dpi));

            assert_eq!(launcher.dimensions(), (size, size), "{dpi} dpi");
            let margin = (size as f32 * MARGIN).round() as u32;
            assert_eq!(*launcher.get_pixel(margin - 1, size / 2), TRANSPARENT);
            assert_eq!(
                *launcher.get_pixel(margin, size / 2),
                RED,
                "The edge is straight."
            );
            assert_eq!(*launcher.get_pixel(size / 2, size / 2), RED);
            assert_eq!(
                *launcher.get_pixel(margin, margin),
                TRANSPARENT,
                "The corners are round."
            );
            assert_eq!(
                *launcher.get_pixel(size - margin - 1, size - margin - 1),
                TRANSPARENT
            );
        }
    }

    #[test]
    fn a_rectangle_is_centered_on_a_square() {
        let icon = icon(&RgbaImage::from_pixel(300, 200, RED));

        let launcher = pixels(&icon.launcher(640));

        // The image fills 160 of the 192 pixels, a third of it transparent.
        assert_eq!(*launcher.get_pixel(96, 30), TRANSPARENT);
        assert_eq!(*launcher.get_pixel(96, 96), RED);
        assert_eq!(*launcher.get_pixel(20, 96), RED);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn the_whole_icon_has_no_margin() {
        let png = pixels(&icon(&RgbaImage::from_pixel(512, 512, RED)).png(256));

        assert_eq!(png.dimensions(), (256, 256));
        assert_eq!(*png.get_pixel(0, 0), RED);
        assert_eq!(*png.get_pixel(255, 255), RED);
    }

    #[test]
    fn transparency_does_not_darken_the_edges() {
        // White on its right half, transparent black on its left one.
        let image =
            RgbaImage::from_fn(
                512,
                512,
                |x, _| {
                    if x < 256 { TRANSPARENT } else { Rgba([255; 4]) }
                },
            );

        let launcher = pixels(&icon(&image).launcher(160));

        let edge = launcher.get_pixel(24, 24);
        assert!(0 < edge[3] && edge[3] < 255, "{edge:?}");
        assert_eq!(edge.0[..3], [255; 3]);
    }

    /// A project in `root`, with the icon `icon` if any.
    fn project(root: &Path, icon: Option<&[u8]>) -> Project {
        fs::create_dir_all(root).unwrap();
        let mut composer = serde_json::json!({ "name": "acme/notes" });
        if let Some(icon) = icon {
            fs::write(root.join("icon.png"), icon).unwrap();
            composer["extra"] = serde_json::json!({ "nativella": { "icon": "icon.png" } });
        }
        fs::write(root.join("composer.json"), composer.to_string()).unwrap();

        Project::load(root).unwrap()
    }

    #[test]
    fn the_icon_of_the_project_or_nativella_s() {
        let root = Path::new(env!("OUT_DIR")).join(format!("icon-test-{}", std::process::id()));

        let red = Icon::of(&project(
            &root.join("red"),
            Some(&png(&RgbaImage::from_pixel(512, 512, RED))),
        ))
        .unwrap();
        assert_eq!(*pixels(&red.launcher(160)).get_pixel(24, 24), RED);

        let nativella = Icon::of(&project(&root.join("none"), None)).unwrap();
        assert_eq!(
            nativella.launcher(160),
            Icon::new(decode(NATIVELLA).unwrap()).launcher(160)
        );

        let error = Icon::of(&project(&root.join("text"), Some(b"Not a PNG")))
            .err()
            .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(
            error.to_string().contains("icon.png is not a PNG file"),
            "{error}"
        );

        let missing = project(&root.join("missing"), Some(b""));
        fs::remove_file(root.join("missing/icon.png")).unwrap();
        let error = Icon::of(&missing).err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(error.to_string().contains("missing/icon.png"), "{error}");

        fs::remove_dir_all(root).unwrap();
    }
}
