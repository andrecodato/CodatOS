//! `connections.importSshConfig` e `connections.exportSshConfig`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use codatos_core::{Method, MethodError, MethodFuture, params};
use codatos_protocol::connections::{
    Connections, DEFAULT_SSH_PORT, Host, Identity, ImportReport, ImportSshConfig,
    METHOD_SSH_CONFIG_EXPORT, METHOD_SSH_CONFIG_IMPORT, SshConfigText,
};
use codatos_store::{Store, Vault};
use serde_json::{Value, json};

use crate::ids::new_id;
use crate::keys::{self, KeyError};
use crate::keys_rpc::{internal, save_key};
use crate::ssh_config::{self, ConfigHost, ExportHost, Jump};

/// Um `ssh_config` maior que isso não é de uma pessoa.
const MAX_CONFIG: usize = 1 << 20;
const MAX_KEY_FILE: u64 = 64 * 1024;

pub fn ssh_config_methods(store: &Store, vault: &Vault) -> Vec<Arc<dyn Method>> {
    [METHOD_SSH_CONFIG_IMPORT, METHOD_SSH_CONFIG_EXPORT]
        .into_iter()
        .map(|name| {
            Arc::new(SshConfigMethod {
                name,
                store: store.clone(),
                vault: vault.clone(),
            }) as Arc<dyn Method>
        })
        .collect()
}

#[derive(Debug)]
struct SshConfigMethod {
    name: &'static str,
    store: Store,
    vault: Vault,
}

impl Method for SshConfigMethod {
    fn name(&self) -> &'static str {
        self.name
    }

    fn call(&self, value: Value) -> MethodFuture<'_> {
        Box::pin(async move {
            if self.name == METHOD_SSH_CONFIG_EXPORT {
                let all = self.store.connections().await.map_err(internal)?;
                return Ok(json!(SshConfigText { text: export(&all) }));
            }
            let p: ImportSshConfig = params(value)?;
            let (text, base) = if let Some(text) = p.text {
                (text, home_ssh_dir())
            } else {
                let dir = home_ssh_dir().ok_or_else(no_config)?;
                let text = tokio::fs::read_to_string(dir.join("config"))
                    .await
                    .map_err(|_| no_config())?;
                (text, Some(dir))
            };
            if text.len() > MAX_CONFIG {
                return Err(MethodError::invalid_params("Arquivo grande demais"));
            }
            let importer = Importer::new(&self.store, &self.vault, base).await?;
            Ok(json!(importer.run(&text).await?))
        })
    }
}

fn no_config() -> MethodError {
    MethodError::new(
        "ssh_config_not_found",
        "Não encontrei o ~/.ssh/config deste PC",
    )
}

fn home_ssh_dir() -> Option<PathBuf> {
    std::env::home_dir().map(|h| h.join(".ssh"))
}

/// Usuário do SO: o que o `ssh` usa quando o bloco não tem `User`.
fn local_user() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "root".to_owned())
}

struct Importer<'a> {
    store: &'a Store,
    vault: &'a Vault,
    ssh_dir: Option<PathBuf>,
    report: ImportReport,
    /// Nome do host → id, dos que já existiam e dos criados agora.
    hosts: HashMap<String, String>,
    /// (usuário, chave) → identidade.
    identities: HashMap<(String, Option<String>), String>,
    /// Fingerprint → (id, nome) da chave.
    keys: HashMap<String, (String, String)>,
    vault_warned: bool,
}

impl<'a> Importer<'a> {
    async fn new(
        store: &'a Store,
        vault: &'a Vault,
        ssh_dir: Option<PathBuf>,
    ) -> Result<Self, MethodError> {
        let all = store.connections().await.map_err(internal)?;
        Ok(Self {
            store,
            vault,
            ssh_dir,
            report: ImportReport::default(),
            hosts: all
                .hosts
                .iter()
                .map(|h| (h.name.clone(), h.id.clone()))
                .collect(),
            identities: all
                .identities
                .iter()
                .map(|i| ((i.username.clone(), i.key_id.clone()), i.id.clone()))
                .collect(),
            keys: all
                .keys
                .iter()
                .map(|k| (k.fingerprint.clone(), (k.id.clone(), k.name.clone())))
                .collect(),
            vault_warned: false,
        })
    }

