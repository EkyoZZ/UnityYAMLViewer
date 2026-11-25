//! Photoshop type-tool data for horizontal point and paragraph text.
use super::{Text, character, paragraph::Align};
use std::fmt::Write;

#[derive(Clone)]
enum Engine {
    Dict(Vec<(String, Engine)>),
    List(Vec<Engine>),
    Text(String),
    Literal(Vec<u8>),
    Atom(String),
    Name(String),
}

impl Engine {
    fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Dict(values) => values
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut Self> {
        let Self::Dict(values) = self else {
            return None;
        };
        values
            .iter_mut()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    fn set(&mut self, key: &str, value: Self) -> Option<()> {
        if let Some(existing) = self.get_mut(key) {
            *existing = value;
        } else if let Self::Dict(values) = self {
            values.push((key.into(), value));
        } else {
            return None;
        }
        Some(())
    }

    fn list_mut(&mut self) -> Option<&mut Vec<Self>> {
        let Self::List(values) = self else {
            return None;
        };
        Some(values)
    }

    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Self::Dict(entries) => {
                out.extend(b"<<");
                encode_entries(entries, out);
                out.extend(b" >>");
            }
            Self::List(values) => {
                out.push(b'[');
                for value in values {
                    out.push(b' ');
                    value.encode(out);
                }
                out.extend(b" ]");
            }
            Self::Text(value) => literal(out, value),
            Self::Literal(value) => out.extend(value),
            Self::Atom(value) => out.extend(value.as_bytes()),
            Self::Name(value) => {
                out.push(b'/');
                out.extend(value.as_bytes());
            }
        }
    }

    fn list(&self) -> Option<&[Self]> {
        match self {
            Self::List(values) => Some(values),
            _ => None,
        }
    }

    fn text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }

    fn number(&self) -> Option<f64> {
        match self {
            Self::Atom(value) => value.parse::<f64>().ok().filter(|n| n.is_finite()),
            _ => None,
        }
    }

    fn flag(&self) -> bool {
        matches!(self, Self::Atom(value) if value == "true")
    }
}

fn token(input: &mut &[u8]) -> Option<String> {
    let len = input
        .iter()
        .take_while(|&&b| !b.is_ascii_whitespace() && !b"<>[]()/".contains(&b))
        .count();
    if len == 0 {
        return None;
    }
    let (word, rest) = input.split_at(len);
    *input = rest;
    Some(std::str::from_utf8(word).ok()?.into())
}

fn ws(input: &mut &[u8]) {
    *input = input.trim_ascii_start();
}

fn parse(input: &mut &[u8], level: u8) -> Option<Engine> {
    if level == 40 {
        return None;
    }
    ws(input);
    if let Some(rest) = input.strip_prefix(b"<<") {
        *input = rest;
        let mut values = Vec::new();
        loop {
            ws(input);
            if let Some(rest) = input.strip_prefix(b">>") {
                *input = rest;
                return Some(Engine::Dict(values));
            }
            *input = input.strip_prefix(b"/")?;
            let key = token(input)?;
            values.push((key, parse(input, level + 1)?));
        }
    }
    if let Some(rest) = input.strip_prefix(b"[") {
        *input = rest;
        let mut values = Vec::new();
        loop {
            ws(input);
            if let Some(rest) = input.strip_prefix(b"]") {
                *input = rest;
                return Some(Engine::List(values));
            }
            values.push(parse(input, level + 1)?);
        }
    }
    if let Some(rest) = input.strip_prefix(b"(") {
        let source = *input;
        *input = rest;
        let mut bytes = Vec::new();
        loop {
            let (&byte, rest) = input.split_first()?;
            *input = rest;
            match byte {
                b')' => break,
                b'\\' => {
                    let (&escaped, rest) = input.split_first()?;
                    *input = rest;
                    bytes.push(escaped);
                }
                _ => bytes.push(byte),
            }
        }
        let value = if let Some(wide) = bytes.strip_prefix(b"\xfe\xff") {
            if wide.len() % 2 != 0 {
                return None;
            }
            String::from_utf16(
                &wide
                    .chunks_exact(2)
                    .map(|b| u16::from_be_bytes([b[0], b[1]]))
                    .collect::<Vec<_>>(),
            )
            .ok()
        } else {
            String::from_utf8(bytes).ok()
        };
        return Some(
            value
                .map(Engine::Text)
                .unwrap_or_else(|| Engine::Literal(source[..source.len() - input.len()].to_vec())),
        );
    }
    if let Some(rest) = input.strip_prefix(b"/") {
        *input = rest;
        return token(input).map(Engine::Name);
    }
    token(input).map(Engine::Atom)
}

