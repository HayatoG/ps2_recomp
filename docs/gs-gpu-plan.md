# Plano: backend GS por GPU

Status: **etapa 1 escrita e testada isoladamente; não integrada ao runtime** (ver [Status](#status)).
Fonte do estudo: leitura dos headers e de partes do
renderer por hardware do PCSX2 (branch master, `pcsx2/GS/`) por um agente, mais o nosso
`include/runtime/gs/gs_backend.h`. O PCSX2 é **GPL-3.0**: usamos só como referência de
comportamento, escrevendo o nosso código com as nossas palavras (ver `LEGAL.md`). Itens marcados
"não confirmado" não foram verificados nas fontes.

## Por que
Medição no `ghost_pc`: 98,7% do tempo da thread do jogo está no rasterizador por software
(`DrawSprite` ~54% e `DrawTriangle` ~41% na abertura); o código recompilado pesa <0,5%. A
abertura roda a ~46% da velocidade real. Otimizar a CPU rende poucos por cento por passo.

## O que o PCSX2 faz (resumo)
- **Camadas:** `GSState` (GIF, registradores, memória local, fila de vértices) → `GSRenderer`
  (vsync, mescla dos dois circuitos de saída) → `GSRendererHW` (traduz estado do GS numa
  configuração de draw) → `GSDevice` (interface abstrata da API) com backends DX11, DX12, Vulkan,
  OpenGL e Metal; `GSTexture` é a textura abstrata.
- **Interface mínima do device:** criar superfície, copiar retângulo, blit com shader, apresentar,
  readback, converter paleta/índices e o draw principal (`RenderHW`). Pós-processamento (FXAA,
  CAS, shade boost) pode ser vazio no começo.
- **Um shader só para o desenho ("tfx")**, parametrizado por seletores que viram permutações
  compiladas sob demanda e guardadas em cache. O próprio código avisa do risco de
  "combinações demais".
- **Sprites** têm classe própria (2 vértices) e são expandidos para quad; pontos e linhas também.
- **A VRAM de 4 MB e o swizzle ficam na CPU.** Na GPU existem *alvos* (texturas lineares) ligados
  a um endereço (TBP0), largura e formato, e um *cache de texturas/alvos* que sincroniza as duas
  pontas: dados sujos da CPU sobem para o alvo; leitura pela CPU (transferência local→host,
  display de alvo sujo) faz readback.
- **Blend do PS2** `((A-B)*C>>7)+D` só mapeia no blend fixo da GPU em alguns casos; o resto é
  feito no shader, lendo o alvo por barreira, framebuffer fetch ou ROV. Existe um nível de
  "precisão de blend" ajustável.
- **Casos difíceis:** render-to-texture (alvo usado como fonte), CLUT/paletas, Z de 16/24/32 bits
  e depth como cor, channel/texture shuffle, DATE (teste de alpha de destino), AFAIL com segundo
  passe, colclip (wrap 0–255), hacks por jogo.
- **Fallback híbrido:** quando a GPU não cobre um caso, renderiza em software sobre a memória
  local e invalida o cache. Também mantêm um renderer de software completo como alternativa.

## Mapeamento para o nosso runtime
O ponto de encaixe já existe: `GSRasterBackend` recebe `Submit(GSPrimitiveBatch)` com o estado
completo, `LoadClut`, `BeginTransfer/UploadImage`, `Flush/TextureFlush/Sync`, `Present`,
`ReadVram/WriteVram/SnapshotVram`. Isso corresponde, em escala menor, ao `GSDevice` +
`GSRendererHW` do PCSX2. O `Initialize` já entrega o ponteiro da VRAM emulada, que é exatamente a
"memória local na CPU" do desenho acima.

## Desenho proposto
1. `GSGpuBackend : GSRasterBackend`, escolhido por variável de ambiente (`PS2X_GS=cpu|d3d11`, padrão `cpu`;
   depois `vulkan`/`d3d12`).
   `GSCpuBackend` continua como **referência** e fallback.
2. **RHI fina** (camada de dispositivo): textura, buffer, pipeline, draw, cópia, readback,
   apresentar. Shaders em **HLSL** uma vez só; D3D usa direto, Vulkan recebe SPIR-V (DXC).
3. Ordem das APIs (decidida pelo usuário): **D3D11 no Windows primeiro**, depois **Vulkan** e **D3D12**.
   D3D11 dá menos código e basta para este GS; Vulkan traz Linux/Steam Deck; D3D12 entra quando a RHI
   estiver estável. ("D3D13" não existe; a versão atual é D3D12.)

## Apresentação hoje (raylib) e o encaixe do D3D11
Verificado no código (`ps2xRuntime/src/lib/ps2_runtime.cpp`, `gs_frontend.cpp`):
- `PS2Runtime::run()` roda o jogo numa `GameThread` e mantém o **laço de janela na thread
  principal**: `InitWindow` (raylib), uma `Texture2D` de 640x`FB_HEIGHT`, e a cada volta
  `UploadFrame` → `GS::latchHostPresentationFrame()` → `m_backend->Flush/Sync/Present(request)`
  → `UpdateTexture` → `DrawTexturePro` com letterbox → `EndDrawing`.
- Portanto **`GSRasterBackend::Present` roda na thread principal** (a da janela), enquanto
  `Submit`, `UploadImage` etc. rodam na `GameThread`. O backend é criado no construtor de `GS`
  (`std::make_unique<GSCpuBackend>()`), sem ponto de escolha hoje.
- `PresentationFrame::pixels` é RGBA8 em ordem de bytes R,G,B,A (= `DXGI_FORMAT_R8G8B8A8_UNORM`).

Como o D3D11 se encaixa:
- **Etapa 1 (feito):** janela Win32 **própria** do backend com swapchain flip-model, criada de
  forma preguiçosa no primeiro `Present` (portanto na thread principal; o `glfwPollEvents` da raylib
  também despacha as mensagens dela, e o backend ainda chama `PeekMessage` só da sua janela). A
  janela raylib continua idêntica. Misturar GL e DXGI no **mesmo** HWND não é confiável; por isso
  janela separada.
- **Depois (decisão pendente):** (a) trocar a apresentação da raylib pelo D3D11 no Windows (a raylib
  só fica para entrada/ImGui, ou sai e entra Win32/XInput); (b) manter a raylib e compartilhar a
  textura D3D11 com o GL por `WGL_NV_DX_interop2` (funciona em NVIDIA/AMD/Intel modernos, mas é
  mais uma peça frágil); (c) readback para a textura raylib (simples, mas devolve parte do custo à
  CPU). Inclinação: (a), porque Vulkan/D3D12 virão depois e a raylib/GL não ajuda nelas.
- **Threading (risco da etapa 2):** o contexto imediato do D3D11 não é thread-safe. Quando o desenho
  for para a GPU, `Submit` (GameThread) e `Present` (thread principal) vão usar o mesmo device.
  Opções: gravar comandos numa fila na GameThread e executar tudo na thread principal; ou mover a
  apresentação para a GameThread; ou `ID3D11Multithread::SetMultithreadProtected` (mais simples,
  risco de contenção). Decidir no início da etapa 2.

## Roteiro por etapas (baixo risco)
Cada etapa fecha com a comparação de imagem contra o `GSCpuBackend` (mesmo script de pad).
1. Janela + `Present` do framebuffer da CPU enviado como textura (prova a RHI sem desenhar nada).
2. Alvo RGBA8 por (FBP, FBW, PSM); triângulos Gouraud sem textura; sprites como 2 triângulos.
3. Textura PSMCT32/16 por upload CPU→GPU com cache por hash de TEX0; modulate/decal; wrap/clamp.
4. CLUT de 8/4 bits (converter na CPU ou textura de paleta).
5. Alpha test simples e depth test (Z de 32 bits); FBMSK por colormask.
6. Blend fixo quando o caso é exato.
7. **Coerência:** qualquer leitura/transferência pela CPU ou display de alvo sujo faz readback.
   Esta etapa decide se a imagem fica correta; fazer antes de otimizar.
8. Depois: render-to-texture direto, AFAIL/segundo passe, DATE, fog, Z de 16/24, blend em shader
   com barreira, colclip, texture shuffle, dither.
9. Etapa final: métricas (FPS em abertura, menu e gameplay), fallback para software nos casos
   raros e tabela de casos não suportados.

## Status
**Etapa 1 — código pronto, testado fora do runtime; integração pendente** (03/11/2026).

| Arquivo (em `ghost_pc/patches/ps2xRuntime/`) | O que é |
|---|---|
| `include/runtime/gs/gs_rhi.h` | RHI fina: `Device` (textura RGBA8, upload, readback, blit letterbox + present, `WaitIdle`, `PumpEvents`), `AdapterInfo`, `DeviceConfig` |
| `src/lib/gs/gs_rhi_d3d11.cpp` | Implementação D3D11: janela Win32 própria, swapchain `FLIP_DISCARD` (2 buffers), VS de triângulo de tela cheia + PS de amostragem em HLSL embutido, compilados com `D3DCompile`; só `d3d11`/`dxgi`/`d3dcompiler` |
| `include/runtime/gs/gs_gpu_backend.h`, `src/lib/gs/gs_gpu_backend.cpp` | `GSGpuBackend : GSRasterBackend`: compõe um `GSCpuBackend` e delega **tudo**; no `Present` devolve o mesmo `PresentationFrame` e também o mostra pela RHI. `CreateGSRasterBackendFromEnv()` lê `PS2X_GS=cpu\|d3d11` (padrão `cpu`) |
| `../gs_gpu_integration.txt` | Trecho de CMake e a troca no `gs_frontend.cpp` para ligar o backend (não aplicado) |

Variáveis: `PS2X_GS=d3d11` (liga), `PS2X_GS_VSYNC=1`, `PS2X_GS_DEBUG=1` (camada de debug do
D3D11, cai para sem debug se não estiver instalada), `PS2X_GS_FILTER=linear` (padrão `point`,
igual à raylib). Logs no stdout com prefixo `[gs-gpu]`/`[gs-rhi]`: adaptador e feature level na
criação, média/máximo do tempo de apresentação a cada 10 s e no encerramento. Falha de device
desliga só a apresentação por GPU (a saída por CPU continua).

**Testado de verdade** (programa isolado `gs_rhi_spike`, que compila o mesmo `gs_rhi_d3d11.cpp`;
textura procedural, nada do jogo; log em `ghost_pc/logs/gs_rhi_spike.log`):
- MSVC 2022 (BuildTools), `/std:c++20 /W4 /permissive-`: sem avisos.
- Adaptador **NVIDIA GeForce RTX 2060**, feature level **11_1**, device criado em ~232 ms.
- Upload → readback de 640x448 RGBA8 **idêntico byte a byte** (antes e depois de 16 mil presents).
- Janela capturada: gradiente na orientação certa (R para a direita, G para baixo).
- Tempo por quadro, 640x448, vsync desligado, 5 s, ~3.200 quadros/s:
  - upload (`Map`+`memcpy`+`Unmap`): média **0,050 ms**, p99 0,074 ms;
  - upload + blit + `Present` (CPU, sem esperar a GPU): média **0,084 ms**, p99 0,114 ms, máx 1,4 ms;
  - upload + blit + `Present` + espera da GPU: média **0,232 ms**, p99 0,280 ms.
  Ou seja, apresentar pela GPU custa <0,3 ms/quadro; não é o gargalo (a CPU gasta dezenas de ms
  rasterizando).
- `gs_gpu_backend.cpp` **só compila** (`cl /c` contra os headers da árvore com patch); não foi
  linkado nem executado dentro do runtime.

**Não testado:** build do `ps2_runtime`/`ps2EntryRunner` com os arquivos novos; rodar o jogo com
`PS2X_GS=d3d11`; MD5 do `shot` igual entre `cpu` e `d3d11` (esperado igual, pois o frame é
devolvido sem alteração); convivência com a janela raylib (foco, mensagens); destruição do device
fora da thread principal (o `DestroyWindow` falharia sem travar e a janela some com o processo);
vsync ligado; redimensionar/minimizar a janela com o jogo rodando; outros adaptadores (AMD/Intel/WARP).

## Riscos
- **Readback** é o maior custo; manter dirty rects e só ler quando necessário.
- **Barreiras** de blend em shader são lentas sem suporte do hardware.
- **Precisão:** regra de borda (top-left), meio-pixel, arredondamento de cor/UV, alpha de destino.
- **Permutações de shader:** manter poucas no início; cache de pipeline em disco (`logs/` ou
  pasta de cache ignorada pelo git).
- **Fidelidade:** a GPU não será bit-idêntica à CPU em todos os casos. A regressão passa a ser
  "diferença visual pequena e medida" (por exemplo, erro máximo por canal) e não mais MD5; manter
  o MD5 só para o backend de CPU.

## Perguntas em aberto
- ~~Vulkan ou D3D11 primeiro?~~ **Decidido pelo usuário:** D3D11 no Windows primeiro; depois Vulkan e D3D12 (os três são objetivo). No Deck, D3D11 via Proton/DXVK serve de caminho curto (ver `distribution.md`).
- ~~Qual biblioteca de janela/entrada o runtime já usa para apresentar?~~ **Respondido:** raylib
  5.5 (via `FetchContent` no `CMakeLists.txt`; no Windows, GLFW + OpenGL), com rlImGui/ImGui para
  o painel de debug; o pad do host também é lido pela raylib (`Kernel/Stubs/Pad.cpp`). Detalhes e
  o encaixe do D3D11 em [Apresentação hoje](#apresentação-hoje-raylib-e-o-encaixe-do-d3d11).
- Quanto do que o jogo usa cai nos "casos difíceis"? medir com um contador de features por draw
  no `GSCpuBackend` antes de investir (barato e dá os dados para priorizar).
