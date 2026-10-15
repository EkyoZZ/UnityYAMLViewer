//! Source-level function groups, deliberately separate from GPU execution.
use super::{Input, annotate_inputs, block_end, re, tokens};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Stage {
    Vert,
    Frag,
    Function,
}

#[derive(Clone, Debug)]
pub(super) struct Group {
    pub name: String,
    pub stage: Stage,
    pub pass: Option<usize>,
    pub signature: String,
    pub body: String,
    pub statements: Vec<String>,
    pub inputs: Vec<(usize, String, String)>,
    pub calls: Vec<usize>,
    pub result_type: String,
    pub argument_types: Vec<String>,
    parameters: Vec<String>,
    call_names: Vec<String>,
}

#[derive(Debug)]
pub(super) struct Interface {
    pub name: String,
    pub fields: Vec<(String, String, String)>,
    pub from: Option<usize>,
    pub to: usize,
}

#[derive(Default, Debug)]
pub(super) struct Graph {
    pub groups: Vec<Group>,
    pub varyings: Vec<(usize, usize)>,
    pub limited: bool,
    pub unresolved: bool,
    pub interfaces: Vec<Interface>,
}

fn identifier(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn statements(body: &[String]) -> (Vec<String>, bool) {
    let mut result = Vec::new();
    let mut start = 0;
    let mut parens = 0;
    for (i, token) in body.iter().enumerate() {
        if token == "(" {
            parens += 1;
        } else if token == ")" {
            parens -= 1;
        }
        if token == ";" && parens == 0 {
            result.push(body[start..=i].join(" "));
            start = i + 1;
            if result.len() == 32 {
                return (result, start < body.len());
            }
        }
    }
    if start < body.len() {
        result.push(body[start..].join(" "));
    }
    (result, false)
}

fn functions(code: &str, pass: Option<usize>, vert: &str, frag: &str, graph: &mut Graph) {
    let t = tokens(code);
    let mut i = 2;
    while i < t.len() {
        if t[i] != "("
            || !identifier(&t[i - 1])
            || !identifier(&t[i - 2])
            || ["if", "for", "while", "switch"].contains(&t[i - 1].as_str())
        {
            i += 1;
            continue;
        }
        let Some(args_end) = block_end(&t, i, "(", ")") else {
            break;
        };
        let mut open = args_end + 1;
        if t.get(open).is_some_and(|s| s == ":") {
            open += 2;
        }
        if t.get(open).is_none_or(|s| s != "{") {
            i = args_end + 1;
            continue;
        }
        let Some(end) = block_end(&t, open, "{", "}") else {
            graph.limited = true;
            break;
        };
        if graph.groups.len() >= 64 {
            graph.limited = true;
            return;
        }
        let name = t[i - 1].clone();
        let argument_types = t[i + 1..args_end]
            .split(|s| s == ",")
            .filter_map(|arg| {
                arg.iter()
                    .find(|s| {
                        identifier(s)
                            && !["in", "out", "inout", "const", "uniform"].contains(&s.as_str())
                    })
                    .cloned()
            })
            .collect();
        let parameters = t[i + 1..args_end]
            .split(|s| s == ",")
            .filter_map(|arg| {
                let semantic = arg.iter().position(|s| s == ":").unwrap_or(arg.len());
                arg[..semantic]
                    .iter()
                    .rev()
                    .find(|s| identifier(s))
                    .cloned()
            })
            .collect();
        let body_tokens = &t[open + 1..end];
        let call_names = body_tokens
            .windows(2)
            .filter(|w| identifier(&w[0]) && w[1] == "(")
            .map(|w| w[0].clone())
            .collect();
        let (statements, limited) = statements(body_tokens);
        graph.limited |= limited;
        graph.groups.push(Group {
            name: name.clone(),
            stage: if name == vert {
                Stage::Vert
            } else if name == frag {
                Stage::Frag
            } else {
                Stage::Function
            },
            pass,
            signature: t[i - 2..open].join(" "),
            body: body_tokens.join(" "),
            statements,
            inputs: Vec::new(),
            calls: Vec::new(),
            result_type: t[i - 2].clone(),
            argument_types,
            parameters,
            call_names,
        });
        i = end + 1;
    }
}

fn schema_fields(
    schemas: &[(Option<usize>, String, Vec<(String, String, String)>)],
    pass: Option<usize>,
    name: &str,
) -> Option<Vec<(String, String, String)>> {
    let local = schemas
        .iter()
        .filter(|(owner, n, _)| *owner == pass && n == name)
        .collect::<Vec<_>>();
    if local.len() == 1 {
        return Some(local[0].2.clone());
    }
    if !local.is_empty() {
        return None;
    }
    let shared = schemas
        .iter()
        .filter(|(owner, n, _)| owner.is_none() && n == name)
        .collect::<Vec<_>>();
    if shared.len() == 1 {
        Some(shared[0].2.clone())
    } else {
        None
    }
}

fn group_height(group: &Group) -> f32 {
    110. + group.inputs.len() as f32 * 24. + group.statements.len().max(1) as f32 * 52.
}

/// Draw source groups, not an executable replacement graph. All edges are
/// either direct source references, declared stage interfaces or known calls.
pub(super) fn draw(
    ui: &mut eframe::egui::Ui,
    graph: &Graph,
    inputs: &[Input],
    input_rects: &[eframe::egui::Rect],
    origin: eframe::egui::Pos2,
    canvas: eframe::egui::Rect,
    zoom: f32,
    positions: &mut std::collections::HashMap<usize, eframe::egui::Vec2>,
    output: eframe::egui::Rect,
) {
    use super::{input_color, output_y};
    use eframe::egui::{self, Color32, Rect, Sense, Stroke};
    let painter = ui.painter().with_clip_rect(canvas);
    let color = |stage| match stage {
        Stage::Vert => Color32::from_rgb(95, 175, 225),
        Stage::Frag => Color32::from_rgb(185, 135, 220),
        Stage::Function => Color32::from_rgb(205, 175, 95),
    };
    let rect = |position: egui::Vec2, size: egui::Vec2| {
        Rect::from_min_size(origin + position * zoom, size * zoom)
    };
    let passes = graph
        .groups
        .iter()
        .filter_map(|g| g.pass)
        .max()
        .map_or(1, |p| p + 1);
    let mut rows = Vec::new();
    let mut bottom = 0.;
    for pass in 0..passes {
        rows.push(bottom);
        let stage_height = graph
            .groups
            .iter()
            .filter(|g| g.pass == Some(pass) && g.stage != Stage::Function)
            .map(group_height)
            .fold(0., f32::max);
        let interface_height = graph
            .interfaces
            .iter()
            .filter(|v| v.from.is_none() && graph.groups[v.to].pass == Some(pass))
            .map(|v| 90. + v.fields.len() as f32 * 28.)
            .fold(0., f32::max);
        bottom += stage_height + interface_height + 120.;
    }
    let mut helper = 0;
    let mut helper_row = bottom;
    let mut helper_height = 0.;
    let groups = graph
        .groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            let default = match group.stage {
                Stage::Vert => egui::vec2(440., rows[group.pass.unwrap_or(0)]),
                Stage::Frag => egui::vec2(1360., rows[group.pass.unwrap_or(0)]),
                Stage::Function => {
                    if helper > 0 && helper % 2 == 0 {
                        helper_row += helper_height + 60.;
                        helper_height = 0.;
                    }
                    let point = egui::vec2(940. + (helper % 2) as f32 * 520., helper_row);
                    helper_height = helper_height.max(group_height(group));
                    helper += 1;
                    point
                }
            };
            rect(
                *positions.entry(1000 + index).or_insert(default),
                egui::vec2(420., group_height(group)),
            )
        })
        .collect::<Vec<_>>();
    let interfaces = graph
        .interfaces
        .iter()
        .enumerate()
        .map(|(index, interface)| {
            let position = if let Some(from) = interface.from {
                egui::vec2(940., rows[graph.groups[from].pass.unwrap_or(0)])
            } else {
                let parent = groups[interface.to];
                egui::vec2(
                    (parent.left() - origin.x) / zoom,
                    (parent.bottom() - origin.y) / zoom + 40.,
                )
            };
            rect(
                *positions.entry(2000 + index).or_insert(position),
                egui::vec2(
                    if interface.from.is_some() { 340. } else { 420. },
                    90. + interface.fields.len() as f32 * 28.,
                ),
            )
        })
        .collect::<Vec<_>>();
    let fill = if ui.visuals().dark_mode {
        Color32::from_rgb(30, 32, 36)
    } else {
        Color32::from_rgb(235, 238, 244)
    };
    let card_fill = if ui.visuals().dark_mode {
        Color32::from_rgb(43, 45, 50)
    } else {
        Color32::from_rgb(250, 250, 252)
    };
    for (index, group) in graph.groups.iter().enumerate() {
        painter.rect_filled(groups[index], 7., fill);
        painter.rect_stroke(
            groups[index],
            7.,
            Stroke::new(1.0_f32, color(group.stage).gamma_multiply(0.55)),
            egui::StrokeKind::Inside,
        );
    }
    for (index, _) in graph.interfaces.iter().enumerate() {
        painter.rect_filled(interfaces[index], 7., fill);
        painter.rect_stroke(
            interfaces[index],
            7.,
            Stroke::new(1.0_f32, Color32::from_rgb(75, 120, 115)),
            egui::StrokeKind::Inside,
        );
    }
    let line = |from: egui::Pos2, to: egui::Pos2, tint: Color32, label: &str| {
        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
            super::wire_points(from, to, zoom),
            false,
            Color32::TRANSPARENT,
            Stroke::new(1.2_f32, tint),
        ));
        if !label.is_empty() {
            painter.text(
                from.lerp(to, 0.5),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(11. * zoom),
                tint,
            );
        }
    };
    let property_port = |group: usize, row: usize| {
        egui::pos2(
            groups[group].left(),
            groups[group].top() + (78. + row as f32 * 24.) * zoom,
        )
    };
    for (index, group) in graph.groups.iter().enumerate() {
        for (row, (input, channel, _)) in group.inputs.iter().enumerate() {
            line(
                egui::pos2(
                    input_rects[*input].right(),
                    input_rects[*input].top() + output_y(&inputs[*input], channel) * zoom,
                ),
                property_port(index, row),
                input_color(&inputs[*input].value),
                "",
            );
        }
        for &called in &group.calls {
            line(
                egui::pos2(groups[index].right(), groups[index].top() + 30. * zoom),
                egui::pos2(groups[called].left(), groups[called].top() + 30. * zoom),
                color(Stage::Function),
                super::tr("函数调用"),
            );
        }
    }
    for (index, interface) in graph.interfaces.iter().enumerate() {
        for (field, _) in interface.fields.iter().enumerate() {
            let left = egui::pos2(
                interfaces[index].left(),
                interfaces[index].top() + (66. + field as f32 * 28.) * zoom,
            );
            let right = egui::pos2(interfaces[index].right(), left.y);
            if let Some(from) = interface.from {
                line(
                    groups[from].right_center(),
                    left,
                    Color32::from_rgb(100, 190, 175),
                    "",
                );
            }
            line(
                right,
                egui::pos2(
                    groups[interface.to].left(),
                    groups[interface.to].top() + 52. * zoom,
                ),
                Color32::from_rgb(100, 190, 175),
                "",
            );
        }
    }
    for &(from, to) in &graph.varyings {
        if !graph
            .interfaces
            .iter()
            .any(|i| i.from == Some(from) && i.to == to)
        {
            line(
                groups[from].right_center(),
                groups[to].left_center(),
                Color32::from_rgb(100, 190, 175),
                "Varyings",
            );
        }
    }
    let fragments = graph
        .groups
        .iter()
        .enumerate()
        .filter(|(_, g)| g.stage == Stage::Frag)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if fragments.len() == 1 {
        line(
            egui::pos2(groups[fragments[0]].right(), groups[fragments[0]].top() + 64. * zoom),
            egui::pos2(output.left(), output.top() + 55. * zoom),
            color(Stage::Frag),
            "Output",
        );
    }
    for (index, group) in graph.groups.iter().enumerate() {
        let r = groups[index];
        if !r.intersects(canvas) {
            continue;
        }
        let title = match group.stage {
            Stage::Vert => "Vert",
            Stage::Frag => "Frag",
            Stage::Function => "Function",
        };
        if group.stage == Stage::Frag && fragments.len() == 1 {
            let port = egui::pos2(r.right(), r.top() + 64. * zoom);
            painter.circle_filled(port, 4. * zoom, color(Stage::Frag));
            painter.with_clip_rect(r.intersect(canvas)).text(
                port - egui::vec2(10. * zoom, 0.), egui::Align2::RIGHT_CENTER,
                "Output", egui::FontId::proportional(11. * zoom), color(Stage::Frag),
            );
        }
        let header = Rect::from_min_size(r.min, egui::vec2(r.width(), 34. * zoom));
        painter.rect_filled(
            header,
            6.,
            color(group.stage).gamma_multiply(if ui.visuals().dark_mode { 0.3 } else { 0.85 }),
        );
        painter.with_clip_rect(header.intersect(canvas)).text(
            header.left_center() + egui::vec2(12. * zoom, 0.),
            egui::Align2::LEFT_CENTER,
            format!(
                "{title} · {}{}",
                group.name,
                group
                    .pass
                    .map_or(String::new(), |p| format!(" · Pass {}", p + 1))
            ),
            egui::FontId::proportional(15. * zoom),
            ui.visuals().text_color(),
        );
        let drag = ui
            .interact(
                header.intersect(canvas),
                ui.id().with(("function_group", index)),
                Sense::click_and_drag(),
            )
            .on_hover_text(&group.signature);
        if drag.dragged() {
            *positions.get_mut(&(1000 + index)).unwrap() += drag.drag_delta() / zoom;
        }
        painter.with_clip_rect(r.intersect(canvas)).text(
            r.min + egui::vec2(12., 42.) * zoom,
            egui::Align2::LEFT_TOP,
            &group.signature,
            egui::FontId::monospace(11. * zoom),
            ui.visuals().weak_text_color(),
        );
        for (row, (input, _, target)) in group.inputs.iter().enumerate() {
            let port = property_port(index, row);
            painter.circle_filled(port, 4. * zoom, input_color(&inputs[*input].value));
            painter.text(
                port + egui::vec2(10. * zoom, 0.),
                egui::Align2::LEFT_CENTER,
                format!("{} · {target}", inputs[*input].name),
                egui::FontId::proportional(11. * zoom),
                ui.visuals().text_color(),
            );
        }
        let top = 102. + group.inputs.len() as f32 * 24.;
        for (statement, source) in group.statements.iter().enumerate() {
            let card = Rect::from_min_size(
                r.min + egui::vec2(12., top + statement as f32 * 52.) * zoom,
                egui::vec2(396., 44.) * zoom,
            );
            if !card.intersects(canvas) {
                continue;
            }
            painter.rect_filled(card, 3., card_fill);
            let text = source.chars().take(512).collect::<String>();
            let galley = painter.layout(
                text,
                egui::FontId::monospace(11. * zoom),
                ui.visuals().text_color(),
                card.width() - 12. * zoom,
            );
            painter.with_clip_rect(card.intersect(canvas)).galley(
                card.min + egui::vec2(6., 5.) * zoom,
                galley,
                ui.visuals().text_color(),
            );
            ui.interact(
                card.intersect(canvas),
                ui.id().with(("source_statement", index, statement)),
                Sense::hover(),
            )
            .on_hover_text(source.chars().take(4096).collect::<String>());
        }
    }
    for (index, interface) in graph.interfaces.iter().enumerate() {
        let r = interfaces[index];
        if !r.intersects(canvas) {
            continue;
        }
        let header = Rect::from_min_size(r.min, egui::vec2(r.width(), 34. * zoom));
        let kind = if interface.from.is_some() {
            "Varyings"
        } else {
            "Vertex Input"
        };
        painter.with_clip_rect(header.intersect(canvas)).text(
            header.left_center() + egui::vec2(12. * zoom, 0.),
            egui::Align2::LEFT_CENTER,
            format!("{kind} · {}", interface.name),
            egui::FontId::proportional(14. * zoom),
            ui.visuals().text_color(),
        );
        let drag = ui.interact(
            header.intersect(canvas),
            ui.id().with(("interface_group", index)),
            Sense::drag(),
        );
        if drag.dragged() {
            *positions.get_mut(&(2000 + index)).unwrap() += drag.drag_delta() / zoom;
        }
        for (field, (ty, name, semantic)) in interface.fields.iter().enumerate() {
            let point = egui::pos2(r.left(), r.top() + (66. + field as f32 * 28.) * zoom);
            painter.circle_filled(point, 4. * zoom, Color32::from_rgb(100, 190, 175));
            painter.circle_filled(
                egui::pos2(r.right(), point.y),
                4. * zoom,
                Color32::from_rgb(100, 190, 175),
            );
            painter.with_clip_rect(r.intersect(canvas)).text(
                point + egui::vec2(10. * zoom, 0.),
                egui::Align2::LEFT_CENTER,
                format!("{name} · {ty} · {semantic}"),
                egui::FontId::monospace(11. * zoom),
                ui.visuals().text_color(),
            );
        }
    }
}

