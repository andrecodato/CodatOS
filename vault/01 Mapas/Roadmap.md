---
tags: [moc, roadmap]
---
# Roadmap

> 📄 Formal: `docs/09-roadmap.md` — aqui é o acompanhamento do dia a dia.

## M0 · Fundação
- [ ] Monorepo (Cargo + pnpm) e CI em matriz Win/macOS/Linux
- [ ] Spike Leptos: anel de CPU + interop xterm.js + arrastar widget → [[Leptos vs alternativas]]
- [ ] Shell Tauri: bandeja, autostart, instância única, janela destruída ao fechar
- [ ] Núcleo com Axum em 127.0.0.1 + WebSocket RPC
- [ ] Tokens do design system, sidebar e topbar
- [ ] Instalador Windows assinado → [[Assinatura de código]]

## M1 · Dashboard local
- [ ] Coletores CPU/RAM/GPU/discos/rede → [[Coletor]]
- [ ] WSL: estado, tamanho, ações → [[WSL por dentro]]
- [ ] Docker host + WSL, deduplicado → [[Docker em múltiplos engines]]
- [ ] Projetos (raízes + editores)
- [ ] Clima → [[Provedores de clima]]
- [ ] Grade de widgets com modo edição → [[Widget]]
- [ ] Navegador local com token
- [ ] Site S1 ("em breve" + mosaico) → [[Site]]

## M2 · Conexões
- [ ] Hosts, grupos, identidades, chaves, known hosts → [[Conexões]]
- [ ] Terminal em abas/painéis, locais/WSL/Docker
- [ ] SFTP painel duplo, túneis, jump hosts, snippets
- [ ] Cofre local com desbloqueio → [[Cofre]]

## M3 · Produtividade
- [ ] [[Financeiro]] (OFX/CSV, visão geral, metas)
- [ ] [[TaskHub]] incorporado
- [ ] Calendário (ICS + Google) → [[Integrações]]
- [ ] Notificações, alertas, tenants locais, modo LAN

## M4 · Nuvem e sync
- [ ] Auth com KDF no cliente, GitHub, 2FA, recuperação → [[Contas e sync]]
- [ ] Itens cifrados + sync com merge
- [ ] Stripe, export/exclusão de conta, self-host compose

## M5 · Acesso remoto
- [ ] [[Relay]] + Noise + PWA → [[Acesso remoto]]

## M6 · Times
- [ ] Convites, grupos, permissões, cofres compartilhados → [[Multitenancy]]

## 1.0
- [ ] Auditoria de segurança, site S2, pacotes winget/Homebrew/Flatpak
