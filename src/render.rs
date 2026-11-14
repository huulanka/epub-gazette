//! The rendering pipeline: validated [`Issue`] in, EPUB bytes out.

use std::collections::HashMap;
use std::sync::OnceLock;

use chrono::{NaiveTime, TimeZone, Utc};
use epub_builder::{EpubBuilder, EpubContent, EpubVersion, ReferenceType, TocElement, ZipLibrary};
use minijinja::{AutoEscape, Environment};
use serde::Serialize;

use crate::graphics::{self, CoverInput, GraphicsError, Illustration};
use crate::locale::{self, Labels};
use crate::model::{Article, Issue};
use crate::theme::{self, Theme};

pub const GENERATOR: &str = concat!("epub-gazette ", env!("CARGO_PKG_VERSION"));

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("the issue is invalid")]
    Invalid(Vec<String>),
    #[error(transparent)]
    Graphics(#[from] GraphicsError),
    #[error("template error: {0}")]
    Template(#[from] minijinja::Error),
    #[error("EPUB packaging failed: {0}")]
    Epub(#[from] epub_builder::Error),
}

/// The finished book plus a suggested file name.
pub struct Rendered {
    pub epub: Vec<u8>,
    pub file_name: String,
}

// ---------------------------------------------------------------------------
// Template environments, one per theme, built once.

fn environment(theme: &'static Theme) -> &'static Environment<'static> {
    static ENVS: OnceLock<HashMap<&'static str, Environment<'static>>> = OnceLock::new();
    let envs = ENVS.get_or_init(|| {
        theme::all()
            .iter()
            .map(|t| {
                let mut env = Environment::new();
                // Everything we emit is XML (XHTML or SVG); escape all of it.
                env.set_auto_escape_callback(|_| AutoEscape::Html);
                env.set_formatter(xml_formatter);
                env.set_syntax(
                    minijinja::syntax::SyntaxConfig::builder()
                        .trim_blocks(true)
                        .lstrip_blocks(true)
                        .build()
                        .expect("valid syntax config"),
                );
                env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
                for (name, src) in t.templates {
                    env.add_template(name, src)
                        .unwrap_or_else(|e| panic!("theme {} template {name}: {e}", t.id));
                }
                (t.id, env)
            })
            .collect()
    });
    &envs[theme.id]
}

/// Escapes like HTML auto-escaping but leaves `/` alone, which keeps URLs in
/// attributes readable. Only `& < > " '` need escaping in XML.
fn xml_formatter(
    out: &mut minijinja::Output,
    state: &mut minijinja::State,
    value: &minijinja::Value,
) -> Result<(), minijinja::Error> {
    match value.as_str() {
        Some(s) if !value.is_safe() => {
            let mut buf = String::with_capacity(s.len());
            for c in s.chars() {
                match c {
                    '&' => buf.push_str("&amp;"),
                    '<' => buf.push_str("&lt;"),
                    '>' => buf.push_str("&gt;"),
                    '"' => buf.push_str("&quot;"),
                    '\'' => buf.push_str("&#39;"),
                    c => buf.push(c),
                }
            }
            out.write_str(&buf).map_err(minijinja::Error::from)
        }
        _ => minijinja::escape_formatter(out, state, value),
    }
}

// ---------------------------------------------------------------------------
// View models handed to the templates

#[derive(Serialize)]
struct FrontImageView {
    src: &'static str,
    alt: String,
    caption_line: Option<String>,
}

#[derive(Serialize)]
struct TocEntry {
    section: String,
    headline: String,
    minutes: Option<u32>,
    href: String,
}

#[derive(Serialize)]
struct PaperView<'a> {
    title: &'a str,
    number: u32,
    volume_roman: Option<String>,
    date_long: String,
    motto: Option<&'a str>,
    front_image: Option<FrontImageView>,
    weather: Option<&'a crate::model::Weather>,
    almanac: Option<&'a crate::model::Almanac>,
    toc: Vec<TocEntry>,
    colophon: Vec<String>,
}

#[derive(Serialize)]
struct ArticleView<'a> {
    anchor: String,
    headline: &'a str,
    kicker: Option<&'a str>,
    dek: Option<&'a str>,
    byline: Option<String>,
    initial: Option<String>,
    first_rest: String,
    before_quote: &'a [String],
    pull_quote: Option<&'a str>,
    after_quote: &'a [String],
    context: Option<&'a str>,
    sources: Option<String>,
}

