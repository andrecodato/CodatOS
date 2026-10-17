# Contribuindo com o CodatOS

Que bom que você quer ajudar! Este guia explica como o projeto é organizado e como propor mudanças.

## Fase atual: M0 concluído, M1 a caminho

A fundação já existe: núcleo local (Axum), janela e bandeja (Tauri 2) e UI em Leptos. As contribuições mais valiosas agora são:

- **Revisar a documentação** em [`docs/`](docs/README.md) e apontar lacunas, riscos ou exageros.
- **Opinar nas [perguntas em aberto](docs/10-perguntas-em-aberto.md)**.
- **Sugerir ideias** por issue (modelo "Ideia / recurso").
- **Testar o app** no seu sistema e abrir issue com o que quebrar.

O ambiente de desenvolvimento roda no **WSL Ubuntu** (Rust, Node/pnpm e Docker ficam lá, não no Windows). Para montar o seu, use os scripts de [`scripts/wsl/`](scripts/wsl/README.md); detalhes em [Stack → Ambiente de desenvolvimento](docs/03-stack.md#ambiente-de-desenvolvimento).

Comandos do dia a dia (precisam do [`just`](https://github.com/casey/just)):

| Comando | O que faz |
|---|---|
| `just dev` | Abre o app em modo dev, com recarga da UI |
| `just check` | O que a CI roda: formatação, clippy sem warnings e testes |
| `just build-win` | Gera o `.exe` do Windows a partir do WSL |
| `just build-linux` | Gera o build do Linux |
| `just measure-idle` | No Windows (PowerShell), mede RAM e CPU do app parado ([resultados](docs/benchmarks/m0.md)) |

## Onde as coisas ficam

| Pasta | Conteúdo |
|---|---|
| `docs/` | Fonte da verdade: requisitos, arquitetura, ADRs |
| `vault/` | Vault do Obsidian com notas de trabalho (conceitos, pesquisa, diário) |
| `apps/`, `crates/` | Código — ver [estrutura do monorepo](docs/02-arquitetura.md#estrutura-do-monorepo-planejada) |

## Idioma

- **Documentação**: português (pt-BR). Traduções para inglês são bem-vindas.
- **Código, nomes, comentários, commits e PRs**: inglês.
- **Issues**: pt-BR ou inglês.

## Fluxo

1. Para algo além de uma correção pequena, **abra uma issue** antes para alinhar.
2. Crie um branch a partir de `main`: `feat/…`, `fix/…`, `docs/…`, `chore/…`.
3. Faça commits seguindo **[Conventional Commits](https://www.conventionalcommits.org/)**:
   - `feat(connections): add jump host chaining`
   - `fix(collectors): avoid starting stopped WSL distros`
   - `docs(adr): add ADR-0009 for plugin sandbox`
4. Abra o PR usando o modelo e preencha o checklist.

## Requisitos e decisões

- Todo requisito tem um ID (`RF-…`, `RNF-…`). PRs de funcionalidade devem citar o ID.
- Requisito novo? Adicione-o na tabela do módulo em `docs/modulos/`.
- Decisão de arquitetura (nova dependência relevante, mudança de protocolo, de banco, de criptografia)? Escreva um **ADR** em `docs/adr/` usando o [modelo](docs/adr/README.md#modelo).

## Regras invioláveis

Nenhum PR pode violar as restrições [R-01 a R-08](docs/01-requisitos.md#4-restrições-invioláveis). Em especial:

- o servidor **nunca** armazena arquivos de usuários;
- segredos **nunca** em texto claro — nem em disco, nem em logs, nem em testes com dados reais;
- nada de telemetria sem opt-in.

## Áreas sensíveis

Mudanças em `crates/crypto`, `crates/vault`, `crates/sync`, no relay ou na autenticação da nuvem exigem **revisão de segurança** de um mantenedor e vetores de teste atualizados. Não implemente primitivas criptográficas próprias.

## Padrões de código (a partir do M0)

| Linguagem | Ferramentas |
|---|---|
| Rust | `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, `cargo deny check` |
| TypeScript | Prettier, ESLint, Vitest |
| Geral | EditorConfig (ver `.editorconfig`), finais de linha LF |

## Obsidian

- Abra a pasta `vault/` como vault.
- Não versione o estado pessoal do Obsidian (`workspace.json` já está no `.gitignore`).
- Quando uma nota amadurecer em decisão ou requisito, promova-a para `docs/` e deixe o link na nota.

## Licença

O CodatOS é distribuído sob a [licença MIT](LICENSE). Ao contribuir, você concorda que sua contribuição seja licenciada nos mesmos termos.

## Código de conduta

Ao participar, você concorda com o [Código de Conduta](CODE_OF_CONDUCT.md).
