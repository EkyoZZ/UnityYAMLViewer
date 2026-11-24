//! 8-bit RGB PSD pixel layers and masks. Unsupported native layer structures
//! go through the existing merged preview and cannot overwrite their source.
use super::super::layer_properties::{Blend, Properties, effects::Light};
use super::*;
use std::io::{self, Read, Write};
const BLENDS: [(&[u8; 4], Blend); 20] = [
    (b"norm", Blend::Normal),
    (b"mul ", Blend::Multiply),
    (b"scrn", Blend::Screen),
    (b"over", Blend::Overlay),
    (b"dark", Blend::Darken),
    (b"lite", Blend::Lighten),
    (b"diff", Blend::Difference),
    (b"smud", Blend::Exclusion),
    (b"lddg", Blend::Add),
    (b"fsub", Blend::Subtract),
    (b"sLit", Blend::SoftLight),
    (b"hLit", Blend::HardLight),
    (b"div ", Blend::Dodge),
    (b"idiv", Blend::Burn),
    (b"fdiv", Blend::Divide),
    (b"lbrn", Blend::LinearBurn),
    (b"hue ", Blend::Hue),
    (b"sat ", Blend::Saturation),
    (b"colr", Blend::Color),
    (b"lum ", Blend::Luminosity),
];
type Result<T> = std::result::Result<T, String>;
struct Bytes<'a>(&'a [u8]);
impl<'a> Bytes<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let (a, b) = self.0.split_at_checked(n).ok_or("Truncated PSD section")?;
        self.0 = b;
        Ok(a)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn long(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn section(&mut self) -> Result<Bytes<'a>> {
        let n = self.long()?;
        Ok(Bytes(self.take(n as usize)?))
    }
    fn rect(&mut self) -> Result<[i32; 4]> {
        Ok([
            self.long()? as i32,
            self.long()? as i32,
            self.long()? as i32,
            self.long()? as i32,
        ])
    }
}
fn dimensions([top, left, bottom, right]: [i32; 4]) -> Result<(usize, usize)> {
    let w = i64::from(right) - i64::from(left);
    let h = i64::from(bottom) - i64::from(top);
    if !(0..=8192).contains(&w) || !(0..=8192).contains(&h) || w * h > MAX_IMAGE_PIXELS as i64 {
        return Err("Unsupported PSD layer extent".into());
    }
    Ok((w as usize, h as usize))
}
#[derive(serde::Serialize, serde::Deserialize)]
struct TextData {
    text: text_layer::Text,
    raster_hash: [u8; 32],
}

