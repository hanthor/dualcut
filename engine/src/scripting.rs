//! In-process TypeScript scripting (feature = "scripting").
//!
//! Scripts receive the current document and return an edited one:
//!   export function edit(project: Project): Project
//! Types for authors: schema/dualcut.d.ts.

use crate::document::Project;
use anyhow::{Context, Result};
use rustyscript::{json_args, Module, Runtime, RuntimeOptions};

pub fn run_script(source: &str, project: &Project) -> Result<Project> {
    let mut runtime = Runtime::new(RuntimeOptions::default())?;
    let module = Module::new("agent-script.ts", source);
    let handle = runtime.load_module(&module)?;
    let value: serde_json::Value = runtime.call_function(
        Some(&handle),
        "edit",
        json_args!(serde_json::to_value(project)?),
    )?;
    let edited: Project =
        serde_json::from_value(value).context("script returned invalid document")?;
    edited.validate()?;
    Ok(edited)
}
