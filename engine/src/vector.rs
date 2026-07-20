//! Vello-rendered vector shapes (feature = "vector").
//!
//! M3 approach: shapes are rasterized once at compile time to cached PNGs
//! (keyed by shape/fill/size) and enter the GES timeline as image clips —
//! so GES-level transforms and opacity/position animations apply to them
//! like any other clip. Live per-frame vector animation (path morphs)
//! comes later with a real Vello source element.

use crate::document::{parse_color, ShapeKind};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use vello::kurbo::{Affine, BezPath, Circle, Ellipse, Point, RoundedRect, Stroke};
use vello::peniko::{Color, Fill};
use vello::wgpu;
use vello::{AaConfig, RenderParams, Renderer, RendererOptions, Scene};

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// One renderer reused across frames — creating it is the expensive
    /// part (shader compilation), and vellosrc renders every frame.
    renderer: std::sync::Mutex<Renderer>,
}

static GPU: OnceLock<Option<Gpu>> = OnceLock::new();

fn gpu() -> Option<&'static Gpu> {
    GPU.get_or_init(|| {
        // Vulkan only: wgpu's GL backend lacks the compute features
        // Vello's shaders need (ARB_arrays_of_arrays panics observed in
        // the wild, #26); no Vulkan means no vector rendering rather
        // than a crashed app.
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = wgpu::Backends::VULKAN;
        let instance = wgpu::Instance::new(desc);
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .ok()?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
        // Shader compilation panics on unsupported drivers; degrade to
        // "no vector rendering" instead of unwinding through GStreamer.
        let renderer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Renderer::new(&device, RendererOptions::default())
        }))
        .ok()?
        .ok()?;
        Some(Gpu { device, queue, renderer: std::sync::Mutex::new(renderer) })
    })
    .as_ref()
}

fn star_path(center: Point, points: u32, outer: f64, inner: f64) -> BezPath {
    let mut path = BezPath::new();
    for i in 0..(points * 2) {
        let r = if i % 2 == 0 { outer } else { inner };
        let a = std::f64::consts::PI * (i as f64) / (points as f64) - std::f64::consts::FRAC_PI_2;
        let p = Point::new(center.x + r * a.cos(), center.y + r * a.sin());
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close_path();
    path
}

fn polygon_path(center: Point, sides: u32, radius: f64) -> BezPath {
    let mut path = BezPath::new();
    for i in 0..sides {
        let a = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64)
            - std::f64::consts::FRAC_PI_2;
        let p = Point::new(center.x + radius * a.cos(), center.y + radius * a.sin());
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close_path();
    path
}

fn build_shape_scene(kind: ShapeKind, fill: Color, w: f64, h: f64) -> Scene {
    let mut scene = Scene::new();
    let cx = w / 2.0;
    let cy = h / 2.0;
    let r = w.min(h) / 2.0 - 2.0;
    match kind {
        ShapeKind::Rect => scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            fill,
            None,
            &RoundedRect::new(0.0, 0.0, w, h, w.min(h) * 0.08),
        ),
        ShapeKind::Circle => {
            scene.fill(Fill::NonZero, Affine::IDENTITY, fill, None, &Circle::new((cx, cy), r))
        }
        ShapeKind::Ellipse => scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            fill,
            None,
            &Ellipse::new((cx, cy), (w / 2.0 - 2.0, h / 2.0 - 2.0), 0.0),
        ),
        ShapeKind::Star => scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            fill,
            None,
            &star_path(Point::new(cx, cy), 5, r, r * 0.42),
        ),
        ShapeKind::Polygon => scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            fill,
            None,
            &polygon_path(Point::new(cx, cy), 6, r),
        ),
        ShapeKind::Line => {
            let mut path = BezPath::new();
            path.move_to((2.0, cy));
            path.line_to((w - 2.0, cy));
            scene.stroke(&Stroke::new(h.max(4.0) * 0.35), Affine::IDENTITY, fill, None, &path);
        }
        ShapeKind::Arrow => {
            let shaft = h * 0.28;
            let head = (w * 0.28).min(h);
            let mut path = BezPath::new();
            path.move_to((2.0, cy - shaft / 2.0));
            path.line_to((w - head, cy - shaft / 2.0));
            path.line_to((w - head, cy - h / 2.0 + 2.0));
            path.line_to((w - 2.0, cy));
            path.line_to((w - head, cy + h / 2.0 - 2.0));
            path.line_to((w - head, cy + shaft / 2.0));
            path.line_to((2.0, cy + shaft / 2.0));
            path.close_path();
            scene.fill(Fill::NonZero, Affine::IDENTITY, fill, None, &path);
        }
    }
    scene
}