fn blend_key(mode: Blend) -> Result<&'static [u8; 4]> {
    BLENDS
        .iter()
        .find(|(_, b)| *b == mode)
        .map(|(k, _)| *k)
        .ok_or_else(|| "Unsupported PSD blend mode".into())
}
fn blend_value(key: &[u8]) -> Result<Blend> {
    if key == b"pass" {
        return Ok(Blend::Normal);
    }
    BLENDS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, b)| *b)
        .ok_or_else(|| "Unsupported PSD blend mode".into())
}
struct Record {
    divider: u32,
    locked: bool,
    text_hash: Option<[u8; 32]>,
    rect: [i32; 4],
    channels: Vec<(i16, usize)>,
    name: String,
    visible: bool,
    settings: Properties,
    mask: Option<([i32; 4], u8)>,
}
fn record(data: &mut Bytes) -> Result<Record> {
    let rect = data.rect()?;
    dimensions(rect)?;
    let count = data.word()?;
    if count > 5 {
        return Err("Additional PSD channels".into());
    }
    let mut channels = Vec::new();
    for _ in 0..count {
        let id = data.word()? as i16;
        let len = data.long()? as usize;
        if !(-2..=2).contains(&id) || channels.iter().any(|(v, _)| *v == id) {
            return Err("Unsupported PSD channel".into());
        }
        channels.push((id, len));
    }
    if data.take(4)? != b"8BIM" {
        return Err("Invalid PSD layer signature".into());
    }
    let mode = data.take(4)?;
    let blend = blend_value(mode)?;
    let mut divider = 0;
    let mut locked = false;
    let mut text_hash = None;
    let pct = |v: u8| ((u16::from(v) * 100 + 127) / 255) as u8;
    let opacity = pct(data.byte()?);
    let clipped = data.byte()? != 0;
    let flags = data.byte()?;
    data.take(1)?;
    let mut settings = Properties {
        blend,
        opacity,
        clipped,
        lock_alpha: flags & 1 != 0,
        ..Default::default()
    };
    let mut extra = data.section()?;
    let mut mask_data = extra.section()?;
    let mask = if mask_data.0.is_empty() {
        None
    } else {
        if mask_data.0.len() != 20 {
            return Err("Unsupported PSD mask parameters".into());
        }
        let rect = mask_data.rect()?;
        dimensions(rect)?;
        let default = mask_data.byte()?;
        let bits = mask_data.byte()?;
        if bits & !3 != 0 {
            return Err("Unsupported PSD mask flags".into());
        }
        settings.mask_enabled = bits & 2 == 0;
        settings.mask_linked = bits & 1 == 0;
        Some((rect, default))
    };
    if mask.is_some() != channels.iter().any(|(id, _)| *id == -2) {
        return Err("Invalid PSD mask channel".into());
    }
    let ranges = extra.section()?;
    if !ranges
        .0
        .chunks(8)
        .all(|b| b == [0, 0, 255, 255, 0, 0, 255, 255])
    {
        return Err("PSD blending ranges".into());
    }
    let n = extra.byte()? as usize;
    let mut name = String::from_utf8_lossy(extra.take(n)?).into_owned();
    extra.take((4 - (n + 1) % 4) % 4)?;
    let mut style = None;
    let mut multi_style = None;
    let mut legacy_style = false;
    while extra.0.len() >= 12 {
        if extra.take(4)? != b"8BIM" {
            return Err("Unsupported PSD layer block".into());
        }
        let key = extra.take(4)?;
        let mut block = extra.section()?;
        let len = block.0.len();
        match key {
            b"luni" => {
                let n = block.long()? as usize;
                let wide = block.take(n.checked_mul(2).ok_or("Invalid layer name")?)?;
                name = String::from_utf16_lossy(
                    &wide
                        .chunks_exact(2)
                        .map(|v| u16::from_be_bytes([v[0], v[1]]))
                        .collect::<Vec<_>>(),
                )
                .trim_end_matches('\0')
                .to_owned();
            }
            b"iOpa" => settings.fill = pct(block.byte()?),
            b"lspf" => {
                let flags = block.long()?;
                if flags & !0x80000007 != 0 {
                    return Err("PSD protected layer".into());
                }
                locked = flags & 0x80000000 != 0;
                settings.lock_alpha = flags & 1 != 0;
                settings.lock_pixels = flags & 2 != 0;
                settings.lock_position = flags & 4 != 0;
            }
            b"uyTx" => {
                if len > 32768 {
                    return Err("Invalid text metadata".into());
                }
                let t: TextData = serde_json::from_slice(block.0).map_err(|e| e.to_string())?;
                if !t.text.valid() || t.text.repeat {
                    return Err("Invalid text settings".into());
                }
                text_hash = Some(t.raster_hash);
                settings.text = Some(t.text);
            }
            b"lsct" | b"lsdk" => {
                divider = block.long()?;
                if divider > 3 {
                    return Err("Unknown PSD group type".into());
                }
                let group_mode = if !block.0.is_empty() {
                    if block.take(4)? != b"8BIM" {
                        return Err("Unknown PSD group signature".into());
                    }
                    block.take(4)?
                } else {
                    mode
                };
                if !block.0.is_empty() && block.long()? != 0 {
                    return Err("Unsupported PSD group subtype".into());
                }
                settings.blend = blend_value(group_mode)?;
                settings.pass_through = group_mode == b"pass";
                settings.collapsed = divider == 2;
            }
            b"lyid" => settings.psd_layer_id = Some(block.long()?),
            b"lfx2" | b"lfxs" => style = Some(block.0),
            b"lmfx" => multi_style = Some(block.0),
            b"lrFX" => legacy_style = true,
            b"lclr" | b"fxrp" | b"clbl" | b"infx" | b"knko" | b"lnsr" | b"lyvr" | b"lmgm"
            | b"vmgm" => {
                if len > 64 {
                    return Err("Unexpected PSD layer metadata".into());
                }
                tag(&mut settings.psd_metadata, key.try_into().unwrap(), block.0);
            }
            _ => return Err(format!("PSD layer block {}", String::from_utf8_lossy(key))),
        }
        if len % 2 != 0 && extra.0.first() == Some(&0) {
            extra.take(1)?;
        }
    }
    if let Some(bytes) = multi_style.or(style) {
        settings.effects =
            super::super::layer_properties::effects::Effects::decode_psd_styles(bytes)?;
    } else if legacy_style {
        return Err("Unsupported legacy PSD layer style".into());
    }
    if !extra.0.iter().all(|v| *v == 0) {
        return Err("Invalid trailing layer data".into());
    }
    Ok(Record {
        divider,
        locked,
        text_hash,
        rect,
        channels,
        name,
        visible: flags & 2 == 0,
        settings,
        mask,
    })
}
fn plane(bytes: &[u8], w: usize, h: usize) -> Result<Vec<u8>> {
    let mut data = Bytes(bytes);
    let mode = data.word()?;
    if mode == 0 {
        return Ok(data.take(w * h)?.to_vec());
    }
    if matches!(mode, 2 | 3) {
        let mut decoded = Vec::with_capacity(w * h);
        flate2::read::ZlibDecoder::new(data.0)
            .take((w * h + 1) as u64)
            .read_to_end(&mut decoded)
            .map_err(|e| e.to_string())?;
        if decoded.len() != w * h {
            return Err("Invalid ZIP layer channel length".into());
        }
        if mode == 3 && w > 0 {
            for row in decoded.chunks_exact_mut(w) {
                for x in 1..w {
                    row[x] = row[x].wrapping_add(row[x - 1]);
                }
            }
        }
        return Ok(decoded);
    }
    if mode != 1 {
        return Err("Unsupported PSD layer compression".into());
    }
    let sizes = (0..h).map(|_| data.word()).collect::<Result<Vec<_>>>()?;
    let mut out = Vec::with_capacity(w * h);
    for n in sizes {
        out.extend(crate::asset_viewer::psd::unpack_row(
            data.take(n as usize)?,
            w,
        )?);
    }
    Ok(out)
}
// Version Info resource 1057 identifies the white compatibility placeholder.
// On output drop cached thumbnails and write back the real-composite flag.
fn image_resources(
    bytes: &[u8],
    mut output: Option<&mut Vec<u8>>,
    mut light: Option<&mut Light>,
) -> Result<bool> {
    let mut section = Bytes(bytes);
    let mut has_merged = true;
    while !section.0.is_empty() {
        let start = section.0;
        if section.take(4)? != b"8BIM" {
            return Err("Invalid PSD image resource".into());
        }
        let id = section.word()?;
        let name_len = usize::from(section.byte()?);
        section.take(name_len + usize::from(name_len % 2 == 0))?;
        let data = section.section()?.0;
        if data.len() % 2 != 0 {
            section.take(1)?;
        }
        if id == 1057 {
            if data.len() < 5 {
                return Err("Invalid PSD Version Info resource".into());
            }
            has_merged = data[4] != 0;
        }
        if let Some(light) = light.as_deref_mut()
            && matches!(id, 1037 | 1049)
        {
            let mut payload = Bytes(data);
            let value = payload.long()? as i32;
            if id == 1037 {
                light.angle = ((i64::from(value) + 180).rem_euclid(360) - 180) as i16;
            } else if let Ok(altitude) = u8::try_from(value)
                && altitude <= 90
            {
                light.altitude = altitude;
            }
        }
        if let Some(out) = output.as_deref_mut()
            && !matches!(id, 1033 | 1036 | 1069)
        {
            let record = &start[..start.len() - section.0.len()];
            let offset = out.len();
            out.extend_from_slice(record);
            if id == 1057 {
                out[offset + record.len() - data.len() - (data.len() % 2) + 4] = 1;
            }
        }
    }
    Ok(has_merged)
}
pub(super) fn global_light(resources: &[u8]) -> Light {
    let mut light = Light::default();
    let _ = image_resources(resources, None, Some(&mut light));
    light
}

