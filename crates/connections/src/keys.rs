//! Chaves SSH (RF-CON-011): gerar, importar (OpenSSH e PuTTY) e descrever.

use codatos_protocol::connections::KeyAlgorithm;
use getrandom::SysRng;
use getrandom::rand_core::UnwrapErr;
use ssh_key::{Algorithm, EcdsaCurve, HashAlg, LineEnding, PrivateKey};
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyError {
    /// A chave está cifrada e o pedido não trouxe passphrase.
    PassphraseRequired,
    WrongPassphrase,
    /// PEM antigo (PKCS#1, PKCS#8, SEC1): converter com `ssh-keygen -p -f chave`.
    UnsupportedFormat,
    Invalid,
}

/// Gera uma chave nova. RSA de 4096 bits leva alguns segundos: chamar fora do runtime.
pub fn generate(algorithm: KeyAlgorithm, comment: &str) -> Result<PrivateKey, KeyError> {
    let algorithm = match algorithm {
        KeyAlgorithm::Ed25519 => Algorithm::Ed25519,
        KeyAlgorithm::EcdsaP256 => Algorithm::Ecdsa {
            curve: EcdsaCurve::NistP256,
        },
        KeyAlgorithm::EcdsaP384 => Algorithm::Ecdsa {
            curve: EcdsaCurve::NistP384,
        },
        KeyAlgorithm::Rsa4096 => Algorithm::Rsa { hash: None },
    };
    let mut key =
        PrivateKey::random(&mut UnwrapErr(SysRng), algorithm).map_err(|_| KeyError::Invalid)?;
    key.set_comment(comment);
    Ok(key)
}

/// Lê uma chave privada OpenSSH (`BEGIN OPENSSH PRIVATE KEY`) ou PuTTY (`.ppk` v2 e v3) e
/// a devolve já decifrada da passphrase.
pub fn import(text: &str, passphrase: Option<&str>) -> Result<PrivateKey, KeyError> {
    let text = text.trim();
    let passphrase = passphrase.filter(|p| !p.is_empty());
    if text.starts_with("PuTTY-User-Key-File-") {
        let encrypted = !text
            .lines()
            .any(|l| l.trim().eq_ignore_ascii_case("Encryption: none"));
        if encrypted && passphrase.is_none() {
            return Err(KeyError::PassphraseRequired);
        }
        return PrivateKey::from_ppk(text, passphrase.map(str::to_owned)).map_err(|_| {
            if encrypted {
                KeyError::WrongPassphrase
            } else {
                KeyError::Invalid
            }
        });
    }
    if text.contains("-----BEGIN OPENSSH PRIVATE KEY-----") {
        let key = PrivateKey::from_openssh(text).map_err(|_| KeyError::Invalid)?;
        if !key.is_encrypted() {
            return Ok(key);
        }
        let passphrase = passphrase.ok_or(KeyError::PassphraseRequired)?;
        return key
            .decrypt(passphrase)
            .map_err(|_| KeyError::WrongPassphrase);
    }
    let legacy = ["RSA ", "EC ", "DSA ", "ENCRYPTED ", ""]
        .iter()
        .any(|kind| text.contains(&format!("-----BEGIN {kind}PRIVATE KEY-----")));
    if legacy {
        return Err(KeyError::UnsupportedFormat);
    }
    Err(KeyError::Invalid)
}

/// Chave privada em formato OpenSSH, sem passphrase, para selar no cofre.
pub fn to_openssh(key: &PrivateKey) -> Result<Zeroizing<String>, KeyError> {
    key.to_openssh(LineEnding::LF)
        .map_err(|_| KeyError::Invalid)
}

/// Algoritmo, linha `authorized_keys` e fingerprint `SHA256:…`.
pub fn describe(key: &PrivateKey) -> Result<(String, String, String), KeyError> {
    let public = key.public_key();
    Ok((
        public.algorithm().as_str().to_owned(),
        public.to_openssh().map_err(|_| KeyError::Invalid)?,
        public.fingerprint(HashAlg::Sha256).to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gera_e_reimporta() {
        for alg in [
            KeyAlgorithm::Ed25519,
            KeyAlgorithm::EcdsaP256,
            KeyAlgorithm::EcdsaP384,
        ] {
            let key = generate(alg, "andre@pc").unwrap();
            let (algorithm, public, fingerprint) = describe(&key).unwrap();
            assert!(public.starts_with(&algorithm));
            assert!(public.ends_with(" andre@pc"));
            assert!(fingerprint.starts_with("SHA256:"));
            let again = import(&to_openssh(&key).unwrap(), None).unwrap();
            assert_eq!(describe(&again).unwrap().2, fingerprint);
        }
    }

    #[test]
    fn passphrase() {
        let key = generate(KeyAlgorithm::Ed25519, "").unwrap();
        let fingerprint = describe(&key).unwrap().2;
        let encrypted = key
            .encrypt(&mut UnwrapErr(SysRng), "frase secreta")
            .unwrap()
            .to_openssh(LineEnding::LF)
            .unwrap();
        assert_eq!(
            import(&encrypted, None).unwrap_err(),
            KeyError::PassphraseRequired
        );
        assert_eq!(
            import(&encrypted, Some("")).unwrap_err(),
            KeyError::PassphraseRequired
        );
        assert_eq!(
            import(&encrypted, Some("errada")).unwrap_err(),
            KeyError::WrongPassphrase
        );
        let key = import(&encrypted, Some("frase secreta")).unwrap();
        assert!(!key.is_encrypted());
        assert_eq!(describe(&key).unwrap().2, fingerprint);
    }

    #[test]
    fn formatos_nao_suportados() {
        let pem = "-----BEGIN RSA PRIVATE KEY-----\nMIIE\n-----END RSA PRIVATE KEY-----";
        assert_eq!(import(pem, None).unwrap_err(), KeyError::UnsupportedFormat);
        let pkcs8 = "-----BEGIN PRIVATE KEY-----\nMC4C\n-----END PRIVATE KEY-----";
        assert_eq!(
            import(pkcs8, None).unwrap_err(),
            KeyError::UnsupportedFormat
        );
        assert_eq!(
            import("ssh-ed25519 AAAA", None).unwrap_err(),
            KeyError::Invalid
        );
    }

    /// Chaves PuTTY v3 dos exemplos do próprio `ssh-key` (a cifrada usa a senha `123`).
    #[test]
    fn putty() {
        let plain = import(include_str!("../tests/ed25519.ppk"), None).unwrap();
        assert_eq!(plain.algorithm(), Algorithm::Ed25519);
        let enc = include_str!("../tests/ed25519_enc.ppk");
        assert_eq!(import(enc, None).unwrap_err(), KeyError::PassphraseRequired);
        assert_eq!(
            import(enc, Some("errada")).unwrap_err(),
            KeyError::WrongPassphrase
        );
        let key = import(enc, Some("123")).unwrap();
        assert_eq!(describe(&key).unwrap().2, describe(&plain).unwrap().2);
    }
}
