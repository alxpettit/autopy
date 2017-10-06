extern crate autopilot;
extern crate pyo3;
use autopilot::geometry::{Point, Rect, Size};
use image;
use image::{ImageFormat, ImageResult, Pixel, Rgba};
use internal::FromImageError;
use pyo3::prelude::*;
use std::fs::File;
use std::path::Path;

#[py::class]
struct Bitmap {
    bitmap: autopilot::bitmap::Bitmap,
    token: PyToken,
}

#[py::methods]
impl<'a> Bitmap {
    /// Saves image to absolute path in the given format. The image
    /// type is determined from the filename if possible, unless
    /// format is given. If the file already exists, it will be
    /// overwritten. Supported formats are png, gif, and bmp.
    ///
    /// Exceptions:
    ///     - `IOError` is thrown if the file could not be saved.
    ///     - `ValueError` is thrown if image couldn't be parsed.
    fn save(&self, path: &str, format: Option<&str>) -> PyResult<()> {
        let format = format
            .or(Path::new(path).extension().and_then(|x| x.to_str()))
            .unwrap_or("");
        if let Some(fmt) = image_format_from_extension(format) {
            let ref mut buffer = try!(File::create(path));
            try!(
                self.bitmap
                    .image
                    .save(buffer, fmt)
                    .map_err(FromImageError::from)
            );
            Ok(())
        } else {
            Err(exc::ValueError::new(format!("Unknown format {}", format)))
        }
    }

    /// Returns `True` if the given point is contained in
    /// `bmp.bounds`.
    fn point_in_bounds(&self, x: f64, y: f64) -> PyResult<bool> {
        Ok(self.bitmap.bounds().is_point_visible(Point::new(x, y)))
    }

    /// Returns `True` if the given rect is contained in `bmp.bounds`.
    fn rect_in_bounds(&self, rect: ((f64, f64), (f64, f64))) -> PyResult<bool> {
        let rect = Rect::new(
            Point::new((rect.0).0, (rect.0).1),
            Size::new((rect.1).0, (rect.1).1),
        );
        Ok(self.bitmap.bounds().is_rect_visible(rect))
    }

    /// Open the image located at the path specified. The image's
    /// format is determined from the path's file extension.
    #[classmethod]
    fn open(cls: &PyType, path: String) -> PyResult<&Bitmap> {
        let image = try!(image::open(path).map_err(FromImageError::from));
        let bmp = autopilot::bitmap::Bitmap::new(image, None);
        let result = try!(cls.py().init_ref(|t| {
            Bitmap {
                bitmap: bmp,
                token: t,
            }
        }));
        Ok(result)
    }


    /// Returns `(r, g, b)` tuple describing the color at a given
    /// point.
    ///
    /// Exceptions:
    ///     - `ValueError` is thrown if the point out of bounds.
    fn get_color(&self, x: f64, y: f64) -> PyResult<(u8, u8, u8)> {
        let point = Point::new(x, y);
        if !self.bitmap.bounds().is_point_visible(point) {
            Err(exc::ValueError::new(
                format!("Point out of bounds {}", point),
            ))
        } else {
            let rgb = self.bitmap.get_pixel(point);
            let (r, g, b, _) = rgb.channels4();
            Ok((r, g, b))
        }
    }

    /// Attempts to find `color` inside `rect` in `bmp` from the given
    /// `start_point`. Returns coordinates if found, or `None` if
    /// not. If `rect` is `None`, `bmp.bounds` is used instead. If
    /// `start_point` is `None`, the origin of `rect` is used.
    ///
    /// Tolerance is defined as a float in the range from 0 to 1,
    /// where 0 is an exact match and 1 matches anything.
    fn find_color(
        &self,
        color: (u8, u8, u8),
        tolerance: Option<f64>,
        rect: Option<((f64, f64), (f64, f64))>,
        start_point: Option<(f64, f64)>,
    ) -> PyResult<Option<(f64, f64)>> {
        let color = Rgba([color.0, color.1, color.2, 255]);
        let rect: Option<Rect> = rect.map(|r| {
            Rect::new(Point::new((r.0).0, (r.0).1), Size::new((r.1).0, (r.1).1))
        });
        let start_point: Option<Point> = start_point.map(|p| Point::new(p.0, p.1));
        if let Some(point) = self.bitmap.find_color(color, tolerance, rect, start_point) {
            Ok(Some((point.x, point.y)))
        } else {
            Ok(None)
        }
    }

    /// Returns list of all `(x, y)` coordinates inside `rect` in
    /// `bmp` matching `color` from the given `start_point`. If `rect`
    /// is `None`, `bmp.bounds` is used instead. If `start_point` is
    /// `None`, the origin of `rect` is used.
    fn find_every_color(
        &self,
        color: (u8, u8, u8),
        tolerance: Option<f64>,
        rect: Option<((f64, f64), (f64, f64))>,
        start_point: Option<(f64, f64)>,
    ) -> PyResult<Vec<(f64, f64)>> {
        let color = Rgba([color.0, color.1, color.2, 255]);
        let rect: Option<Rect> = rect.map(|r| {
            Rect::new(Point::new((r.0).0, (r.0).1), Size::new((r.1).0, (r.1).1))
        });
        let start_point: Option<Point> = start_point.map(|p| Point::new(p.0, p.1));
        let points = self.bitmap
            .find_every_color(color, tolerance, rect, start_point)
            .iter()
            .map(|p| (p.x, p.y))
            .collect();
        Ok(points)
    }

    /// Returns count of color in bitmap. Functionally equivalent to:
    ///
    /// `len(find_every_color(color, tolerance, rect, start_point))`
    ///
    fn count_of_color(
        &self,
        color: (u8, u8, u8),
        tolerance: Option<f64>,
        rect: Option<((f64, f64), (f64, f64))>,
        start_point: Option<(f64, f64)>,
    ) -> PyResult<u64> {
        let color = Rgba([color.0, color.1, color.2, 255]);
        let rect: Option<Rect> = rect.map(|r| {
            Rect::new(Point::new((r.0).0, (r.0).1), Size::new((r.1).0, (r.1).1))
        });
        let start_point: Option<Point> = start_point.map(|p| Point::new(p.0, p.1));
        let count = self.bitmap
            .count_of_color(color, tolerance, rect, start_point);
        Ok(count)
    }

    /// Attempts to find `needle` inside `rect` in `bmp` from the
    /// given `start_point`. Returns coordinates if found, or `None`
    /// if not. If `rect` is `None`, `bmp.bounds` is used instead. If
    /// `start_point` is `None`, the origin of `rect` is used.
    ///
    /// Tolerance is defined as a float in the range from 0 to 1,
    /// where 0 is an exact match and 1 matches anything.
    fn find_bitmap(
        &self,
        needle: &Bitmap,
        tolerance: Option<f64>,
        rect: Option<((f64, f64), (f64, f64))>,
        start_point: Option<(f64, f64)>,
    ) -> PyResult<Option<(f64, f64)>> {
        let rect: Option<Rect> = rect.map(|r| {
            Rect::new(Point::new((r.0).0, (r.0).1), Size::new((r.1).0, (r.1).1))
        });
        let start_point: Option<Point> = start_point.map(|p| Point::new(p.0, p.1));
        if let Some(point) = self.bitmap.find_bitmap(&needle.bitmap, tolerance, rect, start_point) {
            Ok(Some((point.x, point.y)))
        } else {
            Ok(None)
        }
    }

    /// Returns list of all `(x, y)` coordinates inside `rect` in
    /// `bmp` matching `needle` from the given `start_point`. If
    /// `rect` is `None`, `bmp.bounds` is used instead. If
    /// `start_point` is `None`, the origin of `rect` is used.
    fn find_every_bitmap(
        &self,
        needle: &Bitmap,
        tolerance: Option<f64>,
        rect: Option<((f64, f64), (f64, f64))>,
        start_point: Option<(f64, f64)>,
    ) -> PyResult<Vec<(f64, f64)>> {
        let rect: Option<Rect> = rect.map(|r| {
            Rect::new(Point::new((r.0).0, (r.0).1), Size::new((r.1).0, (r.1).1))
        });
        let start_point: Option<Point> = start_point.map(|p| Point::new(p.0, p.1));
        let points = self.bitmap
            .find_every_bitmap(&needle.bitmap, tolerance, rect, start_point)
            .iter()
            .map(|p| (p.x, p.y))
            .collect();
        Ok(points)
    }

    /// Returns count of occurrences of `needle` in
    /// `bmp`. Functionally equivalent to:
    ///
    /// `len(find_every_bitmap(color, tolerance, rect, start_point))`
    ///
    fn count_of_bitmap(
        &self,
        needle: &Bitmap,
        tolerance: Option<f64>,
        rect: Option<((f64, f64), (f64, f64))>,
        start_point: Option<(f64, f64)>,
    ) -> PyResult<u64> {
        let rect: Option<Rect> = rect.map(|r| {
            Rect::new(Point::new((r.0).0, (r.0).1), Size::new((r.1).0, (r.1).1))
        });
        let start_point: Option<Point> = start_point.map(|p| Point::new(p.0, p.1));
        let count = self.bitmap
            .count_of_bitmap(&needle.bitmap, tolerance, rect, start_point);
        Ok(count)
    }

    /// Returns new bitmap object created from a portion of another.
    ///
    /// Exceptions:
    ///     - `ValueError` is thrown if the portion was out of bounds.
    fn cropped(&mut self, rect: ((f64, f64), (f64, f64))) -> PyResult<&Bitmap> {
        let rect = Rect::new(
            Point::new((rect.0).0, (rect.0).1),
            Size::new((rect.1).0, (rect.1).1),
        );
        let bmp = try!(self.bitmap.cropped(rect).map_err(FromImageError::from));
        let result = try!(self.py().init_ref(|t| {
            Bitmap {
                bitmap: bmp,
                token: t,
            }
        }));
        Ok(result)
    }

    #[getter(width)]
    fn width(&self) -> PyResult<f64> {
        Ok(self.bitmap.size.width)
    }

    #[getter(height)]
    fn height(&self) -> PyResult<f64> {
        Ok(self.bitmap.size.height)
    }

    #[getter(size)]
    fn size(&self) -> PyResult<(f64, f64)> {
        Ok((self.bitmap.size.width, self.bitmap.size.height))
    }

    #[getter(bounds)]
    fn bounds(&self) -> PyResult<((f64, f64), (f64, f64))> {
        let bounds = self.bitmap.bounds();
        let result = (
            (bounds.origin.x, bounds.origin.y),
            (bounds.size.width, bounds.size.height),
        );
        Ok(result)
    }
}

