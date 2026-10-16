//! Checagem de `Host` e `Origin` em toda requisição (Segurança §8): protege a API local
//! contra DNS rebinding e contra sites abertos no navegador.

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::server::problem_response;

/// Origens da janela Tauri: `tauri://localhost` no macOS/Linux, `http(s)://tauri.localhost` no Windows.
pub(crate) const TAURI_ORIGINS: [&str; 3] = [
    "tauri://localhost",
    "http://tauri.localhost",
    "https://tauri.localhost",
];

#[derive(Debug)]
pub(crate) struct Allowed {
    hosts: HashSet<String>,
    origins: HashSet<String>,
}

impl Allowed {
    pub(crate) fn new(port: u16, extra_origins: &[String]) -> Self {
        let hosts: HashSet<String> =
            [format!("127.0.0.1:{port}"), format!("localhost:{port}")].into();
        let origins = hosts
            .iter()
            .map(|h| format!("http://{h}"))
            .chain(TAURI_ORIGINS.iter().map(|o| (*o).to_owned()))
            .chain(extra_origins.iter().cloned())
            .collect();
        Self { hosts, origins }
    }

    pub(crate) fn check(&self, headers: &HeaderMap) -> Result<(), &'static str> {
        let host = headers
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
            .ok_or("host_invalid")?;
        if !self.hosts.contains(&host.to_ascii_lowercase()) {
            return Err("host_invalid");
        }
        // Clientes fora do navegador não mandam Origin; o navegador sempre manda em WebSocket e POST.
        if let Some(origin) = headers.get(header::ORIGIN) {
            let origin = origin.to_str().map_err(|_| "origin_invalid")?;
            if !self.origins.contains(origin) {
                return Err("origin_invalid");
            }
        }
        Ok(())
    }
}

pub(crate) async fn host_and_origin(
    State(allowed): State<Arc<Allowed>>,
    request: Request,
    next: Next,
) -> Response {
    match allowed.check(request.headers()) {
        Ok(()) => next.run(request).await,
        Err(code) => {
            tracing::warn!(code, path = %request.uri().path(), "requisição recusada");
            problem_response(StatusCode::FORBIDDEN, code, "Origem não permitida").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(host: Option<&str>, origin: Option<&str>) -> HeaderMap {
        let mut h = HeaderMap::new();
        if let Some(v) = host {
            h.insert(header::HOST, HeaderValue::from_str(v).unwrap());
        }
        if let Some(v) = origin {
            h.insert(header::ORIGIN, HeaderValue::from_str(v).unwrap());
        }
        h
    }

    #[test]
    fn aceita_host_local_e_origens_do_app() {
        let a = Allowed::new(47800, &[]);
        assert!(a.check(&headers(Some("127.0.0.1:47800"), None)).is_ok());
        assert!(a.check(&headers(Some("LOCALHOST:47800"), None)).is_ok());
        for origin in TAURI_ORIGINS {
            assert!(
                a.check(&headers(Some("127.0.0.1:47800"), Some(origin)))
                    .is_ok(),
                "{origin}"
            );
        }
        assert!(
            a.check(&headers(
                Some("127.0.0.1:47800"),
                Some("http://127.0.0.1:47800")
            ))
            .is_ok()
        );
    }

    #[test]
    fn recusa_dns_rebinding_e_sites_estranhos() {
        let a = Allowed::new(47800, &[]);
        assert_eq!(
            a.check(&headers(Some("evil.example:47800"), None)),
            Err("host_invalid")
        );
        assert_eq!(
            a.check(&headers(Some("127.0.0.1:1234"), None)),
            Err("host_invalid")
        );
        assert_eq!(a.check(&headers(None, None)), Err("host_invalid"));
        assert_eq!(
            a.check(&headers(
                Some("127.0.0.1:47800"),
                Some("https://evil.example")
            )),
            Err("origin_invalid")
        );
        assert_eq!(
            a.check(&headers(Some("127.0.0.1:47800"), Some("null"))),
            Err("origin_invalid")
        );
    }

    #[test]
    fn origens_extras_valem_so_quando_configuradas() {
        let dev = "http://127.0.0.1:1420".to_owned();
        assert!(
            Allowed::new(47800, &[])
                .check(&headers(Some("127.0.0.1:47800"), Some(&dev)))
                .is_err()
        );
        assert!(
            Allowed::new(47800, std::slice::from_ref(&dev))
                .check(&headers(Some("127.0.0.1:47800"), Some(&dev)))
                .is_ok()
        );
    }
}
