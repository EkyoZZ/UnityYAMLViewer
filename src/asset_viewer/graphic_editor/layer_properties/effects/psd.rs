//! Native Photoshop action descriptors for editable layer effects.
use super::*;

type Result<T> = std::result::Result<T, String>;

struct Descriptor {
    class: Vec<u8>,
    fields: Vec<(Vec<u8>, Value)>,
}

enum Value {
    Bool(bool),
    Number(f64),
    Unit([u8; 4], f64),
    Enum(Vec<u8>, Vec<u8>),
    Object(Descriptor),
    List(Vec<Value>),
    Text(Vec<u8>),
    Integer(i32),
}

const UNSUPPORTED: &str = "Unsupported PSD layer style descriptor";

impl Descriptor {
    fn new(class: &[u8]) -> Self {
        Self {
            class: class.to_vec(),
            fields: Vec::new(),
        }
    }

    fn with(mut self, key: &[u8], value: Value) -> Self {
        self.fields.push((key.to_vec(), value));
        self
    }

    fn get(&self, key: &[u8]) -> Option<&Value> {
        self.fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, value)| value)
    }

    fn boolean(&self, key: &[u8], fallback: bool) -> Result<bool> {
        match self.get(key) {
            Some(Value::Bool(value)) => Ok(*value),
            None => Ok(fallback),
            _ => Err(UNSUPPORTED.into()),
        }
    }

    fn number(&self, key: &[u8], fallback: f64) -> Result<f64> {
        match self.get(key) {
            Some(Value::Number(n) | Value::Unit(_, n)) if n.is_finite() => Ok(*n),
            Some(Value::Integer(n)) => Ok(f64::from(*n)),
            None => Ok(fallback),
            _ => Err(UNSUPPORTED.into()),
        }
    }

    fn enumeration<'a>(&'a self, key: &[u8], fallback: &'a [u8]) -> Result<&'a [u8]> {
        match self.get(key) {
            Some(Value::Enum(_, value)) => Ok(value),
            None => Ok(fallback),
            _ => Err(UNSUPPORTED.into()),
        }
    }

    fn instances(&self, single: &[u8], multi: &[u8]) -> Result<Vec<&Descriptor>> {
        match (self.get(multi), self.get(single)) {
            (Some(Value::List(values)), _) if values.len() <= MAX_INSTANCES => values
                .iter()
                .map(|v| match v {
                    Value::Object(d) => Ok(d),
                    _ => Err(UNSUPPORTED.into()),
                })
                .collect(),
            (None, Some(Value::Object(d))) => Ok(vec![d]),
            (None, None) => Ok(Vec::new()),
            _ => Err(UNSUPPORTED.into()),
        }
    }

    fn with_instances(self, single: &[u8], multi: &[u8], mut values: Vec<Value>) -> Self {
        match values.len() {
            0 => self,
            1 => self.with(single, values.pop().unwrap()),
            _ => self.with(multi, Value::List(values)),
        }
    }

    fn write(&self, out: &mut Vec<u8>) {
        out.extend(0u32.to_be_bytes()); // empty descriptor name
        write_id(out, &self.class);
        out.extend((self.fields.len() as u32).to_be_bytes());
        for (key, value) in &self.fields {
            write_id(out, key);
            value.write(out);
        }
    }
}

fn write_id(out: &mut Vec<u8>, id: &[u8]) {
    let len = if id.len() == 4 { 0 } else { id.len() as u32 };
    out.extend(len.to_be_bytes());
    out.extend(id);
}

