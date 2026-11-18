//! File-to-file pixel comparison for the pixel-perfect port rule.
//!
//! Two equal-size PNGs (a reference render and the port's render) are
//! compared per pixel: a pixel differs when its max per-channel absolute
//! difference exceeds a tolerance (default 8, the port's convention). The
//! result is a side-by-side composite `reference | port | diff`, where the
//! diff panel paints every differing pixel red over a dimmed reference —
//! the same visualization the port's `tools/compare_shots.py` produces, so
//! an agent (or a human) sees exactly where the port diverges. Overall and
//! per-region differing-pixel shares are reported for the loop's gate.
//!
//! This is a local operation: it needs no running app, unlike the RPC
//! methods. `uzor-dev compare ref.png port.png out.png [--tol N]
//! [--region name:x,y,w,h ...] [--json]`.

use std::io::BufWriter;
use std::path::Path;

/// A named rectangle to report a differing-pixel share for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// Label printed with the share.
    pub name: String,
    /// Left edge, pixels.
    pub x: u32,
    /// Top edge, pixels.
    pub y: u32,
    /// Width, pixels.
    pub w: u32,
    /// Height, pixels.
    pub h: u32,
}

impl Region {
    /// Parse `name:x,y,w,h`.
    pub fn parse(spec: &str) -> Result<Self, CompareError> {
        let (name, box_) = spec.split_once(':').ok_or_else(|| CompareError::BadRegion(spec.to_string()))?;
        let nums: Vec<&str> = box_.split(',').collect();
        if name.is_empty() || nums.len() != 4 {
            return Err(CompareError::BadRegion(spec.to_string()));
        }
        let mut it = nums.iter().map(|n| n.trim().parse::<u32>().map_err(|_| CompareError::BadRegion(spec.to_string())));
        let (x, y, w, h) = (it.next().unwrap()?, it.next().unwrap()?, it.next().unwrap()?, it.next().unwrap()?);
        Ok(Region { name: name.to_string(), x, y, w, h })
    }
}

/// How the diff panel visualises differing pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MapMode {
    /// Differing pixels flat red over a dimmed reference (the port convention).
    #[default]
    Red,
    /// Heat colour-map by difference magnitude (jet: blue→cyan→green→yellow→red),
    /// so an agent sees not just WHERE but HOW BIG each divergence is.
    Thermal,
    /// Pure grayscale magnitude map (0=black, 255=white), reference dropped.
    Abs,
}

/// Options for [`compare_pngs`].
#[derive(Debug, Clone)]
pub struct CompareOptions {
    /// A pixel differs when its max per-channel absolute difference (0-255)
    /// exceeds this. The port's convention is 8.
    pub tol: u8,
    /// Regions to report separately (whole-image share is always reported).
    pub regions: Vec<Region>,
    /// Diff-panel visualisation mode.
    pub map: MapMode,
}

impl Default for CompareOptions {
    fn default() -> Self {
        Self { tol: 8, regions: Vec::new(), map: MapMode::Red }
    }
}

/// Jet-like heat colour for a normalised magnitude `t` in 0..=1.
fn heat(t: f64) -> (u8, u8, u8) {
    let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0) as u8;
    (c(1.5 - (4.0 * t - 3.0).abs()), c(1.5 - (4.0 * t - 2.0).abs()), c(1.5 - (4.0 * t - 1.0).abs()))
}

/// The differing-pixel shares of one comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct CompareReport {
    /// Image width, pixels (both inputs must match).
    pub width: u32,
    /// Image height, pixels.
    pub height: u32,
    /// The tolerance used.
    pub tol: u8,
    /// Whole-image differing-pixel share, 0.0..=1.0.
    pub all: f64,
    /// Per-region shares, in the order the regions were given.
    pub regions: Vec<(String, f64)>,
    /// Per-region `(name, max magnitude, mean magnitude)` over ALL pixels of the
    /// region (not only differing ones): machine-readable "how big is the bug here".
    pub region_stats: Vec<(String, u8, f64)>,
}

/// Failure modes of [`compare_pngs`].
#[derive(Debug)]
pub enum CompareError {
    /// Reading or writing a file failed.
    Io(std::io::Error),
    /// A PNG could not be decoded or encoded.
    Png(String),
    /// The two images have different sizes.
    SizeMismatch {
        /// Reference size `(w, h)`.
        reference: (u32, u32),
        /// Port size `(w, h)`.
        port: (u32, u32),
    },
    /// A region spec is not `name:x,y,w,h` or lies outside the image.
    BadRegion(String),
}

impl std::fmt::Display for CompareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompareError::Io(e) => write!(f, "I/O error: {e}"),
            CompareError::Png(e) => write!(f, "PNG error: {e}"),
            CompareError::SizeMismatch { reference, port } => {
                write!(f, "size differs: reference {}x{}, port {}x{}", reference.0, reference.1, port.0, port.1)
            }
            CompareError::BadRegion(s) => write!(f, "bad region '{s}' (want name:x,y,w,h inside the image)"),
        }
    }
}

