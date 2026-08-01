//! 3MF Core mesh import with schema units.
//!
//! 3MF carries an authoritative `model/@unit` and a tessellated body. Reyn
//! reads the ZIP package, extracts the primary 3D model, and converts triangles
//! into the same `Mesh` path used by STL — without silent repair. Topology
//! gates remain the responsibility of `diagnose_mesh` / preflight.

use crate::cad::Mesh;
use std::io::{Cursor, Read};

pub const TRANSLATOR: &str = "reyn-3mf";
pub const TRANSLATOR_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_3MF_BYTES: usize = 128 * 1024 * 1024;
const MAX_TRIANGLES: usize = 250_000;

#[derive(Clone, Debug)]
pub struct ThreeMfImport {
    pub mesh: Mesh,
    pub declared_unit: String,
    pub warnings: Vec<String>,
}

/// Parse a 3MF package into a triangle mesh and declared length unit.
pub fn parse_3mf(bytes: &[u8]) -> Result<ThreeMfImport, String> {
    if bytes.is_empty() {
        return Err("3MF source is empty".into());
    }
    if bytes.len() > MAX_3MF_BYTES {
        return Err(format!(
            "3MF source is {:.1} MB; the safe import limit is {} MB",
            bytes.len() as f64 / (1024.0 * 1024.0),
            MAX_3MF_BYTES / (1024 * 1024)
        ));
    }

    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|error| format!("the 3MF package could not be opened as a ZIP: {error}"))?;

    let model_index = (0..archive.len())
        .find(|&index| {
            archive
                .name_for_index(index)
                .is_some_and(|name| name.eq_ignore_ascii_case("3D/3dmodel.model"))
        })
        .or_else(|| {
            (0..archive.len()).find(|&index| {
                archive
                    .name_for_index(index)
                    .is_some_and(|name| name.to_ascii_lowercase().ends_with(".model"))
            })
        })
        .ok_or_else(|| {
            "the 3MF package has no 3D model part (expected 3D/3dmodel.model)".to_string()
        })?;

    let mut model_file = archive
        .by_index(model_index)
        .map_err(|error| format!("the 3MF model part could not be read: {error}"))?;
    let mut xml = String::new();
    model_file
        .read_to_string(&mut xml)
        .map_err(|error| format!("the 3MF model XML could not be decoded: {error}"))?;

    parse_3mf_model_xml(&xml)
}

fn parse_3mf_model_xml(xml: &str) -> Result<ThreeMfImport, String> {
    let document = roxmltree::Document::parse(xml)
        .map_err(|error| format!("the 3MF model XML is malformed: {error}"))?;
    let root = document.root_element();
    if !root.has_tag_name("model") {
        return Err("the 3MF model root element must be <model>".into());
    }

    let unit_raw = root
        .attribute("unit")
        .unwrap_or("millimeter")
        .trim()
        .to_ascii_lowercase();
    let declared_unit = match unit_raw.as_str() {
        "millimeter" => "mm",
        "centimeter" => "cm",
        "meter" => "m",
        "inch" => "in",
        "foot" => "ft",
        "micron" | "micrometer" => {
            return Err(
                "3MF micron units are not supported yet; re-export the model in millimetres".into(),
            );
        }
        other => {
            return Err(format!(
                "the 3MF model declares unsupported unit '{other}'; re-export in mm, cm, m, in, or ft"
            ));
        }
    };

    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut triangles: Vec<[usize; 3]> = Vec::new();
    let mut warnings = Vec::new();

    for object in root.descendants().filter(|n| n.has_tag_name("object")) {
        let mesh = match object.children().find(|n| n.has_tag_name("mesh")) {
            Some(mesh) => mesh,
            None => continue,
        };
        let vertex_base = vertices.len();
        if let Some(verts) = mesh.children().find(|n| n.has_tag_name("vertices")) {
            for vertex in verts.children().filter(|n| n.has_tag_name("vertex")) {
                let x = attr_f32(vertex, "x")?;
                let y = attr_f32(vertex, "y")?;
                let z = attr_f32(vertex, "z")?;
                if ![x, y, z].iter().all(|v| v.is_finite()) {
                    return Err("3MF vertex coordinates must be finite".into());
                }
                vertices.push([x, y, z]);
            }
        }
        if let Some(tris) = mesh.children().find(|n| n.has_tag_name("triangles")) {
            for triangle in tris.children().filter(|n| n.has_tag_name("triangle")) {
                let v1 = attr_usize(triangle, "v1")? + vertex_base;
                let v2 = attr_usize(triangle, "v2")? + vertex_base;
                let v3 = attr_usize(triangle, "v3")? + vertex_base;
                triangles.push([v1, v2, v3]);
            }
        }
    }

    if triangles.is_empty() {
        return Err("the 3MF model contains no triangles".into());
    }
    if triangles.len() > MAX_TRIANGLES {
        return Err(format!(
            "3MF tessellation exceeds the safe limit of {MAX_TRIANGLES} triangles"
        ));
    }

    let mut tris = Vec::with_capacity(triangles.len());
    for [a, b, c] in triangles {
        let pa = *vertices
            .get(a)
            .ok_or_else(|| format!("3MF triangle references missing vertex {a}"))?;
        let pb = *vertices
            .get(b)
            .ok_or_else(|| format!("3MF triangle references missing vertex {b}"))?;
        let pc = *vertices
            .get(c)
            .ok_or_else(|| format!("3MF triangle references missing vertex {c}"))?;
        tris.push([pa, pb, pc]);
    }

    if root.descendants().any(|n| n.has_tag_name("component")) {
        warnings.push(
            "3MF components were flattened into one triangle set; occurrence transforms are not preserved as separate bodies."
                .into(),
        );
    }

    Ok(ThreeMfImport {
        mesh: Mesh { tris },
        declared_unit: declared_unit.into(),
        warnings,
    })
}