impl Value {
    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Self::Bool(value) => {
                out.extend(b"bool");
                out.push(u8::from(*value));
            }
            Self::Number(value) => {
                out.extend(b"doub");
                out.extend(value.to_be_bytes());
            }
            Self::Unit(unit, value) => {
                out.extend(b"UntF");
                out.extend(unit);
                out.extend(value.to_be_bytes());
            }
            Self::Enum(kind, value) => {
                out.extend(b"enum");
                write_id(out, kind);
                write_id(out, value);
            }
            Self::Object(value) => {
                out.extend(b"Objc");
                value.write(out);
            }
            Self::List(values) => {
                out.extend(b"VlLs");
                out.extend((values.len() as u32).to_be_bytes());
                for value in values {
                    value.write(out);
                }
            }
            Self::Text(bytes) => {
                out.extend(b"TEXT");
                out.extend(((bytes.len() / 2) as u32).to_be_bytes());
                out.extend(bytes);
            }
            Self::Integer(n) => {
                out.extend(b"long");
                out.extend(n.to_be_bytes());
            }
        }
    }
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let (bytes, rest) = self.0.split_at_checked(len).ok_or(UNSUPPORTED)?;
        self.0 = rest;
        Ok(bytes)
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn number(&mut self) -> Result<f64> {
        Ok(f64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn id(&mut self) -> Result<Vec<u8>> {
        let len = self.u32()?;
        Ok(self.take(if len == 0 { 4 } else { len as usize })?.to_vec())
    }

    fn text(&mut self) -> Result<Vec<u8>> {
        let len = (self.u32()? as usize).checked_mul(2).ok_or(UNSUPPORTED)?;
        Ok(self.take(len)?.to_vec())
    }

    fn count(&mut self) -> Result<usize> {
        let count = self.u32()? as usize;
        if count > 4096 || count > self.0.len() / 4 {
            return Err(UNSUPPORTED.into());
        }
        Ok(count)
    }

    fn object(&mut self, depth: usize) -> Result<Descriptor> {
        self.text()?;
        let class = self.id()?;
        let mut fields = Vec::new();
        for _ in 0..self.count()? {
            fields.push((self.id()?, self.value(depth + 1)?));
        }
        Ok(Descriptor { class, fields })
    }

    fn value(&mut self, depth: usize) -> Result<Value> {
        if depth > 16 {
            return Err(UNSUPPORTED.into());
        }
        Ok(match self.take(4)? {
            b"bool" => Value::Bool(self.take(1)?[0] != 0),
            b"doub" => Value::Number(self.number()?),
            b"UntF" => Value::Unit(self.take(4)?.try_into().unwrap(), self.number()?),
            b"enum" => Value::Enum(self.id()?, self.id()?),
            b"Objc" | b"GlbO" => Value::Object(self.object(depth)?),
            b"VlLs" => Value::List(
                (0..self.count()?)
                    .map(|_| self.value(depth + 1))
                    .collect::<Result<_>>()?,
            ),
            b"TEXT" => Value::Text(self.text()?),
            b"long" => Value::Integer(self.u32()? as i32),
            _ => return Err(UNSUPPORTED.into()),
        })
    }
}

const MODES: &[(&[u8], Blend)] = &[
    (b"Nrml", Blend::Normal),
    (b"Mltp", Blend::Multiply),
    (b"Scrn", Blend::Screen),
    (b"Ovrl", Blend::Overlay),
    (b"Drkn", Blend::Darken),
    (b"Lghn", Blend::Lighten),
    (b"Dfrn", Blend::Difference),
    (b"Xclu", Blend::Exclusion),
    (b"linearDodge", Blend::Add),
    (b"blendSubtraction", Blend::Subtract),
    (b"SftL", Blend::SoftLight),
    (b"HrdL", Blend::HardLight),
    (b"CDdg", Blend::Dodge),
    (b"CBrn", Blend::Burn),
    (b"blendDivide", Blend::Divide),
    (b"linearBurn", Blend::LinearBurn),
    (b"H   ", Blend::Hue),
    (b"Strt", Blend::Saturation),
    (b"Clr ", Blend::Color),
    (b"Lmns", Blend::Luminosity),
];

fn enumerated(kind: &[u8], value: &[u8]) -> Value {
    Value::Enum(kind.to_vec(), value.to_vec())
}

fn bounded(value: f64, min: i16, max: i16) -> Result<i16> {
    if value < f64::from(min) || value > f64::from(max) {
        return Err(UNSUPPORTED.into());
    }
    Ok(value.round() as i16)
}

fn contour_points(contour: Contour) -> Vec<(f64, f64)> {
    match contour {
        Contour::Linear => vec![(0., 0.), (255., 255.)],
        Contour::Cone => vec![(0., 0.), (127.5, 255.), (255., 0.)],
        Contour::Smooth | Contour::Ring => (0..=12)
            .map(|step| {
                let t = f64::from(step) / 12.;
                let value = if contour == Contour::Smooth {
                    t * t * (3. - 2. * t)
                } else {
                    (t * std::f64::consts::TAU).sin().powi(2)
                };
                (255. * t, 255. * value)
            })
            .collect(),
    }
}

fn contour_value(contour: Contour) -> Value {
    let name = match contour {
        Contour::Linear => "Linear",
        Contour::Smooth => "Smooth",
        Contour::Cone => "Cone",
        Contour::Ring => "Ring",
    };
    let name = name
        .encode_utf16()
        .chain([0])
        .flat_map(u16::to_be_bytes)
        .collect();
    let points = contour_points(contour)
        .into_iter()
        .map(|(x, y)| {
            let point = Descriptor::new(b"CrPt")
                .with(b"Hrzn", Value::Number(x))
                .with(b"Vrtc", Value::Number(y));
            Value::Object(if contour == Contour::Cone && x == 127.5 {
                point.with(b"Cnty", Value::Bool(false))
            } else {
                point
            })
        })
        .collect();
    Value::Object(
        Descriptor::new(b"ShpC")
            .with(b"Nm  ", Value::Text(name))
            .with(b"Crv ", Value::List(points)),
    )
}

fn parse_contour(value: Option<&Value>) -> Result<Contour> {
    let Some(value) = value else {
        return Ok(Contour::Linear);
    };
    let Value::Object(shape) = value else {
        return Err(UNSUPPORTED.into());
    };
    let Some(Value::List(points)) = shape.get(b"Crv ") else {
        return Err(UNSUPPORTED.into());
    };
    let actual = points
        .iter()
        .map(|value| {
            let Value::Object(point) = value else {
                return Err(UNSUPPORTED.into());
            };
            Ok((point.number(b"Hrzn", -1.)?, point.number(b"Vrtc", -1.)?))
        })
        .collect::<Result<Vec<_>>>()?;
    [
        Contour::Linear,
        Contour::Smooth,
        Contour::Cone,
        Contour::Ring,
    ]
    .into_iter()
    .find(|&contour| {
        let expected = contour_points(contour);
        expected.len() == actual.len()
            && expected
                .iter()
                .zip(&actual)
                .all(|(&(x, y), &(a, b))| (x - a).abs() <= 0.5 && (y - b).abs() <= 0.5)
    })
    .ok_or_else(|| UNSUPPORTED.into())
}

fn rgb_value([r, g, b]: [u8; 3]) -> Value {
    Value::Object(
        Descriptor::new(b"RGBC")
            .with(b"Rd  ", Value::Number(r.into()))
            .with(b"Grn ", Value::Number(g.into()))
            .with(b"Bl  ", Value::Number(b.into())),
    )
}

fn common(class: &[u8], enabled: bool, mode: Blend, opacity: u8, color: [u8; 3]) -> Descriptor {
    let mode = MODES.iter().find(|(_, blend)| *blend == mode).unwrap().0;
    Descriptor::new(class)
        .with(b"enab", Value::Bool(enabled))
        .with(b"present", Value::Bool(true))
        .with(b"showInDialog", Value::Bool(true))
        .with(b"Md  ", enumerated(b"BlnM", mode))
        .with(b"Opct", Value::Unit(*b"#Prc", opacity.into()))
        .with(b"Clr ", rgb_value(color))
}

fn read_rgb(value: Option<&Value>) -> Result<[u8; 3]> {
    match value {
        Some(Value::Object(rgb)) if rgb.class == b"RGBC" => Ok([
            bounded(rgb.number(b"Rd  ", 0.)?, 0, 255)? as u8,
            bounded(rgb.number(b"Grn ", 0.)?, 0, 255)? as u8,
            bounded(rgb.number(b"Bl  ", 0.)?, 0, 255)? as u8,
        ]),
        None => Ok([0; 3]),
        _ => Err(UNSUPPORTED.into()),
    }
}

fn parse_common(d: &Descriptor) -> Result<(bool, Blend, u8, [u8; 3])> {
    let enabled = d.boolean(b"enab", true)?;
    let key = d.enumeration(b"Md  ", b"Nrml")?;
    let mode = MODES
        .iter()
        .find(|(id, _)| *id == key)
        .ok_or(UNSUPPORTED)?
        .1;
    let opacity = bounded(d.number(b"Opct", 100.)?, 0, 100)? as u8;
    let color = read_rgb(d.get(b"Clr "))?;
    Ok((enabled, mode, opacity, color))
}

const GRADIENT_STYLES: &[(&[u8], GradientStyle)] = &[
    (b"Lnr ", GradientStyle::Linear),
    (b"Rdl ", GradientStyle::Radial),
    (b"Angl", GradientStyle::Angle),
    (b"Rflc", GradientStyle::Reflected),
    (b"Dmnd", GradientStyle::Diamond),
];

fn write_gradient(d: Descriptor, ramp: Gradient) -> Descriptor {
    let style = GRADIENT_STYLES
        .iter()
        .find(|(_, s)| *s == ramp.style)
        .unwrap()
        .0;
    let stops = [0, 4096];
    let colors = stops
        .into_iter()
        .zip(ramp.colors)
        .map(|(at, color)| {
            Value::Object(
                Descriptor::new(b"Clrt")
                    .with(b"Lctn", Value::Integer(at))
                    .with(b"Mdpn", Value::Integer(50))
                    .with(b"Type", enumerated(b"Clry", b"UsrS"))
                    .with(b"Clr ", rgb_value(color)),
            )
        })
        .collect();
    let alpha = stops
        .into_iter()
        .map(|at| {
            Value::Object(
                Descriptor::new(b"TrnS")
                    .with(b"Lctn", Value::Integer(at))
                    .with(b"Mdpn", Value::Integer(50))
                    .with(b"Opct", Value::Unit(*b"#Prc", 100.)),
            )
        })
        .collect();
    let gradient = Descriptor::new(b"Grdn")
        .with(b"Nm  ", Value::Text(vec![0; 2]))
        .with(b"GrdF", enumerated(b"GrdF", b"CstS"))
        .with(b"Intr", Value::Number(4096.))
        .with(b"Clrs", Value::List(colors))
        .with(b"Trns", Value::List(alpha));
    d.with(b"Grad", Value::Object(gradient))
        .with(b"Angl", Value::Unit(*b"#Ang", ramp.angle.into()))
        .with(b"Type", enumerated(b"GrdT", style))
        .with(b"Rvrs", Value::Bool(ramp.reverse))
        .with(b"Algn", Value::Bool(ramp.align_layer))
        .with(b"Dthr", Value::Bool(ramp.dither))
        .with(b"Scl ", Value::Unit(*b"#Prc", ramp.scale.into()))
}

fn gradient_stops<'a>(gradient: &'a Descriptor, key: &[u8]) -> Result<[&'a Descriptor; 2]> {
    let Some(Value::List(values)) = gradient.get(key) else {
        return Err(UNSUPPORTED.into());
    };
    let [Value::Object(first), Value::Object(last)] = values.as_slice() else {
        return Err(UNSUPPORTED.into());
    };
    if first.number(b"Lctn", -1.)? != 0.
        || last.number(b"Lctn", -1.)? != 4096.
        || first.number(b"Mdpn", 50.)? != 50.
        || last.number(b"Mdpn", 50.)? != 50.
    {
        return Err(UNSUPPORTED.into());
    }
    Ok([first, last])
}

