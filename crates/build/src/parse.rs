//! Finds `#[app_function]` functions and `#[app_function_serializable]` structs in the app's
//! Rust sources and converts them into a Kotlin-oriented model.

use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
  path::{Path, PathBuf},
};

use syn::{Attribute, Expr, FnArg, GenericArgument, Item, Lit, Pat, PathArguments, ReturnType, Type};

use crate::docs::Docs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
  Boolean,
  Int,
  Long,
  Float,
  Double,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KType {
  Primitive(Primitive),
  String,
  Nullable(Box<KType>),
  List(Box<KType>),
  PrimitiveArray(Primitive),
  Custom(String),
  Unit,
}

pub struct Param {
  /// JSON key and Kotlin parameter name.
  pub name: String,
  pub rust_name: String,
  pub ty: KType,
}

pub struct Function {
  pub rust_name: String,
  pub kotlin_name: String,
  pub docs: Docs,
  pub params: Vec<Param>,
  pub output: KType,
}

pub struct Field {
  pub name: String,
  pub docs: Docs,
  pub ty: KType,
}

pub struct Serializable {
  pub name: String,
  pub docs: Docs,
  pub fields: Vec<Field>,
}

#[derive(Default)]
pub struct Model {
  pub functions: Vec<Function>,
  pub serializables: Vec<Serializable>,
}

#[derive(Default)]
struct RawModel {
  functions: Vec<(PathBuf, syn::ItemFn)>,
  structs: Vec<(PathBuf, syn::ItemStruct)>,
}

pub fn parse_dirs(dirs: &[PathBuf]) -> Result<Model, Vec<String>> {
  let mut files = Vec::new();
  for dir in dirs {
    collect_rust_files(dir, &mut files);
  }
  files.sort();

  let mut raw = RawModel::default();
  let mut errors = Vec::new();
  for file in files {
    let source = match fs::read_to_string(&file) {
      Ok(source) => source,
      Err(e) => {
        errors.push(format!("{}: {e}", file.display()));
        continue;
      }
    };
    match syn::parse_file(&source) {
      Ok(parsed) => collect_items(&file, parsed.items, &mut raw),
      // rustc reports syntax errors far better than we could.
      Err(_) => continue,
    }
  }

  let known: BTreeSet<String> = raw.structs.iter().map(|(_, s)| s.ident.to_string()).collect();
  let mut model = Model::default();

  let mut seen_structs: BTreeMap<String, PathBuf> = BTreeMap::new();
  for (file, item) in raw.structs {
    if let Some(previous) = seen_structs.insert(item.ident.to_string(), file.clone()) {
      errors.push(format!(
        "{}: struct `{}` is also defined in {}; serializable struct names must be unique",
        file.display(),
        item.ident,
        previous.display()
      ));
      continue;
    }
    match convert_struct(&item, &known) {
      Ok(s) => model.serializables.push(s),
      Err(e) => errors.push(format!("{}: struct `{}`: {e}", file.display(), item.ident)),
    }
  }

  let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
  for (file, item) in raw.functions {
    match convert_function(&item, &known) {
      Ok(f) => {
        if let Some(previous) = seen.insert(f.kotlin_name.clone(), file.clone()) {
          errors.push(format!(
            "{}: app function `{}` is also defined in {}; app function names must be unique",
            file.display(),
            f.rust_name,
            previous.display()
          ));
        }
        model.functions.push(f);
      }
      Err(e) => errors.push(format!("{}: fn `{}`: {e}", file.display(), item.sig.ident)),
    }
  }

  if errors.is_empty() { Ok(model) } else { Err(errors) }
}

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
  let Ok(entries) = fs::read_dir(dir) else { return };
  for entry in entries.flatten() {
    let path = entry.path();
    if path.is_dir() {
      collect_rust_files(&path, out);
    } else if path.extension().is_some_and(|ext| ext == "rs") {
      out.push(path);
    }
  }
}

