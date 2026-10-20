//! Tabela `tunnels` (RF-CON-040). Abrir e fechar fica no crate de conexões.

use codatos_protocol::tunnels::{Tunnel, TunnelKind};

use crate::{Error, Store};

type Row = (
    String,
    String,
    String,
    String,
    String,
    i64,
    String,
    i64,
    bool,
);

fn kind_str(kind: TunnelKind) -> &'static str {
    match kind {
        TunnelKind::Local => "local",
        TunnelKind::Remote => "remote",
        TunnelKind::Dynamic => "dynamic",
    }
}

fn from_row(row: Row) -> Tunnel {
    let (id, host_id, name, kind, bind_host, bind_port, target_host, target_port, auto_start) = row;
    Tunnel {
        id,
        name,
        host_id,
        kind: match kind.as_str() {
            "remote" => TunnelKind::Remote,
            "dynamic" => TunnelKind::Dynamic,
            _ => TunnelKind::Local,
        },
        bind_host,
        bind_port: u16::try_from(bind_port).unwrap_or(0),
        target_host,
        target_port: u16::try_from(target_port).unwrap_or(0),
        auto_start,
    }
}

impl Store {
    /// Todos os túneis, por nome.
    pub async fn tunnels(&self) -> Result<Vec<Tunnel>, Error> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, host_id, name, kind, bind_host, bind_port, target_host, target_port,
                    auto_start
             FROM tunnels ORDER BY name COLLATE NOCASE, id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn tunnel(&self, id: &str) -> Result<Option<Tunnel>, Error> {
        let row: Option<Row> = sqlx::query_as(
            "SELECT id, host_id, name, kind, bind_host, bind_port, target_host, target_port,
                    auto_start
             FROM tunnels WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(from_row))
    }

    pub async fn save_tunnel(&self, tunnel: &Tunnel) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO tunnels (id, host_id, name, kind, bind_host, bind_port, target_host,
                                  target_port, auto_start)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET host_id = excluded.host_id, name = excluded.name,
                 kind = excluded.kind, bind_host = excluded.bind_host,
                 bind_port = excluded.bind_port, target_host = excluded.target_host,
                 target_port = excluded.target_port, auto_start = excluded.auto_start,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(&tunnel.id)
        .bind(&tunnel.host_id)
        .bind(&tunnel.name)
        .bind(kind_str(tunnel.kind))
        .bind(&tunnel.bind_host)
        .bind(i64::from(tunnel.bind_port))
        .bind(&tunnel.target_host)
        .bind(i64::from(tunnel.target_port))
        .bind(tunnel.auto_start)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_tunnel(&self, id: &str) -> Result<bool, Error> {
        let done = sqlx::query("DELETE FROM tunnels WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(done.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codatos_protocol::connections::Host;

    #[tokio::test]
    async fn salva_le_e_some_com_o_host() {
        let store = Store::in_memory().await.unwrap();
        let host: Host = serde_json::from_value(
            serde_json::json!({"id": "h1", "name": "srv", "address": "10.0.0.1"}),
        )
        .unwrap();
        store.save_host(&host).await.unwrap();
        let tunnel = Tunnel {
            id: "t1".into(),
            name: "SOCKS".into(),
            host_id: "h1".into(),
            kind: TunnelKind::Dynamic,
            bind_host: "127.0.0.1".into(),
            bind_port: 1080,
            target_host: String::new(),
            target_port: 0,
            auto_start: true,
        };
        store.save_tunnel(&tunnel).await.unwrap();
        assert_eq!(store.tunnel("t1").await.unwrap(), Some(tunnel.clone()));
        assert_eq!(store.tunnels().await.unwrap(), vec![tunnel]);
        store.delete_host("h1").await.unwrap();
        assert_eq!(store.tunnels().await.unwrap(), Vec::<Tunnel>::new());
    }
}
