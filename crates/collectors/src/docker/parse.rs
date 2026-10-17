//! Leitura das saídas `--format '{{json .}}'` da CLI do Docker.

use codatos_protocol::system::Container;
use serde::Deserialize;

pub(super) struct Info {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) version: String,
}

/// `docker info`. Sem `ID` (CLI sem engine ou engine parado) → `None`.
pub(super) fn info(text: &str) -> Option<Info> {
    #[derive(Deserialize)]
    struct Raw {
        #[serde(rename = "ID", default)]
        id: String,
        #[serde(rename = "Name", default)]
        name: String,
        #[serde(rename = "ServerVersion", default)]
        version: String,
    }
    let raw: Raw = serde_json::from_str(text.trim()).ok()?;
    (!raw.id.is_empty()).then_some(Info {
        id: raw.id,
        name: raw.name,
        version: raw.version,
    })
}

/// `docker ps --all`: um JSON por linha. Os em execução primeiro, depois por nome.
pub(super) fn containers(text: &str) -> Vec<Container> {
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Raw {
        #[serde(rename = "ID")]
        id: String,
        names: String,
        image: String,
        state: String,
        status: String,
        #[serde(default)]
        ports: String,
        #[serde(default)]
        labels: String,
    }
    let mut list: Vec<Container> = text
        .lines()
        .filter_map(|line| serde_json::from_str::<Raw>(line).ok())
        .map(|r| Container {
            id: r.id.chars().take(12).collect(),
            compose_project: label(&r.labels, "com.docker.compose.project"),
            name: r.names,
            image: r.image,
            state: r.state,
            status: r.status,
            ports: r.ports,
            cpu: None,
            memory: None,
        })
        .collect();
    list.sort_by(|a, b| {
        (b.state == "running")
            .cmp(&(a.state == "running"))
            .then_with(|| a.name.cmp(&b.name))
    });
    list
}

/// Valor de um rótulo na lista `a=1,b=2` do `docker ps`.
fn label(labels: &str, key: &str) -> Option<String> {
    labels
        .split(',')
        .filter_map(|pair| pair.split_once('='))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.to_owned())
}

/// Junta o `docker stats --no-stream` (CPU e memória) aos containers pelo ID.
pub(super) fn apply_stats(containers: &mut [Container], text: &str) {
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Raw {
        #[serde(rename = "ID")]
        id: String,
        #[serde(rename = "CPUPerc")]
        cpu_perc: String,
        mem_usage: String,
    }
    for raw in text
        .lines()
        .filter_map(|l| serde_json::from_str::<Raw>(l).ok())
    {
        let Some(container) = containers
            .iter_mut()
            .find(|c| raw.id.starts_with(&c.id) || c.id.starts_with(&raw.id))
        else {
            continue;
        };
        container.cpu = raw.cpu_perc.trim_end_matches('%').trim().parse().ok();
        container.memory = raw.mem_usage.split('/').next().and_then(bytes);
    }
}

/// `"12.5MiB"`, `"1.2GB"`, `"512kB"`, `"0B"` → bytes.
fn bytes(text: &str) -> Option<u64> {
    let text = text.trim();
    let split = text
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(split);
    let number: f64 = number.parse().ok()?;
    let factor: f64 = match unit.trim() {
        "B" | "" => 1.0,
        "kB" | "KB" => 1e3,
        "KiB" => 1024.0,
        "MB" => 1e6,
        "MiB" => 1024.0 * 1024.0,
        "GB" => 1e9,
        "GiB" => 1024.0 * 1024.0 * 1024.0,
        "TB" => 1e12,
        "TiB" => 1024f64.powi(4),
        _ => return None,
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some((number * factor).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_do_engine() {
        let i = info(
            r#"{"ID":"abc:def","Name":"docker-desktop","ServerVersion":"27.3.1","Containers":3}"#,
        )
        .unwrap();
        assert_eq!(i.id, "abc:def");
        assert_eq!(i.name, "docker-desktop");
        assert_eq!(i.version, "27.3.1");
        // Engine parado: a CLI responde só com a parte do cliente.
        assert!(info(r#"{"ID":"","ServerErrors":["Cannot connect"]}"#).is_none());
        assert!(info("").is_none());
    }

    #[test]
    fn containers_com_compose_e_ordem() {
        let text = [
            r#"{"ID":"bbbbbbbbbbbbbbbb","Names":"db","Image":"postgres:17","State":"exited","Status":"Exited (0) 2 days ago","Ports":"","Labels":""}"#,
            r#"{"ID":"aaaaaaaaaaaaaaaa","Names":"api","Image":"node:22","State":"running","Status":"Up 2 hours","Ports":"0.0.0.0:3000->3000/tcp","Labels":"com.docker.compose.project=loja,com.docker.compose.service=api"}"#,
            "linha quebrada",
        ]
        .join("\n");
        let list = containers(&text);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "api");
        assert_eq!(list[0].id, "aaaaaaaaaaaa");
        assert_eq!(list[0].compose_project.as_deref(), Some("loja"));
        assert_eq!(list[1].compose_project, None);
    }

    #[test]
    fn stats_por_id() {
        let mut list = containers(
            r#"{"ID":"aaaaaaaaaaaaaaaa","Names":"api","Image":"node","State":"running","Status":"Up","Ports":"","Labels":""}"#,
        );
        apply_stats(
            &mut list,
            r#"{"ID":"aaaaaaaaaaaa","Name":"api","CPUPerc":"1.50%","MemUsage":"12MiB / 7.6GiB"}"#,
        );
        assert_eq!(list[0].cpu, Some(1.5));
        assert_eq!(list[0].memory, Some(12 * 1024 * 1024));
    }

    #[test]
    fn unidades() {
        assert_eq!(bytes("0B"), Some(0));
        assert_eq!(bytes("512kB"), Some(512_000));
        assert_eq!(bytes("1.5GiB"), Some(1_610_612_736));
        assert_eq!(bytes("--"), None);
    }
}
