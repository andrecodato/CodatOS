# Módulo · Contas, sync e planos

> **Prefixo:** `RF-CONTA` · **Marco principal:** M4 · **Status:** Rascunho
> ↑ [Requisitos](../01-requisitos.md) · Base técnica: [Segurança](../04-seguranca.md) · [Modelo de dados](../05-modelo-de-dados.md) · [APIs → sync](../06-api-e-protocolos.md#protocolo-de-sincronização) · [ADR-0004](../adr/0004-criptografia-ponta-a-ponta.md)

## Objetivo

Permitir que o usuário crie uma conta, assine o plano pago e **sincronize suas configurações** (conexões, chaves, layouts, preferências) entre dispositivos — sem que o servidor jamais consiga ler o conteúdo.

## Telas (mockup v0)

- **Login/cadastro**: imagem de paisagem à esquerda com logo e slogan ("Seu universo, em um só lugar."); cartão à direita com abas **Entrar · Cadastrar**, "E-mail ou usuário", "Senha" (com mostrar/ocultar), "Lembrar de mim", "Esqueceu a senha?", botão **Entrar**, separador "ou", **Entrar com GitHub**, "Não tem uma conta? Cadastre-se".
- **Sincronização**: chave "Ativar sincronização — suas configurações serão criptografadas e sincronizadas"; "O que sincronizar" (Conexões SSH/SFTP, Configurações do sistema, Preferências do painel, Projetos do TaskHub, Tenants e permissões); status ("Sincronizado", última sincronização); conta e "Gerenciar conta"; aviso: "Todos os dados sensíveis (senhas, chaves SSH, tokens etc.) são armazenados de forma segura e criptografada. Nunca são salvos em texto claro."

## Requisitos funcionais

### Login e cadastro

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-CONTA-001 | Tela de login/cadastro conforme mockup, também disponível na web (`app.codatos.dev`). | Deve | M4 |
| RF-CONTA-002 | Cadastro: e-mail, nome, **senha mestra** com medidor de força (zxcvbn), aceite de termos e verificação de e-mail. | Deve | M4 |
| RF-CONTA-003 | Login com derivação de chave no cliente: *prelogin* (parâmetros de KDF) → Argon2id local → envia apenas `authKey`. | Deve | M4 |
| RF-CONTA-004 | Login com GitHub (OAuth). Após autenticar, o cofre ainda pede a **senha mestra** (ou biometria) para desbloquear — o GitHub não tem como abrir seus dados. | Deve | M4 |
| RF-CONTA-005 | 2FA por TOTP com códigos de recuperação. | Deve | M4 |
| RF-CONTA-006 | Passkeys (WebAuthn) para login. | Deveria | M4 |
| RF-CONTA-007 | **Chave de recuperação** gerada no cadastro, exibida uma única vez, com opção de imprimir/salvar. | Deve | M4 |
| RF-CONTA-008 | "Esqueci a senha": com a chave de recuperação, redefine a senha e mantém os dados; sem ela, redefine a conta e **apaga os dados sincronizados** (os dados locais nos dispositivos continuam). A consequência é explicada antes. | Deve | M4 |
| RF-CONTA-009 | Trocar a senha mestra (re-cifra apenas a chave do usuário, não os itens). | Deve | M4 |
| RF-CONTA-010 | Dispositivos: listar, renomear e revogar (encerra sessões e desconecta do relay). | Deve | M4 |
| RF-CONTA-011 | Desbloqueio rápido por biometria/PIN do SO (Windows Hello, Touch ID, Secret Service), guardando a chave no keychain do sistema. | Deveria | M4 |
| RF-CONTA-012 | Exportar todos os dados da conta e excluir a conta (LGPD). | Deve | M4 |
| RF-CONTA-013 | "Lembrar de mim": mantém a sessão (refresh token) mas **não** mantém o cofre desbloqueado além da política de bloqueio. | Deve | M4 |

### Sincronização

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-CONTA-020 | Tela de sincronização conforme mockup, com categorias selecionáveis (ver tabela abaixo). | Deve | M4 |
| RF-CONTA-021 | Somente contas com plano pago ativo podem ativar a sincronização. | Deve | M4 |
| RF-CONTA-022 | Sync incremental e offline-first: alterações enfileiradas localmente, *push* imediato quando online, *pull* por cursor e notificação em tempo quase real via WebSocket. | Deve | M4 |
| RF-CONTA-023 | Conflitos resolvidos por item com número de revisão; mescla por campo quando possível; senão mantém as duas versões e avisa o usuário. | Deve | M4 |
| RF-CONTA-024 | Distinção entre configurações **globais** (sincronizam) e **por dispositivo** (ex.: pastas de projetos, porta local). | Deve | M4 |
| RF-CONTA-025 | Indicador de status (sincronizado, sincronizando, offline, erro) e "sincronizar agora". | Deve | M4 |
| RF-CONTA-026 | Se a assinatura acabar: sync pausa, nada é perdido localmente; dados na nuvem ficam retidos por 90 dias e depois são apagados, com avisos por e-mail. | Deve | M4 |
| RF-CONTA-027 | Histórico de revisões por item (últimas N, cifradas) para desfazer alterações. | Poderia | Futuro |

#### O que sincroniza

| Categoria (tela) | Conteúdo | Sincroniza? |
|---|---|---|
| Conexões SSH/SFTP | Hosts, grupos, identidades, chaves, known hosts, snippets, túneis | ✅ cifrado (segredos e metadados) |
| Configurações do sistema | Preferências globais do app, regras de alerta, integrações (cidade, fontes de calendário + tokens) | ✅ cifrado |
| Preferências do painel | Dashboards, layouts, configurações de widgets | ✅ cifrado |
| Projetos do TaskHub | Projetos e tarefas | ⏳ cifrado, se aprovado na [Q-05](../10-perguntas-em-aberto.md#taskhub) |
| Tenants e permissões | Tenants, membros, grupos, permissões | ✅ nativo do servidor (metadados necessários para autorização) |
| Financeiro | Contas e transações | ❌ por padrão — ver [Q-07](../10-perguntas-em-aberto.md#financeiro) |
| Arquivos, logs, gravações, histórico de métricas | — | ❌ nunca (R-01) |

### Planos (proposta)

| Recurso | Free | Pro | Team |
|---|---|---|---|
| App completo local (dashboard, sistema, conexões, financeiro, TaskHub) | ✅ | ✅ | ✅ |
| Acesso pelo navegador no próprio PC e na rede local | ✅ | ✅ | ✅ |
| Tenants locais (contextos como Pessoal/Trabalho) | ✅ | ✅ | ✅ |
| Sincronização E2E entre dispositivos | — | ✅ | ✅ |
| Controle do PC pelo celular (relay) | — | ✅ | ✅ |
| Membros, grupos e cofres compartilhados | — | — | ✅ (por assento) |
| Servidor self-hosted | ✅ grátis (ver [Q-17](../10-perguntas-em-aberto.md#contas-sync-e-planos)) | — | — |

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-CONTA-030 | Página de planos e checkout via provedor de pagamento (proposta: Stripe, com cartão e Pix), portal de assinatura e webhooks. | Deve | M4 |
| RF-CONTA-031 | O servidor nunca armazena dados de cartão; apenas o ID do cliente/assinatura no provedor. | Deve | M4 |
| RF-CONTA-032 | Período de teste do Pro (proposta: 14 dias). | Poderia | M4 |

## Regras de negócio

- O app inteiro funciona sem conta (R-07). Login só é exigido para sync, acesso remoto e colaboração.
- Perder **senha mestra e chave de recuperação** significa perder os dados sincronizados — consequência inevitável do zero-knowledge, comunicada de forma clara no cadastro.
- Rate limiting: 5 tentativas de login/min por conta e por IP, com bloqueio progressivo; alertas por e-mail em login de novo dispositivo.

## Critérios de aceite

- **Dado** um host criado no PC A com sync ativo, **quando** o PC B fica online, **então** o host aparece no PC B em até 5 s, e o banco do servidor contém apenas texto cifrado.
- **Dado** o mesmo host editado offline nos dois PCs (campos diferentes), **quando** ambos sincronizam, **então** as duas alterações são preservadas.
- **Dado** um login com GitHub em um novo dispositivo, **quando** a autenticação termina, **então** o cofre permanece bloqueado até a senha mestra ser informada.

## Perguntas em aberto

- Preço, planos e provedor de pagamento → [Q-16](../10-perguntas-em-aberto.md#contas-sync-e-planos)
- Self-host gratuito? → [Q-17](../10-perguntas-em-aberto.md#contas-sync-e-planos)
- GitHub + senha mestra separada; passkeys → [Q-18](../10-perguntas-em-aberto.md#contas-sync-e-planos)
- Política de recuperação → [Q-19](../10-perguntas-em-aberto.md#contas-sync-e-planos)
