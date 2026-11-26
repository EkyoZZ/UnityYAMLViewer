use std::collections::{HashMap, HashSet};
use ufbx::{AnimCurve, AnimStack, ElementType, Interpolation};

pub(crate) struct ResultFile {
    pub bytes: Vec<u8>,
    pub before: usize,
    pub after: usize,
    pub error: f64,
}

fn object_id(element: &ufbx::Element) -> Option<i64> {
    Some(element.dom_node.as_ref()?.values.first()?.value_int)
}

// A line evaluated at the four Bézier control times also has four Bézier
// control values. Their differences bound the error throughout the segment.
fn segment_error(keys: &[ufbx::Keyframe], first: usize, last: usize) -> (usize, f64) {
    let a = keys[first];
    let b = keys[last];
    let line = |time: f64| a.value + (b.value - a.value) * ((time - a.time) / (b.time - a.time));
    let mut worst = (first, 0.0_f64);
    for i in first..last {
        let left = keys[i];
        let right = keys[i + 1];
        let mut error = (left.value - line(left.time))
            .abs()
            .max((right.value - line(right.time)).abs());
        if left.interpolation == Interpolation::Cubic {
            error = error
                .max(
                    (left.value + f64::from(left.right.dy)
                        - line(left.time + f64::from(left.right.dx)))
                    .abs(),
                )
                .max(
                    (right.value
                        - f64::from(right.left.dy)
                        - line(right.time - f64::from(right.left.dx)))
                    .abs(),
                );
        }
        if error > worst.1 {
            worst = (i, error);
        }
    }
    (worst.0.clamp(first, last - 1), worst.1)
}

fn reduce(curve: &AnimCurve, start: f64, end: f64, tolerance: f64) -> Option<(Vec<usize>, f64)> {
    let keys = curve.keyframes.as_ref();
    if !(3..=4096).contains(&keys.len())
        || keys.first()?.time < start - 1e-8
        || keys.last()?.time > end + 1e-8
        || keys.iter().any(|key| {
            !key.time.is_finite()
                || !key.value.is_finite()
                || !matches!(
                    key.interpolation,
                    Interpolation::Linear | Interpolation::Cubic
                )
                || [key.left.dx, key.left.dy, key.right.dx, key.right.dy]
                    .iter()
                    .any(|value| !value.is_finite())
        })
        || keys.windows(2).any(|pair| pair[0].time >= pair[1].time)
    {
        return None;
    }
    let mut keep = vec![0, keys.len() - 1];
    let mut todo = vec![(0, keys.len() - 1)];
    let mut error = 0.0_f64;
    while let Some((first, last)) = todo.pop() {
        let (index, bound) = segment_error(keys, first, last);
        if bound <= tolerance {
            error = error.max(bound);
            continue;
        }
        if last == first + 1 {
            return None;
        }
        let split = index.clamp(first + 1, last - 1);
        keep.push(split);
        todo.extend([(first, split), (split, last)]);
    }
    keep.sort_unstable();
    (keep.len() < keys.len()).then_some((keep, error))
}

fn curves(stack: &AnimStack) -> impl Iterator<Item = (&AnimCurve, &str)> {
    stack
        .layers
        .iter()
        .flat_map(|layer| layer.anim_props.iter())
        .filter(|prop| prop.element.type_ == ElementType::Node)
        .flat_map(|prop| {
            prop.anim_value
                .curves
                .iter()
                .flatten()
                .map(move |curve| (&**curve, prop.prop_name.as_ref()))
        })
}

pub(crate) fn verify(
    original: &ufbx::Scene,
    candidate: &ufbx::Scene,
    stack: &AnimStack,
    expected_keys: usize,
) -> Result<(), String> {
    fn object_ids(scene: &ufbx::Scene) -> Option<Vec<i64>> {
        let root = scene.dom_root.as_ref()?;
        let objects = root
            .children
            .iter()
            .find(|node| node.name.as_ref() == "Objects")?;
        objects
            .children
            .iter()
            .map(|node| Some(node.values.first()?.value_int))
            .collect()
    }
    let original_ids = object_ids(original).ok_or("Cannot read original FBX object IDs")?;
    let mut seen = HashSet::new();
    let keys: usize = curves(stack)
        .filter(|(curve, _)| seen.insert(curve.element.element_id))
        .map(|(curve, _)| curve.keyframes.len())
        .sum();
    if object_ids(candidate).as_ref() != Some(&original_ids) || keys != expected_keys {
        return Err(
            "FBX validation failed: object IDs or animation key counts changed unexpectedly".into(),
        );
    }
    Ok(())
}

