//! Tabelas `host_groups`, `hosts`, `identities` e `ssh_keys`, e a herança de configuração dos grupos
//! (RF-CON-002).

use std::collections::{HashMap, HashSet};

use codatos_protocol::connections::{
    Connections, Host, HostDefaults, HostGroup, Identity, Protocol, SshKey,
};
use serde_json::{Map, Value};

use crate::{Error, Store};

/// Profundidade máxima de grupos e de jump hosts encadeados.
const MAX_DEPTH: usize = 16;

/// Host com o que herda dos grupos já aplicado: o que a sessão SSH usa.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveHost {
    pub host: Host,
    pub identity_id: Option<String>,
    pub jump_host_id: Option<String>,
    /// Objeto com as chaves do host por cima das dos grupos (o mais próximo vence).
    pub advanced: Value,
}

type HostRow = (
    String,
    Option<String>,
    String,
    String,
    i64,
    String,
    Option<String>,
    Option<String>,
    String,
    i64,
    Option<String>,
    String,
    String,
    Option<String>,
);

impl Store {
    /// Grupos, hosts e identidades (sem segredos).
    pub async fn connections(&self) -> Result<Connections, Error> {
        let groups: Vec<(String, Option<String>, String, String)> = sqlx::query_as(
            "SELECT id, parent_id, name, defaults FROM host_groups ORDER BY name COLLATE NOCASE, id",
        )
        .fetch_all(&self.pool)
        .await?;
        let groups = groups
            .into_iter()
            .map(|(id, parent_id, name, defaults)| {
                Ok(HostGroup {
                    id,
                    parent_id,
                    name,
                    defaults: serde_json::from_str(&defaults)?,
                })
            })
            .collect::<Result<_, Error>>()?;

        let hosts: Vec<HostRow> = sqlx::query_as(
            "SELECT id, group_id, name, address, port, protocol, identity_id, jump_host_id,
                    tags, favorite, color, notes, advanced, last_used_at
             FROM hosts ORDER BY name COLLATE NOCASE, id",
        )
        .fetch_all(&self.pool)
        .await?;
        let hosts = hosts
            .into_iter()
            .map(host_from_row)
            .collect::<Result<_, _>>()?;

        let identities: Vec<(String, String, String, Option<String>, bool)> = sqlx::query_as(
            "SELECT id, name, username, key_id, password_sealed IS NOT NULL
             FROM identities ORDER BY name COLLATE NOCASE, id",
        )
        .fetch_all(&self.pool)
        .await?;
        let identities = identities
            .into_iter()
            .map(|(id, name, username, key_id, has_password)| Identity {
                id,
                name,
                username,
                key_id,
                has_password,
            })
            .collect();

        let keys: Vec<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, name, algorithm, public_key, fingerprint
             FROM ssh_keys ORDER BY name COLLATE NOCASE, id",
        )
        .fetch_all(&self.pool)
        .await?;
        let keys = keys
            .into_iter()
            .map(|(id, name, algorithm, public_key, fingerprint)| SshKey {
                id,
                name,
                algorithm,
                public_key,
                fingerprint,
            })
            .collect();

