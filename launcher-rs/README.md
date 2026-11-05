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
`crt`, `crt_strength`, `bands`, `queue_kb`, `vfs_cache_mb`, `widescreen`, `show_window`.

**Precedencia** (maior para menor): linha de comando > ambiente do processo (`PS2X_*` ja definidas)
> `launcher.cfg` > `game.toml` (`[run.env]`). So a linha de comando sobrescreve uma variavel que o
usuario ja exportou; o cfg e o `game.toml` nunca a sobrescrevem (o log registra "ja definido no
ambiente").

**Perfis** (cartoes da aba Jogar; sem `profile` = `recommended`): `recommended` = `hw`, escala 1, CRT
desligado, cache automatico, 60 FPS; `deck` = `cpu-thread`; `original` = `cpu` serial sem extras;
`custom` usa as chaves soltas. Editar qualquer opcao da aba Imagem passa para `custom` mantendo o que
se via. `vfs_cache_mb` vale em qualquer perfil.

### Renderizadores (`--renderer` / `renderer =` / `PS2X_GS`)

| Valor | O que e | Velocidade relativa (vsync/s no menu) |
|---|---|---|
| `hw` (padrao) | GS por hardware D3D11 em thread propria; sinonimo `hw-thread` | ~23, o mais rapido |
| `cpu-thread` | GS em thread + rasterizacao em faixas; alternativa segura | ~22 |
| `cpu` | CPU serial puro (referencia); sinonimo `cpu-serial` | ~7 |
| avancado: `cpu-bands` | faixas sem thread de GS | ~12 |
| avancado: `d3d11` | backend ROV (lento) | ~7 |
| avancado: `hw-serial` | o hw antigo, na thread do jogo | menor que `hw` |

Ressalvas do `hw`: pequenas diferencas visuais (ex.: brilho de holofote) e capturas de tela pretas
sem `PS2X_GS_HW_CPUFRAME=1` (so para regressao; lento; nao exposto no launcher).

**Fallback hw -> cpu-thread:** se o `hw` nao iniciar, o runtime escreve `[gs-hw] ... using cpu` no log
do jogo e segue na CPU (lento). O launcher le o log (8 s depois de abrir, ao fim e na checagem da
GUI) e registra um aviso sugerindo `cpu-thread`. No CLI, se o `hw` fechar com erro nos primeiros 20 s,
o `run` tenta uma vez de novo com `cpu-thread` (registrado no log). A GUI so avisa; nao reinicia.
`PS2X_GS_BANDS` nunca e exportado junto com `hw` e `PS2X_GS_SCALE` so com `hw`/`hw-serial` (o
launcher registra o aviso quando ignora).

### Opcoes e variaveis

| Opcao CLI | Chave do cfg | Variavel de ambiente | Valores |
|---|---|---|---|
| `--renderer R` | `renderer` | `PS2X_GS` | tabela acima |
| `--internal-scale N` | `internal_scale` | `PS2X_GS_SCALE` | 1..8 (padrao 1), so hw |
| `--crt [NIVEL]` | `crt` | `PS2X_POSTFX_CRT` | `off` (exporta 0), `suave`, `medio` (padrao de `--crt` sem valor), `forte`; F9 liga/desliga no jogo |
| `--crt-strength P` | `crt_strength` | `PS2X_POSTFX_CRT_STRENGTH` | 0..100 %, exportado como fracao `0.00..1.00`; ausente = a do nivel |
| `--vfs-cache-mb N` | `vfs_cache_mb` | `PS2X_VFS_CACHE_MB` | 0..16384 MiB; 0 desliga; ausente = automatico (nada exportado) |
| `--bands N\|auto\|off` | `bands` | `PS2X_GS_BANDS` | so cpu-thread/cpu-bands; off exporta 0; ausente = padrao do runtime |
| `--queue-kb N` | `queue_kb` | `PS2X_GS_QUEUE_KB` | 256..262144; 8192 reduz a latencia de exibicao |
| `--vsync` | `vsync` | `PS2X_GS_VSYNC` | hw e d3d11 |
| `--fps N` / `--fps-limit N` | `fps` | `PS2X_TARGET_FPS` | 0 = ilimitado (gasta uma thread do driver) |
| `--widescreen` | `widescreen` | `PS2X_WIDESCREEN=1` | **planejado**: sem efeito no runtime ainda |
| `--heartbeat` | `heartbeat` | `PS2X_HEARTBEAT` | diagnostico |

Valores invalidos na CLI sao erro (codigo 2, com a lista aceita); no cfg viram aviso e a chave e
ignorada. Tudo que sera exportado fica no log (`opcoes de execucao: ...`).

**Hardware** (`src/hw.rs`): RAM total/livre (`GlobalMemoryStatusEx`) e VRAM dedicada (PowerShell:
registro do driver, `Win32_VideoController` de reserva) vao para o log em `info`/`run`; a GUI detecta
em segundo plano ao abrir. Avisa, sem bloquear, se RAM < 8 GiB ou VRAM < 4 GiB com escala > 2 ou cache
>= 1024 MiB. Com VRAM >= 6 GiB e escala 1 a GUI sugere 2x como "se quiser nitidez" (nao aplica).

**GUI (aba Imagem):** renderizador (Hardware / CPU thread / CPU serial; "Avancado" recolhido com
d3d11, hw-serial, cpu-bands, cada um com a velocidade e a ressalva), resolucao interna 1x-8x (so hw),
CRT (desligado/suave/medio/forte + forca 25-100 %), cache de arquivos (Automatico/Desligado/256-2048
MB), threads de rasterizacao (Auto/Desligado/2/4/8, so cpu-thread/cpu-bands), fila do GS (padrao/8192/
16384 KB), limite de FPS, VSync, diagnostico e Widescreen desabilitado ("planejado"). Tudo vai para o
`launcher.cfg`. Smoke: `ps2launch run --renderer hw --timeout 20` e `--renderer cpu-thread --timeout 20`.

Tabela de edicao: `[[dump.file]]` no `game.toml` (nome, tamanho, sha256). `verify` confere os tamanhos pelo diretorio da imagem (rapido); `verify --deep` confere os hashes; `hashes` imprime o bloco a partir do manifesto (para uma edicao nova). Edicao desconhecida ou imagem truncada e RECUSADA.

Proximo: `steam-add`, gerar a arte a partir do dump do usuario
(ver `docs/steam-library.md`). Nada do jogo e embutido nem baixado.
