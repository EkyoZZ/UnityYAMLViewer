//! Uncompressed text-serialized Unity Mesh vertex/index buffers.
use super::scene::Vertex;
fn field<'a>(text:&'a str,key:&str)->Option<&'a str> {
    let prefix=format!("{key}:");
    text.lines().find_map(|line|line.trim_start().strip_prefix(&prefix).map(str::trim))
}

pub(crate) struct Mesh {
    pub vertices: Vec<Vertex>,
    pub submeshes: Vec<Vec<u32>>,
}

pub(crate) fn quad()->Mesh {
    Mesh { vertices:[[-0.5,-0.5],[0.5,-0.5],[0.5,0.5],[-0.5,0.5]].into_iter().zip([[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]])
        .map(|(p,uv)|Vertex{position:[p[0],p[1],0.0],uv,color:[1.0;4],normal:[0.0,0.0,-1.0],tangent:[1.0,0.0,0.0,1.0]}).collect(),submeshes:vec![vec![0,2,1,0,3,2]] }
}
pub(crate) fn fbx(path:&std::path::Path)->Result<Mesh,String> {
    if std::fs::metadata(path).map_err(|e|e.to_string())?.len()>64*1024*1024{return Err("FBX exceeds 64 MiB".into());}
    let mut opts=ufbx::LoadOpts::default();opts.ignore_animation=true;opts.ignore_embedded=true;
    opts.temp_allocator.memory_limit=128*1024*1024;opts.result_allocator.memory_limit=128*1024*1024;
    let scene=ufbx::load_file(path.to_str().ok_or("Invalid FBX path")?,opts).map_err(|e|format!("FBX decode: {e:?}"))?;
    let nodes:Vec<_>=scene.nodes.iter().filter(|n|n.mesh.is_some()).collect();
    if nodes.len()!=1{return Err("GPU FBX requires one unambiguous mesh node".into());}
    let node=nodes[0];let mesh=node.mesh.as_ref().unwrap();
    if !mesh.skin_deformers.is_empty(){return Err("GPU FBX skinning is not implemented".into());}
    let meta=std::fs::read_to_string(format!("{}.meta",path.display())).unwrap_or_default();
    let scale=crate::model_preview::importer_scale(&meta,scene.settings.unit_meters)?;
    if mesh.max_face_triangles>500_000{return Err("FBX face exceeds triangle budget".into());}
    let mut scratch=vec![0;mesh.max_face_triangles*3];let mut vertices=Vec::new();let mut slots=Vec::<Vec<u32>>::new();
    for (face_index,face) in mesh.faces.iter().enumerate(){
        let count=mesh.triangulate_face(&mut scratch,*face) as usize*3;
        if vertices.len()+count>1_500_000{return Err("FBX exceeds triangle budget".into());}
        let slot=mesh.face_material.get(face_index).copied().unwrap_or(0) as usize;
        if slot>=64{return Err("FBX exceeds material slot budget".into());}while slots.len()<=slot{slots.push(Vec::new());}
        for &index in &scratch[..count]{
            let p=ufbx::transform_position(&node.geometry_to_node,mesh.vertex_position[index as usize]);
            let uv=if mesh.vertex_uv.exists{let v=mesh.vertex_uv[index as usize];[v.x as f32,v.y as f32]}else{[0.0;2]};
            let normal=if mesh.vertex_normal.exists{let n=ufbx::transform_direction(&node.geometry_to_node,mesh.vertex_normal[index as usize]);[-n.x as f32,n.y as f32,n.z as f32]}else{[0.0,0.0,-1.0]};
            let color=if mesh.vertex_color.exists{let c=mesh.vertex_color[index as usize];[c.x as f32,c.y as f32,c.z as f32,c.w as f32]}else{[1.0;4]};
            let vertex=Vertex{position:[(-p.x*scale) as f32,(p.y*scale) as f32,(p.z*scale) as f32],uv,color,normal,tangent:[1.0,0.0,0.0,1.0]};
            if vertex.position.iter().chain(vertex.uv.iter()).chain(vertex.color.iter()).chain(vertex.normal.iter()).any(|v|!v.is_finite()){return Err("FBX has non-finite vertex data".into());}
            slots[slot].push(vertices.len() as u32);vertices.push(vertex);
        }
    }
    if vertices.is_empty(){return Err("FBX has no triangles".into());}Ok(Mesh{vertices,submeshes:slots})
}
fn number(text:&str,key:&str)->Result<usize,String> {
    field(text,key).ok_or_else(||format!("Missing Mesh field {key}"))?.trim().parse().map_err(|_|format!("Invalid Mesh field {key}"))
}
fn hex(text:&str)->Result<Vec<u8>,String> {
    if text.len()%2!=0 || text.len()>128*1024*1024 {return Err("Invalid or oversized Mesh hex buffer".into());}
    text.as_bytes().chunks_exact(2).map(|p|{
        let a=(p[0] as char).to_digit(16).ok_or("Invalid Mesh hex")?;
        let b=(p[1] as char).to_digit(16).ok_or("Invalid Mesh hex")?;
        Ok((a*16+b) as u8)
    }).collect()
}
pub(crate) fn decode(text:&str)->Result<Mesh,String> {
    if number(text,"m_MeshCompression")?!=0 {return Err("Compressed Unity Mesh is not supported".into());}
    if field(text,"path").is_some_and(|p|!p.trim().trim_matches('"').is_empty()) {return Err("External Mesh stream data is not supported".into());}
    let count=number(text,"m_VertexCount")?;
    if count==0 || count>1_000_000 {return Err("Mesh vertex count exceeds budget or is zero".into());}
    let bytes=hex(field(text,"_typelessdata").ok_or("Missing vertex buffer")?)?;
    if bytes.len()!=number(text,"m_DataSize")? {return Err("Mesh vertex buffer length mismatch".into());}
    let channel_section=text.split_once("m_Channels:").ok_or("Missing vertex channels")?.1.split_once("m_DataSize:").ok_or("Missing vertex data size")?.0;
    let channels=channel_section.split("- stream:").skip(1).map(|part|{
        let stream=part.lines().next().unwrap_or("").trim().parse::<usize>().map_err(|_|"Invalid stream")?;
        Ok((stream,number(part,"offset")?,number(part,"format")?,number(part,"dimension")?))
    }).collect::<Result<Vec<_>,String>>()?;
    let mut stride=0usize;
    for &(stream,offset,format,dim) in &channels {
        if dim==0 {continue;}
        if stream!=0 || dim>4 || !matches!(format,0|2) {return Err("Only stream 0 Float32 / UNorm8 Mesh channels are supported".into());}
        stride=stride.max(offset+dim*if format==0{4}else{1});
    }
    if stride==0 || stride.checked_mul(count)!=Some(bytes.len()) {return Err("Mesh stream stride / buffer mismatch".into());}
    if channels.first().is_none_or(|c|c.3<3) {return Err("Missing position channel".into());}
    let mut vertices=Vec::with_capacity(count);
    for vertex in 0..count {
        let read=|channel:usize,component:usize,default:f32|->Result<f32,String>{
            let Some(&(_,offset,format,dim))=channels.get(channel) else{return Ok(default)};
            if component>=dim {return Ok(default);}
            let index=vertex*stride+offset+component*if format==0{4}else{1};
            let value=if format==0 {f32::from_le_bytes(bytes.get(index..index+4).ok_or("Mesh vertex channel exceeds buffer")?.try_into().unwrap())}else{*bytes.get(index).ok_or("Mesh vertex channel exceeds buffer")? as f32/255.0};
            if !value.is_finite(){return Err("Non-finite Mesh vertex channel".into());}Ok(value)
        };
        vertices.push(Vertex{position:[read(0,0,0.0)?,read(0,1,0.0)?,read(0,2,0.0)?],uv:[read(4,0,0.0)?,read(4,1,0.0)?],color:[read(3,0,1.0)?,read(3,1,1.0)?,read(3,2,1.0)?,read(3,3,1.0)?],normal:[read(1,0,0.0)?,read(1,1,0.0)?,read(1,2,-1.0)?],tangent:[read(2,0,1.0)?,read(2,1,0.0)?,read(2,2,0.0)?,read(2,3,1.0)?]});
    }
    let index_bytes=hex(field(text,"m_IndexBuffer").ok_or("Missing index buffer")?)?;
    let index_size=match number(text,"m_IndexFormat")?{0=>2,1=>4,_=>return Err("Unsupported index format".into())};
    let section=text.split_once("m_SubMeshes:").ok_or("Missing submeshes")?.1.split_once("m_Shapes:").ok_or("Missing Mesh shapes section")?.0;
    let mut submeshes=Vec::new();
    for part in section.split("- serializedVersion:").skip(1) {
        if number(part,"topology")?!=0 {return Err("Only triangle Mesh topology is supported".into());}
        let start=number(part,"firstByte")?;let count=number(part,"indexCount")?;let base=number(part,"baseVertex")?;
        if count%3!=0 {return Err("Triangle index count is not divisible by 3".into());}
        let end=start.checked_add(count.checked_mul(index_size).ok_or("Index count overflow")?).ok_or("Index range overflow")?;
        let mut indices=Vec::with_capacity(count);
        for p in index_bytes.get(start..end).ok_or("Submesh exceeds index buffer")?.chunks_exact(index_size) {
            let index=if index_size==2{u16::from_le_bytes(p.try_into().unwrap()) as usize}else{u32::from_le_bytes(p.try_into().unwrap()) as usize};
            let index=index.checked_add(base).ok_or("Base vertex overflow")?;
            if index>=vertices.len(){return Err("Mesh index exceeds vertex count".into());}indices.push(index as u32);
        }
        submeshes.push(indices);
    }
    if submeshes.is_empty(){return Err("Mesh has no submeshes".into());}
    Ok(Mesh{vertices,submeshes})
}