#[derive(Serialize)]
struct SectionView<'a> {
    title: &'a str,
    note: Option<&'a str>,
    articles: Vec<ArticleView<'a>>,
}

fn section_file(id: &str) -> String {
    format!("r-{id}.xhtml")
}

fn article_anchor(section_id: &str, idx: usize) -> String {
    format!("{section_id}-{}", idx + 1)
}

/// Splits off a drop-cap initial when the paragraph starts with a letter or digit.
fn split_initial(p: &str) -> (Option<String>, String) {
    let mut chars = p.chars();
    match chars.next() {
        Some(c) if c.is_alphanumeric() => (Some(c.to_string()), chars.as_str().to_string()),
        _ => (None, p.to_string()),
    }
}

fn article_view<'a>(
    section_id: &str,
    idx: usize,
    a: &'a Article,
    labels: &Labels,
) -> ArticleView<'a> {
    let (initial, first_rest) = split_initial(&a.paragraphs[0]);
    let rest = &a.paragraphs[1..];
    // The pull quote goes after the second paragraph (or after the first if
    // that is all there is).
    let split = rest.len().min(1);
    let byline = a.minutes.map(|m| format!("{m} {}", labels.minutes));
    let sources = (!a.sources.is_empty()).then(|| {
        a.sources
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    });
    ArticleView {
        anchor: article_anchor(section_id, idx),
        headline: &a.headline,
        kicker: a.kicker.as_deref(),
        dek: a.dek.as_deref(),
        byline,
        initial,
        first_rest,
        before_quote: &rest[..split],
        pull_quote: a.pull_quote.as_deref(),
        after_quote: &rest[split..],
        context: a.context.as_deref(),
        sources,
    }
}

#[derive(Serialize)]
struct PageCtx<'a, T: Serialize> {
    lang: &'a str,
    labels: &'static Labels,
    paper: &'a PaperView<'a>,
    #[serde(flatten)]
    extra: T,
}

#[derive(Serialize)]
struct NoExtra {}

#[derive(Serialize)]
struct SectionExtra<'a> {
    section: SectionView<'a>,
}

#[derive(Serialize)]
struct EndExtra<'a> {
    end_title: &'a str,
    briefs: Option<&'a crate::model::Briefs>,
}

// ---------------------------------------------------------------------------

