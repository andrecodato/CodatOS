# Release e assinatura

> Como uma versão do app desktop sai do `main` e vira instalador no GitHub Releases.
> Stack de referência: [03-stack.md](03-stack.md) (release-please + tauri-action).

## Fluxo

1. Os PRs entram no `main` com título em Conventional Commits (checado pelo `pr-title.yml`).
2. O workflow [`release-please.yml`](../.github/workflows/release-please.yml) mantém aberto um PR `chore(main): release app X.Y.Z` com a próxima versão e o `CHANGELOG.md`.
3. Ao mergear esse PR, o release-please cria a tag **`app-vX.Y.Z`** e o GitHub Release, e chama o [`release.yml`](../.github/workflows/release.yml).
4. O `release.yml` compila com o `tauri-action` em três runners e anexa ao release:

   | Runner | Instalador | Assinado? |
   |---|---|---|
   | `windows-latest` | NSIS (`*-setup.exe`) | Sim, quando os secrets do Azure existem (ver abaixo) |
   | `macos-latest` | DMG (Apple Silicon) | Não no M0 (Developer ID depende da Q-29) |
   | `ubuntu-22.04` | AppImage e `.deb` | Não |

Para refazer os anexos de uma tag que já existe: **Actions → Release → Run workflow** e informe a tag (`app-v0.1.0`).

### Versão

- A versão mora em um lugar só: `[workspace.package] version` no `Cargo.toml` da raiz. O `tauri.conf.json` não tem `version`, então o Tauri usa a do crate.
- O release-please (modo manifesto, `release-type: simple`, componente `app`) atualiza o `Cargo.toml`, as entradas `codatos-*` do `Cargo.lock` e o `.release-please-manifest.json`.
- Antes do 1.0, `feat` sobe o *minor* e `fix` sobe o *patch*; mudança incompatível (`feat!`) também sobe só o *minor*.
- O release-please insere cada versão nova no topo do `CHANGELOG.md`, acima da seção "[Não lançado]" escrita à mão. Quando sair a primeira versão, mover o conteúdo dessa seção para dentro dela.

## Secrets e variáveis

Nada disso vai para o repositório. Configure em **Settings → Secrets and variables → Actions**.

### Assinatura Authenticode (Windows)

A [Q-29](10-perguntas-em-aberto.md) ainda decide entre **Azure Artifact Signing** (antigo Trusted Signing) e um **certificado OV**. O workflow já traz o caminho do Azure; enquanto os secrets não existem, o passo é pulado e o instalador sai sem assinatura (o Windows mostra o alerta do SmartScreen).

| Nome | Tipo | O que é |
|---|---|---|
| `AZURE_CLIENT_ID` | Secret | App Registration com o papel *Artifact Signing Certificate Profile Signer* na conta |
| `AZURE_CLIENT_SECRET` | Secret | Segredo dessa App Registration |
| `AZURE_TENANT_ID` | Secret | Tenant do Azure |
| `AZURE_ARTIFACT_SIGNING_ENDPOINT` | Variável | Endpoint da região, ex.: `https://eus.codesigning.azure.net` |
| `AZURE_ARTIFACT_SIGNING_ACCOUNT` | Variável | Nome da conta de assinatura |
| `AZURE_ARTIFACT_SIGNING_CERTIFICATE_PROFILE` | Variável | Nome do perfil de certificado |

Com eles configurados, o job do Windows instala o [`artifact-signing-cli`](https://github.com/levminer/trusted-signing-cli) e passa ao Tauri um `bundle.windows.signCommand`, que assina o `.exe` do app e o instalador NSIS. Esse caminho ainda não rodou de verdade: valide no primeiro release assinado.

Se a Q-29 escolher certificado OV, troque o passo "Assinatura Authenticode" do `release.yml` por `certificateThumbprint` + importação do `.pfx` (secrets `WINDOWS_CERTIFICATE` em base64 e `WINDOWS_CERTIFICATE_PASSWORD`), conforme o guia de assinatura do Tauri.

### Chave do updater (Ed25519)

O updater só entra no M1, mas o par de chaves deve ser gerado agora para não mudar depois (quem instalar uma versão só aceita atualizações assinadas pela mesma chave).

1. Na sua máquina (não no repositório), gere o par com senha:

   ```sh
   cargo tauri signer generate -w ~/.tauri/codatos.key
   ```

   Isso cria `~/.tauri/codatos.key` (privada) e `~/.tauri/codatos.key.pub` (pública).
2. Guarde a chave privada e a senha no gerenciador de senhas. **Se perder a chave, quem já instalou o app não recebe mais atualizações automáticas.**
3. Crie os secrets:

   | Secret | Conteúdo |
   |---|---|
   | `TAURI_SIGNING_PRIVATE_KEY` | Conteúdo de `~/.tauri/codatos.key` |
   | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | A senha escolhida no passo 1 |

4. No M1, o conteúdo de `codatos.key.pub` vai para `plugins.updater.pubkey` no `tauri.conf.json` (a chave pública pode ficar no repositório), e o `release.yml` passa a usar esses secrets e `includeUpdaterJson: true`.

A chave privada nunca é commitada; o `.gitignore` não é a proteção, e sim ela nunca estar dentro da pasta do projeto.
