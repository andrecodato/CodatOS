---
tags: [conceito, seguranca]
---
# Hash vs criptografia

O pedido original: "senhas, chaves SSH e tokens escondidos com hash". O **objetivo** está certo — ninguém além do usuário lê. A **ferramenta** muda conforme o caso:

| | Hash | Criptografia |
|---|---|---|
| Volta ao original? | ❌ Nunca | ✅ Com a chave |
| Serve para | *Conferir* (senha de login, tokens no servidor) | *Usar depois* (senha de servidor, chave SSH, token OAuth) |
| No CodatOS | Argon2id (senha mestra), SHA-256 (refresh tokens, convites) | XChaCha20-Poly1305 com chaves do [[Cofre]] |

> Uma chave SSH guardada como hash não conecta em lugar nenhum. Por isso: **hash para verificar, criptografia ponta a ponta para guardar**.

Ver [[Zero-knowledge]] · [[Hierarquia de chaves]]
📄 `docs/04-seguranca.md#2-hash-vs-criptografia`
