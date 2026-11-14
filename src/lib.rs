//! # epub-gazette
//!
//! Turns a JSON description of a newspaper issue into a typeset EPUB 3 book
//! that reads well on Kindle and other e-readers. The crate is used by the
//! `epub-gazette` binary (CLI and HTTP server) but can be embedded as well:
//!
//! ```no_run
//! let issue: epub_gazette::Issue = serde_json::from_str(&std::fs::read_to_string("issue.json")?)?;
//! let book = epub_gazette::render(issue)?;
//! std::fs::write(&book.file_name, &book.epub)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod graphics;
pub mod locale;
pub mod model;
pub mod render;
pub mod server;
pub mod theme;

pub use model::Issue;
pub use render::{RenderError, Rendered, render, render_cover};

/// JSON Schema of the `issue.v1` format.
pub fn schema() -> serde_json::Value {
    let mut schema = serde_json::to_value(schemars::schema_for!(Issue)).expect("schema serializes");
    if let Some(obj) = schema.as_object_mut() {
        obj.insert(
            "$id".into(),
            "https://github.com/huulanka/epub-gazette/schema/issue.v1.json".into(),
        );
        obj.insert("title".into(), "epub-gazette issue (v1)".into());
    }
    schema
}
