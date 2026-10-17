fn main() {
    // Comandos do app que a janela pode chamar; cada um vira a permissão
    // `allow-<comando>` usada em capabilities/default.json (ADR-0003: IPC mínimo).
    let manifest = tauri_build::AppManifest::new().commands(&["get_session_token"]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("falha no build do Tauri");
}