    async fn run(mut self, text: &str) -> Result<ImportReport, MethodError> {
        let parsed = ssh_config::parse(text);
        self.report.warnings = parsed.warnings;
        let mut created: Vec<(Host, Vec<String>)> = Vec::new();
        for entry in parsed.hosts {
            if self.hosts.contains_key(&entry.alias) {
                self.report.skipped.push(entry.alias);
                continue;
            }
            let host = self.create_host(&entry).await?;
            created.push((host, entry.proxy_jump));
        }
        // Segunda passada: os saltos podem citar hosts que vêm depois no arquivo.
        let mut jumps: HashMap<String, String> = self
            .store
            .connections()
            .await
            .map_err(internal)?
            .hosts
            .into_iter()
            .filter_map(|h| Some((h.id, h.jump_host_id?)))
            .collect();
        for (mut host, hops) in created {
            let Some(jump) = self.resolve_hops(&hops, &mut jumps).await? else {
                continue;
            };
            if leads_back(&jumps, &jump, &host.id) {
                self.report
                    .warnings
                    .push(format!("{}: ProxyJump em ciclo, ignorado", host.name));
                continue;
            }
            jumps.insert(host.id.clone(), jump.clone());
            host.jump_host_id = Some(jump);
            self.store.save_host(&host).await.map_err(internal)?;
        }
        Ok(self.report)
    }

    async fn create_host(&mut self, entry: &ConfigHost) -> Result<Host, MethodError> {
        let mut key = None;
        for file in &entry.identity_files {
            if let Some(found) = self.load_key(file).await? {
                key = Some(found);
                break;
            }
        }
        let identity_id = if entry.user.is_some() || key.is_some() {
            let user = entry.user.clone().unwrap_or_else(local_user);
            Some(self.identity(&user, key).await?)
        } else {
            None
        };
        let advanced = if entry.other.is_empty() {
            Value::Object(serde_json::Map::new())
        } else {
            json!({ "sshConfig": ssh_config::other_to_json(&entry.other) })
        };
        let host = Host {
            id: new_id(),
            name: entry.alias.clone(),
            address: entry
                .hostname
                .clone()
                .unwrap_or_else(|| entry.alias.clone()),
            port: entry.port.unwrap_or(DEFAULT_SSH_PORT),
            protocol: codatos_protocol::connections::Protocol::Ssh,
            group_id: None,
            identity_id,
            jump_host_id: None,
            tags: Vec::new(),
            favorite: false,
            color: None,
            notes: String::new(),
            advanced,
            last_used_at: None,
        };
        self.store.save_host(&host).await.map_err(internal)?;
        self.hosts.insert(host.name.clone(), host.id.clone());
        self.report.hosts += 1;
        Ok(host)
    }

    /// Id do host do último salto, criando hosts para saltos que não são apelidos.
    async fn resolve_hops(
        &mut self,
        hops: &[String],
        jumps: &mut HashMap<String, String>,
    ) -> Result<Option<String>, MethodError> {
        let mut previous: Option<String> = None;
        for hop in hops {
            let id = if let Some(id) = self.hosts.get(hop) {
                id.clone()
            } else {
                let spec = Jump::parse(hop);
                let entry = ConfigHost {
                    alias: hop.clone(),
                    hostname: Some(spec.address),
                    port: spec.port,
                    user: spec.user,
                    ..ConfigHost::default()
                };
                let mut host = self.create_host(&entry).await?;
                if let Some(prev) = &previous {
                    host.jump_host_id = Some(prev.clone());
                    jumps.insert(host.id.clone(), prev.clone());
                    self.store.save_host(&host).await.map_err(internal)?;
                }
                host.id
            };
            previous = Some(id);
        }
        Ok(previous)
    }

    async fn identity(
        &mut self,
        user: &str,
        key: Option<(String, String)>,
    ) -> Result<String, MethodError> {
        let key_id = key.as_ref().map(|(id, _)| id.clone());
        let lookup = (user.to_owned(), key_id.clone());
        if let Some(id) = self.identities.get(&lookup) {
            return Ok(id.clone());
        }
        let identity = Identity {
            id: new_id(),
            name: match &key {
                Some((_, key_name)) => format!("{user} ({key_name})"),
                None => user.to_owned(),
            },
            username: user.to_owned(),
            key_id,
            has_password: false,
        };
        self.store
            .save_identity(&identity, None)
            .await
            .map_err(internal)?;
        self.identities.insert(lookup, identity.id.clone());
        self.report.identities += 1;
        Ok(identity.id)
    }

