---
tags: [moc, seguranca]
---
# MOC Segurança

> 📄 Formal: `docs/04-seguranca.md` · `docs/adr/0004-criptografia-ponta-a-ponta.md`

## A ideia central
> "Nem nós conseguimos ler suas senhas." — o servidor guarda só texto cifrado.

## Conceitos
- [[Hash vs criptografia]] — por que "hashear tudo" não funciona para chaves SSH
- [[Zero-knowledge]]
- [[Hierarquia de chaves]]
- [[Cofre]]
- [[Relay]] — Noise ponta a ponta
- [[Grupos e permissões]] — onde cada camada aplica as regras

## Restrições invioláveis (resumo)
- R-01 servidor nunca guarda arquivos
- R-02 segredos nunca em texto claro
- R-03 servidor não consegue ler o que sincroniza
- R-04 senha de login nunca chega ao servidor
- R-05 sem telemetria sem opt-in
- R-06 logs sem segredos/conteúdo
- R-07 funciona offline e sem conta
- R-08 tudo open source

## Pontos de atenção
- PWA = código servido pelo servidor → risco de JS malicioso se o servidor for comprometido. Mitigar com SRI, hashes publicados e recomendar o desktop.
- Troca de chave pública de convidado por servidor malicioso → mostrar impressão digital.
- "Conectar sem ver a senha" é barreira de UI, não criptográfica → certificados SSH no futuro.
- Malware no PC está fora do escopo — documentar.
