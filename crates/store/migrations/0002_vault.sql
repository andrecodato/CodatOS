-- Cofre local (docs/04-seguranca.md §7): uma linha com o header do cofre em JSON.
-- A chave do cofre nunca fica aqui em claro: vai embrulhada pela senha ou no keychain do SO.

CREATE TABLE vault (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    header     TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;
