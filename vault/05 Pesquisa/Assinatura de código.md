---
tags: [pesquisa]
status: aberta
relacionado: Q-29
---
# Assinatura de código

Sem assinatura, o Windows (SmartScreen) e o macOS (Gatekeeper) mostram alertas assustadores ao instalar — ruim para um app que pede confiança.

## Windows
- Certificados **OV/EV** exigem chave em hardware (token/HSM); a reputação no SmartScreen cresce com downloads. _(verificar se EV ainda dá reputação imediata)_
- **Azure Trusted Signing**: serviço gerenciado da Microsoft, barato; checar disponibilidade para pessoa física/empresa no Brasil.
- **SignPath Foundation**: assinatura **gratuita para projetos open source** que cumprem os requisitos — forte candidato.

## macOS
- Apple Developer Program (US$ 99/ano) → certificado **Developer ID** + **notarização** (`notarytool`). Sem isso, o app nem abre com duplo clique.

## Linux
- Assinar repositórios (.deb/.rpm) com GPG; publicar checksums SHA-256.

## Atualizações
- O updater do Tauri usa um par Ed25519 próprio (`tauri signer generate`). Guardar a chave privada como segredo do CI; perder = não conseguir atualizar quem já instalou.

[[App desktop]] · [[Roadmap]]
