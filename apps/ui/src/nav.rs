//! Navegação entre as telas pelo `#` da URL (`#/conexoes`), para o voltar do navegador e
//! os links funcionarem igual na janela do app e no acesso pelo navegador.

use leptos::{ev, prelude::*};

use crate::components::icon::Icon;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Dashboard,
    Finance,
    TaskHub,
    Connections,
    Projects,
    System,
    Settings,
}

impl Page {
    pub const ALL: [Self; 7] = [
        Self::Dashboard,
        Self::Finance,
        Self::TaskHub,
        Self::Connections,
        Self::Projects,
        Self::System,
        Self::Settings,
    ];

    fn slug(self) -> &'static str {
        match self {
            Self::Dashboard => "",
            Self::Finance => "financeiro",
            Self::TaskHub => "taskhub",
            Self::Connections => "conexoes",
            Self::Projects => "projetos",
            Self::System => "sistema",
            Self::Settings => "configuracoes",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Finance => "Financeiro",
            Self::TaskHub => "TaskHub",
            Self::Connections => "Conexões",
            Self::Projects => "Projetos",
            Self::System => "Sistema",
            Self::Settings => "Configurações",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Self::Dashboard => Icon::Dashboard,
            Self::Finance => Icon::Wallet,
            Self::TaskHub => Icon::Kanban,
            Self::Connections => Icon::Server,
            Self::Projects => Icon::FolderGit,
            Self::System => Icon::Cpu,
            Self::Settings => Icon::Settings,
        }
    }

    pub fn href(self) -> String {
        format!("#/{}", self.slug())
    }

    /// `#/conexoes` → [`Page::Connections`]; vazio ou desconhecido → dashboard.
    fn from_hash(hash: &str) -> Self {
        let slug = hash.trim_start_matches('#').trim_start_matches('/');
        let slug = slug.split(['/', '?']).next().unwrap_or_default();
        Self::ALL
            .into_iter()
            .find(|p| p.slug() == slug)
            .unwrap_or(Self::Dashboard)
    }
}

fn current() -> Page {
    Page::from_hash(&window().location().hash().unwrap_or_default())
}

/// Tela atual, seguindo o `#` da URL.
pub fn page() -> ReadSignal<Page> {
    let (page, set_page) = signal(current());
    // Vive enquanto a janela existir, como o App: o handle não é removido.
    let _ = window_event_listener(ev::hashchange, move |_| set_page.set(current()));
    page
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telas_pelo_hash() {
        assert_eq!(Page::from_hash(""), Page::Dashboard);
        assert_eq!(Page::from_hash("#/"), Page::Dashboard);
        assert_eq!(Page::from_hash("#/conexoes"), Page::Connections);
        assert_eq!(Page::from_hash("#/conexoes/h1"), Page::Connections);
        assert_eq!(Page::from_hash("#/nada"), Page::Dashboard);
        for page in Page::ALL {
            assert_eq!(Page::from_hash(&page.href()), page);
        }
    }
}
