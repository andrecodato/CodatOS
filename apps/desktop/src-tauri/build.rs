fn main() {
    // O CRT do MSVC que o cargo-xwin baixa (`just build-win`) vem sem os PDBs, e o lld-link
    // avisa LNK4099 para cada objeto dele. Só falta informação de debug do CRT: o binário
    // sai igual. Silencia esse aviso e deixa os outros do linker aparecerem.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg=/IGNORE:4099");
    }

    // Comandos do app que a janela pode chamar; cada um vira a permissão
    // `allow-<comando>` usada em capabilities/default.json (ADR-0003: IPC mínimo).
    let manifest = tauri_build::AppManifest::new().commands(&["get_session_token"]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("falha no build do Tauri");
}
