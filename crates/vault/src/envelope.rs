//! Envelope cifrado v1 (§4): XChaCha20-Poly1305, nonce aleatório de 192 bits, AAD com o item
//! e o cofre.
//!
//! ```json
//! {"v": 1, "alg": "XC20P", "kid": "vault:01J9Z…:k1", "nonce": "base64", "ct": "base64"}
//! ```

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{Error, Key};

/// Versão do formato do envelope.
const VERSION: u8 = 1;
const ALG: &str = "XC20P";

/// Limite por item (R-01): 64 KiB de texto claro.
pub const MAX_ITEM: usize = 64 * 1024;

/// Item cifrado, pronto para gravar em JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u8,
    pub alg: String,
    /// Qual chave cifrou (`vault:<id>:k<versão>`), para a rotação de chaves do M6.
    pub kid: String,
    #[serde(with = "b64")]
    pub nonce: Vec<u8>,
    /// Texto cifrado seguido da tag de 16 bytes.
    #[serde(with = "b64")]
    pub ct: Vec<u8>,
}

/// Dados associados: amarram o envelope ao item e ao cofre, para que ninguém (nem a nuvem,
/// no M4) troque itens entre si ou mova um item para outro cofre sem ser detectado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aad<'a> {
    pub item_id: &'a str,
    pub vault_id: &'a str,
}

impl Aad<'_> {
    /// `codatos ‖ v ‖ len ‖ item_id ‖ len ‖ vault_id`, com comprimentos de 4 bytes para que
    /// `("ab", "c")` e `("a", "bc")` não colidam.
    fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(16 + self.item_id.len() + self.vault_id.len());
        out.extend_from_slice(b"codatos");
        out.push(VERSION);
        for part in [self.item_id, self.vault_id] {
            let len = u32::try_from(part.len()).expect("ids curtos");
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(part.as_bytes());
        }
        out
    }
}

impl Key {
    /// Cifra `plaintext` com um nonce novo.
    pub fn seal(&self, kid: &str, aad: Aad<'_>, plaintext: &[u8]) -> Result<Envelope, Error> {
        self.seal_with_nonce(kid, aad, plaintext, crate::random::<24>())
    }

    fn seal_with_nonce(
        &self,
        kid: &str,
        aad: Aad<'_>,
        plaintext: &[u8],
        nonce: [u8; 24],
    ) -> Result<Envelope, Error> {
        if plaintext.len() > MAX_ITEM {
            return Err(Error::TooLarge(plaintext.len()));
        }
        let ct = XChaCha20Poly1305::new(self.as_bytes().into())
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad: &aad.bytes(),
                },
            )
            .expect("XChaCha20-Poly1305 não falha ao cifrar");
        Ok(Envelope {
            v: VERSION,
            alg: ALG.to_owned(),
            kid: kid.to_owned(),
            nonce: nonce.to_vec(),
            ct,
        })
    }

    /// Decifra um envelope; o texto claro é apagado da memória ao sair de escopo.
    pub fn open(&self, envelope: &Envelope, aad: Aad<'_>) -> Result<Zeroizing<Vec<u8>>, Error> {
        if envelope.v != VERSION || envelope.alg != ALG {
            return Err(Error::Unsupported(format!(
                "envelope v{} {}",
                envelope.v, envelope.alg
            )));
        }
        if envelope.nonce.len() != 24 {
            return Err(Error::Decrypt);
        }
        XChaCha20Poly1305::new(self.as_bytes().into())
            .decrypt(
                XNonce::from_slice(&envelope.nonce),
                Payload {
                    msg: &envelope.ct,
                    aad: &aad.bytes(),
                },
            )
            .map(Zeroizing::new)
            .map_err(|_| Error::Decrypt)
    }
}

