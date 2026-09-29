`ksp_tauri_app_functions.xml` is the schema the `androidx.appfunctions` KSP compiler
(1.0.0-alpha12) generated for the Kotlin equivalent of the app functions in `ksp_src/`.
`tests/generate.rs` checks that this crate produces the same schema from those Rust sources.