fn collect_items(file: &Path, items: Vec<Item>, raw: &mut RawModel) {
  for item in items {
    match item {
      Item::Fn(f) if has_attr(&f.attrs, "app_function") => raw.functions.push((file.to_owned(), f)),
      Item::Struct(s) if has_attr(&s.attrs, "app_function_serializable") => {
        raw.structs.push((file.to_owned(), s))
      }
      Item::Mod(m) => {
        if let Some((_, items)) = m.content {
          collect_items(file, items, raw);
        }
      }
      _ => {}
    }
  }
}

fn has_attr(attrs: &[Attribute], name: &str) -> bool {
  attrs
    .iter()
    .any(|attr| attr.path().segments.last().is_some_and(|s| s.ident == name))
}

pub fn doc_lines(attrs: &[Attribute]) -> Vec<String> {
  attrs
    .iter()
    .filter(|attr| attr.path().is_ident("doc"))
    .filter_map(|attr| match &attr.meta {
      syn::Meta::NameValue(nv) => match &nv.value {
        Expr::Lit(lit) => match &lit.lit {
          Lit::Str(s) => Some(s.value()),
          _ => None,
        },
        _ => None,
      },
      _ => None,
    })
    .flat_map(|doc| doc.lines().map(str::to_owned).collect::<Vec<_>>())
    .map(|line| line.strip_prefix(' ').map(str::to_owned).unwrap_or(line))
    .collect()
}

fn convert_struct(item: &syn::ItemStruct, known: &BTreeSet<String>) -> Result<Serializable, String> {
  if !item.generics.params.is_empty() {
    return Err("serializable structs cannot be generic".into());
  }
  let syn::Fields::Named(fields) = &item.fields else {
    return Err("serializable structs must have named fields".into());
  };
  let fields = fields
    .named
    .iter()
    .map(|field| {
      let rust_name = unraw(&field.ident.as_ref().expect("named field").to_string());
      let ty = convert_type(&field.ty, known).map_err(|e| format!("field `{rust_name}`: {e}"))?;
      if ty == KType::Unit {
        return Err(format!("field `{rust_name}`: `()` is not a valid field type"));
      }
      Ok(Field { name: camel_case(&rust_name), docs: Docs::parse(&doc_lines(&field.attrs)), ty })
    })
    .collect::<Result<Vec<_>, String>>()?;
  Ok(Serializable {
    name: item.ident.to_string(),
    docs: Docs::parse(&doc_lines(&item.attrs)),
    fields,
  })
}

fn convert_function(item: &syn::ItemFn, known: &BTreeSet<String>) -> Result<Function, String> {
  let mut params = Vec::new();
  for input in &item.sig.inputs {
    let FnArg::Typed(input) = input else {
      return Err("app functions must be free functions".into());
    };
    let Pat::Ident(pat) = &*input.pat else {
      return Err("parameters must be plain identifiers".into());
    };
    if last_ident(&input.ty).is_some_and(|ident| ident == "AppHandle") {
      continue;
    }
    let rust_name = unraw(&pat.ident.to_string());
    let ty = convert_type(&input.ty, known).map_err(|e| format!("parameter `{rust_name}`: {e}"))?;
    if ty == KType::Unit {
      return Err(format!("parameter `{rust_name}`: `()` is not a valid parameter type"));
    }
    params.push(Param { name: camel_case(&rust_name), rust_name, ty });
  }

  let output = match &item.sig.output {
    ReturnType::Default => KType::Unit,
    ReturnType::Type(_, ty) => {
      let ty = match result_ok_type(ty) {
        Some(ok) => ok,
        None => ty,
      };
      convert_type(ty, known).map_err(|e| format!("return type: {e}"))?
    }
  };

  let rust_name = unraw(&item.sig.ident.to_string());
  Ok(Function {
    kotlin_name: camel_case(&rust_name),
    rust_name,
    docs: Docs::parse(&doc_lines(&item.attrs)),
    params,
    output,
  })
}

fn unraw(ident: &str) -> String {
  ident.strip_prefix("r#").unwrap_or(ident).to_owned()
}

/// Same conversion as serde's `rename_all = "camelCase"`, so JSON keys match on both sides.
pub fn camel_case(snake: &str) -> String {
  let mut pascal = String::new();
  let mut capitalize = true;
  for ch in snake.chars() {
    if ch == '_' {
      capitalize = true;
    } else if capitalize {
      pascal.push(ch.to_ascii_uppercase());
      capitalize = false;
    } else {
      pascal.push(ch);
    }
  }
  match pascal.chars().next() {
    Some(first) => first.to_ascii_lowercase().to_string() + &pascal[first.len_utf8()..],
    None => pascal,
  }
}

