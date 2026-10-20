# xterm.js embutido

Terminal da tela de Conexões (`apps/ui/src/terminal/`). Vem no bundle da UI, sem CDN, e
só é carregado quando a primeira aba de terminal abre.

| Arquivo | Origem | Licença |
|---|---|---|
| `xterm.js`, `xterm.css` | `@xterm/xterm` 6.0.0 (`lib/xterm.js`, `css/xterm.css`) | MIT, `LICENSE-xterm.txt` |
| `addon-fit.js` | `@xterm/addon-fit` 0.11.0 (`lib/addon-fit.js`) | MIT, `LICENSE-addon-fit.txt` |

SHA-256 dos pacotes do npm de onde os arquivos saíram:

- `xterm-6.0.0.tgz`: `908e66e04af6c8dc6b00dd3b54de088e2e81e5ed866284fd6c2fb3c2d1c7a3f6`
- `addon-fit-0.11.0.tgz`: `26003b4517a132b64e4ff228fd88a5fda3fff5e606c76093f6dcff772e9ecec0`

A única mudança é a linha `//# sourceMappingURL=` removida (os `.map` não vêm junto).
Para atualizar, troque os arquivos pelos de uma versão nova e confira a API usada em
`apps/ui/src/terminal/xterm.rs`.
