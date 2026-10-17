//! Tabelas `dashboards` e `widgets`.

use codatos_protocol::dashboard::{Dashboard, DashboardSummary, Widget};

use crate::{Error, Store};

impl Store {
    pub async fn dashboards(&self) -> Result<Vec<DashboardSummary>, Error> {
        let rows: Vec<(String, String, i64)> =
            sqlx::query_as("SELECT id, name, position FROM dashboards ORDER BY position, id")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows
            .into_iter()
            .map(|(id, name, position)| DashboardSummary { id, name, position })
            .collect())
    }

    /// `None` quando o dashboard não existe.
    pub async fn dashboard(&self, id: &str) -> Result<Option<Dashboard>, Error> {
        let Some((id, name, position)): Option<(String, String, i64)> =
            sqlx::query_as("SELECT id, name, position FROM dashboards WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?
        else {
            return Ok(None);
        };
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT id, kind, layout, config FROM widgets WHERE dashboard_id = ? ORDER BY position",
        )
        .bind(&id)
        .fetch_all(&self.pool)
        .await?;
        let widgets = rows
            .into_iter()
            .map(|(id, kind, layout, config)| {
                Ok(Widget {
                    id,
                    kind,
                    layout: serde_json::from_str(&layout)?,
                    config: serde_json::from_str(&config)?,
                })
            })
            .collect::<Result<_, Error>>()?;
        Ok(Some(Dashboard {
            id,
            name,
            position,
            widgets,
        }))
    }

    /// Cria ou atualiza o dashboard e troca todos os widgets dele, numa transação.
    pub async fn save_dashboard(&self, dashboard: &Dashboard) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO dashboards (id, name, position) VALUES (?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name, position = excluded.position,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(&dashboard.id)
        .bind(&dashboard.name)
        .bind(dashboard.position)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM widgets WHERE dashboard_id = ?")
            .bind(&dashboard.id)
            .execute(&mut *tx)
            .await?;
        for (position, widget) in (0i64..).zip(&dashboard.widgets) {
            sqlx::query(
                "INSERT INTO widgets (id, dashboard_id, kind, layout, config, position)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(&widget.id)
            .bind(&dashboard.id)
            .bind(&widget.kind)
            .bind(widget.layout.to_string())
            .bind(widget.config.to_string())
            .bind(position)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// `false` quando o dashboard não existia. Os widgets vão junto (`ON DELETE CASCADE`).
    pub async fn delete_dashboard(&self, id: &str) -> Result<bool, Error> {
        let result = sqlx::query("DELETE FROM dashboards WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn widget(id: &str, kind: &str) -> Widget {
        Widget {
            id: id.to_owned(),
            kind: kind.to_owned(),
            layout: json!({"lg": {"x": 0, "y": 0, "w": 3, "h": 2}}),
            config: json!({}),
        }
    }

    #[tokio::test]
    async fn salva_le_troca_widgets_e_apaga() {
        let store = Store::in_memory().await.unwrap();
        let mut d = Dashboard {
            id: "d1".to_owned(),
            name: "Dia a dia".to_owned(),
            position: 0,
            widgets: vec![widget("w1", "disks"), widget("w2", "weather")],
        };
        store.save_dashboard(&d).await.unwrap();
        assert_eq!(store.dashboard("d1").await.unwrap(), Some(d.clone()));

        // Salvar de novo troca a lista inteira, na ordem enviada.
        d.name = "Homelab".to_owned();
        d.widgets = vec![widget("w3", "wsl"), widget("w1", "disks")];
        store.save_dashboard(&d).await.unwrap();
        assert_eq!(store.dashboard("d1").await.unwrap(), Some(d));
        assert_eq!(
            store.dashboards().await.unwrap(),
            [DashboardSummary {
                id: "d1".to_owned(),
                name: "Homelab".to_owned(),
                position: 0
            }]
        );

        assert!(store.delete_dashboard("d1").await.unwrap());
        assert!(!store.delete_dashboard("d1").await.unwrap());
        assert_eq!(store.dashboard("d1").await.unwrap(), None);
        let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM widgets")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn persiste_em_arquivo() {
        let dir = std::env::temp_dir().join(format!("codatos-store-{}", std::process::id()));
        let path = dir.join("codatos.db");
        let _ = std::fs::remove_dir_all(&dir);
        {
            let store = Store::open(&path).await.unwrap();
            store
                .save_dashboard(&Dashboard {
                    id: "d1".to_owned(),
                    name: "Dia a dia".to_owned(),
                    position: 0,
                    widgets: vec![widget("w1", "disks")],
                })
                .await
                .unwrap();
            store.pool.close().await;
        }
        let store = Store::open(&path).await.unwrap();
        assert_eq!(
            store.dashboard("d1").await.unwrap().unwrap().widgets.len(),
            1
        );
        store.pool.close().await;
        let _ = std::fs::remove_dir_all(&dir);
    }
}
