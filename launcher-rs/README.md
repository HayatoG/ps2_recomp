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
janela). Chaves: `iso`, `profile`, `renderer`, `fps`, `deck_fps`, `vsync`, `heartbeat`, `internal_scale`,
`crt`, `vfs_cache_mb`, `widescreen`, `show_window`.

**Precedencia** (maior para menor): linha de comando > ambiente do processo (`PS2X_*` ja definidas)
> `launcher.cfg` > `game.toml` (`[run].env`). So a linha de comando sobrescreve uma variavel que o
usuario ja exportou; o cfg e o `game.toml` nunca a sobrescrevem (o log registra "ja definido no
ambiente"). Perfis `original`/`recommended`/`deck` fixam renderer, fps e vsync (e ignoram
`internal_scale`, `crt` e `widescreen`); `vfs_cache_mb` vale em qualquer perfil.

| Opcao CLI | Chave do cfg | Variavel de ambiente | Valores |
|---|---|---|---|
| `--renderer R` | `renderer` | `PS2X_GS` | `cpu` (padrao), `cpu-thread`, `d3d11` (ROV atual), `hw` (em desenvolvimento) |
| `--internal-scale N` | `internal_scale` | `PS2X_GS_SCALE` | inteiro 1..8 (padrao 1); resolucao interna do renderer `hw` |
| `--crt [NIVEL]` | `crt` | `PS2X_POSTFX_CRT` (+ `_STRENGTH`) | `off`, `suave` (0.35), `medio` (0.65, padrao de `--crt` sem valor), `forte` (1.0); `off` exporta `PS2X_POSTFX_CRT=0`; a escala de `_STRENGTH` fica em `options::Crt::strength` |
| `--vfs-cache-mb N` | `vfs_cache_mb` | `PS2X_VFS_CACHE_MB` | 0..16384 MiB; 0 desliga; ausente = automatico (nada exportado) |
| `--widescreen` | `widescreen` | `PS2X_WIDESCREEN=1` | **planejado**: reservado, sem efeito no runtime ainda |
| `--vsync` | `vsync` | `PS2X_GS_VSYNC` | so `d3d11` |
| `--fps N` | `fps` | `PS2X_TARGET_FPS` | 0 = ilimitado |
| `--heartbeat` | `heartbeat` | `PS2X_HEARTBEAT` | diagnostico |

Valores invalidos na CLI sao erro (codigo 2, com a lista de valores aceitos); no cfg viram aviso e
a chave e ignorada. Tudo que sera exportado e registrado no log (`opcoes de execucao: ...`).

**Hardware** (`src/hw.rs`): `ps2launch info` e `run` registram RAM total/livre
(`GlobalMemoryStatusEx`) e VRAM dedicada (PowerShell: registro do driver, com
`Win32_VideoController` de reserva; so consultada em `info` ou quando `internal_scale > 2` /
cache >= 1024 MiB, pois custa algumas centenas de ms). Avisa, sem bloquear, se RAM < 8 GiB ou
VRAM < 4 GiB com escala > 2 ou cache grande. Sem DXGI para nao adicionar crates.

GUI: aba Imagem ganhou os chips `CPU (thread)` e `Hardware`; escala, CRT e cache ainda so pelo
`launcher.cfg`/CLI (pendente).

Tabela de edicao: `[[dump.file]]` no `game.toml` (nome, tamanho, sha256). `verify` confere os tamanhos pelo diretorio da imagem (rapido); `verify --deep` confere os hashes; `hashes` imprime o bloco a partir do manifesto (para uma edicao nova). Edicao desconhecida ou imagem truncada e RECUSADA.

Proximo: `steam-add`, gerar a arte a partir do dump do usuario
(ver `docs/steam-library.md`). Nada do jogo e embutido nem baixado.