fn encode_entries(entries: &[(String, Engine)], out: &mut Vec<u8>) {
    for (key, value) in entries {
        out.extend(b" /");
        out.extend(key.as_bytes());
        out.push(b' ');
        value.encode(out);
    }
}

fn engine_data(data: &[u8]) -> Option<Engine> {
    let address = data.windows(14).position(|key| key == b"EngineDatatdta")? + 14;
    let len = u32::from_be_bytes(data.get(address..address + 4)?.try_into().ok()?) as usize;
    parse(&mut data.get(address + 4..address + 4 + len)?, 0)
}

/// Update numbered Photoshop text objects using the same EngineData written to TySh.
pub(in crate::asset_viewer::graphic_editor) fn update_txt2(
    mut source: &[u8],
    changes: &[(usize, &[u8])],
) -> Option<Vec<u8>> {
    if source.len() > 4 * 1024 * 1024 {
        return None;
    }
    let mut entries = Vec::new();
    while !source.trim_ascii_start().is_empty() {
        ws(&mut source);
        source = source.strip_prefix(b"/")?;
        entries.push((token(&mut source)?, parse(&mut source, 0)?));
    }
    let mut root = Engine::Dict(entries);
    // Older, named text-engine records have a different model and font table.
    if root.get("DocumentObjects").is_some() {
        return None;
    }
    let mut fonts = root.get("0")?.get("1")?.get("0")?.list()?.to_vec();
    let objects = root.get_mut("1")?.get_mut("1")?.list_mut()?;
    for &(index, tysh) in changes {
        update_text_object(objects.get_mut(index)?, &mut fonts, &engine_data(tysh)?)?;
    }
    root.get_mut("0")?
        .get_mut("1")?
        .set("0", Engine::List(fonts))?;
    let Engine::Dict(entries) = root else {
        return None;
    };
    let mut out = Vec::new();
    encode_entries(&entries, &mut out);
    Some(out)
}

fn font_name(entry: &Engine) -> Option<&str> {
    entry.get("0")?.get("0")?.get("0")?.text()
}

