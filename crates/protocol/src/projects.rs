//! Projetos recentes (`docs/modulos/projetos.md`).

use serde::{Deserialize, Serialize};

/// Repositórios das pastas raiz e recentes dos editores, com o estado do git.
pub const TOPIC_PROJECTS: &str = "projects";

/// Abre um projeto no editor, no terminal ou no explorador de arquivos (RF-PROJ-004).
pub const METHOD_PROJECTS_OPEN: &str = "projects.open";

/// Parâmetros de `projects.open`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenProject {
    pub path: String,
    #[serde(default)]
    pub wsl_distro: Option<String>,
    pub target: OpenTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenTarget {
    /// VS Code (Q-15); dentro do WSL, pelo Remote - WSL.
    Editor,
    /// Windows Terminal na pasta do projeto.
    Terminal,
    Explorer,
}

/// Chave em `settings` (escopo `device`: caminhos são por dispositivo) com [`ProjectsConfig`].
pub const SETTING_PROJECTS: &str = "projects";

/// Configuração da descoberta (RF-PROJ-001). Sem ela, vale o padrão da Q-15:
/// `C:\dev` no Windows (`~/dev`, `~/projects`, `~/code` nos outros) e `~` nas distros WSL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectsConfig {
    pub roots: Vec<String>,
    /// Pastas dentro das distros WSL ligadas (`~` = home do usuário da distro).
    pub wsl_roots: Vec<String>,
    /// Profundidade máxima a partir de cada raiz.
    pub max_depth: u8,
    /// Importar recentes do VS Code e dos IDEs JetBrains (RF-PROJ-002).
    pub editors: bool,
}

impl Default for ProjectsConfig {
    fn default() -> Self {
        let roots = if cfg!(windows) {
            vec![r"C:\dev".to_owned()]
        } else {
            vec![
                "~/dev".to_owned(),
                "~/projects".to_owned(),
                "~/code".to_owned(),
            ]
        };
        Self {
            roots,
            wsl_roots: vec!["~".to_owned()],
            max_depth: 3,
            editors: true,
        }
    }
}

/// Item da amostra de `projects`, do mais recente para o mais antigo (RF-PROJ-003).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub name: String,
    pub path: String,
    /// Distro WSL onde o projeto está; `None` no host.
    pub wsl_distro: Option<String>,
    /// `None` quando a pasta não é um repositório git (veio só do editor).
    pub git: Option<GitStatus>,
    /// Último uso num editor (Unix, segundos).
    pub last_opened_at: Option<i64>,
    /// Editor do último uso: `vscode` ou `jetbrains`.
    pub editor: Option<String>,
    /// Linguagem principal pelos arquivos da raiz (Cargo.toml → Rust…).
    pub language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    /// `None` com HEAD destacado.
    pub branch: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    /// Arquivos modificados, novos ou em conflito.
    pub changes: u32,
    /// Unix, segundos.
    pub last_commit_at: Option<i64>,
    pub last_commit_subject: Option<String>,
    /// URL do `origin`, para abrir no navegador e favoritar entre dispositivos.
    pub remote: Option<String>,
}
