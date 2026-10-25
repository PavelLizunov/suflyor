//! Dumps the public API of the exported Slint windows and globals as JSON.
//!
//! The Omarchy plugin build compiles the host logic against generated remote
//! windows with the same Rust API as the Slint-generated ones; this schema is
//! the input of that generator. Usage:
//! `slint-remote-schema slint-experiment/ui/index.slint > remote-ui.json`

use i_slint_compiler::diagnostics::BuildDiagnostics;
use i_slint_compiler::generator::OutputFormat;
use i_slint_compiler::langtype::{Struct, StructName, Type};
use i_slint_compiler::object_tree::{Component, PropertyVisibility};
use i_slint_compiler::CompilerConfiguration;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

fn main() {
    let path = PathBuf::from(std::env::args().nth(1).expect("path to index.slint"));
    let source = std::fs::read_to_string(&path).expect("read index.slint");
    let mut config = CompilerConfiguration::new(OutputFormat::Interpreter);
    // Same style slint-build picks on Linux when SLINT_STYLE is unset.
    config.style = Some("fluent".into());
    let (path, diag, loader) = spin_on::spin_on(i_slint_compiler::load_root_file(
        &path,
        &path,
        source,
        BuildDiagnostics::default(),
        config,
    ));
    if diag.has_errors() {
        for line in diag.to_string_vec() {
            eprintln!("{line}");
        }
        std::process::exit(1);
    }
    let doc = loader.get_document(&path).expect("compiled document");

    let mut types = BTreeMap::new();
    let mut windows = Map::new();
    for component in doc.exported_roots() {
        windows.insert(component.id.to_string(), api(&component, &mut types));
    }
    let mut globals = Map::new();
    for (_, export) in doc.exports.iter() {
        if let Some(component) = export.as_ref().left() {
            if component.is_global() {
                globals.insert(component.id.to_string(), api(component, &mut types));
            }
        }
    }
    let out = json!({ "windows": windows, "globals": globals, "types": types });
    println!("{}", serde_json::to_string_pretty(&out).expect("serialize"));
}

fn api(component: &Rc<Component>, types: &mut BTreeMap<String, Value>) -> Value {
    let root = component.root_element.borrow();
    let mut properties = Map::new();
    let mut callbacks = Map::new();
    let mut functions = Map::new();
    for (name, decl) in &root.property_declarations {
        if !decl.expose_in_public_api {
            continue;
        }
        match &decl.property_type {
            Type::Callback(f) => {
                callbacks.insert(
                    name.to_string(),
                    signature(f.args.as_slice(), &f.arg_names, &f.return_type, types),
                );
            }
            Type::Function(f) => {
                if decl.visibility == PropertyVisibility::Public {
                    functions.insert(
                        name.to_string(),
                        signature(f.args.as_slice(), &f.arg_names, &f.return_type, types),
                    );
                }
            }
            ty => {
                let direction = match decl.visibility {
                    PropertyVisibility::Input => "in",
                    PropertyVisibility::Output => "out",
                    PropertyVisibility::InOut => "in-out",
                    _ => continue,
                };
                properties.insert(
                    name.to_string(),
                    json!({ "dir": direction, "type": ty_json(ty, types) }),
                );
            }
        }
    }
    json!({ "properties": properties, "callbacks": callbacks, "functions": functions })
}

fn signature(
    args: &[Type],
    names: &[impl std::fmt::Display],
    ret: &Type,
    types: &mut BTreeMap<String, Value>,
) -> Value {
    let args: Vec<Value> = args
        .iter()
        .enumerate()
        .map(|(i, t)| json!({ "name": names.get(i).map(|n| n.to_string()).unwrap_or_default(), "type": ty_json(t, types) }))
        .collect();
    json!({ "args": args, "ret": ty_json(ret, types) })
}

fn ty_json(ty: &Type, types: &mut BTreeMap<String, Value>) -> Value {
    match ty {
        Type::Array(inner) => json!({ "array": ty_json(inner, types) }),
        Type::Struct(s) => match &s.name {
            StructName::User { name, .. } => {
                if !types.contains_key(name.as_str()) {
                    types.insert(name.to_string(), Value::Null);
                    let fields = struct_fields(s, types);
                    types.insert(name.to_string(), json!({ "struct": fields }));
                }
                json!({ "ref": name.as_str() })
            }
            StructName::Builtin(_) => {
                json!({ "builtin-struct": ty.to_string(), "fields": struct_fields(s, types) })
            }
            StructName::None => json!({ "anon-struct": struct_fields(s, types) }),
        },
        Type::Enumeration(e) => {
            types.entry(e.name.to_string()).or_insert_with(
                || json!({ "enum": e.values.iter().map(|v| v.to_string()).collect::<Vec<_>>() }),
            );
            json!({ "ref": e.name.as_str() })
        }
        other => json!({ "prim": other.to_string() }),
    }
}

fn struct_fields(s: &Struct, types: &mut BTreeMap<String, Value>) -> Map<String, Value> {
    s.fields
        .iter()
        .map(|(k, v)| (k.to_string(), ty_json(v, types)))
        .collect()
}