fn last_ident(ty: &Type) -> Option<&syn::Ident> {
  match ty {
    Type::Path(path) if path.qself.is_none() => path.path.segments.last().map(|s| &s.ident),
    _ => None,
  }
}

fn generic_args(ty: &Type) -> Vec<&Type> {
  let Type::Path(path) = ty else { return Vec::new() };
  let Some(segment) = path.path.segments.last() else { return Vec::new() };
  let PathArguments::AngleBracketed(args) = &segment.arguments else { return Vec::new() };
  args
    .args
    .iter()
    .filter_map(|arg| match arg {
      GenericArgument::Type(ty) => Some(ty),
      _ => None,
    })
    .collect()
}

fn result_ok_type(ty: &Type) -> Option<&Type> {
  if last_ident(ty)? == "Result" { generic_args(ty).into_iter().next() } else { None }
}

fn single_arg<'a>(ty: &'a Type, name: &str) -> Result<&'a Type, String> {
  match generic_args(ty).as_slice() {
    [inner] => Ok(inner),
    _ => Err(format!("`{name}` must have exactly one type argument")),
  }
}

fn convert_type(ty: &Type, known: &BTreeSet<String>) -> Result<KType, String> {
  match ty {
    Type::Reference(r) => return convert_type(&r.elem, known),
    Type::Paren(p) => return convert_type(&p.elem, known),
    Type::Group(g) => return convert_type(&g.elem, known),
    Type::Tuple(t) if t.elems.is_empty() => return Ok(KType::Unit),
    _ => {}
  }
  let Some(ident) = last_ident(ty) else {
    return Err(format!("unsupported type `{}`", quote_type(ty)));
  };
  let ty = match ident.to_string().as_str() {
    "bool" => KType::Primitive(Primitive::Boolean),
    "i8" | "i16" | "i32" | "u8" | "u16" => KType::Primitive(Primitive::Int),
    "i64" | "u32" | "u64" | "isize" | "usize" => KType::Primitive(Primitive::Long),
    "f32" => KType::Primitive(Primitive::Float),
    "f64" => KType::Primitive(Primitive::Double),
    "String" | "str" => KType::String,
    "Option" => match convert_type(single_arg(ty, "Option")?, known)? {
      KType::Nullable(_) => return Err("nested `Option` is not supported".into()),
      KType::Unit => return Err("`Option<()>` is not supported".into()),
      inner => KType::Nullable(Box::new(inner)),
    },
    "Vec" => match convert_type(single_arg(ty, "Vec")?, known)? {
      KType::Primitive(p) => KType::PrimitiveArray(p),
      KType::Nullable(_) => return Err("`Vec<Option<_>>` is not supported by AppFunctions".into()),
      KType::Unit => return Err("`Vec<()>` is not supported".into()),
      inner => KType::List(Box::new(inner)),
    },
    name if known.contains(name) => KType::Custom(name.to_owned()),
    name => {
      return Err(format!(
        "unsupported type `{name}`; use a primitive, String, Option, Vec or a struct marked #[app_function_serializable]"
      ));
    }
  };
  Ok(ty)
}

fn quote_type(ty: &Type) -> String {
  match ty {
    Type::Path(p) => p
      .path
      .segments
      .iter()
      .map(|s| s.ident.to_string())
      .collect::<Vec<_>>()
      .join("::"),
    Type::Tuple(_) => "(..)".into(),
    Type::Slice(_) | Type::Array(_) => "[..]".into(),
    Type::ImplTrait(_) => "impl ..".into(),
    Type::TraitObject(_) => "dyn ..".into(),
    _ => "..".into(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn camel_case_matches_serde() {
    assert_eq!(camel_case("create_task"), "createTask");
    assert_eq!(camel_case("id"), "id");
    assert_eq!(camel_case("item_2_name"), "item2Name");
    assert_eq!(camel_case("already"), "already");
  }
}
