---
tags: [modulo]
status: planejamento
marco: M4
spec: docs/modulos/contas-e-sync.md
---
# Contas e sync

Login/cadastro (mockup: abas Entrar/Cadastrar, GitHub), senha mestra, 2FA, chave de recuperação, dispositivos; tela de **Sincronização** com categorias; assinatura.

**Conceitos:** [[Zero-knowledge]] · [[Hierarquia de chaves]] · [[Planos]] · [[Cofre]]

## Fios soltos
- Prelogin → Argon2id no cliente → só `authKey` vai ao servidor.
- GitHub autentica, mas o cofre ainda pede senha mestra.
- Merge 3-way no cliente (servidor não entende o conteúdo).
- Sync exige plano pago; dados na nuvem retidos 90 dias após o fim.

## O que sincroniza
Conexões ✅ · Configurações ✅ · Painel ✅ · TaskHub ⏳ (Q-05) · Tenants ✅ · Financeiro ❌ por padrão (Q-07 ✅) · Arquivos ❌ nunca
