# 06 · APIs e protocolos

> **Status:** Rascunho · **Versão:** 0.1 · **Atualizado:** 2026-10-14
> ← [Modelo de dados](05-modelo-de-dados.md) · [Índice](README.md) · Próximo: [Design system](07-design-system.md) →

Três contratos, todos definidos no crate `protocol` (fonte da verdade, gera tipos TS):

1. **API do agente** — entre a UI (desktop/navegador/PWA) e o núcleo no PC.
2. **API da nuvem** — entre o núcleo/PWA e o servidor NestJS.
3. **Relay** — túnel cifrado entre um cliente remoto e o PC.

## API do agente (local)

Base: `http://127.0.0.1:47800` (porta configurável).

### HTTP

| Rota | Uso |
|---|---|
| `GET /` | UI (arquivos estáticos do bundle Leptos) |
| `GET /api/v1/health` | Versão, capacidades e estado do cofre (sem dados sensíveis) |
| `POST /api/v1/session` | Troca token de uso único por cookie de sessão |
| `GET /api/v1/sftp/download/:transfer` | Download de arquivo remoto para o navegador (stream, nada é gravado no servidor) |
| `GET /rpc` (upgrade) | WebSocket principal |

Toda requisição passa pela checagem de `Host` (só `127.0.0.1:<porta>` e `localhost:<porta>`) e, quando há `Origin`, pela lista de origens permitidas: o próprio núcleo e a janela do Tauri (`tauri://localhost`, `http(s)://tauri.localhost`). Fora disso a resposta é `403` com *problem details*. Não há CORS.

`GET /api/v1/health` é pública (nada sensível). O WebSocket exige autenticação:

| Cliente | Como se autentica |
|---|---|
| Janela do app | Token do app, recebido pelo IPC do Tauri, no subprotocolo: `Sec-WebSocket-Protocol: codatos.v1, codatos.token.<token>`. O servidor responde só `codatos.v1`. |
| Navegador | "Abrir no navegador" gera um token de uso único (60 s); a página faz `POST /api/v1/session {"token": …}` e recebe o cookie `codatos_session` (`HttpOnly`, `SameSite=Strict`). |
| Ferramentas locais | `Authorization: Bearer <token do app>`. |

### RPC sobre WebSocket

Mensagens **JSON-RPC 2.0** para comandos e assinaturas; frames **binários** para terminal e SFTP.

```json
→ {"jsonrpc":"2.0","id":7,"method":"docker.containers.list","params":{"engine":"wsl:Ubuntu-22.04"}}
← {"jsonrpc":"2.0","id":7,"result":[{"id":"3f2a…","name":"api","image":"node:22","state":"running","uptimeS":8040}]}

→ {"jsonrpc":"2.0","id":8,"method":"subscribe","params":{"topic":"system.metrics","intervalMs":2000}}
← {"jsonrpc":"2.0","id":8,"result":{"sub":"s1","intervalMs":2000}}
← {"jsonrpc":"2.0","method":"event","params":{"sub":"s1","data":{"cpu":12.4,"ram":{"used":7600000000,"total":16000000000}}}}
← {"jsonrpc":"2.0","method":"event","params":{"sub":"s1","error":{"type":"https://codatos.dev/problems/collector_timeout","title":"A coleta demorou demais","code":"collector_timeout"}}}

→ {"jsonrpc":"2.0","id":9,"method":"unsubscribe","params":{"sub":"s1"}}
← {"jsonrpc":"2.0","id":9,"result":true}
```

Regras das assinaturas:

- `intervalMs` é ajustado ao mínimo do tópico (em geral 500 ms) e a no máximo 1 h; a resposta traz o valor efetivo.
- Um coletor só roda enquanto houver assinantes, e todos os assinantes de um tópico dividem a mesma coleta. Sem assinantes (janela fechada, aba oculta), nada é coletado.
- Falha ou *timeout* de um coletor chega como `event` com `error` (*problem details*) só naquela assinatura; depois de falhas seguidas, o intervalo dobra até 60 s.
- `session.hello` lista em `capabilities` os tópicos disponíveis nesta máquina (ex.: `wsl.distros` só no Windows).
- Até 64 assinaturas por conexão. Se o cliente não lê rápido o bastante, eventos são descartados, nunca acumulados.

### Catálogo de métodos (inicial)

| Domínio | Métodos |
|---|---|
| Sessão | `session.hello` (versão, capacidades), `system.ping`, `vault.status`, `vault.unlock`, `vault.lock` |
| Dashboard | `dashboards.list/get/save`, `widgets.catalog` |
| Assinaturas | `subscribe(topic)`, `unsubscribe(sub)` — tópicos: `system.metrics`, `system.info`, `system.gpu`, `system.cpuSensors`, `disks`, `wsl.distros` (só Windows), `docker.containers`, `projects`, `weather`, `calendar.week`, `tasks.summary`, `finance.summary`, `notifications` |
| Sistema | `processes.list`, `processes.kill` |
| WSL | `wsl.list`, `wsl.start`, `wsl.terminate`, `wsl.shutdown`, `wsl.setDefault` |
| Docker | `docker.engines`, `docker.containers.list`, `docker.container.start/stop/restart`, `docker.logs.follow` |
| Projetos | `projects.list`, `projects.open`, `projects.roots.save` |
| Conexões | `hosts.*`, `identities.*`, `keys.generate/import/exportPublic`, `knownHosts.*`, `snippets.*`, `tunnels.*` |
| Terminal | `terminal.open(hostId \| local \| wsl \| docker)` → `channel`, `terminal.resize`, `terminal.close` |
| SFTP | `sftp.open`, `sftp.list`, `sftp.mkdir/rename/remove/chmod`, `sftp.transfer` → `channel` |
| Financeiro | `finance.accounts.*`, `finance.transactions.*`, `finance.import`, `finance.summary` |
| Tarefas | `tasks.*`, `taskProjects.*` |
| Integrações | `weather.get`, `weather.searchCity`, `calendar.sources.*` |
| Tenants | `tenants.list/switch/create/update/delete` (locais); proxies para a nuvem quando conectados |
| Sync | `sync.status`, `sync.now`, `sync.settings` |

