use crate::ThemeLoadError;

/// Given a string in our theme format, iterate over keys and values
pub(crate) fn iter_items(s: &str) -> impl Iterator<Item = Result<(&str, &str), ThemeLoadError>> {
    s.lines().filter_map(|line| {
        let line = line.split("//").next().unwrap_or(line);
        if line.trim().is_empty() {
            None
        } else {
            let mut split = line.split(':');
            match (split.next(), split.next(), split.next()) {
                (Some(key), Some(val), None) => Some(Ok((key.trim(), val.trim()))),
                _ => Some(Err(ThemeLoadError::ParseThemeLineError(line.to_string()))),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn smoke_test() {
        let my_theme = r#"// this is a comment
            THIS_IS_A_KEY: #fff
            SO_IS_THIS: 1.1
            THIS_HAS_A_COMMENT_AFTER_IT: #fff //oops
            "#;

        let items = iter_items(my_theme)
            .collect::<Result<HashMap<_, _>, _>>()
            .unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items.get("THIS_HAS_A_COMMENT_AFTER_IT"), Some(&"#fff"));
    }
}
