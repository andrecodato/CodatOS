# Changelog

Todas as mudanças relevantes do projeto são registradas aqui.

O formato segue o [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/) e o projeto adota [Versionamento Semântico](https://semver.org/lang/pt-BR/). A partir do M0, este arquivo será gerado automaticamente a partir dos commits (Conventional Commits).

## [Não lançado]

### Adicionado

- Documentação de planejamento v0.1: visão, requisitos (mestre + 11 módulos), arquitetura, stack, segurança, modelo de dados, APIs e protocolos, design system, site, roadmap, perguntas em aberto e glossário.
- ADRs 0001–0008.
- Vault do Obsidian com mapas, conceitos, notas de módulos, pesquisas, diário e templates.
- Arquivos de comunidade: README, CONTRIBUTING, CODE_OF_CONDUCT, SECURITY e modelos de issue/PR.
- Licença MIT.
- Protótipo standalone do hero do site com o mosaico interativo (`apps/site/prototypes/mosaico.html`), com painel de ajustes.
- Scripts do ambiente de desenvolvimento no WSL Ubuntu (`scripts/wsl/`): Rust, Node/pnpm, Docker, dependências do Tauri e verificação.
- M0 · Workspace Cargo + pnpm com os crates `codatos-*` e `just` como atalho dos comandos.
- M0 · CI no GitHub Actions: formatação, clippy, testes e build no Linux, Windows e macOS, mais auditoria de dependências (`cargo deny`, OSV).
- M0 · Núcleo local: API em `127.0.0.1:47800`, WebSocket `/rpc` com JSON-RPC, checagem de Host/Origin e token de sessão (ADR-0003).
- M0 · Casca da UI em Leptos + Tailwind: sidebar, topbar com estado do núcleo e tokens do design system, fontes embutidas.
- M0 · Janela e bandeja (Tauri 2): fechar a janela libera a WebView e o app fica na bandeja; instância única; inicia com o sistema.
- M0 · Releases com release-please e instaladores gerados pelo tauri-action.
- M0 · Script `measure-idle` e primeira medição: 30,4 MB de RAM parado no Windows ([benchmark](docs/benchmarks/m0.md)).

### Decidido

- Nome **CodatOS**; licença **MIT**; UI escrita em Rust (ADR-0002 aceito); TaskHub refeito em Rust usando o projeto atual só como referência; ambiente de testes no WSL Ubuntu.