fn update_text_object(object: &mut Engine, fonts: &mut Vec<Engine>, data: &Engine) -> Option<()> {
    let local_fonts = data.get("ResourceDict")?.get("FontSet")?.list()?;
    let engine = data.get("EngineDict")?;
    let text = engine.get("Editor")?.get("Text")?.text()?;
    let model = object.get_mut("0")?;
    model.get("0")?.text()?;
    model.set("0", Engine::Text(text.to_owned()))?;
    let sources = [
        (
            "ParagraphRun",
            "5",
            &[
                ("Justification", "0"),
                ("FirstLineIndent", "1"),
                ("StartIndent", "2"),
                ("EndIndent", "3"),
                ("SpaceBefore", "4"),
                ("SpaceAfter", "5"),
                ("AutoLeading", "7"),
            ][..],
        ),
        (
            "StyleRun",
            "6",
            &[
                ("FontSize", "1"),
                ("FauxBold", "2"),
                ("FauxItalic", "3"),
                ("AutoLeading", "4"),
                ("Tracking", "8"),
            ][..],
        ),
    ];
    for (source_key, target_key, properties) in sources {
        let source = engine.get(source_key)?;
        let runs = source.get("RunArray")?.list()?;
        let lengths = source.get("RunLengthArray")?.list()?;
        let target = model.get_mut(target_key)?.get_mut("0")?.list_mut()?;
        let template = target.first()?;
        if runs.len() != lengths.len() || runs.len() > 4096 {
            return None;
        }
        let mut updated = Vec::with_capacity(runs.len());
        for (i, (source, length)) in runs.iter().zip(lengths).enumerate() {
            let mut run = target.get(i).unwrap_or(template).clone();
            run.set("1", length.clone())?;
            let entry = run.get_mut("0")?.get_mut("0")?;
            let (source, style) = if target_key == "5" {
                (
                    source.get("ParagraphSheet")?.get("Properties")?,
                    entry.get_mut("5")?,
                )
            } else {
                (
                    source.get("StyleSheet")?.get("StyleSheetData")?,
                    entry.get_mut("6")?,
                )
            };
            for &(from, to) in properties {
                style.set(to, source.get(from)?.clone())?;
            }
            if target_key == "6" {
                let font = source.get("Font")?.number()? as usize;
                let name = local_fonts.get(font)?.get("Name")?.text()?;
                let font = if let Some(index) = fonts
                    .iter()
                    .position(|entry| font_name(entry) == Some(name))
                {
                    index
                } else {
                    let mut entry = fonts
                        .iter()
                        .find(|entry| font_name(entry).is_some())?
                        .clone();
                    let font = entry.get_mut("0")?.get_mut("0")?;
                    font.set("0", Engine::Text(name.to_owned()))?;
                    if let Engine::Dict(fields) = font {
                        fields.retain(|(key, _)| matches!(key.as_str(), "0" | "2"));
                    }
                    fonts.push(entry);
                    fonts.len() - 1
                };
                style.set("0", Engine::Atom(font.to_string()))?;
                let paint = style.get_mut("53")?.get_mut("0")?;
                let color = source.get("FillColor")?;
                paint.set("0", color.get("Type")?.clone())?;
                paint.set("1", color.get("Values")?.clone())?;
            }
            updated.push(run);
        }
        *target = updated;
    }
    // Shaped glyph positions belong to the previous text and style.
    if let Engine::Dict(fields) = object {
        fields.retain(|(key, _)| key != "21");
    }
    if let Some(Engine::Dict(view)) = object.get_mut("1") {
        view.retain(|(key, _)| key != "2");
    }
    Some(())
}

const ALIGNMENTS: [Align; 7] = [
    Align::Left,
    Align::Right,
    Align::Center,
    Align::JustifyLeft,
    Align::JustifyRight,
    Align::JustifyCenter,
    Align::JustifyAll,
];

pub(in crate::asset_viewer::graphic_editor) fn text_index(data: &[u8]) -> Option<i32> {
    let key = b"\0\0\0\x09TextIndexlong";
    let offset = data.windows(key.len()).position(|bytes| bytes == key)? + key.len();
    let index = i32::from_be_bytes(data.get(offset..offset + 4)?.try_into().ok()?);
    (index >= 0).then_some(index)
}

fn number(node: &Engine, key: &str, fallback: f64) -> f64 {
    node.get(key).and_then(Engine::number).unwrap_or(fallback)
}

fn style(node: &Engine, fonts: &[Engine], base: &Text) -> character::Style {
    let family = node
        .get("Font")
        .and_then(Engine::number)
        .and_then(|n| fonts.get(n as usize))
        .and_then(|font| font.get("Name"))
        .and_then(Engine::text)
        .map(str::to_owned);
    let size = number(node, "FontSize", f64::from(base.size))
        .round()
        .clamp(1., 256.) as u16;
    let color = node
        .get("FillColor")
        .and_then(|c| c.get("Values"))
        .and_then(Engine::list)
        .and_then(|c| {
            Some([
                c.get(1)?.number()?,
                c.get(2)?.number()?,
                c.get(3)?.number()?,
            ])
        })
        .map(|c| c.map(|n| (n * 255.).round().clamp(0., 255.) as u8));
    character::Style {
        family,
        size: Some(size),
        color,
    }
}

