//! UI para o navegador (RF-APP "Abrir no navegador"): o núcleo serve arquivos de uma pasta,
//! e a UI está embutida no executável. Na subida, ela é copiada para
//! `<dados do app>/ui/<versão>/` (uma vez por versão) e essa pasta vai para o núcleo.

use std::path::{Component, Path, PathBuf};

use tauri::{AppHandle, Manager};

/// Copia a UI embutida e devolve a pasta; `None` se não houver UI embutida (dev) ou se
/// a cópia falhar. Sem ela, o app funciona; só o navegador fica sem página.
pub fn extract(app: &AppHandle) -> Option<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .ok()?
        .join("ui")
        .join(app.package_info().version.to_string());
    if dir.join("index.html").is_file() {
        return Some(dir);
    }
    let resolver = app.asset_resolver();
    let keys: Vec<String> = resolver.iter().map(|(key, _)| key.into_owned()).collect();
    if keys.is_empty() {
        return None;
    }
    // Copia para uma pasta temporária e renomeia no fim: uma cópia pela metade nunca é usada.
    let partial = dir.with_extension("partial");
    let _ = std::fs::remove_dir_all(&partial);
    for key in keys {
        let Some(relative) = safe_relative(&key) else {
            continue;
        };
        // `get` devolve o conteúdo já descomprimido.
        let Some(asset) = resolver.get(key.clone()) else {
            continue;
        };
        let target = partial.join(relative);
        let written = target
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&target, asset.bytes()));
        if let Err(error) = written {
            tracing::warn!(%error, %key, "não consegui copiar a UI para o navegador");
            return None;
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&partial, &dir).ok()?;
    Some(dir)
}

/// Caminho relativo sem `..` nem raiz, a partir de uma chave como `/assets/app.js`.
fn safe_relative(key: &str) -> Option<PathBuf> {
    let path = Path::new(key.trim_start_matches('/'));
    path.components()
        .all(|c| matches!(c, Component::Normal(_)))
        .then(|| path.to_path_buf())
        .filter(|p| !p.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use super::safe_relative;

    #[test]
    fn so_caminhos_dentro_da_pasta() {
        assert_eq!(
            safe_relative("/assets/app.js").as_deref(),
            Some(std::path::Path::new("assets/app.js"))
        );
        assert_eq!(safe_relative("/../fora"), None);
        assert_eq!(safe_relative("/"), None);
    }
}
