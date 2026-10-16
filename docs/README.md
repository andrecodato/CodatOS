# Documentação do CodatOS

> Esta pasta é a **fonte da verdade** do projeto: requisitos, arquitetura e decisões.
> Ideias soltas, rascunhos, pesquisas e diário de desenvolvimento ficam no vault do Obsidian em [`/vault`](../vault/Home.md). Quando uma ideia amadurece no vault, ela é promovida para cá.

## Ordem de leitura

| # | Documento | Para quê |
|---|---|---|
| 00 | [Visão do produto](00-visao.md) | O problema, para quem é, princípios e não objetivos |
| 01 | [**Requisitos**](01-requisitos.md) | **Documento mestre**: escopo, restrições invioláveis, requisitos não funcionais, índice dos requisitos funcionais e rastreabilidade |
| 02 | [Arquitetura](02-arquitetura.md) | Componentes, processos, fluxos e diagramas |
| 03 | [Stack](03-stack.md) | Tecnologias escolhidas e por quê |
| 04 | [Segurança e privacidade](04-seguranca.md) | Modelo zero-knowledge, hierarquia de chaves, ameaças |
| 05 | [Modelo de dados](05-modelo-de-dados.md) | Banco local (SQLite) e nuvem (PostgreSQL) |
| 06 | [APIs e protocolos](06-api-e-protocolos.md) | API do agente local, API da nuvem, sync e relay |
| 07 | [Design system](07-design-system.md) | Tokens, componentes e inventário de telas (mockup v0) |
| 08 | [Site de apresentação](08-site.md) | Landing page, downloads e o mosaico interativo |
| 09 | [Roadmap](09-roadmap.md) | Marcos M0 → M6, riscos |
| 10 | [Perguntas em aberto](10-perguntas-em-aberto.md) | Decisões pendentes + ideias para enriquecer o produto |
| 11 | [Glossário](11-glossario.md) | Vocabulário do projeto |

## Módulos (requisitos funcionais detalhados)

| Módulo | Prefixo | Documento |
|---|---|---|
| App desktop e configurações | `RF-APP` | [modulos/app-e-configuracoes.md](modulos/app-e-configuracoes.md) |
| Dashboard | `RF-DASH` | [modulos/dashboard.md](modulos/dashboard.md) |
| Monitor do sistema (PC, discos, WSL, Docker) | `RF-SYS` | [modulos/sistema.md](modulos/sistema.md) |
| Projetos | `RF-PROJ` | [modulos/projetos.md](modulos/projetos.md) |
| Financeiro | `RF-FIN` | [modulos/financeiro.md](modulos/financeiro.md) |
| TaskHub | `RF-TASK` | [modulos/taskhub.md](modulos/taskhub.md) |
| Integrações (clima e calendário) | `RF-INT` | [modulos/integracoes.md](modulos/integracoes.md) |
| Conexões (SSH, SFTP, túneis…) | `RF-CON` | [modulos/conexoes.md](modulos/conexoes.md) |
| Contas, sync e planos | `RF-CONTA` | [modulos/contas-e-sync.md](modulos/contas-e-sync.md) |
| Acesso remoto (navegador e celular) | `RF-REM` | [modulos/acesso-remoto.md](modulos/acesso-remoto.md) |
| Multitenancy e permissões | `RF-TEN` | [modulos/multitenancy.md](modulos/multitenancy.md) |

## Decisões de arquitetura

Registradas como ADRs (Architecture Decision Records) em [adr/](adr/README.md).

## Medições

Benchmarks de cada marco, com método e números, em [benchmarks/](benchmarks/):

| Marco | Documento |
|---|---|
| M0 | [RAM e CPU em repouso](benchmarks/m0.md) |

## Convenções

- **IDs**: `RF-<MÓDULO>-NNN` (funcional), `RNF-<ÁREA>-NN` (não funcional), `R-NN` (restrição inviolável), `Q-NN` (pergunta em aberto), `ADR-NNNN` (decisão).
- **Prioridade (MoSCoW)**: **Deve** · **Deveria** · **Poderia** · **Não agora**.
- **Marco**: `M0`…`M6` conforme o [roadmap](09-roadmap.md); `Futuro` = pós-1.0.
- **Status de documento**: Rascunho → Em revisão → Aprovado.
- Toda mudança relevante de arquitetura vira um ADR; todo requisito novo ganha um ID e entra na tabela do módulo.
- Idioma: documentação em **pt-BR**; código, commits e nomes técnicos em **inglês** (ver [CONTRIBUTING](../CONTRIBUTING.md)).
- Nome do produto: **CodatOS** (definido). Ainda provisórios: domínio **codatos.com** (não comprado) e porta local **47800** (ver [Q-01](10-perguntas-em-aberto.md#produto-e-marca)).
- Licença: **MIT** ([LICENSE](../LICENSE)).