fn resources_with_light(resources: &[u8], light: Light) -> Result<Vec<u8>> {
    let mut section = Bytes(resources);
    let mut output = Vec::new();
    while !section.0.is_empty() {
        let start = section.0;
        section.take(4)?;
        let id = section.word()?;
        let name_len = usize::from(section.byte()?);
        section.take(name_len + usize::from(name_len % 2 == 0))?;
        let data_len = section.section()?.0.len();
        section.take(data_len % 2)?;
        if !matches!(id, 1037 | 1049) {
            output.extend(&start[..start.len() - section.0.len()]);
        }
    }
    for (id, value) in [
        (1037u16, i32::from(light.angle)),
        (1049, i32::from(light.altitude)),
    ] {
        output.extend(b"8BIM");
        output.extend(id.to_be_bytes());
        output.extend([0, 0]); // empty Pascal name and alignment byte
        output.extend(4u32.to_be_bytes());
        output.extend(value.to_be_bytes());
    }
    Ok(output)
}
pub(super) fn has_merged_preview(bytes: &[u8]) -> Result<bool> {
    let mut data = Bytes(bytes);
    data.take(26)?;
    data.section()?;
    image_resources(data.section()?.0, None, None)
}
pub(super) fn decode(bytes: &[u8], (w, h): (u32, u32)) -> Result<(Vec<Layer>, Vec<u8>)> {
    let mut data = Bytes(bytes);
    if data.take(4)? != b"8BPS" || data.word()? != 1 {
        return Err("PSD v1 required".into());
    }
    data.take(6)?;
    let channels = data.word()?;
    if !(3..=4).contains(&channels)
        || data.long()? != h
        || data.long()? != w
        || data.word()? != 8
        || data.word()? != 3
    {
        return Err("Layer editing requires 8-bit RGB PSD".into());
    }
    data.section()?;
    let mut resources = vec![];
    image_resources(data.section()?.0, Some(&mut resources), None)?;
    let mut layers = data.section()?;
    if layers.0.is_empty() && channels == 3 {
        return Ok((vec![], resources));
    }
    let mut info = layers.section()?;
    let signed = info.word()? as i16;
    if (channels == 4) != (signed < 0) {
        return Err("Additional PSD channels".into());
    }
    let count = usize::from(signed.unsigned_abs());
    if count > 64 {
        return Err("PSD layer memory limit".into());
    }
    let mut records = (0..count)
        .map(|_| record(&mut info))
        .collect::<Result<Vec<_>>>()?;
    let mut parents = vec![0];
    for (i, r) in records.iter_mut().enumerate().rev() {
        r.settings.parent = *parents.last().ok_or("Invalid PSD group nesting")?;
        match r.divider {
            1 | 2 => {
                r.settings.group_id = i as u64 + 1;
                parents.push(r.settings.group_id);
            }
            3 => {
                if parents.len() == 1 || r.mask.is_some() {
                    return Err("Invalid PSD group divider".into());
                }
                parents.pop();
            }
            _ => {}
        }
        if std::iter::once(r.rect)
            .chain(r.mask.map(|m| m.0))
            .any(|[t, l, b, rt]| t < 0 || l < 0 || b > h as i32 || rt > w as i32)
        {
            return Err("PSD contains off-canvas pixels".into());
        }
    }
    if parents.len() != 1 {
        return Err("Unclosed PSD group".into());
    }
    if records.iter().filter(|r| r.divider != 3).count() > 32
        || records
            .iter()
            .filter(|r| r.divider != 3)
            .map(|r| 4 + u64::from(r.mask.is_some()))
            .sum::<u64>()
            * u64::from(w)
            * u64::from(h)
            > 128 * 1024 * 1024
    {
        return Err("PSD layer memory limit".into());
    }
    let mut result = Vec::with_capacity(count);
    for mut r in records {
        if r.divider == 3 {
            for (_, len) in r.channels {
                info.take(len)?;
            }
            continue;
        }
        let mut im = RgbaImage::new(w, h);
        for y in r.rect[0].max(0)..r.rect[2].min(h as i32) {
            for x in r.rect[1].max(0)..r.rect[3].min(w as i32) {
                im.get_pixel_mut(x as u32, y as u32)[3] = 255;
            }
        }
        if let Some((_, default)) = r.mask {
            r.settings.mask = Some(vec![vec![default; (w * h) as usize]]);
        }
        for (id, n) in r.channels {
            let rect = if id == -2 {
                r.mask.ok_or("Missing PSD mask bounds")?.0
            } else {
                r.rect
            };
            let (pw, ph) = dimensions(rect)?;
            let pixels = plane(info.take(n)?, pw, ph)?;
            for y in rect[0].max(0)..rect[2].min(h as i32) {
                for x in rect[1].max(0)..rect[3].min(w as i32) {
                    let v = pixels[(i64::from(y) - i64::from(rect[0])) as usize * pw
                        + (i64::from(x) - i64::from(rect[1])) as usize];
                    if id == -2 {
                        r.settings.mask.as_mut().unwrap()[0]
                            [y as usize * w as usize + x as usize] = v;
                    } else {
                        im.get_pixel_mut(x as u32, y as u32)
                            [if id == -1 { 3 } else { id as usize }] = v;
                    }
                }
            }
        }
        if let Some(digest) = r.text_hash
            && <[u8; 32]>::from(Sha256::digest(im.as_raw())) != digest
        {
            r.settings.text = None;
        }
        result.push(Layer {
            settings: r.settings,
            locked: r.locked,
            material: false,
            name: r.name,
            visible: r.visible,
            tiles: vec![im],
        });
    }
    if info.0.iter().any(|b| *b != 0) {
        return Err("Unrecognized PSD layer info".into());
    }
    if layers.0.len() >= 4 && !layers.section()?.0.is_empty() {
        return Err("Global PSD mask".into());
    }
    if layers.0.iter().any(|&b| b != 0) {
        return Err("Additional global PSD layer data".into());
    }
    Ok((result, resources))
}
pub(super) fn needs_merged(e: &Editor) -> bool {
    e.layers
        .iter()
        .any(|l| l.settings.adjustment.is_some() || l.settings.effects.has_unmapped_psd_styles())
}
// PSD stores a bounding divider before the children and the group header after them.
fn layer_records<'a>(layers: &'a [Layer], parent: u64, records: &mut Vec<(&'a Layer, u32)>) {
    for l in layers.iter().filter(|l| l.settings.parent == parent) {
        if l.settings.group_id == 0 {
            records.push((l, 0));
        } else {
            records.push((l, 3));
            layer_records(layers, l.settings.group_id, records);
            records.push((l, if l.settings.collapsed { 2 } else { 1 }));
        }
    }
}
fn tag(extra: &mut Vec<u8>, key: &[u8; 4], bytes: &[u8]) {
    extra.extend(b"8BIM");
    extra.extend(key);
    extra.extend((bytes.len() as u32).to_be_bytes());
    extra.extend(bytes);
    if bytes.len() % 2 != 0 {
        extra.push(0);
    }
}
fn rect(out: &mut Vec<u8>, w: u32, h: u32) {
    for n in [0, 0, h, w] {
        out.extend(n.to_be_bytes());
    }
}
pub(super) fn write(out: &mut impl Write, e: &Editor, merged: &RgbaImage) -> Result<()> {
    let (w, h) = merged.dimensions();
    let flat = Layer {
        settings: Default::default(),
        locked: false,
        material: false,
        name: tr("图像").into(),
        visible: true,
        tiles: vec![],
    };
    let flat_only = needs_merged(e);
    let layers = if flat_only {
        std::slice::from_ref(&flat)
    } else {
        &e.layers
    };
    let mut records = vec![];
    layer_records(layers, 0, &mut records);
    let mut table = Vec::new();
    table.extend((-(records.len() as i16)).to_be_bytes());
    let mut plane_bytes = 0usize;
    let reserved: std::collections::HashSet<u32> = layers
        .iter()
        .filter_map(|l| l.settings.psd_layer_id)
        .collect();
    let mut used = std::collections::HashSet::new();
    let mut next_id = 1;
    for &(l, divider) in &records {
        let defaults = Properties::default();
        let p = if divider == 3 { &defaults } else { &l.settings };
        let pixels = if divider == 0 { w * h } else { 0 };
        if divider == 0 {
            rect(&mut table, w, h)
        } else {
            rect(&mut table, 0, 0)
        }
        let mask = p.mask.is_some();
        let count = if mask { 5u16 } else { 4 };
        table.extend(count.to_be_bytes());
        for id in [0i16, 1, 2, -1, -2].into_iter().take(count as usize) {
            table.extend(id.to_be_bytes());
            let size = if id == -2 { w * h } else { pixels };
            table.extend((2 + size).to_be_bytes());
            plane_bytes += 2 + size as usize;
        }
        table.extend(b"8BIM");
        table.extend(blend_key(p.blend)?);
        let opacity = |v: u8| ((u16::from(v) * 255 + 50) / 100) as u8;
        table.extend([
            opacity(p.opacity),
            p.clipped as u8,
            u8::from(p.lock_alpha)
                | (u8::from(!l.visible) << 1)
                | if divider != 0 { 0x18 } else { 0 },
            0,
        ]);
        let mut extra = Vec::new();
        if mask {
            extra.extend(20u32.to_be_bytes());
            rect(&mut extra, w, h);
            extra.extend([
                255,
                u8::from(!p.mask_linked) | (u8::from(!p.mask_enabled) << 1),
                0,
                0,
            ]);
        } else {
            extra.extend([0; 4]);
        }
        extra.extend([0; 4]);
        let record_name = if divider == 3 {
            "</Layer group>"
        } else {
            &l.name
        };
        let name = record_name
            .chars()
            .take(255)
            .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
            .collect::<Vec<_>>();
        extra.push(name.len() as u8);
        extra.extend(name);
        extra.resize(extra.len().next_multiple_of(4), 0);
        let wide: Vec<_> = record_name.encode_utf16().collect();
        let mut unicode = Vec::new();
        unicode.extend((wide.len() as u32).to_be_bytes());
        for c in wide {
            unicode.extend(c.to_be_bytes());
        }
        tag(&mut extra, b"luni", &unicode);
        let id = if let Some(id) = p.psd_layer_id.filter(|&id| id != 0 && used.insert(id)) {
            id
        } else {
            while reserved.contains(&next_id) || used.contains(&next_id) {
                next_id += 1;
            }
            used.insert(next_id);
            next_id
        };
        tag(&mut extra, b"lyid", &id.to_be_bytes());
        extra.extend_from_slice(&p.psd_metadata);
        if let Some((key, mut bytes)) = p.effects.encode_psd_styles() {
            bytes.resize(bytes.len().next_multiple_of(2), 0);
            tag(
                &mut extra,
                if divider == 0 { &key } else { b"lfxs" },
                &bytes,
            );
        }
        if p.fill != 100 {
            tag(&mut extra, b"iOpa", &[opacity(p.fill), 0, 0, 0]);
        }
        let locks = (u32::from(l.locked) << 31)
            | u32::from(p.lock_alpha)
            | u32::from(p.lock_pixels) << 1
            | u32::from(p.lock_position) << 2;
        tag(&mut extra, b"lspf", &locks.to_be_bytes());
        if divider != 0 {
            let mut block = divider.to_be_bytes().to_vec();
            if divider != 3 {
                block.extend(b"8BIM");
                block.extend(if p.pass_through {
                    b"pass"
                } else {
                    blend_key(p.blend)?
                });
            }
            tag(&mut extra, b"lsct", &block);
        }
        if let Some(text) = &p.text {
            let data = TextData {
                text: text.clone(),
                raster_hash: Sha256::digest(l.tiles[0].as_raw()).into(),
            };
            tag(
                &mut extra,
                b"uyTx",
                &serde_json::to_vec(&data).map_err(|err| err.to_string())?,
            );
        }
        table.extend((extra.len() as u32).to_be_bytes());
        table.extend(extra);
    }
    let len = table.len() + plane_bytes;
    let resources = resources_with_light(&e.psd_resources, e.fx_light)?;
    let mut write = || -> io::Result<()> {
        out.write_all(b"8BPS\x00\x01\x00\x00\x00\x00\x00\x00\x00\x04")?;
        out.write_all(&h.to_be_bytes())?;
        out.write_all(&w.to_be_bytes())?;
        out.write_all(&[0, 8, 0, 3, 0, 0, 0, 0])?;
        out.write_all(&(resources.len() as u32).to_be_bytes())?;
        out.write_all(&resources)?;
        out.write_all(&((len.next_multiple_of(2) + 8) as u32).to_be_bytes())?;
        out.write_all(&(len.next_multiple_of(2) as u32).to_be_bytes())?;
        out.write_all(&table)?;
        let mut bytes = Vec::with_capacity(w as usize);
        let mut channel = |out: &mut _, im: &RgbaImage, c: usize, matte: bool| {
            for row in im.rows() {
                bytes.clear();
                bytes.extend(row.map(|p| {
                    if matte && c < 3 {
                        let a = u32::from(p[3]);
                        ((u32::from(p[c]) * a + 255 * (255 - a) + 127) / 255) as u8
                    } else {
                        p[c]
                    }
                }));
                Write::write_all(out, &bytes)?;
            }
            Ok::<_, io::Error>(())
        };
        for &(l, divider) in &records {
            for c in 0..4 {
                out.write_all(&[0; 2])?;
                if divider == 0 {
                    channel(out, if flat_only { merged } else { &l.tiles[0] }, c, false)?;
                }
            }
            if divider != 3
                && let Some(m) = &l.settings.mask
            {
                out.write_all(&[0; 2])?;
                out.write_all(&m[0])?;
            }
        }
        if len % 2 != 0 {
            out.write_all(&[0])?;
        }
        out.write_all(&[0; 4])?;
        out.write_all(&[0; 2])?;
        for c in 0..4 {
            channel(out, merged, c, true)?;
        }
        Ok(())
    };
    write().map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn native_styles_keep_order_raw_pixels_and_rendered_composite() {
        let mut editor = Editor::default();
        let pixels = RgbaImage::from_fn(16, 16, |x, y| {
            if (5..10).contains(&x) && (5..10).contains(&y) {
                Rgba([40, 70, 90, 255])
            } else {
                Rgba([0; 4])
            }
        });
        editor.init_image(&pixels);
        editor.fx_light = Light {
            angle: 65,
            altitude: 42,
        };
        editor.layers[0].settings.effects = serde_json::from_value(serde_json::json!({
            "stroke": true, "stroke_size": 1, "stroke_color": [10, 20, 30],
            "extra_strokes": [
                {"stroke": false, "stroke_size": 4, "stroke_color": [20, 80, 30]},
                {"stroke": true, "stroke_size": 2, "stroke_position": "Inside",
                 "stroke_gradient": true, "stroke_ramp": {"colors": [[5, 50, 150], [220, 130, 15]]}}
            ],
            "shadow": true, "shadow_opacity": 40,
            "shadow_offset": [0, 0], "shadow_shape": {"direction": [90, 2], "global": true},
            "extra_shadows": [
                {"shadow": true, "shadow_offset": [0, 0], "shadow_color": [20, 50, 90],
                 "shadow_shape": {"direction": [-55, 4], "spread": 30,
                                  "quality": {"antialias": true, "noise": 9, "contour": "Cone"}}}
            ],
            "inner_shadow": {"enabled": true},
            "inner_glow": {"enabled": true, "gradient": true},
            "bevel": {"enabled": true},
            "gradient": true, "gradient_opacity": 27
        }))
        .unwrap();
        let composite = editor.composite(0);
        assert_ne!(composite, pixels);
        assert!(!needs_merged(&editor));
        let mut bytes = Vec::new();
        write(&mut bytes, &editor, &composite).unwrap();
        assert!(bytes.windows(4).any(|key| key == b"lmfx"));
        let (layers, resources) = decode(&bytes, (16, 16)).unwrap();
        assert!(global_light(&resources) == editor.fx_light);
        assert_eq!(layers[0].tiles[0], pixels);
        assert!(layers[0].settings.effects == editor.layers[0].settings.effects);
        let preview = crate::asset_viewer::psd::decode(Cursor::new(&bytes)).unwrap();
        for (actual, expected) in preview.pixels().zip(composite.pixels()) {
            assert_eq!(actual[3], expected[3]);
            if expected[3] == 255 {
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn layered_psd_keeps_group_order_unicode_mask_and_source_pixels() {
        let mut editor = Editor::default();
        let bottom = RgbaImage::from_pixel(3, 2, Rgba([31, 22, 11, 255]));
        editor.init_image(&bottom);
        let mut top = editor.layers[0].clone();
        top.name = "描画 ✦".into();
        top.tiles[0] = RgbaImage::from_pixel(3, 2, Rgba([77, 88, 99, 130]));
        top.settings.blend = Blend::Screen;
        top.settings.parent = 21;
        top.settings.clipped = true;
        top.settings.mask = Some(vec![vec![255, 127, 0, 0, 127, 255]]);
        top.settings.mask_linked = false;
        top.settings.fill = 31;
        top.settings.opacity = 72;
        top.locked = true;
        let mut group = editor.layers[0].clone();
        group.name = "组".into();
        group.settings.group_id = 21;
        group.settings.collapsed = true;
        group.tiles[0] = RgbaImage::new(3, 2);
        editor.layers.extend([top.clone(), group]);
        let composite = editor.composite(0);
        let mut bytes = vec![];
        write(&mut bytes, &editor, &composite).unwrap();
        let (layers, _) = decode(&bytes, (3, 2)).unwrap();
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].tiles[0], bottom);
        assert_eq!(layers[1].name, top.name);
        assert_eq!(layers[1].settings.parent, layers[2].settings.group_id);
        assert_ne!(layers[2].settings.group_id, 0);
        assert!(layers[2].settings.collapsed && layers[2].settings.pass_through);
        assert!(layers[1].locked && layers[1].settings.clipped && !layers[1].settings.mask_linked);
        assert_eq!(layers[1].tiles, top.tiles);
        assert!(layers[1].settings.blend == Blend::Screen);
        assert_eq!(
            (layers[1].settings.fill, layers[1].settings.opacity),
            (31, 72)
        );
        assert_eq!(layers[1].settings.mask, top.settings.mask);
        assert_eq!(
            crate::asset_viewer::psd::decode(Cursor::new(&bytes)).unwrap(),
            composite
        );
        let offset = bytes.windows(4).position(|b| b == b"luni").unwrap();
        bytes[offset..offset + 4].copy_from_slice(b"SoLd");
        assert!(decode(&bytes, (3, 2)).is_err());
    }

    #[test]
    fn native_text_and_effects_in_repository_psds_use_existing_composite() {
        for name in ["assets/app_icon.psd", "assets/default_icons/file_icon.psd"] {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
            let doc = super::super::open_image(&path, 8192).unwrap();
            assert!(doc.layers.is_empty() && doc.restriction.is_some());
            assert_eq!(doc.pixels.dimensions(), (1024, 1024));
        }
    }

    #[test]
    fn packed_rows_and_zip_prediction_reset_at_each_row() {
        assert_eq!(
            plane(&[0, 1, 0, 2, 0, 4, 254, 42, 2, 7, 8, 9], 3, 2).unwrap(),
            [42, 42, 42, 7, 8, 9]
        );
        assert!(plane(&[0, 1, 0, 2, 253, 42], 3, 1).is_err());
        let mut zip = flate2::write::ZlibEncoder::new(vec![0, 3], flate2::Compression::fast());
        zip.write_all(&[11, 10, 1, 255, 2, 1]).unwrap();
        let data = zip.finish().unwrap();
        assert_eq!(plane(&data, 3, 2).unwrap(), [11, 21, 22, 255, 1, 2]);
        assert!(plane(&data, 2, 2).is_err());
    }

    #[test]
    fn merged_preview_unmattes_translucent_edges_and_keeps_layer_bytes() {
        let mut e = Editor::default();
        let pixels = RgbaImage::from_pixel(1, 1, Rgba([0, 255, 0, 128]));
        e.init_image(&pixels);
        let mut bytes = vec![];
        write(&mut bytes, &e, &pixels).unwrap();
        assert_eq!(&bytes[bytes.len() - 4..], &[127, 255, 127, 128]);
        assert_eq!(
            crate::asset_viewer::psd::decode(Cursor::new(&bytes)).unwrap(),
            pixels
        );
        assert_eq!(decode(&bytes, (1, 1)).unwrap().0[0].tiles[0], pixels);
    }
}
