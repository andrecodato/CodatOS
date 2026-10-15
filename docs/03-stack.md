# 03 · Stack

> **Status:** Rascunho · **Versão:** 0.1 · **Atualizado:** 2026-10-14
> ← [Arquitetura](02-arquitetura.md) · [Índice](README.md) · Próximo: [Segurança](04-seguranca.md) →

As versões exatas são fixadas no início do M0 (última estável). Abaixo, a escolha e o motivo.

## App desktop e núcleo (Rust)

| Camada | Escolha | Por quê |
|---|---|---|
| Linguagem | **Rust** (stable, edition 2024) | Desempenho, segurança de memória, binário único nos 3 SOs |
| Shell | **Tauri 2** | Usa a WebView do SO (instaladores de ~10 MB, pouca RAM); plugins oficiais de autostart, single-instance, updater, notification, global-shortcut, deep-link, window-state |
| Runtime assíncrono | **Tokio** | Padrão do ecossistema |
| API local | **Axum** + tower-http | HTTP + WebSocket, servir a UI estática |
| Banco local | **SQLite** com **SQLCipher**, via **sqlx** | Arquivo único, transacional, cifrado em repouso; migrações versionadas |
| Métricas | **sysinfo**, **nvml-wrapper** (NVIDIA), **battery** | Cobertura multiplataforma |
| Windows | crate **windows** (Registro, APIs Win32), **wmi** | WSL, VHDX, sensores |
| Docker | **bollard** | Cliente da API do Docker (named pipe / socket) |
| SSH/SFTP | **russh** + **russh-sftp**, **ssh-key** | SSH puro Rust, sem depender de OpenSSH instalado |
| Terminal local | **portable-pty** | ConPTY no Windows, PTY no Unix |
| Git | **gix** (gitoxide) | Status de repositórios sem processo externo |
| Arquivos | **notify** | *Watcher* para descoberta de projetos |
| HTTP cliente | **reqwest** (rustls) | Clima, calendários, nuvem |
| Keychain do SO | **keyring** | Credential Manager / Keychain / Secret Service |
| Criptografia | **RustCrypto**: argon2, chacha20poly1305, x25519-dalek, ed25519-dalek, hkdf, sha2; **snow** (Noise); **zeroize**, **secrecy** | Bibliotecas auditadas e amplamente usadas |
| Logs | **tracing** + tracing-appender | Logs estruturados com rotação |
| Erros | thiserror / anyhow | Convenção do ecossistema |

## UI (Rust → WASM)

| Camada | Escolha | Por quê |
|---|---|---|
| Framework | **Leptos** (CSR) | UI em Rust com reatividade fina e bundles pequenos; roda igual na WebView do Tauri e no navegador |
| Build | **Trunk** | Integração padrão Tauri + Leptos |
| Estilo | **Tailwind CSS 4** + tokens do [design system](07-design-system.md) | Velocidade para chegar ao visual do mockup |
| Ícones | **Lucide** (SVG) | Mesmo estilo do mockup |
| Terminal | **xterm.js** (+ addons fit, webgl, search, web-links) via `wasm-bindgen` | Padrão de mercado; não há equivalente maduro em Rust |
| Gráficos | Componentes SVG próprios (anel, área, barras, rosca, sparkline) | Leves e com o visual exato do mockup |
| Grade do dashboard | Implementação própria com Pointer Events | Controle total de arrastar/redimensionar e acessibilidade |
| i18n | **Fluent** (`leptos_i18n` ou `fluent-rs`) | Plurais e gêneros corretos em pt-BR/en |
| Utilidades | **leptos-use** | Hooks de navegador (media queries, visibilidade, atalhos) |

## Nuvem (TypeScript)

