---
tags: [conceito, seguranca]
aliases: [E2E, Ponta a ponta, Criptografia ponta a ponta]
---
# Zero-knowledge

O servidor **não tem como** conhecer o conteúdo do que guarda. Tudo é cifrado no dispositivo com chaves derivadas da senha mestra, que nunca sai de lá.

- Vazamento total do banco → só texto cifrado + metadados (e-mail, tenants, contagens).
- Login social (GitHub) **autentica**, mas não **desbloqueia** — o cofre ainda pede senha mestra/biometria.
- Perdeu senha mestra **e** chave de recuperação, sem dispositivo confiável desbloqueado → dados da nuvem irrecuperáveis (Q-19 ✅; o servidor nunca guarda cópia da chave).
- Merge, busca e validação de conteúdo só acontecem no cliente.

Como funciona: [[Hierarquia de chaves]] · Por que não "hash de tudo": [[Hash vs criptografia]]

📄 `docs/04-seguranca.md` · `docs/adr/0004-criptografia-ponta-a-ponta.md`
Inspiração: Bitwarden, 1Password, Standard Notes.
