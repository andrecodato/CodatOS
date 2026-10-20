//! Tabela `snippets` e os métodos `snippets.*` (RF-CON-023).

use std::sync::Arc;

use codatos_core::{Method, MethodError, MethodFuture, params};
use codatos_protocol::snippets::{
    METHOD_SNIPPET_DELETE, METHOD_SNIPPET_SAVE, METHOD_SNIPPETS_LIST, Snippet, SnippetId, Snippets,
    errors,
};
use serde_json::{Value, json};

use crate::{Error, Store};

const MAX_ID: usize = 64;
const MAX_NAME: usize = 200;
const MAX_COMMAND: usize = 16 * 1024;

impl Store {
    pub async fn snippets(&self) -> Result<Snippets, Error> {
        let rows: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT id, name, command FROM snippets ORDER BY name COLLATE NOCASE, id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(Snippets {
            snippets: rows
                .into_iter()
                .map(|(id, name, command)| Snippet { id, name, command })
                .collect(),
        })
    }

    pub async fn save_snippet(&self, snippet: &Snippet) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO snippets (id, name, command) VALUES (?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name, command = excluded.command,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(&snippet.id)
        .bind(&snippet.name)
        .bind(&snippet.command)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_snippet(&self, id: &str) -> Result<bool, Error> {
        let done = sqlx::query("DELETE FROM snippets WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(done.rows_affected() > 0)
    }
}

/// Os métodos de snippets, para `ServerConfig::methods`.
pub fn snippet_methods(store: &Store) -> Vec<Arc<dyn Method>> {
    [
        METHOD_SNIPPETS_LIST,
        METHOD_SNIPPET_SAVE,
        METHOD_SNIPPET_DELETE,
    ]
    .into_iter()
    .map(|name| {
        Arc::new(SnippetMethod {
            name,
            store: store.clone(),
        }) as Arc<dyn Method>
    })
    .collect()
}

#[derive(Debug)]
struct SnippetMethod {
    name: &'static str,
    store: Store,
}

impl Method for SnippetMethod {
    fn name(&self) -> &'static str {
        self.name
    }

    fn call(&self, value: Value) -> MethodFuture<'_> {
        Box::pin(async move {
            match self.name {
                METHOD_SNIPPETS_LIST => {
                    return Ok(json!(self.store.snippets().await.map_err(internal)?));
                }
                METHOD_SNIPPET_SAVE => {
                    let snippet: Snippet = params(value)?;
                    if snippet.id.is_empty() || snippet.id.len() > MAX_ID {
                        return Err(MethodError::invalid_params("Id inválido"));
                    }
                    if snippet.name.trim().is_empty() || snippet.name.len() > MAX_NAME {
                        return Err(MethodError::invalid_params("Nome vazio ou grande demais"));
                    }
                    if snippet.command.trim().is_empty() || snippet.command.len() > MAX_COMMAND {
                        return Err(MethodError::invalid_params(
                            "Comando vazio ou grande demais",
                        ));
                    }
                    self.store.save_snippet(&snippet).await.map_err(internal)?;
                }
                METHOD_SNIPPET_DELETE => {
                    let p: SnippetId = params(value)?;
                    if !self.store.delete_snippet(&p.id).await.map_err(internal)? {
                        return Err(MethodError::new(
                            errors::NOT_FOUND,
                            "Snippet não encontrado",
                        ));
                    }
                }
                _ => unreachable!("método registrado sem implementação: {}", self.name),
            }
            Ok(Value::Null)
        })
    }
}

#[allow(clippy::needless_pass_by_value)]
fn internal(error: Error) -> MethodError {
    tracing::error!(%error, "erro no banco local");
    MethodError::new("store_failed", "Erro no banco local")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn salva_lista_e_apaga() {
        let store = Store::in_memory().await.unwrap();
        let methods = snippet_methods(&store);
        let call = |name: &'static str, value: Value| {
            let method = methods.iter().find(|m| m.name() == name).unwrap().clone();
            async move { method.call(value).await }
        };
        call(
            METHOD_SNIPPET_SAVE,
            json!({"id": "s2", "name": "uptime", "command": "uptime"}),
        )
        .await
        .unwrap();
        call(
            METHOD_SNIPPET_SAVE,
            json!({"id": "s1", "name": "Logs", "command": "journalctl -u {{serviço}} -f"}),
        )
        .await
        .unwrap();
        call(
            METHOD_SNIPPET_SAVE,
            json!({"id": "s2", "name": "Uptime", "command": "uptime -p"}),
        )
        .await
        .unwrap();
        let list = call(METHOD_SNIPPETS_LIST, json!({})).await.unwrap();
        assert_eq!(list["snippets"][0]["name"], "Logs");
        assert_eq!(list["snippets"][1]["command"], "uptime -p");
        let err = call(
            METHOD_SNIPPET_SAVE,
            json!({"id": "s3", "name": "vazio", "command": "  "}),
        )
        .await
        .unwrap_err();
        assert_eq!(err.code(), "invalid_params");
        call(METHOD_SNIPPET_DELETE, json!({"id": "s1"}))
            .await
            .unwrap();
        let err = call(METHOD_SNIPPET_DELETE, json!({"id": "s1"}))
            .await
            .unwrap_err();
        assert_eq!(err.code(), errors::NOT_FOUND);
        assert_eq!(store.snippets().await.unwrap().snippets.len(), 1);
    }
}