fn read_gradient(d: &Descriptor) -> Result<Gradient> {
    let Some(Value::Object(gradient)) = d.get(b"Grad") else {
        return Err(UNSUPPORTED.into());
    };
    if gradient.enumeration(b"GrdF", b"CstS")? != b"CstS" {
        return Err(UNSUPPORTED.into());
    }
    if let Some(Value::Object(offset)) = d.get(b"Ofst")
        && (offset.number(b"Hrzn", 0.)? != 0. || offset.number(b"Vrtc", 0.)? != 0.)
    {
        return Err(UNSUPPORTED.into());
    }
    let [first, last] = gradient_stops(gradient, b"Clrs")?;
    for stop in [first, last] {
        if stop.enumeration(b"Type", b"UsrS")? != b"UsrS" {
            return Err(UNSUPPORTED.into());
        }
    }
    for stop in gradient_stops(gradient, b"Trns")? {
        if stop.number(b"Opct", 100.)? != 100. {
            return Err(UNSUPPORTED.into());
        }
    }
    let style = d.enumeration(b"Type", b"Lnr ")?;
    let style = GRADIENT_STYLES
        .iter()
        .find(|(k, _)| *k == style)
        .ok_or(UNSUPPORTED)?
        .1;
    let angle = (d.number(b"Angl", 90.)? + 180.).rem_euclid(360.) - 180.;
    Ok(Gradient {
        colors: [read_rgb(first.get(b"Clr "))?, read_rgb(last.get(b"Clr "))?],
        style,
        angle: angle.round() as i16,
        scale: bounded(d.number(b"Scl ", 100.)?, 1, 400)? as u16,
        reverse: d.boolean(b"Rvrs", false)?,
        align_layer: d.boolean(b"Algn", true)?,
        dither: d.boolean(b"Dthr", false)?,
    })
}

