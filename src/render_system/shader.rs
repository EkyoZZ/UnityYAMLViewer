//! Strict Built-in sprite CG subset translation. Expressions are preserved;
//! unsupported syntax is rejected rather than replaced with guessed effects.
#![allow(dead_code)]
use std::sync::Arc;
use regex::Regex;
use super::scene::{BlendFactor, Program, RenderState, UnityPipeline};

fn re(pattern: &str) -> Regex { Regex::new(pattern).unwrap() }
pub(crate) fn scalar_is_referenced(code:&str,name:&str)->bool {
    re(&format!(r"\b{}\b",regex::escape(name))).find_iter(code).take(2).count()>1
}

pub(crate) fn translate_file(path:&std::path::Path)->Result<(Program,RenderState),String> {
    translate_material_file(path, "")
}
pub(crate) fn translate_material_file(path:&std::path::Path,material:&str)->Result<(Program,RenderState),String> {
    fn expand(path:&std::path::Path,root:&std::path::Path,depth:usize,budget:&mut usize)->Result<String,String> {
        if depth>8 {return Err("Shader include nesting exceeds 8".into());}
        let length=std::fs::metadata(path).map_err(|e|format!("{}: {e}",path.display()))?.len() as usize;
        *budget=budget.checked_add(length).ok_or("Include budget overflow")?;
        if *budget>2*1024*1024 {return Err("Shader and includes exceed 2 MiB".into());}
        let text=std::fs::read_to_string(path).map_err(|e|e.to_string())?;
        let guard=re(r"(?s)^\s*#ifndef\s+(\w+)[ \t]*\r?\n\s*#define\s+(\w+)[ \t]*\r?\n(.*)\r?\n\s*#endif\s*$");
        let without_bom=text.trim_start_matches('\u{feff}');
        let guarded=guard.captures(without_bom).filter(|c|c[1]==c[2]);
        let text=guarded.as_ref().map_or(without_bom,|c|c.get(3).unwrap().as_str());
        let include=re(r#"^\s*#include\s+"([^"]+)"\s*$"#);
        let mut output=String::new();
        for line in text.lines() {
            if let Some(c)=include.captures(line) {
                if &c[1]=="UnityCG.cginc" {output.push_str(line);} else {
                    let relative=std::path::Path::new(&c[1]);
                    if relative.is_absolute() {return Err("Unsafe include path".into());}
                    let resolved=if c[1].starts_with("Packages/"){package_include(root,relative,path)?}else if c[1].starts_with("Assets/"){root.join(relative)}else{path.parent().unwrap().join(relative)};
                    let mut normalized=std::path::PathBuf::new();
                    for component in resolved.components() {match component {
                        std::path::Component::ParentDir=>{if !normalized.pop(){return Err("Unsafe include path".into());}},
                        std::path::Component::CurDir=>{},other=>normalized.push(other.as_os_str()),
                    }}
                    let resolved=normalized.canonicalize().map_err(|e|format!("Shader include {}: {e}",normalized.display()))?;
                    let allowed=["Assets","Packages","Library/PackageCache"].iter().filter_map(|dir|root.join(dir).canonicalize().ok()).any(|dir|resolved.starts_with(dir));
                    if !allowed {return Err("Unsafe include path: outside project resource directories".into());}
                    output.push_str(&expand(&resolved,root,depth+1,budget)?);
                }
            } else {output.push_str(line);}
            output.push('\n');
        }
        Ok(output)
    }
    let container=path.ancestors().find(|p|p.file_name().is_some_and(|n|n=="Assets" || n=="Packages" || n=="PackageCache")).ok_or("Shader is not inside a Unity project resource directory")?;
    let root=if container.file_name().is_some_and(|n|n=="PackageCache") {container.parent().and_then(|p|p.parent())}else{container.parent()}.ok_or("Missing Unity project root")?;
    translate(&material_render_states(&expand(path,root,0,&mut 0)?,material)?)
}
fn package_include(root:&std::path::Path,relative:&std::path::Path,source:&std::path::Path)->Result<std::path::PathBuf,String> {
    let embedded=root.join(relative);if embedded.is_file() {return Ok(embedded);}
    let mut parts=relative.components();parts.next();
    let Some(std::path::Component::Normal(name))=parts.next() else{return Err("Invalid package include".into());};
    let tail:std::path::PathBuf=parts.collect();let name=name.to_string_lossy();let cache=root.join("Library/PackageCache");
    let matches=|path:&std::path::Path|path.file_name().is_some_and(|n|{let n=n.to_string_lossy();n==name || n.starts_with(&format!("{name}@"))});
    if let Some(package)=source.ancestors().find(|path|path.parent()==Some(cache.as_path()) && matches(path)) {return Ok(package.join(&tail));}
    let candidates:Vec<_>=std::fs::read_dir(cache).into_iter().flatten().flatten().map(|entry|entry.path()).filter(|path|matches(path) && path.join(&tail).is_file()).collect();
    if candidates.len()==1 {Ok(candidates[0].join(tail))}else{Err(format!("Package include unavailable or ambiguous: {}",relative.display()))}
}

fn material_render_states(source:&str,material:&str)->Result<String,String> {
    let mut values=std::collections::HashMap::new();
    for c in re(r"(?m)^\s*(?:\[[^\]\r\n]*\]\s*)*(_\w+)\s*\([^\r\n]*\)\s*=\s*([-+0-9.eE]+)").captures_iter(source) {
        if let Ok(v)=c[2].parse::<f64>() {values.insert(c[1].to_owned(),v);}
    }
    for c in re(r"(?m)^\s*-\s*(_\w+):\s*([-+0-9.eE]+)\s*$").captures_iter(material) {
        if let Ok(v)=c[2].parse::<f64>() {values.insert(c[1].to_owned(),v);}
    }
    let start=re(r"(?i)\bSubShader\b").find(source).ok_or("Missing SubShader")?.end();
    let end=start+source[start..].find("CGPROGRAM").ok_or("Missing CGPROGRAM")?;
    let mut failure=None;
    let state=re(r"(?i)\b(Blend)\s*((?:\[\s*_\w+\s*\]|\w+)[ \t]+(?:\[\s*_\w+\s*\]|\w+))|\b(Cull|ZWrite|ZTest)\s*(\[\s*_\w+\s*\]|\w+)");
    let bound=state.replace_all(&source[start..end],|c:&regex::Captures|{
        let command=c.get(1).or_else(||c.get(3)).unwrap().as_str();
        let value=c.get(2).or_else(||c.get(4)).unwrap().as_str();
        let kind=command.to_ascii_lowercase();
        let arguments=re(r"\[\s*(_\w+)\s*\]").replace_all(value,|p:&regex::Captures|{
            let v=values.get(&p[1]).copied();
            let names:&[&str]=match kind.as_str(){
                "blend"=>&["Zero","One","DstColor","SrcColor","OneMinusDstColor","SrcAlpha","OneMinusSrcColor","DstAlpha","OneMinusDstAlpha","SrcAlphaSaturate","OneMinusSrcAlpha"],
                "cull"=>&["Off","Front","Back"],"zwrite"=>&["Off","On"],
                "ztest"=>&["Off","Never","Less","Equal","LEqual","Greater","NotEqual","GEqual","Always"],_=>&[]};
            if let Some(v)=v.filter(|v|v.is_finite() && *v>=0.0 && v.fract()==0.0 && *v<(names.len() as f64)) {names[v as usize].to_owned()}
            else {failure=Some(format!("Missing or invalid material render state parameter: {}",&p[1]));p[0].to_owned()}
        });
        format!("{} {}",command,arguments)
    });
    if let Some(error)=failure {return Err(error);}
    Ok(format!("{}{}{}",&source[..start],bound,&source[end..]))
}

pub(crate) fn translate(source: &str) -> Result<(Program, RenderState), String> {
    // Included UTF-8 files may each carry a BOM. Embedded BOMs otherwise
    // prevent declarations at line starts from becoming GLSL uniforms.
    let source=source.replace('\u{feff}', "");
    let source = re(r"(?s)/\*.*?\*/|//[^\r\n]*").replace_all(&source, "");
    if source.contains("UniversalPipeline") {
        return Err("URP requires a separate compatibility layer".into());
    }
    if source.matches("CGPROGRAM").count() != 1 {
        return Err("Only a single Built-in CG pass is supported".into());
    }
    let state_start=re(r"(?i)\bSubShader\b").find(&source).ok_or("Missing SubShader")?.end();
    let state_prefix=source[state_start..].split("CGPROGRAM").next().unwrap();
    // State names must not be matched inside property labels or CG function
    // arguments (for example a variable named offset).
    let states=re(r#""(?:\\.|[^"\\])*""#).replace_all(state_prefix,"");
    // Preview intentionally ignores stencil masks while preserving the pass.
    let states=re(r"(?is)\bStencil\s*\{[^{}]*\}").replace_all(&states,"");
    let factor = |name: &str| -> Result<BlendFactor, String> { Ok(match name.to_ascii_lowercase().as_str() {
        "zero" => BlendFactor::Zero, "one" => BlendFactor::One,
        "srcalpha" => BlendFactor::SourceAlpha, "oneminussrcalpha" => BlendFactor::OneMinusSourceAlpha,
        "dstcolor" => BlendFactor::DestinationColor,
        "srccolor"=>BlendFactor::SourceColor,"oneminussrccolor"=>BlendFactor::OneMinusSourceColor,
        "dstalpha"=>BlendFactor::DestinationAlpha,"oneminusdstalpha"=>BlendFactor::OneMinusDestinationAlpha,
        "oneminusdstcolor"=>BlendFactor::OneMinusDestinationColor,"srcalphasaturate"=>BlendFactor::SourceAlphaSaturate,
        _ => return Err(format!("Unsupported blend factor: {name}")),
    }) };
    let blend_re=re(r"(?i)\bBlend\s+(\w+)\s+(\w+)");
    let blend=blend_re.captures(&states);
    let blend_off=re(r"(?i)\bBlend\s+Off\b").is_match(&states);
    let blend=if blend_off {None}else{blend};
    if blend.is_none() && !blend_off && re(r"(?i)\bBlend\b").is_match(&states) {return Err("Unsupported Blend state".into());}
    if !re(r"(?i)\bCull\s+(Off|Back)\b").is_match(&states) || !re(r"(?i)\bZWrite\s+Off\b").is_match(&states) {
        return Err("Compatibility requires Cull Off/Back and ZWrite Off".into());
    }
    if re(r"(?i)\b(BlendOp|Stencil|UsePass|GrabPass|Offset)\b").is_match(&states) || re(r"(?i)\bZTest\s+(\w+)").captures(&states).is_some_and(|c|!["LEqual","Off","Always"].iter().any(|v|c[1].eq_ignore_ascii_case(v))) {
        return Err("Unsupported ShaderLab render state".into());
    }
    let mask = re(r"(?i)\bColorMask\s+(\w+)").captures(&states).map(|c|c[1].to_ascii_uppercase()).unwrap_or("RGBA".into());
    if !matches!(mask.as_str(), "RGB" | "RGBA") { return Err("Unsupported ColorMask".into()); }
    let state = RenderState { source: if let Some(b)=&blend {factor(&b[1])?}else{BlendFactor::One}, destination:if let Some(b)=&blend {factor(&b[2])?}else{BlendFactor::Zero},
        color_mask: [true, true, true, mask == "RGBA"], depth_write: false,cull_back:re(r"(?i)\bCull\s+Back\b").is_match(&states) };
    let body = source.split_once("CGPROGRAM").unwrap().1.split_once("ENDCG").ok_or("Missing ENDCG")?.0;
    let vertex = re(r"#pragma\s+vertex\s+(\w+)").captures(body).ok_or("Missing vertex entry")?[1].to_owned();
    let fragment = re(r"#pragma\s+fragment\s+(\w+)").captures(body).ok_or("Missing fragment entry")?[1].to_owned();
    let mut lines = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            if trimmed == "#include \"UnityCG.cginc\"" || trimmed.starts_with("#pragma vertex ") || trimmed.starts_with("#pragma fragment ") || trimmed.starts_with("#pragma target ") || trimmed=="#pragma multi_compile_instancing" { continue; }
            if re(r"^#define\s+\w+\s+[-+0-9.eE]+$").is_match(trimmed) {lines.push(line);continue;}
            return Err(format!("Unsupported CG directive: {trimmed}"));
        }
        lines.push(line);
    }
    let mut code = lines.join("\n");
    let structs = re(r"(?s)struct\s+(\w+)\s*\{(.*?)\}\s*;");
    let layouts: Vec<_> = structs.captures_iter(&code).map(|c|(c[1].to_owned(),c[2].to_owned())).collect();
    let entry=re(&format!(r"\b(\w+)\s+{vertex}\s*\(\s*(\w+)\s+\w+\s*\)")).captures(&code).ok_or("Unsupported vertex entry signature")?;
    let input=entry[2].to_owned();let output=entry[1].to_owned();
    let mut vertex_assignments=String::new();let mut vertex_varyings=String::new();let mut fragment_varyings=String::new();let mut output_assignments=String::new();let mut fragment_assignments=String::new();let mut position=None;
    let gl_type=|t:&str|->Result<String,String>{let t=t.replace("fixed","float").replace("half","float");Ok(match t.as_str(){"float"=>"float".into(),"float2"=>"vec2".into(),"float3"=>"vec3".into(),"float4"=>"vec4".into(),_=>return Err(format!("Unsupported interface type: {t}"))})};
    for (name, fields) in layouts.iter().filter(|(name,_)|name==&input || name==&output) {
        let fields = re(r"UNITY_VERTEX_INPUT_INSTANCE_ID|UNITY_VERTEX_OUTPUT_STEREO").replace_all(fields, "");
        let valid = re(r"\s*(\w+)\s+(\w+)\s*:\s*(\w+)\s*;");
        if !valid.replace_all(&fields, "").trim().is_empty() {return Err(format!("Unsupported interface layout: {name}"));}
        for field in valid.captures_iter(&fields) {
            let ty=gl_type(&field[1])?;let id=&field[2];let semantic=&field[3];
            if name==&input {
                let value=match semantic {"POSITION" if ty=="vec4"=>"vec4(a_position,1.0)","POSITION" if ty=="vec3"=>"a_position","COLOR" if ty=="vec4"=>"a_color","TEXCOORD0" if ty=="vec2"=>"a_uv","TEXCOORD0" if ty=="vec4"=>"vec4(a_uv,0.0,0.0)","TEXCOORD1" | "TEXCOORD2" if ty=="float"=>"0.0","TEXCOORD1" | "TEXCOORD2" if ty=="vec2"=>"vec2(0.0)","TEXCOORD1" | "TEXCOORD2" if ty=="vec3"=>"vec3(0.0)","TEXCOORD1" | "TEXCOORD2" if ty=="vec4"=>"vec4(0.0)","NORMAL" if ty=="vec3"=>"a_normal","TANGENT" if ty=="vec4"=>"a_tangent",_=>return Err(format!("Unsupported input semantic {semantic} / {ty}"))};
                vertex_assignments.push_str(&format!("i.{id}={value};"));
            } else if semantic=="SV_POSITION" {
                position=Some(id.to_owned());fragment_assignments.push_str(&format!("i.{id}=gl_FragCoord;"));
            } else if semantic.starts_with("TEXCOORD") || semantic=="COLOR" {
                vertex_varyings.push_str(&format!("out {ty} varying_{id};\n"));fragment_varyings.push_str(&format!("in {ty} varying_{id};\n"));
                output_assignments.push_str(&format!("varying_{id}=o.{id};"));fragment_assignments.push_str(&format!("i.{id}=varying_{id};"));
            } else {return Err(format!("Unsupported output semantic: {semantic}"));}
        }
    }
    let position=position.ok_or("Missing SV_POSITION output")?;
    code = re(r"UNITY_VERTEX_INPUT_INSTANCE_ID|UNITY_VERTEX_OUTPUT_STEREO|UNITY_SETUP_INSTANCE_ID\s*\(\s*\w+\s*\)\s*;|UNITY_INITIALIZE_VERTEX_OUTPUT_STEREO\s*\(\s*\w+\s*\)\s*;").replace_all(&code, "").into_owned();
    code = re(r":\s*(POSITION|SV_POSITION|COLOR|NORMAL|TANGENT|TEXCOORD[0-9]+|SV_Target)\b").replace_all(&code, "").into_owned();
    code = re(r"\b(inline|UNITY_BRANCH)\b").replace_all(&code, "").into_owned();
    code = re(r"\b([0-9]+(?:\.[0-9]*)?)h\b").replace_all(&code,"$1").into_owned();
    for (from,to) in [("float4x4","mat4"),("float2x2","mat2"),("float3x3","mat3"),("float4","vec4"),("half4","vec4"),("fixed4","vec4"),("float3","vec3"),("half3","vec3"),("fixed3","vec3"),("float2","vec2"),("half2","vec2"),("fixed2","vec2"),("half","float"),("fixed","float"),("tex2D","texture"),("lerp","mix")] {
        code = re(&format!(r"\b{from}\b")).replace_all(&code,to).into_owned();
    }
    code = re(r"(?m)^\s*(sampler2D|float|vec[234]|mat4)\s+(_\w+)\s*;").replace_all(&code,"uniform $1 $2;").into_owned();
    // HLSL narrows vector expressions to their first component when assigned
    // to a scalar. Bare Unity _Time is vec4; do not substitute _Time.y.
    code=re(r"\bfloat\s+(\w+)\s*=\s*([^;]+);").replace_all(&code,|c:&regex::Captures|{
        let expression=&c[2];
        let converted=re(r"\b_Time\b").replace_all(expression,|t:&regex::Captures|{
            if expression[t.get(0).unwrap().end()..].trim_start().starts_with('.') {t[0].to_owned()}else{"_Time.x".to_owned()}
        });
        format!("float {} = {};",&c[1],converted)
    }).into_owned();
    code=explicit_vector_initializers(&code);
    code=re(r"(\.\s*[xyzwrgba]\s*[+*/-]?=\s*)([^;]+);").replace_all(&code,|c:&regex::Captures|{
        let expression=&c[2];
        let expression=re(r"\b_Time\b").replace_all(expression,|t:&regex::Captures|{
            if expression[t.get(0).unwrap().end()..].trim_start().starts_with('.') {t[0].to_owned()}else{"_Time.x".to_owned()}
        });
        format!("{}{};",&c[1],expression)
    }).into_owned();
    code=local_uniform_writes(&code);
    code=narrow_scalar_components(&code);
    code=re(r"\b(vec[234])\s+(\w+)\s*=\s*([^;]+);").replace_all(&code,|c:&regex::Captures|{
        if c[3].trim().ends_with(".xy") || c[3].trim().ends_with(".xyz") {c[0].to_owned()}
        else {format!("{} {} = {}({});",&c[1],&c[2],&c[1],&c[3])}
    }).into_owned();
    // HLSL permits scalar constants as vector returns. GLSL requires an
    // explicit constructor; identify the enclosing function's declared type.
    let functions=re(r"\b(vec[234])\s+\w+\s*\([^)]*\)\s*\{");
    let ranges:Vec<_>=functions.captures_iter(&code).map(|c|(c[1].to_owned(),c.get(0).unwrap().end())).collect();
    for (ty,start) in ranges.into_iter().rev() {
        let mut depth=1usize;let mut end=None;
        for (offset,ch) in code[start..].char_indices() {if ch=='{'{depth+=1;}else if ch=='}'{depth-=1;if depth==0{end=Some(start+offset);break;}}}
        if let Some(end)=end {
            let converted=re(r"\breturn\s+([-+]?[0-9]+(?:\.[0-9]*)?)\s*;").replace_all(&code[start..end],format!("return {ty}($1);")).into_owned();
            code.replace_range(start..end,&converted);
        }
    }
    let prefix = "#version 330 core\nuniform mat4 unity_ObjectToWorld;\nuniform mat4 unity_MatrixVP;\nuniform vec4 _Time;\n#define UnityObjectToClipPos(v) (unity_MatrixVP * unity_ObjectToWorld * (v))\n#define TRANSFORM_TEX(v,n) ((v) * n##_ST.xy + n##_ST.zw)\n#define mul(a,b) ((a)*(b))\n#define saturate(v) clamp(v,0.0,1.0)\nvec3 UnpackNormal(vec4 n){return n.xyz*2.0-1.0;}\nfloat Luminance(vec4 c){return dot(c.rgb,vec3(0.22,0.707,0.071));}\n";
    // Never declare an overload named pow: some GL drivers hide the entire
    // built-in overload set when a user function shares its name.
    code=re(r"\bpow\s*\(").replace_all(&code,"unity_compat_pow(").into_owned();
    let mut prefix=format!("{prefix}float unity_compat_pow(float x,float y){{return pow(x,y);}}\n");
    prefix.push_str("void sincos(float a,out float s,out float c){s=sin(a);c=cos(a);}\n");
    prefix.push_str("float rsqrt(float x){return inversesqrt(x);}\n");
    prefix.push_str("float Luminance(vec3 c){return dot(c,vec3(0.22,0.707,0.071));}\n#define UNITY_TRANSFER_FOG(o,p)\n#define UNITY_APPLY_FOG(c,col)\nfloat unity_compat_lerp(float a,float b,float t){return mix(a,b,t);}\n");
    code=re(r"\bmix\s*\(").replace_all(&code,"unity_compat_lerp(").into_owned();
    for ty in ["vec2","vec3","vec4"] {
        prefix.push_str(&format!("void sincos({ty} a,out {ty} s,out {ty} c){{s=sin(a);c=cos(a);}}\n"));
        prefix.push_str(&format!("{ty} unity_compat_lerp({ty} a,{ty} b,float t){{return mix(a,b,t);}}\n{ty} unity_compat_lerp({ty} a,{ty} b,{ty} t){{return mix(a,b,t);}}\n{ty} unity_compat_lerp(float a,{ty} b,float t){{return mix({ty}(a),b,t);}}\n{ty} unity_compat_lerp({ty} a,float b,float t){{return mix(a,{ty}(b),t);}}\n"));
        prefix.push_str(&format!("{ty} unity_compat_pow({ty} x,{ty} y){{return pow(x,y);}}\n{ty} unity_compat_pow({ty} x,float y){{return pow(x,{ty}(y));}}\n{ty} unity_compat_pow(float x,{ty} y){{return pow({ty}(x),y);}}\n"));
    }
    if function_reaches_clip(&code,&vertex) {return Err("clip is only supported in the fragment entry call graph".into());}
    code=re(r"\bclip\s*\(").replace_all(&code,"unity_compat_clip(").into_owned();
    // Both stages contain the shared helper source. Fragment-only helpers are
    // unreachable in the vertex entry, but their names must still resolve.
    let mut vertex_clip="void unity_compat_clip(float v){}\n".to_owned();
    let mut fragment_clip="void unity_compat_clip(float v){if(v<0.0)discard;}\n".to_owned();
    for ty in ["vec2","vec3","vec4"] {
        vertex_clip.push_str(&format!("void unity_compat_clip({ty} v){{}}\n"));
        fragment_clip.push_str(&format!("void unity_compat_clip({ty} v){{if(any(lessThan(v,{ty}(0.0))))discard;}}\n"));
    }
    let vs = format!("{prefix}{vertex_clip}{code}\nlayout(location=0) in vec3 a_position;\nlayout(location=1) in vec2 a_uv;\nlayout(location=2) in vec4 a_color;\nlayout(location=3) in vec3 a_normal;\nlayout(location=4) in vec4 a_tangent;\n{vertex_varyings}void main() {{ {input} i; {vertex_assignments} {output} o={vertex}(i); gl_Position=o.{position}; {output_assignments} }}");
    let fs = format!("{prefix}{fragment_clip}{code}\n{fragment_varyings}out vec4 output_color;\nvoid main() {{ {output} i; {fragment_assignments} output_color={fragment}(i); }}");
    Ok((Program { pipeline: UnityPipeline::BuiltIn, vertex: Arc::from(vs), fragment: Arc::from(fs), revision: 1 },state))
}
fn function_reaches_clip(code:&str,entry:&str)->bool {
    let mut calls=std::collections::HashMap::<String,Vec<String>>::new();
    for function in re(r"\b\w+\s+(\w+)\s*\([^)]*\)\s*\{").captures_iter(code) {
        let start=function.get(0).unwrap().end();let mut depth=1;let mut end=None;
        for (offset,ch) in code[start..].char_indices(){if ch=='{'{depth+=1;}else if ch=='}'{depth-=1;if depth==0{end=Some(start+offset);break;}}}
        if let Some(end)=end {calls.entry(function[1].to_owned()).or_default().extend(re(r"\b(\w+)\s*\(").captures_iter(&code[start..end]).map(|c|c[1].to_owned()));}
    }
    let mut pending=vec![entry.to_owned()];let mut seen=std::collections::HashSet::new();
    while let Some(name)=pending.pop() {if name=="clip"{return true;}if !seen.insert(name.clone()){continue;}if let Some(children)=calls.get(&name){pending.extend(children.iter().cloned());}}
    false
}
fn local_uniform_writes(code:&str)->String {
    let uniforms:Vec<_>=re(r"\buniform\s+(float|vec[234])\s+(_\w+)\s*;").captures_iter(code).map(|c|(c[1].to_owned(),c[2].to_owned())).collect();
    let ranges:Vec<_>=re(r"\b\w+\s+\w+\s*\([^)]*\)\s*\{").find_iter(code).map(|m|m.end()).collect();
    let mut output=code.to_owned();
    for start in ranges.into_iter().rev() {
        let mut depth=1;let mut end=None;
        for (offset,ch) in output[start..].char_indices() {if ch=='{'{depth+=1;}else if ch=='}'{depth-=1;if depth==0 {end=Some(start+offset);break;}}}
        let Some(end)=end else{continue;};
        let mut body=output[start..end].to_owned();let mut locals=String::new();
        for (ty,name) in &uniforms {
            if re(&format!(r"\b{}\b(?:\.[xyzwrgba]+)?\s*(?:[+*/-]?=|\+\+|--)",regex::escape(name))).is_match(&body) {
                let local=format!("preview_local{name}");
                body=re(&format!(r"\b{}\b",regex::escape(name))).replace_all(&body,local.as_str()).into_owned();
                locals.push_str(&format!("{ty} {local}={name};\n"));
            }
        }
        output.replace_range(start..end,&format!("{locals}{body}"));
    }
    output
}
fn narrow_scalar_components(code:&str)->String {
    let globals:Vec<_>=re(r"\buniform\s+vec[234]\s+(\w+)\s*;").captures_iter(code).map(|c|c[1].to_owned()).collect();
    let functions:Vec<_>=re(r"\b\w+\s+\w+\s*\(([^)]*)\)\s*\{").captures_iter(code).map(|c|(c.get(0).unwrap().end(),c[1].to_owned())).collect();
    let mut output=code.to_owned();
    for (start,args) in functions.into_iter().rev() {
        let mut depth=1;let mut end=None;
        for (offset,ch) in output[start..].char_indices(){if ch=='{'{depth+=1;}else if ch=='}'{depth-=1;if depth==0{end=Some(start+offset);break;}}}
        let Some(end)=end else{continue;};
        let body=&output[start..end];
        let vectors:Vec<_>=globals.iter().cloned().chain(re(r"\bvec[234]\s+(\w+)\b").captures_iter(&format!("{args}\n{body}")).map(|c|c[1].to_owned())).collect();
        let converted=re(r"(\b\w+(?:\.\w+)*\.[xyzwrgba]\s*[+*/-]?=\s*)([\w.\s+*/-]+);").replace_all(body,|c:&regex::Captures|{
            let expression=&c[2];
            let vector=vectors.iter().any(|name|re(&format!(r"\b{}\b",regex::escape(name))).find_iter(expression).any(|m|!expression[m.end()..].trim_start().starts_with('.') && !expression[..m.start()].trim_end().ends_with('.')));
            if vector {format!("{}({}).x;",&c[1],expression)}else{c[0].to_owned()}
        }).into_owned();
        output.replace_range(start..end,&converted);
    }
    output
}

