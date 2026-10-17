//! Cliente HTTP compartilhado para APIs externas (clima).

use std::sync::OnceLock;
use std::time::Duration;

/// Um cliente só para o app inteiro (reaproveita conexões e a configuração de TLS).
pub(crate) fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(concat!("CodatOS/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(10))
            .build()
            .expect("configuração fixa do cliente HTTP")
    })
}
