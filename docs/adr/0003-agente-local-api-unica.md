# ADR-0003 · Núcleo local com uma API única para desktop, navegador e celular

- **Status:** Aceito (2026-10-17, no fechamento do M0)
- **Data:** 2026-10-14
- **Relacionados:** [Arquitetura](../02-arquitetura.md) · [APIs](../06-api-e-protocolos.md#api-do-agente-local) · [Acesso remoto](../modulos/acesso-remoto.md) · [Segurança §8](../04-seguranca.md#8-api-local)

## Contexto

O app precisa estar disponível na janela nativa, no navegador do próprio PC, em outros dispositivos da rede e no celular de qualquer lugar. Manter caminhos de comunicação diferentes para cada caso multiplica bugs e superfícies de ataque.

## Decisão

- Todo o trabalho acontece no **núcleo Rust** (`codatos-core`), que expõe **uma API**: HTTP para arquivos estáticos e sessão, **WebSocket com JSON-RPC** para comandos/assinaturas e **canais binários** para terminal/SFTP.
- A UI usa essa API **inclusive dentro da janela do Tauri**. O IPC do Tauri fica restrito a funções de janela/bandeja/diálogos.
- Acesso remoto reutiliza o mesmo protocolo dentro de um túnel cifrado ([ADR-0007](0007-relay-noise.md)).
- Segredos e chaves nunca saem do núcleo; a UI recebe apenas dados de exibição.
- **Origem da UI no desktop** (decidido em 2026-10-15): a janela do Tauri carrega o bundle **embutido no app** (`tauri://localhost` no macOS/Linux, `http(s)://tauri.localhost` no Windows) e fala com o núcleo em `127.0.0.1:47800` por WebSocket. O núcleo aceita essas origens numa lista fixa; o token do app chega à UI pelo IPC do Tauri e vai no subprotocolo do WebSocket, nunca na URL. Descartado: apontar a janela para `http://127.0.0.1:47800`, que exigiria liberar o IPC do Tauri para uma origem HTTP.

## Alternativas consideradas

- **IPC do Tauri para o desktop + API HTTP só para o navegador** — dois caminhos para manter e testar.
- **Núcleo como serviço do sistema (Windows Service/launchd/systemd)** — sobrevive ao logout, mas exige instalação com privilégio e complica o autostart por usuário. Fica como evolução possível.

## Consequências

- ✅ Uma única superfície de API para testar, documentar e proteger.
- ✅ Recursos novos aparecem automaticamente no navegador e no celular.
- ⚠️ A API local precisa de proteções fortes contra sites maliciosos (token, Host/Origin, SameSite) — ver [Segurança §8](../04-seguranca.md#8-api-local).
- ⚠️ Uma porta TCP local fica aberta enquanto o app roda (somente em `127.0.0.1` por padrão).