/// Validates, sanitizes and renders an issue into an EPUB 3 book.
pub fn render(mut issue: Issue) -> Result<Rendered, RenderError> {
    issue.sanitize();
    issue.validate().map_err(RenderError::Invalid)?;
    let theme = theme::get(&issue.theme).expect("validated");
    let env = environment(theme);
    let lang = issue.language.as_str();
    let labels = locale::labels(lang);

    // Graphics
    let illustration: Option<Illustration> = issue
        .front_image
        .as_ref()
        .map(|i| graphics::prepare_illustration(&i.data))
        .transpose()?;
    let masthead_png = graphics::masthead(theme, env, &issue.title)?;
    let volume_roman = issue.volume.and_then(locale::roman);
    let cover_jpg = make_cover(&issue, theme, env, illustration.as_ref())?;

    // Shared page data
    let front_image = issue
        .front_image
        .as_ref()
        .filter(|_| illustration.is_some())
        .map(|img| {
            let caption_line = match (img.caption.as_deref(), img.credit.as_deref()) {
                (Some(c), Some(cr)) => Some(format!("{c} · {cr}")),
                (Some(c), None) => Some(c.to_string()),
                (None, Some(cr)) => Some(cr.to_string()),
                (None, None) => None,
            };
            FrontImageView {
                src: "images/titelbild.jpg",
                alt: img
                    .caption
                    .clone()
                    .unwrap_or_else(|| labels.front_image.to_string()),
                caption_line,
            }
        });
    let toc = issue
        .sections
        .iter()
        .flat_map(|s| {
            s.articles.iter().enumerate().map(move |(i, a)| TocEntry {
                section: s.title.clone(),
                headline: a.teaser.clone().unwrap_or_else(|| a.headline.clone()),
                minutes: a.minutes,
                href: format!("{}#{}", section_file(&s.id), article_anchor(&s.id, i)),
            })
        })
        .chain(issue.briefs.as_ref().map(|b| {
            TocEntry {
                section: b.title.clone(),
                headline: b
                    .items
                    .iter()
                    .map(|i| i.lead.as_str())
                    .collect::<Vec<_>>()
                    .join(" · "),
                minutes: None,
                href: "schluss.xhtml".into(),
            }
        }))
        .collect();
    let mut colophon = issue.colophon.clone();
    if let Some(line) = front_image.as_ref().and_then(|f| f.caption_line.as_ref()) {
        colophon.push(format!("{}: {line}", labels.front_image));
    }
    colophon.push(labels.typeset_with.to_string());
    let paper = PaperView {
        title: &issue.title,
        number: issue.number,
        volume_roman,
        date_long: locale::date_long(issue.date, lang),
        motto: issue.motto.as_deref(),
        front_image,
        weather: issue.weather.as_ref(),
        almanac: issue.almanac.as_ref(),
        toc,
        colophon,
    };
    let page = |template: &str, extra: &dyn erased::Extra| -> Result<String, RenderError> {
        Ok(extra.render(env.get_template(template)?, lang, labels, &paper)?)
    };

    // Book
    let mut book = EpubBuilder::new(ZipLibrary::new()?)?;
    book.epub_version(EpubVersion::V30);
    book.metadata(
        "title",
        format!("{} – {}", issue.title, locale::date_short(issue.date, lang)),
    )?;
    // No dc:creator: epub-builder 0.8 gives dc:creator and dc:language the
    // same id attribute, which epubcheck rejects (RSC-005).
    book.metadata("lang", lang)?;
    book.metadata("generator", GENERATOR)?;
    book.metadata("toc_name", labels.index)?;
    book.metadata(
        "description",
        format!(
            "{} {}, {}",
            labels.number,
            issue.number,
            locale::date_long(issue.date, lang)
        ),
    )?;
    let stamp = Utc.from_utc_datetime(
        &issue
            .date
            .and_time(NaiveTime::from_hms_opt(4, 0, 0).expect("valid")),
    );
    book.set_publication_date(stamp);
    book.set_modified_date(stamp);
    book.set_uuid(uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_URL,
        format!(
            "urn:epub-gazette:{}:{}:{}",
            issue.title, issue.date, issue.number
        )
        .as_bytes(),
    ));
    book.stylesheet(theme.stylesheet.as_bytes())?;
    for f in theme.fonts.iter().filter(|f| f.embed) {
        book.add_resource(format!("fonts/{}", f.file), f.data, "font/ttf")?;
    }
    book.add_cover_image("images/cover.jpg", cover_jpg.as_slice(), "image/jpeg")?;
    book.add_resource("images/masthead.png", masthead_png.as_slice(), "image/png")?;
    if let Some(ill) = &illustration {
        book.add_resource("images/titelbild.jpg", ill.jpeg.as_slice(), "image/jpeg")?;
    }

    let front = page("front.xhtml", &NoExtra {})?;
    book.add_content(
        EpubContent::new("titel.xhtml", front.as_bytes())
            .title(labels.front_page)
            .reftype(ReferenceType::Text),
    )?;

    for s in &issue.sections {
        let view = SectionView {
            title: &s.title,
            note: s.note.as_deref(),
            articles: s
                .articles
                .iter()
                .enumerate()
                .map(|(i, a)| article_view(&s.id, i, a, labels))
                .collect(),
        };
        let file = section_file(&s.id);
        let xhtml = page("section.xhtml", &SectionExtra { section: view })?;
        let mut content = EpubContent::new(file.as_str(), xhtml.as_bytes()).title(s.title.as_str());
        for (i, a) in s.articles.iter().enumerate() {
            content = content.child(TocElement::new(
                format!("{file}#{}", article_anchor(&s.id, i)),
                a.headline.as_str(),
            ));
        }
        book.add_content(content)?;
    }

    let end_title = issue
        .briefs
        .as_ref()
        .map_or(labels.colophon, |b| b.title.as_str());
    let end = page(
        "end.xhtml",
        &EndExtra {
            end_title,
            briefs: issue.briefs.as_ref(),
        },
    )?;
    book.add_content(EpubContent::new("schluss.xhtml", end.as_bytes()).title(end_title))?;

    let mut epub = Vec::new();
    book.generate(&mut epub)?;
    let slug: String = issue
        .title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    Ok(Rendered {
        epub,
        file_name: format!(
            "{}-{}.epub",
            if slug.is_empty() { "gazette" } else { &slug },
            issue.date
        ),
    })
}