pub(in crate::asset_viewer::graphic_editor) fn read(data: &[u8], canvas: [u32; 2]) -> Option<Text> {
    if data.len() > 1024 * 1024
        || !data.starts_with(&1u16.to_be_bytes())
        || data.get(50..56) != Some(b"\x00\x32\x00\x00\x00\x10")
    {
        return None;
    }
    let transform: [f64; 6] = std::array::from_fn(|i| {
        data.get(2 + i * 8..10 + i * 8)
            .and_then(|v| v.try_into().ok())
            .map(f64::from_be_bytes)
            .unwrap_or(f64::NAN)
    });
    if transform.iter().any(|v| !v.is_finite() || v.abs() > 65536.) {
        return None;
    }
    let root = engine_data(data)?;
    let engine = root.get("EngineDict")?;
    let resources = root.get("ResourceDict")?;
    let fonts = resources.get("FontSet")?.list()?;
    let content = engine.get("Editor")?.get("Text")?.text()?;
    let content = content
        .strip_suffix('\r')
        .unwrap_or(content)
        .replace('\r', "\n");
    let mut text = Text::new(60, [0, 0, 0, 255]);
    text.content = content;
    text.angle = 0;
    text.repeat = false;
    text.linear = [
        transform[0] as f32,
        transform[1] as f32,
        transform[2] as f32,
        transform[3] as f32,
    ];
    let styles = engine.get("StyleRun")?;
    fn run(node: &Engine) -> Option<&[Engine]> {
        node.get("RunArray")?.list()
    }
    fn style_data(node: &Engine) -> Option<&Engine> {
        node.get("StyleSheet")?.get("StyleSheetData")
    }
    let nodes = run(styles)?;
    let first = style_data(nodes.first()?)?;
    let base = style(first, fonts, &text);
    text.family = base.family?;
    text.size = base.size?;
    text.color = base.color.unwrap_or(text.color);
    text.bold = first.get("FauxBold").is_some_and(Engine::flag);
    text.italic = first.get("FauxItalic").is_some_and(Engine::flag);
    text.tracking = (number(first, "Tracking", 0.) * f64::from(text.size) / 1000.)
        .round()
        .clamp(-32., 128.) as i16;
    let lengths = styles.get("RunLengthArray")?.list()?;
    if lengths.len() != nodes.len() {
        return None;
    }
    let mut pending = text.content.chars().peekable();
    let mut spans = Vec::new();
    for (node, len) in nodes.iter().zip(lengths) {
        let data = style_data(node)?;
        let mut current = style(data, fonts, &text);
        current.family = current.family.filter(|name| name != &text.family);
        current.size = current.size.filter(|size| *size != text.size);
        current.color = current.color.filter(|color| *color != text.color);
        let mut remaining = len.number()? as usize;
        while let Some(ch) = pending.peek() {
            if remaining == 0 {
                break;
            }
            remaining = remaining.checked_sub(ch.len_utf16())?;
            pending.next();
            spans.push(current.clone());
        }
    }
    if pending.next().is_some() {
        return None;
    }
    text.character_runs = character::compact(spans);
    let para = run(engine.get("ParagraphRun")?)?
        .first()?
        .get("ParagraphSheet")?
        .get("Properties")?;
    text.paragraph.align = *ALIGNMENTS.get(number(para, "Justification", 0.) as usize)?;
    text.paragraph.first = number(para, "FirstLineIndent", 0.)
        .round()
        .clamp(-256., 256.) as i16;
    text.paragraph.left = number(para, "StartIndent", 0.).round().clamp(-256., 256.) as i16;
    text.paragraph.right = number(para, "EndIndent", 0.).round().clamp(-256., 256.) as i16;
    text.paragraph.before = number(para, "SpaceBefore", 0.).round().clamp(0., 256.) as u16;
    text.paragraph.after = number(para, "SpaceAfter", 0.).round().clamp(0., 256.) as u16;
    text.line_height = (number(para, "AutoLeading", 1.2) * 100.)
        .round()
        .clamp(25., 300.) as u16;
    let shapes = engine.get("Rendered")?.get("Shapes")?;
    if number(shapes, "WritingDirection", 0.) != 0. {
        return None;
    }
    let shape = shapes.get("Children")?.list()?.first()?;
    let photoshop = shape.get("Cookie")?.get("Photoshop")?;
    let baseline = match number(shape, "ShapeType", 0.) as i32 {
        0 => {
            text.point_text = true;
            (0., -f64::from(text.size) * 0.8)
        }
        1 => {
            let bounds = photoshop.get("BoxBounds")?.list()?;
            let (left, top, right, bottom) = (
                bounds.first()?.number()?,
                bounds.get(1)?.number()?,
                bounds.get(2)?.number()?,
                bounds.get(3)?.number()?,
            );
            let width = (right - left).round();
            if !(1. ..=2048.).contains(&width) || bottom <= top {
                return None;
            }
            text.paragraph.width = width as u16;
            ((left + right) / 2., (top + bottom) / 2.)
        }
        _ => return None,
    };
    text.offset = [
        (transform[4] + transform[0] * baseline.0 + transform[2] * baseline.1).round() as i32
            - canvas[0] as i32 / 2,
        (transform[5] + transform[1] * baseline.0 + transform[3] * baseline.1).round() as i32
            - canvas[1] as i32 / 2,
    ];
    text.valid().then_some(text)
}

