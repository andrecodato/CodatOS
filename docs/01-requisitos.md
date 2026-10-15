# 01 · Requisitos (documento mestre)

> **Status:** Rascunho · **Versão:** 0.1 · **Atualizado:** 2026-10-14
> ← [Visão](00-visao.md) · [Índice](README.md) · Próximo: [Arquitetura](02-arquitetura.md) →

Este é o documento central de requisitos do CodatOS. Ele define **escopo**, **restrições invioláveis**, **requisitos não funcionais** e o **índice dos requisitos funcionais**, que são detalhados nos documentos de cada [módulo](README.md#módulos-requisitos-funcionais-detalhados).

---

## 1. Escopo

### Dentro do escopo (1.0)

- App desktop nativo (Windows, macOS, Linux) que inicia com o sistema e vive na bandeja.
- A mesma interface acessível pelo navegador (localhost, LAN e — no plano pago — remotamente pelo celular).
- Dashboard configurável com widgets de: financeiro, tarefas, calendário da semana, clima, uso do PC (CPU/RAM/GPU/temperatura/energia), discos, projetos recentes, WSL e Docker (inclusive dentro das distros WSL).
- Módulos completos: Financeiro, TaskHub, Conexões (SSH/SFTP/túneis, estilo Termius), Projetos, Configurações.
- Contas (login/cadastro), sincronização de configurações criptografada de ponta a ponta para assinantes.
- Multitenancy: tenants, membros (sub users), grupos de permissão configuráveis.
- Site de apresentação com download e link para o código.

### Fora do escopo

- Armazenar arquivos do usuário no servidor (ver [R-01](#4-restrições-invioláveis)).
- Acesso remoto à área de trabalho (vídeo da tela).
- Transações bancárias, investimentos, Open Finance com escrita.
- Monitoramento corporativo/MDM de frotas de máquinas.
- App mobile nativo na 1.0 (o celular usa a PWA; ver [Q-24](10-perguntas-em-aberto.md#acesso-remoto)).

## 2. Plataformas-alvo

| Plataforma | Versões | Observações |
|---|---|---|
| Windows | 10 22H2+ e 11 (x64, arm64) | Requer WebView2 (já presente no 11; instalador embute bootstrapper). Recursos de WSL só aqui. |
| macOS | 13 Ventura+ (Apple Silicon e Intel) | Build universal, assinado e notarizado. |
| Linux | Ubuntu 22.04+, Fedora 39+, Arch; X11 e Wayland | Requer `webkit2gtk-4.1`. Pacotes: AppImage, .deb, .rpm, Flatpak (Deveria). |
| Navegador | Duas últimas versões de Chrome, Edge, Firefox, Safari | Mesma UI do desktop. |
| Celular | iOS 16.4+ (Safari), Android 10+ (Chrome) | PWA instalável. |

## 3. Atores

| Ator | Descrição |
|---|---|
| **Usuário local** | Usa o app no próprio PC, com ou sem conta. |
| **Dono da conta** | Tem conta na nuvem; pode assinar o plano pago, sincronizar e acessar remotamente. |
| **Administrador de tenant** | Gerencia membros, grupos e permissões de um tenant. |
| **Membro (sub user)** | Pessoa convidada para um tenant, com permissões dadas pelos grupos. |
| **Dispositivo** | Instância do app (agente) registrada na conta. |
| **Cliente remoto** | Navegador/PWA que controla um dispositivo via relay. |

## 4. Restrições invioláveis

Estas regras valem acima de qualquer requisito. Um PR que viole uma delas não é aceito.

| ID | Restrição |
|---|---|
| **R-01** | **O servidor nunca armazena arquivos do usuário** — nada de uploads, transferências SFTP, gravações de terminal, backups ou anexos. Apenas configurações e metadados mínimos. O servidor impõe limite de tamanho por item (proposta: 64 KiB) e rejeita qualquer conteúdo acima. |
| **R-02** | **Segredos** (senhas de hosts, chaves privadas SSH, passphrases, tokens OAuth e de API) **nunca são armazenados em texto claro** — nem no disco local, nem na nuvem — e nunca trafegam em claro para fora do agente local. |
| **R-03** | **O servidor não consegue ler** segredos nem configurações sincronizadas: tudo é cifrado no dispositivo com chaves que o servidor nunca recebe (zero-knowledge). Ver [ADR-0004](adr/0004-criptografia-ponta-a-ponta.md). |
| **R-04** | **A senha de login nunca chega ao servidor.** O cliente deriva uma chave (Argon2id) e envia apenas um derivado; o servidor guarda somente o **hash** (Argon2id) desse derivado. |
| **R-05** | **Sem telemetria sem opt-in.** Relatórios de erro só com consentimento e sem dados pessoais. |
| **R-06** | **Logs nunca contêm** segredos, conteúdo de terminal, nomes de arquivos remotos ou dados financeiros — no cliente e no servidor. |
| **R-07** | **O app funciona 100% offline e sem conta**, exceto sync, acesso remoto e colaboração. |
| **R-08** | **Todo o código é open source**; nenhuma garantia de segurança depende de código fechado. |

> [!IMPORTANT]
> **Por que não usar "hash" para tudo?** Hash é uma via de mão única: serve para *conferir* uma senha de login, mas não dá para "des-hashear". Uma chave SSH ou uma senha de servidor guardada só como hash não poderia ser usada para conectar. Por isso o CodatOS usa **hash** (Argon2id) para verificar a senha de login e **criptografia de ponta a ponta** (XChaCha20-Poly1305 com chaves derivadas no seu dispositivo) para tudo que precisa ser recuperado depois. O efeito que você quer — "ninguém além de mim consegue ler" — é garantido pela criptografia ponta a ponta. Detalhes em [Segurança](04-seguranca.md#2-hash-vs-criptografia).

## 5. Requisitos funcionais — índice

Cada módulo detalha seus requisitos com ID, prioridade, marco e critérios de aceite.

| Módulo | Prefixo | Resumo | Marco principal |
|---|---|---|---|
| [App e configurações](modulos/app-e-configuracoes.md) | `RF-APP` | Iniciar com o SO, bandeja, janela, menu lateral, paleta de comandos, atualizações, onboarding | M0–M1 |
| [Dashboard](modulos/dashboard.md) | `RF-DASH` | Grade de widgets configurável, múltiplos dashboards, atualização em tempo real | M1 |
| [Monitor do sistema](modulos/sistema.md) | `RF-SYS` | CPU/RAM/GPU/temperatura/energia, discos, WSL (estado e tamanho), Docker (host e dentro do WSL) | M1 |
| [Projetos](modulos/projetos.md) | `RF-PROJ` | Descoberta de repositórios, recentes do editor, status git, ações rápidas | M1 |
| [Conexões](modulos/conexoes.md) | `RF-CON` | Hosts, identidades, chaves, terminal em abas, SFTP, túneis, jump hosts | M2 |
| [Financeiro](modulos/financeiro.md) | `RF-FIN` | Contas, transações, importação OFX/CSV, categorias, metas, visão geral | M3 |
| [TaskHub](modulos/taskhub.md) | `RF-TASK` | Tarefas, projetos, quadros — incorporação do TaskHub existente | M3 |
| [Integrações](modulos/integracoes.md) | `RF-INT` | Clima do dia e calendário da semana (Google, Outlook, CalDAV, ICS) | M1 / M3 |
| [Contas, sync e planos](modulos/contas-e-sync.md) | `RF-CONTA` | Login/cadastro, cofre E2E, sincronização, dispositivos, assinatura | M4 |
| [Acesso remoto](modulos/acesso-remoto.md) | `RF-REM` | UI no navegador local/LAN; controle pelo celular via relay | M1 / M5 |
| [Multitenancy](modulos/multitenancy.md) | `RF-TEN` | Tenants, membros, grupos e permissões, cofres compartilhados | M3 / M6 |

## 6. Requisitos não funcionais

### Desempenho

| ID | Requisito |
|---|---|
| RNF-DES-01 | Em repouso (janela fechada, na bandeja): CPU média < 1% e RAM < 80 MB (núcleo, sem WebView). Com janela aberta: RAM < 300 MB. |
| RNF-DES-02 | Do login no SO até o dashboard interativo: < 2 s após o app iniciar (SSD). O autostart não pode atrasar o login (início com baixa prioridade/atraso configurável). |
| RNF-DES-03 | Coletores rodam só para widgets/telas visíveis ou alertas ativos; intervalo padrão 2 s em primeiro plano e 30 s em segundo plano. |
| RNF-DES-04 | Terminal: latência adicional < 16 ms localmente e < 150 ms via relay (p95, mesma região). |
| RNF-DES-05 | Bundle da UI (WASM, gzip) < 2 MB; instalador < 20 MB. |
| RNF-DES-06 | Listas longas (transações, hosts, containers) virtualizadas; 10 mil itens sem travar. |

### Segurança (detalhes em [04 · Segurança](04-seguranca.md))

| ID | Requisito |
|---|---|
| RNF-SEG-01 | KDF no cliente: Argon2id (mínimo m=64 MiB, t=3, p=4, parâmetros versionados). Servidor: Argon2id sobre o derivado recebido. |
| RNF-SEG-02 | Criptografia: XChaCha20-Poly1305 (dados), X25519 (troca de chaves), Ed25519 (assinaturas), HKDF-SHA256 (derivação). Apenas bibliotecas auditadas (RustCrypto/libsodium). |
| RNF-SEG-03 | Nuvem só com TLS 1.3 e HSTS; nenhum endpoint em HTTP. |
| RNF-SEG-04 | API local: escuta em `127.0.0.1` por padrão, exige token de sessão, valida `Host` e `Origin` (anti DNS rebinding/CSRF). LAN somente opt-in, com pareamento e TLS. |
| RNF-SEG-05 | Cofre bloqueia automaticamente após inatividade (padrão 15 min) e quando a sessão do SO é bloqueada. |
| RNF-SEG-06 | 2FA (TOTP) e passkeys; rate limiting e bloqueio progressivo contra força bruta. |
| RNF-SEG-07 | CI com `cargo-audit`, `cargo-deny`, `osv-scanner`/`pnpm audit`; SBOM publicado em cada release. |
| RNF-SEG-08 | Binários assinados (Authenticode, Apple notarization); atualizações assinadas (chave Ed25519 do updater). |
| RNF-SEG-09 | Segredos decifrados só no núcleo Rust, em memória protegida (`zeroize`/`secrecy`); a UI nunca recebe chaves privadas. |

### Privacidade e LGPD

| ID | Requisito |
|---|---|
| RNF-PRI-01 | Exportação e exclusão de conta self-service (LGPD art. 18). |
| RNF-PRI-02 | Coleta mínima: e-mail e nome (opcional). Dados de pagamento ficam apenas no provedor de pagamento. |
| RNF-PRI-03 | Política de privacidade e termos em linguagem simples, em pt-BR e en. |
| RNF-PRI-04 | IPs em logs do servidor com retenção máxima de 14 dias. |
| RNF-PRI-05 | Modo privacidade: ocultar valores sensíveis (financeiro, IPs) com um atalho. |

### Confiabilidade

| ID | Requisito |
|---|---|
| RNF-CON-01 | Sync offline-first: operações enfileiradas localmente e reaplicadas; nenhuma perda de dados em conflito. |
| RNF-CON-02 | Falha de um coletor ou integração não derruba o dashboard: erro isolado no widget. |
| RNF-CON-03 | Disponibilidade alvo da nuvem: 99,5%/mês. O app continua 100% funcional se a nuvem cair. |
| RNF-CON-04 | Backups do PostgreSQL cifrados, retenção de 30 dias, restauração testada mensalmente. |
| RNF-CON-05 | Migrações de banco local versionadas, com backup automático antes de migrar. |

### Portabilidade, usabilidade e acessibilidade

| ID | Requisito |
|---|---|
| RNF-POR-01 | Windows, macOS e Linux em x64 e arm64. Recursos específicos (WSL) aparecem só onde existem. |
| RNF-POR-02 | UI idêntica no desktop e no navegador (mesmo bundle). |
| RNF-USA-01 | WCAG 2.2 AA: contraste, foco visível, navegação completa por teclado, rótulos para leitores de tela. |
| RNF-USA-02 | Respeitar `prefers-reduced-motion`. |
| RNF-USA-03 | Tema escuro como padrão; tema claro **Deveria** existir. |
| RNF-USA-04 | Layout responsivo de 360 px (celular) a 4K. |
| RNF-USA-05 | Atalhos de teclado para todas as ações frequentes; paleta de comandos (Ctrl/Cmd+K). |
| RNF-I18N-01 | pt-BR e en-US no lançamento; textos externalizados (Fluent). |
| RNF-I18N-02 | Moeda, datas e números no locale do usuário. |

### Manutenibilidade e observabilidade

| ID | Requisito |
|---|---|
| RNF-MAN-01 | Monorepo; CI com lint, testes e build nos três sistemas a cada PR. |
| RNF-MAN-02 | Cobertura ≥ 70% nos crates de criptografia, sync e protocolo; 100% dos vetores de teste de criptografia. |
| RNF-MAN-03 | Conventional Commits, SemVer e changelog gerado automaticamente. |
| RNF-MAN-04 | Toda decisão arquitetural registrada como ADR. |
| RNF-OBS-01 | Logs locais estruturados com rotação e nível configurável, respeitando R-06. |
| RNF-OBS-02 | Nuvem com métricas (OpenTelemetry/Prometheus), health checks e alertas. |
| RNF-ESC-01 | Nuvem stateless e escalável horizontalmente; relay com presença compartilhada (Valkey). Meta inicial: 5 mil conexões WebSocket por instância de relay. |

## 7. Definição de pronto (para qualquer requisito)

- [ ] Critérios de aceite do módulo atendidos e testados (unitário + integração onde fizer sentido).
- [ ] Funciona nos três SOs (ou está marcado como específico de plataforma).
- [ ] Acessível por teclado e com textos em pt-BR e en.
- [ ] Nenhuma restrição R-01…R-08 violada; revisão de segurança para tudo que toca segredos, sync ou rede.
- [ ] Documentação do módulo atualizada.

## 8. Rastreabilidade — do pedido original aos requisitos

| Pedido original | Onde está coberto |
|---|---|
| Abrir ao iniciar o Windows | [RF-APP-001…003](modulos/app-e-configuracoes.md#requisitos-funcionais) |
| Acessível via navegador | [RF-REM-001…003](modulos/acesso-remoto.md#requisitos-funcionais), [ADR-0003](adr/0003-agente-local-api-unica.md) |
| Resumo do financeiro no dashboard | [RF-FIN-012](modulos/financeiro.md#requisitos-funcionais), [RF-DASH](modulos/dashboard.md) |
| Tarefas | [RF-TASK-005](modulos/taskhub.md#requisitos-funcionais) |
| Calendário da semana | [RF-INT-010…016](modulos/integracoes.md#calendário) |
| Previsão do tempo do dia | [RF-INT-001…004](modulos/integracoes.md#clima) |
| Temperatura e consumo do PC | [RF-SYS-001…004](modulos/sistema.md#hardware) |
| Tamanho dos discos | [RF-SYS-010](modulos/sistema.md#discos) |
| Projetos recentes | [RF-PROJ](modulos/projetos.md) |
| WSL em execução e tamanho das instâncias | [RF-SYS-020…024](modulos/sistema.md#wsl-somente-windows) |
| Dockers rodando (inclusive dentro do WSL) | [RF-SYS-030…032](modulos/sistema.md#docker) |
| Tudo configurável no dashboard | [RF-DASH-002…004](modulos/dashboard.md#requisitos-funcionais) |
| Menu lateral (Dashboard, Financeiro, TaskHub…) | [RF-APP-006](modulos/app-e-configuracoes.md#requisitos-funcionais) |
| Login e cadastro para sincronizar configurações | [RF-CONTA-001…011](modulos/contas-e-sync.md#login-e-cadastro) |
| Sincronizar SSH, SFTP etc. | [RF-CONTA-020…026](modulos/contas-e-sync.md#sincronização) |
| Gerenciador de conexões estilo Termius | [RF-CON](modulos/conexoes.md) |
| Multitenancy com sub users e grupos configuráveis | [RF-TEN](modulos/multitenancy.md) |
| Sync e controle pelo celular só para pagantes | [RF-CONTA-021](modulos/contas-e-sync.md#sincronização), [RF-REM-004…010](modulos/acesso-remoto.md#requisitos-funcionais) |
| Não salvar arquivos | [R-01](#4-restrições-invioláveis) |
| Senhas, chaves e tokens protegidos | [R-02…R-04](#4-restrições-invioláveis), [04 · Segurança](04-seguranca.md) |
| Site moderno com download e GitHub | [08 · Site](08-site.md) |
| Totalmente open source | [R-08](#4-restrições-invioláveis), [Q-26](10-perguntas-em-aberto.md#open-source-e-comunidade) |
| Front em Rust, nativo em Windows/Linux/macOS | [ADR-0002](adr/0002-tauri-e-leptos.md), [03 · Stack](03-stack.md) |
| Backend NestJS + Postgres em Docker | [ADR-0006](adr/0006-nestjs-postgres.md) |

## 9. Histórico

| Versão | Data | Mudança |
|---|---|---|
| 0.1 | 2026-10-14 | Primeira versão a partir do briefing e do mockup v0. |
