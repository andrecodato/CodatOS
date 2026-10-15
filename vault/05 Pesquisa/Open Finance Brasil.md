---
tags: [pesquisa]
status: aberta
relacionado: Q-06
---
# Open Finance Brasil

Ideia: importar transações automaticamente em vez de OFX/CSV.

## O que se sabe
- O Open Finance Brasil é regulado pelo Banco Central; participar **diretamente** exige ser instituição autorizada.
- O caminho para apps é usar **agregadores** (ex.: Pluggy, Belvo), que cobram por conexão/uso. _(levantar preços e se há plano para desenvolvedor)_
- O consentimento do usuário tem validade e precisa ser renovado.
- Implicação de arquitetura: o agregador recebe os dados bancários → conflita com o espírito [[Local-first]]/[[Zero-knowledge]] e teria custo recorrente por usuário.

## Alternativas sem custo
- **OFX**: a maioria dos bancos brasileiros exporta extrato em OFX.
- **CSV**: faturas de cartão e apps como Nubank/Inter.
- Lançamento manual rápido (paleta de comandos, celular).

**Inclinação:** OFX/CSV na 1.0; Open Finance só se virar recurso pago com consentimento explícito.

[[Financeiro]]
