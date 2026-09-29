//! Renders the files the Android side reads: the AppFunctions XML schema the system indexes,
//! the type table `TauriAppFunctionService` converts arguments and results with, and the app
//! description.

use std::{collections::BTreeMap, fmt::Write};

use crate::parse::{Function, KType, Model, Primitive, Serializable};

/// Package of the plugin's Android library. Function IDs and object names are qualified with
/// it, the way KSP qualifies them with the Kotlin package.
const PACKAGE: &str = "com.plugin.google_app_functions";
/// The service registered in the plugin's AndroidManifest.xml.
pub const SERVICE: &str = "com.plugin.google_app_functions.TauriAppFunctionService";
/// Asset paths inside `app/src/main/assets`. The plugin's AndroidManifest.xml and
/// `TauriAppFunctionService` reference them. `generated` directories are git-ignored by the
/// Tauri Android template.
pub const SCHEMA_ASSET: &str = "generated/tauri_app_functions.xml";
pub const TYPES_ASSET: &str = "generated/tauri_app_functions.json";

// AppFunctionDataTypeMetadata.TYPE_* in androidx.appfunctions.
const TYPE_UNIT: u8 = 0;
const TYPE_BOOLEAN: u8 = 1;
const TYPE_OBJECT: u8 = 3;
const TYPE_DOUBLE: u8 = 4;
const TYPE_FLOAT: u8 = 5;
const TYPE_LONG: u8 = 6;
const TYPE_INT: u8 = 7;
const TYPE_STRING: u8 = 8;
const TYPE_ARRAY: u8 = 10;
const TYPE_REFERENCE: u8 = 11;

pub fn function_id(f: &Function) -> String {
  format!("{SERVICE}#{}", f.name)
}

fn object_name(name: &str) -> String {
  format!("{PACKAGE}.{name}")
}

fn description(lines: &[String]) -> String {
  lines
    .iter()
    .map(|line| line.trim_end())
    .collect::<Vec<_>>()
    .join("\n")
}

/// Structs reachable from the functions, keyed by qualified name (KSP writes them sorted).
fn referenced_objects(model: &Model) -> BTreeMap<String, &Serializable> {
  fn visit<'a>(ty: &KType, model: &'a Model, out: &mut BTreeMap<String, &'a Serializable>) {
    match ty {
      KType::Nullable(inner) | KType::List(inner) => visit(inner, model, out),
      KType::Custom(name) => {
        let Some(s) = model.serializables.iter().find(|s| &s.name == name) else {
          return;
        };
        if out.insert(object_name(name), s).is_none() {
          for field in &s.fields {
            visit(&field.ty, model, out);
          }
        }
      }
      _ => {}
    }
  }
  let mut out = BTreeMap::new();
  for f in &model.functions {
    for p in &f.params {
      visit(&p.ty, model, &mut out);
    }
    visit(&f.output, model, &mut out);
  }
  out
}

// ---------------------------------------------------------------------------------------------
// XML schema, in the exact layout the androidx.appfunctions KSP compiler writes it.

struct Xml {
  out: String,
  depth: usize,
}

impl Xml {
  fn open(&mut self, tag: &str) {
    self.indent();
    let _ = writeln!(self.out, "<{tag}>");
    self.depth += 1;
  }

  fn close(&mut self, tag: &str) {
    self.depth -= 1;
    self.indent();
    let _ = writeln!(self.out, "</{tag}>");
  }

  fn text(&mut self, tag: &str, text: &str) {
    self.indent();
    let _ = writeln!(self.out, "<{tag}>{}</{tag}>", escape_text(text));
  }

  fn text_if_any(&mut self, tag: &str, text: &str) {
    if !text.is_empty() {
      self.text(tag, text);
    }
  }

  fn indent(&mut self) {
    self.out.push_str(&"    ".repeat(self.depth));
  }
}

