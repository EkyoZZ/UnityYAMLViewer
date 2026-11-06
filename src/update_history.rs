//! About history is generated from the per-version Markdown documents at build time.
use eframe::egui;

struct Version {
    number: &'static str,
    languages: [&'static [&'static str]; 7],
}

include!(concat!(env!("OUT_DIR"), "/about_history.rs"));

pub fn reset_on_open(older: &mut bool) {
    *older = OLDER_EXPANDED_ON_OPEN;
}

pub fn show(ui: &mut egui::Ui, older: &mut bool) {
    let language = crate::localization::language_index();
    for version in VERSIONS.iter().take(INITIAL_VISIBLE_VERSIONS) {
        show_version(ui, version, language);
    }
    if VERSIONS.len() > INITIAL_VISIBLE_VERSIONS {
        if ui.button(crate::localization::tr(if *older {
            "收起历史版本"
        } else {
            "查看更多历史"
        })).clicked() {
            *older = !*older;
        }
        if *older {
            for version in VERSIONS.iter().skip(INITIAL_VISIBLE_VERSIONS) {
                show_version(ui, version, language);
            }
        }
    }
}

fn show_version(ui: &mut egui::Ui, version: &Version, language: usize) {
    ui.label(egui::RichText::new(version.number).strong());
    for text in version.languages[language] {
        ui.label(*text);
    }
    ui.add_space(8.0);
}