pub(crate) fn decode_reference(text:&str,file_id:&str)->Result<Mesh,String> {
    for part in text.split("--- !u!43 &").skip(1) {
        let (header,body)=part.split_once('\n').ok_or("Invalid Mesh object header")?;
        if header.trim().split_whitespace().next()==Some(file_id) {
            return decode(body.split("--- !u!").next().unwrap_or(body));
        }
    }
    Err(format!("Unity Mesh fileID {file_id} not found"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore="Requires UNITY_SHADER_TEST_PROJECT"]
    fn real_beach_fbx_gpu_geometry() {
        let root=std::path::PathBuf::from(std::env::var("UNITY_SHADER_TEST_PROJECT").unwrap());
        for name in ["map_101_beach","map_101_beach2","map_101_beach3","map_101_beach4"]{
            let mesh=fbx(&root.join(format!("Assets/Res/Mesh/Map/{name}.FBX"))).unwrap();
            assert!(!mesh.vertices.is_empty());assert_eq!(mesh.submeshes.len(),1);
            assert_eq!(mesh.vertices.len(),mesh.submeshes[0].len());
            assert!(mesh.vertices.iter().any(|v|v.uv!=[0.0;2]));
        }
    }
    const SAMPLE:&str=include_str!("../../tests/fixtures/l9584_river1.asset");
    #[test]
    fn reads_real_unity_river_mesh() {
        let mesh=decode(SAMPLE).unwrap();
        assert_eq!(mesh.vertices.len(),23);
        assert_eq!(mesh.submeshes[0].len(),63);
        assert_eq!(&mesh.submeshes[0][..3],&[10,7,21]);
        assert!(mesh.vertices[0].position[0]>800.0);
        let [a,b,c]=[10,7,21].map(|i|mesh.vertices[i].position);
        assert!((b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])<0.0);
        assert_eq!(decode_reference(SAMPLE,"4300000").unwrap().vertices.len(),23);
        assert!(decode_reference(SAMPLE,"4300001").is_err());
    }
    #[test]
    fn invalid_index_and_compression_are_rejected() {
        assert!(decode(&SAMPLE.replace("m_MeshCompression: 0","m_MeshCompression: 1")).is_err());
        assert!(decode(&SAMPLE.replace("m_IndexBuffer: 0a00","m_IndexBuffer: ffff")).is_err());
    }
}