/// Renders only the cover, for previews and debugging.
pub fn render_cover(mut issue: Issue) -> Result<Vec<u8>, RenderError> {
    issue.sanitize();
    issue.validate().map_err(RenderError::Invalid)?;
    let theme = theme::get(&issue.theme).expect("validated");
    let illustration = issue
        .front_image
        .as_ref()
        .map(|i| graphics::prepare_illustration(&i.data))
        .transpose()?;
    make_cover(&issue, theme, environment(theme), illustration.as_ref())
}

fn make_cover(
    issue: &Issue,
    theme: &Theme,
    env: &Environment<'static>,
    illustration: Option<&Illustration>,
) -> Result<Vec<u8>, RenderError> {
    let lang = issue.language.as_str();
    let labels = locale::labels(lang);
    let mut parts = Vec::new();
    if let Some(v) = issue.volume.and_then(locale::roman) {
        parts.push(format!("{} {v}", labels.volume));
    }
    parts.push(format!("{} {}", labels.number, issue.number));
    parts.push(locale::date_long(issue.date, lang));
    let headlines = issue
        .sections
        .iter()
        .flat_map(|s| s.articles.iter())
        .map(|a| a.teaser.as_deref().unwrap_or(&a.headline))
        .collect();
    let input = CoverInput {
        title: &issue.title,
        dateline: parts.join(" · ").to_uppercase(),
        headlines,
        illustration,
    };
    Ok(graphics::cover(theme, env, &input)?)
}

/// Small helper so pages with different extra context share one closure.
mod erased {
    use super::*;

    pub trait Extra {
        fn render(
            &self,
            tpl: minijinja::Template<'_, '_>,
            lang: &str,
            labels: &'static Labels,
            paper: &PaperView<'_>,
        ) -> Result<String, minijinja::Error>;
    }

    impl<T: Serialize> Extra for T {
        fn render(
            &self,
            tpl: minijinja::Template<'_, '_>,
            lang: &str,
            labels: &'static Labels,
            paper: &PaperView<'_>,
        ) -> Result<String, minijinja::Error> {
            tpl.render(minijinja::value::Serde(PageCtx {
                lang,
                labels,
                paper,
                extra: self,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials() {
        assert_eq!(
            split_initial("Die Zahlen"),
            (Some("D".into()), "ie Zahlen".into())
        );
        assert_eq!(split_initial("Über"), (Some("Ü".into()), "ber".into()));
        assert_eq!(split_initial("»Zitat«"), (None, "»Zitat«".into()));
    }
}