fn write_shadow(d: Descriptor, shape: ShadowShape, offset: [i16; 2], size: u8) -> Descriptor {
    let (angle, distance) = shape.polar(offset);
    d.with(b"lagl", Value::Unit(*b"#Ang", angle.into()))
        .with(b"Dstn", Value::Unit(*b"#Pxl", distance.into()))
        .with(b"blur", Value::Unit(*b"#Pxl", size.into()))
        .with(b"Ckmt", Value::Unit(*b"#Pxl", shape.spread.into()))
        .with(b"uglg", Value::Bool(shape.global))
        .with(b"Nose", Value::Unit(*b"#Prc", shape.quality.noise.into()))
        .with(b"AntA", Value::Bool(shape.quality.antialias))
        .with(b"TrnS", contour_value(shape.quality.contour))
}

fn read_quality(d: &Descriptor) -> Result<Quality> {
    Ok(Quality {
        contour: parse_contour(d.get(b"TrnS"))?,
        antialias: d.boolean(b"AntA", false)?,
        noise: bounded(d.number(b"Nose", 0.)?, 0, 100)? as u8,
    })
}

fn read_angle(value: f64) -> i16 {
    if (-180. ..=180.).contains(&value) {
        value.round() as i16
    } else {
        ((value + 180.).rem_euclid(360.) - 180.).round() as i16
    }
}

