//! File-backed agent API. The project file on disk is the bus between the
//! HTTP surface and whatever holds the file open (the editor app watches
//! mtime, so agent writes appear live). Serving and editing share no
//! memory — every request reads/writes the file.

use crate::document::Project;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tiny_http::{Header, Method, Response, Server};

fn json_response(status: u16, body: String) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(body)
        .with_status_code(status)
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
}

/// Serve the agent API for `path` on 127.0.0.1:`port`, blocking forever.
/// Spawn on a thread to run alongside a UI.
pub fn serve_file_api(path: PathBuf, port: u16) -> Result<()> {
    let server = Server::http(("127.0.0.1", port))
        .map_err(|e| anyhow::anyhow!("binding 127.0.0.1:{port}: {e}"))?;
    println!("agent API on http://127.0.0.1:{port} (project: {})", path.display());

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().clone();
        let mut body = String::new();
        let _ = request.as_reader().read_to_string(&mut body);

        let load = || -> Result<Project> {
            Project::from_json(&std::fs::read_to_string(&path)?)
                .with_context(|| format!("loading {}", path.display()))
        };

        let response = match (&method, url.as_str()) {
            (Method::Get, "/project") => match load() {
                Ok(p) => json_response(200, p.to_json()),
                Err(e) => json_response(500, format!(r#"{{"error":{:?}}}"#, e.to_string())),
            },
            (Method::Post, "/project") => match Project::from_json(&body) {
                Ok(project) => match std::fs::write(&path, project.to_json()) {
                    Ok(()) => json_response(200, r#"{"ok":true}"#.into()),
                    Err(e) => json_response(500, format!(r#"{{"error":{:?}}}"#, e.to_string())),
                },
                Err(e) => json_response(400, format!(r#"{{"error":{:?}}}"#, e.to_string())),
            },
            #[cfg(feature = "scripting")]
            (Method::Post, "/script") => match load()
                .and_then(|p| crate::scripting::run_script(&body, &p))
            {
                Ok(edited) => match std::fs::write(&path, edited.to_json()) {
                    Ok(()) => json_response(200, r#"{"ok":true}"#.into()),
                    Err(e) => json_response(500, format!(r#"{{"error":{:?}}}"#, e.to_string())),
                },
                Err(e) => json_response(400, format!(r#"{{"error":{:?}}}"#, e.to_string())),
            },
            (Method::Get, "/status") => match load() {
                Ok(p) => json_response(
                    200,
                    serde_json::json!({
                        "engine": "dualcut",
                        "version": env!("CARGO_PKG_VERSION"),
                        "project": p.meta.title,
                        "duration": p.duration(),
                        "scenes": p.scenes.len(),
                    })
                    .to_string(),
                ),
                Err(e) => json_response(500, format!(r#"{{"error":{:?}}}"#, e.to_string())),
            },
            _ => json_response(404, r#"{"error":"not found"}"#.into()),
        };
        let _ = request.respond(response);
    }
    Ok(())
}
