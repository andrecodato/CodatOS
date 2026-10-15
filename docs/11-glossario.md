# 11 · Glossário

> ← [Perguntas em aberto](10-perguntas-em-aberto.md) · [Índice](README.md)

| Termo | Significado |
|---|---|
| **Agente / núcleo (`codat-core`)** | Parte Rust do app que roda no PC: coleta métricas, conecta em servidores, guarda o cofre, sincroniza e serve a API local. |
| **API local** | HTTP + WebSocket servidos pelo núcleo em `127.0.0.1:47800`; usada pela janela, pelo navegador e (via relay) pelo celular. |
| **AAD** | *Additional Authenticated Data*: dados não cifrados, mas autenticados junto do texto cifrado (ex.: ID do item). |
| **Argon2id** | Função de derivação de chave/hash de senha resistente a GPU; usada para a senha mestra. |
| **Capacidade** | Recurso que um dispositivo suporta (ex.: `wsl`, `docker`, `nvml`); a UI se adapta a elas. |
| **Chave de recuperação** | Código gerado no cadastro que permite recuperar o cofre se a senha mestra for esquecida. |
| **Cofre (vault)** | Conjunto de itens cifrados com a mesma chave. Todo usuário tem um cofre pessoal; tenants podem ter cofres compartilhados. Não confundir com o *vault do Obsidian*. |
| **Coletor** | Componente do núcleo que lê uma fonte de dados (CPU, discos, WSL, Docker…). |
| **Dispositivo** | Uma instalação do CodatOS (ou uma sessão da PWA) registrada na conta, com seu próprio par de chaves. |
| **E2E / ponta a ponta** | Os dados são cifrados no seu dispositivo e só decifrados em outro dispositivo seu; quem está no meio (servidor, relay) não lê. |
| **Engine Docker** | Um daemon Docker (Docker Desktop, dockerd nativo, dockerd dentro de uma distro WSL, Podman). |
| **Grupo** | Conjunto de permissões dentro de um tenant, atribuído a membros. |
| **Item** | Unidade de sincronização: um registro (host, identidade, layout…) serializado e cifrado. |
| **Local-first** | Arquitetura em que o dispositivo é a fonte da verdade e funciona offline; a nuvem só sincroniza. |
| **Marco (M0–M6)** | Etapa do [roadmap](09-roadmap.md). |
| **Membro / sub user** | Pessoa convidada para um tenant. |
| **Modo privacidade** | Oculta valores sensíveis (financeiro, IPs) na tela. |
| **Noise** | Framework de protocolos criptográficos usado no canal ponta a ponta do acesso remoto. |
| **Permissão** | Chave do catálogo (ex.: `connections.use`) que libera uma ação. |
| **Plano Free / Pro / Team** | Níveis de uso: local gratuito; sync + acesso remoto; colaboração em times. |
| **PWA** | *Progressive Web App*: a UI do CodatOS instalável no celular pelo navegador. |
| **Relay** | Serviço da nuvem que encaminha bytes cifrados entre o celular e o PC, sem conseguir lê-los. |
| **RLS** | *Row-Level Security* do PostgreSQL: o banco filtra linhas por usuário/tenant. |
| **Senha mestra** | Senha da conta que deriva as chaves de criptografia; nunca sai do dispositivo. |
| **Tenant** | Espaço isolado (Pessoal, Trabalho…) com seus próprios dashboards, conexões, finanças, tarefas, membros e grupos. |
| **Widget** | Cartão do dashboard que exibe dados de um módulo. |
| **WSL** | *Windows Subsystem for Linux*; cada **distro** (instância) tem um disco virtual `ext4.vhdx`. |
| **Zero-knowledge** | O provedor do serviço não tem como conhecer o conteúdo dos dados que armazena. |
