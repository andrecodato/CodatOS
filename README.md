<div align="center">

# CodatOS

### Seu universo, em um só lugar.

O painel de controle pessoal do seu computador.<br/>
Finanças, tarefas, agenda, servidores e a saúde do seu PC numa tela só —<br/>
**bonito, leve, privado e open source.**

[O que faz](#-o-que-ele-faz) · [Privacidade](#-privacidade-de-verdade) · [Planos](#-gratuito-e-open-source) · [Status](#-status-do-projeto) · [Documentação](docs/README.md)

![Status](https://img.shields.io/badge/status-em%20desenvolvimento-7C5CFF?style=flat-square)
![Plataformas](https://img.shields.io/badge/Windows%20%7C%20macOS%20%7C%20Linux-3B82F6?style=flat-square)
![Feito com Rust](https://img.shields.io/badge/feito%20com-Rust-F59E0B?style=flat-square)
![Licença MIT](https://img.shields.io/badge/licen%C3%A7a-MIT-22C55E?style=flat-square)
[![Estrelas no GitHub](https://img.shields.io/github/stars/andrecodato/CodatOS?style=flat-square&color=3B82F6)](https://github.com/andrecodato/CodatOS)

</div>

<!-- Screenshot do produto: docs/assets/mockup-v0.png -->

---

## A ideia

Todo dia a gente abre dez coisas diferentes para entender o próprio computador e a própria rotina: o Gerenciador de Tarefas, o terminal para ver o que está rodando no WSL e no Docker, um app para conectar nos servidores, uma planilha para o dinheiro, outro app para as tarefas, a agenda, a previsão do tempo…

O **CodatOS** junta tudo isso num painel só, que abre junto com o computador e fica sempre a um clique — na janela do app, no navegador ou no celular.

## ✨ O que ele faz

**📊 Um dashboard que é seu**
Monte a tela do seu jeito: arraste, redimensione e escolha os cartões. Saldo do mês, tarefas do dia, agenda da semana, clima, uso do PC, discos, projetos recentes — tudo de relance.

**🖥️ Seu PC sem segredos**
Uso de CPU, memória e placa de vídeo, temperatura, espaço em disco, quais distros do WSL estão ligadas e quanto cada uma ocupa, quais containers Docker estão rodando — inclusive os de dentro do WSL. E dá para ligar e desligar tudo dali.

**🔐 Conexões como deveriam ser**
Seus servidores organizados, chaves SSH guardadas com segurança, terminal moderno em abas, transferência de arquivos por SFTP, túneis — no estilo dos melhores gerenciadores de conexão, e sincronizado entre seus dispositivos.

**💰 Finanças e ✅ tarefas no mesmo lugar**
Receitas, despesas, categorias e metas, com importação do extrato do banco. E o **TaskHub** integrado para suas tarefas, projetos e quadros.

**🌐 Em qualquer lugar**
Abra no navegador do próprio PC ou de outro computador da casa. Com o plano Pro, controle o PC pelo celular de onde estiver: veja o dashboard, pare um container, bloqueie a tela.

**👥 Para você e para o seu time**
Separe a vida em espaços (Pessoal, Trabalho, Estudos). Convide pessoas, crie grupos e decida exatamente o que cada um pode ver e fazer.

## 🔒 Privacidade de verdade

> **Nem nós conseguimos ler suas senhas.**

- **Tudo fica no seu computador.** O CodatOS funciona completo sem internet e sem conta.
- **O que sincroniza vai trancado.** Senhas, chaves e configurações são criptografadas no seu dispositivo *antes* de sair dele. Nosso servidor guarda só um cadeado fechado — e a chave nunca passa por lá.
- **Nunca guardamos seus arquivos.** O servidor armazena apenas configurações, nada mais.
- **Sem rastreamento.** Nenhuma telemetria sem você pedir.
- **Código aberto.** Qualquer pessoa pode conferir como tudo isso funciona.

Quer os detalhes técnicos? Veja [Segurança e privacidade](docs/04-seguranca.md).

## 💸 Gratuito e open source

| | **Free** | **Pro** | **Team** |
|---|:---:|:---:|:---:|
| App completo no seu computador | ✅ | ✅ | ✅ |
| Acesso pelo navegador (no PC e na rede de casa) | ✅ | ✅ | ✅ |
| Sincronização criptografada entre dispositivos | — | ✅ | ✅ |
| Controle do PC pelo celular | — | ✅ | ✅ |
| Membros, grupos e acessos compartilhados | — | — | ✅ |

Prefere não depender de ninguém? A ideia é que você possa **hospedar seu próprio servidor** de sincronização.

## 💻 Onde funciona

**Windows** · **macOS** · **Linux** · qualquer **navegador** moderno · **celular** (app web instalável)

## 🧭 Status do projeto

O CodatOS está **em desenvolvimento**. A fundação (M0) está pronta: o app abre com o sistema, mora na bandeja e usa cerca de 30 MB de RAM parado ([medição](docs/benchmarks/m0.md)). Falta só assinar o instalador do Windows.

- [x] Visão, requisitos e arquitetura
- [x] Protótipo do site com o mosaico interativo
- [x] **M0** · Fundação — o app abre com o sistema e mora na bandeja
- [ ] **M1** · Dashboard local — PC, discos, WSL, Docker, projetos e clima
- [ ] **M2** · Conexões — SSH, SFTP, chaves e túneis
- [ ] **M3** · Produtividade — financeiro, TaskHub e agenda
- [ ] **M4** · Nuvem — contas e sincronização criptografada
- [ ] **M5** · Acesso remoto — controle pelo celular
- [ ] **M6** · Times — membros, grupos e permissões

Detalhes no [roadmap](docs/09-roadmap.md).

## ❓ Perguntas frequentes

**Preciso criar conta?**
Não. Conta só é necessária para sincronizar entre dispositivos, usar o celular de fora de casa ou colaborar com outras pessoas.

**Vocês conseguem ver minhas senhas ou servidores?**
Não. Tudo que sincroniza é criptografado no seu dispositivo com uma chave que só você tem.

**E se eu esquecer minha senha?**
No cadastro você recebe uma chave de recuperação. Guarde-a bem: sem a senha e sem essa chave, nem nós conseguimos recuperar os dados sincronizados — é o preço (e a garantia) da privacidade total. Os dados no seu PC continuam lá.

**Vai pesar no meu computador?**
A meta é quase nada: feito em Rust, consome menos de 1% de CPU quando você não está olhando.

## 🤝 Como participar

O projeto é aberto desde o primeiro dia. Neste momento, a melhor ajuda é:

- ler a [documentação](docs/README.md) e dar opinião;
- sugerir ideias e recursos nas issues;
- responder às [perguntas em aberto](docs/10-perguntas-em-aberto.md).

Veja o [guia de contribuição](CONTRIBUTING.md) e o [código de conduta](CODE_OF_CONDUCT.md). Encontrou uma falha de segurança? Siga a [política de segurança](SECURITY.md).

## 📚 Documentação

| Para quem | Comece por |
|---|---|
| Quer entender o produto | [Visão](docs/00-visao.md) |
| Quer saber exatamente o que será feito | [Requisitos](docs/01-requisitos.md) |
| Quer contribuir com código | [Arquitetura](docs/02-arquitetura.md) e [Stack](docs/03-stack.md) |
| Se preocupa com segurança | [Segurança e privacidade](docs/04-seguranca.md) |
| Usa Obsidian | Abra a pasta [`vault/`](vault/Home.md) como vault |

## 📄 Licença

CodatOS é software livre sob a [licença MIT](LICENSE): use, estude, modifique e distribua à vontade.

<div align="center">
<br/>
Feito com ☕ e 🦀 por André Codato
</div>
