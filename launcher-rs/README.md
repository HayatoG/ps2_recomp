# ps2launch (launcher em Rust)

Launcher leve e generico para ports PS2: exige o dump original do usuario, valida serial + sha256
do ELF de boot, extrai os dados do disco para `<jogo>/work/` (atomico, com manifesto e checagem de
espaco) e inicia o port. Substitui `launcher/ps2launcher.py` (que fica como prototipo). Plano e
decisoes: `docs/launcher-plan.md`.

- Dependencias minimas: `serde`, `toml`, `sha2`. Binario de release ~0,5 MB.
- Le o mesmo `game.toml` do launcher Python (`ghost_pc/game.toml`).
- Aceita ISO 2048 e imagens BIN de 2352 bytes por setor (modos 1 e 2).
- Logs em `<jogo>/logs/launcher_AAAA-MM-DD.log` (UTC) e `run_*.log` (saida do port).

```
cargo build --release
target\release\ps2launch.exe verify  --game ..\ghost_pc [--deep]
target\release\ps2launch.exe extract --game ..\ghost_pc [--force]
target\release\ps2launch.exe run     --game ..\ghost_pc --fps 60
```

Tabela de edicao: `[[dump.file]]` no `game.toml` (nome, tamanho, sha256). `verify` confere os tamanhos pelo diretorio da imagem (rapido); `verify --deep` confere os hashes; `hashes` imprime o bloco a partir do manifesto (para uma edicao nova). Edicao desconhecida ou imagem truncada e RECUSADA.

Proximo: `launcher.cfg`, GUI minima com gamepad, `artwork` e `steam-add`
(ver `docs/steam-library.md`). Nada do jogo e embutido nem baixado.
