mod common;

use std::collections::HashSet;

use common::{files, sample, unzip};

#[test]
fn mimetype_is_first_and_uncompressed() {
    let book = epub_gazette::render(sample()).unwrap();
    let entries = unzip(&book.epub);
    let (name, data, stored) = &entries[0];
    assert_eq!(name, "mimetype");
    assert_eq!(data, b"application/epub+zip");
    assert!(stored, "mimetype must be stored without compression");
    assert_eq!(book.file_name, "the-ragazzi-observer-2026-11-14.epub");
}

#[test]
fn every_xhtml_file_is_well_formed_and_links_resolve() {
    let book = epub_gazette::render(sample()).unwrap();
    let files = files(&book.epub);
    let xhtml: Vec<_> = files
        .keys()
        .filter(|n| n.ends_with(".xhtml"))
        .cloned()
        .collect();
    assert!(xhtml.len() >= 7, "{xhtml:?}");

    for name in &xhtml {
        let text = std::str::from_utf8(&files[name]).unwrap();
        let doc = common::parse_xml(text).unwrap_or_else(|e| panic!("{name}: {e}"));
        let ids: HashSet<&str> = doc
            .descendants()
            .filter_map(|n| n.attribute("id"))
            .collect();
        for node in doc.descendants() {
            for attr in ["href", "src"] {
                let Some(target) = node.attribute(attr) else {
                    continue;
                };
                if target.starts_with("http") {
                    continue;
                }
                let (path, frag) = target.split_once('#').unwrap_or((target, ""));
                let file = if path.is_empty() {
                    name.clone()
                } else {
                    format!("OEBPS/{path}")
                };
                assert!(files.contains_key(&file), "{name}: broken {attr}={target}");
                if !frag.is_empty() && path.is_empty() {
                    assert!(ids.contains(frag), "{name}: missing anchor #{frag}");
                } else if !frag.is_empty() {
                    let other =
                        common::parse_xml(std::str::from_utf8(&files[&file]).unwrap()).unwrap();
                    assert!(
                        other.descendants().any(|n| n.attribute("id") == Some(frag)),
                        "{name}: missing anchor {target}"
                    );
                }
            }
        }
    }
}

#[test]
fn package_document_declares_cover_and_fonts() {
    let book = epub_gazette::render(sample()).unwrap();
    let files = files(&book.epub);
    let opf = std::str::from_utf8(&files["OEBPS/content.opf"]).unwrap();
    let doc = common::parse_xml(opf).unwrap();

    assert!(
        opf.contains(r#"<meta name="cover" content="cover-image"/>"#),
        "Kindle needs the EPUB 2 cover meta"
    );
    assert!(opf.contains(r#"properties="cover-image""#));
    assert!(opf.contains("<dc:title>The Ragazzi Observer – Sa. 14.11.2026</dc:title>"));

    let ids: Vec<&str> = doc
        .descendants()
        .filter_map(|n| n.attribute("id"))
        .collect();
    let unique: HashSet<&str> = ids.iter().copied().collect();
    assert_eq!(
        ids.len(),
        unique.len(),
        "duplicate ids in content.opf: {ids:?}"
    );

    for item in doc.descendants().filter(|n| n.has_tag_name("item")) {
        let href = item.attribute("href").unwrap();
        assert!(
            files.contains_key(&format!("OEBPS/{href}")),
            "manifest lists missing file {href}"
        );
    }
    assert!(files.contains_key("OEBPS/fonts/LibreCaslonText-Regular.ttf"));
    assert!(
        !files.contains_key("OEBPS/fonts/UnifrakturMaguntia-Book.ttf"),
        "masthead font is rasterized only"
    );
}

#[test]
fn images_are_grayscale_and_sized_for_e_ink() {
    let book = epub_gazette::render(sample()).unwrap();
    let files = files(&book.epub);
    let cover = image::load_from_memory(&files["OEBPS/images/cover.jpg"]).unwrap();
    assert_eq!((cover.width(), cover.height()), (1264, 1680));
    assert_eq!(cover.color(), image::ColorType::L8);
    let mast = image::load_from_memory(&files["OEBPS/images/masthead.png"]).unwrap();
    assert_eq!(mast.width(), 1400);
    let front = image::load_from_memory(&files["OEBPS/images/titelbild.jpg"]).unwrap();
    assert!(front.width() <= 1200);
}

#[test]
fn rendering_is_deterministic() {
    let a = files(&epub_gazette::render(sample()).unwrap().epub);
    let b = files(&epub_gazette::render(sample()).unwrap().epub);
    assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>());
    for (name, data) in &a {
        assert_eq!(data, &b[name], "{name} differs between runs");
    }
}

#[test]
fn section_markup_snapshot() {
    let book = epub_gazette::render(sample()).unwrap();
    let files = files(&book.epub);
    insta::assert_snapshot!(
        "r-politik.xhtml",
        std::str::from_utf8(&files["OEBPS/r-politik.xhtml"]).unwrap()
    );
    insta::assert_snapshot!(
        "titel.xhtml",
        std::str::from_utf8(&files["OEBPS/titel.xhtml"]).unwrap()
    );
}

#[test]
fn hostile_text_is_escaped() {
    let mut issue = sample();
    issue.front_image = None;
    issue.sections[0].articles[0].headline = "<script>alert('x')</script> & \"Zitat\"".into();
    issue.sections[0].articles[0].paragraphs[0] = "Text mit \u{0} Steuerzeichen </p><p>".into();
    let book = epub_gazette::render(issue).unwrap();
    let files = files(&book.epub);
    let text = std::str::from_utf8(&files["OEBPS/r-politik.xhtml"]).unwrap();
    common::parse_xml(text).unwrap();
    assert!(text.contains("&lt;script&gt;"));
    assert!(!text.contains('\u{0}'));
}

#[test]
fn works_without_optional_parts() {
    let mut issue = sample();
    issue.front_image = None;
    issue.weather = None;
    issue.almanac = None;
    issue.briefs = None;
    issue.volume = None;
    issue.colophon.clear();
    let book = epub_gazette::render(issue).unwrap();
    let files = files(&book.epub);
    assert!(!files.contains_key("OEBPS/images/titelbild.jpg"));
    let front = std::str::from_utf8(&files["OEBPS/titel.xhtml"]).unwrap();
    assert!(!front.contains("Jahrgang"));
    assert!(!front.contains("class=\"extras\""));
}

#[test]
fn invalid_issue_reports_details() {
    let mut issue = sample();
    issue.sections[1].id = "politik".into();
    match epub_gazette::render(issue) {
        Err(epub_gazette::RenderError::Invalid(errs)) => {
            assert!(errs[0].contains("used twice"), "{errs:?}")
        }
        other => panic!(
            "expected validation error, got {:?}",
            other.map(|r| r.file_name)
        ),
    }
}

#[test]
fn committed_schema_is_up_to_date() {
    let generated = serde_json::to_string_pretty(&epub_gazette::schema()).unwrap() + "\n";
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/schema/issue.v1.json");
    if std::env::var_os("UPDATE_SCHEMA").is_some() {
        std::fs::write(path, &generated).unwrap();
    }
    let committed = std::fs::read_to_string(path).unwrap_or_default();
    assert_eq!(
        committed, generated,
        "schema/issue.v1.json is stale; run `UPDATE_SCHEMA=1 cargo test`"
    );
}
