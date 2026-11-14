//! The `issue.v1` input format.
//!
//! An [`Issue`] is everything the renderer needs to typeset one edition of a
//! newspaper. It is deliberately presentation-free: the theme decides how it
//! looks. All text is plain text (no HTML); the renderer escapes it.

use chrono::NaiveDate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One edition of a newspaper.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Issue {
    /// Name of the newspaper, e.g. "The Ragazzi Observer".
    pub title: String,
    /// Publication date (ISO 8601, `YYYY-MM-DD`).
    pub date: NaiveDate,
    /// Running issue number, starting at 1.
    pub number: u32,
    /// Volume ("Jahrgang"). Rendered as a Roman numeral by classic themes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<u32>,
    /// BCP 47 language tag of the content. Only `de` has localized labels so far.
    #[serde(default = "default_language")]
    pub language: String,
    /// Optional motto printed under the masthead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motto: Option<String>,
    /// Theme identifier. See `epub-gazette themes`.
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Illustration for the front page and the cover (e.g. a public-domain engraving).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub front_image: Option<Image>,
    /// Short weather report for the front page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weather: Option<Weather>,
    /// Almanac entry ("on this day") for the front page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub almanac: Option<Almanac>,
    /// The sections ("Ressorts") in reading order. Each becomes its own chapter.
    pub sections: Vec<Section>,
    /// Short news items, printed after the sections.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefs: Option<Briefs>,
    /// Lines for the colophon ("Zu dieser Ausgabe") at the very end.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colophon: Vec<String>,
}

/// A raster image, transported inline.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Image {
    /// Base64-encoded JPEG, PNG, GIF, WebP or TIFF. A `data:` URI prefix is accepted.
    pub data: String,
    /// Caption shown under the image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    /// Attribution, e.g. "Wikimedia Commons, gemeinfrei".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credit: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Weather {
    /// Place name, e.g. "Hamburg".
    pub location: String,
    /// One or two sentences.
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Almanac {
    /// Heading, e.g. "Heute vor 137 Jahren".
    pub heading: String,
    /// One or two sentences.
    pub text: String,
}

