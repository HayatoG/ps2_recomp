# Architecture

> **Estudo de caso / Case study.** PT-BR: projeto de estudo pessoal; o desenvolvimento só continua no meu tempo disponível, quando possível (sem prazos nem garantia de conclusão). EN: personal case study; development only continues in my available time, when possible (no deadlines, no guarantee of completion). Ver/see `README.md`.

Como este projeto roda um jogo de PlayStation 2 no PC: uma seção por peça do hardware ou da
biblioteca Sony que o jogo usava, dizendo o que a substitui no host e onde está esse código.
Comece por aqui; `LEGAL.md`, `tools.md` e `similar-projects.md` aprofundam outros pontos. O
glossário é a última seção ([Termos](#termos)).

Modelo de documentação: o `docs/port/ARCHITECTURE.md` do ico-pc (MIT). A abordagem técnica é
diferente: o ico-pc compila C decompilado à mão; aqui o código do jogo é **recompilado
estaticamente** do ELF. Detalhes de um jogo específico ficam em `<jogo>/docs/ARCHITECTURE.md`
(ex.: `ghost_pc/docs/ARCHITECTURE.md`).

> Estado: a camada de GS roda **só por software**. O backend de GPU tem a etapa 1 escrita
> (RHI D3D11 que apresenta o frame da CPU; testada fora do runtime, ainda não integrada) —
> [GS: hoje e plano](#gs-hoje-e-plano). Áudio, vídeo (FMV) e memory card ainda não foram
> tratados neste projeto.

## Arquivos

| caminho | o que guarda |
|---|---|
| `AGENTS.md`, `refs.txt` | regras e método (o `refs.txt` nunca é versionado) |
| `tools/` | ferramentas genéricas, com caminhos por argumento: extração de ISO, formatos, ciclo de recompilação, Ghidra headless |
| `tools/ghidra/` | scripts Java do Ghidra: aplicar símbolos, criar funções faltantes, varredura de ponteiros, decompilar um endereço |
| `launcher/ps2launcher.py` | launcher genérico: `info`, `verify`, `extract`, `run`, lendo o `game.toml` de cada jogo |
| `docs/` | docs genéricos (este arquivo, `LEGAL.md`, `tools.md`, `similar-projects.md`) |
| `third_party/PS2Recomp/` | PS2Recomp (GPL-3.0), clonado e **não versionado**; os nossos patches ficam em `<jogo>/patches/` |
| `<jogo>/` | um jogo, repositório próprio: `game.toml`, `docs/`, `patches/`, `pad_scripts/`, `work/` (não versionado) |

Dentro do PS2Recomp (referência, nomes de pasta reais):

| caminho | o que guarda |
|---|---|
| `ps2xRecomp/` | o recompilador `ps2_recomp`: lê o ELF e um `.toml` + CSV de funções e gera C++ |
| `ps2xRuntime/include/runtime/`, `ps2xRuntime/src/lib/` | o runtime: memória, escalonador do EE, pad, áudio, VFS, VU1, GS |
| `ps2xRuntime/src/lib/Kernel/Syscalls/`, `Kernel/Stubs/` | syscalls do kernel e as funções do SDK emuladas por nome (`CD`, `DMA`, `FileIO`, `GS`, `Pad`, `SIF`, `RPC`, `MemoryCard`, `IPU`, `MPEG`…) |
| `ps2xRuntime/src/lib/gs/`, `include/runtime/gs/` | o GS: frontend (registradores, GIF, transferências), interface `GSRasterBackend` e o backend por CPU `GSCpuBackend` |
| `ps2xRuntime/src/lib/vu/` | interpretador de VU1 |
| `ps2xIOP/` | suporte a módulos IRX do IOP |

## Fluxo geral

```
dump do usuário (ISO) ──launcher──► work/ (arquivos extraídos, não versionado)
ELF ──Ghidra (R5900) + scripts──► símbolos/funções (CSV) ──► .toml
.toml + ELF ──ps2_recomp──► C++ (dezenas de milhares de funções)
C++ + ps2xRuntime (+ patches do jogo) ──MSVC──► executável
launcher run ──► executável + ELF/dados do usuário
```

O ciclo inteiro, de Ghidra até o build incremental, está em `tools/recomp_cycle.ps1`. A cópia do
C++ gerado só substitui os arquivos que mudaram (`tools/sync_recomp_output.py`), porque qualquer
mudança num cabeçalho incluído pelo PCH força um rebuild completo de cerca de 14 minutos.

## O que roda

O código do jogo é o ELF **recompilado**: cada função MIPS R5900 vira uma função C++ que opera
sobre um contexto de registradores de 128 bits e sobre a RAM emulada. Nada é interpretado, exceto
os microprogramas da VU1, que têm interpretador próprio (`ps2xRuntime/src/lib/vu/`).

O executável do jogo é o `ps2EntryRunner`, linkado com o runtime e com as funções geradas. O
runtime emula o **SDK Sony por nome** (tabela em `include/ps2_call_list.h`): o jogo chama
`sceCdRead`, `scePadRead` etc. e o runtime responde em C++ sem rodar o código original da
biblioteca. Funções do SDK que o recompilador reconhece viram **stubs**; as que precisam de
tratamento específico do jogo são substituídas por **overrides** (ver abaixo).

## EE: CPU e memória

- Os registradores GPR de 128 bits (incluindo as instruções MMI) são tipos de 128 bits no host
  com SSE/AVX; a FPU do EE e as instruções COP2 (VU0 macro) são traduzidas inline.
- A RAM do EE é um bloco do host (mesmos endereços físicos do PS2); registradores mapeados em
  memória e scratchpad seguem o mapa do EE (`include/runtime/ps2_memory.h`).
- O build de desempenho usa `/O2` com `PS2X_ENABLE_LTCG=OFF`: a otimização de programa inteiro
  estourava a pilha do linker com ~41 mil funções e não dava ganho no runtime isolado.
- A thread do jogo precisa de pilha grande (`/STACK` de 256 MB): o código recompilado usa mais
  pilha nativa do que o original.

## Kernel do EE: threads, semáforos, interrupções

`ps2xRuntime/src/lib/Kernel/EeScheduler.cpp` (`include/runtime/ee_scheduler.h`) implementa as
threads, semáforos e o despacho de interrupções do EE. Pontos que importam:

- **Vblank** vem do relógio do host (60 Hz por padrão; `PS2X_VIDEO_HZ=50` para PAL). O launcher
  escolhe pela região do jogo.
- **Conclusão de DMA** é um sinal em nível, não em borda: se o aviso chega antes de o jogo
  registrar o handler, o runtime guarda o pendente e entrega depois (o "latch" do DMAC).
  Sem isso o jogo espera para sempre.
- O pacing para 30/60 FPS é feito no host (`PS2X_TARGET_FPS`).

## SDK emulado, IOP, SIF e disco

Não há IOP real rodando. As chamadas SIF/RPC do jogo viram stubs em
`Kernel/Stubs/{SIF,RPC}.cpp`. O disco (`sceCd*`) lê arquivos extraídos em `work/` pelo VFS
(`ps2_vfs.cpp`); `fio`/`sceCdSearchFile` e esperas de CD que travam são tratadas por stubs ou
overrides do jogo. Módulos IRX do IOP são só documentados (`<jogo>/docs/iop-modules.md`); o
suporte do PS2Recomp a eles fica em `ps2xIOP/` e não é o caminho que usamos hoje.

## Pad e gamepad virtual

`Kernel/Stubs/Pad.cpp` implementa libpad. O jogo faz um handshake de modo: ele só aceita
**DualShock analógico (id 0x73)**, então `scePadInfoMode` anuncia esse modo; anunciar só o
digital faz o menu ignorar o controle. O **gamepad virtual** (`ps2_virtual_pad`) injeta um estado
de botões/analógicos a partir de um script indexado por frame (`<jogo>/pad_scripts/`), o que
permite navegar e medir sem ninguém apertar nada:

```
<frame> <BOTOES|-> [lx ly rx ry]     estado do pad a partir desse frame
<frame> shot [nome]                  captura o frame do guest
<frame> quit                         encerra
```

## GS: hoje e plano

Fluxo atual: o jogo escreve pacotes GIF (por DMA) → `gs_frontend.cpp` decodifica registradores e
monta primitivas → `GSRasterBackend::Submit(GSPrimitiveBatch)` → `GSCpuBackend` rasteriza na VRAM
de 4 MB emulada → `Present` entrega os pixels ao host. A janela, a textura de apresentação e a
entrada do host são da **raylib** (GLFW + OpenGL no Windows); `Present` roda na thread da janela,
o desenho na `GameThread`.

`GSRasterBackend` (`include/runtime/gs/gs_backend.h`) é a fronteira para trocar o renderer:
recebe primitivas já montadas com o estado completo (`GSDrawState`: TEX0, ALPHA, TEST, ZBUF,
FRAME…), CLUT, transferências de imagem, `Flush/TextureFlush/Sync`, `Present` e leitura/escrita
da VRAM.

Medição que motiva o plano: 98,7% do tempo da thread do jogo está no rasterizador por software
(`DrawSprite` e `DrawTriangle`); o código recompilado pesa menos de 0,5%. O plano é um
`GSGpuBackend` ao lado do `GSCpuBackend`, escolhido por variável de ambiente, sobre uma camada
fina de dispositivo (RHI) com shaders HLSL únicos: Vulkan (ou D3D11) primeiro, D3D12 depois, e
fallback para software nos casos raros. A saída do backend por CPU é a **referência** para
comparar. O roteiro detalhado está em `gs-gpu-plan.md` (a partir da leitura do PCSX2, que é
GPL-3.0, então só como referência de comportamento).

## VU1 e MPEG/IPU

O VU1 tem interpretador próprio (`src/lib/vu/`). O IPU/MPEG (vídeos) tem stubs em
`Kernel/Stubs/{IPU,MPEG}.cpp`; a reprodução real de FMV ainda não foi tratada.

## Áudio

`ps2_audio.cpp` e `Kernel/Stubs/Audio.cpp` existem no runtime; o caminho de som do jogo
(motor de som sobre o SPU2) **não foi testado** neste projeto. `[PREENCHER]` após o primeiro
teste.

## Memory card

`Kernel/Stubs/MemoryCard.cpp` existe; **não testado**. Pastas `mc0`/`mc1` do jogo ficam fora do
git (saves).

## Patches e overrides do jogo

Nada é editado direto em `third_party/PS2Recomp`: as modificações vivem em
`<jogo>/patches/ps2xRuntime-modified.patch` (aplica com
`git apply --directory=third_party/PS2Recomp`) e os arquivos novos em `<jogo>/patches/ps2xRuntime/`.
Remendos por endereço (retornar 0/1 numa função do SDK, por exemplo) ficam num único arquivo de
overrides do jogo, registrado com `PS2_REGISTER_GAME_OVERRIDE` e compilado no executável (não na
biblioteca estática, onde o linker o descartaria). Editar o `.toml` para o mesmo fim custa um
rebuild completo.

## Diagnóstico e logs

Todo programa grava log em `<jogo>/logs/` (regra 6). O runtime tem: heartbeat (`[hb]`, IRQ, pad,
leitura de memória), contador de FPS e relógio do guest, criador de capturas de frame, log de
alvos ausentes e um **profiler de amostragem** (`ps2_sampling_profiler`, resolvido com
`tools/resolve_profile.py` contra o `.map` do linker). O `launcher run` expõe isso por
`--heartbeat --peek --fps-log --shot-dir --missing-log --pad-script`.

## Regressão

A verificação de fidelidade é por **imagem**: um script de pad leva o jogo a uma tela, o runtime
salva o frame do guest e comparamos o hash com uma referência guardada fora do git
(`<jogo>/work/reference/`). Toda otimização de GS tem que manter a saída idêntica bit a bit.

## Termos

- **EE**: Emotion Engine, a CPU principal (MIPS R5900).
- **IOP**: processador de I/O (MIPS R3000) que roda os módulos IRX; aqui é emulado por nome.
- **GS**: Graphics Synthesizer, a GPU do PS2 (VRAM de 4 MB).
- **GIF**: interface que leva pacotes de desenho do EE/VU1 ao GS.
- **VU0/VU1**: unidades vetoriais; a VU0 tem modo macro (COP2), a VU1 roda microprogramas.
- **SIF/RPC**: ponte EE↔IOP.
- **Stub**: função do SDK emulada em C++ pelo runtime.
- **Override**: substituição, por endereço, de uma função do jogo.
- **Recompilação estática**: traduzir o ELF para C++ antes de rodar, sem interpretar.
- **Latch do DMAC**: guarda de um aviso de DMA que chegou antes do handler.
- **PCH / unity build**: cabeçalho pré-compilado e compilação em bloco do runtime.