fn read_shadow(d: &Descriptor) -> Result<ShadowShape> {
    Ok(ShadowShape {
        direction: Some((
            read_angle(d.number(b"lagl", 120.)?),
            bounded(d.number(b"Dstn", 5.)?, 0, 128)? as u8,
        )),
        global: d.boolean(b"uglg", true)?,
        spread: bounded(d.number(b"Ckmt", 0.)?, 0, 100)? as u8,
        quality: read_quality(d)?,
    })
}

fn write_glow(class: &[u8], glow: Glow) -> Value {
    let d = common(
        class,
        glow.enabled,
        glow.ink.mode,
        glow.ink.opacity,
        glow.ink.color,
    )
    .with(
        b"GlwT",
        enumerated(b"BETE", if glow.precise { b"PrBL" } else { b"SfBL" }),
    )
    .with(b"blur", Value::Unit(*b"#Pxl", glow.size.into()))
    .with(b"Ckmt", Value::Unit(*b"#Pxl", glow.spread.into()))
    .with(b"Inpr", Value::Unit(*b"#Prc", glow.range.into()))
    .with(b"ShdN", Value::Unit(*b"#Prc", glow.jitter.into()))
    .with(b"Nose", Value::Unit(*b"#Prc", glow.quality.noise.into()))
    .with(b"AntA", Value::Bool(glow.quality.antialias))
    .with(b"TrnS", contour_value(glow.quality.contour));
    let d = if class == b"IrGl" {
        d.with(
            b"glwS",
            enumerated(b"IGSr", if glow.center { b"SrcC" } else { b"SrcE" }),
        )
    } else {
        d
    };
    Value::Object(if glow.gradient {
        write_gradient(
            d,
            Gradient {
                colors: [glow.ink.color, glow.end_color],
                ..Gradient::default()
            },
        )
    } else {
        d
    })
}

fn read_glow(d: &Descriptor) -> Result<Glow> {
    let (enabled, mode, opacity, color) = parse_common(d)?;
    let ramp = d.get(b"Grad").map(|_| read_gradient(d)).transpose()?;
    let [color, end_color] = match ramp {
        Some(ramp) => {
            if ramp
                != (Gradient {
                    colors: ramp.colors,
                    ..Gradient::default()
                })
            {
                return Err(UNSUPPORTED.into());
            }
            ramp.colors
        }
        None => [color, Glow::default().end_color],
    };
    let center = match d.enumeration(b"glwS", b"SrcE")? {
        b"SrcC" => true,
        b"SrcE" => false,
        _ => return Err(UNSUPPORTED.into()),
    };
    let precise = match d.enumeration(b"GlwT", b"SfBL")? {
        b"PrBL" => true,
        b"SfBL" => false,
        _ => return Err(UNSUPPORTED.into()),
    };
    Ok(Glow {
        enabled,
        ink: Ink {
            mode,
            color,
            opacity,
        },
        size: bounded(d.number(b"blur", 5.)?, 1, 64)? as u8,
        spread: bounded(d.number(b"Ckmt", 0.)?, 0, 100)? as u8,
        range: bounded(d.number(b"Inpr", 50.)?, 1, 100)? as u8,
        jitter: bounded(d.number(b"ShdN", 0.)?, 0, 100)? as u8,
        quality: read_quality(d)?,
        center,
        precise,
        gradient: ramp.is_some(),
        end_color,
    })
}

fn blend_enum(mode: Blend) -> Value {
    enumerated(
        b"BlnM",
        MODES.iter().find(|(_, blend)| *blend == mode).unwrap().0,
    )
}

fn read_ink(d: &Descriptor, mode: &[u8], color: &[u8], opacity: &[u8]) -> Result<Ink> {
    let key = d.enumeration(mode, b"Nrml")?;
    Ok(Ink {
        mode: MODES
            .iter()
            .find(|(id, _)| *id == key)
            .ok_or(UNSUPPORTED)?
            .1,
        color: read_rgb(d.get(color))?,
        opacity: bounded(d.number(opacity, 75.)?, 0, 100)? as u8,
    })
}