| Camada | Escolha | Por quê |
|---|---|---|
| Runtime | **Node.js LTS** | Suporte longo |
| Framework | **NestJS** (adapter Fastify) | Preferência do autor, módulos bem organizados, ótimo para auth/RBAC/billing |
| Banco | **PostgreSQL** (≥ 17) em Docker | Robusto, RLS para isolamento por tenant, JSONB |
| ORM / SQL | **Drizzle ORM** | Próximo do SQL, tipado, migrações simples, convive bem com RLS (`set_config` por transação) |
| Cache / pub-sub | **Valkey** (fork BSD do Redis) | Presença do relay, pub/sub entre instâncias, rate limiting |
| Validação | **zod** (via nestjs-zod) | Esquemas compartilháveis com o front web |
| Auth | **argon2**, **otplib** (TOTP), **@simplewebauthn/server** (passkeys), OAuth GitHub | Padrões consolidados |
| WebSocket | **@nestjs/websockets** com adapter `ws` | Relay e eventos de sync |
| Pagamentos | **Stripe** (cartão + Pix) — decidido na [Q-16](10-perguntas-em-aberto.md#contas-sync-e-planos) | Assinaturas, portal do cliente, webhooks |
| E-mail | Provedor transacional (Resend/Postmark/SES) | Verificação, convites, alertas |
| Observabilidade | **pino**, **OpenTelemetry**, Prometheus | Logs/métricas sem dados sensíveis |
| Docs da API | OpenAPI (Swagger do Nest) | Gera o cliente Rust da nuvem |
| Testes | Vitest + **Testcontainers** (Postgres/Valkey reais) | Testes de integração confiáveis |

> **Sugestão sobre o backend**: NestJS + Postgres é uma ótima escolha e foi mantida. A alternativa avaliada foi um backend em **Rust/Axum** para compartilhar crates com o app. Como o servidor é *zero-knowledge* — não cifra nem decifra nada, só guarda blobs e aplica autorização — o ganho de compartilhar código é pequeno, enquanto o ecossistema do Nest para auth, billing e admin acelera muito. Os contratos ficam alinhados pelo crate `protocol` gerando tipos TS. Detalhes no [ADR-0006](adr/0006-nestjs-postgres.md).

## Infraestrutura e entrega

| Item | Escolha |
|---|---|
| Containers | Docker + Compose (desenvolvimento e self-host) |
| Proxy/TLS | **Caddy** (HTTPS automático) |
| CI/CD | **GitHub Actions**: lint/test/build em matriz Windows/macOS/Linux; **tauri-action** para releases |
| Releases | **release-please** (Conventional Commits → versão + changelog) |
| Dependências | **Renovate**, **cargo-deny**, **cargo-audit**, **osv-scanner** |
| Distribuição | GitHub Releases (MSI/NSIS, DMG universal, AppImage/.deb/.rpm); winget, Homebrew Cask e Flatpak como Deveria |
| Assinatura | Authenticode (Windows), Developer ID + notarização (macOS), chave Ed25519 do updater |
| Hospedagem da nuvem | A definir ([Q-28](10-perguntas-em-aberto.md#infraestrutura-e-custos)) — proposta: VPS (Hetzner) ou Fly.io com região em São Paulo |

## Ambiente de desenvolvimento

Decisão do autor: **as ferramentas de build e teste ficam no WSL Ubuntu**, não no Windows hospedeiro.

| Ferramenta | Onde |
|---|---|
| Rust (rustup, stable), alvo `wasm32-unknown-unknown`, Trunk | WSL Ubuntu |
| Node.js LTS + pnpm (nuvem e site) | WSL Ubuntu |
| Docker Engine + Compose (PostgreSQL, Valkey, API da nuvem) | WSL Ubuntu |
| Dependências do Tauri para Linux (`webkit2gtk-4.1` e afins) | WSL Ubuntu — o app Linux abre na tela do Windows via WSLg |
| `cargo-xwin` (cross-compilação para `x86_64-pc-windows-msvc`) | WSL Ubuntu |

**App Windows:** proposta de compilar no WSL com `cargo-xwin` e executar o `.exe` no Windows pela interoperabilidade do WSL — é assim que se testam os recursos exclusivos do Windows (coletor de WSL, bandeja, autostart, WebView2). O instalador NSIS pode ser gerado no Linux (suporte experimental do Tauri); o MSI exige Windows, então fica a cargo do CI. Ver [Q-35](10-perguntas-em-aberto.md#infraestrutura-e-custos).

**Configurado e verificado em 2026-10-14** (Ubuntu 26.04.1 LTS, WSL2, systemd): Rust 1.99 · Node 24.22 · pnpm 12.11 · Docker 29.9 + Compose 5.6 · WebKitGTK 2.52 · clang 21. Para reproduzir em outra máquina, use os scripts de [`scripts/wsl/`](../scripts/wsl/README.md). Ferramentas específicas do M0 (Trunk, `tauri-cli`, `cargo-xwin`) serão instaladas com versões fixadas quando o workspace for criado.

## Site

**Astro** + Tailwind + TypeScript com ilhas mínimas; mosaico interativo em Canvas 2D; hospedagem estática (Cloudflare Pages ou GitHub Pages). A documentação pública pode usar **Starlight** (Astro) lendo esta pasta `docs/`. Ver [08 · Site](08-site.md).

## Alternativas descartadas (resumo)

| Alternativa | Motivo |
|---|---|
| Electron | Consumo de RAM incompatível com "abrir com o Windows e ficar leve" |
| GUI nativa em Rust (egui, iced, Slint, GPUI) | Difícil atingir o visual do mockup e **impossível servir a mesma UI no navegador** |
| UI em React/Svelte + TypeScript | Ecossistema maior e mais rápido para UI, mas a interface deve ser escrita em Rust (decisão do autor — [ADR-0002](adr/0002-tauri-e-leptos.md) aceito) |
| Dioxus | Também viável (desktop + web); Tauri foi preferido pelos plugins maduros (autostart, updater, bandeja, assinatura) |
| Backend em Rust | Ver nota acima e [ADR-0006](adr/0006-nestjs-postgres.md) |
