# 10 · Perguntas em aberto

> **Status:** Bloqueantes respondidas (2026-10-15); demais aguardando · **Atualizado:** 2026-10-15
> ← [Roadmap](09-roadmap.md) · [Índice](README.md) · Próximo: [Glossário](11-glossario.md) →

Cada pergunta tem uma **proposta padrão** — se não houver resposta, seguimos com ela. Ao decidir, registre a resposta aqui e atualize os documentos indicados (ou crie um ADR).

**Legenda de impacto:** 🔴 bloqueia um marco · 🟡 muda escopo · 🟢 detalhe

## Produto e marca

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-01 | ~~CodatOS ou Nexora?~~ ✅ **CodatOS** (decidido em 2026-10-14). Domínio: **`codatos.com`** escolhido como **provisório** em 2026-10-15 (ainda não comprado), com `codatos.com.br` redirecionando. | Confirmar disponibilidade e fazer busca de marca no INPI antes de comprar | 🟢 [Site](08-site.md) |
| Q-02 | Público: só você, devs em geral ou também equipes pequenas? | Você primeiro (dogfooding), devs em geral na 1.0, equipes com o Team | 🟢 [Visão](00-visao.md) |
| Q-03 | Idiomas no lançamento: pt-BR + en? Docs em pt-BR e código em inglês? | Sim para ambos | 🟢 [CONTRIBUTING](../CONTRIBUTING.md) |