/// Rasterize a shape to raw RGBA pixels (row-major, tightly packed).
/// `rotate` is radians about the center — used by vellosrc for live frames.
pub fn render_shape_rgba(
    kind: ShapeKind,
    fill_hex: &str,
    width: u32,
    height: u32,
    rotate: f64,
) -> Result<Vec<u8>> {
    let gpu = gpu().context("no GPU/Vulkan adapter available for shape rendering")?;
    let mut renderer = gpu.renderer.lock().unwrap();

    let argb = parse_color(fill_hex);
    let color = Color::from_rgba8(
        ((argb >> 16) & 0xff) as u8,
        ((argb >> 8) & 0xff) as u8,
        (argb & 0xff) as u8,
        ((argb >> 24) & 0xff) as u8,
    );
    let mut scene = build_shape_scene(kind, color, width as f64, height as f64);
    if rotate != 0.0 {
        let rotated = {
            let mut s = Scene::new();
            s.append(
                &scene,
                Some(Affine::rotate_about(
                    rotate,
                    vello::kurbo::Point::new(width as f64 / 2.0, height as f64 / 2.0),
                )),
            );
            s
        };
        scene = rotated;
    }

    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shape target"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    renderer
        .render_to_texture(
            &gpu.device,
            &gpu.queue,
            &scene,
            &view,
            &RenderParams {
                base_color: Color::from_rgba8(0, 0, 0, 0),
                width,
                height,
                antialiasing_method: AaConfig::Area,
            },
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let bytes_per_row = (width * 4).next_multiple_of(256);
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (bytes_per_row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    gpu.queue.submit([encoder.finish()]);

    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map readback buffer"));
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let data = slice.get_mapped_range();

    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let row = &data[(y * bytes_per_row) as usize..][..(width * 4) as usize];
        pixels.extend_from_slice(row);
    }
    Ok(pixels)
}

/// Rasterize a shape to a transparent PNG in `cache_dir`, returning its
/// path. Cached by shape/fill/size.
pub fn shape_png(
    cache_dir: &Path,
    kind: ShapeKind,
    fill_hex: &str,
    width: u32,
    height: u32,
) -> Result<PathBuf> {
    shape_png_maybe_inverted(cache_dir, kind, fill_hex, width, height, 0.0, false)
}

/// As [`shape_png`], but with an optional soft (`feather`, Gaussian sigma
/// in pixels) edge and/or the painted/unpainted regions swapped
/// (`invert`): opaque where the shape was absent, transparent where it
/// was present. Used to bake a freeform shape mask matte (#41): rather
/// than a GStreamer element inverting/feathering a live alpha stream (no
/// plain video-invert element exists in the available gst-plugins-{good,
/// bad} set, and `videobalance`'s `contrast` clamps at 0 instead of
/// negating), both transforms are baked into the raster once and cached.
pub fn shape_png_maybe_inverted(
    cache_dir: &Path,
    kind: ShapeKind,
    fill_hex: &str,
    width: u32,
    height: u32,
    feather: f64,
    invert: bool,
) -> Result<PathBuf> {
    let file = cache_dir.join(format!(
        "shape-{kind:?}-{}-{width}x{height}-f{feather}{}.png",
        fill_hex.trim_start_matches('#'),
        if invert { "-inv" } else { "" }
    ));
    if file.exists() {
        return Ok(file);
    }
    std::fs::create_dir_all(cache_dir)?;
    let mut pixels = render_shape_rgba(kind, fill_hex, width, height, 0.0)?;
    if invert {
        let argb = parse_color(fill_hex);
        let (r, g, b) = (((argb >> 16) & 0xff) as u8, ((argb >> 8) & 0xff) as u8, (argb & 0xff) as u8);
        for px in pixels.chunks_exact_mut(4) {
            if px[3] == 0 {
                px.copy_from_slice(&[r, g, b, 255]);
            } else {
                px.copy_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    let mut img = image::RgbaImage::from_raw(width, height, pixels).context("image from raw")?;
    if feather > 0.0 {
        img = image::imageops::blur(&img, feather as f32);
    }
    img.save(&file).with_context(|| format!("saving {}", file.display()))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every test skips cleanly (rather than failing) if this host has no
    /// usable Vulkan/GPU adapter -- `gpu()` returning `None` is a real,
    /// already-handled outcome (`render_shape_rgba` turns it into a
    /// contextualized `Err`, not a panic), not something these tests exist
    /// to re-litigate. What they actually check is the raster's *content*
    /// once rendering succeeds.
    macro_rules! skip_if_no_gpu {
        ($result:expr) => {
            match $result {
                Ok(v) => v,
                Err(e) if e.to_string().contains("no GPU") => {
                    eprintln!("skipping: {e}");
                    return;
                }
                Err(e) => panic!("unexpected error: {e:#}"),
            }
        };
    }

    fn tmp_cache(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("dualcut-vector-test-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn shape_png_has_the_requested_dimensions() {
        let cache = tmp_cache("dims");
        let path = skip_if_no_gpu!(shape_png(&cache, ShapeKind::Rect, "#ff0000", 64, 32));
        let img = image::open(&path).expect("valid png").to_rgba8();
        assert_eq!((img.width(), img.height()), (64, 32));
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn rect_shape_fills_the_whole_canvas_opaquely() {
        let cache = tmp_cache("rect-fill");
        let path = skip_if_no_gpu!(shape_png(&cache, ShapeKind::Rect, "#ff0000", 40, 40));
        let img = image::open(&path).expect("valid png").to_rgba8();
        // A rect shape has no margin, so even a corner pixel should be
        // opaque and roughly the requested red.
        let px = img.get_pixel(1, 1);
        assert!(px[3] > 200, "corner should be opaque, got alpha={}", px[3]);
        assert!(px[0] > 150 && px[1] < 100, "corner should be reddish, got {px:?}");
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn circle_shape_is_transparent_outside_the_circle() {
        let cache = tmp_cache("circle-corner");
        let path = skip_if_no_gpu!(shape_png(&cache, ShapeKind::Circle, "#00ff00", 60, 60));
        let img = image::open(&path).expect("valid png").to_rgba8();
        // A circle inscribed in a square canvas never reaches the
        // corners -- unlike Rect, this actually distinguishes shape logic
        // from "the whole canvas is painted."
        let corner = img.get_pixel(1, 1);
        let center = img.get_pixel(30, 30);
        assert_eq!(corner[3], 0, "corner outside a circle should be fully transparent");
        assert!(center[3] > 200, "center inside a circle should be opaque");
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn invert_swaps_painted_and_unpainted_regions() {
        let cache = tmp_cache("invert");
        let normal = skip_if_no_gpu!(shape_png_maybe_inverted(
            &cache,
            ShapeKind::Circle,
            "#0000ff",
            60,
            60,
            0.0,
            false
        ));
        let inverted = skip_if_no_gpu!(shape_png_maybe_inverted(
            &cache,
            ShapeKind::Circle,
            "#0000ff",
            60,
            60,
            0.0,
            true
        ));
        let normal = image::open(&normal).expect("valid png").to_rgba8();
        let inverted = image::open(&inverted).expect("valid png").to_rgba8();
        // Center (inside the circle): opaque normally, transparent inverted.
        assert!(normal.get_pixel(30, 30)[3] > 200);
        assert_eq!(inverted.get_pixel(30, 30)[3], 0);
        // Corner (outside the circle): transparent normally, opaque inverted.
        assert_eq!(normal.get_pixel(1, 1)[3], 0);
        assert!(inverted.get_pixel(1, 1)[3] > 200);
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn feathering_softens_the_edge_instead_of_a_hard_cutoff() {
        let cache = tmp_cache("feather");
        let sharp = skip_if_no_gpu!(shape_png_maybe_inverted(
            &cache,
            ShapeKind::Circle,
            "#ffffff",
            80,
            80,
            0.0,
            false
        ));
        let soft = skip_if_no_gpu!(shape_png_maybe_inverted(
            &cache,
            ShapeKind::Circle,
            "#ffffff",
            80,
            80,
            8.0,
            false
        ));
        let sharp = image::open(&sharp).expect("valid png").to_rgba8();
        let soft = image::open(&soft).expect("valid png").to_rgba8();
        // Scan outward from the center along one row; the feathered
        // version's alpha should fall off gradually (more intermediate
        // values near the edge) rather than jumping straight from opaque
        // to zero like the unfeathered raster.
        let row = 40;
        let sharp_intermediate =
            (0..80).filter(|&x| { let a = sharp.get_pixel(x, row)[3]; a > 10 && a < 245 }).count();
        let soft_intermediate =
            (0..80).filter(|&x| { let a = soft.get_pixel(x, row)[3]; a > 10 && a < 245 }).count();
        assert!(
            soft_intermediate > sharp_intermediate,
            "feathered edge should have more intermediate-alpha pixels: sharp={sharp_intermediate} soft={soft_intermediate}"
        );
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn results_are_cached_by_content_not_regenerated() {
        let cache = tmp_cache("cache");
        let first = skip_if_no_gpu!(shape_png(&cache, ShapeKind::Star, "#123456", 32, 32));
        let mtime1 = std::fs::metadata(&first).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let second = skip_if_no_gpu!(shape_png(&cache, ShapeKind::Star, "#123456", 32, 32));
        let mtime2 = std::fs::metadata(&second).unwrap().modified().unwrap();
        assert_eq!(first, second, "identical params should reuse the same cache path");
        assert_eq!(mtime1, mtime2, "second call should not have rewritten the file");
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn different_shapes_produce_different_cache_files() {
        let cache = tmp_cache("distinct");
        let rect = skip_if_no_gpu!(shape_png(&cache, ShapeKind::Rect, "#ffffff", 32, 32));
        let circle = skip_if_no_gpu!(shape_png(&cache, ShapeKind::Circle, "#ffffff", 32, 32));
        assert_ne!(rect, circle);
        let _ = std::fs::remove_dir_all(&cache);
    }
}
