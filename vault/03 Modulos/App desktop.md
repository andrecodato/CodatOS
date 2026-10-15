---
tags: [modulo]
status: planejamento
marco: M0
spec: docs/modulos/app-e-configuracoes.md
---
# App desktop

O casco: inicia com o SO, vive na bandeja, janela sem moldura, menu lateral, paleta de comandos, atualizações, configurações.

**Conceitos:** [[Agente local]] · [[Tenant]] (seletor na sidebar)

## Fios soltos
- Autostart **por usuário** (sem admin); atraso configurável para não pesar no login.
- Fechar a janela = destruir a WebView (meta < 80 MB em repouso).
- Barra de título: semáforo no macOS, controles à direita no Windows/Linux.
- "Sistema" no menu lateral é opcional — mockup não tem, mas os widgets precisam de uma página "Ver todos".

## Ideias
- Atalho global estilo Raycast (Ctrl+Alt+Espaço).
- Modo painel para monitor secundário.

## Pesquisa
- [[Assinatura de código]]
