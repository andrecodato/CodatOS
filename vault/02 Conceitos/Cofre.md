---
tags: [conceito, seguranca]
aliases: [Vault (cofre), Cofres]
---
# Cofre

Conjunto de **itens cifrados** com a mesma chave. (Não confundir com este vault do Obsidian 🙂)

- Todo usuário tem um cofre pessoal; [[Tenant|tenants]] podem ter cofres compartilhados.
- **Item** = um registro (host, identidade, chave, layout…) serializado e cifrado inteiro (XChaCha20-Poly1305, ≤ 64 KiB).
- O servidor vê só `id`, `vault_id`, revisão, tamanho e datas — o tipo do item fica dentro do texto cifrado.
- Localmente: SQLite cifrado (SQLCipher) **+** segredos com segunda camada que exige desbloqueio (senha/PIN/Windows Hello).
- Bloqueio automático: 15 min de inatividade, bloqueio do SO, suspensão.

[[Hierarquia de chaves]] · [[Zero-knowledge]] · 📄 `docs/05-modelo-de-dados.md`