fn write_bevel(b: Bevel) -> Value {
    let relief = match b.style {
        Relief::Inner => b"InrB",
        Relief::Outer => b"OtrB",
        Relief::Emboss => b"Embs",
        Relief::Pillow => b"PlEb",
    };
    let technique = match b.technique {
        Carve::Smooth => b"SfBL",
        Carve::Hard => b"PrBL",
        Carve::Soft => b"Slmt",
    };
    Value::Object(
        Descriptor::new(b"ebbl")
            .with(b"enab", Value::Bool(b.enabled))
            .with(b"present", Value::Bool(true))
            .with(b"showInDialog", Value::Bool(true))
            .with(b"bvlS", enumerated(b"bvlS", relief))
            .with(b"bvlT", enumerated(b"bvlT", technique))
            .with(
                b"bvlD",
                enumerated(b"bvlD", if b.down { b"Out " } else { b"In  " }),
            )
            .with(b"srgR", Value::Unit(*b"#Prc", b.depth.into()))
            .with(b"blur", Value::Unit(*b"#Pxl", b.size.into()))
            .with(b"Sftn", Value::Unit(*b"#Pxl", b.soften.into()))
            .with(b"lagl", Value::Unit(*b"#Ang", b.light.angle.into()))
            .with(b"Lald", Value::Unit(*b"#Ang", b.light.altitude.into()))
            .with(b"uglg", Value::Bool(b.global))
            .with(b"TrnS", contour_value(b.contour))
            .with(b"antialiasGloss", Value::Bool(b.antialias))
            .with(b"hglM", blend_enum(b.highlight.mode))
            .with(b"hglC", rgb_value(b.highlight.color))
            .with(b"hglO", Value::Unit(*b"#Prc", b.highlight.opacity.into()))
            .with(b"sdwM", blend_enum(b.shade.mode))
            .with(b"sdwC", rgb_value(b.shade.color))
            .with(b"sdwO", Value::Unit(*b"#Prc", b.shade.opacity.into()))
            .with(b"useShape", Value::Bool(false))
            .with(b"useTexture", Value::Bool(false)),
    )
}

fn read_bevel(d: &Descriptor) -> Result<Bevel> {
    if d.boolean(b"useShape", false)? || d.boolean(b"useTexture", false)? {
        return Err(UNSUPPORTED.into());
    }
    let style = match d.enumeration(b"bvlS", b"InrB")? {
        b"InrB" => Relief::Inner,
        b"OtrB" => Relief::Outer,
        b"Embs" => Relief::Emboss,
        b"PlEb" => Relief::Pillow,
        _ => return Err(UNSUPPORTED.into()),
    };
    let technique = match d.enumeration(b"bvlT", b"SfBL")? {
        b"SfBL" => Carve::Smooth,
        b"PrBL" => Carve::Hard,
        b"Slmt" => Carve::Soft,
        _ => return Err(UNSUPPORTED.into()),
    };
    let down = match d.enumeration(b"bvlD", b"In  ")? {
        b"In  " => false,
        b"Out " => true,
        _ => return Err(UNSUPPORTED.into()),
    };
    Ok(Bevel {
        enabled: d.boolean(b"enab", true)?,
        style,
        technique,
        down,
        depth: bounded(d.number(b"srgR", 100.)?, 1, 1000)? as u16,
        size: bounded(d.number(b"blur", 5.)?, 1, 64)? as u8,
        soften: bounded(d.number(b"Sftn", 0.)?, 0, 16)? as u8,
        light: Light {
            angle: read_angle(d.number(b"lagl", 120.)?),
            altitude: bounded(d.number(b"Lald", 30.)?, 0, 90)? as u8,
        },
        global: d.boolean(b"uglg", true)?,
        contour: parse_contour(d.get(b"TrnS"))?,
        antialias: d.boolean(b"antialiasGloss", false)?,
        highlight: read_ink(d, b"hglM", b"hglC", b"hglO")?,
        shade: read_ink(d, b"sdwM", b"sdwC", b"sdwO")?,
    })
}

impl Effects {
    // Keep the compatibility export for styles whose parameters cannot yet be mapped.
    pub(crate) fn has_unmapped_psd_styles(&self) -> bool {
        self.legacy_appearance && self.active()
    }

