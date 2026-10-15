---
tags: [pesquisa]
status: aberta
---
# Benchmark: Termius

Referência para o módulo [[Conexões]]. Objetivo: paridade no essencial + diferenciais claros.

## Paridade (o que o Termius tem e o CodatOS precisa ter)
- [ ] Hosts com grupos, tags, herança de configurações
- [ ] Keychain: chaves, identidades, gerar/importar
- [ ] Known hosts com verificação
- [ ] Terminal em abas/split, temas, busca
- [ ] Snippets
- [ ] Port forwarding (L/R/D) salvo
- [ ] SFTP com painel duplo
- [ ] Jump hosts / proxy
- [ ] Sync cifrado ponta a ponta entre dispositivos
- [ ] Compartilhamento em equipe (vaults)
- [ ] Mobile (no nosso caso: PWA controlando o PC)
- [ ] Mosh / Telnet / Serial (depois)

## Diferenciais do CodatOS
- **Open source** e **self-host** do servidor de sync.
- **Integrado ao dashboard**: status dos hosts ao lado de WSL, Docker, projetos.
- Terminais de **WSL e containers Docker** no mesmo lugar (abas Docker/WSL do mockup).
- **Local-first**: tudo funciona grátis e offline; paga-se só pela nuvem.
- Celular controla **o PC inteiro**, não só as conexões.

## A investigar
- Formato de export do Termius (para o importador).
- Como eles tratam "compartilhar sem revelar" em times.
