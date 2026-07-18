//! Headless GES render: build the demo timeline, encode to MP4.
//!
//! Usage: render <output.mp4> [input-media-uri]

use anyhow::{Context, Result};
use dualcut_engine::{build_demo_timeline, init, mp4_profile, run_to_eos};
use gstreamer as gst;
use ges::prelude::*;
use gstreamer_editing_services as ges;

fn main() -> Result<()> {
    init()?;

    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "out.mp4".into());
    let media_uri = args.next();

    let timeline = build_demo_timeline(media_uri.as_deref())?;

    let pipeline = ges::Pipeline::new();
    pipeline.set_timeline(&timeline).context("attaching timeline")?;

    let out_abs = std::path::absolute(&out)?;
    let uri = format!("file://{}", out_abs.display());
    pipeline
        .set_render_settings(&uri, &mp4_profile())
        .context("setting render settings")?;
    pipeline
        .set_mode(ges::PipelineFlags::RENDER)
        .context("setting render mode")?;

    println!("rendering timeline -> {}", out_abs.display());
    let start = std::time::Instant::now();
    run_to_eos(&pipeline)?;
    println!("done in {:.1?}", start.elapsed());
    Ok(())
}
