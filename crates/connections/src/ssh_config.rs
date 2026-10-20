//! `~/.ssh/config` (RF-CON-005): leitura dos blocos `Host` e escrita dos hosts cadastrados.
//!
//! Só o que vira cadastro é interpretado: `HostName`, `Port`, `User`, `IdentityFile` e
//! `ProxyJump`. As demais opções do bloco vão para a aba "Avançado" do host
//! (`advanced.sshConfig`) e voltam na exportação. Blocos com curinga (`Host *`), `Match` e
//! `Include` são ignorados, com aviso.

use std::fmt::Write as _;

use serde_json::{Map, Value};

/// Um bloco `Host` sem curingas.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigHost {
    /// Primeiro nome do `Host` (o apelido usado no `ssh apelido`).
    pub alias: String,
    pub hostname: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub identity_files: Vec<String>,
    /// Saltos do `ProxyJump`, do primeiro ao último (`none` vira vazio).
    pub proxy_jump: Vec<String>,
    /// Outras opções, na ordem do arquivo, com o nome como escrito.
    pub other: Vec<(String, String)>,
}

/// Resultado da leitura: os hosts e o que foi ignorado.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Parsed {
    pub hosts: Vec<ConfigHost>,
    pub warnings: Vec<String>,
}

/// Separa `Chave valor` ou `Chave=valor`, tirando aspas do valor.
fn split_line(line: &str) -> Option<(&str, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let end = line.find(|c: char| c.is_whitespace() || c == '=')?;
    let key = &line[..end];
    let rest = line[end..].trim_start();
    let rest = rest.strip_prefix('=').unwrap_or(rest).trim();
    Some((key, unquote(rest)))
}

fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(value)
        .to_owned()
}

/// Lê o texto de um `ssh_config`. Vale o primeiro valor de cada opção, como no OpenSSH.
#[must_use]
pub fn parse(text: &str) -> Parsed {
    let mut parsed = Parsed::default();
    // `None`: fora de bloco ou em bloco ignorado.
    let mut current: Option<ConfigHost> = None;
    for (n, line) in text.lines().enumerate() {
        let Some((key, value)) = split_line(line) else {
            continue;
        };
        let lower = key.to_ascii_lowercase();
        match lower.as_str() {
            "host" => {
                parsed.hosts.extend(current.take());
                let patterns: Vec<&str> = value.split_whitespace().collect();
                let alias = patterns
                    .iter()
                    .find(|p| !p.contains(['*', '?', '!']))
                    .map(|p| (*p).to_owned());
                match alias {
                    Some(alias) if patterns.len() == 1 => {
                        current = Some(ConfigHost {
                            alias,
                            ..ConfigHost::default()
                        });
                    }
                    Some(alias) => {
                        parsed.warnings.push(format!(
                            "Linha {}: `Host {value}` tem vários nomes; usado só {alias}",
                            n + 1
                        ));
                        current = Some(ConfigHost {
                            alias,
                            ..ConfigHost::default()
                        });
                    }
                    None => {
                        if value != "*" {
                            parsed.warnings.push(format!(
                                "Linha {}: `Host {value}` ignorado (curinga)",
                                n + 1
                            ));
                        }
                    }
                }
            }
            "match" => {
                parsed.hosts.extend(current.take());
                parsed
                    .warnings
                    .push(format!("Linha {}: bloco `Match` ignorado", n + 1));
            }
            "include" => parsed
                .warnings
                .push(format!("Linha {}: `Include {value}` não é lido", n + 1)),
            _ => {
                let Some(host) = current.as_mut() else {
                    continue;
                };
                match lower.as_str() {
                    "hostname" => {
                        host.hostname.get_or_insert(value);
                    }
                    "port" => {
                        if host.port.is_none() {
                            host.port = value.parse().ok().filter(|p| *p > 0);
                        }
                    }
                    "user" => {
                        host.user.get_or_insert(value);
                    }
                    "identityfile" => host.identity_files.push(value),
                    "proxyjump" => {
                        if host.proxy_jump.is_empty() && !value.eq_ignore_ascii_case("none") {
                            host.proxy_jump = value
                                .split(',')
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .map(str::to_owned)
                                .collect();
                        }
                    }
                    _ => host.other.push((key.to_owned(), value)),
                }
            }
        }
    }
    parsed.hosts.extend(current);
    parsed
}

/// Um salto do `ProxyJump`: apelido, ou `[usuário@]endereço[:porta]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jump {
    pub user: Option<String>,
    pub address: String,
    pub port: Option<u16>,
}

impl Jump {
    #[must_use]
    pub fn parse(spec: &str) -> Self {
        let spec = spec.strip_prefix("ssh://").unwrap_or(spec);
        let (user, rest) = match spec.rsplit_once('@') {
            Some((user, rest)) => (Some(user.to_owned()), rest),
            None => (None, spec),
        };
        // `[::1]:22` ou `host:22`; IPv6 sem colchetes fica inteiro.
        let (address, port) = if let Some(inner) = rest.strip_prefix('[') {
            match inner.split_once(']') {
                Some((addr, tail)) => (addr, tail.strip_prefix(':').and_then(|p| p.parse().ok())),
                None => (rest, None),
            }
        } else {
            match rest.rsplit_once(':') {
                Some((addr, port)) if !addr.contains(':') => (addr, port.parse().ok()),
                _ => (rest, None),
            }
        };
        Self {
            user,
            address: address.to_owned(),
            port,
        }
    }
}

/// O que a exportação precisa saber de cada host, já com a herança aplicada.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportHost {
    pub alias: String,
    pub address: String,
    pub port: u16,
    pub user: Option<String>,
    /// Nome da chave no cofre: não há arquivo para o `IdentityFile`.
    pub key_name: Option<String>,
    /// Apelidos dos saltos, do primeiro ao último.
    pub proxy_jump: Vec<String>,
    pub other: Vec<(String, String)>,
}

