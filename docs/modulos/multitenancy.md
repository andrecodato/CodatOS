# Módulo · Multitenancy e permissões

> **Prefixo:** `RF-TEN` · **Marco principal:** M3 (tenants locais) / M6 (colaboração) · **Status:** Rascunho
> ↑ [Requisitos](../01-requisitos.md) · Base técnica: [Segurança → cofres compartilhados](../04-seguranca.md#5-fluxos) · [Modelo de dados](../05-modelo-de-dados.md#nuvem-postgresql) · [Conexões](conexoes.md#compartilhamento-e-permissões)

## Objetivo

Separar contextos (ex.: **Pessoal**, **Trabalho**, **Estudos**) e permitir colaboração: cada **tenant** tem seus dashboards, conexões, finanças e tarefas; pode ter **membros** (sub users) e **grupos de permissão** criados, editados e excluídos livremente pelos administradores daquele tenant.

## Conceitos

| Conceito | Definição |
|---|---|
| **Tenant** | Espaço isolado (workspace). Todo usuário tem o tenant **Pessoal**, criado automaticamente e não excluível. |
| **Membro** | Pessoa com conta própria convidada para o tenant (sub user). |
| **Grupo** | Conjunto nomeado de permissões. Grupos do sistema: **Administradores** (tudo), **Visualizadores** (somente leitura). Os demais são livres. |
| **Permissão** | Chave do [catálogo](#catálogo-de-permissões), com escopo opcional (o tenant todo ou recursos específicos: pasta de conexões, tag, cofre). |
| **Cofre do tenant** | Itens cifrados compartilhados entre membros com acesso; a chave do cofre é entregue a cada membro cifrada com a chave pública dele. |

**Sem conta** (Free/local), tenants funcionam como contextos locais isolados. **Com o plano Team**, tenants vivem na nuvem e ganham membros, grupos e cofres compartilhados.

## Telas (mockup v0)

"Tenancy e Permissões", abas **Tenants · Usuários · Grupos · Permissões**:

- Lista de tenants (Pessoal — 3 usuários, 2 grupos — Ativo; Trabalho; Estudos) e **+ Novo tenant**.
- Detalhes do tenant: nome, criado em, **Editar**/**Excluir**; usuários com papel (Você/Admin, Membro); grupos (Administradores, Desenvolvedores — SSH, SFTP, Docker…, Visualizadores — apenas leitura) e **+ Novo grupo**.

## Requisitos funcionais

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-TEN-001 | Criar, renomear, arquivar e excluir tenants. Excluir exige digitar o nome; o tenant Pessoal não pode ser excluído. | Deve | M3 / M6 |
| RF-TEN-002 | Trocar o tenant ativo pelo seletor da barra lateral; dashboards, conexões, financeiro e tarefas passam a ser os do tenant. | Deve | M3 |
| RF-TEN-003 | Convidar por e-mail com grupos iniciais; convite expira em 7 dias; aceitar/recusar; reenviar/cancelar. | Deve | M6 |
| RF-TEN-004 | Listar membros com grupos, status (convidado, ativo, suspenso) e último acesso; suspender e remover. | Deve | M6 |
| RF-TEN-005 | Grupos: criar, editar, duplicar e excluir; adicionar/remover membros. Grupos do sistema podem ter membros alterados, mas não ser excluídos. | Deve | M6 |
| RF-TEN-006 | Atribuir permissões do catálogo a grupos, com escopo opcional por recurso. | Deve | M6 |
| RF-TEN-007 | Permissão efetiva = **união** das permissões dos grupos do membro (sem regras de negação na 1.0). | Deve | M6 |
| RF-TEN-008 | Aba **Permissões**: matriz grupo × permissão e "ver como" (simular a visão de um membro). | Deveria | M6 |
| RF-TEN-009 | Proteções: sempre ≥ 1 administrador; impedir remover o último admin; transferir a propriedade do tenant. | Deve | M6 |
| RF-TEN-010 | Cofres compartilhados com **rotação de chave** ao remover um membro (itens re-cifrados de forma gradual). | Deve | M6 |
| RF-TEN-011 | Log de auditoria do tenant (ações administrativas, convites, acessos a recursos compartilhados), retenção de 90 dias, filtrável. | Deveria | M6 |
| RF-TEN-012 | Membros "gerenciados" (contas criadas pelo admin, ex.: família) — depende da [Q-20](../10-perguntas-em-aberto.md#multitenancy). | Poderia | Futuro |
| RF-TEN-013 | SSO (OIDC/SAML) e provisionamento SCIM. | Não agora | Futuro |

## Catálogo de permissões

| Chave | Permite |
|---|---|
| `tenant.manage` | Editar, arquivar e excluir o tenant; transferir propriedade |
| `members.manage` | Convidar, suspender e remover membros |
| `groups.manage` | Criar/editar/excluir grupos e suas permissões |
| `billing.manage` | Ver e alterar a assinatura do tenant |
| `audit.view` | Ver o log de auditoria |
| `dashboard.view` / `dashboard.edit` | Ver / editar dashboards do tenant |
| `system.view` / `system.control` | Ver métricas, WSL e Docker / executar ações (parar container, encerrar distro, processos, energia) |
| `projects.view` | Ver projetos |
| `finance.view` / `finance.edit` | Ver / editar o financeiro do tenant |
| `tasks.view` / `tasks.edit` / `tasks.assign` | Ver / editar / atribuir tarefas |
| `connections.view` | Ver hosts (sem segredos) |
| `connections.use` | Conectar (o segredo é decifrado no dispositivo do membro) |
| `connections.edit` | Criar e editar hosts, identidades e chaves |
| `connections.reveal` | Ver/copiar senhas e exportar chaves |
| `sftp.read` / `sftp.write` | Navegar e baixar / enviar, renomear e excluir |
| `remote.access` | Acessar remotamente os dispositivos ligados ao tenant |
| `sync.manage` | Configurar o que o tenant sincroniza |

### Grupos padrão

| Grupo | Permissões |
|---|---|
| Administradores *(sistema)* | Todas |
| Desenvolvedores *(modelo)* | `dashboard.view`, `system.view`, `projects.view`, `connections.view`, `connections.use`, `sftp.read`, `sftp.write`, `tasks.view`, `tasks.edit` |
| Visualizadores *(sistema)* | Todas as `*.view` |

## Onde as permissões são aplicadas

| Camada | O que garante |
|---|---|
| **Servidor** | Quem é membro, quem pode alterar membros/grupos, quem recebe a chave de cada cofre, quem pode gravar itens. Postgres com RLS por tenant. |
| **Criptografia** | Só quem tem a chave do cofre consegue ler os itens — mesmo com o banco vazado. |
| **Agente (PC)** | Ações locais e remotas sobre o dispositivo (`system.control`, `remote.access`). |
| **UI** | Esconde o que não é permitido (conveniência, nunca a única barreira). |

## Critérios de aceite

- **Dado** um membro só no grupo Visualizadores, **quando** ele tenta parar um container pelo acesso remoto, **então** o agente recusa a ação e registra a tentativa na auditoria.
- **Dado** que um membro foi removido do tenant, **quando** a remoção é confirmada, **então** ele perde acesso imediato e a chave do cofre é rotacionada.
- **Dado** um tenant com um único administrador, **quando** ele tenta sair do grupo Administradores, **então** a ação é bloqueada com explicação.

## Perguntas em aberto

- O que é um tenant para você e quem são os sub users? → [Q-20](../10-perguntas-em-aberto.md#multitenancy)
- Multitenancy só no plano pago? Cobrança por assento? → [Q-21](../10-perguntas-em-aberto.md#multitenancy)
- Permissões por recurso já na 1.0? → [Q-22](../10-perguntas-em-aberto.md#multitenancy)
