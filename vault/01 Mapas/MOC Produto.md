---
tags: [moc]
---
# MOC Produto

> 📄 Formal: `docs/00-visao.md` · `docs/01-requisitos.md`

## Em uma frase
**O painel de controle pessoal do seu computador**: abre com o sistema e reúne finanças, tarefas, agenda, clima, saúde do PC (temperatura, discos, WSL, Docker), projetos e conexões SSH/SFTP — no desktop, no navegador e no celular.

## Princípios
1. [[Local-first]]
2. [[Zero-knowledge]] — e nunca guardar arquivos (R-01)
3. Uma interface em todo lugar → [[Agente local]]
4. Leve de verdade (Rust)
5. Bonito por padrão (mockup v0)
6. Open source de ponta a ponta

## Personas
- **Dev full-stack de casa** (o autor): Windows + WSL + Docker + VPS + homelab
- **Dev multiplataforma**: muitos servidores, quer sync confiável
- **Freelancer / pequena equipe**: tenants por cliente → [[Tenant]]
- **Pessoa organizada**: finanças + tarefas + agenda

## Módulos
| Menu                    | Nota                   | Marco |
| ----------------------- | ---------------------- | ----- |
| (casco)                 | [[App desktop]]        | M0–M1 |
| Dashboard               | [[Dashboard]]          | M1    |
| Sistema                 | [[Monitor do sistema]] | M1    |
| Projetos                | [[Projetos]]           | M1    |
| Conexões                | [[Conexões]]           | M2    |
| Financeiro              | [[Financeiro]]         | M3    |
| TaskHub                 | [[TaskHub]]            | M3    |
| (widgets)               | [[Integrações]]        | M1/M3 |
| Configurações → Conta   | [[Contas e sync]]      | M4    |
| (PWA)                   | [[Acesso remoto]]      | M5    |
| Configurações → Tenancy | [[Multitenancy]]       | M6    |
| —                       | [[Site]]               | S1/S2 |

## Negócio
- [[Planos]] — Free / Pro / Team / Self-host

## Referência visual
Mockup v0 — salvar a imagem em `98 Anexos/mockup-v0.png` para aparecer aqui:

![[mockup-v0.png]]