/// Texto `ssh_config` dos hosts.
#[must_use]
pub fn render(hosts: &[ExportHost]) -> String {
    let mut out = String::from("# Gerado pelo CodatOS. As chaves ficam no cofre do CodatOS.\n");
    for h in hosts {
        let _ = write!(
            out,
            "\nHost {}\n    HostName {}\n",
            alias(&h.alias),
            h.address
        );
        if h.port != 22 {
            let _ = writeln!(out, "    Port {}", h.port);
        }
        if let Some(user) = &h.user {
            let _ = writeln!(out, "    User {}", quote(user));
        }
        if !h.proxy_jump.is_empty() {
            let hops: Vec<String> = h.proxy_jump.iter().map(|j| alias(j)).collect();
            let _ = writeln!(out, "    ProxyJump {}", hops.join(","));
        }
        for (key, value) in &h.other {
            let _ = writeln!(out, "    {key} {}", quote(value));
        }
        if let Some(key) = &h.key_name {
            let _ = writeln!(out, "    # Chave \"{key}\" no cofre do CodatOS");
        }
    }
    out
}

/// Apelido válido no `Host`: espaços viram `-`.
#[must_use]
pub fn alias(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join("-")
}

fn quote(value: &str) -> String {
    if value.contains(char::is_whitespace) {
        format!("\"{value}\"")
    } else {
        value.to_owned()
    }
}

/// Opções extras guardadas em `advanced.sshConfig` (objeto `nome → valor`).
#[must_use]
pub fn other_to_json(other: &[(String, String)]) -> Value {
    let map: Map<String, Value> = other
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    Value::Object(map)
}

#[must_use]
pub fn other_from_json(advanced: &Value) -> Vec<(String, String)> {
    advanced
        .get("sshConfig")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
                // Só nomes de opção: nada de quebras de linha no arquivo exportado.
                .filter(|(k, v)| {
                    k.chars().all(|c| c.is_ascii_alphanumeric()) && !v.contains(['\n', '\r'])
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = r#"
# comentário
Include ~/.ssh/config.d/*

Host *
    ServerAliveInterval 60

Host bastiao
    HostName bastion.exemplo.com
    User ops
    IdentityFile ~/.ssh/id_ed25519

Host web
    HostName=10.0.0.5
    Port 2222
    User deploy
    User ignorado
    ProxyJump bastiao
    IdentityFile "~/.ssh/minha chave"
    ForwardAgent yes

Host *.interno !x
    User x

Host db
    HostName 10.0.0.9
    ProxyJump ops@bastion.exemplo.com:2200,web

Match host foo
    User y
"#;

    #[test]
    fn le_os_blocos() {
        let parsed = parse(CONFIG);
        let aliases: Vec<&str> = parsed.hosts.iter().map(|h| h.alias.as_str()).collect();
        assert_eq!(aliases, ["bastiao", "web", "db"]);
        let web = &parsed.hosts[1];
        assert_eq!(web.hostname.as_deref(), Some("10.0.0.5"));
        assert_eq!(web.port, Some(2222));
        assert_eq!(web.user.as_deref(), Some("deploy"));
        assert_eq!(web.identity_files, ["~/.ssh/minha chave"]);
        assert_eq!(web.proxy_jump, ["bastiao"]);
        assert_eq!(web.other, [("ForwardAgent".to_owned(), "yes".to_owned())]);
        assert_eq!(
            parsed.hosts[2].proxy_jump,
            ["ops@bastion.exemplo.com:2200", "web"]
        );
        // Include, curinga e Match avisam; `Host *` não.
        assert_eq!(parsed.warnings.len(), 3, "{:?}", parsed.warnings);
    }

    #[test]
    fn saltos() {
        assert_eq!(
            Jump::parse("ops@bastion:2200"),
            Jump {
                user: Some("ops".into()),
                address: "bastion".into(),
                port: Some(2200)
            }
        );
        assert_eq!(Jump::parse("[::1]:22").address, "::1");
        assert_eq!(Jump::parse("fe80::1").port, None);
        assert_eq!(Jump::parse("bastiao").user, None);
    }

    #[test]
    fn exporta_e_le_de_novo() {
        let text = render(&[
            ExportHost {
                alias: "bastiao".into(),
                address: "bastion.exemplo.com".into(),
                port: 22,
                user: Some("ops".into()),
                key_name: Some("Notebook".into()),
                ..ExportHost::default()
            },
            ExportHost {
                alias: "meu web".into(),
                address: "10.0.0.5".into(),
                port: 2222,
                proxy_jump: vec!["bastiao".into()],
                other: vec![("ForwardAgent".into(), "yes".into())],
                ..ExportHost::default()
            },
        ]);
        assert!(text.contains("# Chave \"Notebook\""));
        let parsed = parse(&text);
        assert_eq!(parsed.hosts.len(), 2);
        assert_eq!(parsed.hosts[1].alias, "meu-web");
        assert_eq!(parsed.hosts[1].port, Some(2222));
        assert_eq!(parsed.hosts[1].proxy_jump, ["bastiao"]);
        assert_eq!(parsed.hosts[1].other.len(), 1);
    }

    #[test]
    fn opcoes_extras_sem_quebra_de_linha() {
        let advanced = serde_json::json!({"sshConfig": {"ForwardAgent": "yes", "Bad\nKey": "x", "Ok": "a\nb"}});
        assert_eq!(
            other_from_json(&advanced),
            [("ForwardAgent".to_owned(), "yes".to_owned())]
        );
    }
}
