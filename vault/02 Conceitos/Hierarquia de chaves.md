---
tags: [conceito, seguranca]
---
# Hierarquia de chaves

```
senha mestra ──Argon2id──▶ master key
                              ├─HKDF "auth"─▶ authKey ──▶ servidor guarda Argon2id(authKey)
                              └─HKDF "wrap"─▶ wrapKey ──▶ abre a USER KEY (aleatória)
chave de recuperação ───────────────────────────────────▶ também abre a USER KEY
USER KEY ─▶ chaves privadas (X25519 troca · Ed25519 identidade) + chave do cofre pessoal
X25519 privada ─▶ abre chaves dos cofres de tenants (seladas p/ cada membro)
Ed25519 identidade ─▶ assina chaves de dispositivos e sessões remotas
```

- Trocar a senha = re-cifrar só a user key.
- Cada [[Cofre]] tem sua chave → compartilhar = selar a chave para a chave pública do membro.
- Remover membro = rotacionar a chave do cofre.
- A assinatura da identidade impede o servidor de se passar por um dispositivo seu no [[Relay]].

📄 `docs/04-seguranca.md#3-hierarquia-de-chaves`
