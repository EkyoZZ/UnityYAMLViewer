//! Editable source documents. Never use the player's normalized ASS as an original.
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Ass,
    Ssa,
    Srt,
    Vtt,
}
impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Ass => "ass",
            Self::Ssa => "ssa",
            Self::Srt => "srt",
            Self::Vtt => "vtt",
        }
    }
    pub fn from_path(path: &Path) -> Result<Self, String> {
        match path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "ass" => Ok(Self::Ass),
            "ssa" => Ok(Self::Ssa),
            "srt" => Ok(Self::Srt),
            "vtt" | "webvtt" => Ok(Self::Vtt),
            _ => Err("Supported subtitle formats: ASS, SSA, SRT, WebVTT".into()),
        }
    }
    pub fn styled(self) -> bool {
        matches!(self, Self::Ass | Self::Ssa)
    }
    pub fn detect(text: &str, path: &Path) -> Result<Self, String> {
        let text = text.trim_start_matches('\u{feff}');
        let first = text.lines().next().unwrap_or("");
        if first == "WEBVTT" || first.starts_with("WEBVTT ") || first.starts_with("WEBVTT\t") {
            return Ok(Self::Vtt);
        }
        if text
            .lines()
            .any(|l| l.trim().eq_ignore_ascii_case("[V4 Styles]"))
        {
            return Ok(Self::Ssa);
        }
        if text
            .lines()
            .any(|l| l.trim().eq_ignore_ascii_case("[V4+ Styles]"))
        {
            return Ok(Self::Ass);
        }
        if text.lines().take(4).any(|l| l.contains("-->")) {
            return Ok(Self::Srt);
        }
        Self::from_path(path)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub prefix: String,
    pub columns: Vec<String>,
    pub values: Vec<String>,
}
impl Record {
    pub fn get(&self, name: &str) -> &str {
        self.columns
            .iter()
            .position(|s| s.eq_ignore_ascii_case(name))
            .and_then(|i| self.values.get(i))
            .map_or("", |s| {
                if ["Text", "Identifier", "Settings", "Timing"]
                    .iter()
                    .any(|field| name.eq_ignore_ascii_case(field))
                {
                    s.as_str()
                } else {
                    s.trim()
                }
            })
    }
    pub fn set(&mut self, name: &str, value: String) {
        if let Some(i) = self
            .columns
            .iter()
            .position(|s| s.eq_ignore_ascii_case(name))
        {
            self.values[i] = value;
        }
    }
    pub fn start(&self) -> i64 {
        parse_time(self.get("Start")).unwrap_or(0)
    }
    pub fn end(&self) -> i64 {
        parse_time(self.get("End")).unwrap_or(0)
    }
    pub fn times(&mut self, start: i64, end: i64) {
        if start != self.start() {
            self.set("Start", ass_time(start));
        }
        if end != self.end() {
            self.set("End", ass_time(end));
        }
    }
    pub fn comment(&self) -> bool {
        self.prefix
            .trim_start()
            .get(..8)
            .is_some_and(|s| s.eq_ignore_ascii_case("Comment:"))
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    Raw(String),
    Cue(Record),
    Style(Record),
}
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub format: Format,
    pub lines: Vec<Line>,
    newline: String,
    bom: bool,
    trailing: bool,
}

pub fn parse_time(value: &str) -> Option<i64> {
    let normalized = value.trim().replace(',', ".");
    let mut parts = normalized.split(':');
    let h = parts.next()?.parse::<i64>().ok()?;
    let m = parts.next()?.parse::<i64>().ok()?;
    let sec = parts.next()?;
    if parts.next().is_some() || h < 0 || !(0..60).contains(&m) {
        return None;
    }
    let (s, f) = sec.split_once('.').unwrap_or((sec, ""));
    let s = s.parse::<i64>().ok()?;
    if !(0..60).contains(&s) || h > 99_999 || !f.bytes().all(|b| b.is_ascii_digit()) || f.len() > 3
    {
        return None;
    }
    let ms = if f.is_empty() {
        0
    } else {
        f.parse::<i64>().ok()? * 10_i64.pow(3 - f.len() as u32)
    };
    h.checked_mul(3_600_000)?
        .checked_add(m * 60_000 + s * 1000 + ms)
}
pub fn display_time(ms: i64) -> String {
    let n = ms.max(0);
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        n / 3_600_000,
        n / 60_000 % 60,
        n / 1000 % 60,
        n % 1000
    )
}
fn ass_time(ms: i64) -> String {
    let n = (ms.max(0) + 5) / 10;
    format!(
        "{}:{:02}:{:02}.{:02}",
        n / 360_000,
        n / 6000 % 60,
        n / 100 % 60,
        n % 100
    )
}

