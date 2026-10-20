//! Snippets (RF-CON-023): comandos salvos para mandar a um ou vários terminais. Variáveis
//! são `{{nome}}` no próprio comando; a UI pergunta o valor de cada uma antes de mandar.

use serde::{Deserialize, Serialize};

/// Lista os snippets; devolve [`Snippets`].
pub const METHOD_SNIPPETS_LIST: &str = "snippets.list";
/// Cria ou atualiza ([`Snippet`]).
pub const METHOD_SNIPPET_SAVE: &str = "snippets.save";
/// Apaga ([`SnippetId`]).
pub const METHOD_SNIPPET_DELETE: &str = "snippets.delete";

pub mod errors {
    pub const NOT_FOUND: &str = "snippet_not_found";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub command: String,
}

/// Resposta de `snippets.list`, por nome.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippets {
    pub snippets: Vec<Snippet>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetId {
    pub id: String,
}

/// Nomes das variáveis `{{nome}}` do comando, na ordem em que aparecem, sem repetir.
/// Nome é letra, número, `_` ou `-`; o resto entre chaves duplas fica como texto.
#[must_use]
pub fn variables(command: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for (_, name, _) in placeholders(command) {
        if !names.iter().any(|n| n == name) {
            names.push(name.to_owned());
        }
    }
    names
}

/// O comando com cada `{{nome}}` trocado pelo valor; variável sem valor fica como está.
#[must_use]
pub fn fill(command: &str, value: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(command.len());
    let mut last = 0;
    for (start, name, end) in placeholders(command) {
        if let Some(v) = value(name) {
            out.push_str(&command[last..start]);
            out.push_str(&v);
            last = end;
        }
    }
    out.push_str(&command[last..]);
    out
}

/// Cada `{{nome}}`: início, nome e fim (exclusivo), em bytes.
fn placeholders(command: &str) -> Vec<(usize, &str, usize)> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(open) = command[from..].find("{{").map(|i| from + i) {
        let inner = open + 2;
        let Some(close) = command[inner..].find("}}").map(|i| inner + i) else {
            break;
        };
        let name = command[inner..close].trim();
        if !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            found.push((open, name, close + 2));
            from = close + 2;
        } else {
            from = inner;
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variaveis_e_preenchimento() {
        let command = "ssh {{ usuario }}@{{host}} -p {{porta}}; echo {{host}} {{}} {{a b}} {x}";
        assert_eq!(variables(command), ["usuario", "host", "porta"]);
        let filled = fill(command, |name| match name {
            "usuario" => Some("andre".into()),
            "host" => Some("srv".into()),
            _ => None,
        });
        assert_eq!(
            filled,
            "ssh andre@srv -p {{porta}}; echo srv {{}} {{a b}} {x}"
        );
        assert_eq!(variables("sem nada {{"), Vec::<String>::new());
        assert_eq!(variables("{{{{x}}"), ["x"]);
        assert_eq!(variables("{{ação}}"), ["ação"]);
    }
}
