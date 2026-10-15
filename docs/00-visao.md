# 00 · Visão do produto

> **Status:** Rascunho · **Versão:** 0.1 · **Atualizado:** 2026-10-14
> ← [Índice](README.md) · Próximo: [01 · Requisitos](01-requisitos.md) →

## Em uma frase

**CodatOS é o painel de controle pessoal do seu computador**: um app leve que abre junto com o sistema e reúne, numa única tela bonita, suas finanças, tarefas, agenda, clima, a saúde do PC (temperatura, discos, WSL, Docker), seus projetos e suas conexões SSH/SFTP — no desktop, no navegador e no celular.

## O problema

Quem usa o computador como ferramenta de trabalho vive pulando entre dezenas de lugares:

- o Gerenciador de Tarefas para ver se o PC está sofrendo;
- `wsl -l -v` e `docker ps` (em cada distro!) para saber o que está rodando e quanto disco cada coisa ocupa;
- Termius/PuTTY para servidores, com senhas e chaves espalhadas;
- uma planilha ou app para o financeiro, outro para tarefas, outro para a agenda;
- o Explorer/VS Code para achar "aquele projeto" da semana passada.

Nada disso conversa entre si, nada aparece de relance e quase nada funciona longe do PC.

## A proposta

Um único lugar, configurável, que:

1. **Mostra** — dashboard de widgets que você monta do seu jeito.
2. **Age** — conecta em servidores, sobe e derruba containers e distros, abre projetos.
3. **Acompanha você** — o mesmo app no navegador e, no plano pago, controle do PC pelo celular.
4. **Respeita sua privacidade** — tudo fica no seu dispositivo; o que sincroniza vai criptografado de ponta a ponta. Nós nunca conseguimos ler.

## Para quem

| Persona | Contexto | O que mais importa |
|---|---|---|
| **Dev "full-stack de casa"** (persona principal — o próprio autor) | Windows + WSL + Docker, alguns VPS, homelab | Ver tudo de relance; SSH rápido; saber o que consome CPU e disco |
| **Dev multiplataforma** | macOS/Linux, muitos servidores | Gerenciador de conexões confiável e sincronizado |
| **Freelancer / pequena equipe** | Clientes diferentes, servidores compartilhados | Tenants separados por cliente; acesso compartilhado com permissões |
| **Pessoa organizada** | Quer finanças, tarefas e agenda num só lugar | Dashboard bonito e privacidade |

## Princípios

1. **Local-first** — o app é completo sem internet e sem conta. A nuvem é opcional. ([ADR-0005](adr/0005-local-first.md))
2. **Zero-knowledge** — segredos e configurações sincronizadas são cifrados no seu dispositivo; o servidor guarda apenas dados ilegíveis. ([ADR-0004](adr/0004-criptografia-ponta-a-ponta.md))
3. **Nunca guardamos seus arquivos** — o servidor só armazena configurações. ([R-01](01-requisitos.md#4-restrições-invioláveis))
4. **Uma interface, todo lugar** — a mesma UI no app, no navegador e no celular. ([ADR-0003](adr/0003-agente-local-api-unica.md))
5. **Leve de verdade** — Rust nativo; quase zero de CPU quando ninguém está olhando.
6. **Bonito por padrão** — estética moderna e escura, no nível de Vercel e Linear. ([Design system](07-design-system.md))
7. **Open source de ponta a ponta** — app, servidor e site. Qualquer pessoa pode auditar e hospedar o próprio servidor.

## Não objetivos

- Não é serviço de backup nem de armazenamento de arquivos.
- Não é ERP financeiro nem app de investimentos; não executa transações bancárias.
- Não substitui o Docker Desktop/Portainer nem ferramentas de monitoramento corporativo (Zabbix, Grafana).
- Não é acesso remoto de tela (TeamViewer/AnyDesk): o controle remoto é **do CodatOS**, não da área de trabalho.
- Não coleta telemetria por padrão.

## Modelo de sustentabilidade

Open source com nuvem paga opcional, no modelo Bitwarden/Standard Notes:

- **Free** — app completo, local, sem conta.
- **Pro** — paga a infraestrutura de sincronização e do relay de acesso remoto pelo celular.
- **Team** — membros, grupos de permissão e cofres compartilhados.
- **Self-host** — quem quiser hospeda o próprio servidor.

Detalhes em [Contas, sync e planos](modulos/contas-e-sync.md#planos-proposta) e na [Q-16](10-perguntas-em-aberto.md#contas-sync-e-planos).

## Como saberemos que deu certo

- O autor usa o CodatOS como primeira tela do dia, todos os dias, e desinstala o Termius.
- O dashboard fica pronto em menos de 2 s após o login no Windows e consome menos de 1% de CPU em repouso.
- Nenhum incidente de segurança consegue expor segredos de usuários, porque o servidor não teria como lê-los.
- Comunidade ativa: contribuições externas, layouts e widgets compartilhados, estrelas no GitHub.

## Referências e inspirações

- **Termius** — gerenciador de conexões e sincronização de chaves.
- **Raycast** — paleta de comandos e velocidade.
- **Homepage / Homarr / Dashy** — dashboards self-hosted baseados em widgets.
- **Bitwarden** — modelo de criptografia e de negócio open source.
- **Vercel, Linear, Render, claude.ai** — linguagem visual.
- **Mockup v0** — referência visual oficial do app ([design system](07-design-system.md#inventário-de-telas-mockup-v0)).