impl Document {
    pub fn parse(text: &str, format: Format) -> Result<Self, String> {
        let bom = text.starts_with('\u{feff}');
        let text = text.trim_start_matches('\u{feff}');
        let mut doc = Self {
            format,
            lines: Vec::new(),
            newline: if text.contains("\r\n") {
                "\r\n"
            } else if text.contains('\r') {
                "\r"
            } else {
                "\n"
            }
            .into(),
            bom,
            trailing: text.ends_with(['\n', '\r']),
        };
        if format == Format::Vtt {
            doc.parse_vtt(text)?;
        } else if format == Format::Srt {
            let normalized = text.replace("\r\n", "\n");
            for block in normalized.split("\n\n").filter(|s| !s.trim().is_empty()) {
                let mut lines = block.lines();
                let first = lines.next().unwrap_or("");
                let timing = if first.contains("-->") {
                    first
                } else {
                    lines.next().ok_or("Missing SRT timing")?
                };
                let (start, end) = timing.split_once("-->").ok_or("Invalid SRT timing")?;
                let start = parse_time(start).ok_or("Invalid SRT start")?;
                // SRT placement suffixes cannot be silently discarded by this editor.
                let end = parse_time(end).ok_or("Invalid or unsupported SRT end/placement")?;
                let text = lines.collect::<Vec<_>>().join("\n");
                let mut record = Self::default_record();
                record.set("Start", ass_time(start));
                record.set("End", ass_time(end));
                // SRT keeps millisecond precision internally, including values not divisible by 10.
                record.set("Start", display_time(start));
                record.set("End", display_time(end));
                record.set("Text", text);
                doc.lines.push(Line::Cue(record));
            }
        } else {
            let mut section = String::new();
            let mut event_columns = Vec::new();
            let mut style_columns = Vec::new();
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    section = trimmed.to_ascii_lowercase();
                }
                if let Some((key, rest)) = line.split_once(':') {
                    if key.trim().eq_ignore_ascii_case("Format") {
                        let columns = rest.split(',').map(|s| s.trim().to_owned()).collect();
                        match section.as_str() {
                            "[events]" => event_columns = columns,
                            "[v4+ styles]" | "[v4 styles]" => style_columns = columns,
                            _ => {}
                        }
                    }
                    let cue = section == "[events]"
                        && (key.trim().eq_ignore_ascii_case("Dialogue")
                            || key.trim().eq_ignore_ascii_case("Comment"));
                    let style = matches!(section.as_str(), "[v4+ styles]" | "[v4 styles]")
                        && key.trim().eq_ignore_ascii_case("Style");
                    if cue || style {
                        let columns = if cue { &event_columns } else { &style_columns };
                        if columns.is_empty() {
                            return Err("ASS record has no Format declaration".into());
                        }
                        if cue
                            && columns
                                .last()
                                .is_none_or(|s| !s.eq_ignore_ascii_case("Text"))
                        {
                            return Err("ASS event Text must be the final Format column".into());
                        }
                        let space = rest.len() - rest.trim_start_matches([' ', '\t']).len();
                        let values = rest[space..]
                            .splitn(columns.len(), ',')
                            .map(str::to_owned)
                            .collect::<Vec<_>>();
                        if values.len() != columns.len() {
                            return Err("ASS record does not match Format".into());
                        }
                        let record = Record {
                            prefix: format!("{key}:{}", &rest[..space]),
                            columns: columns.clone(),
                            values,
                        };
                        if cue
                            && (parse_time(record.get("Start")).is_none()
                                || parse_time(record.get("End")).is_none())
                        {
                            return Err("Invalid ASS event time".into());
                        }
                        doc.lines.push(if cue {
                            Line::Cue(record)
                        } else {
                            Line::Style(record)
                        });
                        continue;
                    }
                }
                doc.lines.push(Line::Raw(line.to_owned()));
            }
            if !doc
                .lines
                .iter()
                .any(|l| matches!(l,Line::Raw(s) if s.trim().eq_ignore_ascii_case("[Events]")))
            {
                return Err("Missing ASS Events section".into());
            }
        }
        Ok(doc)
    }
    pub fn default_record() -> Record {
        Record {
            prefix: "Dialogue: ".into(),
            columns: [
                "Layer", "Start", "End", "Style", "Name", "MarginL", "MarginR", "MarginV",
                "Effect", "Text",
            ]
            .map(str::to_owned)
            .into(),
            values: [
                "0",
                "0:00:00.00",
                "0:00:02.00",
                "Default",
                "",
                "0",
                "0",
                "0",
                "",
                "",
            ]
            .map(str::to_owned)
            .into(),
        }
    }
    pub fn new_record(&self) -> Record {
        let mut record = Self::default_record();
        if self.format == Format::Ssa {
            record.columns[0] = "Marked".into();
            record.values[0] = "Marked=0".into();
        }
        if self.format == Format::Vtt {
            for name in [
                "Identifier",
                "Settings",
                "Timing",
                "OriginalStart",
                "OriginalEnd",
                "OriginalSettings",
            ] {
                record.columns.push(name.into());
                record.values.push(String::new());
            }
            record.prefix.clear();
        }
        record
    }
    fn parse_vtt(&mut self, text: &str) -> Result<(), String> {
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let rows = normalized.lines().collect::<Vec<_>>();
        if rows.first().is_none_or(|s| {
            !(*s == "WEBVTT" || s.starts_with("WEBVTT ") || s.starts_with("WEBVTT\t"))
        }) {
            return Err("Missing WEBVTT signature".into());
        }
        let timing=regex::Regex::new(r"^[ \t]*((?:[0-9]{2,}:)?[0-9]{2}:[0-9]{2}\.[0-9]{3})[ \t]+-->[ \t]+((?:[0-9]{2,}:)?[0-9]{2}:[0-9]{2}\.[0-9]{3})([ \t].*)?$").unwrap();
        let mut at = 0;
        while at < rows.len() && !rows[at].is_empty() {
            self.lines.push(Line::Raw(rows[at].into()));
            at += 1;
        }
        while at < rows.len() {
            if rows[at].is_empty() {
                self.lines.push(Line::Raw(String::new()));
                at += 1;
                continue;
            }
            let begin = at;
            while at < rows.len() && !rows[at].is_empty() {
                at += 1;
            }
            let block = &rows[begin..at];
            let first = block[0];
            if first == "STYLE"
                || first == "REGION"
                || first == "NOTE"
                || first.starts_with("NOTE ")
                || first.starts_with("NOTE\t")
            {
                self.lines
                    .extend(block.iter().map(|s| Line::Raw((*s).into())));
                continue;
            }
            let (identifier, row) = if timing.is_match(first) {
                ("", 0)
            } else {
                (first, 1)
            };
            let line = block.get(row).ok_or("Invalid WebVTT cue")?;
            let captures = timing.captures(line).ok_or("Invalid WebVTT cue timing")?;
            let start = parse_vtt_time(&captures[1]).ok_or("Invalid WebVTT start")?;
            let end = parse_vtt_time(&captures[2]).ok_or("Invalid WebVTT end")?;
            let settings = captures.get(3).map_or("", |m| m.as_str());
            let mut record = self.new_record();
            for (name, value) in [
                ("Identifier", identifier.to_owned()),
                ("Settings", settings.to_owned()),
                ("Timing", (*line).to_owned()),
                ("OriginalStart", start.to_string()),
                ("OriginalEnd", end.to_string()),
                ("OriginalSettings", settings.to_owned()),
                ("Start", display_time(start)),
                ("End", display_time(end)),
                ("Text", block[row + 1..].join("\n")),
            ] {
                record.set(name, value);
            }
            self.lines.push(Line::Cue(record));
        }
        Ok(())
    }
    pub fn cues(&self) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| matches!(l, Line::Cue(_)).then_some(i))
            .collect()
    }
    pub fn cue(&self, i: usize) -> Option<&Record> {
        match self.lines.get(i)? {
            Line::Cue(r) => Some(r),
            _ => None,
        }
    }
    pub fn cue_mut(&mut self, i: usize) -> Option<&mut Record> {
        match self.lines.get_mut(i)? {
            Line::Cue(r) => Some(r),
            _ => None,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        for line in &self.lines {
            if let Line::Cue(record) | Line::Style(record) = line {
                for (column, value) in record.columns.iter().zip(&record.values) {
                    if (value.contains(['\r', '\n'])
                        && !(!self.format.styled() && column.eq_ignore_ascii_case("Text")))
                        || (self.format.styled()
                            && !column.eq_ignore_ascii_case("Text")
                            && value.contains(','))
                    {
                        return Err(format!(
                            "Invalid line break or comma in ASS field: {column}"
                        ));
                    }
                }
            }
        }
        if self.format == Format::Vtt {
            for i in self.cues() {
                let record = self.cue(i).unwrap();
                if record.get("Identifier").contains("-->")
                    || record.get("Text").contains("-->")
                    || record.get("Settings").contains("-->")
                {
                    return Err("WebVTT cue must not contain --> outside its timing line".into());
                }
                if !record.get("Text").is_empty()
                    && record.get("Text").split('\n').any(str::is_empty)
                {
                    return Err("WebVTT cue payload cannot contain empty lines".into());
                }
            }
        }
        for (n, i) in self.cues().into_iter().enumerate() {
            let r = self.cue(i).unwrap();
            if r.end() <= r.start() {
                return Err(format!("Subtitle {}: end must be after start", n + 1));
            }
        }
        Ok(())
    }
    pub fn serialize(&self) -> String {
        if self.format == Format::Vtt {
            let mut rows = Vec::new();
            for (i, line) in self.lines.iter().enumerate() {
                match line {
                    Line::Raw(raw) => rows.push(raw.clone()),
                    Line::Style(_) => {}
                    Line::Cue(record) => {
                        if !record.get("Identifier").is_empty() {
                            rows.push(record.get("Identifier").to_owned());
                        }
                        let unchanged = record.get("OriginalStart").parse::<i64>().ok()
                            == Some(record.start())
                            && record.get("OriginalEnd").parse::<i64>().ok() == Some(record.end())
                            && record.get("Settings").trim()
                                == record.get("OriginalSettings").trim();
                        rows.push(if unchanged {
                            record.get("Timing").to_owned()
                        } else {
                            format!(
                                "{} --> {}{}",
                                display_time(record.start()),
                                display_time(record.end()),
                                if record.get("Settings").trim().is_empty() {
                                    String::new()
                                } else {
                                    format!(" {}", record.get("Settings").trim())
                                }
                            )
                        });
                        rows.push(record.get("Text").replace('\n', &self.newline));
                        if self
                            .lines
                            .get(i + 1)
                            .is_some_and(|l| !matches!(l,Line::Raw(s) if s.is_empty()))
                        {
                            rows.push(String::new());
                        }
                    }
                }
            }
            let mut text = rows.join(&self.newline);
            if self.trailing {
                text.push_str(&self.newline);
            }
            if self.bom {
                text.insert(0, '\u{feff}');
            }
            return text;
        }
        if self.format == Format::Srt {
            let blocks = self
                .cues()
                .into_iter()
                .enumerate()
                .map(|(n, i)| {
                    let r = self.cue(i).unwrap();
                    format!(
                        "{}\n{} --> {}\n{}",
                        n + 1,
                        display_time(r.start()).replace('.', ","),
                        display_time(r.end()).replace('.', ","),
                        r.get("Text")
                    )
                })
                .collect::<Vec<_>>();
            return format!("{}\n", blocks.join("\n\n")).replace('\n', &self.newline);
        }
        let mut text = self
            .lines
            .iter()
            .map(|line| match line {
                Line::Raw(s) => s.clone(),
                Line::Cue(r) | Line::Style(r) => format!("{}{}", r.prefix, r.values.join(",")),
            })
            .collect::<Vec<_>>()
            .join(&self.newline);
        if self.trailing {
            text.push_str(&self.newline);
        }
        if self.bom {
            text.insert(0, '\u{feff}');
        }
        text
    }
    pub fn to_ass(&self, size: Option<[usize; 2]>) -> String {
        if self.format == Format::Ass {
            return self.serialize();
        }
        if self.format == Format::Ssa {
            return ssa_to_ass(self);
        }
        let [w, h] = size.unwrap_or([1920, 1080]);
        let mut text = format!(
            "[Script Info]\nScriptType: v4.00+\nPlayResX: {w}\nPlayResY: {h}\nWrapStyle: 0\nScaledBorderAndShadow: yes\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Arial,48,&H00FFFFFF,&H000000FF,&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,2,1,2,40,40,40,1\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n"
        );
        for i in self.cues() {
            let r = self.cue(i).unwrap();
            let content = srt_to_ass(&if self.format == Format::Vtt {
                vtt_plain_markup(r.get("Text"))
            } else {
                r.get("Text").to_owned()
            });
            text.push_str(&format!(
                "Dialogue: 0,{},{},Default,,0,0,0,,{content}\n",
                ass_time(r.start()),
                ass_time(r.end())
            ));
        }
        text
    }
    pub fn shift(&mut self, indices: &[usize], delta: i64) {
        let minimum = indices
            .iter()
            .filter_map(|i| self.cue(*i))
            .map(Record::start)
            .min()
            .unwrap_or(0);
        let delta = delta.max(-minimum);
        let format = self.format;
        for &i in indices {
            if let Some(r) = self.cue_mut(i) {
                let start = r.start() + delta;
                let end = r.end() + delta;
                if !format.styled() {
                    r.set("Start", display_time(start));
                    r.set("End", display_time(end));
                } else {
                    r.times(start, end);
                }
            }
        }
    }
    pub fn conversion_warning(&self, target: Format) -> bool {
        static LOSS: OnceLock<regex::Regex> = OnceLock::new();
        let loss = LOSS.get_or_init(|| regex::Regex::new(r"(?i)</?(?:font|s)\b|\{\\an").unwrap());
        self.format != target
            && (self.format.styled()
                || self.format == Format::Vtt
                || target == Format::Vtt
                    && self
                        .cues()
                        .iter()
                        .any(|i| loss.is_match(self.cue(*i).unwrap().get("Text"))))
    }
    pub fn convert(&self, target: Format, size: Option<[usize; 2]>) -> Result<String, String> {
        if target == self.format {
            return Ok(self.serialize());
        }
        match target {
            Format::Ass => Ok(self.to_ass(size)),
            Format::Ssa => Err(
                "Creating legacy SSA from other formats is unsupported; save as ASS instead".into(),
            ),
            Format::Srt => {
                if self.format == Format::Vtt {
                    let mut srt = self.clone();
                    srt.format = Format::Srt;
                    for i in srt.cues() {
                        let r = srt.cue_mut(i).unwrap();
                        r.set("Text", vtt_plain_markup(r.get("Text")));
                    }
                    Ok(srt.serialize())
                } else {
                    convert_to_srt(&self.to_ass(size))
                }
            }
            Format::Vtt => {
                let plain = if self.format == Format::Srt {
                    self.clone()
                } else {
                    Document::parse(&convert_to_srt(&self.to_ass(size))?, Format::Srt)?
                };
                let mut vtt = Document::parse("WEBVTT\n\n", Format::Vtt)?;
                for i in plain.cues() {
                    let old = plain.cue(i).unwrap();
                    let mut r = vtt.new_record();
                    r.set("Start", display_time(old.start()));
                    r.set("End", display_time(old.end()));
                    r.set("Text", srt_to_vtt_markup(old.get("Text")));
                    vtt.lines.push(Line::Cue(r));
                }
                Ok(vtt.serialize())
            }
        }
    }
}

