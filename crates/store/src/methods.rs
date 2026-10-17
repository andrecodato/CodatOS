//! Métodos JSON-RPC do banco: `settings.*` e `dashboards.*`.

use std::sync::Arc;

use codatos_core::{Method, MethodError, MethodFuture, params};
use codatos_protocol::dashboard::{
    Dashboard, DashboardId, METHOD_DASHBOARDS_DELETE, METHOD_DASHBOARDS_GET,
    METHOD_DASHBOARDS_LIST, METHOD_DASHBOARDS_SAVE, METHOD_SETTINGS_GET, METHOD_SETTINGS_SET,
    SetSetting, SettingKey,
};
use serde_json::{Value, json};

use crate::{Error, Store};

/// Limites para um cliente não encher o banco por engano.
const MAX_WIDGETS: usize = 200;
const MAX_JSON_BYTES: usize = 64 * 1024;

/// Os métodos do banco, para `ServerConfig::methods`.
pub fn methods(store: &Store) -> Vec<Arc<dyn Method>> {
    [
        METHOD_SETTINGS_GET,
        METHOD_SETTINGS_SET,
        METHOD_DASHBOARDS_LIST,
        METHOD_DASHBOARDS_GET,
        METHOD_DASHBOARDS_SAVE,
        METHOD_DASHBOARDS_DELETE,
    ]
    .into_iter()
    .map(|name| {
        Arc::new(StoreMethod {
            name,
            store: store.clone(),
        }) as Arc<dyn Method>
    })
    .collect()
}

#[derive(Debug)]
struct StoreMethod {
    name: &'static str,
    store: Store,
}

impl Method for StoreMethod {
    fn name(&self) -> &'static str {
        self.name
    }

    fn call(&self, value: Value) -> MethodFuture<'_> {
        Box::pin(async move {
            let store = &self.store;
            let result = match self.name {
                METHOD_SETTINGS_GET => {
                    let p: SettingKey = params(value)?;
                    json!(store.setting(&p.key, p.scope).await.map_err(internal)?)
                }
                METHOD_SETTINGS_SET => {
                    let p: SetSetting = params(value)?;
                    check_size(&p.value)?;
                    store
                        .set_setting(&p.key, p.scope, &p.value)
                        .await
                        .map_err(internal)?;
                    Value::Null
                }
                METHOD_DASHBOARDS_LIST => json!(store.dashboards().await.map_err(internal)?),
                METHOD_DASHBOARDS_GET => {
                    let p: DashboardId = params(value)?;
                    let dashboard = store.dashboard(&p.id).await.map_err(internal)?;
                    json!(dashboard.ok_or_else(|| {
                        MethodError::new("dashboard_not_found", "Dashboard não encontrado")
                    })?)
                }
                METHOD_DASHBOARDS_SAVE => {
                    let d: Dashboard = params(value)?;
                    validate(&d)?;
                    store.save_dashboard(&d).await.map_err(internal)?;
                    Value::Null
                }
                METHOD_DASHBOARDS_DELETE => {
                    let p: DashboardId = params(value)?;
                    json!(store.delete_dashboard(&p.id).await.map_err(internal)?)
                }
                _ => unreachable!("só os nomes de `methods`"),
            };
            Ok(result)
        })
    }
}

fn validate(d: &Dashboard) -> Result<(), MethodError> {
    if d.id.is_empty() || d.name.trim().is_empty() {
        return Err(MethodError::invalid_params("Dashboard sem id ou nome"));
    }
    if d.widgets.len() > MAX_WIDGETS {
        return Err(MethodError::invalid_params(format!(
            "No máximo {MAX_WIDGETS} widgets por dashboard"
        )));
    }
    let mut ids = std::collections::HashSet::new();
    for w in &d.widgets {
        if w.id.is_empty() || !ids.insert(&w.id) {
            return Err(MethodError::invalid_params(
                "Widget sem id ou com id repetido",
            ));
        }
        check_size(&w.layout)?;
        check_size(&w.config)?;
    }
    Ok(())
}

fn check_size(value: &Value) -> Result<(), MethodError> {
    if value.to_string().len() > MAX_JSON_BYTES {
        return Err(MethodError::invalid_params("Valor grande demais"));
    }
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
fn internal(error: Error) -> MethodError {
    tracing::error!(%error, "erro no banco local");
    MethodError::new("store_failed", "Erro no banco local")
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn call(store: &Store, name: &str, value: Value) -> Result<Value, MethodError> {
        let all = methods(store);
        let method = all.iter().find(|m| m.name() == name).unwrap();
        method.call(value).await
    }

    #[tokio::test]
    async fn dashboards_pela_api() {
        let store = Store::in_memory().await.unwrap();
        let d = json!({"id": "d1", "name": "Dia a dia", "position": 0, "widgets": [
            {"id": "w1", "kind": "disks", "layout": {"lg": {"x": 0, "y": 0, "w": 3, "h": 2}}, "config": {}}
        ]});
        assert_eq!(
            call(&store, "dashboards.save", d.clone()).await,
            Ok(Value::Null)
        );
        assert_eq!(
            call(&store, "dashboards.get", json!({"id": "d1"})).await,
            Ok(d)
        );

        let err = call(&store, "dashboards.get", json!({"id": "x"}))
            .await
            .unwrap_err();
        assert_eq!(err.code(), "dashboard_not_found");

        let repeated = json!({"id": "d2", "name": "x", "position": 1, "widgets": [
            {"id": "w", "kind": "a", "layout": {}, "config": {}},
            {"id": "w", "kind": "b", "layout": {}, "config": {}}
        ]});
        let err = call(&store, "dashboards.save", repeated).await.unwrap_err();
        assert_eq!(err.code(), "invalid_params");
    }

    #[tokio::test]
    async fn settings_pela_api() {
        let store = Store::in_memory().await.unwrap();
        call(
            &store,
            "settings.set",
            json!({"key": "nome", "value": "André"}),
        )
        .await
        .unwrap();
        assert_eq!(
            call(&store, "settings.get", json!({"key": "nome"})).await,
            Ok(json!("André"))
        );
        assert_eq!(
            call(
                &store,
                "settings.get",
                json!({"key": "nome", "scope": "device"})
            )
            .await,
            Ok(Value::Null)
        );
    }
}
