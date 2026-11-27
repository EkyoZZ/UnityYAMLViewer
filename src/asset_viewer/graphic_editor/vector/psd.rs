use super::*;

const ERROR: &str = "Unsupported PSD vector path";

fn operation(value: u16) -> Result<PathOp, String> {
    match value {
        0 => Ok(PathOp::Exclude),
        1 => Ok(PathOp::Combine),
        2 => Ok(PathOp::Subtract),
        3 => Ok(PathOp::Intersect),
        _ => Err(ERROR.into()),
    }
}

pub(in crate::asset_viewer::graphic_editor) fn decode(
    data: &[u8],
    dimensions: [u32; 2],
    fill: Option<[u8; 3]>,
    mut rectangles: std::collections::BTreeMap<u32, Geometry>,
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
    let mut remaining = 0;
    let mut contours = Vec::<(PathOp, u32, Vec<Knot>)>::new();
    let mut origins = std::collections::BTreeSet::new();
    for record in bytes.chunks_exact(26) {
        match read_u16(&record[..2]) {
            0 if remaining == 0 && contours.len() < 512 => {
                remaining = usize::from(read_u16(&record[2..4]));
                if !(2..=512).contains(&remaining) {
                    return Err(ERROR.into());
                }
                let raw = read_u16(&record[4..6]);
                let op = if raw == u16::MAX && contours.is_empty() {
                    PathOp::Combine
                } else {
                    operation(raw)?
                };
                let origin = u32::from_be_bytes(record[12..16].try_into().unwrap());
                if !origins.insert(origin) {
                    // Multiple contours sharing an origin cannot each represent that entire rectangle.
                    rectangles.remove(&origin);
                }
                contours.push((op, origin, Vec::with_capacity(remaining)));
            }
            tag @ (1 | 2) if remaining > 0 => {
                remaining -= 1;
                contours.last_mut().unwrap().2.push(Knot {
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
    if remaining != 0 || contours.is_empty() {
        return Err(ERROR.into());
    }
    let inverted = (flags & 1 != 0) ^ (contours[0].0 == PathOp::Subtract);
    let mut geometry_for = |origin, knots| {
        if fill.is_some() && let Some(geometry) = rectangles.remove(&origin) {
            let shape = Vector { geometry, fill, inverted: false };
            if shape.valid() { return shape.geometry; }
        }
        Geometry::Path(knots)
    };
    let geometry = if contours.len() == 1 {
        let (_, origin, knots) = contours.pop().unwrap();
        geometry_for(origin, knots)
    } else {
        if fill.is_none() || inverted || contours[0].0 == PathOp::Subtract {
            return Err(ERROR.into());
        }
        Geometry::Compound(
            contours
                .into_iter()
                .enumerate()
                .map(|(index, (operation, origin, knots))| ShapePart {
                    vector: Vector {
                        geometry: geometry_for(origin, knots),
                        fill,
                        inverted: false,
                    },
                    operation: if index == 0 {
                        PathOp::Combine
                    } else {
                        operation
                    },
                })
                .collect(),
        )
    };
    let vector = Vector {
        geometry,
        fill,
        inverted,
    };
    if !vector.valid() || (fill.is_some() && (inverted || flags & 4 != 0)) {
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
    let mut out = 3u32.to_be_bytes().to_vec();
    let flags = u32::from(vector.inverted) | (u32::from(!linked) << 1) | (u32::from(!enabled) << 2);
    out.extend(flags.to_be_bytes());
    out.extend(record(6));
    out.extend(record(8));
    if let Geometry::Compound(parts) = &vector.geometry {
        for (index, part) in parts.iter().enumerate() {
            if part.vector.fill != vector.fill {
                return Err(ERROR.into());
            }
            write_contour(&mut out, &part.vector, part.operation, dimensions, index as u32)?;
        }
    } else {
        write_contour(&mut out, vector, PathOp::Combine, dimensions, 0)?;
    }
    Ok(out)
}

fn record(selector: u16) -> [u8; 26] {
    let mut bytes = [0u8; 26];
    bytes[..2].copy_from_slice(&selector.to_be_bytes());
    bytes
}

fn write_contour(
    out: &mut Vec<u8>,
    vector: &Vector,
    operation: PathOp,
    dimensions: [u32; 2],
    origin: u32,
) -> Result<(), String> {
    let knots = vector.knots();
    if !(2..=512).contains(&knots.len()) {
        return Err(ERROR.into());
    }
    let code: u16 = match operation {
        PathOp::Exclude => 0,
        PathOp::Combine => 1,
        PathOp::Subtract => 2,
        PathOp::Intersect => 3,
    };
    let mut header = record(0);
    header[2..4].copy_from_slice(&(knots.len() as u16).to_be_bytes());
    header[4..6].copy_from_slice(&code.to_be_bytes());
    header[12..16].copy_from_slice(&origin.to_be_bytes());
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
    Ok(())
}
