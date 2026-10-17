//! Catálogo de widgets (RF-DASH-003): o que pode ir para a grade, com tamanho padrão e mínimo.

use leptos::prelude::*;
use serde_json::Value;

use super::widgets;
use crate::components::icon::Icon;

/// Um tipo de widget. Cada instância na grade tem a própria configuração (RF-DASH-004).
pub struct Kind {
    /// Gravado em `Widget::kind`; não muda depois de publicado.
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: Icon,
    /// Largura × altura ao adicionar, em células.
    pub size: (u16, u16),
    /// Menor tamanho em que o conteúdo ainda cabe.
    pub min: (u16, u16),
    pub render: fn(&Value) -> AnyView,
}

impl std::fmt::Debug for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kind")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

pub const KINDS: &[Kind] = &[
    Kind {
        id: "system.usage",
        name: "Uso do PC",
        description: "CPU e memória agora.",
        icon: Icon::Cpu,
        size: (4, 2),
        min: (3, 2),
        render: widgets::usage::render,
    },
    Kind {
        id: "clock",
        name: "Relógio",
        description: "Hora e data deste PC.",
        icon: Icon::Clock,
        size: (3, 2),
        min: (2, 2),
        render: widgets::clock::render,
    },
];

pub fn find(id: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|k| k.id == id)
}

/// Busca por nome ou descrição, sem diferenciar maiúsculas e acentos.
pub fn search(query: &str) -> Vec<&'static Kind> {
    let query = fold(query.trim());
    KINDS
        .iter()
        .filter(|k| {
            query.is_empty()
                || fold(k.name).contains(&query)
                || fold(k.description).contains(&query)
        })
        .collect()
}

fn fold(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'Á' | 'À' | 'Â' | 'Ã' => 'a',
            'é' | 'ê' | 'É' | 'Ê' => 'e',
            'í' | 'Í' => 'i',
            'ó' | 'ô' | 'õ' | 'Ó' | 'Ô' | 'Õ' => 'o',
            'ú' | 'Ú' => 'u',
            'ç' | 'Ç' => 'c',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_unicos_e_tamanhos_validos() {
        for (i, k) in KINDS.iter().enumerate() {
            assert!(
                KINDS[i + 1..].iter().all(|o| o.id != k.id),
                "{} repetido",
                k.id
            );
            assert!(k.min.0 <= k.size.0 && k.min.1 <= k.size.1, "{}", k.id);
            assert!(k.size.0 <= super::super::grid::COLUMNS, "{}", k.id);
        }
    }

    #[test]
    fn busca_ignora_acentos() {
        assert_eq!(search("relogio")[0].id, "clock");
        assert_eq!(search("MEMÓRIA")[0].id, "system.usage");
        assert_eq!(search("").len(), KINDS.len());
        assert!(search("xyz").is_empty());
    }
}