fn parse_vtt_time(text: &str) -> Option<i64> {
    parse_time(&if text.matches(':').count() == 1 {
        format!("00:{text}")
    } else {
        text.to_owned()
    })
}
pub fn ssa_alignment_to_ass(value: usize) -> usize {
    match value {
        4 => 6,
        8 => 3,
        5 => 7,
        6 => 8,
        7 => 9,
        9 => 4,
        10 => 5,
        11 => 6,
        1..=3 => value,
        _ => 2,
    }
}
pub fn ass_alignment_to_ssa(value: usize) -> usize {
    match value {
        7 => 5,
        8 => 6,
        9 => 7,
        4 => 9,
        5 => 10,
        6 => 11,
        1..=3 => value,
        _ => 2,
    }
}
fn vtt_plain_markup(text: &str) -> String {
    static TAGS: OnceLock<regex::Regex> = OnceLock::new();
    TAGS.get_or_init(||regex::Regex::new(r"(?i)</?(?:c(?:\.[^ >]+)?|v(?:[ \t]+[^>]*)?|lang(?:[ \t]+[^>]*)?|ruby|rt)>|<(?:[0-9]{2,}:)?[0-9]{2}:[0-9]{2}\.[0-9]{3}>").unwrap()).replace_all(text,"").into_owned()
}
fn srt_to_vtt_markup(text: &str) -> String {
    static TAGS: OnceLock<regex::Regex> = OnceLock::new();
    let tags =
        TAGS.get_or_init(|| regex::Regex::new(r"(?i)</?([a-z][a-z0-9]*)(?:\s[^>]*)?>").unwrap());
    let text = tags.replace_all(text, |caps: &regex::Captures<'_>| {
        let tag = caps[1].to_ascii_lowercase();
        if matches!(tag.as_str(), "b" | "i" | "u") {
            format!(
                "<{}{tag}>",
                if caps[0].starts_with("</") { "/" } else { "" }
            )
        } else {
            String::new()
        }
    });
    static ALIGN: OnceLock<regex::Regex> = OnceLock::new();
    ALIGN
        .get_or_init(|| regex::Regex::new(r"\{\\an[1-9]\}").unwrap())
        .replace_all(&text, "")
        .into_owned()
}
fn ssa_to_ass(document: &Document) -> String {
    const STYLE: &str = "Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding";
    const EVENTS: &str = "Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text";
    let mut section = String::new();
    let mut out = Vec::new();
    for line in &document.lines {
        match line {
            Line::Raw(raw) => {
                let trimmed = raw.trim();
                if trimmed.starts_with('[') {
                    section = trimmed.to_ascii_lowercase();
                }
                out.push(if section == "[v4 styles]" && trimmed.starts_with('[') {
                    "[V4+ Styles]".into()
                } else if raw
                    .split_once(':')
                    .is_some_and(|(key, _)| key.trim().eq_ignore_ascii_case("ScriptType"))
                {
                    "ScriptType: v4.00+".into()
                } else if raw
                    .split_once(':')
                    .is_some_and(|(key, _)| key.trim().eq_ignore_ascii_case("Format"))
                    && section == "[v4 styles]"
                {
                    format!("Format: {STYLE}")
                } else if raw
                    .split_once(':')
                    .is_some_and(|(key, _)| key.trim().eq_ignore_ascii_case("Format"))
                    && section == "[events]"
                {
                    format!("Format: {EVENTS}")
                } else {
                    raw.clone()
                });
            }
            Line::Style(record) => {
                let values = STYLE
                    .split(',')
                    .map(str::trim)
                    .map(|field| {
                        let source = if field == "OutlineColour" {
                            "BackColour"
                        } else {
                            field
                        };
                        match field {
                            "ScaleX" | "ScaleY" => "100".into(),
                            "Underline" | "StrikeOut" | "Spacing" | "Angle" => "0".into(),
                            "Alignment" => {
                                ssa_alignment_to_ass(record.get("Alignment").parse().unwrap_or(2))
                                    .to_string()
                            }
                            "PrimaryColour" | "SecondaryColour" | "OutlineColour"
                            | "BackColour" => {
                                let raw = record.get(source);
                                let color = if raw.get(..2).is_some_and(|prefix| {
                                    prefix.eq_ignore_ascii_case("&h")
                                        || prefix.eq_ignore_ascii_case("0x")
                                }) {
                                    u32::from_str_radix(raw[2..].trim_end_matches('&'), 16)
                                        .unwrap_or(0)
                                } else {
                                    raw.parse::<i64>().unwrap_or(0) as u32
                                };
                                let alpha = if field == "BackColour" {
                                    128
                                } else {
                                    record
                                        .get("AlphaLevel")
                                        .parse::<u32>()
                                        .unwrap_or(0)
                                        .min(255)
                                };
                                format!("&H{:08X}", (color & 0x00ffffff) | (alpha << 24))
                            }
                            _ => record.get(field).to_owned(),
                        }
                    })
                    .collect::<Vec<_>>();
                out.push(format!("Style: {}", values.join(",")));
            }
            Line::Cue(record) => {
                let values = EVENTS
                    .split(',')
                    .map(str::trim)
                    .map(|field| {
                        if field == "Layer" {
                            "0".into()
                        } else {
                            record.get(field).to_owned()
                        }
                    })
                    .collect::<Vec<_>>();
                out.push(format!("{}{}", record.prefix, values.join(",")));
            }
        }
    }
    out.join("\n") + "\n"
}