fn escape_text(text: &str) -> String {
  let mut out = String::with_capacity(text.len());
  for ch in text.chars() {
    match ch {
      '&' => out.push_str("&amp;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '\r' => out.push_str("&#13;"),
      c => out.push(c),
    }
  }
  out
}

fn primitive_type(p: Primitive) -> u8 {
  match p {
    Primitive::Boolean => TYPE_BOOLEAN,
    Primitive::Int => TYPE_INT,
    Primitive::Long => TYPE_LONG,
    Primitive::Float => TYPE_FLOAT,
    Primitive::Double => TYPE_DOUBLE,
  }
}

/// Writes an `AppFunctionDataTypeMetadata` element describing `ty`.
fn data_type(xml: &mut Xml, tag: &str, ty: &KType, description: &str) {
  let (ty, nullable) = match ty {
    KType::Nullable(inner) => (&**inner, true),
    ty => (ty, false),
  };
  xml.open(tag);
  if let KType::Custom(name) = ty {
    xml.text("dataTypeReference", &object_name(name));
  }
  xml.text("id", "unused");
  xml.text("isNullable", &nullable.to_string());
  match ty {
    KType::List(item) => data_type(xml, "itemType", item, ""),
    KType::PrimitiveArray(p) => data_type(xml, "itemType", &KType::Primitive(*p), ""),
    _ => {}
  }
  xml.text_if_any("description", description);
  let code = match ty {
    KType::Primitive(p) => primitive_type(*p),
    KType::String => TYPE_STRING,
    KType::List(_) | KType::PrimitiveArray(_) => TYPE_ARRAY,
    KType::Custom(_) => TYPE_REFERENCE,
    KType::Unit => TYPE_UNIT,
    KType::Nullable(_) => unreachable!("nested Option is rejected by the parser"),
  };
  xml.text("type", &code.to_string());
  xml.close(tag);
}

pub fn render_schema(model: &Model) -> String {
  let mut xml = Xml {
    out: String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n"),
    depth: 0,
  };
  let objects = referenced_objects(model);
  if model.functions.is_empty() {
    xml.out.push_str("<appfunctions/>\n");
    return xml.out;
  }

  xml.open("appfunctions");
  for f in &model.functions {
    let id = function_id(f);
    xml.open("appfunction");
    xml.text("id", &id);
    xml.text("enabledByDefault", "true");
    xml.text_if_any("description", &description(&f.docs.description));
    for p in &f.params {
      xml.open("parameters");
      data_type(&mut xml, "dataTypeMetadata", &p.ty, "");
      xml.text("id", "unused");
      let required = !matches!(p.ty, KType::Nullable(_));
      xml.text("isRequired", &required.to_string());
      xml.text("name", &p.name);
      xml.text_if_any(
        "description",
        f.docs.param(&p.rust_name).unwrap_or_default(),
      );
      xml.close("parameters");
    }
    xml.open("response");
    xml.text("id", "unused");
    data_type(&mut xml, "valueType", &f.output, "");
    xml.text_if_any("description", f.docs.returns.as_deref().unwrap_or_default());
    xml.close("response");
    xml.text("functionId", &id);
    xml.close("appfunction");
  }

  if !objects.is_empty() {
    xml.open("AppFunctionComponentMetadataDocument");
    for (name, s) in &objects {
      xml.open("dataTypes");
      xml.open("dataTypeMetadata");
      xml.text("id", "unused");
      xml.text("isNullable", "true");
      xml.text("objectQualifiedName", name);
      xml.text_if_any("description", &description(&s.docs.description));
      for field in &s.fields {
        xml.open("properties");
        data_type(
          &mut xml,
          "dataTypeMetadata",
          &field.ty,
          &description(&field.docs.description),
        );
        xml.text("id", "unused");
        xml.text("name", &field.name);
        xml.close("properties");
      }
      for field in &s.fields {
        xml.text("required", &field.name);
      }
      xml.text("type", &TYPE_OBJECT.to_string());
      xml.close("dataTypeMetadata");
      xml.text("id", "unused");
      xml.text("name", name);
      xml.close("dataTypes");
    }
    xml.text("id", SERVICE);
    xml.close("AppFunctionComponentMetadataDocument");
  }
  xml.close("appfunctions");
  xml.out
}

// ---------------------------------------------------------------------------------------------
// Type table for TauriAppFunctionService.
//
// {"library": "...",
//  "functions": {"<function id>": {"name": "<rust name>", "params": [["title", "string"]], "returns": "@<object>"}},
//  "objects": {"<object>": [["city", "string"], ["street", "string?"]]}}
//
// A type is a base (`boolean`, `int`, `long`, `float`, `double`, `string`, `unit` or `@` plus
// an object name), then `[]` for a list, then `?` when nullable.

fn type_code(ty: &KType) -> String {
  match ty {
    KType::Primitive(p) => primitive_code(*p).to_owned(),
    KType::String => "string".to_owned(),
    KType::Unit => "unit".to_owned(),
    KType::Custom(name) => format!("@{}", object_name(name)),
    KType::List(item) => format!("{}[]", type_code(item)),
    KType::PrimitiveArray(p) => format!("{}[]", primitive_code(*p)),
    KType::Nullable(inner) => format!("{}?", type_code(inner)),
  }
}

fn primitive_code(p: Primitive) -> &'static str {
  match p {
    Primitive::Boolean => "boolean",
    Primitive::Int => "int",
    Primitive::Long => "long",
    Primitive::Float => "float",
    Primitive::Double => "double",
  }
}

