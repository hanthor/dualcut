//! dualcut app shell (M2 in progress): GNOME/libadwaita window previewing
//! a project document through GES + gtk4paintablesink.
//!
//! - Loads a project JSON (arg 1; falls back to the built-in M0 demo).
//! - Transport: play/pause, seek bar, timecode.
//! - Live reload: the project file's mtime is polled; external edits (from
//!   agents or $EDITOR) rebuild the timeline in place, preserving position.
//!
//! Usage: preview [project.json | media-uri]

use anyhow::{Context, Result};
use dualcut_engine::{build_demo_timeline, document::Project, init, mapping};
use ges::prelude::*;
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_editing_services as ges;
use gtk::glib;
use gtk4 as gtk;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::SystemTime;

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("io.github.hanthor.Dualcut")
        .build();
    app.connect_activate(|app| {
        if let Err(e) = build_ui(app) {
            eprintln!("error: {e:#}");
            app.quit();
        }
    });
    app.run_with_args::<&str>(&[])
}

struct AppState {
    pipeline: ges::Pipeline,
    project_path: Option<PathBuf>,
    mtime: Option<SystemTime>,
    duration: f64,
}

/// GES pipelines are single-timeline: build a fresh pipeline + sink for a
/// timeline and hand back the paintable for the preview widget.
fn make_pipeline(timeline: &ges::Timeline) -> Result<(ges::Pipeline, gtk::gdk::Paintable)> {
    let pipeline = ges::Pipeline::new();
    pipeline.set_timeline(timeline).context("attaching timeline")?;
    let sink = gst::ElementFactory::make("gtk4paintablesink")
        .build()
        .context("creating gtk4paintablesink")?;
    let paintable = sink.property::<gtk::gdk::Paintable>("paintable");
    let video_sink: gst::Element = match gst::ElementFactory::make("glsinkbin")
        .property("sink", &sink)
        .build()
    {
        Ok(glsink) => glsink,
        Err(_) => sink.clone(),
    };
    pipeline.preview_set_video_sink(Some(&video_sink));
    Ok((pipeline, paintable))
}

fn start_paused(pipeline: &ges::Pipeline) -> Result<()> {
    if pipeline.set_state(gst::State::Paused).is_err() {
        // No usable audio output (headless/CI): swap in a fake audio sink.
        let _ = pipeline.set_state(gst::State::Null);
        if let Ok(fake) = gst::ElementFactory::make("fakesink").build() {
            pipeline.preview_set_audio_sink(Some(&fake));
        }
        pipeline.set_state(gst::State::Paused).context("pausing pipeline")?;
    }
    Ok(())
}

fn load_timeline(path: &std::path::Path) -> Result<(ges::Timeline, f64)> {
    let json = std::fs::read_to_string(path)?;
    let project = Project::from_json(&json)?;
    let base_dir = path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
    let compiled = mapping::compile(&project, &base_dir)?;
    for warning in &compiled.warnings {
        eprintln!("warning: {warning}");
    }
    Ok((compiled.timeline, project.duration()))
}

