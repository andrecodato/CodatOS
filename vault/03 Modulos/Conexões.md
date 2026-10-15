---
tags: [modulo]
status: planejamento
marco: M2
spec: docs/modulos/conexoes.md
---
# Conexões

Gerenciador estilo Termius: hosts e grupos, identidades, chaves (gerar/importar), known hosts, terminal em abas e painéis, SFTP de painel duplo, túneis (-L/-R/-D), jump hosts, snippets. Abas **SSH/SFTP · Docker · WSL** (mockup).

**Conceitos:** [[Cofre]] · [[Zero-knowledge]] · [[Grupos e permissões]]

## Fios soltos
- `russh` + `russh-sftp` (puro Rust) · `portable-pty` para terminais locais · xterm.js na UI.
- Chave privada **nunca** chega à UI (nem no navegador/celular).
- Importar `~/.ssh/config` com ProxyJump/IdentityFile logo no M2.
- Transferências SFTP diretas; no remoto, o [[Relay]] só encaminha cifrado (R-01).

## Pesquisa
- [[Benchmark Termius]]

## Ideias
- Integração com agentes SSH do 1Password/Bitwarden.
- Certificados SSH de curta duração para times.
- Broadcast de digitação para várias sessões.
