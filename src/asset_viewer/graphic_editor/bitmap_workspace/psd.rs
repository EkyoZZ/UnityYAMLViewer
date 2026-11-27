//! 8-bit RGB PSD pixel layers and masks. Unsupported native layer structures
//! go through the existing merged preview and cannot overwrite their source.
use super::super::layer_properties::{Blend, Properties, effects::Light};
use super::super::vector::{Geometry, Vector};
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
    rect: [i32; 4],
    channels: Vec<(i16, usize)>,
    name: String,
    visible: bool,
    settings: Properties,
    mask: Option<([i32; 4], u8)>,
}
fn record(data: &mut Bytes, canvas: [u32; 2]) -> Result<Record> {
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
    let mut vector_mask = None;
    let mut vector_fill = None;
    let mut live_shape = None;
    let mut vector_origination_version = false;
    let mut has_shape_stroke = false;
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
            b"uyTx" => {}
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
            b"shmd" if len <= 1024 * 1024 => tag(&mut settings.psd_metadata, b"shmd", block.0),
            b"TySh" => {
                settings.psd_native_text = true;
                settings.text = text_layer::psd::read(block.0, canvas);
                if settings.text.is_some() {
                    settings.psd_tysh = Some(block.0.into());
                }
            }
            b"lfx2" | b"lfxs" => style = Some(block.0),
            b"lmfx" => multi_style = Some(block.0),
            b"lrFX" => legacy_style = true,
            b"vmsk" | b"vsms" => vector_mask = Some(block.0),
            b"SoCo" => {
                vector_fill = Some(super::super::layer_properties::read_solid_fill(block.0)?)
            }
            b"vscg" if block.0.starts_with(b"SoCo") => {
                vector_fill = Some(super::super::layer_properties::read_solid_fill(
                    &block.0[4..],
                )?);
            }
            b"vogk" => live_shape = Some(block.0),
            // Photoshop adds this version marker beside its shape origination data.
            // Editable contours still come from vmsk/vsms, including compound shapes.
            b"vowv" if block.0 == [0, 0, 0, 2] => vector_origination_version = true,
            b"vstk" => {
                if !super::super::layer_properties::solid_shape_stroke_supported(block.0)? {
                    return Err("Unsupported PSD vector stroke or disabled fill".into());
                }
                has_shape_stroke = true;
            }
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
    if settings.psd_native_text
        && ([-1, 0, 1, 2].into_iter().any(|id| {
            !channels
                .iter()
                .any(|&(channel, len)| channel == id && len > 2)
        }) || rect[0] == rect[2]
            || rect[1] == rect[3])
    {
        return Err("PSD native text has no usable layer pixels".into());
    }
    if !extra.0.iter().all(|v| *v == 0) {
        return Err("Invalid trailing layer data".into());
    }
    if vector_origination_version && (vector_mask.is_none() || vector_fill.is_none()) {
        return Err("PSD vector origination marker without a supported shape".into());
    }
    if let Some(path) = vector_mask {
        if divider != 0 || settings.psd_native_text || (mask.is_some() && vector_fill.is_none()) {
            return Err(
                "Unsupported PSD vector mask combined with another mask or layer type".into(),
            );
        }
        let rectangles = live_shape.and_then(super::super::layer_properties::read_live_rectangles)
            .unwrap_or_default();
        let (vector, enabled, linked) =
            super::super::vector::psd::decode(path, canvas, vector_fill, rectangles)?;
        if vector.fill.is_none() {
            settings.mask_enabled = enabled;
            settings.mask_linked = linked;
        }
        settings.vector = Some(vector);
    } else if vector_fill.is_some() || live_shape.is_some() || has_shape_stroke {
        return Err("PSD shape missing its vector path".into());
    }
    Ok(Record {
        divider,
        locked,
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

fn resources_with_light(resources: &[u8], light: Light, merged: &RgbaImage) -> Result<Vec<u8>> {
    let mut section = Bytes(resources);
    let mut output = Vec::new();
    let mut has_version_info = false;
    while !section.0.is_empty() {
        let start = section.0;
        section.take(4)?;
        let id = section.word()?;
        let name_len = usize::from(section.byte()?);
        section.take(name_len + usize::from(name_len % 2 == 0))?;
        let data_len = section.section()?.0.len();
        section.take(data_len % 2)?;
        if !matches!(id, 1033 | 1036 | 1037 | 1049 | 1057) {
            output.extend(&start[..start.len() - section.0.len()]);
        } else if id == 1057 && data_len >= 5 {
            let record = &start[..start.len() - section.0.len()];
            let offset = output.len() + record.len() - data_len - data_len % 2;
            output.extend(record);
            output[offset + 4] = 1;
            has_version_info = true;
        }
    }
    if !has_version_info {
        let mut version = vec![0, 0, 0, 1, 1]; // Version 1, real merged image present.
        for name in ["Unity YAML Viewer", "Unity YAML Viewer"] {
            version.extend((name.encode_utf16().count() as u32).to_be_bytes());
            version.extend(name.encode_utf16().flat_map(u16::to_be_bytes));
        }
        version.extend(1u32.to_be_bytes());
        output.extend(b"8BIM");
        output.extend(1057u16.to_be_bytes());
        output.extend([0, 0]);
        output.extend((version.len() as u32).to_be_bytes());
        output.extend(version);
        output.push(0); // The resource payload has an odd length.
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
    write_thumbnail(&mut output, merged)?;
    Ok(output)
}

fn write_thumbnail(output: &mut Vec<u8>, merged: &RgbaImage) -> Result<()> {
    let (w, h) = merged.dimensions();
    let longest = w.max(h);
    if longest == 0 {
        return Err("Invalid PSD thumbnail dimensions".into());
    }
    let thumbnail = if longest > 160 {
        image::imageops::resize(
            merged,
            (w * 160 / longest).max(1),
            (h * 160 / longest).max(1),
            image::imageops::FilterType::Triangle,
        )
    } else {
        merged.clone()
    };
    let (w, h) = thumbnail.dimensions();
    let rgb = image::RgbImage::from_fn(w, h, |x, y| {
        let [r, g, b, alpha] = thumbnail.get_pixel(x, y).0;
        image::Rgb([r, g, b].map(|channel| {
            let alpha = u32::from(alpha);
            ((u32::from(channel) * alpha + 255 * (255 - alpha) + 127) / 255) as u8
        }))
    });
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 88)
        .encode_image(&rgb)
        .map_err(|error| error.to_string())?;
    let row_bytes = (w * 3).next_multiple_of(4);
    output.extend(b"8BIM");
    output.extend(1036u16.to_be_bytes());
    output.extend([0, 0]);
    output.extend((28 + jpeg.len() as u32).to_be_bytes());
    for word in [1, w, h, row_bytes, row_bytes * h, jpeg.len() as u32] {
        output.extend(word.to_be_bytes());
    }
    output.extend(24u16.to_be_bytes());
    output.extend(1u16.to_be_bytes());
    output.extend(&jpeg);
    if jpeg.len() % 2 != 0 {
        output.push(0);
    }
    Ok(())
}
pub(super) fn has_merged_preview(bytes: &[u8]) -> Result<bool> {
    let mut data = Bytes(bytes);
    data.take(26)?;
    data.section()?;
    image_resources(data.section()?.0, None, None)
}
fn retain_photoshop_metadata(key: &[u8], data: &[u8]) -> bool {
    match key {
        // A disabled Content Authenticity descriptor, with an empty generation GUID.
        b"CAI " => data == b"\0\0\0\x03\0\0\0\x10\0\0\0\x01\0\0\0\0\0\0null\0\0\0\x02\0\0\0\0enabbool\0\0\0\0\x10generationalGuidTEXT\0\0\0\x01\0\0\0\0\0\0\0\0\0\0",
        // The descriptor reports no generative technology in use.
        b"GenI" => data == b"\0\0\0\x10\0\0\0\x01\0\0\0\0\0\x0bgenTechInfo\0\0\0\x01\0\0\0\x0eisUsingGenTechlong\0\0\0\0",
        // Photoshop's empty filter-mask record, without an associated filter.
        b"FMsk" => data == [0, 0, 255, 255, 0, 0, 0, 0, 0, 0, 0, 50],
        // Source compositor information is a document descriptor.
        b"cinf" => data.len() <= 4096
            && data.starts_with(b"\0\0\0\x10\0\0\0\x01\0\0\0\0\0\0null\0\0\0\x08\0\0\0\0VrsnObjc"),
        _ => false,
    }
}

pub(super) fn decode(
    bytes: &[u8],
    (w, h): (u32, u32),
) -> Result<(Vec<Layer>, Vec<u8>, Vec<u8>, bool)> {
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
        return Ok((vec![], resources, vec![], false));
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
        .map(|_| record(&mut info, [w, h]))
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
            .map(|r| {
                4 + u64::from(
                    r.mask.is_some()
                        || r.settings.vector.as_ref().is_some_and(|v| v.fill.is_none()),
                )
            })
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
        if let Some(vector) = &r.settings.vector {
            let raster = vector
                .raster(w, h)
                .ok_or("Unable to render PSD vector path")?;
            if vector.fill.is_some() {
                im = raster;
            } else {
                r.settings.mask = Some(vec![raster.pixels().map(|p| p[3]).collect()]);
            }
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
    let mut patterns = Vec::new();
    let mut has_unwritten_metadata = false;
    while layers.0.len() >= 12 && layers.0.starts_with(b"8BIM") {
        let start = layers.0;
        layers.take(4)?;
        let key = layers.take(4)?;
        let block = layers.section()?;
        let pattern = matches!(key, b"Patt" | b"Pat2" | b"Pat3");
        let retained_metadata = retain_photoshop_metadata(key, block.0);
        let unwritten_metadata = !retained_metadata && matches!(key, b"CAI " | b"GenI" | b"cinf");
        has_unwritten_metadata |= unwritten_metadata;
        if !pattern
            && !retained_metadata
            && !unwritten_metadata
            && (key != b"Txt2" || !result.iter().any(|l| l.settings.psd_native_text))
        {
            return Err(format!(
                "Additional PSD layer data {}",
                String::from_utf8_lossy(key)
            ));
        }
        for _ in 0..(4 - block.0.len() % 4) % 4 {
            if layers.0.first() == Some(&0) {
                layers.take(1)?;
            }
        }
        if pattern || retained_metadata || key == b"Txt2" {
            patterns.extend_from_slice(&start[..start.len() - layers.0.len()]);
        }
    }
    if layers.0.iter().any(|&b| b != 0) {
        return Err("Additional global PSD layer data".into());
    }
    let text_count = result
        .iter()
        .filter(|layer| layer.settings.psd_native_text)
        .count();
    for layer in &mut result {
        if layer.settings.psd_tysh.is_some() {
            layer.settings.psd_text_count = text_count;
        }
    }
    Ok((result, resources, patterns, has_unwritten_metadata))
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
    let size = bytes.len().next_multiple_of(4);
    extra.extend(b"8BIM");
    extra.extend(key);
    extra.extend((size as u32).to_be_bytes());
    extra.extend(bytes);
    extra.resize(extra.len() + size - bytes.len(), 0);
}

fn write_layer_metadata(extra: &mut Vec<u8>, metadata: &[u8]) -> Result<()> {
    let mut data = Bytes(metadata);
    while !data.0.is_empty() {
        if data.take(4)? != b"8BIM" {
            return Err("Invalid PSD layer metadata signature".into());
        }
        let key = data.take(4)?.try_into().unwrap();
        let block = data.section()?.0;
        tag(extra, key, block);
        data.take(block.len() % 2)?;
    }
    Ok(())
}
fn rect(out: &mut Vec<u8>, w: u32, h: u32) {
    for n in [0, 0, h, w] {
        out.extend(n.to_be_bytes());
    }
}
fn global_blocks(
    source: &[u8],
    keep_text_engine: bool,
    changes: &[(usize, &[u8])],
) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut blocks = Bytes(source);
    while !blocks.0.is_empty() {
        let start = blocks.0;
        blocks.take(4)?;
        let key = blocks.take(4)?;
        let payload = blocks.section()?.0;
        for _ in 0..(4 - payload.len() % 4) % 4 {
            if blocks.0.first() == Some(&0) {
                blocks.take(1)?;
            }
        }
        if key != b"Txt2" || keep_text_engine {
            output.extend_from_slice(&start[..start.len() - blocks.0.len()]);
        } else if !changes.is_empty()
            && let Some(updated) = text_layer::psd::update_txt2(payload, changes)
        {
            output.extend(b"8BIMTxt2");
            output.extend((updated.len() as u32).to_be_bytes());
            output.extend(updated);
            output.resize(output.len().next_multiple_of(4), 0);
        }
    }
    Ok(output)
}

fn expand_compound_shapes(layers: &[Layer], w: u32, h: u32) -> Result<Vec<Layer>> {
    let mut group_id = layers.iter().map(|layer| layer.settings.group_id).max().unwrap_or(0);
    let mut expanded = Vec::with_capacity(layers.len());
    for layer in layers {
        let Some(Vector { geometry: Geometry::Compound(parts), .. }) = &layer.settings.vector else {
            expanded.push(layer.clone());
            continue;
        };
        let mut runs = parts.chunk_by(|a, b| a.vector.fill == b.vector.fill).peekable();
        let first = runs.next().ok_or("Empty PSD compound shape")?;
        if runs.peek().is_none() {
            let mut layer = layer.clone();
            layer.settings.vector.as_mut().unwrap().fill = first[0].vector.fill;
            expanded.push(layer);
            continue;
        }

        group_id = group_id.checked_add(1).ok_or("No available PSD group ID")?;
        let mut settings = layer.settings.clone();
        settings.group_id = group_id;
        settings.vector = None;
        settings.vector_origin = None;
        settings.pass_through = false;
        expanded.push(Layer { settings, tiles: Vec::new(), ..layer.clone() });
        for (index, run) in std::iter::once(first).chain(runs).enumerate() {
            let vector = if run.len() == 1 {
                run[0].vector.clone()
            } else {
                Vector {
                    geometry: Geometry::Compound(run.to_vec()),
                    fill: run[0].vector.fill,
                    inverted: false,
                }
            };
            let raster = vector.raster(w, h).ok_or("Unable to render PSD shape")?;
            expanded.push(Layer {
                name: format!("{} {}", tr("形状"), index + 1),
                settings: Properties { vector: Some(vector), parent: group_id, ..Default::default() },
                visible: true,
                material: false,
                locked: false,
                tiles: vec![raster],
            });
        }
    }
    Ok(expanded)
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
    let expanded;
    let layers = if flat_only {
        std::slice::from_ref(&flat)
    } else if e.layers.iter().any(|layer| layer.settings.vector.as_ref().is_some_and(Vector::is_compound)) {
        expanded = expand_compound_shapes(&e.layers, w, h)?;
        &expanded
    } else {
        &e.layers
    };
    let mut records = vec![];
    layer_records(layers, 0, &mut records);
    let mut native_indices = std::collections::HashSet::new();
    let mut text_count = 0;
    let mut source_text_counts = Vec::new();
    let original_type_data: Vec<_> = records
        .iter()
        .map(|(layer, divider)| {
            let p = &layer.settings;
            if *divider != 0 || p.text.as_ref().is_none_or(|text| text.repeat) {
                return None;
            }
            text_count += 1;
            let raw = p.psd_tysh.as_deref()?;
            let index = text_layer::psd::text_index(raw)?;
            if text_layer::psd::read(raw, [w, h]).as_ref() != p.text.as_ref()
                || !native_indices.insert(index)
            {
                return None;
            }
            source_text_counts.push(p.psd_text_count);
            Some(raw)
        })
        .collect();
    let keep_text_engine = text_count != 0
        && source_text_counts.len() == text_count
        && source_text_counts.iter().all(|&count| count == text_count);
    let mut source_indices = std::collections::HashSet::new();
    let can_update_text_engine = text_count != 0
        && records
            .iter()
            .filter(|(layer, divider)| {
                *divider == 0
                    && layer
                        .settings
                        .text
                        .as_ref()
                        .is_some_and(|text| !text.repeat)
            })
            .all(|(layer, _)| {
                let p = &layer.settings;
                p.psd_text_count == text_count
                    && p.psd_tysh
                        .as_deref()
                        .and_then(text_layer::psd::text_index)
                        .is_some_and(|index| source_indices.insert(index))
            });
    let mut text_changes = Vec::new();
    let mut table = Vec::new();
    let merged_has_alpha = merged.pixels().any(|pixel| pixel[3] != 255);
    let layer_count = records.len() as i16;
    table.extend(
        if merged_has_alpha {
            -layer_count
        } else {
            layer_count
        }
        .to_be_bytes(),
    );
    let mut plane_bytes = 0usize;
    let reserved: std::collections::HashSet<u32> = layers
        .iter()
        .filter_map(|l| l.settings.psd_layer_id)
        .collect();
    let mut used = std::collections::HashSet::new();
    let mut next_id = 1;
    let mut text_index = 0;
    for (&(l, divider), original_tysh) in records.iter().zip(original_type_data) {
        let defaults = Properties::default();
        let p = if divider == 3 { &defaults } else { &l.settings };
        let pixels = if divider == 0 { w * h } else { 0 };
        if divider == 0 {
            rect(&mut table, w, h)
        } else {
            rect(&mut table, 0, 0)
        }
        let mask = p.mask.is_some() && p.vector.as_ref().is_none_or(|v| v.fill.is_some());
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
        write_layer_metadata(&mut extra, &p.psd_metadata)?;
        if let Some(vector) = &p.vector {
            let shape = vector.fill.is_some();
            let path = super::super::vector::psd::encode(
                vector,
                [w, h],
                shape || p.mask_enabled,
                shape || p.mask_linked,
            )?;
            tag(&mut extra, b"vmsk", &path);
            if let Some(color) = vector.fill {
                let fill = super::super::layer_properties::write_solid_fill(color);
                tag(&mut extra, b"SoCo", &fill);
                if let Some(origination) =
                    super::super::layer_properties::write_live_rectangles(&vector.geometry)
                {
                    tag(&mut extra, b"vogk", &origination);
                }
            }
        }
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
        if divider == 0
            && let Some(text) = p.text.as_ref().filter(|text| !text.repeat)
        {
            if let Some(raw) = original_tysh {
                tag(&mut extra, b"TySh", raw);
            } else {
                let index = if can_update_text_engine {
                    text_layer::psd::text_index(p.psd_tysh.as_deref().unwrap()).unwrap()
                } else {
                    while native_indices.contains(&text_index) {
                        text_index += 1;
                    }
                    let index = text_index;
                    text_index += 1;
                    index
                };
                let tysh =
                    text_layer::psd::write(text, [w, h], index, e.font_samples.catalog.as_deref());
                tag(&mut extra, b"TySh", &tysh);
                if can_update_text_engine {
                    text_changes.push((index as usize, tysh));
                }
            }
        }
        table.extend((extra.len() as u32).to_be_bytes());
        table.extend(extra);
    }
    let changes: Vec<_> = text_changes
        .iter()
        .map(|(index, tysh)| (*index, tysh.as_slice()))
        .collect();
    let global_data = global_blocks(&e.psd_patterns, keep_text_engine, &changes)?;
    let len = table.len() + plane_bytes;
    let resources = resources_with_light(&e.psd_resources, e.fx_light, merged)?;
    let mut write = || -> io::Result<()> {
        out.write_all(b"8BPS\x00\x01\x00\x00\x00\x00\x00\x00")?;
        let merged_channels = if merged_has_alpha { 4u16 } else { 3u16 };
        out.write_all(&merged_channels.to_be_bytes())?;
        out.write_all(&h.to_be_bytes())?;
        out.write_all(&w.to_be_bytes())?;
        out.write_all(&[0, 8, 0, 3, 0, 0, 0, 0])?;
        out.write_all(&(resources.len() as u32).to_be_bytes())?;
        out.write_all(&resources)?;
        out.write_all(&((len.next_multiple_of(2) + 8 + global_data.len()) as u32).to_be_bytes())?;
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
                && l.settings.vector.as_ref().is_none_or(|v| v.fill.is_some())
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
        out.write_all(&global_data)?;
        out.write_all(&[0; 2])?;
        for c in 0..usize::from(merged_channels) {
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
    fn photoshop_layer_tags_count_padding_inside_the_payload() {
        let mut bytes = Vec::new();
        tag(&mut bytes, b"SoCo", &[1, 2, 3, 4, 5]);
        tag(&mut bytes, b"lyid", &1u32.to_be_bytes());
        let mut reader = Bytes(&bytes);
        assert_eq!(reader.take(8).unwrap(), b"8BIMSoCo");
        assert_eq!(reader.section().unwrap().0, &[1, 2, 3, 4, 5, 0, 0, 0]);
        assert_eq!(reader.take(8).unwrap(), b"8BIMlyid");
    }

    #[test]
    fn psd_thumbnail_encodes_the_composite_in_rgb_order() {
        let pixels = RgbaImage::from_pixel(2, 2, Rgba([230, 40, 25, 255]));
        let mut bytes = Vec::new();
        write_thumbnail(&mut bytes, &pixels).unwrap();
        let mut reader = Bytes(&bytes);
        assert_eq!(reader.take(4).unwrap(), b"8BIM");
        assert_eq!(reader.word().unwrap(), 1036);
        reader.take(2).unwrap();
        let data = reader.section().unwrap();
        let rgb = image::load_from_memory(&data.0[28..]).unwrap().to_rgb8();
        let [r, g, b] = rgb.get_pixel(0, 0).0;
        assert!(r > 200 && g < 60 && b < 60);
    }
    #[test]
    fn editable_psd_rectangle_keeps_its_rounding_and_pixel_cache() {
        use crate::asset_viewer::graphic_editor::vector::{Geometry, Vector};
        let mut editor = Editor::default();
        editor.init_image(&RgbaImage::new(24, 16));
        let vector = Vector {
            geometry: Geometry::Rectangle {
                min: [2., 3.],
                max: [18., 12.],
                radii: [3., 1., 4., 0.],
                angle: 0.,
            },
            fill: Some([11, 183, 104]),
            inverted: false,
        };
        let pixels = vector.raster(24, 16).unwrap();
        editor.layers[0].settings.vector = Some(vector.clone());
        editor.layers[0].tiles[0] = pixels.clone();
        editor.tiles[0] = pixels.clone();
        editor.pixels = pixels.clone();
        let mut bytes = Vec::new();
        write(&mut bytes, &editor, &pixels).unwrap();
        let (layers, _, _, _) = decode(&bytes, (24, 16)).unwrap();
        assert!(layers[0].settings.vector.as_ref() == Some(&vector));
        assert_eq!(layers[0].tiles[0], pixels);
        assert!(bytes.windows(4).any(|key| key == b"vogk"));
    }

    #[test]
    fn compound_psd_preserves_operations_and_groups_color_runs_in_order() {
        use super::super::super::vector::{PathOp, ShapePart};

        let red = Some([170, 0, 0]);
        let blue = Some([0, 0, 190]);
        let shape = |x, fill, operation| ShapePart {
            vector: Vector {
                geometry: Geometry::Rectangle {
                    min: [x, 2.],
                    max: [x + 8., 9.],
                    radii: [0.; 4],
                    angle: 0.,
                },
                fill,
                inverted: false,
            },
            operation,
        };
        let mut parts = vec![
            shape(1., red, PathOp::Combine),
            shape(3., red, PathOp::Subtract),
            shape(3., red, PathOp::Intersect),
            shape(5., red, PathOp::Exclude),
        ];
        parts[1].vector.geometry = Geometry::Path(parts[1].vector.knots());
        if let Geometry::Rectangle { radii, .. } = &mut parts[2].vector.geometry {
            *radii = [1., 2., 0., 3.];
        }
        for mixed in [false, true] {
            if mixed {
                parts.extend([
                    shape(4., blue, PathOp::Combine),
                    shape(6., red, PathOp::Combine),
                ]);
            }
            let vector = Vector { geometry: Geometry::Compound(parts.clone()), fill: red, inverted: false };
            let pixels = vector.raster(24, 16).unwrap();
            let mut editor = Editor::default();
            editor.init_image(&pixels);
            editor.layers[0].settings.vector = Some(vector);
            if !mixed {
                tag(&mut editor.layers[0].settings.psd_metadata, b"vowv", &2_u32.to_be_bytes());
            }
            let mut bytes = Vec::new();
            write(&mut bytes, &editor, &pixels).unwrap();
            let (layers, _, _, _) = decode(&bytes, (24, 16)).unwrap();
            let shapes: Vec<_> = layers.iter().filter(|l| l.settings.vector.is_some()).collect();
            assert_eq!(shapes.len(), if mixed { 3 } else { 1 });
            if mixed {
                let group = layers.iter().find(|l| l.settings.group_id != 0).unwrap();
                assert!(shapes.iter().all(|l| l.settings.parent == group.settings.group_id));
                assert_eq!(shapes.iter().map(|l| l.settings.vector.as_ref().unwrap().fill).collect::<Vec<_>>(),
                    vec![red, blue, red]);
            }
            let shape = shapes[0].settings.vector.as_ref().unwrap();
            let Geometry::Compound(restored) = &shape.geometry else {
                panic!("PSD must retain the subpaths");
            };
            assert_eq!(restored.len(), 4);
            assert!(restored.iter().zip(&parts).all(|(actual, expected)| actual.operation == expected.operation));
            for (actual, expected) in restored.iter().zip(&parts) {
                if matches!(expected.vector.geometry, Geometry::Path(_)) {
                    assert!(matches!(actual.vector.geometry, Geometry::Path(_)));
                } else {
                    assert!(actual.vector.geometry == expected.vector.geometry);
                }
            }
        }
    }

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
                 "stroke_gradient": true, "stroke_ramp": {"colors": [[5, 50, 150], [220, 130, 15]],
                    "stops": [
                        {"position": 0, "color": [5, 50, 150]},
                        {"position": 1433, "color": [30, 220, 70]},
                        {"position": 4096, "color": [220, 130, 15]},
                        null, null, null, null, null, null, null, null, null, null, null, null, null
                    ]}}
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
        let (layers, resources, _, _) = decode(&bytes, (16, 16)).unwrap();
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
        tag(&mut top.settings.psd_metadata, b"shmd", &[0; 4]);
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
        tag(&mut editor.psd_patterns, b"Patt", &[5, 10, 15, 20]);
        tag(&mut editor.psd_patterns, b"Pat3", &[1, 2, 3, 4]);
        let mut expected_patterns = editor.psd_patterns.clone();
        let empty_filter_mask = [0, 0, 255, 255, 0, 0, 0, 0, 0, 0, 0, 50];
        tag(&mut expected_patterns, b"FMsk", &empty_filter_mask);
        tag(&mut editor.psd_patterns, b"CAI ", &[7, 3, 9]);
        tag(&mut editor.psd_patterns, b"GenI", &[1, 2, 3]);
        tag(
            &mut editor.psd_patterns,
            b"FMsk",
            &[0, 0, 255, 255, 0, 0, 0, 0, 0, 0, 0, 50],
        );
        tag(&mut editor.psd_patterns, b"cinf", &[1, 2]);
        let composite = editor.composite(0);
        let mut bytes = vec![];
        write(&mut bytes, &editor, &composite).unwrap();
        let (layers, _, patterns, has_unwritten_metadata) = decode(&bytes, (3, 2)).unwrap();
        assert_eq!(patterns, expected_patterns);
        assert!(has_unwritten_metadata);
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].tiles[0], bottom);
        assert_eq!(layers[1].name, top.name);
        assert_eq!(layers[1].settings.psd_metadata, top.settings.psd_metadata);
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
    #[ignore = "requires the user's original Photoshop fixture"]
    fn photoshop_sabi_fixture_opens_as_editable_text() {
        let path = std::env::var_os("UYV_PSD_TEXT_FIXTURE").expect("fixture path");
        let bytes = std::fs::read(path).expect("read Photoshop fixture");
        let (layers, resources, patterns, protected) =
            decode(&bytes, (256, 128)).expect("parse layered Photoshop fixture");
        assert!(!protected, "the fixture metadata should round-trip");
        let source_metadata: Vec<_> = [b"CAI ", b"GenI", b"FMsk", b"cinf"]
            .into_iter()
            .map(|key| {
                let offset = patterns
                    .windows(8)
                    .position(|block| block.starts_with(b"8BIM") && &block[4..] == key)
                    .expect("source metadata must be preserved");
                let mut record = Bytes(&patterns[offset + 8..]);
                (key, record.section().unwrap().0.to_vec())
            })
            .collect();
        let names: Vec<_> = layers.iter().map(|layer| layer.name.as_str()).collect();
        assert_eq!(names, ["图层 1", "图像", "SABI"]);
        let text = layers[2].settings.text.as_ref().expect("editable SABI");
        assert_eq!(text.content, "SABI");
        assert_eq!(text.family, "GenJyuuGothic-Heavy");
        let mut editor = Editor::default();
        editor.init_image(&RgbaImage::new(256, 128));
        editor.layers = layers;
        editor.psd_resources = resources;
        editor.psd_patterns = patterns;
        let original_tysh = editor.layers[2].settings.psd_tysh.clone();
        assert!(original_tysh.is_some());
        assert!(
            editor
                .psd_patterns
                .windows(8)
                .any(|bytes| bytes == b"8BIMTxt2")
        );
        editor.layers[0].tiles[0].put_pixel(0, 0, image::Rgba([12, 34, 56, 255]));
        let mut saved = Vec::new();
        write(&mut saved, &editor, &editor.composite(0)).expect("save non-text edit");
        let (saved_layers, _, saved_global, _) =
            decode(&saved, (256, 128)).expect("read non-text edit");
        assert_eq!(saved_layers[2].settings.psd_tysh, original_tysh);
        assert_eq!(saved_global, editor.psd_patterns);
        if let Some(output) = std::env::var_os("UYV_PSD_UNCHANGED_TEXT_OUTPUT") {
            std::fs::write(output, saved).expect("save unchanged text fixture");
        }
        let layer = &mut editor.layers[2];
        let text = layer.settings.text.as_mut().unwrap();
        text.content = "SABI edited".into();
        text.family = "ArialMT".into();
        let fonts = text_layer::font_samples::Catalog::read();
        layer.tiles[0] = text.raster(256, 128, &fonts).expect("render edited text");
        let mut exported = Vec::new();
        write(&mut exported, &editor, &editor.composite(0)).expect("write native PSD text");
        let (reopened, _, globals, _) = decode(&exported, (256, 128)).expect("reopen exported PSD");
        assert_eq!(reopened.len(), 3);
        assert!(globals.windows(8).any(|bytes| bytes == b"8BIMTxt2"));
        assert_ne!(globals, editor.psd_patterns);
        for (key, source) in source_metadata {
            let offset = globals
                .windows(8)
                .position(|block| block.starts_with(b"8BIM") && &block[4..] == key)
                .expect("edited export should retain source metadata");
            let mut record = Bytes(&globals[offset + 8..]);
            assert_eq!(record.section().unwrap().0, source);
        }
        assert_eq!(
            reopened[2].settings.text.as_ref().unwrap().content,
            "SABI edited"
        );
        if let Some(output) = std::env::var_os("UYV_PSD_TEXT_OUTPUT") {
            std::fs::write(output, exported).expect("save fixture for Photoshop verification");
        }
    }

    #[test]
    fn repository_psds_with_unsupported_structures_use_existing_composite() {
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