fn key(out: &mut Vec<u8>, name: &[u8]) {
    out.extend(
        if name.len() == 4 {
            0
        } else {
            name.len() as u32
        }
        .to_be_bytes(),
    );
    out.extend(name);
}

fn unicode(out: &mut Vec<u8>, value: &str) {
    out.extend((value.encode_utf16().count() as u32).to_be_bytes());
    out.extend(value.encode_utf16().flat_map(u16::to_be_bytes));
}

fn literal(out: &mut Vec<u8>, value: &str) {
    out.extend(b"(\xfe\xff");
    for byte in value.encode_utf16().flat_map(u16::to_be_bytes) {
        if b"()\\".contains(&byte) {
            out.push(b'\\');
        }
        out.push(byte);
    }
    out.push(b')');
}

pub(in crate::asset_viewer::graphic_editor) fn write(
    text: &Text,
    canvas: [u32; 2],
    index: i32,
    fonts: Option<&super::font_samples::Catalog>,
) -> Vec<u8> {
    let content = text.content.replace('\n', "\r");
    let mut font_set: Vec<&str> = Vec::new();
    let mut runs: Vec<((usize, u16, [u8; 3]), usize)> = Vec::new();
    for (i, ch) in content.chars().enumerate() {
        let s = character::get(&text.character_runs, i);
        let family = s.and_then(|s| s.family.as_deref()).unwrap_or(&text.family);
        let font = font_set
            .iter()
            .position(|&n| n == family)
            .unwrap_or_else(|| {
                font_set.push(family);
                font_set.len() - 1
            });
        let style = (
            font,
            s.and_then(|s| s.size).unwrap_or(text.size),
            s.and_then(|s| s.color).unwrap_or(text.color),
        );
        if let Some((prior, len)) = runs.last_mut()
            && *prior == style
        {
            *len += ch.len_utf16();
        } else {
            runs.push((style, ch.len_utf16()));
        }
    }
    if runs.is_empty() {
        font_set.push(&text.family);
        runs.push(((0, text.size, text.color), 0));
    }
    runs.last_mut().unwrap().1 += 1;
    let mut engine = b"<< /EngineDict << /Editor << /Text ".to_vec();
    literal(&mut engine, &(content.clone() + "\r"));
    let p = &text.paragraph;
    let align = ALIGNMENTS.iter().position(|&a| a == p.align).unwrap_or(0);
    let para = format!(
        "<< /ParagraphSheet << /DefaultStyleSheet 0 /Properties << /Justification {align} /FirstLineIndent {} /StartIndent {} /EndIndent {} /SpaceBefore {} /SpaceAfter {} /AutoLeading {:.3} >> >> /Adjustments << /Axis [ 1.0 0.0 1.0 ] /XY [ 0.0 0.0 ] >> >>",
        p.first,
        p.left,
        p.right,
        p.before,
        p.after,
        f64::from(text.line_height) / 100.
    );
    let mut body = format!(" >> /ParagraphRun << /DefaultRunData {para} /RunArray [");
    let lengths: Vec<usize> = content
        .split('\r')
        .map(|s| s.encode_utf16().count() + 1)
        .collect();
    for _ in &lengths {
        let _ = write!(body, " {para}");
    }
    body.push_str(" ] /RunLengthArray [");
    for &len in &lengths {
        let _ = write!(body, " {len}");
    }
    body.push_str(" ] /IsJoinable 1 >> /StyleRun << /DefaultRunData << /StyleSheet << /StyleSheetData << >> >> >> /RunArray [");
    let style = |font: usize, size: u16, color: [u8; 3]| {
        let [r, g, b] = color.map(|c| f64::from(c) / 255.);
        let tracking = (1000. * f64::from(text.tracking) / f64::from(size)).round() as i32;
        format!(
            "<< /Font {font} /FontSize {size}.0 /FauxBold {} /FauxItalic {} /Tracking {tracking} /AutoLeading true /FillFlag true /FillColor << /Type 1 /Values [ 1.0 {r:.6} {g:.6} {b:.6} ] >> >>",
            text.bold, text.italic
        )
    };
    for &((font, size, color), _) in &runs {
        let _ = write!(
            body,
            " << /StyleSheet << /StyleSheetData {} >> >>",
            style(font, size, color)
        );
    }
    body.push_str(" ] /RunLengthArray [");
    for &(_, len) in &runs {
        let _ = write!(body, " {len}");
    }
    let (shape, shape_geometry, bounds) = if !text.point_text && p.width > 0 {
        let half_width = f64::from(p.width) / 2.;
        let half_height = f64::from(canvas[1]) / 2.;
        (
            1,
            format!(
                "/BoxBounds [ {} {} {half_width} {half_height} ]",
                -half_width, -half_height
            ),
            [-half_width, -half_height, half_width, half_height],
        )
    } else {
        (0, "/PointBase [ 0.0 0.0 ]".into(), [0.; 4])
    };
    let _ = write!(
        body,
        " ] /IsJoinable 2 >> /AntiAlias 3 /UseFractionalGlyphWidths true /Rendered << /Version 1 /Shapes << /WritingDirection 0 /Children [ << /ShapeType {shape} /Procession 0 /Lines << /WritingDirection 0 /Children [ ] >> /Cookie << /Photoshop << /ShapeType {shape} {shape_geometry} /Base << /ShapeType {shape} /TransformPoint0 [ 1.0 0.0 ] /TransformPoint1 [ 0.0 1.0 ] /TransformPoint2 [ 0.0 0.0 ] >> >> >> >> ] >> >> >>"
    );
    engine.extend(body.as_bytes());
    let mut resources = format!(" << /TheNormalStyleSheet 0 /TheNormalParagraphSheet 0 /ParagraphSheetSet [ {} ] /StyleSheetSet [ << /StyleSheetData {} >> ] /FontSet [", para.strip_prefix("<< /ParagraphSheet ").and_then(|s| s.split_once(" /Adjustments")).map_or("<< /Properties << >> >>", |(props, _)| props), style(0, text.size, text.color)).into_bytes();
    for family in font_set {
        let postscript = fonts
            .and_then(|f| {
                f.db.query(&resvg::usvg::fontdb::Query {
                    families: &[resvg::usvg::fontdb::Family::Name(family)],
                    ..Default::default()
                })
                .and_then(|id| f.db.face(id))
            })
            .map_or(family, |face| &face.post_script_name);
        resources.extend(b" << /Name ");
        literal(&mut resources, postscript);
        resources.extend(b" /Script 0 /FontType 0 /Synthetic 0 >>");
    }
    resources.extend(b" ] /SuperscriptSize .583 /SuperscriptPosition .333 /SubscriptSize .583 /SubscriptPosition .333 /SmallCapSize .7 >>");
    engine.extend(b" /ResourceDict");
    engine.extend(&resources);
    engine.extend(b" /DocumentResources");
    engine.extend(&resources);
    engine.extend(b" >>");

    let [a, b, c, d] = text.linear.map(f64::from);
    let (sin, cos) = f64::from(text.angle).to_radians().sin_cos();
    let (a, b, c, d) = (
        a * cos + c * sin,
        b * cos + d * sin,
        c * cos - a * sin,
        d * cos - b * sin,
    );
    let baseline = if shape == 0 {
        f64::from(text.size) * 0.8
    } else {
        0.
    };
    let x = f64::from(text.offset[0]) + f64::from(canvas[0] / 2) + c * baseline;
    let y = f64::from(text.offset[1]) + f64::from(canvas[1] / 2) + d * baseline;
    let mut out = 1u16.to_be_bytes().to_vec();
    out.extend([a, b, c, d, x, y].into_iter().flat_map(f64::to_be_bytes));
    out.extend(b"\x00\x32\x00\x00\x00\x10");
    unicode(&mut out, "");
    key(&mut out, b"TxLr");
    out.extend(8u32.to_be_bytes());
    key(&mut out, b"Txt ");
    out.extend(b"TEXT");
    unicode(&mut out, &(content + "\0"));
    for (name, kind, value) in [
        (
            b"textGridding".as_slice(),
            b"textGridding".as_slice(),
            b"None".as_slice(),
        ),
        (b"Ornt", b"Ornt", b"Hrzn"),
        (b"AntA", b"Annt", b"AnSm"),
    ] {
        key(&mut out, name);
        out.extend(b"enum");
        key(&mut out, kind);
        key(&mut out, value);
    }
    key(&mut out, b"TextIndex");
    out.extend(b"long");
    out.extend(index.to_be_bytes());
    for name in [b"bounds".as_slice(), b"boundingBox"] {
        key(&mut out, name);
        out.extend(b"Objc");
        unicode(&mut out, "");
        key(&mut out, name);
        out.extend(4u32.to_be_bytes());
        for (coord, value) in [b"Left", b"Top ", b"Rght", b"Btom"].into_iter().zip(bounds) {
            key(&mut out, coord);
            out.extend(b"UntF#Pnt");
            out.extend(value.to_be_bytes());
        }
    }
    key(&mut out, b"EngineData");
    out.extend(b"tdta");
    out.extend((engine.len() as u32).to_be_bytes());
    out.extend(engine);
    out.extend(b"\x00\x01\x00\x00\x00\x10");
    unicode(&mut out, "");
    key(&mut out, b"warp");
    out.extend(5u32.to_be_bytes());
    key(&mut out, b"warpStyle");
    out.extend(b"enum");
    key(&mut out, b"warpStyle");
    key(&mut out, b"warpNone");
    for name in [
        b"warpValue".as_slice(),
        b"warpPerspective",
        b"warpPerspectiveOther",
    ] {
        key(&mut out, name);
        out.extend(b"doub");
        out.extend(0f64.to_be_bytes());
    }
    key(&mut out, b"warpRotate");
    out.extend(b"enum");
    key(&mut out, b"Ornt");
    key(&mut out, b"Hrzn");
    out.extend([0; 16]);
    out.resize(out.len().next_multiple_of(4), 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_native_text_interprets_utf16_style_lengths() {
        let mut text = Text::new(150, [10, 20, 30, 255]);
        text.content = "A😀(\\)\nB".into();
        text.angle = 0;
        text.repeat = false;
        text.point_text = true;
        text.offset = [-54, -25];
        text.paragraph.align = Align::Right;
        text.paragraph.left = 5;
        text.paragraph.right = 7;
        text.paragraph.first = -2;
        text.paragraph.before = 3;
        text.paragraph.after = 4;
        text.character_runs = character::compact(vec![
            character::Style::default(),
            character::Style {
                family: Some("Arial".into()),
                size: Some(49),
                color: Some([0, 255, 162]),
            },
        ]);
        let block = write(&text, [256, 128], 0, None);
        let actual = read(&block, [256, 128]).expect("native type data");
        assert!(actual == text);
    }
}
