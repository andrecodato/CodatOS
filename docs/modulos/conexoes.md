# Módulo · Conexões

> **Prefixo:** `RF-CON` · **Marco principal:** M2 · **Status:** Em implementação ([plano do M2](https://claude.ai/code/artifact/4c65cf33-a212-49c2-b7ea-08c00b42a1b8))
> ↑ [Requisitos](../01-requisitos.md) · Relacionados: [Segurança](../04-seguranca.md) · [Contas e sync](contas-e-sync.md) · [Multitenancy](multitenancy.md) · [Sistema](sistema.md) · [APIs → canais binários](../06-api-e-protocolos.md#canais-binários-terminal-e-sftp)

## Objetivo

Um gerenciador de conexões no nível do **Termius**: hosts organizados, identidades e chaves reutilizáveis, terminal moderno em abas, SFTP de painel duplo, túneis e jump hosts — com segredos sempre cifrados e sincronização ponta a ponta para assinantes.

## Telas (mockup v0)

- **Lista** com abas **SSH/SFTP · Docker · WSL**, busca e **+ Nova conexão**. Cada item: ícone do SO, nome, `ssh 192.168.1.10 – Ubuntu 22.04`, status **Online** e botão **Conectar**.
- **Detalhe da conexão**: cabeçalho com nome, endereço e status; abas **Geral · Autenticação · Permissões · Avançado**; campos Nome, Host, Porta, Tipo; terminal ao lado.

## Requisitos funcionais

### Hosts e organização

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-CON-001 | Cadastro de host: nome, endereço, porta, protocolo, identidade, grupo, tags, cor/ícone, SO (detectado na 1ª conexão), notas. | Deve | M2 |
| RF-CON-002 | Grupos/pastas aninhados com herança (usuário, identidade, jump host, configurações avançadas). | Deveria | M2 |
| RF-CON-003 | Busca, filtros por tag/grupo, favoritos e ordenação por uso recente. | Deve | M2 |
| RF-CON-004 | Status online por verificação TCP periódica (opcional por host/grupo). | Deveria | M2 |
| RF-CON-005 | Importar de `~/.ssh/config`; exportar para `~/.ssh/config`. | Deve | M2 |
| RF-CON-006 | Importar de PuTTY (Registro), MobaXterm e Termius (export). | Deveria | M2 |

### Autenticação e chaveiro

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-CON-010 | Identidades reutilizáveis: usuário + senha e/ou chave. | Deve | M2 |
| RF-CON-011 | Chaves SSH: gerar (Ed25519, ECDSA, RSA 4096), importar (OpenSSH, PEM, PPK), passphrase, exportar chave pública, copiar para o host (equivalente a `ssh-copy-id`). | Deve | M2 |
| RF-CON-012 | Usar agentes SSH existentes: OpenSSH Agent do Windows, Pageant, `ssh-agent`, agentes do 1Password/Bitwarden; *agent forwarding* opcional por host. | Deveria | M2 |
| RF-CON-013 | Known hosts: verificação de fingerprint na 1ª conexão, alerta bloqueante se mudar (possível MITM), gerenciamento da lista. | Deve | M2 |
| RF-CON-014 | Autenticação *keyboard-interactive* (2FA/OTP no servidor). | Deve | M2 |
| RF-CON-015 | Certificados SSH e chaves FIDO2 (`sk-ssh-ed25519`). | Poderia | Futuro |

### Terminal

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-CON-020 | Terminal em abas e painéis divididos; renomear, reordenar, duplicar sessão. | Deve | M2 |
| RF-CON-021 | Emulação xterm-256color/truecolor, Unicode completo, busca no scrollback, copiar/colar, links clicáveis, scrollback configurável. | Deve | M2 |
| RF-CON-022 | Temas de terminal e fontes (JetBrains Mono padrão), ligaduras opcionais. | Deveria | M2 |
| RF-CON-023 | Snippets: comandos salvos com variáveis, executáveis em uma ou várias sessões. | Deveria | M2 |
| RF-CON-024 | Keepalive e reconexão automática. | Deve | M2 |
| RF-CON-025 | Terminais locais (PowerShell, CMD, bash, zsh), shell das distros WSL e `exec` em containers Docker — abas **WSL** e **Docker** da tela. | Deve | M2 |
| RF-CON-026 | Transmitir digitação para várias sessões (broadcast). | Poderia | Futuro |
| RF-CON-027 | Gravação de sessão: desligada; se ligada, **somente local e cifrada**, nunca sincronizada (R-01). | Poderia | Futuro |
| RF-CON-028 | Mosh, Telnet e Serial. | Poderia | Futuro |

### SFTP

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-CON-030 | Navegador SFTP de painel duplo (local ↔ remoto) com arrastar e soltar, fila de transferências, progresso, pausa e retomada. | Deve | M2 |
| RF-CON-031 | Operações: criar pasta, renomear, excluir, permissões (chmod), propriedades; editar arquivo remoto num editor externo com envio automático ao salvar. | Deveria | M2 |
| RF-CON-032 | Transferências ocorrem **diretamente** entre o dispositivo e o host. No acesso remoto, o relay apenas encaminha bytes cifrados, sem armazenar nada (R-01). | Deve | M2 |

### Túneis e rotas

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-CON-040 | Port forwarding local (`-L`), remoto (`-R`) e dinâmico/SOCKS (`-D`): salvos, com status, opção de iniciar com o app. | Deveria | M2 |
| RF-CON-041 | Jump hosts encadeados (ProxyJump) e proxy SOCKS/HTTP por host. | Deve | M2 |

### Outros protocolos (dependem da [Q-10](../10-perguntas-em-aberto.md#conexões))

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-CON-045 | RDP: gerar `.rdp` e abrir o cliente nativo (mstsc / Microsoft Remote Desktop / Remmina). | Poderia | Futuro |
| RF-CON-046 | VNC, FTP/FTPS, S3, Kubernetes (`kubectl exec`), clientes de banco de dados. | Poderia | Futuro |

### Compartilhamento e permissões

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-CON-050 | Hosts, identidades e chaves pertencem a um cofre pessoal ou de um tenant (compartilhado com grupos). | Deve | M6 |
| RF-CON-051 | Permissões por recurso: ver, conectar, editar, revelar segredo (aba **Permissões** do detalhe). | Deve | M6 |
| RF-CON-052 | Auditoria: quem conectou em qual host e quando (metadado, nunca conteúdo). | Deveria | M6 |

## Aba "Avançado" (configurações por host)

Keepalive, codificação, variáveis de ambiente, comando inicial, tema do terminal, agent forwarding, compressão, algoritmos permitidos (KEX/cifras/MAC), timeout, comportamento de *backspace*, tipo de terminal (`TERM`).

## Regras de negócio

- Segredos (senhas, chaves privadas, passphrases) são decifrados **somente no núcleo Rust**, no momento da conexão. A UI — inclusive no navegador e no celular — nunca recebe a chave privada (RNF-SEG-09).
- "Conectar sem poder ver a senha" (permissão `connections.use` sem `connections.reveal`) é uma **barreira de interface**, não criptográfica: quem pode conectar tem a chave decifrada no próprio dispositivo. Para isolamento forte, o caminho é usar certificados SSH de curta duração (RF-CON-015). Isso deve ser explicado na UI de permissões.
- Conteúdo de terminal e arquivos SFTP nunca são registrados em log (R-06).

## Critérios de aceite

- **Dado** um `~/.ssh/config` com 20 hosts, `ProxyJump` e `IdentityFile`, **quando** importado, **então** os hosts, identidades e jump hosts são criados e conectam sem ajustes.
- **Dado** que a fingerprint de um host mudou, **quando** o usuário tenta conectar, **então** a conexão é bloqueada com alerta explicando o risco e a opção consciente de aceitar a nova chave.
- **Dado** uma transferência SFTP de 2 GB interrompida, **quando** a conexão volta, **então** ela retoma do ponto em que parou.

## Perguntas em aberto

- Quais protocolos além de SSH/SFTP? → [Q-10](../10-perguntas-em-aberto.md#conexões)
- Histórico/gravação de sessão? → [Q-11](../10-perguntas-em-aberto.md#conexões)
- Snippets e túneis no MVP? → [Q-12](../10-perguntas-em-aberto.md#conexões)
- Desbloqueio do cofre sem conta? → [Q-36](../10-perguntas-em-aberto.md#conexões)
- Banco inteiro cifrado no M2? → [Q-37](../10-perguntas-em-aberto.md#conexões)

As três primeiras seguem a proposta padrão no M2.

## Decisões técnicas do M2

- **SSH/SFTP**: `russh` e `russh-sftp` com o backend criptográfico `ring` (compila no build do Windows pelo WSL; o `aws-lc-rs` pede CMake e NASM).
- **Terminal**: xterm.js empacotado na UI (sem CDN), ligado ao Leptos por `wasm-bindgen`; dados nos [canais binários](../06-api-e-protocolos.md#canais-binários-terminal-e-sftp) do mesmo WebSocket `/rpc`.
- **Terminais locais**: `portable-pty` (ConPTY no Windows); WSL por `wsl.exe -d <distro>` e Docker por `docker exec -it`.
- **Importação**: só `~/.ssh/config` no M2; PuTTY, MobaXterm e Termius depois.
- **Status online**: verificação TCP só enquanto a lista de hosts estiver na tela, para o repouso continuar perto do M1.