Erros seguem JSON-RPC com `data` no formato *problem details* (RFC 9457): `type`, `title`, `detail`, `code` (ex.: `vault_locked`, `permission_denied`, `capability_unavailable`).

### Canais binários (terminal e SFTP)

```text
┌──────────────┬────────────┬──────────────────────┐
│ channel: u32 │ kind: u8   │ payload               │
└──────────────┴────────────┴──────────────────────┘
kind: 0 = dados · 1 = resize (cols,rows u16) · 2 = fechar · 3 = janela/ack (controle de fluxo)
```

Controle de fluxo por janela de crédito evita que uma saída enorme (`cat` de log gigante) trave a UI ou o relay.

## API da nuvem (REST)

Base: `https://api.codatos.dev/v1`. Autenticação: `Authorization: Bearer <access token>` (15 min) + refresh token rotativo.

| Grupo | Rotas |
|---|---|
| Auth | `POST /auth/prelogin`, `POST /auth/register`, `POST /auth/login`, `POST /auth/refresh`, `POST /auth/logout`, `GET /auth/github` + callback, `POST /auth/2fa/*`, `POST /auth/passkeys/*`, `POST /auth/recover` |
| Conta | `GET/PATCH /me`, `POST /me/password` (re-wrap), `GET /me/export`, `DELETE /me` |
| Dispositivos | `GET /devices`, `POST /devices`, `PATCH /devices/:id`, `DELETE /devices/:id` |
| Tenants | `GET/POST /tenants`, `PATCH/DELETE /tenants/:id`, `POST /tenants/:id/transfer` |
| Membros | `GET /tenants/:id/members`, `POST /tenants/:id/invites`, `POST /invites/:token/accept`, `PATCH/DELETE /tenants/:id/members/:userId` |
| Grupos | `GET/POST /tenants/:id/groups`, `PATCH/DELETE /groups/:id`, `PUT /groups/:id/permissions`, `PUT /groups/:id/members` |
| Permissões | `GET /permissions` (catálogo), `GET /tenants/:id/me/permissions` (efetivas) |
| Cofres | `GET /vaults`, `POST /vaults`, `PUT /vaults/:id/keys` (re-selagem/rotação), `GET /users/:id/public-key` |
| Sync | `GET /sync/changes`, `POST /sync/push`, WS `/events` |
| Billing | `POST /billing/checkout`, `POST /billing/portal`, `POST /billing/webhook` |
| Auditoria | `GET /tenants/:id/audit` |

## Protocolo de sincronização

**Pull** — `GET /sync/changes?since=<cursor>&limit=500`

```json
{
  "items": [
    { "id": "01J9…", "vaultId": "01J8…", "revision": 12, "deleted": false,
      "ciphertext": "base64…", "updatedAt": "2026-10-14T13:02:11Z" }
  ],
  "cursor": "184467",
  "hasMore": false
}
```

**Push** — `POST /sync/push`

```json
{ "changes": [ { "id": "01J9…", "vaultId": "01J8…", "baseRevision": 11, "ciphertext": "base64…", "deleted": false } ] }
```

Resposta por item: `{"id":"01J9…","status":"ok","revision":12}` ou `{"id":"01J9…","status":"conflict","current":{…}}`.

**Conflitos** — o servidor não entende o conteúdo, então o merge é no cliente: ele decifra a versão atual, compara com a base guardada no `sync_outbox` e com a local (merge 3-way por campo). Mesmo campo alterado nos dois lados → mantém a mais recente e salva a outra em `sync_conflicts` para o usuário decidir.

**Tempo real** — o cliente mantém `WS /events`; o servidor avisa `{"type":"sync.changed","cursor":"184470"}` e o cliente faz pull.

**Gate de plano** — `push` exige assinatura ativa; `changes` continua funcionando por 90 dias após o fim do plano (para não prender dados).

## Protocolo do relay

1. **Agente** conecta em `wss://relay.codatos.dev/v1/agent` com token do dispositivo → `hello{deviceId, version}`; o relay registra presença (Valkey).
2. **Cliente** conecta em `wss://relay.codatos.dev/v1/client` com sessão da conta → `open{deviceId}`. O relay valida que o usuário é dono do dispositivo ou tem `remote.access` num tenant ligado a ele, cria um `tunnelId` e avisa o agente.
3. **Handshake Noise XX** de ponta a ponta. Cada lado prova sua chave estática; o agente exige que a chave do cliente esteja **assinada pela identidade Ed25519 do usuário** (ver [Segurança §9](04-seguranca.md#9-relay-e-acesso-remoto)).
4. Dentro do canal Noise trafega **o mesmo protocolo da API do agente** (JSON-RPC + canais binários), fragmentado em mensagens de até 64 KiB.
5. Ping a cada 25 s; túnel encerrado após 10 min ocioso; limites de banda por plano.

O relay só vê: quem conectou em qual dispositivo, quando e quantos bytes — nunca o conteúdo.

## Versionamento

- APIs com prefixo `/v1`; mudanças incompatíveis → `/v2` com período de convivência.
- `session.hello` troca versões e capacidades; a UI se adapta a agentes mais antigos (útil quando o celular fala com um PC desatualizado).
- Formatos cifrados carregam `v` no envelope ([Segurança §4](04-seguranca.md#4-formato-dos-dados-cifrados)).