impl std::error::Error for CompareError {}

impl From<std::io::Error> for CompareError {
    fn from(e: std::io::Error) -> Self {
        CompareError::Io(e)
    }
}

/// Decode a PNG to 8-bit RGB (alpha dropped), expanding palette/grayscale
/// inputs so any screenshot format compares the same way.
fn load_rgb(path: &Path) -> Result<(u32, u32, Vec<u8>), CompareError> {
    let file = std::fs::File::open(path)?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|e| CompareError::Png(e.to_string()))?;
    let (w, h, color_type) = {
        let info = reader.info();
        (info.width, info.height, info.color_type)
    };
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).map_err(|e| CompareError::Png(e.to_string()))?;
    let bytes = &buf[..frame.buffer_size()];
    let rgb = match color_type {
        png::ColorType::Rgb => bytes.to_vec(),
        png::ColorType::Rgba => {
            let mut out = Vec::with_capacity((w * h * 3) as usize);
            for px in bytes.chunks_exact(4) {
                out.extend_from_slice(&px[..3]);
            }
            out
        }
        other => return Err(CompareError::Png(format!("unsupported color type after EXPAND: {other:?}"))),
    };
    Ok((w, h, rgb))
}

/// Encode 8-bit RGB as a PNG.
fn write_rgb(path: &Path, w: u32, h: u32, rgb: &[u8]) -> Result<(), CompareError> {
    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| CompareError::Png(e.to_string()))?;
    writer.write_image_data(rgb).map_err(|e| CompareError::Png(e.to_string()))?;
    Ok(())
}

/// Max per-channel absolute difference of two RGB pixels.
fn pixel_diff(a: &[u8], b: &[u8]) -> u8 {
    a.iter().zip(b.iter()).map(|(x, y)| x.abs_diff(*y)).max().unwrap_or(0)
}

/// Differing-pixel share of two equal-length RGB buffers at `tol`.
pub fn diff_fraction(a: &[u8], b: &[u8], tol: u8) -> f64 {
    debug_assert_eq!(a.len(), b.len());
    let total = a.len() / 3;
    if total == 0 {
        return 0.0;
    }
    let differing = a.chunks_exact(3).zip(b.chunks_exact(3)).filter(|(x, y)| pixel_diff(x, y) > tol).count();
    differing as f64 / total as f64
}

