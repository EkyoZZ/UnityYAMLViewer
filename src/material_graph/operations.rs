//! Expression graphs built from shader source; independently implemented here.
use super::{HashMap, Input, Value, egui, flow, probe, re, stages};
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Ty {
    pub width: usize,
    pub color: bool,
    pub texture: bool,
}
impl Ty {
    fn numeric(width: usize, color: bool) -> Self {
        Self {
            width,
            color: color && (1..=4).contains(&width),
            texture: false,
        }
    }
    pub fn label(self) -> &'static str {
        if self.texture {
            "Tex"
        } else {
            match (self.width, self.color) {
                (1, _) => "Out",
                (2, true) => "RG",
                (2, false) => "XY",
                (3, true) => "RGB",
                (4, true) => "RGBA",
                (3, false) => "XYZ",
                (4, false) => "XYZW",
                _ => "Out",
            }
        }
    }
}
#[derive(Clone, Debug)]
pub(super) struct Port {
    pub name: String,
    pub value: String,
    pub from: Option<usize>,
    pub channel: Option<String>,
    pub external: bool,
    pub ty: Ty,
}
#[derive(Clone, Debug)]
pub(super) struct Node {
    pub title: String,
    pub inputs: Vec<Port>,
    pub ty: Ty,
    pub expression: String,
    pub step: usize,
    pub target: String,
    pub probe: Result<probe::Probe, String>,
    pub fallback: bool,
}
#[derive(Clone, Debug, Default)]
pub(super) struct Graph {
    pub nodes: Vec<Node>,
    pub roots: Vec<usize>,
    pub limited: bool,
}
#[derive(Clone, Debug)]
enum Expr {
    Leaf(String),
    Member(Box<Expr>, String),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}