fn json_string(value: &str) -> String {
  let mut out = String::from("\"");
  for ch in value.chars() {
    match ch {
      '"' => out.push_str("\\\""),
      '\\' => out.push_str("\\\\"),
      c if (c as u32) < 0x20 => {
        let _ = write!(out, "\\u{:04x}", c as u32);
      }
      c => out.push(c),
    }
  }
  out.push('"');
  out
}

fn json_members<'a>(members: impl Iterator<Item = (&'a str, &'a KType)>) -> String {
  let members: Vec<String> = members
    .map(|(name, ty)| format!("[{}, {}]", json_string(name), json_string(&type_code(ty))))
    .collect();
  format!("[{}]", members.join(", "))
}

pub fn render_types(model: &Model, library: &str) -> String {
  let mut out = String::from("{\n");
  let _ = writeln!(out, "  \"library\": {},", json_string(library));

  out.push_str("  \"functions\": {");
  let functions: Vec<String> = model
    .functions
    .iter()
    .map(|f| {
      format!(
        "\n    {}: {{\"name\": {}, \"params\": {}, \"returns\": {}}}",
        json_string(&function_id(f)),
        json_string(&f.rust_name),
        json_members(f.params.iter().map(|p| (p.name.as_str(), &p.ty))),
        json_string(&type_code(&f.output)),
      )
    })
    .collect();
  out.push_str(&functions.join(","));
  out.push_str(if functions.is_empty() {
    "},\n"
  } else {
    "\n  },\n"
  });

  out.push_str("  \"objects\": {");
  let objects = referenced_objects(model);
  let rendered: Vec<String> = objects
    .iter()
    .map(|(name, s)| {
      format!(
        "\n    {}: {}",
        json_string(name),
        json_members(s.fields.iter().map(|f| (f.name.as_str(), &f.ty))),
      )
    })
    .collect();
  out.push_str(&rendered.join(","));
  out.push_str(if rendered.is_empty() {
    "}\n"
  } else {
    "\n  }\n"
  });
  out.push_str("}\n");
  out
}

/// `res/xml` document holding the app description agents read before choosing a function.
pub fn render_app_metadata(description: &str) -> String {
  let escaped = description
    .replace('&', "&amp;")
    .replace('"', "&quot;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('\n', "&#10;");
  format!(
    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
     <!-- THIS FILE IS AUTO-GENERATED BY tauri-plugin-google-app-functions-build. DO NOT EDIT. -->\n\
     <AppFunctionAppMetadata xmlns:appfn=\"http://schemas.android.com/apk/androidx.appfunctions\"\n    \
     appfn:description=\"{escaped}\" />\n"
  )
}