pub(super) fn parse(source: &str, inputs: &[Input]) -> Graph {
    let mut graph = Graph::default();
    // Strip comments without touching quoted strings containing // or braces.
    let source = re(r#"(?s)/\*.*?\*/|//[^\r\n]*|"(?:\\.|[^"\\])*""#).replace_all(
        source,
        |c: &regex::Captures| {
            if c[0].starts_with('"') {
                c[0].to_owned()
            } else {
                " ".into()
            }
        },
    );
    let blocks =
        re(r"(?s)\b(CGPROGRAM|HLSLPROGRAM|CGINCLUDE|HLSLINCLUDE)\b(.*?)\b(?:ENDCG|ENDHLSL)\b");
    let mut pass = 0;
    let mut entries: Vec<(usize, String, String, bool)> = Vec::new();
    let mut schemas: Vec<(Option<usize>, String, Vec<(String, String, String)>)> = Vec::new();
    for c in blocks.captures_iter(&source) {
        let owner = if c[1].ends_with("INCLUDE") {
            None
        } else {
            Some(pass)
        };
        for structure in re(r"(?s)\bstruct\s+(\w+)\s*\{([^{}]*)\}").captures_iter(&c[2]) {
            let mut fields: Vec<_> = re(r"\b(\w+)\s+(\w+)\s*:\s*(\w+)\s*;")
                .captures_iter(&structure[2])
                .map(|f| (f[1].to_owned(), f[2].to_owned(), f[3].to_owned()))
                .collect();
            if fields.len() > 64 {
                fields.truncate(64);
                graph.limited = true;
            }
            schemas.push((owner, structure[1].to_owned(), fields));
        }
        if c[1].ends_with("INCLUDE") {
            functions(&c[2], None, "", "", &mut graph);
            continue;
        }
        if pass >= 16 {
            graph.limited = true;
            break;
        }
        let vertex = re(r"(?m)^\s*#pragma\s+vertex\s+(\w+)")
            .captures(&c[2])
            .map(|c| c[1].to_owned())
            .unwrap_or_default();
        let fragment = re(r"(?m)^\s*#pragma\s+fragment\s+(\w+)")
            .captures(&c[2])
            .map(|c| c[1].to_owned())
            .unwrap_or_default();
        functions(&c[2], Some(pass), &vertex, &fragment, &mut graph);
        entries.push((
            pass,
            vertex,
            fragment,
            !re(r"#pragma\s+(?:geometry|hull|domain)\b").is_match(&c[2]),
        ));
        pass += 1;
    }
    // Entry functions may be defined in CGINCLUDE/HLSLINCLUDE. Associate a
    // shared definition with each unambiguous pass entry before building edges.
    for (pass, vertex, fragment, _) in &entries {
        for (name, stage) in [(vertex, Stage::Vert), (fragment, Stage::Frag)] {
            if name.is_empty()
                || graph
                    .groups
                    .iter()
                    .any(|g| g.pass == Some(*pass) && g.name == *name)
            {
                continue;
            }
            let shared = graph
                .groups
                .iter()
                .filter(|g| g.pass.is_none() && g.name == *name)
                .collect::<Vec<_>>();
            if shared.len() == 1 && graph.groups.len() < 64 {
                let mut group = shared[0].clone();
                group.stage = stage;
                group.pass = Some(*pass);
                graph.groups.push(group);
            } else {
                graph.unresolved = true;
                if graph.groups.len() >= 64 {
                    graph.limited = true;
                }
            }
        }
    }
    graph.unresolved |= entries.is_empty()
        || entries.iter().any(|(pass, vertex, fragment, _)| {
            vertex.is_empty()
                || fragment.is_empty()
                || graph
                    .groups
                    .iter()
                    .filter(|g| g.pass == Some(*pass) && g.stage == Stage::Vert)
                    .count()
                    != 1
                || graph
                    .groups
                    .iter()
                    .filter(|g| g.pass == Some(*pass) && g.stage == Stage::Frag)
                    .count()
                    != 1
        });
    let stage_names = entries
        .iter()
        .flat_map(|(_, v, f, _)| [v, f])
        .filter(|n| !n.is_empty())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let called_names = graph
        .groups
        .iter()
        .flat_map(|g| g.call_names.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>();
    graph.groups.retain(|g| {
        g.pass.is_some() || !stage_names.contains(&g.name) || called_names.contains(&g.name)
    });
    for (pass, _, _, direct) in &entries {
        if !direct {
            continue;
        }
        let verts = graph
            .groups
            .iter()
            .enumerate()
            .filter(|(_, g)| g.pass == Some(*pass) && g.stage == Stage::Vert)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        let frags = graph
            .groups
            .iter()
            .enumerate()
            .filter(|(_, g)| g.pass == Some(*pass) && g.stage == Stage::Frag)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if verts.len() == 1 && frags.len() == 1 {
            graph.varyings.push((verts[0], frags[0]));
        }
    }
    for (index, group) in graph.groups.iter().enumerate() {
        if group.stage != Stage::Vert {
            continue;
        }
        if let Some(name) = group.argument_types.first() {
            if let Some(fields) = schema_fields(&schemas, group.pass, name) {
                graph.interfaces.push(Interface {
                    name: name.clone(),
                    fields,
                    from: None,
                    to: index,
                });
            }
        }
    }
    for &(from, to) in &graph.varyings {
        let producer = &graph.groups[from];
        let consumer = &graph.groups[to];
        if consumer.argument_types.contains(&producer.result_type) {
            if let Some(fields) = schema_fields(&schemas, producer.pass, &producer.result_type) {
                graph.interfaces.push(Interface {
                    name: producer.result_type.clone(),
                    fields,
                    from: Some(from),
                    to,
                });
            }
        }
    }
    for i in 0..graph.groups.len() {
        let calls = graph.groups[i]
            .call_names
            .iter()
            .filter_map(|name| {
                let local = graph
                    .groups
                    .iter()
                    .enumerate()
                    .filter(|(_, g)| g.name == *name && g.pass == graph.groups[i].pass)
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();
                if local.len() == 1 {
                    return Some(local[0]);
                }
                if !local.is_empty() {
                    return None;
                }
                let shared = graph
                    .groups
                    .iter()
                    .enumerate()
                    .filter(|(_, g)| g.name == *name && g.pass.is_none())
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();
                if shared.len() == 1 {
                    Some(shared[0])
                } else {
                    None
                }
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        graph.groups[i].calls = calls;
        for (index, input) in inputs.iter().enumerate() {
            let group = &graph.groups[i];
            if group.parameters.contains(&input.name)
                || !re(&format!(r"\b{}\b", regex::escape(&input.name))).is_match(&group.body)
            {
                continue;
            }
            let mut used = input.clone();
            used.bindings.clear();
            // Synthetic declaration gives the existing binding annotator the
            // same declaration/use distinction as a complete shader source.
            let declaration = match input.value {
                super::Value::Float(..) => "float",
                super::Value::Vector(..) => "float4",
                super::Value::Texture { .. } => "sampler2D",
            };
            annotate_inputs(
                &format!("{} {}; {}", declaration, input.name, group.body),
                std::slice::from_mut(&mut used),
            );
            for (output, target) in used.bindings {
                graph.groups[i].inputs.push((index, output, target));
            }
        }
    }
    graph
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn varying_channels_are_separate_from_mesh_uv_channels() {
        let source = "CGINCLUDE\nstruct appdata { float4 vertex:POSITION; float2 uv:TEXCOORD0; float2 uv3:TEXCOORD3; }; struct v2f { float4 vertex:SV_POSITION; float4 color:COLOR; float2 custom:TEXCOORD2; UNITY_VERTEX_OUTPUT_STEREO }; v2f V(appdata v){v2f o; o.custom=v.uv; return o;} float4 F(v2f i):SV_Target {return float4(i.custom,0,1);}\nENDCG\nCGPROGRAM\n#pragma vertex V\n#pragma fragment F\nENDCG";
        let graph = parse(source, &[]);
        assert_eq!(graph.groups.len(), 2);
        assert_eq!(graph.groups[0].stage, Stage::Vert);
        assert_eq!(graph.groups[1].stage, Stage::Frag);
        let input = graph.interfaces.iter().find(|i| i.from.is_none()).unwrap();
        assert_eq!(input.name, "appdata");
        assert!(
            input
                .fields
                .iter()
                .any(|(_, n, s)| n == "uv3" && s == "TEXCOORD3")
        );
        let varying = graph.interfaces.iter().find(|i| i.from.is_some()).unwrap();
        assert_eq!(varying.name, "v2f");
        assert!(
            varying
                .fields
                .iter()
                .any(|(_, n, s)| n == "custom" && s == "TEXCOORD2")
        );
        assert!(varying.fields.iter().all(|(_, n, _)| n != "uv3"));
    }
    #[test]
    fn entries_and_helpers_keep_separate_groups_and_calls() {
        let source = "CGPROGRAM\n#pragma vertex VertexMain\n#pragma fragment PixelMain\nfloat4 Helper(float4 x){return x*2;}\nfloat4 VertexMain(float4 p){return Helper(p);}\nfloat4 PixelMain(float4 p):SV_Target {for(int i=0;i<2;i++){p*=2;}return Helper(p);}\nENDCG";
        let graph = parse(source, &[]);
        assert_eq!(graph.groups.len(), 3);
        assert_eq!(graph.groups[0].stage, Stage::Function);
        assert_eq!(graph.groups[1].stage, Stage::Vert);
        assert_eq!(graph.groups[2].stage, Stage::Frag);
        assert_eq!(graph.groups[1].calls, vec![0]);
        assert_eq!(graph.groups[2].calls, vec![0]);
        assert_eq!(graph.varyings, vec![(1, 2)]);
        assert!(graph.groups[2].statements[0].contains("i < 2"));
    }
    #[test]
    fn overloads_and_comments_do_not_invent_call_edges() {
        let source = "CGPROGRAM\n#pragma vertex v\n#pragma fragment f\n// float Fake(){return 1;}\nfloat H(float x){return x;} float H(float2 x){return x.x;}\nfloat v(float x){return H(x);} float f(float x){return x;}\nENDCG";
        let graph = parse(source, &[]);
        assert_eq!(graph.groups.len(), 4);
        assert!(graph.groups[2].calls.is_empty());
    }
    #[test]
    fn global_inputs_are_bound_to_the_using_function_not_shadowing_parameters() {
        let inputs = super::super::properties("Properties { _Amount(\"A\",Float)=1 }").unwrap();
        let graph = parse(
            "CGPROGRAM\n#pragma vertex v\n#pragma fragment f\nfloat v(float _Amount){return _Amount;} float f(float p){return p*_Amount;}\nENDCG",
            &inputs,
        );
        assert!(graph.groups[0].inputs.is_empty());
        assert_eq!(graph.groups[1].inputs.len(), 1);
    }
}
