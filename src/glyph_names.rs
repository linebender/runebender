//! postscript glyph name utilities.
//!
//! This file relies on code that is generated in our build.rs script, which
//! is based on the Adobe Glyph List For New Fonts, at
//! https://github.com/adobe-type-tools/agl-aglfn/blob/master/aglfn.txt

include!(concat!(env!("OUT_DIR"), "/glyph_names_codegen.rs"));

/// Given a `char`, returns the postscript name for that `char`s glyph,
/// if one exists in the aglfn.
pub fn glyph_name_for_char(chr: char) -> Option<&'static str> {
    GLYPH_NAMES
        .binary_search_by(|probe| probe.0.cmp(&chr))
        .ok()
        .map(|idx| GLYPH_NAMES[idx].1)
}

/// Given a glyph (represented as a &str), return the postcript name, if one
/// exists in aglfn.
///
/// This returns `None` if there is more than one `char` in the glyph.
///
/// This is a convenience method; we will more often have `&str` than `char`.
pub fn glyph_name_for_glyph(glyph: &str) -> Option<&'static str> {
    let mut chars = glyph.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => glyph_name_for_char(c),
        _ => None,
    }
}

fn is_valid_glyph_name(name: &str) -> bool {
    name.chars()
        .all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_'))
}

pub fn validate_and_standardize_name(name: &str) -> Result<String, IllegalName> {
    match glyph_name_for_glyph(name) {
        Some(canonical_name) => Ok(canonical_name.to_string()),
        None if is_valid_glyph_name(name) => Ok(name.to_string()),
        _ => Err(IllegalName),
    }
}

/// Given a glyph name, guess what the unicode value is?
///
/// Works fine for known glyph names, otherwise just uses the first character :shrug:
pub fn codepoints_for_glyph(name: &str) -> Option<Vec<char>> {
    GLYPH_NAMES
        .iter()
        .find(|(_, n)| *n == name)
        .map(|(c, _)| vec![*c])
        .or_else(|| {
            let mut chars = name.chars();
            // if we're at most one char long, use that as our codepoint
            match (chars.next(), chars.next()) {
                (Some(c), None) => Some(vec![c]),
                _ => None,
            }
        })
}

/// An error indicating a name included illegal characters.
#[derive(Clone)]
pub struct IllegalName;

impl std::fmt::Display for IllegalName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Glyph names can only include a-z, A-Z, 0-9, '.', '_'.")
    }
}

impl std::fmt::Debug for IllegalName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "IllegalName: {}", self)
    }
}

impl std::error::Error for IllegalName {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test() {
        assert_eq!(glyph_name_for_char('c'), Some("c"));
        assert_eq!(glyph_name_for_glyph("c"), Some("c"));
        assert_eq!(glyph_name_for_char('C'), Some("C"));
        assert_eq!(glyph_name_for_glyph("C"), Some("C"));

        assert_eq!(glyph_name_for_char('é'), Some("eacute"));
        assert_eq!(glyph_name_for_glyph("é"), Some("eacute"));

        assert_eq!(glyph_name_for_char('<'), Some("less"));
        assert_eq!(glyph_name_for_glyph("ء"), None);
        assert_eq!(glyph_name_for_glyph("!"), Some("exclam"));
    }

    #[test]
    fn codepoints_for_glyph_() {
        assert_eq!(codepoints_for_glyph("A"), Some(vec!['A']));
        assert_eq!(codepoints_for_glyph("eacute"), Some(vec!['é']));
        assert_eq!(codepoints_for_glyph("some-string"), None);
    }

    #[test]
    fn filtering() {
        assert!(validate_and_standardize_name("hi_this_is_fine.11").is_ok());
        assert!(validate_and_standardize_name("hi_this_is_fine 11").is_err());
        assert!(validate_and_standardize_name("ZAa09.s_is_finz.11").is_ok());
        assert!(validate_and_standardize_name("newGlyph.69").is_ok());
    }
}
