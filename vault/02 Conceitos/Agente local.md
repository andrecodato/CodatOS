---
tags: [conceito, arquitetura]
aliases: [Núcleo, codat-core, API local]
---
# Agente local (núcleo `codat-core`)

O processo Rust que faz **todo** o trabalho no PC e expõe **uma única API** (HTTP + WebSocket em `127.0.0.1:47800`).

- Janela do Tauri, navegador e celular usam **a mesma UI** e **a mesma API** → uma superfície só para testar e proteger.
- Contém: [[Coletor|coletores]], conexões SSH/SFTP, [[Cofre]], sync, cliente do [[Relay]], notificações.
- Segredos nunca saem dele — a UI só recebe dados de exibição.
- Fechar a janela destrói a WebView; o núcleo segue na bandeja (pouca RAM).

**Riscos:** API em localhost é alvo de sites maliciosos → token de sessão, validação de `Host`/`Origin`, `SameSite=Strict`.

📄 `docs/adr/0003-agente-local-api-unica.md` · `docs/06-api-e-protocolos.md`
Módulos: [[App desktop]] · [[Acesso remoto]]