    pub(crate) fn encode_psd_styles(&self) -> Option<([u8; 4], Vec<u8>)> {
        if *self == Self::default() {
            return None;
        }
        let strokes = rows(&self.stroke, &self.extra_strokes)
            .filter(|_| self.stroke != Stroke::default() || !self.extra_strokes.is_empty())
            .map(|s| {
                let position: &[u8] = match s.position {
                    Position::Outside => b"OutF",
                    Position::Inside => b"InsF",
                    Position::Center => b"CtrF",
                };
                let paint: &[u8] = if s.gradient { b"GrFl" } else { b"SClr" };
                let stroke = common(b"FrFX", s.enabled, s.mode, s.opacity, s.color)
                    .with(b"Styl", enumerated(b"FStl", position))
                    .with(b"PntT", enumerated(b"FrFl", paint))
                    .with(b"Sz  ", Value::Unit(*b"#Pxl", s.size.into()));
                Value::Object(if s.gradient {
                    write_gradient(stroke, s.ramp)
                } else {
                    stroke
                })
            })
            .collect::<Vec<_>>();
        let shadows = rows(&self.shadow, &self.extra_shadows)
            .filter(|_| self.shadow != DropShadow::default() || !self.extra_shadows.is_empty())
            .map(|s| {
                let d = common(b"DrSh", s.enabled, s.mode, s.opacity, s.color)
                    .with(b"layerConceals", Value::Bool(s.knockout));
                Value::Object(write_shadow(d, s.shape, s.offset, s.radius))
            })
            .collect::<Vec<_>>();
        let overlays = rows(&self.overlay, &self.extra_overlays)
            .filter(|_| self.overlay != ColorOverlay::default() || !self.extra_overlays.is_empty())
            .map(|s| Value::Object(common(b"SoFi", s.enabled, s.mode, s.opacity, s.color)))
            .collect::<Vec<_>>();
        let gradients = rows(&self.gradient, &self.extra_gradients)
            .filter(|_| {
                self.gradient != GradientOverlay::default() || !self.extra_gradients.is_empty()
            })
            .map(|s| {
                Value::Object(write_gradient(
                    common(b"GrFl", s.enabled, s.mode, s.opacity, [0; 3]),
                    s.ramp,
                ))
            })
            .collect::<Vec<_>>();
        let inner = rows(&self.inner_shadow, &self.extra_inner_shadows)
            .filter(|_| {
                self.inner_shadow != InnerShadow::default() || !self.extra_inner_shadows.is_empty()
            })
            .map(|s| {
                Value::Object(write_shadow(
                    common(b"IrSh", s.enabled, s.ink.mode, s.ink.opacity, s.ink.color),
                    s.shape,
                    [0; 2],
                    s.size,
                ))
            })
            .collect::<Vec<_>>();
        let multi = [
            strokes.len(),
            shadows.len(),
            overlays.len(),
            gradients.len(),
            inner.len(),
        ]
        .into_iter()
        .any(|n| n > 1);
        let mut root = Descriptor::new(b"null")
            .with(b"Scl ", Value::Unit(*b"#Prc", 100.))
            .with(b"masterFXSwitch", Value::Bool(!self.master_disabled))
            .with(
                b"numModifyingFX",
                Value::Integer(self.pages().filter(|p| self.enabled(*p)).count() as i32),
            )
            .with_instances(b"FrFX", b"frameFXMulti", strokes)
            .with_instances(b"DrSh", b"dropShadowMulti", shadows)
            .with_instances(b"SoFi", b"solidFillMulti", overlays)
            .with_instances(b"GrFl", b"gradientFillMulti", gradients)
            .with_instances(b"IrSh", b"innerShadowMulti", inner);
        for (key, glow) in [(b"IrGl", self.inner_glow), (b"OrGl", self.outer_glow)] {
            if glow != Glow::default() {
                root = root.with(key, write_glow(key, glow));
            }
        }
        if self.bevel != Bevel::default() {
            root = root.with(b"ebbl", write_bevel(self.bevel));
        }
        let mut bytes = vec![0, 0, 0, 0, 0, 0, 0, 16];
        root.write(&mut bytes);
        Some((if multi { *b"lmfx" } else { *b"lfx2" }, bytes))
    }

