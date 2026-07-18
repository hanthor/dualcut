//! GTK4 preview: the demo GES timeline playing in a window through
//! gtk4paintablesink (GL / DMABuf zero-copy on GTK >= 4.14).
//!
//! Usage: preview [input-media-uri]   (requires a display session)

use anyhow::{Context, Result};
use dualcut_engine::{build_demo_timeline, init};
use ges::prelude::*;
use gstreamer as gst;
use gst::prelude::*;
use gstreamer_editing_services as ges;
use gtk::glib;
use gtk4 as gtk;
use libadwaita as adw;
use libadwaita::prelude::*;

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("dev.dualcut.Preview")
        .build();
    app.connect_activate(|app| {
        if let Err(e) = build_ui(app) {
            eprintln!("error: {e:#}");
            app.quit();
        }
    });
    // Hand only post-`--` args to GTK so our media-uri arg passes through.
    app.run_with_args::<&str>(&[])
}

fn build_ui(app: &adw::Application) -> Result<()> {
    init()?;
    gstgtk4::plugin_register_static().context("registering gtk4paintablesink")?;

    let media_uri = std::env::args().nth(1);
    let timeline = build_demo_timeline(media_uri.as_deref())?;

    let pipeline = ges::Pipeline::new();
    pipeline.set_timeline(&timeline).context("attaching timeline")?;

    let sink = gst::ElementFactory::make("gtk4paintablesink")
        .build()
        .context("creating gtk4paintablesink")?;
    let paintable = sink.property::<gtk::gdk::Paintable>("paintable");
    // glsinkbin uploads frames on the GPU before they reach the paintable.
    let video_sink: gst::Element = match gst::ElementFactory::make("glsinkbin")
        .property("sink", &sink)
        .build()
    {
        Ok(glsink) => glsink,
        Err(_) => sink.clone(),
    };
    pipeline.preview_set_video_sink(Some(&video_sink));

    let picture = gtk::Picture::builder()
        .paintable(&paintable)
        .content_fit(gtk::ContentFit::Contain)
        .hexpand(true)
        .vexpand(true)
        .build();

    let play = gtk::Button::from_icon_name("media-playback-start-symbolic");
    {
        let pipeline = pipeline.clone();
        play.connect_clicked(move |btn| {
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

    let bar = adw::HeaderBar::new();
    bar.pack_start(&play);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&bar);
    content.append(&picture);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("dualcut preview — M0")
        .default_width(960)
        .default_height(600)
        .content(&content)
        .build();
    window.present();

    pipeline.set_state(gst::State::Paused).context("pausing pipeline")?;
    Ok(())
}
