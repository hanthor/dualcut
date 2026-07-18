//! First-frame thumbnails for media clips (feature = "preview").
//! Cached as small PNGs next to the shape cache.

use anyhow::{Context, Result};
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;
use std::path::{Path, PathBuf};

const W: i32 = 128;
const H: i32 = 72;

/// Extract (or fetch from cache) a small first-frame thumbnail for a media
/// URI. Synchronous — call from a worker thread.
pub fn thumbnail_png(cache_dir: &Path, uri: &str) -> Result<PathBuf> {
    let key = format!("thumb-{:016x}.png", fxhash(uri));
    let file = cache_dir.join(key);
    if file.exists() {
        return Ok(file);
    }
    std::fs::create_dir_all(cache_dir)?;

    let pipeline = gst::parse::launch(&format!(
        "uridecodebin uri={uri} ! videoconvert ! videoscale ! \
         video/x-raw,format=RGB,width={W},height={H},pixel-aspect-ratio=1/1 ! \
         appsink name=sink sync=false"
    ))?
    .downcast::<gst::Pipeline>()
    .map_err(|_| anyhow::anyhow!("not a pipeline"))?;
    let sink = pipeline
        .by_name("sink")
        .context("appsink missing")?
        .downcast::<gst_app::AppSink>()
        .map_err(|_| anyhow::anyhow!("not an appsink"))?;

    pipeline.set_state(gst::State::Paused)?;
    let (res, _, _) = pipeline.state(gst::ClockTime::from_seconds(5));
    res.context("prerolling for thumbnail")?;
    let sample = sink.pull_preroll().context("pulling preroll sample")?;
    let buffer = sample.buffer().context("sample has no buffer")?;
    let map = buffer.map_readable()?;

    // RGB rows may be padded to 4-byte alignment.
    let stride = ((W * 3 + 3) & !3) as usize;
    let mut img = image::RgbImage::new(W as u32, H as u32);
    for y in 0..H as usize {
        let row = &map[y * stride..][..W as usize * 3];
        for x in 0..W as usize {
            let i = x * 3;
            img.put_pixel(x as u32, y as u32, image::Rgb([row[i], row[i + 1], row[i + 2]]));
        }
    }
    pipeline.set_state(gst::State::Null)?;
    img.save(&file)?;
    Ok(file)
}

fn fxhash(s: &str) -> u64 {
    // Tiny stable hash; only used for cache filenames.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