/// Bytes como base64 padrão no JSON.
pub(crate) mod b64 {
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&STANDARD.encode(bytes))
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let text = String::deserialize(d)?;
        STANDARD.decode(text).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AAD: Aad<'static> = Aad {
        item_id: "01J9ZITEM",
        vault_id: "01J9ZVAULT",
    };

    #[test]
    fn ida_e_volta() {
        let key = Key::generate();
        let env = key.seal("vault:v:k1", AAD, b"senha do servidor").unwrap();
        assert_eq!(env.v, 1);
        assert_eq!(env.alg, "XC20P");
        assert_eq!(env.nonce.len(), 24);
        assert_eq!(
            key.open(&env, AAD).unwrap().as_slice(),
            b"senha do servidor"
        );
    }

    #[test]
    fn nonce_novo_a_cada_selo() {
        let key = Key::generate();
        let a = key.seal("k", AAD, b"x").unwrap();
        let b = key.seal("k", AAD, b"x").unwrap();
        assert_ne!(a.nonce, b.nonce);
        assert_ne!(a.ct, b.ct);
    }

    #[test]
    fn item_movido_ou_trocado_nao_abre() {
        let key = Key::generate();
        let env = key.seal("k", AAD, b"segredo").unwrap();
        let other_item = Aad {
            item_id: "01J9ZOUTRO",
            ..AAD
        };
        let other_vault = Aad {
            vault_id: "01J9ZOUTRO",
            ..AAD
        };
        assert_eq!(key.open(&env, other_item), Err(Error::Decrypt));
        assert_eq!(key.open(&env, other_vault), Err(Error::Decrypt));
    }

    #[test]
    fn aad_nao_colide_ao_mover_a_fronteira() {
        let a = Aad {
            item_id: "ab",
            vault_id: "c",
        };
        let b = Aad {
            item_id: "a",
            vault_id: "bc",
        };
        assert_ne!(a.bytes(), b.bytes());
    }

    #[test]
    fn adulterado_ou_outra_chave_nao_abre() {
        let key = Key::generate();
        let mut env = key.seal("k", AAD, b"segredo").unwrap();
        assert_eq!(Key::generate().open(&env, AAD), Err(Error::Decrypt));
        env.ct[0] ^= 1;
        assert_eq!(key.open(&env, AAD), Err(Error::Decrypt));
    }

    #[test]
    fn versao_desconhecida() {
        let key = Key::generate();
        let mut env = key.seal("k", AAD, b"x").unwrap();
        env.v = 2;
        assert!(matches!(key.open(&env, AAD), Err(Error::Unsupported(_))));
    }

    #[test]
    fn limite_de_64_kib() {
        let key = Key::generate();
        assert!(key.seal("k", AAD, &vec![0; MAX_ITEM]).is_ok());
        assert_eq!(
            key.seal("k", AAD, &vec![0; MAX_ITEM + 1]),
            Err(Error::TooLarge(MAX_ITEM + 1))
        );
    }

    #[test]
    fn json_com_base64() {
        let key = Key::from_bytes([1; 32]);
        let env = key.seal_with_nonce("k", AAD, b"oi", [2; 24]).unwrap();
        let json = serde_json::to_value(&env).unwrap();
        assert_eq!(json["nonce"], "AgICAgICAgICAgICAgICAgICAgICAgIC");
        let back: Envelope = serde_json::from_value(json).unwrap();
        assert_eq!(back, env);
    }

    /// Vetor A.3.1 do draft-irtf-cfrg-xchacha: confere a biblioteca, sem o nosso AAD.
    #[test]
    fn vetor_do_draft_xchacha() {
        let key: [u8; 32] = core::array::from_fn(|i| 0x80 + u8::try_from(i).unwrap());
        let nonce: [u8; 24] = core::array::from_fn(|i| 0x40 + u8::try_from(i).unwrap());
        let aad = [
            0x50, 0x51, 0x52, 0x53, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7,
        ];
        let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
        let ct = XChaCha20Poly1305::new((&key).into())
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad: &aad,
                },
            )
            .unwrap();
        let hex = crate::kdf::tests::hex(&ct);
        assert!(
            hex.starts_with("bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb")
        );
        assert!(hex.ends_with("c0875924c1c7987947deafd8780acf49"));
    }

    /// Vetor fixo do nosso formato (chave, nonce e AAD fixos): trava o envelope v1.
    #[test]
    fn vetor_fixo_do_envelope() {
        let key = Key::from_bytes([0x11; 32]);
        let env = key
            .seal_with_nonce("vault:v:k1", AAD, b"codatos", [0x22; 24])
            .unwrap();
        assert_eq!(crate::kdf::tests::hex(&env.ct), VECTOR_ENVELOPE_CT);
    }

    /// Conferido com uma implementação independente (`HChaCha20` + ChaCha20-Poly1305 da
    /// `cryptography`, em Python) ao criar o formato v1; não pode mudar.
    const VECTOR_ENVELOPE_CT: &str = "e008836d28d697b4cee67c2ec6c9d01ec80971fa171f40";
}
