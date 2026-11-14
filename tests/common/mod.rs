#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read};

pub fn sample_json() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/sample-issue.json"
    ))
    .unwrap()
}

pub fn sample() -> epub_gazette::Issue {
    serde_json::from_str(&sample_json()).unwrap()
}

/// All entries of the EPUB (zip) in archive order.
pub fn unzip(epub: &[u8]) -> Vec<(String, Vec<u8>, bool)> {
    let mut zip = zip::ZipArchive::new(Cursor::new(epub)).unwrap();
    (0..zip.len())
        .map(|i| {
            let mut f = zip.by_index(i).unwrap();
            let stored = f.compression() == zip::CompressionMethod::Stored;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).unwrap();
            (f.name().unwrap().to_string(), buf, stored)
        })
        .collect()
}

pub fn files(epub: &[u8]) -> BTreeMap<String, Vec<u8>> {
    unzip(epub).into_iter().map(|(n, d, _)| (n, d)).collect()
}

/// Parses XHTML; EPUB 3 documents carry `<!DOCTYPE html>`, so DTDs are allowed.
pub fn parse_xml(text: &str) -> Result<roxmltree::Document<'_>, roxmltree::Error> {
    roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
}
