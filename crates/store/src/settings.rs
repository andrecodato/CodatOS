//! Tabela `settings`: chave → JSON, por escopo.

use codatos_protocol::dashboard::SettingScope;
use serde_json::Value;

use crate::{Error, Store};

fn scope_name(scope: SettingScope) -> &'static str {
    match scope {
        SettingScope::Global => "global",
        SettingScope::Device => "device",
    }
}

impl Store {
    /// `None` quando a preferência não existe.
    pub async fn setting(&self, key: &str, scope: SettingScope) -> Result<Option<Value>, Error> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT value FROM settings WHERE key = ? AND scope = ?")
                .bind(key)
                .bind(scope_name(scope))
                .fetch_optional(&self.pool)
                .await?;
        row.map(|(v,)| serde_json::from_str(&v).map_err(Error::from))
            .transpose()
    }

    /// `Value::Null` apaga a preferência.
    pub async fn set_setting(
        &self,
        key: &str,
        scope: SettingScope,
        value: &Value,
    ) -> Result<(), Error> {
        if value.is_null() {
            sqlx::query("DELETE FROM settings WHERE key = ? AND scope = ?")
                .bind(key)
                .bind(scope_name(scope))
                .execute(&self.pool)
                .await?;
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO settings (key, scope, value) VALUES (?, ?, ?)
             ON CONFLICT (key, scope) DO UPDATE SET value = excluded.value,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(key)
        .bind(scope_name(scope))
        .bind(value.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn grava_le_e_apaga() {
        let store = Store::in_memory().await.unwrap();
        let g = SettingScope::Global;
        assert_eq!(store.setting("tema", g).await.unwrap(), None);

        store
            .set_setting("tema", g, &json!("escuro"))
            .await
            .unwrap();
        store
            .set_setting("tema", g, &json!({"modo": "claro"}))
            .await
            .unwrap();
        assert_eq!(
            store.setting("tema", g).await.unwrap(),
            Some(json!({"modo": "claro"}))
        );
        // Escopos não se misturam.
        assert_eq!(
            store.setting("tema", SettingScope::Device).await.unwrap(),
            None
        );

        store.set_setting("tema", g, &Value::Null).await.unwrap();
        assert_eq!(store.setting("tema", g).await.unwrap(), None);
    }
}
