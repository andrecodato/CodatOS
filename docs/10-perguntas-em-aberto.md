# 10 · Perguntas em aberto

> **Status:** Aguardando respostas · **Atualizado:** 2026-10-14
> ← [Roadmap](09-roadmap.md) · [Índice](README.md) · Próximo: [Glossário](11-glossario.md) →

Cada pergunta tem uma **proposta padrão** — se não houver resposta, seguimos com ela. Ao decidir, registre a resposta aqui e atualize os documentos indicados (ou crie um ADR).

**Legenda de impacto:** 🔴 bloqueia um marco · 🟡 muda escopo · 🟢 detalhe

## Produto e marca

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-01 | ~~CodatOS ou Nexora?~~ ✅ **CodatOS** (decidido em 2026-10-14). Resta: qual domínio? | Checar disponibilidade de marca e domínio (`codatos.dev`, `.app`, `.com.br`) antes do site | 🟢 [Site](08-site.md) |
| Q-02 | Público: só você, devs em geral ou também equipes pequenas? | Você primeiro (dogfooding), devs em geral na 1.0, equipes com o Team | 🟢 [Visão](00-visao.md) |
| Q-03 | Idiomas no lançamento: pt-BR + en? Docs em pt-BR e código em inglês? | Sim para ambos | 🟢 [CONTRIBUTING](../CONTRIBUTING.md) |

## TaskHub

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-04 | ✅ **Decidido em 2026-10-14**: o TaskHub (pasta `TaskHub/`) é só **referência funcional**; o módulo será **refeito na stack Rust** (núcleo + Leptos), sem ponte com a API antiga. | — | [Estratégia](modulos/taskhub.md#estratégia-reescrita-em-rust) |
| Q-05 | As tarefas devem sincronizar entre dispositivos do CodatOS (cifradas)? O sync com GitHub Projects continua sendo usado no dia a dia? | Sim para ambos: sync E2E entre dispositivos + GitHub Projects opcional por projeto | 🟡 [Sync](modulos/contas-e-sync.md#o-que-sincroniza) |
| Q-33 | ✅ **Decidido em 2026-10-14**: a pasta `TaskHub/` é só referência local e fica **fora do repositório** (`/TaskHub/` no `.gitignore`). | — | — |
| Q-34 | As **Áreas** do TaskHub (Estágio, Faculdade, Pessoal) viram **tenants** do CodatOS (como no mockup: Pessoal, Trabalho, Estudos) ou continuam como **campo** das tarefas dentro de um tenant? | Continuam como campo (permite ver tudo num quadro só); tenants ficam para separações fortes (ex.: empresa) | 🟡 [Multitenancy](modulos/multitenancy.md) |

## Financeiro

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-06 | De onde vêm os dados: lançamento manual, importação OFX/CSV, Open Finance (Pluggy/Belvo — pago) ou um app que você já usa? | Manual + OFX/CSV na 1.0 | 🟡 [Financeiro](modulos/financeiro.md) |
| Q-07 | Os dados financeiros podem sincronizar entre dispositivos (cifrados ponta a ponta) ou ficam **só no PC** (o celular vê ao vivo pelo relay)? | Só no PC por padrão; sync cifrado como opção futura | 🟡 [ADR-0005](adr/0005-local-first.md) |
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
| Q-13 | Temperatura de CPU e consumo em watts no Windows exigem driver de kernel. Aceita um componente **opcional** com privilégio (LibreHardwareMonitor/PawnIO ou helper próprio) ou prefere só o que funciona sem admin? | Sem admin por padrão + opt-in para sensores avançados | 🟡 [Sistema](modulos/sistema.md#temperatura-e-energia-no-windows) |
| Q-14 | Além de ver, quais controles você quer: iniciar/parar WSL e containers (sim), matar processos, suspender/desligar o PC, Wake-on-LAN? | WSL/Docker no M1–M2; energia e processos no M3 | 🟢 |
| Q-15 | Onde ficam seus projetos (pastas raiz, dentro do WSL?) e quais editores você usa? | `C:\dev`, `~/` nas distros WSL; VS Code e Cursor | 🟢 [Projetos](modulos/projetos.md) |

## Contas, sync e planos

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-16 | Preço dos planos? Provedor de pagamento: **Stripe** (cartão + Pix), **Mercado Pago**/**Asaas** (foco Brasil) ou um *merchant of record* como **Paddle/Lemon Squeezy** (cuida de impostos globais)? | Stripe; preço definido perto do M4 | 🟡 [Planos](modulos/contas-e-sync.md#planos-proposta) |
| Q-17 | Quem hospedar o próprio servidor ganha sync e acesso remoto de graça (modelo Bitwarden/Vaultwarden)? | Sim — reforça a confiança no open source | 🟡 |
| Q-18 | Ok o login com GitHub exigir a **senha mestra** para desbloquear o cofre? Passkeys na 1.0? | Sim e Deveria | 🟢 [ADR-0004](adr/0004-criptografia-ponta-a-ponta.md) |
| Q-19 | Aceita a regra "perdeu senha mestra **e** chave de recuperação = perdeu os dados sincronizados"? (É o preço do zero-knowledge.) | Sim, comunicando bem | 🔴 M4 |

## Multitenancy

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-20 | O que é um tenant para você — contextos pessoais (Pessoal/Trabalho/Estudos), empresas/clientes, ou ambos? Sub users são **pessoas com conta própria** (convite por e-mail) ou **perfis gerenciados** criados por você (ex.: família)? | Ambos os usos; sub users com conta própria; perfis gerenciados depois | 🔴 M6 |
| Q-21 | Multitenancy colaborativa só no plano pago (Team, por assento)? Tenants locais grátis? | Sim e sim | 🟡 |
| Q-22 | Permissões por recurso (ex.: só o host X) já na 1.0, ou só por módulo? | Por módulo + escopo por pasta de conexões | 🟢 [Multitenancy](modulos/multitenancy.md) |

## Acesso remoto

| ID | Pergunta | Proposta padrão | Impacto |
|---|---|---|---|
| Q-23 | O que o celular deve poder fazer: ver dashboard, controlar containers/WSL, abrir terminal no PC, desligar/bloquear o PC, Wake-on-LAN? | Ver + controlar WSL/Docker + bloquear/suspender; terminal desligado por padrão | 🟡 [Acesso remoto](modulos/acesso-remoto.md#capacidades-remotas-proposta) |
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
| Q-35 | ✅ **WSL2 + Ubuntu 26.04 configurados em 2026-10-14** (Rust, Node/pnpm, Docker — ver [`scripts/wsl/`](../scripts/wsl/README.md)). Resta validar no M0: compilar o app Windows no WSL com `cargo-xwin` e executar o `.exe` no Windows. | Cross-compilar a partir do WSL; MSI fica a cargo do CI | 🟡 M0 — [Stack](03-stack.md#ambiente-de-desenvolvimento) |
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