fn srt_to_ass(text: &str) -> String {
    static ENTITY: OnceLock<regex::Regex> = OnceLock::new();
    let entity = ENTITY.get_or_init(|| {
        regex::Regex::new(r"&(#x[0-9a-fA-F]+|#[0-9]+|amp|lt|gt|quot|apos|nbsp);").unwrap()
    });
    let text = entity.replace_all(text, |caps: &regex::Captures<'_>| match &caps[1] {
        "amp" => "&".into(),
        "lt" => "<".into(),
        "gt" => ">".into(),
        "quot" => "\"".into(),
        "apos" => "'".into(),
        "nbsp" => "\u{a0}".into(),
        value => {
            let number = if let Some(hex) = value.strip_prefix("#x") {
                u32::from_str_radix(hex, 16).ok()
            } else {
                value.strip_prefix('#').and_then(|v| v.parse().ok())
            };
            number
                .and_then(char::from_u32)
                .map_or_else(|| caps[0].to_owned(), |c| c.to_string())
        }
    });
    // Literal ASS control characters in SRT must not become executable tags.
    let text = text
        .replace('\\', "\\\\")
        .replace('{', "\\{")
        .replace('}', "\\}");
    let mut text = text;
    static TAGS: OnceLock<regex::Regex> = OnceLock::new();
    static ATTRIBUTES: OnceLock<regex::Regex> = OnceLock::new();
    let tags = TAGS
        .get_or_init(|| regex::Regex::new(r"(?i)</?\s*(?:i|b|u|s|font)(?:\s+[^>]*)?\s*>").unwrap());
    let attributes = ATTRIBUTES.get_or_init(|| {
        regex::Regex::new(r#"(?i)(color|size|face)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#)
            .unwrap()
    });
    text = tags
        .replace_all(&text, |caps: &regex::Captures<'_>| {
            let tag = caps[0].to_ascii_lowercase();
            let closing = tag.starts_with("</");
            if tag.contains("font") {
                if closing {
                    return "{\\c\\fs\\fn}".to_owned();
                }
                let mut style = String::new();
                for attr in attributes.captures_iter(&caps[0]) {
                    let value = attr
                        .get(2)
                        .or_else(|| attr.get(3))
                        .or_else(|| attr.get(4))
                        .map_or("", |m| m.as_str());
                    match attr[1].to_ascii_lowercase().as_str() {
                        "color" => {
                            let hex = value.trim_start_matches('#');
                            if hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                                style.push_str(&format!(
                                    "\\c&H{}{}{}&",
                                    &hex[4..6],
                                    &hex[2..4],
                                    &hex[0..2]
                                ));
                            }
                        }
                        "size" => {
                            if let Ok(size) = value.parse::<u32>() {
                                if (1..=500).contains(&size) {
                                    style.push_str(&format!("\\fs{size}"));
                                }
                            }
                        }
                        "face" => {
                            if !value.contains(['\\', '{', '}', ',']) {
                                style.push_str(&format!("\\fn{value}"));
                            }
                        }
                        _ => {}
                    }
                }
                if style.is_empty() {
                    String::new()
                } else {
                    format!("{{{style}}}")
                }
            } else {
                let name = tag
                    .trim_start_matches(['<', '/', ' '])
                    .chars()
                    .next()
                    .unwrap();
                format!("{{\\{name}{}}}", if closing { 0 } else { 1 })
            }
        })
        .into_owned();
    text.replace('\n', "\\N")
}

