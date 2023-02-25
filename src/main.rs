use num::Complex;
use png::EncodingError;
use std::{fs::File, io::BufWriter, str::FromStr};

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() != 5 {
        eprintln!("Usage: {} FILE PIXELS UPPERLEFT LOWERRIGHT", args[0]);
        eprintln!(
            "Example: {} mandel.png 1000x750 -1.20,0.35 -1,0.20",
            args[0]
        );
        std::process::exit(1);
    }

    let bounds =
        parse_pair::<u32>(&args[2], 'x').expect(&format!("Unexpected dimensions: {}", &args[2]));
    let upper_left = parse_complex(&args[3]).expect("error parsing upper left corner point");
    let lower_right = parse_complex(&args[4]).expect("error parsing lower right corner point");
    let mut pixels = vec![255; bounds.0 as usize * bounds.1 as usize];
    let filename = &args[1];
    let threads = 8;
    let rows_per_band = bounds.1 / threads + 1;
    let bands = pixels
        .chunks_mut((rows_per_band * bounds.0) as usize)
        .collect::<Vec<_>>();
    crossbeam::scope(|spawner| {
        for (i, band) in bands.into_iter().enumerate() {
            let top = rows_per_band as usize * i;
            let height = band.len() / bounds.0 as usize;
            let band_upper_left = pixel_to_point(bounds, (0, top as u32), upper_left, lower_right);
            let band_lower_right = pixel_to_point(
                bounds,
                (bounds.0, (top + height) as u32),
                upper_left,
                lower_right,
            );
            let band_bounds = (bounds.0, height as u32);
            spawner.spawn(move |_| {
                render(band, band_bounds, band_upper_left, band_lower_right);
            });
        }
    }).unwrap();
    write_image(&filename, &pixels, bounds).expect("Error writing png to the file");
}

fn render(
    pixels: &mut [u8],
    bounds: (u32, u32),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>,
) {
    for row in 0..bounds.1 {
        for column in 0..bounds.0 {
            let point = pixel_to_point(bounds, (column, row), upper_left, lower_right);
            pixels[(row * bounds.0 + column) as usize] = match escape_time(point, 255) {
                None => 0,
                Some(x) => 255 - x as u8,
            };
        }
    }
}

fn pixel_to_point(
    bounds: (u32, u32),
    pixel: (u32, u32),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>,
) -> Complex<f64> {
    let (width, height) = (
        lower_right.re - upper_left.re,
        upper_left.im - lower_right.im,
    );
    Complex {
        re: upper_left.re + pixel.0 as f64 * width / (bounds.0 as f64),
        im: upper_left.im - pixel.1 as f64 * height / (bounds.1 as f64),
    }
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(
        pixel_to_point(
            (100, 100),
            (25, 75),
            Complex { re: -1.0, im: 1.0 },
            Complex { re: 1.0, im: -1.0 }
        ),
        Complex { re: -0.5, im: -0.5 }
    );
    assert_eq!(
        pixel_to_point(
            (100, 100),
            (100, 0),
            Complex { re: -1.0, im: 1.0 },
            Complex { re: 1.0, im: -1.0 }
        ),
        Complex { re: 1.0, im: 1.0 }
    );
}

fn escape_time(c: Complex<f64>, limit: u32) -> Option<u32> {
    let mut z = Complex { re: 0.0, im: 0.0 };
    for i in 0..limit {
        if z.norm_sqr() > 4.0 {
            return Some(i);
        }
        z = z * z + c;
    }
    None
}

fn write_image(filename: &str, pixels: &[u8], bounds: (u32, u32)) -> Result<(), EncodingError> {
    let file = File::create(filename).unwrap();
    let ref mut w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, bounds.0 as u32, bounds.1 as u32); // Width is 2 pixels and height is 1.
    encoder.set_color(png::ColorType::Grayscale);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(pixels)?;
    Ok(())
}

#[test]
fn test_write_to_file() {
    let file_name = "test.png";
    let bounds: (u32, u32) = (1000, 1000);
    let mut pixels = vec![255; bounds.0 as usize * bounds.1 as usize];
    for i in 0..(bounds.0 / 2) {
        for j in 0..bounds.1 {
            pixels[(i * bounds.1 + j) as usize] = 0
        }
    }
    write_image(file_name, &pixels, bounds).unwrap();
}

fn parse_complex(s: &str) -> Option<Complex<f64>> {
    match parse_pair::<f64>(s, ',') {
        Some((re, im)) => Some(Complex { re, im }),
        None => None,
    }
}

#[test]
fn test_parse_complex() {
    assert_eq!(
        parse_complex("1.25,-0.0625"),
        Some(Complex {
            re: 1.25,
            im: -0.0625
        })
    );
    assert_eq!(parse_complex(",-0.0625"), None);
}

fn parse_pair<T: FromStr>(s: &str, seperator: char) -> Option<(T, T)> {
    match s.find(seperator) {
        None => None,
        Some(index) => match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) {
            (Ok(a), Ok(b)) => Some((a, b)),
            _ => None,
        },
    }
}

#[test]
fn test_parse_pair() {
    assert_eq!(parse_pair::<i32>("", ','), None);
    assert_eq!(parse_pair::<i32>("10,", ','), None);
    assert_eq!(parse_pair::<i32>(",10", ','), None);
    assert_eq!(parse_pair::<i32>("10,20", ','), Some((10, 20)));
    assert_eq!(parse_pair::<i32>("10,20xy", ','), None);
    assert_eq!(parse_pair::<f64>("0.5x", 'x'), None);
    assert_eq!(parse_pair::<f64>("0.5x1.5", 'x'), Some((0.5, 1.5)));
}
