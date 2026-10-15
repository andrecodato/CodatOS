---
tags: [conceito, seguranca]
aliases: [RBAC, Permissões, Grupos]
---
# Grupos e permissões

- **Permissão** = chave do catálogo (`connections.use`, `system.control`, `finance.view`…) + escopo opcional (pasta, tag, cofre).
- **Grupo** = conjunto de permissões; criado/editado/excluído por quem tem `groups.manage`.
- **Permissão efetiva** = união dos grupos do membro (sem "negar" na 1.0).
- Grupos do sistema: **Administradores** (tudo) e **Visualizadores** (`*.view`). Modelo: **Desenvolvedores**.

## Quem aplica o quê
| Camada | Garante |
|---|---|
| Servidor (RLS) | Membros, grupos, distribuição de chaves, escrita de itens |
| Criptografia | Só quem tem a chave do [[Cofre]] lê |
| [[Agente local]] | Ações no PC (`system.control`, `remote.access`) |
| UI | Conveniência — nunca a única barreira |

⚠️ `connections.use` sem `connections.reveal` é barreira de UI: quem conecta tem o segredo decifrado no próprio dispositivo.

📄 `docs/modulos/multitenancy.md#catálogo-de-permissões` · [[Tenant]]