// libavcodec otherwise exposes vector drawing commands as visible SRT text.
fn without_drawings(text: &str) -> String {
    let tags = regex::Regex::new(r"\{[^}]*\}").unwrap();
    let mode = regex::Regex::new(r"\\p([0-9]+)\b").unwrap();
    let mut drawing = false;
    let mut result = String::new();
    let mut previous = 0;
    for tag in tags.find_iter(text) {
        if !drawing {
            result.push_str(&text[previous..tag.start()]);
        }
        for captures in mode.captures_iter(tag.as_str()) {
            drawing = captures[1].parse::<u32>().unwrap_or(0) != 0;
        }
        let kept = mode.replace_all(tag.as_str(), "");
        if kept != "{}" {
            result.push_str(&kept);
        }
        previous = tag.end();
    }
    if !drawing {
        result.push_str(&text[previous..]);
    }
    result
}

/// Conservative conversion through the subtitle codec, never through player styling.
pub fn convert_to_srt(ass: &str) -> Result<String, String> {
    use std::io::Write;
    let mut document = Document::parse(ass, Format::Ass)?;
    let tags = regex::Regex::new(r"\{[^}]*\}").unwrap();
    document.lines.retain_mut(|line| {
        if let Line::Cue(record) = line {
            if record.comment() {
                return false;
            }
            let text = without_drawings(record.get("Text"));
            if tags
                .replace_all(&text, "")
                .replace("\\N", "")
                .trim()
                .is_empty()
            {
                return false;
            }
            record.set("Text", text);
        }
        true
    });
    if document.cues().is_empty() {
        return Ok(String::new());
    }
    let ffmpeg = crate::media_tools::find_tool("ffmpeg").ok_or("FFmpeg unavailable")?;
    let mut child = crate::media_tools::command(&ffmpeg)
        .args([
            "-nostdin", "-v", "error", "-f", "ass", "-i", "pipe:0", "-map", "0:s:0", "-c:s", "srt",
            "-f", "srt", "pipe:1",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut input = child.stdin.take().ok_or("Missing FFmpeg stdin")?;
    let bytes = document.serialize().into_bytes();
    let writer = std::thread::spawn(move || input.write_all(&bytes));
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    writer
        .join()
        .map_err(|_| "Subtitle conversion writer failed")?
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }
    let text = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    Ok(regex::Regex::new(r"\{\\an[1-9]\}")
        .unwrap()
        .replace_all(&text, "")
        .into_owned())
}

pub fn read(path: &Path) -> Result<(Document, Vec<u8>), String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 32 * 1024 * 1024 {
        return Err("Subtitle exceeds 32 MiB".into());
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("Subtitle exceeds 32 MiB".into());
    }
    let text = if bytes.starts_with(&[0xff, 0xfe]) {
        encoding_rs::UTF_16LE
            .decode_with_bom_removal(&bytes)
            .0
            .into_owned()
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        encoding_rs::UTF_16BE
            .decode_with_bom_removal(&bytes)
            .0
            .into_owned()
    } else {
        String::from_utf8(bytes.clone())
            .map_err(|_| "Subtitle is not UTF-8/UTF-16; convert its encoding before editing")?
    };
    let format = Format::detect(&text, path)?;
    Ok((Document::parse(&text, format)?, bytes))
}

