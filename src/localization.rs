//! UI translations. Unity property names and serialized values remain untouched.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{collections::HashMap, sync::OnceLock};

pub const LANGUAGES: [(&str, &str); 7] = [
    ("zh-CN", "简体中文"), ("zh-TW", "繁體中文"), ("en", "English"),
    ("ja", "日本語"), ("ko", "한국어"), ("de", "Deutsch"), ("fr", "Français"),
];
static LANGUAGE: AtomicUsize = AtomicUsize::new(0);
pub fn set_language(code: &str) {
    LANGUAGE.store(LANGUAGES.iter().position(|(value, _)| *value == code).unwrap_or(0), Ordering::Relaxed);
}
pub fn language_index() -> usize { LANGUAGE.load(Ordering::Relaxed) }
fn catalog() -> &'static HashMap<&'static str, [&'static str; 7]> {
    static CATALOG: OnceLock<HashMap<&'static str, [&'static str; 7]>> = OnceLock::new();
    CATALOG.get_or_init(|| include_str!("translations.tsv").lines().filter(|line| !line.trim().is_empty()).map(|line| {
        let row: [&str; 7] = line.split('\t').collect::<Vec<_>>().try_into().expect("translation must contain seven languages");
        (row[0], row)
    }).collect())
}
pub fn tr(source: &str) -> &str {
    let index = language_index();
    if index == 0 { return source; }
    catalog().get(source).map_or(source, |row| row[index])
}

/// Used only on language changes, for UI messages already stored in open tabs.
/// Interpolated paths, names and values are carried over without translation.
pub fn retranslate(source: &str) -> String {
    retranslate_to(source, language_index())
}

fn retranslate_to(source: &str, language: usize) -> String {
    if let Some(row) = catalog().values().find(|row| row.contains(&source)) {
        return row[language].to_owned();
    }
    let placeholder = regex::Regex::new(r"\{[^{}]*\}").unwrap();
    for row in catalog().values() {
        for template in row {
            if !placeholder.is_match(template) { continue; }
            let mut pattern = String::from("(?s)^");
            let mut previous = 0;
            for item in placeholder.find_iter(template) {
                pattern.push_str(&regex::escape(&template[previous..item.start()]));
                pattern.push_str("(.*?)");
                previous = item.end();
            }
            pattern.push_str(&regex::escape(&template[previous..]));
            pattern.push('$');
            let pattern = regex::Regex::new(&pattern).unwrap();
            if let Some(captures) = pattern.captures(source) {
                let mut unnamed = 0;
                let mut values = HashMap::new();
                for (index, token) in placeholder.find_iter(template).enumerate() {
                    let name = token.as_str()[1..token.as_str().len()-1].split(':').next().unwrap();
                    let key = if name.is_empty() { let key = unnamed.to_string(); unnamed += 1; key } else { name.to_owned() };
                    values.insert(key, captures.get(index+1).unwrap().as_str());
                }
                unnamed = 0;
                return placeholder.replace_all(row[language], |caps: &regex::Captures<'_>| {
                    let name = caps[0][1..caps[0].len()-1].split(':').next().unwrap();
                    let key = if name.is_empty() { let key = unnamed.to_string(); unnamed += 1; key } else { name.to_owned() };
                    values.get(&key).copied().unwrap_or("").to_owned()
                }).into_owned();
            }
        }
    }
    source.to_owned()
}


/// Translate a format template before interpolation so paths, names and values are never translated.
#[macro_export]
macro_rules! lf {
    ($source:tt $(, $arg:expr)* $(,)?) => {{
        $crate::localized_format!($source $(, $arg)*)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_catalog_ignores_blank_lines_and_has_all_languages() {
        let rows: Vec<_> = include_str!("translations.tsv").lines()
            .filter(|line| !line.trim().is_empty()).collect();
        for row in &rows {
            assert_eq!(row.split('\t').count(), LANGUAGES.len(), "Invalid translation row: {row}");
        }
        assert_eq!(catalog().len(), rows.len(), "Duplicate translation keys");
    }

    #[test]
    fn translations_preserve_user_content_and_survive_switching() {
        let original = "已交给系统默认程序打开：C:/工程/设置.prefab";
        let english = retranslate_to(original, 2);
        assert_eq!(english, "Opened with the default application: C:/工程/设置.prefab");
        assert_eq!(retranslate_to(&english, 0), original);
        assert_eq!(retranslate_to("文件已被外部修改，请重新打开后再进入编辑模式。", 3),
            "ファイルが外部で変更されました。再度開いて編集モードに切り替えてください。");
        assert_eq!(retranslate_to("OS error 32: C:/工程/资源.asset", 5), "OS error 32: C:/工程/资源.asset");
        for index in 0..7 {
            assert!(!catalog()["设置"][index].is_empty());
            assert!(!catalog()["保存编辑"][index].is_empty());
        }
    }
}