/// A section ("Ressort") such as politics or science.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Section {
    /// Stable slug (`[a-z0-9-]`, max. 40 chars), used for file names and links.
    pub id: String,
    /// Display title, e.g. "Politik & Welt".
    pub title: String,
    /// Optional note under the section heading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub articles: Vec<Article>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Article {
    pub headline: String,
    /// Small label above the headline, e.g. "Fortsetzung" or "Hintergrund".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kicker: Option<String>,
    /// Sub-headline / standfirst.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dek: Option<String>,
    /// Estimated reading time in minutes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutes: Option<u32>,
    /// Shorter headline for the front-page index. Defaults to `headline`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teaser: Option<String>,
    /// Body text, one string per paragraph.
    pub paragraphs: Vec<String>,
    /// Highlighted quote, placed after the second paragraph.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_quote: Option<String>,
    /// Short analysis box ("Einordnung") after the body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Source>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// Publication name, e.g. "tagesschau.de".
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Briefs {
    #[serde(default = "default_briefs_title")]
    pub title: String,
    pub items: Vec<Brief>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Brief {
    /// Bold lead-in, e.g. "Bahn".
    pub lead: String,
    pub text: String,
}

fn default_language() -> String {
    "de".into()
}
fn default_theme() -> String {
    crate::theme::DEFAULT_THEME.into()
}
fn default_briefs_title() -> String {
    "Kurz notiert".into()
}

/// Section ids that would collide with generated files.
const RESERVED_IDS: &[&str] = &["titel", "kurz", "nav", "toc", "cover", "impressum"];

/// Upper bound for the decoded front image.
pub const MAX_IMAGE_BYTES: usize = 12 * 1024 * 1024;

impl Issue {
    /// Checks the issue for semantic errors that the JSON schema cannot express.
    /// Returns every problem found, not just the first.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errs = Vec::new();
        let mut req = |ok: bool, msg: String| {
            if !ok {
                errs.push(msg);
            }
        };

        req(
            !self.title.trim().is_empty(),
            "title must not be empty".into(),
        );
        req(
            self.title.chars().count() <= 80,
            "title is longer than 80 characters".into(),
        );
        req(self.number >= 1, "number must be at least 1".into());
        req(
            crate::theme::get(&self.theme).is_some(),
            format!("unknown theme '{}'", self.theme),
        );
        req(
            !self.sections.is_empty(),
            "at least one section is required".into(),
        );
        if let Some(img) = &self.front_image {
            req(
                !img.data.trim().is_empty(),
                "front_image.data must not be empty".into(),
            );
            // base64 is ~4/3 of the binary size
            req(
                img.data.len() <= MAX_IMAGE_BYTES / 3 * 4 + 64,
                "front_image is larger than 12 MiB".into(),
            );
        }

        let mut seen = std::collections::HashSet::new();
        for (si, s) in self.sections.iter().enumerate() {
            let at = format!("sections[{si}]");
            let slug_ok = !s.id.is_empty()
                && s.id.len() <= 40
                && s.id
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
            req(
                slug_ok,
                format!("{at}.id '{}' must match [a-z0-9-]{{1,40}}", s.id),
            );
            req(
                !RESERVED_IDS.contains(&s.id.as_str()),
                format!("{at}.id '{}' is reserved", s.id),
            );
            req(
                seen.insert(s.id.clone()),
                format!("{at}.id '{}' is used twice", s.id),
            );
            req(
                !s.title.trim().is_empty(),
                format!("{at}.title must not be empty"),
            );
            req(!s.articles.is_empty(), format!("{at} has no articles"));
            for (ai, a) in s.articles.iter().enumerate() {
                let at = format!("{at}.articles[{ai}]");
                req(
                    !a.headline.trim().is_empty(),
                    format!("{at}.headline must not be empty"),
                );
                req(!a.paragraphs.is_empty(), format!("{at} has no paragraphs"));
                req(
                    a.paragraphs.iter().all(|p| !p.trim().is_empty()),
                    format!("{at} contains an empty paragraph"),
                );
                req(
                    a.minutes.is_none_or(|m| m <= 120),
                    format!("{at}.minutes is implausible"),
                );
                req(
                    a.sources.iter().all(|s| !s.name.trim().is_empty()),
                    format!("{at} has a source without a name"),
                );
            }
        }
        if let Some(b) = &self.briefs {
            req(!b.items.is_empty(), "briefs.items must not be empty".into());
        }

        if errs.is_empty() { Ok(()) } else { Err(errs) }
    }

    /// Removes characters that are not allowed in XML 1.0 and normalizes
    /// whitespace in every text field, so that LLM output can never produce
    /// a malformed XHTML document.
    pub fn sanitize(&mut self) {
        fn clean(s: &mut String) {
            let filtered: String = s
                .chars()
                .filter(|&c| matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}'))
                .collect();
            *s = filtered.split_whitespace().collect::<Vec<_>>().join(" ");
        }
        fn clean_opt(s: &mut Option<String>) {
            if let Some(v) = s {
                clean(v);
                if v.is_empty() {
                    *s = None;
                }
            }
        }

        clean(&mut self.title);
        clean_opt(&mut self.motto);
        if let Some(img) = &mut self.front_image {
            clean_opt(&mut img.caption);
            clean_opt(&mut img.credit);
        }
        if let Some(w) = &mut self.weather {
            clean(&mut w.location);
            clean(&mut w.text);
        }
        if let Some(a) = &mut self.almanac {
            clean(&mut a.heading);
            clean(&mut a.text);
        }
        for s in &mut self.sections {
            clean(&mut s.title);
            clean_opt(&mut s.note);
            for a in &mut s.articles {
                clean(&mut a.headline);
                clean_opt(&mut a.kicker);
                clean_opt(&mut a.dek);
                clean_opt(&mut a.teaser);
                clean_opt(&mut a.pull_quote);
                clean_opt(&mut a.context);
                a.paragraphs.iter_mut().for_each(clean);
                for src in &mut a.sources {
                    clean(&mut src.name);
                    clean_opt(&mut src.url);
                }
            }
        }
        if let Some(b) = &mut self.briefs {
            clean(&mut b.title);
            for i in &mut b.items {
                clean(&mut i.lead);
                clean(&mut i.text);
            }
        }
        self.colophon.iter_mut().for_each(clean);
        self.colophon.retain(|l| !l.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> Issue {
        serde_json::from_value(serde_json::json!({
            "title": "Test Gazette",
            "date": "2026-11-14",
            "number": 1,
            "sections": [{
                "id": "politik",
                "title": "Politik",
                "articles": [{ "headline": "H", "paragraphs": ["Text."] }]
            }]
        }))
        .unwrap()
    }

    #[test]
    fn minimal_issue_is_valid() {
        let i = minimal();
        assert_eq!(i.language, "de");
        assert_eq!(i.theme, crate::theme::DEFAULT_THEME);
        i.validate().unwrap();
    }

    #[test]
    fn reports_all_errors() {
        let mut i = minimal();
        i.title = " ".into();
        i.sections[0].id = "Bad Id".into();
        i.sections[0].articles[0].paragraphs = vec![];
        let errs = i.validate().unwrap_err();
        assert_eq!(errs.len(), 3, "{errs:?}");
    }

    #[test]
    fn rejects_duplicate_and_reserved_ids() {
        let mut i = minimal();
        let mut dup = i.sections[0].clone();
        dup.title = "Nochmal".into();
        i.sections.push(dup);
        let mut reserved = i.sections[0].clone();
        reserved.id = "titel".into();
        i.sections.push(reserved);
        let errs = i.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("used twice")));
        assert!(errs.iter().any(|e| e.contains("reserved")));
    }

    #[test]
    fn rejects_unknown_fields() {
        let r: Result<Issue, _> = serde_json::from_value(serde_json::json!({
            "title": "T", "date": "2026-11-14", "number": 1, "sections": [], "colour": "red"
        }));
        assert!(r.is_err());
    }

    #[test]
    fn sanitize_strips_control_chars_and_collapses_whitespace() {
        let mut i = minimal();
        i.sections[0].articles[0].paragraphs[0] = "Ein\u{0}  Text\u{1b} mit\n\nUmbruch".into();
        i.motto = Some("   ".into());
        i.sanitize();
        assert_eq!(
            i.sections[0].articles[0].paragraphs[0],
            "Ein Text mit Umbruch"
        );
        assert!(i.motto.is_none());
    }
}
