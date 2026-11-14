//! Themes are compiled into the binary so the container image is a single,
//! self-contained file. A theme consists of a stylesheet, XHTML/SVG templates
//! and the fonts it uses.

pub const DEFAULT_THEME: &str = "klassisch-1900";

/// A font file shipped with a theme.
pub struct FontAsset {
    /// File name inside the EPUB's `fonts/` directory.
    pub file: &'static str,
    pub data: &'static [u8],
    /// Whether the EPUB needs it. Fonts only used for raster graphics
    /// (masthead, cover) are not embedded in the book.
    pub embed: bool,
}

pub struct Theme {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub stylesheet: &'static str,
    /// `(name, source)` pairs. Required names: `base.xhtml`, `front.xhtml`,
    /// `section.xhtml`, `end.xhtml`, `masthead.svg`, `cover.svg`.
    pub templates: &'static [(&'static str, &'static str)],
    pub fonts: &'static [FontAsset],
    /// Font family used to measure and draw the masthead.
    pub masthead_font: &'static str,
    /// Font family (bold) used for cover headlines.
    pub headline_font: &'static str,
}

impl Theme {
    pub fn font_data(&self, file: &str) -> &'static [u8] {
        self.fonts
            .iter()
            .find(|f| f.file == file)
            .map(|f| f.data)
            .unwrap_or_else(|| panic!("theme {} has no font {file}", self.id))
    }
}

macro_rules! theme_file {
    ($theme:literal, $path:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/themes/",
            $theme,
            "/",
            $path
        ))
    };
}
macro_rules! font {
    ($theme:literal, $file:literal, $embed:expr) => {
        FontAsset {
            file: $file,
            data: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/themes/",
                $theme,
                "/fonts/",
                $file
            )),
            embed: $embed,
        }
    };
}

static KLASSISCH_1900: Theme = Theme {
    id: "klassisch-1900",
    name: "Klassisch 1900",
    description: "Fraktur masthead, Old Standard headlines, Libre Caslon body text, double rules and drop caps.",
    stylesheet: theme_file!("klassisch-1900", "style.css"),
    templates: &[
        (
            "base.xhtml",
            theme_file!("klassisch-1900", "templates/base.xhtml"),
        ),
        (
            "front.xhtml",
            theme_file!("klassisch-1900", "templates/front.xhtml"),
        ),
        (
            "section.xhtml",
            theme_file!("klassisch-1900", "templates/section.xhtml"),
        ),
        (
            "end.xhtml",
            theme_file!("klassisch-1900", "templates/end.xhtml"),
        ),
        (
            "masthead.svg",
            theme_file!("klassisch-1900", "templates/masthead.svg"),
        ),
        (
            "cover.svg",
            theme_file!("klassisch-1900", "templates/cover.svg"),
        ),
    ],
    fonts: &[
        font!("klassisch-1900", "UnifrakturMaguntia-Book.ttf", false),
        font!("klassisch-1900", "OldStandard-Regular.ttf", true),
        font!("klassisch-1900", "OldStandard-Bold.ttf", true),
        font!("klassisch-1900", "OldStandard-Italic.ttf", true),
        font!("klassisch-1900", "LibreCaslonText-Regular.ttf", true),
        font!("klassisch-1900", "LibreCaslonText-Bold.ttf", true),
        font!("klassisch-1900", "LibreCaslonText-Italic.ttf", true),
    ],
    masthead_font: "UnifrakturMaguntia-Book.ttf",
    headline_font: "OldStandard-Bold.ttf",
};

static THEMES: &[&Theme] = &[&KLASSISCH_1900];

pub fn get(id: &str) -> Option<&'static Theme> {
    THEMES.iter().copied().find(|t| t.id == id)
}

pub fn all() -> &'static [&'static Theme] {
    THEMES
}
