use std::borrow::Cow;

// Decode YAML quoted scalar syntax for display only. Invalid escapes remain
// untouched; the original serialized text is never replaced by this view.
pub fn display(value: &str) -> Cow<'_, str> {
    if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        let body = &value[1..value.len() - 1];
        let mut chars = body.chars().peekable();
        let mut decoded = String::new();
        while let Some(character) = chars.next() {
            if character == '\'' && chars.next() != Some('\'') { return Cow::Borrowed(value); }
            decoded.push(character);
        }
        return Cow::Owned(decoded);
    }
    if !value.starts_with('"') || !value.ends_with('"') || value.len() < 2 { return Cow::Borrowed(value); }
    decode_double(&value[1..value.len() - 1]).map(Cow::Owned).unwrap_or(Cow::Borrowed(value))
}

fn hex(chars: &mut std::str::Chars<'_>, digits: usize) -> Option<u32> {
    let mut value = 0;
    for _ in 0..digits { value = value * 16 + chars.next()?.to_digit(16)?; }
    Some(value)
}

fn decode_double(body: &str) -> Option<String> {
    let mut chars = body.chars();
    let mut result = String::new();
    while let Some(character) = chars.next() {
        if character == '"' { return None; }
        if character != '\\' { result.push(character); continue; }
        let escaped = chars.next()?;
        let character = match escaped {
            '0' => '\0', 'a' => '\u{7}', 'b' => '\u{8}', 't' => '\t', 'n' => '\n',
            'v' => '\u{b}', 'f' => '\u{c}', 'r' => '\r', 'e' => '\u{1b}',
            ' ' => ' ', '"' => '"', '/' => '/', '\\' => '\\',
            'N' => '\u{85}', '_' => '\u{a0}', 'L' => '\u{2028}', 'P' => '\u{2029}',
            'x' | 'u' | 'U' => {
                let mut value = hex(&mut chars, match escaped { 'x' => 2, 'u' => 4, _ => 8 })?;
                if escaped == 'u' && (0xd800..=0xdbff).contains(&value) {
                    if chars.next()? != '\\' || chars.next()? != 'u' { return None; }
                    let low = hex(&mut chars, 4)?;
                    if !(0xdc00..=0xdfff).contains(&low) { return None; }
                    value = 0x10000 + ((value - 0xd800) << 10) + low - 0xdc00;
                }
                char::from_u32(value)?
            }
            _ => return None,
        };
        result.push(character);
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quoted_unicode_is_decoded_without_changing_literal_backslashes() {
        assert_eq!(display(r#""\u7070\u8272""#), "灰色");
        assert_eq!(display(r#""\\u7070""#), r"\u7070");
        assert_eq!(display(r"\u7070"), r"\u7070");
        assert_eq!(display(r#""\uD83D\uDE00""#), "😀");
        assert_eq!(display(r#""\U0001F600""#), "😀");
        assert_eq!(display("'it''s 灰色'"), "it's 灰色");
        for invalid in [r#""\uD800""#, r#""\uZZZZ""#, r#""\UFFFFFFFF""#] {
            assert_eq!(display(invalid), invalid);
        }
    }
}
