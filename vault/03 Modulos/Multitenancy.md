---
tags: [modulo]
status: planejamento
marco: M6
spec: docs/modulos/multitenancy.md
---
# Multitenancy

Tela "Tenancy e Permissões" (mockup): abas Tenants · Usuários · Grupos · Permissões. Tenants com membros, grupos criáveis/editáveis/excluíveis, catálogo de permissões, cofres compartilhados, auditoria.

**Conceitos:** [[Tenant]] · [[Grupos e permissões]] · [[Cofre]] · [[Hierarquia de chaves]]

## Fios soltos
- Sempre ≥ 1 admin; não remover o último.
- Remover membro → revogar + rotacionar chave do cofre.
- Fingerprint da chave pública do convidado (evitar troca por servidor malicioso).
- Postgres RLS como segunda barreira.

## Em aberto
- ✅ Q-20: tenant = contextos e empresas; sub user com conta própria
- Só no Team? Por assento? (Q-21)
