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

## Janela (`ps2launch-gui`)

Segundo binario, atras da feature `gui` (o CLI continua so com serde/toml/sha2):

```
cargo build --release --features gui
target\release\ps2launch-gui.exe --game ..\ghost_pc            # janela (ou joga direto se show_window = 0)
target\release\ps2launch-gui.exe --game ..\ghost_pc --settings # sempre abre a janela
target\release\ps2launch-gui.exe --game ..\ghost_pc --no-window # valida, extrai o que falta e inicia
target\release\ps2launch-gui.exe --game ..\ghost_pc --check    # valida dump + port; sai 0 (pronto) ou 2
```

- Abas Jogar (imagem do disco, perfis Original/Recomendado/Steam Deck/Personalizado, dados
  extraidos com botao Extrair e progresso), Imagem (`PS2X_GS`, `PS2X_TARGET_FPS`, `PS2X_GS_VSYNC`,
  `PS2X_HEARTBEAT`; o que o runtime nao tem aparece "em breve") e Controles (so leitura).
- Jogar salva o `launcher.cfg`, inicia o port desacoplado e fecha a janela; se o port cair em ate
  1,5 s, a janela mostra o erro.
- Teclado: Tab/Shift+Tab/setas, Espaco/Enter, Ctrl+Tab ou Q/E (abas), Enter sem foco = Jogar, Esc
  fecha. Controle (gilrs): D-pad, A, B (volta), LB/RB (abas), Start (Jogar).
- Arte opcional e local em `<jogo>/artwork/` (`capsule_vertical.png` ou `hero.png`; `icon.png` =
  icone da janela; `logo.png` no placeholder). Sem arte, um placeholder gerado em codigo. Ponto de
  foco do corte: `[art] focus = [x, y]` no `game.toml`. Nada de arte e embutido.
- Fontes: Segoe UI lidas de `C:\Windows\Fonts` em tempo de execucao (senao, as padrao do egui).
- Textos em `src/gui/strings.rs` (pt-BR; preparado para outro idioma).

## `launcher.cfg`

`<jogo>/launcher.cfg` (`chave = valor`, `#` comenta), gerado comentado na primeira execucao (CLI ou
janela). Chaves: `iso`, `profile`, `renderer`, `fps`, `deck_fps`, `vsync`, `heartbeat`,
`show_window`. Precedencia: flag do CLI (`--iso`, `--fps`, `--renderer`, `--vsync`, `--heartbeat`)
> `launcher.cfg` > padrao do `game.toml`/runtime.

Tabela de edicao: `[[dump.file]]` no `game.toml` (nome, tamanho, sha256). `verify` confere os tamanhos pelo diretorio da imagem (rapido); `verify --deep` confere os hashes; `hashes` imprime o bloco a partir do manifesto (para uma edicao nova). Edicao desconhecida ou imagem truncada e RECUSADA.

Proximo: `steam-add`, gerar a arte a partir do dump do usuario
(ver `docs/steam-library.md`). Nada do jogo e embutido nem baixado.