        Ok(Connections {
            groups,
            hosts,
            identities,
            keys,
        })
    }

    /// Grava uma chave nova; `private_key_sealed` é o envelope do cofre com a chave
    /// privada em formato OpenSSH, sem passphrase.
    pub async fn insert_key(&self, key: &SshKey, private_key_sealed: &str) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO ssh_keys (id, name, algorithm, public_key, fingerprint, private_key_sealed)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&key.id)
        .bind(&key.name)
        .bind(&key.algorithm)
        .bind(&key.public_key)
        .bind(&key.fingerprint)
        .bind(private_key_sealed)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn rename_key(&self, id: &str, name: &str) -> Result<bool, Error> {
        let done = sqlx::query(
            "UPDATE ssh_keys SET name = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?",
        )
        .bind(name)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(done.rows_affected() > 0)
    }

    pub async fn delete_key(&self, id: &str) -> Result<bool, Error> {
        let done = sqlx::query("DELETE FROM ssh_keys WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(done.rows_affected() > 0)
    }

    /// Envelope da chave privada, para o núcleo decifrar na hora de conectar.
    pub async fn key_private(&self, id: &str) -> Result<Option<String>, Error> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT private_key_sealed FROM ssh_keys WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|(sealed,)| sealed))
    }

    pub async fn save_group(&self, group: &HostGroup) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO host_groups (id, parent_id, name, defaults) VALUES (?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET parent_id = excluded.parent_id,
                 name = excluded.name, defaults = excluded.defaults,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(&group.id)
        .bind(&group.parent_id)
        .bind(&group.name)
        .bind(serde_json::to_string(&group.defaults)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Subgrupos e hosts do grupo passam para o grupo pai. `false` se não existia.
    pub async fn delete_group(&self, id: &str) -> Result<bool, Error> {
        let mut tx = self.pool.begin().await?;
        let Some((parent,)): Option<(Option<String>,)> =
            sqlx::query_as("SELECT parent_id FROM host_groups WHERE id = ?")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?
        else {
            return Ok(false);
        };
        sqlx::query("UPDATE host_groups SET parent_id = ? WHERE parent_id = ?")
            .bind(&parent)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE hosts SET group_id = ? WHERE group_id = ?")
            .bind(&parent)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM host_groups WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }

    /// Cria ou atualiza o host. `last_used_at` não muda por aqui.
    pub async fn save_host(&self, host: &Host) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO hosts (id, group_id, name, address, port, protocol, identity_id,
                                jump_host_id, tags, favorite, color, notes, advanced)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET group_id = excluded.group_id,
                 name = excluded.name, address = excluded.address, port = excluded.port,
                 protocol = excluded.protocol, identity_id = excluded.identity_id,
                 jump_host_id = excluded.jump_host_id, tags = excluded.tags,
                 favorite = excluded.favorite, color = excluded.color, notes = excluded.notes,
                 advanced = excluded.advanced,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(&host.id)
        .bind(&host.group_id)
        .bind(&host.name)
        .bind(&host.address)
        .bind(i64::from(host.port))
        .bind(host.protocol.as_str())
        .bind(&host.identity_id)
        .bind(&host.jump_host_id)
        .bind(serde_json::to_string(&host.tags)?)
        .bind(host.favorite)
        .bind(&host.color)
        .bind(&host.notes)
        .bind(serde_json::to_string(&host.advanced)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_host(&self, id: &str) -> Result<bool, Error> {
        let done = sqlx::query("DELETE FROM hosts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(done.rows_affected() > 0)
    }

    /// Marca o uso do host, para a ordenação por uso recente (RF-CON-003).
    pub async fn touch_host(&self, id: &str) -> Result<(), Error> {
        sqlx::query(
            "UPDATE hosts SET last_used_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Cria ou atualiza a identidade. `password`: `None` mantém a senha guardada,
    /// `Some(None)` apaga e `Some(Some(envelope))` troca pelo envelope já selado.
    pub async fn save_identity(
        &self,
        identity: &Identity,
        password: Option<Option<&str>>,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO identities (id, name, username, key_id) VALUES (?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name,
                 username = excluded.username, key_id = excluded.key_id,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(&identity.id)
        .bind(&identity.name)
        .bind(&identity.username)
        .bind(&identity.key_id)
        .execute(&mut *tx)
        .await?;
        if let Some(sealed) = password {
            sqlx::query("UPDATE identities SET password_sealed = ? WHERE id = ?")
                .bind(sealed)
                .bind(&identity.id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_identity(&self, id: &str) -> Result<bool, Error> {
        let done = sqlx::query("DELETE FROM identities WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(done.rows_affected() > 0)
    }

    /// Envelope da senha da identidade, para o núcleo decifrar na hora de conectar.
    pub async fn identity_password(&self, id: &str) -> Result<Option<String>, Error> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT password_sealed FROM identities WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.and_then(|(sealed,)| sealed))
    }

    /// O host com a herança dos grupos aplicada. `None` se não existe.
    pub async fn effective_host(&self, id: &str) -> Result<Option<EffectiveHost>, Error> {
        Ok(effective(&self.connections().await?, id))
    }
}

fn host_from_row(row: HostRow) -> Result<Host, Error> {
    let (
        id,
        group_id,
        name,
        address,
        port,
        protocol,
        identity_id,
        jump_host_id,
        tags,
        favorite,
        color,
        notes,
        advanced,
        last_used_at,
    ) = row;
    Ok(Host {
        id,
        name,
        address,
        port: u16::try_from(port).unwrap_or(codatos_protocol::connections::DEFAULT_SSH_PORT),
        protocol: Protocol::parse(&protocol).unwrap_or_default(),
        group_id,
        identity_id,
        jump_host_id,
        tags: serde_json::from_str(&tags)?,
        favorite: favorite != 0,
        color,
        notes,
        advanced: serde_json::from_str(&advanced)?,
        last_used_at,
    })
}

/// Grupos do host, do mais próximo ao mais distante. Para em ciclos.
fn group_chain<'a>(
    groups: &HashMap<&str, &'a HostGroup>,
    start: Option<&str>,
) -> Vec<&'a HostGroup> {
    let mut chain = Vec::new();
    let mut seen = HashSet::new();
    let mut next = start;
    while let Some(id) = next {
        let Some(group) = groups.get(id) else { break };
        if !seen.insert(id) || chain.len() >= MAX_DEPTH {
            break;
        }
        chain.push(*group);
        next = group.parent_id.as_deref();
    }
    chain
}

/// Aplica a herança dos grupos ao host `id`.
pub(crate) fn effective(all: &Connections, id: &str) -> Option<EffectiveHost> {
    let host = all.hosts.iter().find(|h| h.id == id)?;
    let groups: HashMap<&str, &HostGroup> = all.groups.iter().map(|g| (g.id.as_str(), g)).collect();
    let chain = group_chain(&groups, host.group_id.as_deref());
    let inherited = |pick: fn(&HostDefaults) -> &Option<String>| {
        chain.iter().find_map(|g| pick(&g.defaults).clone())
    };
    let identity_id = host
        .identity_id
        .clone()
        .or_else(|| inherited(|d| &d.identity_id));
    // Um jump host herdado do próprio grupo não aponta para ele mesmo.
    let jump_host_id = host
        .jump_host_id
        .clone()
        .or_else(|| inherited(|d| &d.jump_host_id))
        .filter(|j| *j != host.id);
    let mut advanced = Map::new();
    for layer in chain
        .iter()
        .rev()
        .map(|g| &g.defaults.advanced)
        .chain([&host.advanced])
    {
        if let Value::Object(map) = layer {
            advanced.extend(map.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
    }
    Some(EffectiveHost {
        host: host.clone(),
        identity_id,
        jump_host_id,
        advanced: Value::Object(advanced),
    })
}

/// O grupo `id` não pode ficar dentro dele mesmo.
pub(crate) fn group_cycle(all: &Connections, id: &str, parent: Option<&str>) -> bool {
    let groups: HashMap<&str, &HostGroup> = all.groups.iter().map(|g| (g.id.as_str(), g)).collect();
    parent == Some(id) || group_chain(&groups, parent).iter().any(|g| g.id == id)
}

/// Algum host cuja cadeia de jump hosts volta para ele mesmo ou passa do limite?
pub(crate) fn jump_cycle(all: &Connections) -> Option<String> {
    let jumps: HashMap<&str, Option<String>> = all
        .hosts
        .iter()
        .filter_map(|h| Some((h.id.as_str(), effective(all, &h.id)?.jump_host_id)))
        .collect();
    for host in &all.hosts {
        let mut seen = HashSet::from([host.id.as_str()]);
        let mut next = jumps.get(host.id.as_str()).and_then(Option::as_deref);
        while let Some(id) = next {
            if !seen.insert(id) || seen.len() > MAX_DEPTH {
                return Some(host.name.clone());
            }
            next = jumps.get(id).and_then(Option::as_deref);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn group(id: &str, parent: Option<&str>, defaults: Value) -> HostGroup {
        HostGroup {
            id: id.into(),
            parent_id: parent.map(Into::into),
            name: id.into(),
            defaults: serde_json::from_value(defaults).unwrap(),
        }
    }

    fn host(id: &str, group: Option<&str>, extra: &Value) -> Host {
        let mut v = json!({"id": id, "name": id, "address": format!("{id}.lan"), "groupId": group});
        v.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn heranca_do_grupo_mais_proximo() {
        let all = Connections {
            groups: vec![
                group(
                    "raiz",
                    None,
                    json!({"identityId": "i-raiz", "jumpHostId": "bastiao", "advanced": {"keepalive": 30, "term": "xterm"}}),
                ),
                group(
                    "prod",
                    Some("raiz"),
                    json!({"identityId": "i-prod", "advanced": {"keepalive": 10}}),
                ),
            ],
            hosts: vec![
                host(
                    "web",
                    Some("prod"),
                    &json!({"advanced": {"term": "xterm-256color"}}),
                ),
                host("bastiao", Some("raiz"), &json!({})),
                host("solto", None, &json!({"identityId": "i-solto"})),
            ],
            identities: vec![],
            keys: vec![],
        };
        let web = effective(&all, "web").unwrap();
        assert_eq!(web.identity_id.as_deref(), Some("i-prod"));
        assert_eq!(web.jump_host_id.as_deref(), Some("bastiao"));
        assert_eq!(
            web.advanced,
            json!({"keepalive": 10, "term": "xterm-256color"})
        );
        // O bastião herda o próprio nome como jump host: ignorado.
        assert_eq!(effective(&all, "bastiao").unwrap().jump_host_id, None);
        let solto = effective(&all, "solto").unwrap();
        assert_eq!(solto.identity_id.as_deref(), Some("i-solto"));
        assert_eq!(solto.advanced, json!({}));
        assert!(effective(&all, "nada").is_none());
    }

    #[test]
    fn ciclos() {
        let mut all = Connections {
            groups: vec![
                group("a", None, json!({})),
                group("b", Some("a"), json!({})),
            ],
            hosts: vec![
                host("x", None, &json!({"jumpHostId": "y"})),
                host("y", None, &json!({})),
            ],
            identities: vec![],
            keys: vec![],
        };
        assert!(group_cycle(&all, "a", Some("b")));
        assert!(group_cycle(&all, "a", Some("a")));
        assert!(!group_cycle(&all, "b", Some("a")));
        assert_eq!(jump_cycle(&all), None);
        all.hosts[1].jump_host_id = Some("x".into());
        assert!(jump_cycle(&all).is_some());
    }
}