## TaskHub

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-04 | ✅ **Decidido em 2026-10-14**: o TaskHub (pasta `TaskHub/`) é só **referência funcional**; o módulo será **refeito na stack Rust** (núcleo + Leptos), sem ponte com a API antiga. | — | [Estratégia](modulos/taskhub.md#estratégia-reescrita-em-rust) |
| Q-05 | As tarefas devem sincronizar entre dispositivos do CodatOS (cifradas)? O sync com GitHub Projects continua sendo usado no dia a dia? | Sim para ambos: sync E2E entre dispositivos + GitHub Projects opcional por projeto | 🟡 [Sync](modulos/contas-e-sync.md#o-que-sincroniza) |
| Q-33 | ✅ **Decidido em 2026-10-14**: a pasta `TaskHub/` é só referência local e fica **fora do repositório** (`/TaskHub/` no `.gitignore`). | — | — |
| Q-34 | ✅ **Decidido em 2026-10-15**: a Área continua **campo** (etiqueta) da tarefa dentro do tenant; Estágio, Faculdade e Pessoal aparecem no mesmo quadro. Tenants ficam para separações fortes (ex.: um "Trabalho" com hosts e colegas próprios). | — | [Multitenancy](modulos/multitenancy.md) |

## Financeiro

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-06 | De onde vêm os dados: lançamento manual, importação OFX/CSV, Open Finance (Pluggy/Belvo — pago) ou um app que você já usa? | Manual + OFX/CSV na 1.0 | 🟡 [Financeiro](modulos/financeiro.md) |
| Q-07 | ✅ **Decidido em 2026-10-15**: financeiro fica **só no PC** por padrão (o celular lê ao vivo pelo relay), mas é gravado desde o M3 como **item de cofre**, para o sync E2E poder ser ligado depois sem migração. O usuário pode **exportar** os dados para backup: arquivo cifrado com senha (restaura no CodatOS) e CSV/OFX em texto aberto (com aviso). | — | [Financeiro](modulos/financeiro.md) · [ADR-0005](adr/0005-local-first.md) |
| Q-08 | Precisa de cartão de crédito com faturas, investimentos, multimoeda? | Faturas sim; investimentos só saldo; multimoeda depois | 🟢 |

## Calendário e tarefas

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-09 | Quais calendários você usa (Google, Outlook/365, iCloud, outro)? | ICS + Google primeiro | 🟢 [Integrações](modulos/integracoes.md#calendário) |

## Conexões

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-10 | O "etc." de SSH/SFTP inclui quais protocolos: RDP, VNC, Telnet, Serial, FTP/FTPS, Mosh, Kubernetes, bancos de dados? | SSH, SFTP, túneis, terminais locais/WSL/Docker na 1.0; RDP via cliente nativo depois | 🟡 [Conexões](modulos/conexoes.md) |
| Q-11 | Quer histórico/gravação de sessões de terminal? (Somente local e cifrado, por causa da R-01.) | Não na 1.0 | 🟢 |
| Q-12 | Snippets e port forwarding já no M2? | Sim (você parece usar homelab/VPS) | 🟢 |

## Sistema

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-13 | ✅ **Decidido em 2026-10-15**: sem admin por padrão; temperatura e watts da CPU vêm do **LibreHardwareMonitor** (servidor web local, `http://127.0.0.1:8085/data.json`) quando ele estiver rodando. Sem driver nem serviço privilegiado próprio na 1.0 (reavaliar depois); HWiNFO pode virar segunda fonte. | — | [Sistema](modulos/sistema.md#temperatura-e-energia-no-windows) |
| Q-14 | Além de ver, quais controles você quer: iniciar/parar WSL e containers (sim), matar processos, suspender/desligar o PC, Wake-on-LAN? | WSL/Docker no M1–M2; energia e processos no M3 | 🟢 |
| Q-15 | Onde ficam seus projetos (pastas raiz, dentro do WSL?) e quais editores você usa? | `C:\dev`, `~/` nas distros WSL; VS Code e Cursor | 🟢 [Projetos](modulos/projetos.md) |

## Contas, sync e planos

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-16 | ✅ **Decidido em 2026-10-15**: provedor **Stripe** (cartão + Pix). Preço definido perto do M4. Atenção: com Stripe o CodatOS é o vendedor, então impostos de vendas internacionais são responsabilidade nossa (avaliar Stripe Tax). | — | [Planos](modulos/contas-e-sync.md#planos-proposta) |
| Q-17 | Quem hospedar o próprio servidor ganha sync e acesso remoto de graça (modelo Bitwarden/Vaultwarden)? | Sim — reforça a confiança no open source | 🟡 |
| Q-18 | Ok o login com GitHub exigir a **senha mestra** para desbloquear o cofre? Passkeys na 1.0? | Sim e Deveria | 🟢 [ADR-0004](adr/0004-criptografia-ponta-a-ponta.md) |
| Q-19 | ✅ **Decidido em 2026-10-15**: aceita a regra do zero-knowledge, **sem cópia da chave no servidor**. M4: chave de recuperação (o cadastro exige confirmá-la) + **recuperação por dispositivo confiável** (um aparelho desbloqueado define nova senha mestra). Desbloqueio por passkey junto com a Q-18; contato de emergência depois da 1.0; recuperação pelo admin só no Team, desligada por padrão. | — | [Segurança](04-seguranca.md) · [ADR-0004](adr/0004-criptografia-ponta-a-ponta.md) |

## Multitenancy

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-20 | ✅ **Decidido em 2026-10-15**: tenant serve para contextos (Pessoal/Trabalho) **e** para empresas/clientes; sub user é **pessoa com conta própria**, convidada por e-mail. Perfis gerenciados (família) ficam para depois da 1.0. `tenant_id` existe no modelo desde o M3. | — | [Multitenancy](modulos/multitenancy.md) |
| Q-21 | Multitenancy colaborativa só no plano pago (Team, por assento)? Tenants locais grátis? | Sim e sim | 🟡 |
| Q-22 | Permissões por recurso (ex.: só o host X) já na 1.0, ou só por módulo? | Por módulo + escopo por pasta de conexões | 🟢 [Multitenancy](modulos/multitenancy.md) |

## Acesso remoto

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-23 | ✅ **Decidido em 2026-10-15**: **três níveis por aparelho pareado**, alteráveis no PC: **Ver** (só leitura), **Controlar** (padrão: + criar tarefa/lançamento, WSL/containers, bloquear/suspender/reiniciar/desligar com biometria) e **Total** (+ terminal e SSH através do PC, com biometria e tempo limite). SFTP e Wake-on-LAN depois da 1.0. | — | [Acesso remoto](modulos/acesso-remoto.md#capacidades-remotas-proposta) |
| Q-24 | PWA basta no celular ou quer app nativo (Tauri mobile) no futuro? | PWA na 1.0 | 🟢 |
| Q-25 | Acesso pelo navegador **em outro PC da rede local** é gratuito? | Sim (só o remoto pela internet é Pro) | 🟢 |

## Open source e comunidade

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-26 | ✅ **MIT para todo o projeto** (decidido em 2026-10-14) — ver [LICENSE](../LICENSE). Consequência aceita: qualquer um pode reutilizar o código, inclusive em serviços fechados. | — | — |
| Q-27 | ✅ Repositório: [github.com/andrecodato/CodatOS](https://github.com/andrecodato/CodatOS) (decidido em 2026-10-14). | — | — |

## Infraestrutura e custos

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-28 | Onde hospedar a nuvem? Região? | VPS Hetzner (custo) ou Fly.io com região GRU (São Paulo) | 🟡 |
| Q-29 | Orçamento para assinatura de código? (Apple Developer: US$ 99/ano; Windows: Azure Trusted Signing ou certificado OV/EV.) Sem isso, Windows e macOS mostram alertas ao instalar. | Windows primeiro; Apple antes de divulgar para macOS | 🟡 [Roadmap → riscos](09-roadmap.md#riscos) |
| Q-35 | ✅ **WSL2 + Ubuntu 26.04 configurados em 2026-10-14** (ver [`scripts/wsl/`](../scripts/wsl/README.md)). ✅ **Decidido em 2026-10-15**: testar no início do M0 a compilação do app Windows no WSL com `cargo-xwin`; se falhar, instalar Rust/MSVC também no Windows para o dia a dia. O MSI assinado sai do CI em qualquer caso. | — | M0 — [Stack](03-stack.md#ambiente-de-desenvolvimento) |
| Q-30 | Clima: Open-Meteo é gratuito só para uso não comercial. Usar MET Norway (permite uso comercial com atribuição) ou contratar o Open-Meteo quando houver plano pago? | Open-Meteo no início, revisar antes do M4 | 🟢 [Integrações](modulos/integracoes.md#clima) |

## Stack

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-31 | ✅ **UI escrita em Rust** (Leptos/WASM), decidido em 2026-10-14 — [ADR-0002](adr/0002-tauri-e-leptos.md) aceito. | — | — |
| Q-32 | Confirmar NestJS + PostgreSQL + Valkey + Drizzle (em vez de Prisma)? | Sim | 🟡 [ADR-0006](adr/0006-nestjs-postgres.md) |

## Ideias para enriquecer o produto

Marque o que deve entrar (e em qual marco). Cada ideia aprovada vira requisito no módulo correspondente.

- [ ] **Rotinas / "modo trabalho"**: um clique (ou ao iniciar o PC) sobe distros e containers, conecta túneis, abre o editor no projeto do dia.
- [ ] **Alertas inteligentes**: disco > 90%, container caiu, temperatura alta, host SSH offline, conta a vencer, evento em 10 min.
- [ ] **Health check de servidores**: ping/HTTP/TCP dos seus VPS com histórico de uptime no dashboard.
- [ ] **Widget GitHub**: PRs para revisar, CI quebrado, issues atribuídas.
- [ ] **Gerenciador de `docker compose`**: subir/derrubar stacks por projeto.
- [ ] **Modo painel** para monitor secundário/tablet velho como "display" do homelab.
- [ ] **Atalho global** (Ctrl+Alt+Espaço) abrindo uma paleta estilo Raycast: conectar, abrir projeto, nova tarefa, nova despesa.
- [ ] **Importadores**: Termius, MobaXterm, PuTTY, Organizze/Mobills (financeiro), Todoist.
- [ ] **Widgets da comunidade** (SDK com plugins WASM em sandbox) e galeria de layouts.
- [ ] **Integração Tailscale**: listar dispositivos da tailnet como hosts e usar como caminho remoto para self-hosters.
- [ ] **Wake-on-LAN** a partir do celular via outro dispositivo da casa.
- [ ] **Notas rápidas / scratchpad** local no dashboard.
- [ ] **Pomodoro/foco** integrado às tarefas e ao modo "não perturbe" das notificações.
- [ ] **Relatório semanal** local: horas por projeto (via git), gastos da semana, tarefas concluídas.
