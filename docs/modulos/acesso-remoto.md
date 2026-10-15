# Módulo · Acesso remoto (navegador e celular)

> **Prefixo:** `RF-REM` · **Marco principal:** M1 (local) / M5 (remoto) · **Status:** Rascunho
> ↑ [Requisitos](../01-requisitos.md) · Base técnica: [ADR-0003](../adr/0003-agente-local-api-unica.md) · [ADR-0007](../adr/0007-relay-noise.md) · [Arquitetura](../02-arquitetura.md) · [APIs → relay](../06-api-e-protocolos.md#protocolo-do-relay) · [Segurança](../04-seguranca.md#9-relay-e-acesso-remoto)

## Objetivo

Usar o CodatOS **fora da janela do app**: no navegador do próprio PC, em outro computador da casa e — para assinantes — no celular, de qualquer lugar, controlando o PC que está ligado.

## Três modos

| Modo | Como acessa | Requisitos | Plano |
|---|---|---|---|
| **Local** | `http://127.0.0.1:47800` (bandeja → "Abrir no navegador") | App em execução | Free |
| **Rede local (LAN)** | `https://<ip-do-pc>:47800` em outro dispositivo da rede | Opt-in + pareamento por QR code | Free |
| **Remoto** | `https://app.codatos.dev` (PWA) → escolher o PC | Conta + plano pago + "Acesso remoto" ligado no PC | Pro |

No modo remoto o PC mantém **uma conexão de saída** com o relay: nenhuma porta precisa ser aberta no roteador. O canal entre o celular e o PC é cifrado de ponta a ponta (Noise); o relay só encaminha bytes que não consegue ler.

## Requisitos funcionais

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-REM-001 | O núcleo serve a **mesma UI** do app no navegador local. | Deve | M1 |
| RF-REM-002 | Autenticação local: "Abrir no navegador" gera um token de uso único que vira cookie de sessão `HttpOnly`/`SameSite=Strict`; sem token, o navegador pede PIN/desbloqueio do cofre. | Deve | M1 |
| RF-REM-003 | Modo LAN opt-in: escuta na interface escolhida, TLS com certificado do próprio dispositivo, pareamento por QR code/código de 6 dígitos, lista de dispositivos pareados revogáveis. | Deveria | M3 |
| RF-REM-004 | Ativar "Acesso remoto" por dispositivo (Pro): conexão WebSocket de saída persistente com o relay, com reconexão e *backoff*. | Deve | M5 |
| RF-REM-005 | PWA instalável: lista de dispositivos da conta com status online/offline e "visto por último". | Deve | M5 |
| RF-REM-006 | Canal E2E (Noise) entre cliente e PC, autenticado pelas chaves de dispositivo registradas na conta. | Deve | M5 |
| RF-REM-007 | Capacidades remotas — ver tabela abaixo. | — | M5 |
| RF-REM-008 | Ações sensíveis pedem confirmação (e reautenticação quando críticas) e geram notificação no PC: "iPhone do André parou o container `api`". | Deve | M5 |
| RF-REM-009 | Sessões remotas ativas visíveis no PC (indicador na bandeja) e encerráveis com um clique. | Deve | M5 |
| RF-REM-010 | Política de acesso por dispositivo: só o dono, ou membros do tenant com `remote.access`. | Deveria | M6 |
| RF-REM-011 | Caminho P2P (WebRTC) quando possível, usando o relay só para sinalização. | Poderia | Futuro |
| RF-REM-012 | Guia para self-hosters: acesso remoto sem a nuvem via Tailscale/ZeroTier + modo LAN. | Poderia | M5 |

### Capacidades remotas (proposta)

| Capacidade | Prioridade | Observação |
|---|---|---|
| Ver dashboard e métricas ao vivo | Deve | Dados lidos do PC, nunca da nuvem |
| Ver financeiro e tarefas; criar tarefa/transação | Deve | Respeita modo privacidade |
| Iniciar/parar containers e distros WSL | Deve | Confirmação + notificação no PC |
| Bloquear a tela, suspender, reiniciar, desligar o PC | Deveria | Reautenticação |
| Terminal do PC e SSH "através" do PC | Deveria | Reautenticação + aviso; desligado por padrão |
| SFTP através do PC | Poderia | Bytes cifrados pelo relay, nunca armazenados |
| Wake-on-LAN (acordar o PC) | Poderia | Requer outro dispositivo CodatOS ligado na mesma rede |

## Regras de negócio

- O acesso remoto é desligado por padrão e ligado **por dispositivo**.
- O relay nunca persiste mensagens; mantém apenas presença (online/offline) em memória/Valkey.
- A PWA é servida pelo domínio da nuvem. Como todo cliente web, ela depende de confiar no código servido — risco e mitigações em [Segurança](../04-seguranca.md#10-nuvem). Para máxima segurança, o app desktop é sempre o cliente recomendado.

## Critérios de aceite

- **Dado** o PC com acesso remoto ligado atrás de um NAT sem portas abertas, **quando** o usuário abre a PWA no 4G, **então** o PC aparece online e o dashboard carrega em até 3 s.
- **Dado** que o celular parou um container, **quando** a ação conclui, **então** o PC exibe notificação nativa com o nome do dispositivo e da ação.
- **Dado** que o dispositivo celular foi revogado na conta, **quando** ele tenta reconectar, **então** o relay recusa e o PC encerra qualquer sessão ativa dele.

## Perguntas em aberto

- O que o celular deve poder fazer? → [Q-23](../10-perguntas-em-aberto.md#acesso-remoto)
- PWA basta ou app nativo? → [Q-24](../10-perguntas-em-aberto.md#acesso-remoto)
- Modo LAN gratuito? → [Q-25](../10-perguntas-em-aberto.md#acesso-remoto)