impl Expr {
    fn text(&self) -> String {
        match self {
            Self::Leaf(s) => s.clone(),
            Self::Member(e, s) => format!("({}).{s}", e.text()),
            Self::Unary(op, e) => format!("({op}{})", e.text()),
            Self::Binary(op, a, b) => format!("({} {op} {})", a.text(), b.text()),
            Self::Call(name, args) => format!(
                "{name}({})",
                args.iter().map(Self::text).collect::<Vec<_>>().join(", ")
            ),
        }
    }
    fn reference(&self) -> Option<String> {
        match self {
            Self::Leaf(s) if !number(s) => Some(s.clone()),
            Self::Member(e, s) => e.reference().map(|e| format!("{e}.{s}")),
            _ => None,
        }
    }
}
fn number(s: &str) -> bool {
    s.trim_end_matches(['f', 'F', 'h', 'H'])
        .parse::<f64>()
        .is_ok()
}
struct Parser {
    tokens: Vec<String>,
    at: usize,
}
fn parse(text: &str) -> Result<Expr, String> {
    // Restore numeric suffixes separated by the source viewer's tokenizer.
    let normalized =
        re(r"((?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)\s+([fFhH])\b").replace_all(text, "$1$2");
    let text = normalized.as_ref();
    if re(r"\+\s*\+|-\s*-").is_match(text) {
        return Err("Increment / decrement expressions remain source statements".into());
    }
    let pattern = re(
        r"(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?[fFhH]?|[A-Za-z_]\w*|==|!=|<=|>=|&&|\|\||[()+*/%,.<>!~&|^\-]",
    );
    let mut last = 0;
    let mut tokens = Vec::new();
    for m in pattern.find_iter(text) {
        if !text[last..m.start()].trim().is_empty() {
            return Err("Unsupported expression syntax".into());
        }
        tokens.push(m.as_str().to_owned());
        last = m.end();
    }
    if !text[last..].trim().is_empty() || tokens.is_empty() || tokens.len() > 512 {
        return Err("Unsupported or oversized expression".into());
    }
    let mut p = Parser { tokens, at: 0 };
    let result = p.expr(0, 0)?;
    if p.at != p.tokens.len() {
        return Err("Expression has unsupported trailing syntax".into());
    }
    Ok(result)
}
impl Parser {
    fn take(&mut self, s: &str) -> bool {
        if self.tokens.get(self.at).is_some_and(|t| t == s) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn expr(&mut self, min: u8, depth: usize) -> Result<Expr, String> {
        if depth > 48 {
            return Err("Expression nesting limit".into());
        }
        let token = self
            .tokens
            .get(self.at)
            .cloned()
            .ok_or("Missing expression")?;
        self.at += 1;
        let mut left = if ["+", "-", "!", "~"].contains(&token.as_str()) {
            Expr::Unary(token, Box::new(self.expr(90, depth + 1)?))
        } else if token == "(" {
            let e = self.expr(0, depth + 1)?;
            if !self.take(")") {
                return Err("Unclosed parentheses".into());
            }
            e
        } else if number(&token) || re(r"^[A-Za-z_]\w*$").is_match(&token) {
            Expr::Leaf(token)
        } else {
            return Err("Invalid expression operand".into());
        };
        loop {
            if self.take(".") {
                let field = self.tokens.get(self.at).cloned().ok_or("Missing member")?;
                if !re(r"^[A-Za-z_]\w*$").is_match(&field) {
                    return Err("Invalid member".into());
                }
                self.at += 1;
                left = Expr::Member(Box::new(left), field);
                continue;
            }
            if self.tokens.get(self.at).is_some_and(|s| s == "(") {
                let Expr::Leaf(name) = left else {
                    return Err("Only named calls are supported".into());
                };
                self.at += 1;
                let mut args = Vec::new();
                if !self.take(")") {
                    loop {
                        args.push(self.expr(0, depth + 1)?);
                        if self.take(")") {
                            break;
                        }
                        if !self.take(",") {
                            return Err("Missing argument separator".into());
                        }
                    }
                }
                left = Expr::Call(name, args);
                continue;
            }
            let Some(op) = self.tokens.get(self.at) else {
                break;
            };
            let priority = match op.as_str() {
                "||" => 10,
                "&&" => 20,
                "|" => 25,
                "^" => 26,
                "&" => 27,
                "==" | "!=" => 30,
                "<" | ">" | "<=" | ">=" => 40,
                "+" | "-" => 50,
                "*" | "/" | "%" => 60,
                _ => break,
            };
            if priority < min {
                break;
            }
            let op = op.clone();
            self.at += 1;
            let right = self.expr(priority + 1, depth + 1)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }
}
fn color_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    ["color", "colour", "tint", "albedo", "emission", "alpha", "opacity"]
        .iter()
        .any(|s| n.contains(s))
}
fn vector_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    ["normal", "tangent", "position", "texcoord"]
        .iter()
        .any(|s| n.contains(s))
        || n.split('.').any(|s| s.starts_with("uv"))
}
fn swizzle(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 4
        && (s.chars().all(|c| "rgba".contains(c)) || s.chars().all(|c| "xyzw".contains(c)))
}
fn type_name(name: &str) -> Ty {
    if name.starts_with("sampler") {
        Ty {
            texture: true,
            ..Default::default()
        }
    } else {
        Ty::numeric(probe::width(name).unwrap_or(0), false)
    }
}
fn types(group: &stages::Group, source: &str, inputs: &[Input]) -> HashMap<String, Ty> {
    let mut result = HashMap::new();
    let numeric = re(r"\b((?:fixed|half|float|int|uint|bool)[1-4]?|sampler2D)\s+(\w+)\b");
    for c in numeric
        .captures_iter(source)
        .chain(numeric.captures_iter(&group.signature))
        .chain(numeric.captures_iter(&group.body))
    {
        let mut ty = type_name(&c[1]);
        ty.color = ty.width > 0 && color_name(&c[2]);
        result.insert(c[2].to_owned(), ty);
    }
    for input in inputs {
        let ty = match input.value {
            Value::Float(..) => Ty::numeric(1, false),
            Value::Vector(_, color) => Ty::numeric(4, color),
            Value::Texture { .. } => Ty {
                texture: true,
                ..Default::default()
            },
        };
        result.insert(input.name.clone(), ty);
    }
    for schema in re(r"(?s)\bstruct\s+(\w+)\s*\{([^}]*)\}").captures_iter(source) {
        let mut owners = group
            .argument_types
            .iter()
            .zip(&group.parameters)
            .filter(|(ty, _)| **ty == schema[1])
            .map(|(_, p)| p.clone())
            .collect::<Vec<_>>();
        owners.extend(
            re(&format!(r"\b{}\s+(\w+)\s*;", regex::escape(&schema[1])))
                .captures_iter(&group.body)
                .map(|c| c[1].to_owned()),
        );
        for owner in owners {
            for field in numeric.captures_iter(&schema[2]) {
                let mut ty = type_name(&field[1]);
                ty.color = ty.width > 0
                    && (color_name(&field[2])
                        || re(&format!(r"\b{}\s*:\s*COLOR", regex::escape(&field[2])))
                            .is_match(&schema[2]));
                result.insert(format!("{owner}.{}", &field[2]), ty);
            }
        }
    }
    result
        .entry("_Time".into())
        .or_insert(Ty::numeric(4, false));
    result
}
fn has_custom_call(expression: &str, calls: &HashMap<String, (String, Vec<String>)>) -> bool {
    re(r"\b([A-Za-z_]\w*)\s*\(")
        .captures_iter(expression)
        .any(|c| calls.contains_key(&c[1]))
}
struct Builder<'a> {
    group: &'a stages::Group,
    graph: Graph,
    types: HashMap<String, Ty>,
    calls: &'a HashMap<String, (String, Vec<String>)>,
    helper_vertex: Result<bool, String>,
    step: usize,
}
impl Builder<'_> {
    fn ty(&self, name: &str) -> Ty {
        if let Some(ty) = self.types.get(name) {
            *ty
        } else if let Some((base, field)) = name.rsplit_once('.') {
            if swizzle(field) {
                let ty = self.ty(base);
                Ty::numeric(
                    field.len(),
                    ty.color || field.chars().all(|c| "rgba".contains(c)),
                )
            } else {
                Ty::default()
            }
        } else {
            Ty::default()
        }
    }
    fn reference(&mut self, name: &str) -> Port {
        let ty = self.ty(name);
        let mut sources = self.group.flow.reads[self.step]
            .iter()
            .filter(|edge| edge.value == name || edge.value.starts_with(&format!("{name}.")))
            .filter_map(|edge| edge.from)
            .filter_map(|step| self.graph.roots.get(step).copied())
            .collect::<Vec<_>>();
        sources.sort();
        sources.dedup();
        let from = if sources.len() == 1 {
            Some(sources[0])
        } else if sources.len() > 1 {
            // A component-written variable has several producers. Keep the read explicit,
            // instead of inventing arithmetic that reconstructs its shader state.
            let ports = sources
                .into_iter()
                .enumerate()
                .map(|(i, from)| Port {
                    name: format!("Source {}", i + 1),
                    value: self.graph.nodes[from].expression.clone(),
                    from: Some(from),
                    channel: None,
                    external: false,
                    ty: self.graph.nodes[from].ty,
                })
                .collect();
            Some(self.push(format!("Read {name}"), ports, ty, name.to_owned(), false))
        } else {
            None
        };
        Port {
            name: String::new(),
            value: name.into(),
            from,
            channel: None,
            external: from.is_none(),
            ty,
        }
    }
    fn push(
        &mut self,
        title: String,
        inputs: Vec<Port>,
        ty: Ty,
        expression: String,
        fallback: bool,
    ) -> usize {
        let index = self.graph.nodes.len();
        let mut p = probe::expression_candidate(self.group, self.step, &expression, ty.width);
        if self.group.stage == stages::Stage::Function {
            p = match &self.helper_vertex {
                Ok(vertex) => p.map(|mut p| {
                    p.vertex = *vertex;
                    p
                }),
                Err(error) => Err(error.clone()),
            };
        }
        if !ty.color || !(1..=4).contains(&ty.width) || fallback {
            p = Err("This node has no color preview".into());
        }
        if has_custom_call(&expression, self.calls) {
            p = Err("Function call preview requires its completed assignment".into());
        }
        self.graph.nodes.push(Node {
            title,
            inputs,
            ty,
            expression,
            step: self.step,
            target: String::new(),
            probe: p,
            fallback,
        });
        index
    }
    fn node_value(&self, index: usize) -> Port {
        let node = &self.graph.nodes[index];
        Port {
            name: String::new(),
            value: node.expression.clone(),
            from: Some(index),
            channel: None,
            external: false,
            ty: node.ty,
        }
    }
    fn expression(&mut self, expr: &Expr) -> Result<Port, String> {
        if self.graph.nodes.len() >= 224 {
            return Err("Operation node budget reached".into());
        }
        let expression = expr.text();
        match expr {
            Expr::Leaf(s) if number(s) || s == "true" || s == "false" => Ok(Port {
                name: String::new(),
                value: s.clone(),
                from: None,
                channel: None,
                external: false,
                ty: Ty::numeric(1, false),
            }),
            Expr::Leaf(s) => Ok(self.reference(s)),
            Expr::Member(base, field) => {
                if let Some(name) = expr.reference() {
                    let value = self.reference(&name);
                    if !swizzle(field) {
                        return Ok(value);
                    }
                    if let Some(from) = value.from {
                        let source = self.graph.nodes[from].ty;
                        if source.width == 1 && value.ty.width == 1 {
                            return Ok(value);
                        }
                        let canonical = field
                            .chars()
                            .map(|c| "rgba".find(c).or_else(|| "xyzw".find(c)))
                            .collect::<Option<Vec<_>>>();
                        if let Some(indices) = canonical {
                            if indices.iter().all(|index| *index < source.width) {
                                let channel = if source.width == value.ty.width
                                    && indices.iter().copied().eq(0..source.width)
                                {
                                    Some(None)
                                } else if source.width == 1 && field.len() == 1 {
                                    Some(None)
                                } else if field.len() == 1 {
                                    Some(Some(
                                        if source.color { "RGBA" } else { "XYZW" }
                                            .chars()
                                            .nth(indices[0])
                                            .unwrap()
                                            .to_string(),
                                    ))
                                } else if source.width == 4 && indices == vec![0, 1, 2] {
                                    Some(Some(if source.color { "RGB" } else { "XYZ" }.into()))
                                } else {
                                    None
                                };
                                if let Some(channel) = channel {
                                    return Ok(Port { channel, ..value });
                                }
                            }
                        }
                    } else {
                        return Ok(value);
                    }
                    let mut input = value.clone();
                    if let Some(from) = input.from {
                        input.ty = self.graph.nodes[from].ty;
                    } else if let Some(base) = base.reference() {
                        input.ty = self.ty(&base);
                        input.value = base;
                    }
                    let i = self.push(
                        format!("Swizzle {}", field.to_ascii_uppercase()),
                        vec![Port {
                            name: "In".into(),
                            ..input
                        }],
                        value.ty,
                        expression,
                        false,
                    );
                    return Ok(self.node_value(i));
                }
                if !swizzle(field) {
                    return Err("Unsupported result member".into());
                }
                let mut value = self.expression(base)?;
                let ty = Ty::numeric(field.len(), value.ty.color || field.chars().all(|c| "rgba".contains(c)));
                if let Some(from) = value.from {
                    let source = self.graph.nodes[from].ty;
                    let indices = field
                        .chars()
                        .map(|c| "rgba".find(c).or_else(|| "xyzw".find(c)))
                        .collect::<Option<Vec<_>>>();
                    if let Some(indices) = indices {
                        if indices.iter().all(|i| *i < source.width) {
                            let channel = if source.width == field.len()
                                && indices.iter().copied().eq(0..source.width)
                            {
                                Some(None)
                            } else if field.len() == 1 {
                                Some(Some(
                                    if source.color { "RGBA" } else { "XYZW" }
                                        .chars()
                                        .nth(indices[0])
                                        .unwrap()
                                        .to_string(),
                                ))
                            } else if source.width == 4 && indices == vec![0, 1, 2] {
                                Some(Some(if source.color { "RGB" } else { "XYZ" }.into()))
                            } else {
                                None
                            };
                            if let Some(channel) = channel {
                                value.channel = channel;
                                value.ty = ty;
                                value.value = expression;
                                return Ok(value);
                            }
                        }
                    }
                }
                value.name = "In".into();
                let i = self.push(
                    format!("Swizzle {}", field.to_ascii_uppercase()),
                    vec![value],
                    ty,
                    expression,
                    false,
                );
                Ok(self.node_value(i))
            }
            Expr::Unary(op, e) => {
                let mut value = self.expression(e)?;
                value.name = "In".into();
                let ty = if op == "!" {
                    Ty::numeric(1, value.ty.color)
                } else {
                    value.ty
                };
                let i = self.push(
                    match op.as_str() {
                        "-" => "Negate",
                        "+" => "Positive",
                        "!" => "Not",
                        _ => "Bitwise Not",
                    }
                    .into(),
                    vec![value],
                    ty,
                    expression,
                    false,
                );
                Ok(self.node_value(i))
            }
            Expr::Binary(op, a, b) => {
                let mut a = self.expression(a)?;
                let mut b = self.expression(b)?;
                a.name = "A".into();
                b.name = "B".into();
                let numeric = ["+", "-", "*", "/", "%"].contains(&op.as_str());
                let ty = if numeric {
                    Ty::numeric(a.ty.width.max(b.ty.width), a.ty.color || b.ty.color)
                } else {
                    Ty::numeric(a.ty.width.max(b.ty.width), a.ty.color || b.ty.color)
                };
                let title = match op.as_str() {
                    "+" => "Add",
                    "-" => "Subtract",
                    "*" => "Multiply",
                    "/" => "Divide",
                    "%" => "Remainder",
                    "==" => "Equal",
                    "!=" => "Not Equal",
                    "<" => "Less",
                    ">" => "Greater",
                    "<=" => "Less Equal",
                    ">=" => "Greater Equal",
                    "&&" => "And",
                    "||" => "Or",
                    _ => op,
                };
                let i = self.push(title.into(), vec![a, b], ty, expression, false);
                Ok(self.node_value(i))
            }
            Expr::Call(name, args) => {
                let mut ports = args
                    .iter()
                    .map(|a| self.expression(a))
                    .collect::<Result<Vec<_>, _>>()?;
                let names: Vec<String> = match name.as_str() {
                    "lerp" => vec!["A", "B", "T"],
                    "tex2D" => vec!["Tex", "UV"],
                    "pow" => vec!["Base", "Exp"],
                    "clamp" => vec!["In", "Min", "Max"],
                    "step" => vec!["Edge", "In"],
                    "smoothstep" => vec!["Min", "Max", "In"],
                    "saturate" | "normalize" | "abs" | "sin" | "cos" | "frac" | "floor"
                    | "ceil" => vec!["In"],
                    _ => vec![],
                }
                .into_iter()
                .map(str::to_owned)
                .collect();
                let names = self
                    .calls
                    .get(name)
                    .map_or(names, |(_, names)| names.clone());
                for (i, p) in ports.iter_mut().enumerate() {
                    p.name = names
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| ((b'A' + (i.min(25)) as u8) as char).to_string());
                }
                let constructor = probe::width(name);
                let sampled = name == "tex2D";
                let scalar = ["dot", "length", "distance"].contains(&name.as_str());
                let width = if sampled {
                    4
                } else if scalar {
                    1
                } else if let Some(width) = constructor {
                    width
                } else if let Some((ty, _)) = self.calls.get(name) {
                    probe::width(ty).unwrap_or(0)
                } else if [
                    "lerp",
                    "pow",
                    "clamp",
                    "step",
                    "smoothstep",
                    "saturate",
                    "normalize",
                    "abs",
                    "sin",
                    "cos",
                    "frac",
                    "floor",
                    "ceil",
                    "min",
                    "max",
                    "sqrt",
                    "exp",
                    "log",
                    "cross",
                ]
                .contains(&name.as_str())
                {
                    ports.iter().map(|p| p.ty.width).max().unwrap_or(0)
                } else {
                    0
                };
                let ty = Ty::numeric(width, sampled || ports.iter().any(|p| p.ty.color));
                let title = match name.as_str() {
                    "tex2D" => "Texture Sample",
                    "lerp" => "Lerp",
                    "pow" => "Power",
                    "saturate" => "Saturate",
                    "dot" => "Dot",
                    "normalize" => "Normalize",
                    _ => name,
                };
                let i = self.push(title.into(), ports, ty, expression, false);
                Ok(self.node_value(i))
            }
        }
    }
}
fn statement(text: &str) -> Result<(String, Expr, bool), String> {
    if re(r"[{}]|\b(if|else|for|while|switch|do|discard)\b").is_match(text) {
        return Err("Control flow remains a source statement".into());
    }
    let text = text.trim().trim_end_matches(';').trim();
    if let Some(value) = text.strip_prefix("return ") {
        return Ok(("return".into(), parse(value)?, true));
    }
    let assignment = re(
        r"^\s*(?:(?:fixed|half|float|int|uint|bool)[1-4]?\s+)?([A-Za-z_]\w*(?:\s*\.\s*\w+)*)\s*([+*/%\-]?)\s*=\s*(.+)$",
    );
    let c = assignment
        .captures(text)
        .ok_or("Uninitialized or unsupported statement")?;
    let target = c[1].split_whitespace().collect::<String>();
    let rhs = parse(&c[3])?;
    let expr = if c[2].is_empty() {
        rhs
    } else {
        Expr::Binary(c[2].to_owned(), Box::new(parse(&target)?), Box::new(rhs))
    };
    Ok((target, expr, false))
}
pub(super) fn build(
    group: &stages::Group,
    source: &str,
    inputs: &[Input],
    calls: &HashMap<String, (String, Vec<String>)>,
    helper_vertex: Result<bool, String>,
) -> Graph {
    let mut b = Builder {
        group,
        graph: Graph::default(),
        types: types(group, source, inputs),
        calls,
        helper_vertex,
        step: 0,
    };
    for (step, text) in group.statements.iter().enumerate() {
        b.step = step;
        let checkpoint = b.graph.nodes.len();
        let result = statement(text).and_then(|(target, expr, returned)| {
            let expected = if returned {
                type_name(&group.result_type)
            } else {
                b.ty(&target)
            };
            let value = b.expression(&expr)?;
            let mut index = if let Some(index) = value.from.filter(|index| {
                *index >= checkpoint
                    && value.channel.is_none()
                    && b.graph.nodes[*index].ty.width == value.ty.width
            }) {
                index
            } else {
                b.push(
                    if returned {
                        "Return"
                    } else if !value.external && value.from.is_none() {
                        "Constant"
                    } else {
                        "Assign"
                    }
                    .into(),
                    vec![Port {
                        name: "In".into(),
                        ..value.clone()
                    }],
                    value.ty,
                    expr.text(),
                    false,
                )
            };
            let converted = expected.width > 0 && expected.width != b.graph.nodes[index].ty.width;
            if converted {
                let value = b.node_value(index);
                let ty = Ty::numeric(expected.width, expected.color || value.ty.color);
                index = b.push(
                    "Convert".into(),
                    vec![Port {
                        name: "In".into(),
                        ..value
                    }],
                    ty,
                    expr.text(),
                    false,
                );
            }
            let node = &mut b.graph.nodes[index];
            node.ty.color |= expected.color;
            node.target = target.clone();
            if !returned {
                let mut ty = node.ty;
                ty.color |= ty.width > 0 && color_name(&target);
                if vector_name(&target) {
                    ty.color = false;
                }
                node.ty = ty;
                b.types.insert(target, ty);
            }
            if b.graph.nodes[index].ty.color
                && b.graph.nodes[index].probe.is_err()
                && !has_custom_call(&b.graph.nodes[index].expression, calls)
            {
                let mut p = probe::expression_candidate(
                    group,
                    step,
                    &b.graph.nodes[index].expression,
                    b.graph.nodes[index].ty.width,
                );
                if group.stage == stages::Stage::Function {
                    p = match &b.helper_vertex {
                        Ok(vertex) => p.map(|mut p| {
                            p.vertex = *vertex;
                            p
                        }),
                        Err(error) => Err(error.clone()),
                    };
                }
                b.graph.nodes[index].probe = p;
            }
            if !b.graph.nodes[index].ty.color {
                b.graph.nodes[index].probe = Err("This node has no color preview".into());
            }
            // Whole-statement probes read the assigned value after execution, so custom
            // calls are not re-evaluated before the original vertex computation.
            if b.graph.nodes[index].ty.color
                && has_custom_call(&b.graph.nodes[index].expression, calls)
            {
                b.graph.nodes[index].probe = group.probes[step].clone();
            }
            if converted {
                // Read after the assignment when HLSL performs implicit broadcasting
                // or truncation; a raw RHS probe would have the wrong shape.
                b.graph.nodes[index].probe = if b.graph.nodes[index].ty.color {
                    group.probes[step].clone()
                } else {
                    Err("This node has no color preview".into())
                };
            }
            Ok(index)
        });
        match result {
            Ok(root) => b.graph.roots.push(root),
            Err(error) => {
                b.graph.nodes.truncate(checkpoint);
                b.graph.limited = true;
                let ports = group.flow.reads[step]
                    .iter()
                    .map(|edge| Port {
                        name: edge.value.clone(),
                        value: edge.value.clone(),
                        from: edge.from.and_then(|s| b.graph.roots.get(s).copied()),
                        channel: None,
                        external: edge.from.is_none(),
                        ty: b.ty(&edge.value),
                    })
                    .collect();
                let index = b.push(
                    "Source Statement".into(),
                    ports,
                    Ty::default(),
                    text.clone(),
                    true,
                );
                b.graph.nodes[index].probe = Err(error);
                b.graph.nodes[index].target = probe::target(text).unwrap_or_default();
                b.graph.roots.push(index);
            }
        }
    }
    b.graph
}
pub(super) fn plan(graph: &Graph, original: &flow::Plan) -> flow::Plan {
    flow::Plan {
        reads: graph
            .nodes
            .iter()
            .map(|n| {
                n.inputs
                    .iter()
                    .filter(|p| p.from.is_some() || p.external)
                    .map(|p| flow::Edge {
                        from: p.from,
                        value: p.value.clone(),
                    })
                    .collect()
            })
            .collect(),
        outputs: original
            .outputs
            .iter()
            .filter_map(|step| graph.roots.get(*step).copied())
            .collect(),
        visual: graph
            .nodes
            .iter()
            .map(|n| n.ty.color && n.probe.is_ok())
            .collect(),
        layout: Vec::new(),
        output: egui::Vec2::ZERO,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declared_vector_shape_survives_scalar_broadcast() {
        let source = "CGPROGRAM\n#pragma fragment f\nfloat4 f(float4 color):SV_Target{float4 localColor=1;float3 normal=0;return localColor;}\nENDCG";
        let graph = stages::parse(source, &[]);
        let nodes = &graph.groups[0].operations.nodes;
        let color = nodes.iter().find(|n| n.target == "localColor").unwrap();
        assert_eq!(color.ty.width, 4);
        assert!(color.ty.color);
        assert!(color.probe.as_ref().unwrap().expression.is_none());
        let normal = nodes.iter().find(|n| n.target == "normal").unwrap();
        assert_eq!(normal.ty.width, 3);
        assert!(!normal.ty.color);
    }
    #[test]
    fn texture_rgb_and_alpha_connect_to_corresponding_ports_without_extra_swizzles() {
        let source = "CGPROGRAM\n#pragma fragment f\nsampler2D _Tex;struct F{float2 uv:TEXCOORD0;};float4 f(F i):SV_Target{float4 col=tex2D(_Tex,i.uv);float3 rgb=col.rgb*col.a;return col;}\nENDCG";
        let graph = stages::parse(source, &[]);
        let nodes = &graph.groups[0].operations.nodes;
        let multiply = nodes.iter().find(|n| n.title == "Multiply").unwrap();
        assert_eq!(multiply.inputs[0].channel.as_deref(), Some("RGB"));
        assert_eq!(multiply.inputs[1].channel.as_deref(), Some("A"));
        assert!(!nodes.iter().any(|n| n.title.starts_with("Swizzle")));
    }
    #[test]
    fn color_components_and_their_operations_keep_previews() {
        let source = "CGPROGRAM\n#pragma fragment f\nfloat4 f(float4 c):SV_Target{float r=c.r;float g=c.g;float b=c.b;float a=c.a;float2 pair=c.rg;float mask=saturate(a*0.5);float2 uv=float2(0,1);return float4(r,g,b,mask);}\nENDCG";
        let graph = stages::parse(source, &[]);
        let nodes = &graph.groups[0].operations.nodes;
        for target in ["r", "g", "b", "a", "pair", "mask"] {
            let node = nodes.iter().find(|n| n.target == target).unwrap();
            assert!(node.ty.color, "{target} lost its color origin");
            let probe = node.probe.as_ref().unwrap();
            let instrumented = super::probe::instrument(source, probe).unwrap();
            assert!(instrumented.contains("float4("));
        }
        for title in ["Multiply", "Saturate"] {
            assert!(nodes.iter().any(|n| n.title == title && n.probe.is_ok()));
        }
        let uv = nodes.iter().find(|n| n.target == "uv").unwrap();
        assert!(!uv.ty.color);
        assert!(uv.probe.is_err());
    }
    #[test]
    fn named_scalar_alpha_can_be_previewed_without_color_input() {
        let source = "CGPROGRAM\n#pragma fragment f\nfloat4 f():SV_Target{float alpha=0.5;return float4(1,1,1,alpha);}\nENDCG";
        let graph = stages::parse(source, &[]);
        let node = graph.groups[0].operations.nodes.iter().find(|n| n.target == "alpha").unwrap();
        assert_eq!(node.ty.width, 1);
        assert!(node.ty.color);
        assert!(node.probe.is_ok());
    }
    #[test]
    fn position_and_normal_targets_keep_vector_semantics() {
        let source = "CGPROGRAM\n#pragma fragment f\nfloat4 f(float4 color):SV_Target{float3 normal=color.rgb*2;return color;}\nENDCG";
        let graph = stages::parse(source, &[]);
        let node = graph.groups[0]
            .operations
            .nodes
            .iter()
            .find(|n| n.target == "normal")
            .unwrap();
        assert_eq!(node.ty.label(), "XYZ");
        assert!(!node.ty.color);
        assert!(node.probe.is_err());
    }
    #[test]
    fn formula_becomes_distinct_typed_nodes_and_keeps_scalar_compact() {
        let source = "CGPROGRAM\n#pragma fragment f\nfloat4 f(float4 color):SV_Target{float amount=0.5;float3 result=color.rgb*amount+color.rgb;return float4(result,color.a);}\nENDCG";
        let graph = stages::parse(source, &[]);
        let g = &graph.groups[0];
        let multiply = g
            .operations
            .nodes
            .iter()
            .find(|n| n.title == "Multiply")
            .unwrap();
        assert_eq!(multiply.inputs[0].name, "A");
        assert_eq!(multiply.inputs[1].name, "B");
        assert_eq!(multiply.ty.label(), "RGB");
        assert!(multiply.probe.is_ok());
        let add = g
            .operations
            .nodes
            .iter()
            .find(|n| n.title == "Add")
            .unwrap();
        assert!(add.inputs[0].from.is_some());
        assert!(add.inputs[1].external);
        let constant = g
            .operations
            .nodes
            .iter()
            .find(|n| n.title == "Constant")
            .unwrap();
        assert_eq!(constant.ty.width, 1);
        assert!(!constant.ty.color);
        assert!(constant.probe.is_err());
        assert!(g.operations.nodes.len() > g.statements.len());
    }
    #[test]
    fn unsupported_branch_is_kept_instead_of_partially_rewritten() {
        let source = "CGPROGRAM\n#pragma fragment f\nfloat4 f(float4 c):SV_Target{c=c.a>0.5?c:0;return c;}\nENDCG";
        let graph = stages::parse(source, &[]);
        assert_eq!(
            graph.groups[0].operations.nodes[0].title,
            "Source Statement"
        );
        assert!(graph.groups[0].operations.nodes[0].fallback);
        assert!(parse("c++").is_err());
        assert!(parse("--c").is_err());
        assert_eq!(parse("c * 1.0 f").unwrap().text(), "(c * 1.0f)");
    }
    #[test]
    fn precedence_and_associativity_are_preserved() {
        assert_eq!(parse("a+b*c").unwrap().text(), "(a + (b * c))");
        assert_eq!(parse("a-b-c").unwrap().text(), "((a - b) - c)");
        assert_eq!(parse("-(a+b)*2").unwrap().text(), "((-(a + b)) * 2)");
        assert!(parse("a ? b : c").is_err());
        assert!(parse("values[0]").is_err());
    }
    #[test]
    fn swizzle_calls_and_scientific_literals_survive() {
        assert_eq!(
            parse("lerp(c.rgb,tex2D(_Tex,uv).rgb,1e-2)").unwrap().text(),
            "lerp((c).rgb, (tex2D(_Tex, uv)).rgb, 1e-2)"
        );
    }
}