/// This module defines the class `Bitmap` for accessing bitmaps and
/// searching for bitmaps on-screen.
///
/// It also defines functions for taking screenshots of the screen.
#[py::modinit(bitmap)]
fn init(py: Python, m: &PyModule) -> PyResult<()> {
    /// Returns a screengrab of the given portion of the main
    /// display, or the entire display if rect is `None`.
    ///
    /// Exceptions:
    ///     - `ValueError` is thrown if the rect is out of bounds or the image
    ///       failed to parse.
    #[pyfn(m, "capture_screen")]
    fn capture_screen(python: Python, rect: Option<((f64, f64), (f64, f64))>) -> PyResult<&Bitmap> {
        let result: ImageResult<autopilot::bitmap::Bitmap> = if let Some(rect) = rect {
            let portion = Rect::new(
                Point::new((rect.0).0, (rect.0).1),
                Size::new((rect.1).0, (rect.1).1),
            );
            autopilot::bitmap::capture_screen_portion(portion)
        } else {
            autopilot::bitmap::capture_screen()
        };
        let bmp = try!(result.map_err(FromImageError::from));
        let result = try!(python.init_ref(|t| {
            Bitmap {
                bitmap: bmp,
                token: t,
            }
        }));
        Ok(result)
    }

    m.add_class::<Bitmap>()?;
    Ok(())
}

fn image_format_from_extension(extension: &str) -> Option<ImageFormat> {
    let extension: &str = &(extension.to_lowercase());
    match extension {
        "bmp" => Some(ImageFormat::BMP),
        "gif" => Some(ImageFormat::GIF),
        "hdr" => Some(ImageFormat::HDR),

        "jpeg" => Some(ImageFormat::JPEG),
        "png" => Some(ImageFormat::PNG),
        "tga" => Some(ImageFormat::TGA),
        "tiff" => Some(ImageFormat::TIFF),
        "webp" => Some(ImageFormat::WEBP),
        _ => None,
    }
}