/// Compare two equal-size PNGs, write the `reference | port | diff`
/// composite to `out_path`, and return the differing-pixel shares.
///
/// The diff panel paints differing pixels red `(255, 40, 40)` over the
/// reference dimmed to 40% — the port's `compare_shots.py` visualization.
pub fn compare_pngs(ref_path: &Path, port_path: &Path, out_path: &Path, opts: &CompareOptions) -> Result<CompareReport, CompareError> {
    let (w, h, reference) = load_rgb(ref_path)?;
    let (pw, ph, port) = load_rgb(port_path)?;
    if (w, h) != (pw, ph) {
        return Err(CompareError::SizeMismatch { reference: (w, h), port: (pw, ph) });
    }
    let total = (w * h) as usize;
    let mut mask = vec![false; total];
    let mut mag = vec![0u8; total];
    let mut differing = 0usize;
    for i in 0..total {
        let dv = pixel_diff(&reference[i * 3..i * 3 + 3], &port[i * 3..i * 3 + 3]);
        mag[i] = dv;
        let d = dv > opts.tol;
        mask[i] = d;
        differing += d as usize;
    }
    let all = differing as f64 / total.max(1) as f64;

    let mut regions = Vec::new();
    let mut region_stats = Vec::new();
    for r in &opts.regions {
        if r.x.saturating_add(r.w) > w || r.y.saturating_add(r.h) > h || r.w == 0 || r.h == 0 {
            return Err(CompareError::BadRegion(format!("{}:{},{},{},{}", r.name, r.x, r.y, r.w, r.h)));
        }
        let mut count = 0usize;
        let mut max = 0u8;
        let mut sum = 0u64;
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                let i = (y * w + x) as usize;
                count += mask[i] as usize;
                max = max.max(mag[i]);
                sum += mag[i] as u64;
            }
        }
        let n = (r.w * r.h) as f64;
        regions.push((r.name.clone(), count as f64 / n));
        region_stats.push((r.name.clone(), max, sum as f64 / n));
    }

    // Composite: reference | port | diff, 8 px gutters, dark background.
    let gap = 8u32;
    let out_w = w * 3 + gap * 2;
    let mut composite = vec![40u8; (out_w * h * 3) as usize];
    let put = |composite: &mut [u8], x0: u32, src: &[u8]| {
        for y in 0..h {
            let dst = ((y * out_w + x0) * 3) as usize;
            let src_row = (y * w * 3) as usize;
            composite[dst..dst + (w * 3) as usize].copy_from_slice(&src[src_row..src_row + (w * 3) as usize]);
        }
    };
    put(&mut composite, 0, &reference);
    put(&mut composite, w + gap, &port);
    let mut diff_panel = vec![0u8; (w * h * 3) as usize];
    let dim = |c: u8| (c as f64 * 0.4) as u8;
    for i in 0..total {
        let (r, g, b) = match opts.map {
            MapMode::Red => {
                if mask[i] {
                    (255u8, 40u8, 40u8)
                } else {
                    (dim(reference[i * 3]), dim(reference[i * 3 + 1]), dim(reference[i * 3 + 2]))
                }
            }
            MapMode::Thermal => {
                if mask[i] {
                    heat(mag[i] as f64 / 255.0)
                } else {
                    (dim(reference[i * 3]), dim(reference[i * 3 + 1]), dim(reference[i * 3 + 2]))
                }
            }
            MapMode::Abs => (mag[i], mag[i], mag[i]),
        };
        diff_panel[i * 3] = r;
        diff_panel[i * 3 + 1] = g;
        diff_panel[i * 3 + 2] = b;
    }
    put(&mut composite, w * 2 + gap * 2, &diff_panel);
    write_rgb(out_path, out_w, h, &composite)?;

    Ok(CompareReport { width: w, height: h, tol: opts.tol, all, regions, region_stats })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_solid(path: &Path, w: u32, h: u32, rgb: [u8; 3]) {
        let buf: Vec<u8> = (0..w * h).flat_map(|_| rgb).collect();
        write_rgb(path, w, h, &buf).unwrap();
    }

    fn scratch(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("uzor-compare-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn identical_images_have_no_diff() {
        let d = scratch("identical");
        let (a, b, out) = (d.join("a.png"), d.join("b.png"), d.join("out.png"));
        write_solid(&a, 16, 16, [10, 20, 30]);
        write_solid(&b, 16, 16, [10, 20, 30]);
        let r = compare_pngs(&a, &b, &out, &CompareOptions::default()).unwrap();
        assert_eq!(r.all, 0.0);
        assert!(out.exists());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_changed_block_reports_its_share_and_region() {
        let d = scratch("block");
        let (a, b, out) = (d.join("a.png"), d.join("b.png"), d.join("out.png"));
        write_solid(&a, 10, 10, [0, 0, 0]);
        // Top-left 5x5 white => 25 of 100 pixels differ.
        let mut buf = vec![0u8; 10 * 10 * 3];
        for y in 0..5 {
            for x in 0..5 {
                let i = (y * 10 + x) * 3;
                buf[i..i + 3].copy_from_slice(&[255, 255, 255]);
            }
        }
        write_rgb(&b, 10, 10, &buf).unwrap();
        let opts = CompareOptions { tol: 8, regions: vec![Region::parse("corner:0,0,5,5").unwrap(), Region::parse("rest:5,0,5,10").unwrap()], map: MapMode::Red };
        let r = compare_pngs(&a, &b, &out, &opts).unwrap();
        assert!((r.all - 0.25).abs() < 1e-9, "all={}", r.all);
        assert!((r.regions[0].1 - 1.0).abs() < 1e-9);
        assert_eq!(r.regions[1].1, 0.0);
        // Composite is 3 panels + 2 gutters wide.
        let (cw, ch, _) = load_rgb(&out).unwrap();
        assert_eq!((cw, ch), (10 * 3 + 16, 10));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn tolerance_ignores_small_changes() {
        let d = scratch("tol");
        let (a, b, out) = (d.join("a.png"), d.join("b.png"), d.join("out.png"));
        write_solid(&a, 4, 4, [100, 100, 100]);
        write_solid(&b, 4, 4, [105, 100, 100]);
        let r = compare_pngs(&a, &b, &out, &CompareOptions { tol: 8, regions: vec![], map: MapMode::Red }).unwrap();
        assert_eq!(r.all, 0.0);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn size_mismatch_is_an_error() {
        let d = scratch("size");
        let (a, b, out) = (d.join("a.png"), d.join("b.png"), d.join("out.png"));
        write_solid(&a, 4, 4, [0, 0, 0]);
        write_solid(&b, 8, 4, [0, 0, 0]);
        assert!(matches!(compare_pngs(&a, &b, &out, &CompareOptions::default()), Err(CompareError::SizeMismatch { .. })));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn region_parse_rejects_junk() {
        assert!(Region::parse("nope").is_err());
        assert!(Region::parse("x:1,2,3").is_err());
        assert!(Region::parse(":1,2,3,4").is_err());
        assert_eq!(Region::parse("ok:1,2,3,4").unwrap(), Region { name: "ok".into(), x: 1, y: 2, w: 3, h: 4 });
    }
}