pub(crate) fn optimize(
    source: &[u8],
    scene: &ufbx::Scene,
    stack: &AnimStack,
    range: (f64, f64),
    scale: f64,
    tolerance: f64,
) -> Result<ResultFile, String> {
    let mut replacements = HashMap::new();
    let mut shared = HashSet::new();
    let mut seen = HashSet::new();
    for other in &scene.anim_stacks {
        for layer in &other.layers {
            for value in &layer.anim_values {
                for curve in value.curves.iter().flatten() {
                    if other.element.element_id != stack.element.element_id {
                        shared.insert(curve.element.element_id);
                    }
                }
            }
        }
    }
    let mut before = 0;
    let mut removed = 0;
    let mut error = 0.0_f64;
    for (curve, property) in curves(stack) {
        if !seen.insert(curve.element.element_id) {
            continue;
        }
        before += curve.keyframes.len();
        let multiplier = match property {
            "Lcl Translation" => scale.abs(),
            "Lcl Scaling" => 1.0,
            _ => continue,
        };
        if shared.contains(&curve.element.element_id) || curve.element.connections_src.len() != 1 {
            continue;
        }
        let Some(id) = object_id(&curve.element) else {
            continue;
        };
        let Some((keep, bound)) = reduce(
            curve,
            range.0,
            range.1,
            tolerance / multiplier.max(f64::MIN_POSITIVE),
        ) else {
            continue;
        };
        removed += curve.keyframes.len() - keep.len();
        error = error.max(bound * multiplier);
        replacements.insert(id, (curve.keyframes.len(), keep));
    }
    let bytes = if replacements.is_empty() {
        source.to_vec()
    } else {
        rewrite(source, &replacements).ok_or("Unsupported binary FBX curve layout")?
    };
    Ok(ResultFile {
        bytes,
        before,
        after: before - removed,
        error,
    })
}

fn integer(bytes: &[u8], at: usize, width: usize) -> Option<usize> {
    let mut data = [0; 8];
    data[..width].copy_from_slice(bytes.get(at..at.checked_add(width)?)?);
    usize::try_from(u64::from_le_bytes(data)).ok()
}

fn rewrite(source: &[u8], curves: &HashMap<i64, (usize, Vec<usize>)>) -> Option<Vec<u8>> {
    if !source.starts_with(b"Kaydara FBX Binary  \0\x1a\0") {
        return None;
    }
    let version = integer(source, 23, 4)?;
    let width = match version {
        7100..=7499 => 4,
        7500..=7700 => 8,
        _ => return None,
    };
    let mut writer = Writer {
        source,
        width,
        curves,
        output: source[..27].to_vec(),
        written: 0,
    };
    let tail = writer.nodes(27, source.len(), None, 0)?;
    if writer.written != curves.len() * 5 {
        return None;
    }
    // Some exporters emit an entire padding block when already aligned.
    let footer = source.get(tail..)?;
    let padding = footer.len().checked_sub(160)?;
    if padding > 16 || (tail + padding) % 16 != 0
        || footer.get(16..16 + padding)?.iter().any(|&byte| byte != 0)
    {
        return None;
    }
    writer.output.extend_from_slice(&footer[..16]);
    let output_padding = 16 - writer.output.len() % 16;
    writer.output.resize(writer.output.len() + output_padding, 0);
    writer.output.extend_from_slice(&footer[16 + padding..]);
    Some(writer.output)
}

struct Writer<'a> {
    source: &'a [u8],
    width: usize,
    curves: &'a HashMap<i64, (usize, Vec<usize>)>,
    output: Vec<u8>,
    written: usize,
}