fn build_ui(app: &adw::Application) -> Result<()> {
    init()?;
    gstgtk4::plugin_register_static().context("registering gtk4paintablesink")?;

    let arg = std::env::args().nth(1);
    let (timeline, project_path, duration) = match &arg {
        Some(path) if path.ends_with(".json") => {
            let path = PathBuf::from(path);
            let (timeline, duration) = load_timeline(&path)?;
            (timeline, Some(path), duration)
        }
        other => (build_demo_timeline(other.as_deref())?, None, 8.0),
    };

    let (pipeline, paintable) = make_pipeline(&timeline)?;

    let mtime = project_path.as_ref().and_then(|p| p.metadata().ok()?.modified().ok());
    let state = Rc::new(RefCell::new(AppState {
        pipeline: pipeline.clone(),
        project_path,
        mtime,
        duration,
    }));

    let picture = gtk::Picture::builder()
        .paintable(&paintable)
        .content_fit(gtk::ContentFit::Contain)
        .hexpand(true)
        .vexpand(true)
        .build();

    let play = gtk::Button::from_icon_name("media-playback-start-symbolic");
    let time_label = gtk::Label::new(Some("0:00.0"));
    time_label.add_css_class("numeric");
    let seek = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, duration.max(0.1), 0.05);
    seek.set_hexpand(true);
    seek.set_draw_value(false);

    {
        let state = state.clone();
        play.connect_clicked(move |btn| {
            let pipeline = state.borrow().pipeline.clone();
            let playing = pipeline.current_state() == gst::State::Playing;
            let next = if playing { gst::State::Paused } else { gst::State::Playing };
            let _ = pipeline.set_state(next);
            btn.set_icon_name(if playing {
                "media-playback-start-symbolic"
            } else {
                "media-playback-pause-symbolic"
            });
        });
    }
    {
        let state = state.clone();
        seek.connect_change_value(move |_, _, value| {
            let _ = state.borrow().pipeline.seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::ClockTime::from_useconds((value.max(0.0) * 1e6) as u64),
            );
            glib::Propagation::Proceed
        });
    }

    // Position updates + live project reload, 5x/s.
    {
        let state = state.clone();
        let seek = seek.clone();
        let time_label = time_label.clone();
        let picture = picture.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
            let mut st = state.borrow_mut();
            if let Some(pos) = st.pipeline.query_position::<gst::ClockTime>() {
                let secs = pos.nseconds() as f64 / 1e9;
                seek.set_value(secs);
                time_label.set_text(&format!(
                    "{}:{:04.1} / {}:{:04.1}",
                    (secs / 60.0) as u32,
                    secs % 60.0,
                    (st.duration / 60.0) as u32,
                    st.duration % 60.0
                ));
            }
            // External edit? Rebuild the timeline in place.
            if let Some(path) = st.project_path.clone() {
                let new_mtime = path.metadata().ok().and_then(|m| m.modified().ok());
                if new_mtime.is_some() && new_mtime != st.mtime {
                    st.mtime = new_mtime;
                    match load_timeline(&path).and_then(|(timeline, duration)| {
                        make_pipeline(&timeline).map(|(p, pt)| (p, pt, duration))
                    }) {
                        Ok((new_pipeline, new_paintable, duration)) => {
                            let was_playing =
                                st.pipeline.current_state() == gst::State::Playing;
                            let pos = st.pipeline.query_position::<gst::ClockTime>();
                            let _ = st.pipeline.set_state(gst::State::Null);
                            picture.set_paintable(Some(&new_paintable));
                            st.pipeline = new_pipeline;
                            st.duration = duration;
                            seek.set_range(0.0, duration.max(0.1));
                            let _ = start_paused(&st.pipeline);
                            if was_playing {
                                let _ = st.pipeline.set_state(gst::State::Playing);
                            }
                            if let Some(pos) = pos {
                                let _ = st.pipeline.seek_simple(
                                    gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                                    pos.min(gst::ClockTime::from_useconds((duration * 1e6) as u64)),
                                );
                            }
                            println!("project reloaded ({duration:.1}s)");
                        }
                        Err(e) => eprintln!("reload failed (keeping current timeline): {e:#}"),
                    }
                }
            }
            glib::ControlFlow::Continue
        });
    }

    let bar = adw::HeaderBar::new();
    bar.pack_start(&play);
    bar.pack_end(&time_label);

    let transport = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    transport.set_margin_start(12);
    transport.set_margin_end(12);
    transport.set_margin_bottom(8);
    transport.append(&seek);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&bar);
    content.append(&picture);
    content.append(&transport);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("dualcut")
        .default_width(1024)
        .default_height(640)
        .content(&content)
        .build();
    window.present();

    start_paused(&pipeline)?;
    Ok(())
}
