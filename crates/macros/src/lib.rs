//! Procedural macros for `tauri-plugin-google-app-functions`. Use them through the re-exports
//! in that crate.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
  parse_macro_input, spanned::Spanned, Error, Fields, FnArg, ItemFn, ItemStruct, Pat, ReturnType,
  Type,
};

/// Registers a free function as an Android app function.
///
/// Parameters and the return value must be types supported by AppFunctions (see the crate
/// README). A parameter of type `tauri::AppHandle` is injected rather than exposed. The return
/// type may be `Result<T, E>` with `E: Into<AppFunctionError>`.
///
/// Doc comments become the descriptions agents read; document parameters in a `# Arguments`
/// section.
#[proc_macro_attribute]
pub fn app_function(attr: TokenStream, item: TokenStream) -> TokenStream {
  if !attr.is_empty() {
    return Error::new(
      TokenStream2::from(attr).span(),
      "#[app_function] takes no arguments",
    )
    .into_compile_error()
    .into();
  }
  let function = parse_macro_input!(item as ItemFn);
  expand_app_function(function)
    .unwrap_or_else(Error::into_compile_error)
    .into()
}

/// Derives the serde impls needed to pass a struct to or from an app function, and marks it
/// for the schema generator, which describes it as an AppFunctions object.
#[proc_macro_attribute]
pub fn app_function_serializable(attr: TokenStream, item: TokenStream) -> TokenStream {
  if !attr.is_empty() {
    return Error::new(
      TokenStream2::from(attr).span(),
      "#[app_function_serializable] takes no arguments",
    )
    .into_compile_error()
    .into();
  }
  let item = parse_macro_input!(item as ItemStruct);
  if !matches!(item.fields, Fields::Named(_)) {
    return Error::new(
      item.ident.span(),
      "#[app_function_serializable] requires a struct with named fields",
    )
    .into_compile_error()
    .into();
  }
  quote! {
    #[derive(
      ::tauri_plugin_google_app_functions::__private::serde::Serialize,
      ::tauri_plugin_google_app_functions::__private::serde::Deserialize,
    )]
    #[serde(
      crate = "::tauri_plugin_google_app_functions::__private::serde",
      rename_all = "camelCase"
    )]
    #item
  }
  .into()
}

fn expand_app_function(function: ItemFn) -> syn::Result<TokenStream2> {
  let sig = &function.sig;
  if !sig.generics.params.is_empty() || sig.generics.where_clause.is_some() {
    return Err(Error::new(
      sig.generics.span(),
      "app functions cannot be generic",
    ));
  }
  if let Some(variadic) = &sig.variadic {
    return Err(Error::new(variadic.span(), "app functions cannot be variadic"));
  }

  let mut fields = Vec::new();
  let mut field_names = Vec::new();
  let mut injected = Vec::new();
  let mut call_args = Vec::new();

  for input in &sig.inputs {
    let FnArg::Typed(input) = input else {
      return Err(Error::new(
        input.span(),
        "#[app_function] only supports free functions",
      ));
    };
    let Pat::Ident(pat) = &*input.pat else {
      return Err(Error::new(
        input.pat.span(),
        "app function parameters must be plain identifiers",
      ));
    };
    let ident = &pat.ident;
    let ty = &input.ty;
    if is_app_handle(ty) {
      injected.push(ident.clone());
    } else {
      fields.push(quote!(#ident: #ty));
      field_names.push(ident.clone());
    }
    call_args.push(ident.clone());
  }

  let fn_ident = &sig.ident;
  let name = fn_ident.to_string();
  let name = name.strip_prefix("r#").unwrap_or(&name);
  let awaited = sig.asyncness.map(|_| quote!(.await));
  let serialize = match &sig.output {
    ReturnType::Type(_, ty) if is_result(ty) => format_ident!("serialize_result"),
    _ => format_ident!("serialize_output"),
  };

  Ok(quote! {
    #function

    const _: () = {
      use ::tauri_plugin_google_app_functions::__private as __p;

      #[derive(__p::serde::Deserialize)]
      #[serde(
        crate = "::tauri_plugin_google_app_functions::__private::serde",
        rename_all = "camelCase"
      )]
      struct __Args {
        #(#fields,)*
      }

      fn __handler(args: __p::serde_json::Value) -> __p::BoxFuture {
        ::std::boxed::Box::pin(async move {
          let __Args { #(#field_names,)* } =
            __p::serde_json::from_value(args).map_err(__p::invalid_arguments)?;
          #(let #injected = ::tauri_plugin_google_app_functions::app_handle()?;)*
          let output = #fn_ident(#(#call_args),*) #awaited;
          __p::#serialize(output)
        })
      }

      __p::inventory::submit! {
        __p::AppFunctionEntry { name: #name, handler: __handler }
      }
    };
  })
}

fn last_segment(ty: &Type) -> Option<&syn::PathSegment> {
  match ty {
    Type::Path(path) if path.qself.is_none() => path.path.segments.last(),
    _ => None,
  }
}

fn is_app_handle(ty: &Type) -> bool {
  last_segment(ty).is_some_and(|segment| segment.ident == "AppHandle")
}

fn is_result(ty: &Type) -> bool {
  last_segment(ty).is_some_and(|segment| segment.ident == "Result")
}
