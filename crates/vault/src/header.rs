//! Cabeçalho do cofre: o id, a versão da chave e como a chave do cofre está protegida.
//!
//! Sem conta (§7), o usuário escolhe na criação ([Q-36]):
//! - **senha do cofre**: a chave do cofre vai embrulhada por uma chave derivada da senha;
//! - **sessão do SO**: o header não guarda a chave; o núcleo a guarda no keychain do SO
//!   (Gerenciador de Credenciais no Windows) e o cofre abre sozinho no login.
//!
//! Trocar a senha só re-embrulha a chave do cofre; os itens não são re-cifrados.
//!
//! [Q-36]: ../../../docs/10-perguntas-em-aberto.md

use serde::{Deserialize, Serialize};

use crate::{Aad, Envelope, Error, KdfParams, Key};

const VERSION: u8 = 1;
/// Id do "item" da chave embrulhada no AAD.
const WRAPPED_ITEM: &str = "vault-key";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub v: u8,
    pub vault_id: String,
    /// Versão da chave do cofre (`k1`, `k2`…), para rotação no M6.
    pub key_version: u32,
    pub protection: Protection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Protection {
    Password { kdf: KdfParams, wrapped: Envelope },
    OsSession,
}

impl Header {
    /// Cofre novo protegido por senha. Devolve o header e a chave do cofre, já aberta.
    pub fn with_password(
        vault_id: &str,
        password: &[u8],
        kdf: KdfParams,
    ) -> Result<(Self, Key), Error> {
        let key = Key::generate();
        let mut header = Self::new(vault_id);
        header.set_password(&key, password, kdf)?;
        Ok((header, key))
    }

    /// Cofre novo no modo sessão do SO: quem chama guarda a chave no keychain.
    pub fn os_session(vault_id: &str) -> (Self, Key) {
        (Self::new(vault_id), Key::generate())
    }

    fn new(vault_id: &str) -> Self {
        Self {
            v: VERSION,
            vault_id: vault_id.to_owned(),
            key_version: 1,
            protection: Protection::OsSession,
        }
    }

    /// `vault:<id>:k<versão>`, o `kid` dos envelopes deste cofre.
    pub fn kid(&self) -> String {
        format!("vault:{}:k{}", self.vault_id, self.key_version)
    }

    pub fn uses_password(&self) -> bool {
        matches!(self.protection, Protection::Password { .. })
    }

    /// Abre a chave do cofre com a senha.
    pub fn unlock(&self, password: &[u8]) -> Result<Key, Error> {
        self.check_version()?;
        let Protection::Password { kdf, wrapped } = &self.protection else {
            return Err(Error::NoPassword);
        };
        let wrap = kdf.wrap_key(password)?;
        let bytes = wrap
            .open(wrapped, self.wrapped_aad())
            .map_err(|error| match error {
                Error::Decrypt => Error::WrongPassword,
                other => other,
            })?;
        let array: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| Error::WrongPassword)?;
        Ok(Key::from_bytes(array))
    }

    /// Passa a proteger `key` (já aberta) com `password`: cria ou troca a senha, ou sai do
    /// modo sessão do SO (quem chama apaga a chave do keychain).
    pub fn set_password(
        &mut self,
        key: &Key,
        password: &[u8],
        kdf: KdfParams,
    ) -> Result<(), Error> {
        let wrap = kdf.wrap_key(password)?;
        let wrapped = wrap.seal(&self.wrap_kid(), self.wrapped_aad(), key.as_bytes())?;
        self.protection = Protection::Password { kdf, wrapped };
        Ok(())
    }

    /// Passa para o modo sessão do SO (quem chama guarda a chave no keychain).
    pub fn set_os_session(&mut self) {
        self.protection = Protection::OsSession;
    }

    fn check_version(&self) -> Result<(), Error> {
        if self.v == VERSION {
            Ok(())
        } else {
            Err(Error::Unsupported(format!("header v{}", self.v)))
        }
    }

    fn wrap_kid(&self) -> String {
        format!("vault:{}:wrap", self.vault_id)
    }

    fn wrapped_aad(&self) -> Aad<'_> {
        Aad {
            item_id: WRAPPED_ITEM,
            vault_id: &self.vault_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cheap() -> KdfParams {
        KdfParams::with_cost(8, 1, 1)
    }

    #[test]
    fn senha_certa_abre_a_mesma_chave() {
        let (header, key) = Header::with_password("01J9ZV", b"senha boa", cheap()).unwrap();
        assert!(header.uses_password());
        assert_eq!(
            header.unlock(b"senha boa").unwrap().as_bytes(),
            key.as_bytes()
        );
        assert_eq!(
            header.unlock(b"senha ruim").unwrap_err(),
            Error::WrongPassword
        );
    }

    #[test]
    fn itens_continuam_abrindo_depois_de_trocar_a_senha() {
        let (mut header, key) = Header::with_password("01J9ZV", b"antiga", cheap()).unwrap();
        let aad = Aad {
            item_id: "host-1",
            vault_id: &header.vault_id.clone(),
        };
        let item = key.seal(&header.kid(), aad, b"segredo").unwrap();

        header.set_password(&key, b"nova", cheap()).unwrap();
        assert_eq!(header.unlock(b"antiga").unwrap_err(), Error::WrongPassword);
        let reopened = header.unlock(b"nova").unwrap();
        assert_eq!(reopened.open(&item, aad).unwrap().as_slice(), b"segredo");
    }

    #[test]
    fn modo_sessao_nao_tem_senha() {
        let (mut header, key) = Header::os_session("01J9ZV");
        assert!(!header.uses_password());
        assert_eq!(header.unlock(b"x").unwrap_err(), Error::NoPassword);

        header
            .set_password(&key, b"agora com senha", cheap())
            .unwrap();
        assert_eq!(
            header.unlock(b"agora com senha").unwrap().as_bytes(),
            key.as_bytes()
        );
        header.set_os_session();
        assert!(!header.uses_password());
    }

    #[test]
    fn chave_embrulhada_nao_serve_em_outro_cofre() {
        let (header, _) = Header::with_password("01J9ZV", b"senha", cheap()).unwrap();
        let mut moved = header.clone();
        moved.vault_id = "01J9ZOUTRO".to_owned();
        assert_eq!(moved.unlock(b"senha").unwrap_err(), Error::WrongPassword);
    }

    #[test]
    fn json_do_header() {
        let (header, _) = Header::with_password("01J9ZV", b"senha", cheap()).unwrap();
        let json = serde_json::to_value(&header).unwrap();
        assert_eq!(json["v"], 1);
        assert_eq!(json["vaultId"], "01J9ZV");
        assert_eq!(json["protection"]["kind"], "password");
        assert_eq!(json["protection"]["kdf"]["alg"], "argon2id");
        let back: Header = serde_json::from_value(json).unwrap();
        assert_eq!(back, header);
        assert_eq!(header.kid(), "vault:01J9ZV:k1");

        let (session, _) = Header::os_session("01J9ZV");
        assert_eq!(
            serde_json::to_value(&session).unwrap()["protection"],
            serde_json::json!({"kind": "osSession"})
        );
    }

    #[test]
    fn versao_desconhecida() {
        let (mut header, _) = Header::with_password("01J9ZV", b"senha", cheap()).unwrap();
        header.v = 9;
        assert!(matches!(
            header.unlock(b"senha"),
            Err(Error::Unsupported(_))
        ));
    }
}
