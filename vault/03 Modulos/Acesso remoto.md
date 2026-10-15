---
tags: [modulo]
status: planejamento
marco: M5
spec: docs/modulos/acesso-remoto.md
---
# Acesso remoto

| Modo | Onde | Plano |
|---|---|---|
| Local | `http://127.0.0.1:47800` | Free |
| LAN | outro dispositivo da casa, pareado por QR | Free |
| Remoto | PWA em `app.codatos.dev` via [[Relay]] | Pro |

**Conceitos:** [[Relay]] · [[Agente local]] · [[Hierarquia de chaves]]

## Fios soltos
- Ações sensíveis → confirmação + notificação no PC ("iPhone do André parou o container api").
- Indicador na bandeja quando há sessão remota.
- Risco da PWA (código servido pelo servidor) → SRI + hashes publicados + recomendar desktop.

## Ideias
- Wake-on-LAN via outro dispositivo da rede.
- WebRTC P2P para reduzir latência e custo.
- Guia Tailscale para self-hosters.
