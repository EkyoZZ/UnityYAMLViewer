use super::*;

const ERROR: &str = "Unsupported PSD vector path";

pub(in crate::asset_viewer::graphic_editor) fn decode(
    data: &[u8],
    dimensions: [u32; 2],
    fill: Option<[u8; 3]>,
) -> Result<(Vector, bool, bool), String> {
    let (head, mut bytes) = data.split_at_checked(8).ok_or(ERROR)?;
    if bytes.len() % 26 == 2 && data.len() % 4 == 0 {
        bytes = bytes.strip_suffix(&[0, 0]).ok_or(ERROR)?;
    }
    let flags = u32::from_be_bytes(head[4..8].try_into().unwrap());
    if head[..4] != [0, 0, 0, 3] || flags & !7 != 0 || bytes.len() % 26 != 0 {
        return Err(ERROR.into());
    }
    let read_u16 = |s: &[u8]| u16::from_be_bytes(s.try_into().unwrap());
    let coord = |bytes: &[u8], dimension: u32| {
        let value = i32::from_be_bytes(bytes.try_into().unwrap());
        (f64::from(value) / 16777216. * f64::from(dimension)) as f32
    };
    let point = |bytes: &[u8]| {
        [
            coord(&bytes[4..8], dimensions[0]),
            coord(&bytes[..4], dimensions[1]),
        ]
    };
    let mut count = None;
    let mut inverted = flags & 1 != 0;
    let mut knots = Vec::new();
    for record in bytes.chunks_exact(26) {
        match read_u16(&record[..2]) {
            0 if count.is_none() => {
                let length = usize::from(read_u16(&record[2..4]));
                if !(2..=512).contains(&length)
                    || !matches!(read_u16(&record[4..6]), 0 | 1 | 2 | 65535)
                {
                    return Err(ERROR.into());
                }
                inverted ^= read_u16(&record[4..6]) == 2;
                count = Some(length);
            }
            tag @ (1 | 2) if count.is_some_and(|len| knots.len() < len) => {
                knots.push(Knot {
                    incoming: point(&record[2..10]),
                    point: point(&record[10..18]),
                    outgoing: point(&record[18..26]),
                    linked: tag == 1,
                });
            }
            6..=8 => {}
            _ => return Err(ERROR.into()),
        }
    }
    if count != Some(knots.len()) || (fill.is_some() && (inverted || flags & 4 != 0)) {
        return Err(ERROR.into());
    }
    let vector = Vector {
        geometry: Geometry::Path(knots),
        fill,
        inverted,
    };
    if !vector.valid() {
        return Err(ERROR.into());
    }
    Ok((vector, flags & 4 == 0, flags & 2 == 0))
}

pub(in crate::asset_viewer::graphic_editor) fn encode(
    vector: &Vector,
    dimensions: [u32; 2],
    enabled: bool,
    linked: bool,
) -> Result<Vec<u8>, String> {
    let knots = vector.knots();
    let mut out = 3u32.to_be_bytes().to_vec();
    let flags = u32::from(vector.inverted) | (u32::from(!linked) << 1) | (u32::from(!enabled) << 2);
    out.extend(flags.to_be_bytes());
    let record = |selector: u16| {
        let mut bytes = [0u8; 26];
        bytes[..2].copy_from_slice(&selector.to_be_bytes());
        bytes
    };
    out.extend(record(6));
    out.extend(record(8));
    let mut header = record(0);
    header[2..4].copy_from_slice(&(knots.len() as u16).to_be_bytes());
    header[4..6].copy_from_slice(&1u16.to_be_bytes());
    out.extend(header);
    for knot in knots {
        let mut bytes = record(if knot.linked { 1 } else { 2 });
        for (slot, point) in
            bytes[2..]
                .chunks_exact_mut(8)
                .zip([knot.incoming, knot.point, knot.outgoing])
        {
            for (target, axis) in slot.chunks_exact_mut(4).zip([1, 0]) {
                let value = f64::from(point[axis]) / f64::from(dimensions[axis]);
                if !(-16.0..16.0).contains(&value) {
                    return Err(ERROR.into());
                }
                target.copy_from_slice(&((value * 16777216.).round() as i32).to_be_bytes());
            }
        }
        out.extend(bytes);
    }
    Ok(out)
}
