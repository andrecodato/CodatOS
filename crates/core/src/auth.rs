//! Tokens do núcleo: o do app (janela Tauri), os de uso único (abrir no navegador) e as sessões.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use subtle::ConstantTimeEq;

/// Validade do token de uso único do "Abrir no navegador".
const ONE_TIME_TTL: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub(crate) struct Auth {
    app_token: String,
    one_time: Mutex<HashMap<String, Instant>>,
    sessions: Mutex<HashSet<String>>,
}

impl Auth {
    pub(crate) fn new() -> Self {
        Self {
            app_token: random_token(),
            one_time: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashSet::new()),
        }
    }

    pub(crate) fn app_token(&self) -> &str {
        &self.app_token
    }

    pub(crate) fn is_app_token(&self, candidate: &str) -> bool {
        self.app_token.as_bytes().ct_eq(candidate.as_bytes()).into()
    }

    pub(crate) fn issue_one_time(&self) -> String {
        let token = random_token();
        let mut pending = lock(&self.one_time);
        let now = Instant::now();
        pending.retain(|_, expires| *expires > now);
        pending.insert(token.clone(), now + ONE_TIME_TTL);
        token
    }

    /// Troca um token de uso único por um id de sessão. O token deixa de valer na hora.
    pub(crate) fn redeem(&self, token: &str) -> Option<String> {
        let expires = lock(&self.one_time).remove(token)?;
        if expires <= Instant::now() {
            return None;
        }
        let session = random_token();
        lock(&self.sessions).insert(session.clone());
        Some(session)
    }

    pub(crate) fn has_session(&self, id: &str) -> bool {
        lock(&self.sessions).contains(id)
    }
}

/// 256 bits aleatórios do SO, em hexadecimal.
fn random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("gerador aleatório do SO indisponível");
    bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // Um pânico com o lock na mão não deixa estado inconsistente aqui (só conjuntos de tokens).
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_sao_hex_de_256_bits_e_distintos() {
        let a = random_token();
        assert_eq!(a.len(), 64);
        assert!(a.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_ne!(a, random_token());
    }

    #[test]
    fn token_de_uso_unico_vale_uma_vez() {
        let auth = Auth::new();
        let t = auth.issue_one_time();
        let session = auth.redeem(&t).expect("primeiro uso vale");
        assert!(auth.has_session(&session));
        assert!(auth.redeem(&t).is_none());
    }

    #[test]
    fn token_de_uso_unico_expirado_nao_vale() {
        let auth = Auth::new();
        let t = random_token();
        lock(&auth.one_time).insert(
            t.clone(),
            Instant::now().checked_sub(Duration::from_secs(1)).unwrap(),
        );
        assert!(auth.redeem(&t).is_none());
    }

    #[test]
    fn compara_token_do_app() {
        let auth = Auth::new();
        let token = auth.app_token().to_owned();
        assert!(auth.is_app_token(&token));
        assert!(!auth.is_app_token(&token[..63]));
        assert!(!auth.is_app_token(""));
    }
}
