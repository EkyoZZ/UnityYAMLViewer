//! Named function boundary ports shared by expanded and collapsed views.
use super::{Input, Value, egui, operations::Ty, re, stages};
#[derive(Clone, Debug)]
pub(super) struct Port {
    pub name: String,
    pub ty: Ty,
    pub dtype: String,
}
#[derive(Clone, Debug, Default)]
pub(super) struct Interface {
    pub inputs: Vec<Port>,
    pub properties: Vec<usize>,
    pub return_name: String,
    pub return_ty: Ty,
    pub merged: Option<usize>,
    pub single_return: bool,
}
fn dtype(ty: Ty) -> String {
    if ty.width == 1 {
        "float".into()
    } else if ty.width == 0 && !ty.texture {
        "?".into()
    } else {
        ty.label().into()
    }
}
fn shape(ty: &str) -> Ty {
    Ty {
        width: super::probe::width(ty).unwrap_or(0),
        color: false,
        texture: ty.starts_with("sampler"),
    }
}
fn add(ports: &mut Vec<Port>, name: String, ty: Ty, dtype: String) -> usize {
    if let Some(row) = ports.iter().position(|p| p.name == name) {
        return row;
    }
    let row = ports.len();
    ports.push(Port { name, ty, dtype });
    row
}
fn binding_name(input: &Input, channel: &str) -> String {
    match channel {
        "Out" | "RGBA" | "XYZW" | "Tex" => input.name.clone(),
        "ST" => format!("{}_ST", input.name),
        _ => format!("{}.{}", input.name, channel.to_ascii_lowercase()),
    }
}
pub(super) fn build(group: &stages::Group, inputs: &[Input]) -> Interface {
    let mut result = Interface::default();
    let reads = group
        .operations
        .nodes
        .iter()
        .flat_map(|n| &n.inputs)
        .filter(|p| p.external)
        .collect::<Vec<_>>();
    for (name, ty) in group.parameters.iter().zip(&group.argument_types) {
        let matches = reads
            .iter()
            .filter(|p| p.value == *name || p.value.starts_with(&format!("{name}.")))
            .collect::<Vec<_>>();
        if matches.is_empty() {
            let ty = shape(ty);
            add(&mut result.inputs, name.clone(), ty, dtype(ty));
        } else {
            for p in matches {
                add(
                    &mut result.inputs,
                    p.value.clone(),
                    p.ty,
                    if p.value == *name && p.ty.width == 1 {
                        ty.clone()
                    } else {
                        dtype(p.ty)
                    },
                );
            }
        }
    }
    for p in reads {
        add(&mut result.inputs, p.value.clone(), p.ty, dtype(p.ty));
    }
    for (index, channel, _) in &group.inputs {
        let input = &inputs[*index];
        let name = binding_name(input, channel);
        let ty = match input.value {
            Value::Float(..) => Ty {
                width: 1,
                ..Default::default()
            },
            Value::Vector(_, color) => Ty {
                width: if channel == "RGB" || channel == "XYZ" {
                    3
                } else if channel.len() == 1 {
                    1
                } else {
                    4
                },
                color,
                ..Default::default()
            },
            Value::Texture { .. } => Ty {
                texture: true,
                ..Default::default()
            },
        };
        let row = add(
            &mut result.inputs,
            name,
            ty,
            if input.integer {
                "int".into()
            } else {
                dtype(ty)
            },
        );
        result.properties.push(row);
    }
    result.single_return = re(r"\breturn\b").find_iter(&group.body).count() == 1;
    if result.single_return {
        if let Some(&node) = group.flow.outputs.first() {
            let operation = &group.operations.nodes[node];
            result.return_ty = operation.ty;
            result.return_name = group.statements[operation.step]
                .trim()
                .trim_end_matches(';')
                .trim()
                .to_owned();
            if operation.title == "Return" && !operation.fallback && operation.inputs.len() == 1 {
                result.merged = Some(node);
            }
        }
    }
    if result.return_name.is_empty() {
        result.return_name = "return".into();
        result.return_ty = shape(&group.result_type);
    }
    result
}
impl Interface {
    pub fn row(&self, value: &str) -> usize {
        self.inputs
            .iter()
            .position(|p| p.name == value)
            .or_else(|| {
                self.inputs.iter().position(|p| {
                    value.starts_with(&format!("{}.", p.name))
                        || p.name.starts_with(&format!("{value}."))
                })
            })
            .unwrap_or(0)
    }
    pub fn parameter_rows(&self, name: &str) -> Vec<usize> {
        self.inputs
            .iter()
            .enumerate()
            .filter_map(|(row, p)| {
                (p.name == name || p.name.starts_with(&format!("{name}."))).then_some(row)
            })
            .collect()
    }
    pub fn channels(&self) -> Vec<String> {
        let ty = self.return_ty;
        let mut result = Vec::new();
        if ty.width == 4 {
            result.push(if ty.color { "RGB" } else { "XYZ" }.into());
        }
        if ty.width > 1 {
            result.extend(
                if ty.color { "RGBA" } else { "XYZW" }
                    .chars()
                    .take(ty.width)
                    .map(|c| c.to_string()),
            );
        }
        result
    }
    pub fn collapsed_size(&self) -> egui::Vec2 {
        egui::vec2(
            340.,
            76. + self.inputs.len().max(self.channels().len() + 1).max(1) as f32 * 20.,
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn simple_return_is_merged_and_component_inputs_are_named() {
        let g = stages::parse(
            "CGPROGRAM\n#pragma fragment f\nfloat4 f(float4 color):SV_Target{float4 c=color;return c;}\nENDCG",
            &[],
        );
        let interface = build(&g.groups[0], &[]);
        assert!(interface.merged.is_some());
        assert_eq!(interface.return_name, "return c");
        assert!(interface.inputs.iter().any(|p| p.name == "color"));
    }
    #[test]
    fn multiple_returns_never_collapse_into_one_internal_value() {
        let g = stages::parse(
            "CGPROGRAM\n#pragma fragment f\nfloat4 f(float4 c):SV_Target{if(c.a>0.5){return c;}return 0;}\nENDCG",
            &[],
        );
        let interface = build(&g.groups[0], &[]);
        assert!(!interface.single_return);
        assert!(interface.merged.is_none());
    }
}
