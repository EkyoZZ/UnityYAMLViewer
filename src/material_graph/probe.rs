//! Stage probes preserve shader computation and surface selected intermediate values.
use super::{re, tokens, stages};
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Probe {
    pub function: String,
    pub step: usize,
    pub statement: String,
    pub target: String,
    pub alpha: bool,
    pub vertex: bool,
    pub expression: Option<(String,usize)>,
}
pub(super) fn target(statement: &str) -> Result<String, String> {
    let declaration = re(r"^\s*(?:fixed|half|float|int|uint|bool)[1-4]?\s+(\w+)\s*=");
    if let Some(c) = declaration.captures(statement) { return Ok(c[1].to_owned()); }
    if let Some(c) = re(r"^\s*([A-Za-z_]\w*(?:\s*\.\s*\w+)*)\s*(?:[+*/-]?\s*=)").captures(statement) {
        return Ok(c[1].split_whitespace().collect());
    }
    if let Some(c) = re(r"^\s*return\s+([A-Za-z_]\w*(?:\s*\.\s*\w+)*)\s*;\s*$").captures(statement) {
        return Ok(c[1].split_whitespace().collect());
    }
    Err("Step preview requires an initialized scalar/vector assignment or variable return".into())
}
pub(super) fn candidate(group: &stages::Group, step: usize) -> Result<Probe, String> {
    if group.stage == stages::Stage::Function { return Err("Helper steps require a call context".into()); }
    let statement = group.statements.get(step).ok_or("Step unavailable")?;
    let prefix = group.statements[..=step].join("\n");
    if prefix.contains('{') || prefix.contains('}') || re(r"\b(if|else|for|while|switch|do)\b").is_match(&prefix) {
        return Err("Step preview does not support branching or loop prefixes".into());
    }
    if group.statements[..step].iter().any(|s|re(r"\breturn\b").is_match(s)) {
        return Err("Step follows an earlier return".into());
    }
    if group.stage == stages::Stage::Vert && statement.trim_start().starts_with("return") && width(&group.result_type).is_none() {
        return Err("Select an output struct field instead of the entire vertex result".into());
    }
    Ok(Probe { function: group.name.clone(), step, target: target(statement)?, statement: statement.clone(), alpha: false, vertex: group.stage == stages::Stage::Vert, expression:None })
}
pub(super) fn expression_candidate(group:&stages::Group,step:usize,expression:&str,width:usize)->Result<Probe,String>{
    if !(1..=4).contains(&width){return Err("Expression type unavailable".into());}
    let original=group.statements.get(step).ok_or("Expression step unavailable")?.clone();
    let mut shadow=group.clone();
    if shadow.stage==stages::Stage::Function{shadow.stage=stages::Stage::Frag;}
    shadow.statements[step]="float4 preview_expression = 0;".into();
    let mut p=candidate(&shadow,step)?;p.statement=original;p.target=expression.to_owned();p.expression=Some((expression.into(),width));Ok(p)
}
pub(super) fn width(ty: &str) -> Option<usize> {
    re(r"^(?:fixed|half|float|int|uint|bool)([1-4]?)$").captures(ty)
        .map(|c| if c[1].is_empty() {1} else {c[1].parse().unwrap()})
}
pub(super) fn result_width(source: &str, prefix: &str, parameter_type: &str, parameter: &str, target: &str) -> Result<usize, String> {
    let parts = target.split('.').collect::<Vec<_>>();
    let ty = if parts[0] == parameter { parameter_type.to_owned() } else {
        let declaration = re(&format!(r"\b(\w+)\s+{}\s*(?:=|;)",regex::escape(parts[0])));
        let declarations = declaration.captures_iter(prefix).filter(|c| &c[1] != "return").collect::<Vec<_>>();
        if declarations.len() != 1 { return Err("Step variable type is unavailable or ambiguous".into()); }
        declarations[0][1].to_owned()
    };
    let mut result = if let Some(count) = width(&ty) { (count, 1) } else {
        let pattern = re(&format!(r"(?s)\bstruct\s+{}\s*\{{([^}}]*)\}}",regex::escape(&ty)));
        let fields = pattern.captures(source).ok_or("Step struct unavailable")?;
        let field = parts.get(1).ok_or("Cannot preview an entire struct")?;
        let declaration = re(&format!(r"\b(\w+)\s+{}\s*:",regex::escape(field)));
        let field = declaration.captures(&fields[1]).ok_or("Step field type unavailable")?;
        (width(&field[1]).ok_or("Unsupported field type")?, 2)
    };
    for field in &parts[result.1..] {
        let alphabet = if field.chars().all(|c|"xyzw".contains(c)) {"xyzw"}
            else if field.chars().all(|c|"rgba".contains(c)) {"rgba"} else {return Err("Unsupported step field".into());};
        if field.is_empty() || field.len() > 4 || field.chars().any(|c|alphabet.find(c).unwrap() >= result.0) {
            return Err("Invalid step component selection".into());
        }
        result.0 = field.len();
    }
    Ok(result.0)
}
pub(super) fn instrument(source: &str, probe: &Probe) -> Result<String, String> {
    let source = re(r"(?s)/\*.*?\*/|//[^\r\n]*").replace_all(source, "");
    let stage = if probe.vertex { "vertex" } else { "fragment" };
    let entry_pattern = re(&format!(r"#pragma\s+{stage}\s+(\w+)"));
    let entry = entry_pattern.captures(&source).ok_or("Missing stage entry")?;
    if entry[1] != probe.function { return super::helper::instrument(&source,probe); }
    let signature = re(&format!(r"\b\w+\s+{}\s*\(\s*(\w+)\s+(\w+)\s*\)\s*(?::\s*\w+)?\s*\{{",regex::escape(&probe.function)));
    let matches = signature.captures_iter(&source).collect::<Vec<_>>();
    if matches.len() != 1 { return Err("Fragment entry unavailable or ambiguous".into()); }
    let capture = &matches[0]; let start = capture.get(0).unwrap().end();
    let mut depth = 1; let mut end = None;
    for (offset, ch) in source[start..].char_indices() {
        if ch == '{' {depth += 1;} else if ch == '}' {depth -= 1;if depth == 0 {end = Some(start + offset);break;}}
    }
    let end = end.ok_or("Unterminated fragment body")?;
    let body = &source[start..end];
    let mut parens = 0; let mut spans = Vec::new(); let mut previous = 0;
    for (offset, ch) in body.char_indices() {
        if ch == '(' {parens += 1;} else if ch == ')' {parens -= 1;}
        if ch == ';' && parens == 0 {spans.push((previous,offset+1));previous = offset+1;}
    }
    let &(step_start, step_end) = spans.get(probe.step).ok_or("Step no longer exists")?;
    if tokens(&body[step_start..step_end]) != tokens(&probe.statement) { return Err("Shader source changed; reload the node preview".into()); }
    if re(r"\breturn\b").is_match(&body[..step_start]) { return Err("Step follows an earlier return".into()); }
    let prefix = &body[..step_end];
    if prefix.contains('{') || prefix.contains('}') || re(r"\b(if|else|for|while|switch|do)\b").is_match(prefix) {
        return Err("Step preview requires a straight-line fragment prefix".into());
    }
    let count = if let Some((_,width))=&probe.expression{*width}else{result_width(&source,prefix,&capture[1],&capture[2],&probe.target)?};
    let value = &probe.target;
    let result = if probe.alpha && count == 4 {format!("float4({value}.a,{value}.a,{value}.a,1)")}
        else if probe.alpha && count != 1 {return Err("Alpha view requires a scalar or four-component result".into());}
        else {match count {
            1=>format!("float4({value},{value},{value},1)"),
            2=>format!("float4({value},0,1)"),
            3=>format!("float4({value},1)"),
            4=>format!("float4({value}.rgb,1)"),
            _=>return Err("Unsupported step value width".into()),
        }};
    if probe.vertex { return vertex_instrument(&source, start, end, if probe.expression.is_some(){step_start}else{step_end}, &result); }
    let prefix = if probe.expression.is_some() || body[step_start..step_end].trim_start().starts_with("return") { &body[..step_start] } else { prefix };
    Ok(format!("{}\n{}\nreturn {};\n{}",&source[..start],prefix,result,&source[end..]))
}
fn vertex_instrument(source: &str, start: usize, end: usize, step_end: usize, result: &str) -> Result<String, String> {
    for name in ["preview_step_saved", "preview_step_value"] {
        if re(&format!(r"\b{name}\b")).is_match(source) { return Err("Shader uses a reserved probe identifier".into()); }
    }
    let body = &source[start..end];
    let returns = re(r"\breturn\s+(\w+)\s*;");
    let matches = returns.captures_iter(body).collect::<Vec<_>>();
    if matches.len() != 1 || re(r"\breturn\b").find_iter(body).count() != 1 { return Err("Vertex preview requires one final struct return".into()); }
    let returned = &matches[0];
    if returned.get(0).unwrap().start() < step_end { return Err("Select a vertex assignment before the final return".into()); }
    let return_name = &returned[1];
    let declaration = re(&format!(r"\b(\w+)\s+{}\s*;",regex::escape(return_name)));
    let ty = declaration.captures(body).ok_or("Vertex output struct unavailable")?[1].to_owned();
    let tail = returns.replace(&body[step_end..],format!("{return_name}.preview_step_value=preview_step_saved;return {return_name};"));
    let mut source = format!("{}{}\nfloat4 preview_step_saved={};\n{}{}",&source[..start],&body[..step_end],result,tail,&source[end..]);
    let schema = re(&format!(r"(?s)\bstruct\s+{}\s*\{{([^}}]*)\}}",regex::escape(&ty)));
    let structures = schema.captures_iter(&source).collect::<Vec<_>>();
    if structures.len() != 1 { return Err("Vertex output schema unavailable or ambiguous".into()); }
    let fields = structures[0].get(1).unwrap();
    let semantic = re(r"\bTEXCOORD(\d+)\b");
    let used = semantic.captures_iter(fields.as_str()).filter_map(|c|c[1].parse::<usize>().ok()).collect::<Vec<_>>();
    let channel = (0..16).find(|c|!used.contains(c)).ok_or("No free varying channel for vertex probe")?;
    let insert = fields.end();
    source.insert_str(insert,&format!("float4 preview_step_value:TEXCOORD{channel};\n"));
    let fragment = re(r"#pragma\s+fragment\s+(\w+)").captures(&source).ok_or("Missing fragment entry")?[1].to_owned();
    let signature = re(&format!(r"\b(?:fixed|half|float)4\s+{}\s*\(\s*{}\s+(\w+)\s*\)\s*(?::\s*SV_Target\d*)?\s*\{{",regex::escape(&fragment),regex::escape(&ty)));
    let matches = signature.captures_iter(&source).collect::<Vec<_>>();
    if matches.len()!=1 { return Err("Fragment input does not match vertex output schema".into()); }
    let capture = &matches[0]; let parameter = capture[1].to_owned(); let body_start = capture.get(0).unwrap().end();
    let mut depth = 1; let mut body_end = None;
    for (offset,ch) in source[body_start..].char_indices() {
        if ch=='{' {depth+=1;} else if ch=='}' {depth-=1;if depth==0 {body_end=Some(body_start+offset);break;}}
    }
    let body_end = body_end.ok_or("Unterminated fragment body")?;
    source.replace_range(body_start..body_end,&format!("return {parameter}.preview_step_value;"));
    Ok(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expression_probe_reads_the_rhs_before_compound_assignment(){
        let source="CGPROGRAM\n#pragma fragment f\nfloat4 f(float4 color):SV_Target{color.rgb*=2;return color;}\nENDCG";
        let graph=stages::parse(source,&[]);let node=graph.groups[0].operations.nodes.iter().find(|n|n.title=="Multiply").unwrap();let p=node.probe.as_ref().unwrap();let instrumented=instrument(source,p).unwrap();
        let body=instrumented.split("SV_Target{").nth(1).unwrap();assert!(!body.contains("color.rgb*=2"));assert!(body.contains("* 2"));assert!(!body.contains("preview_step_saved"));
    }
    const SOURCE: &str = "Shader \"x\" { SubShader { CGPROGRAM\n#pragma fragment frag\nstruct V {float2 uv:TEXCOORD0;}; fixed4 frag(V i):SV_Target { fixed4 c = tex2D(_MainTex,i.uv); c.a *= 0.5; c.rgb *= c.a; return c; } ENDCG } }";
    const VERTEX_SOURCE: &str = r#"Shader "vertex-step" {SubShader {Cull Off ZWrite Off Blend One OneMinusSrcAlpha Pass {CGPROGRAM
#pragma vertex vert
#pragma fragment frag
#include "UnityCG.cginc"
struct V {float4 vertex:POSITION;float2 uv:TEXCOORD0;};
struct F {float4 vertex:SV_POSITION;float2 uv:TEXCOORD0;};
sampler2D _MainTex;
F vert(V v){F o;o.uv=v.uv;o.uv*=2;o.vertex=UnityObjectToClipPos(v.vertex);return o;}
fixed4 frag(F i):SV_Target{return tex2D(_MainTex,i.uv);}
ENDCG }}}"#;
    #[test]
    fn vertex_expression_probe_preserves_original_assignments_and_position(){
        let graph=stages::parse(VERTEX_SOURCE,&[]);let group=graph.groups.iter().find(|g|g.name=="vert").unwrap();let p=expression_candidate(group,1,"float3(v.uv,0)",3).unwrap();let output=instrument(VERTEX_SOURCE,&p).unwrap();assert!(output.contains("o.uv=v.uv"));assert!(output.contains("o.vertex=UnityObjectToClipPos(v.vertex)"));assert!(output.contains("preview_step_saved=float4(float3(v.uv,0),1)"));assert!(output.contains("preview_step_value"));
    }
    #[test]
    fn vertex_probe_captures_selected_value_and_keeps_later_geometry() {
        let probe = Probe { function:"vert".into(),step:1,statement:"o.uv=v.uv;".into(),target:"o.uv".into(),alpha:false,vertex:true,expression:None };
        let source = instrument(VERTEX_SOURCE,&probe).unwrap();
        assert!(source.find("preview_step_saved=float4(o.uv,0,1)").unwrap()<source.find("o.uv*=2").unwrap());
        assert!(source.contains("o.vertex=UnityObjectToClipPos(v.vertex)"));
        assert!(source.contains("o.preview_step_value=preview_step_saved;return o;"));
        assert!(source.contains("preview_step_value:TEXCOORD1"));
        let (program,_) = crate::render_system::shader::translate_preview_source(&source, "").unwrap();
        assert!(program.vertex.contains("gl_Position=o.vertex"));
        assert!(program.vertex.contains("varying_preview_step_value=o.preview_step_value"));
        assert!(program.fragment.contains("i.preview_step_value=varying_preview_step_value"));
        assert!(!program.fragment.contains("texture(_MainTex"));
        assert!(instrument(VERTEX_SOURCE,&Probe { alpha:true,..probe }).is_err());
    }
    #[test]
    fn vertex_probe_rejects_early_returns_or_exhausted_varyings() {
        let probe = Probe { function:"vert".into(),step:1,statement:"o.uv=v.uv;".into(),target:"o.uv".into(),alpha:false,vertex:true,expression:None };
        assert!(instrument(&VERTEX_SOURCE.replace("o.uv*=2;","if(v.uv.x>0)return o;o.uv*=2;"),&probe).is_err());
        let fields = (1..16).map(|i|format!("float2 extra{i}:TEXCOORD{i};")).collect::<String>();
        assert!(instrument(&VERTEX_SOURCE.replace("float2 uv:TEXCOORD0;};\nsampler",&format!("float2 uv:TEXCOORD0;{fields}}};\nsampler")),&probe).is_err());
    }
    #[test]
    fn normalized_compound_assignments_keep_preview_targets() {
        assert_eq!(target("disPos . y + = _Time . y * 0.1 ;").unwrap(),"disPos.y");
        assert_eq!(target("i . uv . y - = dis * pixel ;").unwrap(),"i.uv.y");
    }
    #[test]
    fn fragment_probe_keeps_prefix_and_omits_later_operations() {
        let probe = Probe { function:"frag".into(),step:1,statement:"c . a *= 0.5 ;".into(),target:"c.a".into(),alpha:false,vertex:false,expression:None };
        let source = instrument(SOURCE,&probe).unwrap();
        assert!(source.contains("c.a *= 0.5;")); assert!(!source.contains("c.rgb *= c.a;"));
        assert!(source.contains("return float4(c.a,c.a,c.a,1);"));
        let stale = Probe { statement:"c.a *= 0.1;".into(), ..probe }; assert!(instrument(SOURCE,&stale).is_err());
    }
    #[test]
    fn instrumented_shader_translates_with_original_uv_and_helper_computation() {
        let source = r#"Shader "step" { SubShader { Cull Off ZWrite Off Blend One OneMinusSrcAlpha Pass { CGPROGRAM
#pragma vertex vert
#pragma fragment frag
#include "UnityCG.cginc"
struct V {float4 vertex:POSITION;float2 uv:TEXCOORD0;};
struct F {float4 vertex:SV_POSITION;float2 uv:TEXCOORD0;};
sampler2D _MainTex;
F vert(V v){F o;o.vertex=UnityObjectToClipPos(v.vertex);o.uv=v.uv;return o;}
float change(float x){return x*0.5;}
fixed4 frag(F i):SV_Target {float2 p=i.uv;p.y+=_Time.y*0.1;fixed4 c=tex2D(_MainTex,p);c.a=change(c.a);c.rgb*=c.a;return c;}
ENDCG } } }"#;
        let probe = Probe { function:"frag".into(),step:3,statement:"c.a=change(c.a);".into(),target:"c.a".into(),alpha:false,vertex:false,expression:None };
        let source = instrument(source,&probe).unwrap();
        let (program, state) = crate::render_system::shader::translate_preview_source(&source, "").unwrap();
        assert!(program.fragment.contains("texture(_MainTex,p)"));
        assert!(program.fragment.contains("c.a=change(c.a)"));
        assert!(!program.fragment.contains("c.rgb*=c.a"));
        assert_eq!(state.source,crate::render_system::scene::BlendFactor::One);
        assert_eq!(state.destination,crate::render_system::scene::BlendFactor::OneMinusSourceAlpha);
    }
    #[test]
    fn alpha_and_uv_types_are_explicit_and_branching_is_rejected() {
        let probe = Probe { function:"frag".into(),step:0,statement:"fixed4 c = tex2D(_MainTex,i.uv);".into(),target:"c".into(),alpha:true,vertex:false,expression:None };
        assert!(instrument(SOURCE,&probe).unwrap().contains("return float4(c.a,c.a,c.a,1);"));
        assert_eq!(result_width(SOURCE,"float2 p = i.uv;","V","i","i.uv.y").unwrap(),1);
        assert!(instrument(&SOURCE.replace("fixed4 c =", "if(i.uv.x>0){} fixed4 c ="), &probe).is_err());
    }
}