    /// (id, nome) da chave do `IdentityFile`, importando se ainda não estiver no cofre.
    async fn load_key(&mut self, file: &str) -> Result<Option<(String, String)>, MethodError> {
        let Some(path) = self.expand(file) else {
            return Ok(None);
        };
        let Some(text) = read_small(&path).await.map(zeroize::Zeroizing::new) else {
            self.report
                .warnings
                .push(format!("{file}: arquivo não encontrado"));
            return Ok(None);
        };
        let key = match keys::import(&text, None) {
            Ok(key) => key,
            Err(KeyError::PassphraseRequired) => {
                self.report.warnings.push(format!(
                    "{file}: a chave tem passphrase; importe pela tela de chaves"
                ));
                return Ok(None);
            }
            Err(_) => {
                self.report
                    .warnings
                    .push(format!("{file}: formato de chave não suportado"));
                return Ok(None);
            }
        };
        let fingerprint = keys::describe(&key).map_err(|_| internal_key())?.2;
        if let Some(found) = self.keys.get(&fingerprint) {
            return Ok(Some(found.clone()));
        }
        let name = path
            .file_name()
            .map_or_else(|| file.to_owned(), |n| n.to_string_lossy().into_owned());
        let id = new_id();
        match save_key(self.store, self.vault, id.clone(), name.clone(), &key).await {
            Ok(_) => {
                self.report.keys += 1;
                self.keys.insert(fingerprint, (id.clone(), name.clone()));
                Ok(Some((id, name)))
            }
            Err(e) if is_vault_closed(&e) => {
                if !self.vault_warned {
                    self.vault_warned = true;
                    self.report.warnings.push(
                        "Cofre bloqueado ou não criado: as chaves não foram importadas".to_owned(),
                    );
                }
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// `~/` e caminhos relativos ao `~/.ssh`, como o OpenSSH.
    fn expand(&self, file: &str) -> Option<PathBuf> {
        if let Some(rest) = file.strip_prefix("~/").or_else(|| file.strip_prefix("~\\")) {
            return std::env::home_dir().map(|h| h.join(rest));
        }
        let path = Path::new(file);
        if path.is_absolute() {
            Some(path.to_path_buf())
        } else {
            self.ssh_dir.as_ref().map(|d| d.join(path))
        }
    }
}

fn is_vault_closed(error: &MethodError) -> bool {
    use codatos_protocol::vault::errors;
    [errors::LOCKED, errors::ABSENT].contains(&error.code())
}

fn internal_key() -> MethodError {
    MethodError::new("key_failed", "Chave ilegível")
}

async fn read_small(path: &Path) -> Option<String> {
    let meta = tokio::fs::metadata(path).await.ok()?;
    if !meta.is_file() || meta.len() > MAX_KEY_FILE {
        return None;
    }
    tokio::fs::read_to_string(path).await.ok()
}

/// O salto `start` acaba voltando para `target`?
fn leads_back(jumps: &HashMap<String, String>, start: &str, target: &str) -> bool {
    let mut current = Some(start);
    for _ in 0..32 {
        match current {
            Some(id) if id == target => return true,
            Some(id) => current = jumps.get(id).map(String::as_str),
            None => return false,
        }
    }
    true
}

/// Hosts cadastrados como `ssh_config`, com a herança dos grupos aplicada.
fn export(all: &Connections) -> String {
    let names: HashMap<&str, &str> = all
        .hosts
        .iter()
        .map(|h| (h.id.as_str(), h.name.as_str()))
        .collect();
    let identities: HashMap<&str, &Identity> =
        all.identities.iter().map(|i| (i.id.as_str(), i)).collect();
    let keys: HashMap<&str, &str> = all
        .keys
        .iter()
        .map(|k| (k.id.as_str(), k.name.as_str()))
        .collect();
    let hosts: Vec<ExportHost> = all
        .hosts
        .iter()
        .filter_map(|h| codatos_store::effective(all, &h.id))
        .map(|e| {
            let identity = e.identity_id.as_deref().and_then(|id| identities.get(id));
            ExportHost {
                alias: e.host.name.clone(),
                address: e.host.address.clone(),
                port: e.host.port,
                user: identity.map(|i| i.username.clone()),
                key_name: identity
                    .and_then(|i| i.key_id.as_deref())
                    .and_then(|k| keys.get(k))
                    .map(|n| (*n).to_owned()),
                // O `ssh` aplica o bloco do salto, inclusive o ProxyJump dele.
                proxy_jump: e
                    .jump_host_id
                    .as_deref()
                    .and_then(|j| names.get(j))
                    .map(|n| vec![(*n).to_owned()])
                    .unwrap_or_default(),
                other: ssh_config::other_from_json(&e.advanced),
            }
        })
        .collect();
    ssh_config::render(&hosts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys_rpc::tests::env;

    async fn call(
        store: &Store,
        vault: &Vault,
        name: &str,
        value: Value,
    ) -> Result<Value, MethodError> {
        let all = ssh_config_methods(store, vault);
        let method = all.iter().find(|m| m.name() == name).unwrap();
        method.call(value).await
    }

    #[tokio::test]
    async fn importa_saltos_identidades_e_chaves() {
        let dir = std::env::temp_dir().join(format!("codatos-ssh-{}", new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let key =
            keys::generate(codatos_protocol::connections::KeyAlgorithm::Ed25519, "eu").unwrap();
        std::fs::write(
            dir.join("id_ed25519"),
            keys::to_openssh(&key).unwrap().as_bytes(),
        )
        .unwrap();
        let key_path = dir.join("id_ed25519");
        let config = format!(
            "Host web\n  HostName 10.0.0.5\n  User deploy\n  IdentityFile {}\n  ProxyJump ops@bastion:2200\n  ForwardAgent yes\n\
             Host db\n  HostName 10.0.0.9\n  User deploy\n  IdentityFile {}\n  ProxyJump web\n\
             Host a\n  ProxyJump b\nHost b\n  ProxyJump a\n\
             Host semchave\n  IdentityFile ~/nao-existe\n",
            key_path.display(),
            key_path.display()
        );

        let (store, vault) = env(true).await;
        let report: ImportReport = serde_json::from_value(
            call(
                &store,
                &vault,
                METHOD_SSH_CONFIG_IMPORT,
                json!({"text": config}),
            )
            .await
            .unwrap(),
        )
        .unwrap();
        // web, db, a, b, semchave e o salto ops@bastion:2200.
        assert_eq!(report.hosts, 6, "{report:?}");
        assert_eq!(report.keys, 1);
        // deploy (id_ed25519), ops e o usuário local do `semchave` não entram: sem User nem chave.
        assert_eq!(report.identities, 2, "{report:?}");
        assert!(report.warnings.iter().any(|w| w.contains("ciclo")));
        assert!(report.warnings.iter().any(|w| w.contains("nao-existe")));

        let all = store.connections().await.unwrap();
        let by_name = |n: &str| all.hosts.iter().find(|h| h.name == n).unwrap();
        let bastion = by_name("ops@bastion:2200");
        assert_eq!((bastion.address.as_str(), bastion.port), ("bastion", 2200));
        assert_eq!(
            by_name("web").jump_host_id.as_deref(),
            Some(bastion.id.as_str())
        );
        assert_eq!(
            by_name("db").jump_host_id.as_deref(),
            Some(by_name("web").id.as_str())
        );
        assert_eq!(by_name("web").identity_id, by_name("db").identity_id);
        assert_eq!(by_name("web").advanced["sshConfig"]["ForwardAgent"], "yes");

        // Reimportar não duplica nada.
        let again: ImportReport = serde_json::from_value(
            call(
                &store,
                &vault,
                METHOD_SSH_CONFIG_IMPORT,
                json!({"text": config}),
            )
            .await
            .unwrap(),
        )
        .unwrap();
        assert_eq!(again.hosts, 0);
        assert_eq!(again.skipped.len(), 5);

        let text = call(&store, &vault, METHOD_SSH_CONFIG_EXPORT, Value::Null)
            .await
            .unwrap()["text"]
            .as_str()
            .unwrap()
            .to_owned();
        let parsed = ssh_config::parse(&text);
        let web = parsed.hosts.iter().find(|h| h.alias == "web").unwrap();
        assert_eq!(web.user.as_deref(), Some("deploy"));
        assert_eq!(web.proxy_jump, ["ops@bastion:2200"]);
        assert!(text.contains("# Chave \"id_ed25519\""));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn cofre_fechado_importa_sem_chaves() {
        let dir = std::env::temp_dir().join(format!("codatos-ssh-{}", new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let key = keys::generate(codatos_protocol::connections::KeyAlgorithm::Ed25519, "").unwrap();
        std::fs::write(dir.join("k"), keys::to_openssh(&key).unwrap().as_bytes()).unwrap();
        let config = format!(
            "Host web\n  User x\n  IdentityFile {}\n",
            dir.join("k").display()
        );
        let (store, vault) = env(false).await;
        let report: ImportReport = serde_json::from_value(
            call(
                &store,
                &vault,
                METHOD_SSH_CONFIG_IMPORT,
                json!({"text": config}),
            )
            .await
            .unwrap(),
        )
        .unwrap();
        assert_eq!((report.hosts, report.keys, report.identities), (1, 0, 1));
        assert!(report.warnings[0].contains("Cofre"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
