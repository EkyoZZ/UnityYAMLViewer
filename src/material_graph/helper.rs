//! Helper inspection evaluates a unique direct stage call with its actual arguments.
use super::{stages,probe,re,tokens,block_end};
fn context(graph:&stages::Graph,index:usize)->Result<(usize,usize,Vec<String>),String>{
    let helper=&graph.groups[index];
    if graph.limited {return Err("Helper preview requires an untruncated function graph".into());}
    if graph.groups.iter().filter(|g|g.name==helper.name).count()!=1{return Err("Overloaded helper preview is ambiguous".into());}
    if re(r"\b(out|inout)\b").is_match(&helper.signature){return Err("Helper out / inout parameters are not supported for snapshots".into());}
    if helper.argument_types.iter().any(|ty|probe::width(ty).is_none()){return Err("Helper snapshot currently requires numeric parameters".into());}
    let mut sites=Vec::new();
    for (caller,group) in graph.groups.iter().enumerate(){
        if !group.calls.contains(&index){continue;}
        for (step,statement) in group.statements.iter().enumerate(){
            let t=tokens(statement);
            for i in 0..t.len().saturating_sub(1){
                if t[i]!=helper.name||t[i+1]!="("{continue;}
                let end=block_end(&t,i+1,"(",")").ok_or("Unclosed helper call")?;
                let mut args=Vec::new();let mut start=i+2;let mut depth=0;
                for j in i+2..end {if t[j]=="("{depth+=1;}else if t[j]==")"{depth-=1;}else if t[j]==","&&depth==0{args.push(t[start..j].join(" "));start=j+1;}}
                if start<end{args.push(t[start..end].join(" "));}
                sites.push((caller,step,args));
            }
        }
    }
    if sites.len()!=1{return Err("Helper snapshot requires one unique call site; multiple or unused calls are display-only".into());}
    let (caller,step,args)=sites.pop().unwrap();let group=&graph.groups[caller];
    if group.stage==stages::Stage::Function{return Err("Nested helper calls are display-only for now".into());}
    if args.len()!=helper.parameters.len(){return Err("Helper argument count mismatch".into());}
    let prefix=group.statements[..=step].join("\n");
    if re(r"\b(if|else|for|while|switch|do)\b|[{}]|\+\s*\+|-\s*-").is_match(&prefix)||group.statements[..step].iter().any(|s|re(r"\breturn\b").is_match(s)){
        return Err("Helper snapshot requires a straight-line stage call without increment side effects".into());
    }
    // Argument expressions may use pure built-ins, but user calls could have side effects.
    for arg in &args {for group in &graph.groups {if re(&format!(r"\b{}\s*\(",regex::escape(&group.name))).is_match(arg){return Err("User function calls inside helper arguments are display-only".into());}}}
    Ok((caller,step,args))
}
pub(super) fn vertex_context(graph:&stages::Graph,index:usize)->Result<bool,String>{let (caller,_,_)=context(graph,index)?;Ok(graph.groups[caller].stage==stages::Stage::Vert)}
pub(super) fn candidate(graph:&stages::Graph,index:usize,step:usize)->Result<probe::Probe,String>{
    let group=&graph.groups[index];let (caller,_,_)=context(graph,index)?;
    let mut numeric=group.clone();numeric.stage=stages::Stage::Frag;
    let mut result=probe::candidate(&numeric,step)?;result.vertex=graph.groups[caller].stage==stages::Stage::Vert;Ok(result)
}
pub(super) fn instrument(source:&str,selected:&probe::Probe)->Result<String,String>{
    let graph=stages::structure(source);
    let groups=graph.groups.iter().enumerate().filter(|(_,g)|g.name==selected.function&&g.stage==stages::Stage::Function).collect::<Vec<_>>();
    if groups.len()!=1{return Err("Selected helper is unavailable or ambiguous".into());}
    let (index,helper)=groups[0];let expected=if let Some((expr,width))=&selected.expression{let mut p=probe::expression_candidate(helper,selected.step,expr,*width)?;p.vertex=vertex_context(&graph,index)?;p}else{candidate(&graph,index,selected.step)?};
    if tokens(&expected.statement)!=tokens(&selected.statement)||expected.target!=selected.target||expected.vertex!=selected.vertex{return Err("Helper source or call context changed; reload the node preview".into());}
    let (caller,call_step,args)=context(&graph,index)?;let caller=&graph.groups[caller];
    let prefix=helper.statements[..=selected.step].join("\n");
    if re(r"\b(discard|clip)\b|\+\s*\+|-\s*-").is_match(&prefix){return Err("Helper snapshot side effects are unsupported".into());}
    if re(r"\bpreview_helper_\w*\b").is_match(source){return Err("Shader uses reserved helper snapshot identifiers".into());}
    for &called in &helper.calls {if re(&format!(r"\b{}\s*\(",regex::escape(&graph.groups[called].name))).is_match(&prefix){return Err("Nested helper snapshot prefixes are display-only for now".into());}}
    let declarations=helper.argument_types.iter().zip(&helper.parameters).map(|(ty,p)|format!("{ty} {p};")).collect::<Vec<_>>().join("\n");
    let count=if let Some((_,width))=&selected.expression{*width}else{probe::result_width(source,&format!("{declarations}\n{prefix}"),"","",&selected.target)?};
    let mut names=std::collections::HashMap::new();
    for p in &helper.parameters{names.insert(p.clone(),format!("preview_helper_{p}"));}
    for c in re(r"\b(?:float|half|fixed|int|uint|bool)[1-4]?\s+(\w+)\b").captures_iter(&prefix){names.insert(c[1].to_owned(),format!("preview_helper_{}",&c[1]));}
    let rename=|text:&str|re(r"\b[A-Za-z_]\w*\b").replace_all(text,|c:&regex::Captures<'_>|{
        if text[..c.get(0).unwrap().start()].trim_end().ends_with('.') {c[0].to_owned()}else{names.get(&c[0]).cloned().unwrap_or_else(||c[0].to_owned())}
    }).into_owned();
    let bindings=helper.argument_types.iter().zip(&helper.parameters).zip(args).map(|((ty,p),arg)|format!("{ty} {} = {arg};",names[p])).collect::<Vec<_>>().join("\n");
    let mut statements=helper.statements[..=selected.step].to_vec();
    if selected.expression.is_some() || statements.last().is_some_and(|s|s.trim_start().starts_with("return ")){statements.pop();}
    let ty=if count==1{"float".to_owned()}else{format!("float{count}")};
    let result_statement=format!("{ty} preview_helper_result = {};",rename(&selected.target));
    let inserted=format!("{bindings}\n{}\n{result_statement}\n",rename(&statements.join("\n")));
    let before=caller.statements[..call_step].join("\n");let after=caller.statements[call_step..].join("\n");
    let signature=re(&format!(r"\b\w+\s+{}\s*\([^)]*\)\s*(?::\s*\w+)?\s*\{{",regex::escape(&caller.name)));
    let matches=signature.find_iter(source).collect::<Vec<_>>();if matches.len()!=1{return Err("Caller source is unavailable or ambiguous".into());}
    let start=matches[0].end();let mut depth=1;let mut end=None;
    for (offset,ch) in source[start..].char_indices(){if ch=='{'{depth+=1;}else if ch=='}'{depth-=1;if depth==0{end=Some(start+offset);break;}}}
    let end=end.ok_or("Unterminated caller")?;
    let generated=format!("{}\n{before}\n{inserted}\n{after}\n{}",&source[..start],&source[end..]);
    let step=tokens(&format!("{before}\n{inserted}")).iter().filter(|s|s.as_str()==";").count()-1;
    probe::instrument(&generated,&probe::Probe{function:caller.name.clone(),step,statement:result_statement,target:"preview_helper_result".into(),alpha:selected.alpha,vertex:selected.vertex,expression:None})
}
#[cfg(test)]
mod tests{
    use super::*;
    const SOURCE:&str="CGPROGRAM\n#pragma fragment frag\nstruct F{float2 uv:TEXCOORD0;};float4 Change(float4 color,float scale){float3 rgb=color.rgb*scale;float4 result=float4(rgb,color.a);return result;}float4 frag(F i):SV_Target{float4 c=tex2D(_MainTex,i.uv);float4 finalColor=Change(c,0.5);return finalColor;}\nENDCG";
    #[test]fn helper_expression_uses_bound_arguments_before_current_assignment(){let graph=stages::parse(SOURCE,&[]);let group=graph.groups.iter().find(|g|g.name=="Change").unwrap();let node=group.operations.nodes.iter().find(|n|n.title=="Multiply").unwrap();let result=instrument(SOURCE,node.probe.as_ref().unwrap()).unwrap();assert!(result.contains("preview_helper_color = c"));assert!(result.contains("preview_helper_scale = 0.5"));assert!(result.contains("preview_helper_result"));assert!(!result.contains("preview_helper_color).preview_helper_rgb"));}
    #[test]fn helper_uses_actual_call_arguments_and_only_selected_prefix(){let graph=stages::parse(SOURCE,&[]);let index=graph.groups.iter().position(|g|g.name=="Change").unwrap();let p=candidate(&graph,index,0).unwrap();let result=instrument(SOURCE,&p).unwrap();assert!(result.contains("preview_helper_color = c"));assert!(result.contains("preview_helper_scale = 0.5"));assert!(result.contains("preview_helper_color . rgb"));assert!(!result.contains("preview_helper_color . preview_helper_rgb"));assert!(!result.contains("preview_helper_result = result"));assert!(result.contains("return float4(preview_helper_result,1)"));}
    #[test]fn multiple_calls_and_nested_context_do_not_pick_arbitrary_inputs(){let source=SOURCE.replace("return finalColor;","finalColor=Change(c,1);return finalColor;");let graph=stages::parse(&source,&[]);let index=graph.groups.iter().position(|g|g.name=="Change").unwrap();assert!(candidate(&graph,index,0).is_err());}
}