impl Writer<'_> {
    fn nodes(
        &mut self,
        mut position: usize,
        limit: usize,
        curve: Option<i64>,
        depth: u8,
    ) -> Option<usize> {
        if depth > 64 {
            return None;
        }
        let header = self.width * 3 + 1;
        while position < limit {
            let end = integer(self.source, position, self.width)?;
            if end == 0 {
                let after = position.checked_add(header)?;
                let sentinel = self.source.get(position..after)?;
                if after > limit || sentinel.iter().any(|&byte| byte != 0) {
                    return None;
                }
                self.output.extend_from_slice(sentinel);
                return Some(after);
            }
            let name_start = position.checked_add(header)?;
            let props_start =
                name_start.checked_add(usize::from(*self.source.get(name_start - 1)?))?;
            let props_end = props_start.checked_add(integer(
                self.source,
                position + 2 * self.width,
                self.width,
            )?)?;
            if props_end > end || end > limit {
                return None;
            }
            let name = self.source.get(name_start..props_start)?;
            let props = self.source.get(props_start..props_end)?;
            let output_start = self.output.len();
            self.output
                .extend_from_slice(self.source.get(position..props_start)?);
            let output_props = self.output.len();
            let mut child_curve = None;
            if name == b"AnimationCurve" && depth == 1 && props.first() == Some(&b'L') {
                child_curve = Some(i64::from_le_bytes(props.get(1..9)?.try_into().ok()?));
            }
            if let Some((count, keep)) = curve.and_then(|id| self.curves.get(&id))
                && let Some(data) = replacement(name, props, *count, keep)?
            {
                self.output.extend_from_slice(&data);
                self.written += 1;
            } else {
                self.output.extend_from_slice(props);
            }
            let prop_size = self.output.len() - output_props;
            if props_end < end && self.nodes(props_end, end, child_curve, depth + 1)? != end {
                return None;
            }
            self.set_offset(output_start, self.output.len())?;
            self.set_offset(output_start + 2 * self.width, prop_size)?;
            position = end;
        }
        Some(position)
    }

    fn set_offset(&mut self, at: usize, value: usize) -> Option<()> {
        if self.width == 4 && value > u32::MAX as usize {
            return None;
        }
        self.output[at..at + self.width]
            .copy_from_slice(&(value as u64).to_le_bytes()[..self.width]);
        Some(())
    }
}

fn replacement(name: &[u8], props: &[u8], count: usize, keep: &[usize]) -> Option<Option<Vec<u8>>> {
    use std::io::Read;
    let (tag, width) = match name {
        b"KeyTime" => (b'l', 8),
        b"KeyValueFloat" => (b'f', 4),
        b"KeyAttrFlags" => (b'i', 4),
        b"KeyAttrDataFloat" => (*props.first()?, 4),
        b"KeyAttrRefCount" => (b'i', 4),
        _ => return Some(None),
    };
    if props.first() != Some(&tag) || !matches!(tag, b'l' | b'f' | b'i') {
        return None;
    }
    let raw = match name {
        b"KeyAttrFlags" => 4_u32.to_le_bytes().to_vec(), // Linear interpolation
        b"KeyAttrDataFloat" => vec![0; 16],
        b"KeyAttrRefCount" => (keep.len() as u32).to_le_bytes().to_vec(),
        _ => {
            if integer(props, 1, 4)? != count
                || integer(props, 9, 4)? != props.len().checked_sub(13)?
            {
                return None;
            }
            let payload = props.get(13..)?;
            let data = match integer(props, 5, 4)? {
                0 => payload.to_vec(),
                1 => {
                    let mut decoded = Vec::new();
                    flate2::read::ZlibDecoder::new(payload)
                        .take((count * width + 1) as u64)
                        .read_to_end(&mut decoded)
                        .ok()?;
                    decoded
                }
                _ => return None,
            };
            if data.len() != count * width {
                return None;
            }
            keep.iter()
                .flat_map(|&i| data[i * width..(i + 1) * width].iter().copied())
                .collect()
        }
    };
    let mut property = vec![tag];
    property.extend_from_slice(&((raw.len() / width) as u32).to_le_bytes());
    property.extend_from_slice(&0_u32.to_le_bytes());
    property.extend_from_slice(&(raw.len() as u32).to_le_bytes());
    property.extend_from_slice(&raw);
    Some(Some(property))
}