/// Write a sibling first, check external changes, keep one backup, then replace atomically.
pub fn save(path: &Path, bytes: &[u8], expected: Option<&[u8]>) -> Result<(), String> {
    use std::io::Write;
    if let Some(expected) = expected {
        if std::fs::read(path).map_err(|e| e.to_string())? != expected {
            return Err("Subtitle changed externally; reload or save to another path".into());
        }
    }
    let parent = path.parent().ok_or("Missing subtitle folder")?;
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temporary = parent.join(format!(
        ".uyv-subtitle-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        if let Some(expected) = expected {
            if std::fs::read(path).map_err(|e| e.to_string())? != expected {
                return Err("Subtitle changed externally".into());
            }
        }
        if path.exists() {
            let mut backup = path.as_os_str().to_os_string();
            backup.push(".bak");
            std::fs::copy(path, PathBuf::from(backup)).map_err(|e| e.to_string())?;
        }
        crate::playlist::replace(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn saved_revisions() -> &'static std::sync::Mutex<std::collections::HashMap<PathBuf, u64>> {
    static REVISIONS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<PathBuf, u64>>,
    > = std::sync::OnceLock::new();
    REVISIONS.get_or_init(Default::default)
}
pub fn saved_revision(path: &Path) -> u64 {
    let key = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
    saved_revisions()
        .lock()
        .unwrap()
        .get(&key)
        .copied()
        .unwrap_or(0)
}
pub fn notify_saved(path: &Path) {
    let key = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
    let mut revisions = saved_revisions().lock().unwrap();
    *revisions.entry(key).or_default() += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plain_text_backslash_n_is_not_a_line_break() {
        for (format, text) in [
            (
                Format::Srt,
                "1\n00:00:01,123 --> 00:00:02,987\nLiteral \\N\nsecond\n",
            ),
            (
                Format::Vtt,
                "WEBVTT\n\n00:01.123 --> 00:02.987\nLiteral \\N\nsecond\n",
            ),
        ] {
            let mut doc = Document::parse(text, format).unwrap();
            assert_eq!(doc.serialize(), text);
            let i = doc.cues()[0];
            doc.cue_mut(i)
                .unwrap()
                .set("Text", "Edited \\N\nsecond".into());
            let output = doc.serialize();
            assert!(output.contains("Edited \\N\nsecond"));
        }
    }
    #[test]
    fn webvtt_legacy_newlines_and_plain_markup_conversion() {
        let text = "WEBVTT\r\r00:01.123 --> 00:02.987\r<i>hello</i>\r";
        let doc = Document::parse(text, Format::Vtt).unwrap();
        assert_eq!(doc.serialize(), text);
        let srt = Document::parse(
            "1\n00:00:01,123 --> 00:00:02,987\n<font color=\"red\"><B>hello</B></font>\n",
            Format::Srt,
        )
        .unwrap();
        assert!(srt.conversion_warning(Format::Vtt));
        let converted = srt.convert(Format::Vtt, None).unwrap();
        assert!(converted.contains("<b>hello</b>"));
        assert!(!converted.contains("<font"));
    }
    #[test]
    fn webvtt_roundtrip_preserves_blocks_ids_settings_and_shorthand() {
        let text = "\u{feff}WEBVTT example\r\nLanguage: en\r\n\r\nSTYLE\r\n::cue(.red) { color: red; }\r\n\r\nREGION\r\nid:one\r\nwidth:40%\r\n\r\nNOTE metadata --> preserved\r\nnotes\r\n\r\ncue-one\r\n00:01.123 --> 00:03.987 line:20%,start position:10%,line-left align:start region:one\r\n<v Alice><c.red><i>Hello</i></c></v>\r\nsecond\r\n\r\n00:04.001 --> 00:06.009\r\nWorld\r\n";
        let mut doc = Document::parse(text, Format::Vtt).unwrap();
        assert_eq!(doc.serialize(), text);
        doc.validate().unwrap();
        let indices = doc.cues();
        doc.shift(&indices, 1);
        let edited = doc.serialize();
        assert!(edited.contains("cue-one\r\n00:00:01.124 --> 00:00:03.988 line:20%,start position:10%,line-left align:start region:one"));
        assert!(edited.contains("::cue(.red) { color: red; }"));
        let ass = doc.to_ass(None);
        assert!(ass.contains("{\\i1}Hello{\\i0}\\Nsecond"));
        assert!(!ass.contains("<v Alice>"));
    }
    #[test]
    fn webvtt_conversion_keeps_milliseconds_and_warns_about_metadata() {
        let vtt = Document::parse(
            "WEBVTT\n\nid\n00:01.123 --> 00:02.987 line:50%\n<v Alice>Hello</v>\n",
            Format::Vtt,
        )
        .unwrap();
        assert!(vtt.conversion_warning(Format::Srt));
        assert!(!vtt.conversion_warning(Format::Vtt));
        let srt = vtt.convert(Format::Srt, None).unwrap();
        assert_eq!(srt, "1\n00:00:01,123 --> 00:00:02,987\nHello\n");
        let doc = Document::parse(&srt, Format::Srt).unwrap();
        let vtt = doc.convert(Format::Vtt, None).unwrap();
        assert!(vtt.contains("00:00:01.123 --> 00:00:02.987"));
    }
    #[test]
    fn content_detection_precedes_filename_extension() {
        assert_eq!(
            Format::detect("WEBVTT\n\n", Path::new("wrong.srt")).unwrap(),
            Format::Vtt
        );
        assert_eq!(
            Format::detect("[Script Info]\n[V4 Styles]\n", Path::new("wrong.ass")).unwrap(),
            Format::Ssa
        );
        assert_eq!(
            Format::detect(
                "1\n00:00:01,000 --> 00:00:02,000\nhello\n",
                Path::new("wrong.vtt")
            )
            .unwrap(),
            Format::Srt
        );
    }
    #[test]
    fn ssa_roundtrip_and_preview_preserve_legacy_meaning() {
        let text = "[Script Info]\nScriptType: v4.00\nPlayResX: 640\nPlayResY: 360\n[V4 Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, TertiaryColour, BackColour, Bold, Italic, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, AlphaLevel, Encoding\nStyle: Sign,Arial,28,16777215,255,65280,0,-1,0,1,2,1,6,10,20,30,32,1\n[Events]\nFormat: Marked, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: Marked=1,0:00:01.00,0:00:02.00,Sign,actor,11,22,33,,{\\a6}hello,world\n";
        let mut doc = Document::parse(text, Format::Ssa).unwrap();
        assert_eq!(doc.serialize(), text);
        let i = doc.cues()[0];
        doc.shift(&[i], 100);
        assert!(doc.serialize().contains(
            "Dialogue: Marked=1,0:00:01.10,0:00:02.10,Sign,actor,11,22,33,,{\\a6}hello,world"
        ));
        let ass = Document::parse(&doc.to_ass(None), Format::Ass).unwrap();
        let style = ass
            .lines
            .iter()
            .find_map(|l| {
                if let Line::Style(r) = l {
                    Some(r)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(style.get("Alignment"), "8");
        assert_eq!(style.get("OutlineColour"), "&H20000000");
        assert_eq!(style.get("BackColour"), "&H80000000");
    }
    #[test]
    fn legacy_alignment_mapping_does_not_confuse_ass_numpad() {
        for n in 1..=9 {
            assert_eq!(ssa_alignment_to_ass(ass_alignment_to_ssa(n)), n);
        }
        assert_eq!(ssa_alignment_to_ass(4), 6);
        assert_eq!(ssa_alignment_to_ass(8), 3);
        assert!(
            Document::parse("", Format::Srt)
                .unwrap()
                .convert(Format::Ssa, None)
                .is_err()
        );
    }
    #[test]
    fn drawing_conversion_discards_vectors_and_keeps_following_text() {
        assert_eq!(
            without_drawings("{\\p1}m 0 0 l 20 0{\\p0}visible"),
            "visible"
        );
        assert_eq!(
            without_drawings("before{\\an7\\p1}m 0 0{\\p0}after"),
            "before{\\an7}after"
        );
        assert_eq!(without_drawings("{\\p1}m 0 0"), "");
    }
    #[test]
    fn srt_html_font_and_entities_convert_without_literal_html() {
        let text = srt_to_ass("<FONT color=\"#123456\" size=\"24\"><I>A &amp; B</I></FONT>");
        assert_eq!(text, "{\\c&H563412&\\fs24}{\\i1}A & B{\\i0}{\\c\\fs\\fn}");
    }
    #[test]
    fn ass_text_spaces_and_lowercase_comments_survive() {
        let text = "[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\ncomment: 0,0:00:00.00,0:00:01.00,Default,,0,0,0,,  keep spaces  \n";
        let doc = Document::parse(text, Format::Ass).unwrap();
        assert!(doc.cue(doc.cues()[0]).unwrap().comment());
        assert_eq!(
            doc.cue(doc.cues()[0]).unwrap().get("Text"),
            "  keep spaces  "
        );
        assert_eq!(doc.serialize(), text);
    }
    #[test]
    fn save_replaces_after_backup_and_preserves_external_original() {
        let path =
            std::env::temp_dir().join(format!("uyv-subtitle-backup-{}.srt", std::process::id()));
        std::fs::write(&path, b"original").unwrap();
        save(&path, b"edited", Some(b"original")).unwrap();
        let backup = PathBuf::from(format!("{}.bak", path.display()));
        assert_eq!(std::fs::read(&path).unwrap(), b"edited");
        assert_eq!(std::fs::read(&backup).unwrap(), b"original");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(backup).unwrap();
    }
    #[test]
    fn ass_roundtrip_preserves_sections_and_effects() {
        let text = "\u{feff}[Script Info]\r\nPlayResX: 1280\r\n[V4+ Styles]\r\nFormat: Name, Fontname\r\nStyle: Sign,Arial\r\n[Events]\r\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\r\nComment: 1,0:00:01.00,0:00:02.00,Sign,,0,0,0,,note\r\nDialogue: 2,0:00:01.23,0:00:03.45,Sign,actor,10,20,30,Banner,{\\move(1,2,3,4)\\k20}hello,world\r\n[Fonts]\r\nfontname: attachment\r\nunknown-data\r\n";
        let mut doc = Document::parse(text, Format::Ass).unwrap();
        assert_eq!(doc.serialize(), text);
        let i = *doc.cues().last().unwrap();
        doc.cue_mut(i).unwrap().times(2000, 4000);
        let changed = doc.serialize();
        assert!(changed.contains(
            "2,0:00:02.00,0:00:04.00,Sign,actor,10,20,30,Banner,{\\move(1,2,3,4)\\k20}hello,world"
        ));
        assert!(changed.ends_with("unknown-data\r\n"));
    }
    #[test]
    fn srt_keeps_milliseconds_and_common_markup() {
        let text = "1\n00:00:01,123 --> 00:00:02,987\n<i>hello</i>\nworld\n";
        let mut doc = Document::parse(text, Format::Srt).unwrap();
        assert_eq!(doc.serialize(), text);
        let i = doc.cues()[0];
        doc.shift(&[i], 100);
        assert_eq!(doc.cue(i).unwrap().start(), 1223);
        let ass = doc.to_ass(None);
        assert!(ass.contains("{\\i1}hello{\\i0}\\Nworld"));
    }
    #[test]
    fn negative_shift_preserves_relative_spacing() {
        let mut doc = Document::parse(
            "1\n00:00:01,000 --> 00:00:02,000\none\n\n2\n00:00:03,000 --> 00:00:04,000\ntwo\n",
            Format::Srt,
        )
        .unwrap();
        let indices = doc.cues();
        doc.shift(&indices, -5000);
        assert_eq!(doc.cue(indices[0]).unwrap().start(), 0);
        assert_eq!(doc.cue(indices[1]).unwrap().start(), 2000);
    }
    #[test]
    fn source_conflict_does_not_overwrite() {
        let path =
            std::env::temp_dir().join(format!("uyv-subtitle-conflict-{}.srt", std::process::id()));
        std::fs::write(&path, b"external").unwrap();
        assert!(save(&path, b"edited", Some(b"original")).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"external");
        std::fs::remove_file(path).unwrap();
    }
}