    pub(crate) fn decode_psd_styles(bytes: &[u8]) -> Result<Self> {
        let mut input = Reader(bytes);
        if input.u32()? != 0 || input.u32()? != 16 {
            return Err(UNSUPPORTED.into());
        }
        let root = input.object(0)?;
        if input.0.iter().any(|b| *b != 0) || root.number(b"Scl ", 100.)? != 100. {
            return Err(UNSUPPORTED.into());
        }
        for (key, value) in &root.fields {
            if !matches!(
                key.as_slice(),
                b"Scl "
                    | b"masterFXSwitch"
                    | b"numModifyingFX"
                    | b"FrFX"
                    | b"frameFXMulti"
                    | b"DrSh"
                    | b"dropShadowMulti"
                    | b"SoFi"
                    | b"solidFillMulti"
                    | b"GrFl"
                    | b"gradientFillMulti"
                    | b"IrSh"
                    | b"innerShadowMulti"
                    | b"IrGl"
                    | b"OrGl"
                    | b"ebbl"
            ) && !matches!(value, Value::Object(d) if !d.boolean(b"present", true)?)
            {
                return Err(UNSUPPORTED.into());
            }
        }
        let mut result = Self {
            master_disabled: !root.boolean(b"masterFXSwitch", true)?,
            ..Self::default()
        };
        let mut strokes = Vec::new();
        for d in root.instances(b"FrFX", b"frameFXMulti")? {
            if !d.boolean(b"present", true)? {
                continue;
            }
            let (enabled, mode, opacity, color) = parse_common(d)?;
            if d.boolean(b"overprint", false)? {
                return Err(UNSUPPORTED.into());
            }
            let (gradient, ramp) = match d.enumeration(b"PntT", b"SClr")? {
                b"SClr" => (false, Gradient::default()),
                b"GrFl" => (true, read_gradient(d)?),
                _ => return Err(UNSUPPORTED.into()),
            };
            let position = match d.enumeration(b"Styl", b"OutF")? {
                b"OutF" => Position::Outside,
                b"InsF" => Position::Inside,
                b"CtrF" => Position::Center,
                _ => return Err(UNSUPPORTED.into()),
            };
            strokes.push(Stroke {
                enabled,
                mode,
                opacity,
                color,
                position,
                size: bounded(d.number(b"Sz  ", 3.)?, 0, 64)? as u8,
                gradient,
                ramp,
            });
        }
        let mut shadows = Vec::new();
        for d in root.instances(b"DrSh", b"dropShadowMulti")? {
            if !d.boolean(b"present", true)? {
                continue;
            }
            let (enabled, mode, opacity, color) = parse_common(d)?;
            shadows.push(DropShadow {
                enabled,
                mode,
                opacity,
                color,
                shape: read_shadow(d)?,
                radius: bounded(d.number(b"blur", 5.)?, 0, 64)? as u8,
                knockout: d.boolean(b"layerConceals", true)?,
                offset: [0; 2],
            });
        }
        let mut overlays = Vec::new();
        for d in root.instances(b"SoFi", b"solidFillMulti")? {
            if !d.boolean(b"present", true)? {
                continue;
            }
            let (enabled, mode, opacity, color) = parse_common(d)?;
            overlays.push(ColorOverlay {
                enabled,
                mode,
                opacity,
                color,
            });
        }
        if !strokes.is_empty() {
            result.stroke = strokes.remove(0);
            result.extra_strokes = strokes;
        }
        if !shadows.is_empty() {
            result.shadow = shadows.remove(0);
            result.extra_shadows = shadows;
        }
        if !overlays.is_empty() {
            result.overlay = overlays.remove(0);
            result.extra_overlays = overlays;
        }
        let mut gradients = Vec::new();
        for d in root.instances(b"GrFl", b"gradientFillMulti")? {
            if !d.boolean(b"present", true)? {
                continue;
            }
            let (enabled, mode, opacity, _) = parse_common(d)?;
            gradients.push(GradientOverlay {
                enabled,
                mode,
                opacity,
                ramp: read_gradient(d)?,
            });
        }
        if !gradients.is_empty() {
            result.gradient = gradients.remove(0);
            result.extra_gradients = gradients;
        }
        let mut inner = Vec::new();
        for d in root.instances(b"IrSh", b"innerShadowMulti")? {
            if !d.boolean(b"present", true)? {
                continue;
            }
            let (enabled, mode, opacity, color) = parse_common(d)?;
            inner.push(InnerShadow {
                enabled,
                ink: Ink {
                    mode,
                    opacity,
                    color,
                },
                shape: read_shadow(d)?,
                size: bounded(d.number(b"blur", 5.)?, 0, 64)? as u8,
            });
        }
        if !inner.is_empty() {
            result.inner_shadow = inner.remove(0);
            result.extra_inner_shadows = inner;
        }
        for (key, setting) in [
            (b"IrGl", &mut result.inner_glow),
            (b"OrGl", &mut result.outer_glow),
        ] {
            if let Some(Value::Object(d)) = root.get(key)
                && d.boolean(b"present", true)?
            {
                *setting = read_glow(d)?;
            }
        }
        if let Some(Value::Object(d)) = root.get(b"ebbl")
            && d.boolean(b"present", true)?
        {
            result.bevel = read_bevel(d)?;
        }
        Ok(result)
    }
}