fn attr_f32(node: roxmltree::Node<'_, '_>, name: &str) -> Result<f32, String> {
    node.attribute(name)
        .ok_or_else(|| format!("3MF element is missing attribute '{name}'"))?
        .parse::<f32>()
        .map_err(|_| format!("3MF attribute '{name}' is not a finite number"))
}

fn attr_usize(node: roxmltree::Node<'_, '_>, name: &str) -> Result<usize, String> {
    node.attribute(name)
        .ok_or_else(|| format!("3MF element is missing attribute '{name}'"))?
        .parse::<usize>()
        .map_err(|_| format!("3MF attribute '{name}' is not a non-negative integer"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn cuboid_3mf_bytes() -> Vec<u8> {
        let model = r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
          <vertex x="0" y="0" z="0"/>
          <vertex x="10" y="0" z="0"/>
          <vertex x="10" y="10" z="0"/>
          <vertex x="0" y="10" z="0"/>
          <vertex x="0" y="0" z="10"/>
          <vertex x="10" y="0" z="10"/>
          <vertex x="10" y="10" z="10"/>
          <vertex x="0" y="10" z="10"/>
        </vertices>
        <triangles>
          <triangle v1="0" v2="1" v3="2"/><triangle v1="0" v2="2" v3="3"/>
          <triangle v1="4" v2="6" v3="5"/><triangle v1="4" v2="7" v3="6"/>
          <triangle v1="0" v2="4" v3="5"/><triangle v1="0" v2="5" v3="1"/>
          <triangle v1="1" v2="5" v3="6"/><triangle v1="1" v2="6" v3="2"/>
          <triangle v1="2" v2="6" v3="7"/><triangle v1="2" v2="7" v3="3"/>
          <triangle v1="3" v2="7" v3="4"/><triangle v1="3" v2="4" v3="0"/>
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="1"/>
  </build>
</model>"#;
        let cursor = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("3D/3dmodel.model", options).unwrap();
        zip.write_all(model.as_bytes()).unwrap();
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn parses_millimetre_cuboid() {
        let imported = parse_3mf(&cuboid_3mf_bytes()).expect("3MF cuboid");
        assert_eq!(imported.declared_unit, "mm");
        assert_eq!(imported.mesh.tris.len(), 12);
        let diag = crate::cad::diagnose_mesh(&imported.mesh);
        assert_eq!(diag.boundary_edges, 0);
        assert_eq!(diag.components, 1);
    }

    #[test]
    fn rejects_empty_package() {
        assert!(parse_3mf(&[]).unwrap_err().contains("empty"));
    }
}
