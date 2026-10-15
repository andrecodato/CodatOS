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

### Decidido

- Nome **CodatOS**; licença **MIT**; UI escrita em Rust (ADR-0002 aceito); TaskHub refeito em Rust usando o projeto atual só como referência; ambiente de testes no WSL Ubuntu.
