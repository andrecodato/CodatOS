---
tags: [conceito]
aliases: [Local first, Offline-first]
---
# Local-first

O **dispositivo é a fonte da verdade**; a nuvem é um espelho cifrado opcional.

- O app funciona 100% sem internet e sem conta (R-07).
- **Dados** (financeiro, métricas, logs) ficam no PC. O celular vê **ao vivo pelo PC**, via [[Relay]] — nada fica guardado na nuvem.
- **Configurações** (conexões, layouts, preferências) sincronizam cifradas → [[Zero-knowledge]].

**Por quê:** privacidade por arquitetura, leituras instantâneas, servidor barato (só blobs pequenos).
**Custo:** ver dados pelo celular exige o PC ligado; backup local (export cifrado) é responsabilidade do usuário.

📄 `docs/adr/0005-local-first.md`
Relacionado: [[Cofre]] · [[Contas e sync]] · [[Financeiro]]