/// Resolve struct fields through each function's typed parameters. Do not
/// rewrite a field merely because another struct has the same field name.
fn explicit_vector_initializers(code:&str)->String {
    use std::collections::HashMap;
    let mut fields=HashMap::<String,HashMap<String,usize>>::new();
    for structure in re(r"(?s)struct\s+(\w+)\s*\{(.*?)\}\s*;").captures_iter(code) {
        let members=re(r"\bvec([234])\s+(\w+)\s*;").captures_iter(&structure[2]).map(|c|(c[2].to_owned(),c[1].parse().unwrap())).collect();
        fields.insert(structure[1].to_owned(),members);
    }
    let functions:Vec<_>=re(r"\b\w+\s+\w+\s*\(([^)]*)\)\s*\{").captures_iter(code).map(|c|(c.get(0).unwrap().end(),c[1].to_owned())).collect();
    let mut output=code.to_owned();
    let scalar_globals:std::collections::HashSet<_>=re(r"\buniform\s+float\s+(\w+)\s*;").captures_iter(code).map(|c|c[1].to_owned()).collect();
    for (start,arguments) in functions.into_iter().rev() {
        let parameters:HashMap<_,_>=arguments.split(',').filter_map(|arg|{let parts:Vec<_>=arg.split_whitespace().collect();if parts.len()==2 {Some((parts[1],parts[0]))}else{None}}).collect();
        let mut depth=1usize;let mut end=None;
        for (offset,ch) in output[start..].char_indices(){if ch=='{'{depth+=1;}else if ch=='}'{depth-=1;if depth==0{end=Some(start+offset);break;}}}
        let Some(end)=end else{continue;};
        let narrowed=re(r"\bvec([234])\s+(\w+)\s*=\s*(\w+)\.(\w+)\s*;").replace_all(&output[start..end],|c:&regex::Captures|{
            let target=c[1].parse::<usize>().unwrap();
            let source=parameters.get(&c[3]).and_then(|ty|fields.get(*ty)).and_then(|members|members.get(&c[4]));
            if source.is_some_and(|source|*source>target) {
                format!("vec{} {} = {}.{}.{};",&c[1],&c[2],&c[3],&c[4],&"xyzw"[..target])
            }else{c[0].to_owned()}
        }).into_owned();
        let narrowed=re(r"\bvec([234])\s+(\w+)\s*=\s*(\w+)\s*\*\s*(\w+)\s*;").replace_all(&narrowed,|c:&regex::Captures|{
            let target=c[1].parse::<usize>().unwrap();
            let source=parameters.get(&c[3]).and_then(|ty|ty.strip_prefix("vec")).and_then(|n|n.parse::<usize>().ok());
            let scalar=scalar_globals.contains(&c[4]) || parameters.get(&c[4]).is_some_and(|ty|*ty=="float");
            if source.is_some_and(|source|source>target) && scalar {
                format!("vec{} {} = ({} * {}).{};",&c[1],&c[2],&c[3],&c[4],&"xyzw"[..target])
            }else{c[0].to_owned()}
        }).into_owned();
        output.replace_range(start..end,&narrowed);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fragment_clip_preserves_discard_and_rejects_vertex_calls() {
        let source=SAMPLE.replace("return col;","clip(col.a - 0.001); return col;");
        let (program,_)=translate(&source).unwrap();
        assert!(program.fragment.contains("if(v<0.0)discard;"));
        assert!(program.fragment.contains("unity_compat_clip(col.a - 0.001)"));
        assert!(!program.vertex.contains("discard"));
        assert!(function_reaches_clip("void helper(){clip(-1.0);} vec4 vert(vec4 v){helper();return v;}","vert"));
        assert!(!function_reaches_clip("void helper(){clip(-1.0);} vec4 vert(vec4 v){return v;}","vert"));
    }
    #[test]
    fn radial_wind_uses_non_instanced_shader_path() {
        let common=include_str!("../../tests/fixtures/CommonMath.cginc").replace("#ifndef COMMON_MATH_INCLUDED","").replace("#define COMMON_MATH_INCLUDED","").replace("#endif","");
        let source=include_str!("../../tests/fixtures/sprite_radial_wind.shader").replace("#include \"CGIncludes/CommonMath.cginc\"",&common);
        let (program,state)=translate(&source).unwrap();
        assert_eq!(state.source,BlendFactor::One);
        assert!(program.vertex.contains("rsqrt(lenSq)"));
        assert!(program.vertex.contains("float rsqrt(float x){return inversesqrt(x);}"));
        assert!(program.vertex.contains("uniform vec4 _RendererColor;"));
        assert!(!program.vertex.contains("UNITY_SETUP_INSTANCE_ID"));
    }
    #[test]
    fn package_shader_relative_include_and_resource_boundary() {
        let root=std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("shader-package-test-{}",std::process::id()));
        let runtime=root.join("Packages/test.package/Runtime");
        std::fs::create_dir_all(&runtime).unwrap();
        let path=runtime.join("test.shader");
        let include=runtime.parent().unwrap().join("Common.cginc");
        std::fs::write(&include,"#ifndef COMMON_INCLUDED\n#define COMMON_INCLUDED\n#define FACTOR 1.0\n#endif\n").unwrap();
        let source=SAMPLE.replace("#include \"UnityCG.cginc\"","#include \"UnityCG.cginc\"\n#include \"../Common.cginc\"");
        std::fs::write(&path,&source).unwrap();
        let translated=translate_file(&path);assert!(translated.is_ok(),"{:?}",translated.err());
        let cache_runtime=root.join("Library/PackageCache/cache.package@1.0/Runtime");
        std::fs::create_dir_all(&cache_runtime).unwrap();
        let cache_shader=cache_runtime.join("test.shader");
        let cache_include=cache_runtime.parent().unwrap().join("Common.cginc");
        std::fs::write(&cache_include,"#define FACTOR 1.0\n").unwrap();
        std::fs::write(&cache_shader,source.replace("../Common.cginc","Packages/cache.package/Common.cginc")).unwrap();
        assert!(translate_file(&cache_shader).is_ok());
        let outside=root.join("outside.cginc");std::fs::write(&outside,"\n").unwrap();
        std::fs::write(&path,source.replace("../Common.cginc","../../../outside.cginc")).unwrap();
        assert!(translate_file(&path).unwrap_err().contains("Unsafe include path"));
        for file in [&path,&include,&outside,&cache_shader,&cache_include] {std::fs::remove_file(file).unwrap();}
        for directory in [&runtime,&runtime.parent().unwrap().to_path_buf(),&root.join("Packages"),&cache_runtime,&cache_runtime.parent().unwrap().to_path_buf(),&root.join("Library/PackageCache"),&root.join("Library"),&root] {std::fs::remove_dir(directory).unwrap();}
    }
    #[test]
    fn sincos_disabled_blend_and_scalar_component_narrowing() {
        let source=SAMPLE.replace("Blend DstColor SrcAlpha","Blend Off").replace("CGPROGRAM","CGPROGRAM\nfloat2 helper(float2 uv){float4 dis = float4(1,2,3,4); uv.y -= dis * 0.01; float s,c; sincos(1.0,s,c); return uv;}");
        let (program,state)=translate(&source).unwrap();
        assert_eq!(state.source,BlendFactor::One);assert_eq!(state.destination,BlendFactor::Zero);
        assert!(program.fragment.contains("uv.y -= (dis * 0.01).x;"));
        assert!(program.vertex.contains("void sincos(float a,out float s,out float c)"));
    }
    #[test]
    fn ocean_opaque_vectors_matrix_and_mutable_uniform() {
        let source=SAMPLE.replace("Blend DstColor SrcAlpha","").replace("CGPROGRAM","CGPROGRAM\nfloat4 _LightDir;\nfloat2 helper(){float3 p = float4(1,2,3,4); float2 q = p; _LightDir.y += sin(_Time); float2x2 m = float2x2(1,0,0,1); return mul(q,m);}");
        let (program,state)=translate(&source).unwrap();
        assert_eq!(state.source,BlendFactor::One);
        assert_eq!(state.destination,BlendFactor::Zero);
        assert!(program.vertex.contains("vec3 p = vec3(vec4(1,2,3,4));"));
        assert!(program.vertex.contains("vec2 q = vec2(p);"));
        assert!(program.vertex.contains("vec4 preview_local_LightDir = vec4(_LightDir);"));
        assert!(program.vertex.contains("preview_local_LightDir.y += sin(_Time.x)"));
        assert!(program.vertex.contains("mat2 m = mat2(1,0,0,1)"));
    }
    #[test]
    fn water_scalar_time_luminance_and_lerp_compatibility() {
        assert!(!scalar_is_referenced("uniform float _Shininess;","_Shininess"));
        assert!(scalar_is_referenced("uniform float _Shininess; float f(){return _Shininess;}","_Shininess"));
        let source=SAMPLE.replace("CGPROGRAM","CGPROGRAM\nfloat wave(float x){float offset = cos((x - _Time) * 60) * 0.004 + sin(x + _Time.y); return offset;}\nfloat light(float3 c){return Luminance(c);}\nfloat2 flow(){return lerp(0,float2(1,2),1);}");
        let (program,_)=translate(&source).unwrap();
        assert!(program.vertex.contains("cos((x - _Time.x) * 60)"));
        assert!(program.vertex.contains("sin(x + _Time.y)"));
        assert!(program.vertex.contains("unity_compat_lerp(0,vec2(1,2),1)"));
        assert!(program.vertex.contains("float Luminance(vec3 c)"));
        assert!(program.vertex.contains("#define UNITY_TRANSFER_FOG(o,p)\n"));
    }
    #[test]
    fn stencil_is_ignored_without_replacing_shader_math() {
        let source=SAMPLE.replace("Cull Off","Stencil { Ref [_StencilRef] Comp Equal Pass Keep }\nCull Off");
        let (program,state)=translate(&source).unwrap();
        let (original,original_state)=translate(SAMPLE).unwrap();
        assert_eq!(program.vertex,original.vertex);
        assert_eq!(program.fragment,original.fragment);
        assert_eq!(state.source,original_state.source);
        assert_eq!(state.destination,original_state.destination);
    }
    #[test]
    fn material_blend_states_override_shader_defaults() {
        let source=format!("Properties {{\n[Enum(UnityEngine.Rendering.BlendMode)] _SrcBlend(\"Source\", Float) = 1\n_DstBlend(\"Destination\", Float) = 0\n_Cull(\"Cull\", Float) = 2\n_ZWrite(\"Write\", Float) = 0\n_ZTest(\"Test\", Float) = 4\n}}\n{}",SAMPLE.replace("Blend DstColor SrcAlpha","Blend [_SrcBlend] [_DstBlend]").replace("Cull Off","Cull[_Cull]").replace("ZWrite Off","ZWrite[_ZWrite]\nZTest [_ZTest]"));
        let material="m_Floats:\n- _SrcBlend: 5\n- _DstBlend: 10\n- _Cull: 0\n- _ZTest: 0\n";
        let (_,state)=translate(&material_render_states(&source,material).unwrap()).unwrap();
        assert_eq!(state.source,BlendFactor::SourceAlpha);
        assert_eq!(state.destination,BlendFactor::OneMinusSourceAlpha);
        assert!(!state.cull_back);
        let (_,defaults)=translate(&material_render_states(&source,"").unwrap()).unwrap();
        assert_eq!(defaults.source,BlendFactor::One);
        assert!(defaults.cull_back);
        assert!(material_render_states(&source,"- _SrcBlend: 999\n").is_err());
    }
    const SAMPLE:&str=include_str!("../../tests/fixtures/pf_sprite_overlay.shader");
    #[test]
    fn ocean_source_keeps_multitexture_flow_and_global_color() {
        let source=include_str!("../../tests/fixtures/pf_ocean_2d_new.shader").replace("#include \"Assets/Res/Shaders/DayNightSystemShaderHelper.cginc\"",include_str!("../../tests/fixtures/DayNightSystemShaderHelper.cginc"));
        let (program,state)=translate(&source).unwrap();
        assert!(program.fragment.contains("texture(_MaskTex, i.uvMask).rgb"));
        assert!(program.fragment.contains("FinalGlobalColor(col, _GlobalDayNightWaterColor"));
        assert!(program.vertex.contains("_Time.yyyy * _BumpDirection * _NormalSpeed"));
        assert!(program.vertex.contains("varying_bumpCoords=o.bumpCoords"));
        assert!(state.cull_back);
    }
    #[test]
    fn caustics_branch_vector_return_and_additive_state() {
        let (program,state)=translate(include_str!("../../tests/fixtures/pf_river_caustics_2d.shader")).unwrap();
        assert!(program.fragment.contains("return vec4(0);"));
        assert!(program.fragment.contains("unity_compat_pow(saturate(caustics), _Contrast)"));
        assert_eq!(state.source,BlendFactor::SourceAlpha);
        assert_eq!(state.destination,BlendFactor::One);
    }
    #[test]
    fn pow_compatibility_does_not_shadow_glsl_builtins() {
        let source=include_str!("../../tests/fixtures/pf_ocean_2d_new.shader").replace("#include \"Assets/Res/Shaders/DayNightSystemShaderHelper.cginc\"",include_str!("../../tests/fixtures/DayNightSystemShaderHelper.cginc"));
        let (program,_)=translate(&source).unwrap();
        for stage in [&program.vertex,&program.fragment] {
            assert!(!re(r"\b(float|vec[234])\s+pow\s*\(").is_match(stage));
            assert!(stage.contains("float unity_compat_pow(float x,float y){return pow(x,y);}"));
            assert!(stage.contains("vec3 unity_compat_pow(vec3 x,float y){return pow(x,vec3(y));}"));
            assert!(stage.contains("unity_compat_pow(depth, _MaskPower)"));
        }
    }
    #[test]
    fn beach_shader_accepts_case_insensitive_states_and_float4_uv() {
        let source=include_str!("../../tests/fixtures/pf_beach_wave.shader").replace("#include \"Assets/Res/Shaders/DayNightSystemShaderHelper.cginc\"",include_str!("../../tests/fixtures/DayNightSystemShaderHelper.cginc"));
        let (program,state)=translate(&source).unwrap();
        assert_eq!(state.source,BlendFactor::SourceAlpha);
        assert!(!state.cull_back);
        assert!(program.vertex.contains("i.uv=vec4(a_uv,0.0,0.0)"));
        assert!(program.fragment.contains("_DistortionTex_ST"));
        assert!(program.vertex.contains("vec2 inuv = v.uv.xy;"));
        assert!(!program.vertex.contains("vec2 inuv = v.uv;"));
        assert!(program.fragment.contains("uniform vec4 _GlobalDayNightColor;"),"{}",program.fragment);
        assert!(program.fragment.contains("uniform vec4 _TintColor;"));
    }
    #[test]
    fn vector_narrowing_uses_parameter_struct_types() {
        let code="struct Wide { vec4 uv; }; struct Narrow { vec2 uv; }; vec2 first(Wide v){ vec2 t = v.uv; return t; } vec2 second(Narrow v){ vec2 t = v.uv; return t; }";
        let converted=explicit_vector_initializers(code);
        assert!(converted.contains("first(Wide v){ vec2 t = v.uv.xy;"));
        assert!(converted.contains("second(Narrow v){ vec2 t = v.uv;"));
        let converted=explicit_vector_initializers("uniform float _Brightness; vec4 ColorA(vec4 c){vec3 finalColor = c * _Brightness; return c;} vec4 Other(vec4 c, vec4 factor){vec3 t = c * factor; return c;}");
        assert!(converted.contains("vec3 finalColor = (c * _Brightness).xyz;"));
        assert!(converted.contains("vec3 t = c * factor;"));
    }
    #[test]
    fn translates_referenced_shader_without_substituting_fragment_math() {
        let (program,state)=translate(SAMPLE).unwrap();
        assert!(program.fragment.contains("col = unity_compat_lerp(col,col*i.color,col.a)"));
        assert!(program.fragment.contains("abs(_ToggleDayLit - _LightIntensity)"));
        assert_eq!(state.source,BlendFactor::DestinationColor);
        assert_eq!(state.destination,BlendFactor::SourceAlpha);
        assert_eq!(state.color_mask,[true,true,true,false]);
    }
    #[test]
    fn changed_fragment_expression_is_translated_from_source() {
        let shader=SAMPLE.replace("col = lerp(col,col*i.color,col.a);","col.rgb = col.rgb * i.color.rgb;");
        let (program,_)=translate(&shader).unwrap();
        assert!(program.fragment.contains("col.rgb = col.rgb * i.color.rgb;"));
        assert!(!program.fragment.contains("unity_compat_lerp(col,col*i.color,col.a)"));
    }
    #[test]
    fn unsupported_pipeline_and_states_fail_explicitly() {
        assert!(translate(&SAMPLE.replace("UnityCG.cginc","Packages/com.unity.render-pipelines.universal/ShaderLibrary/Core.hlsl")).is_err());
        assert!(translate(&SAMPLE.replace("ZWrite Off","ZWrite On")).is_err());
        assert!(translate(&SAMPLE.replace("Blend DstColor SrcAlpha","BlendOp Max\nBlend DstColor SrcAlpha")).is_err());
    }
}
