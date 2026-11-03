# Design: GS por GPU, etapa 2 (desenhar na GPU sem perder a coerência com a VRAM)

Status: **proposta de design, nada implementado** (03/11/2026). Continua `gs-gpu-plan.md` (etapa 1:
RHI D3D11 que só apresenta o frame da CPU). Tudo que aqui é afirmado sobre o código foi lido em
`ghost_pc/work/ps2recomp_game/ps2xRuntime/` (cópia com os patches aplicados); o que **não** foi
confirmado está marcado **[não confirmado]** e listado na seção 9. O PCSX2 (GPL-3.0) entra só como
referência de comportamento, pelo resumo do `gs-gpu-plan.md`; nenhum código dele é usado.

Resumo das decisões:

| Pergunta | Recomendação |
|---|---|
| Threads | **(a) thread GS dedicada** que possui o device D3D11, a janela D3D, a VRAM da CPU e o rasterizador de fallback; a `GameThread` só enfileira comandos |
| Fonte da verdade | **Por página de VRAM (8 KB)**: dona = CPU ou um alvo GPU; escrita/leitura cruzada força upload ou write-back só das páginas afetadas |
| Primeira fase | Sprites sem textura **com Z16S** (94% dos draws escrevem Z: sem Z na GPU quase nada sai da CPU) |
| Z16S | `D32_FLOAT_S8X24_UINT`, valor `z/65536` (exato para inteiros de 16 bits), `GREATER_EQUAL`/`GREATER`, ZMSK = write mask zero |
| CLUT | **Não** converter textura indexada por paleta: índices em `R8_UINT` + espelho do buffer de CLUT (512 x 16 bits) na GPU, lookup no shader |
| Verificação | **Modo diferencial por draw** (CPU continua verdade; GPU repete o draw e compara a bbox) + diff de PNG em telas estáticas |
| RHI | Acrescentar formatos, alvos de cor/profundidade, buffers dinâmicos, pipelines por chave, cópias/readback por retângulo, fences; regras do GS ficam fora |

---

## 0. O que o código mostra hoje (base para as decisões)

Verificado:

- `GS` (`gs_frontend.cpp`) monta **uma primitiva por `Submit`** (`GSPrimitiveBatch`: até 3 vértices +
  `GSDrawState` completo) em `vertexKick`, sob `m_stateMutex`. ~450–500 `Submit` por present
  (medição do contador).
- `GSCpuBackend` serializa tudo num `m_mutex`. `Flush()` é vazio e `Sync()` só conta estatística
  ("GPU backends may wait on fences/readbacks here").
- `Present` roda na **thread da janela** (raylib): `UploadFrame` → `GS::latchHostPresentationFrame`
  → `Flush` + `Sync(Presentation)` + `Present(request)`. O `GSCpuBackend::Present` copia os **4 MB
  inteiros** da VRAM (`SnapshotVram`) e compõe os dois circuitos (PMODE, BGCOLOR, campo) na CPU,
  com duas heurísticas que **leem a VRAM**: fonte preferida (`updatePreferredDisplaySourceForDraw`,
  sprite de cópia de tela cheia) e busca do primeiro frame "não preto" em `contextFrames`.
- Ninguém fora do backend lê a VRAM diretamente: `GS::ReadVram` não tem chamador externo;
  `getGSVRAM` só é usado para `m_gs.init`; `gs_vram_offset` em `ps2_memory.cpp` não é chamado.
  Leituras de VRAM que saem do GS: `consumeLocalToHostBytes` (stub `sceGsExecStoreImage`),
  `snapshotVRAM` (debug), `setRasterBackend` (troca a quente) e o `Present`.
- `GetTransferSnapshot` é chamado pelo frontend só para debug (histórico pausado por padrão).
- O rasterizador da CPU (a **referência**) tem particularidades que a GPU precisa imitar ou tolerar:
  - XY em ponto flutuante já dividido por 16; `XYOFFSET` aplicado como `ofx >> 4` (inteiro).
  - Sprite: cantos truncados para inteiro, cobre `[x0, x0+max(1,x1-x0)-1]` (largura zero desenha 1
    coluna); cor, Z e fog do **v1**; UV interpolado linearmente com amostra no centro do pixel e,
    com `FST`, quantizado a 1/16 (`(u*16)+0.5`).
  - Triângulo: teste no centro do pixel com tolerância `1e-4` **nas três arestas** (sem regra
    top-left: aresta compartilhada pode ser desenhada duas vezes), cor Gouraud truncada, Z
    interpolado em `double` e arredondado (`z+0.5`), STQ interpolado e dividido depois.
  - Ordem por pixel: scissor → fog → alpha test (AFAIL) → DATE → Z test → blend → FBA → FBMSK → escrita.
  - Blend `((A-B)*C >> 7) + D` em inteiros, **sempre com clamp** (COLCLAMP ignorado). AA1, dither,
    SCANMSK e o bit ZTE também são ignorados (nenhuma referência a eles em `gs_cpu_backend.cpp`).
  - Z16S: compara o **z completo** (até 32 bits) com o valor de 16 bits guardado e grava os 16 bits
    baixos (truncamento). O endereço do Z usa o `FBW` do FRAME.
  - CT24: leitura do destino força alpha `0x80`; escrita preserva o byte alto guardado.
  - Leitura de textura passa pelo `TexturePageCache` (uma página de 8 KB "congelada" até troca de
    página ou `TEXFLUSH`), que imita o cache de textura do GS.

Medições que pesam no design (`gs-gpu-plan.md`, menu/título/abertura): sprites 52–89% dos draws;
triângulos 37–60% dos pixels; **Z16S com escrita em ~93% dos draws**; CT32 95% / CT24 5%;
render-to-texture 5–7% dos draws mas **20–29% dos pixels**; CLUT recarregada em até 86% dos draws
indexados; blend "outro" até 38% dos draws; **0 leituras local→host**. Não estão no documento
(estão no log, que não está no repo): taxas de DATE, fog, ATST, AFAIL, FBMSK, PABE, filtro e wrap.

---

## 1. Modelo de threads

### Opções

| | Como funciona | Prós | Contras |
|---|---|---|---|
| **(a) Thread GS dedicada** | `GameThread` grava comandos numa fila; uma thread "GS" é a única que toca D3D11, VRAM da CPU e fallback; `Present` vira pedido à thread GS | Um dono só do contexto imediato; tira **toda** a rasterização (inclusive o fallback de CPU) da `GameThread`; mesmo modelo de Vulkan/D3D12 (uma thread de submissão); o PCSX2 usa esse modelo (thread de GS apresentando numa janela de outra thread) | Cópia dos dados de upload para a fila; pontos de sincronização explícitos; backpressure |
| (b) Present na `GameThread` | Device criado e usado só na `GameThread`; a janela da raylib fica na principal | Sem fila; menos código | A `GameThread` paga o custo da API, das esperas de readback e do `Present`/vsync; não há ponto natural de present na `GameThread` (hoje ele é puxado pela janela); não ganha paralelismo |
| (c) `ID3D11Multithread` / deferred contexts | Lock interno do D3D11 (`SetMultithreadProtected`) ou gravar command lists na `GameThread` | Mudança pequena | O lock não protege o **estado ligado** (o blit da janela entre dois draws bagunça o pipeline do jogo); deferred contexts não fazem readback (`Map` de leitura só no imediato) e vários drivers os emulam em software; não se traduz para Vulkan/D3D12 |

**Escolha: (a).** (b) fica como plano B se a fila se mostrar instável. (c) descartada.

### Desenho

```
GameThread (produtor, sob GS::m_stateMutex)        thread principal (raylib)
  Submit/LoadClut/BeginTransfer/UploadImage/...      UploadFrame -> latchHostPresentationFrame
        |  push (cópia do comando)                          |  Present(request)
        v                                                    v
  GSCommandRing (bounded, ex.: 8 MB)          slot "pedido de present" (só o mais recente)
        \______________________  ______________________/
                               \/
                        thread "GS" (consumidor)
   dona de: gs_rhi::Device + janela D3D, GSTargetCache, GSTextureCache, espelho de CLUT,
            VRAM da CPU (ponteiro do Initialize) e o GSCpuBackend de fallback
```

- **Comandos assíncronos** (só enfileiram): `Submit` (o batch inteiro, ~350 bytes), `LoadClut`,
  `BeginTransfer` (host→local e local→local), `UploadImage` (**copia** `data` para a fila: o ponteiro
  é memória do guest), `TextureFlush`, `ClearFramebuffer`, `WriteVram`, `Reset`.
  - `ClearFramebuffer` retorna `bool` sem esperar: o resultado só depende de `fbw != 0` e do PSM
    (mesma regra do `GSCpuBackend`), calculável no produtor.
  - `GetTransferSnapshot` é respondido no produtor por um espelho simples (total/copiados/direção,
    avançado por `UploadImage` pelos bytes e bpp). Só é usado no debug.
- **Comandos síncronos** (enfileiram e esperam uma promessa): `ConsumeLocalToHostBytes` (a thread
  GS faz write-back das páginas de origem, roda a transferência e devolve os bytes),
  `ReadVram`, `SnapshotVram` (drena, write-back de tudo, copia), `Sync(Reset)`.
- `Flush()`: só acorda a thread GS. `Sync(Finish)`: **não espera** (o frontend já liga o bit FINISH
  no CSR; o jogo não observa o momento em que a GPU termina). `Sync(LocalToHost)`/`DebugReadback`:
  drenam a fila. `Sync(Presentation)`: não faz nada (o `Present` já se ordena).
- **Present**: a thread da janela grava o `GSPresentationRequest` num slot (sobrescreve o anterior)
  e espera o `PresentationFrame` com timeout (ex.: 50 ms; ao estourar, devolve o último frame pronto
  e conta o atraso no log). A thread GS atende o slot **entre dois comandos**, o que equivale à
  semântica atual (o `GSCpuBackend::Present` também tira a foto da VRAM num ponto arbitrário entre
  primitivas, sob `m_mutex`).
- **Backpressure**: fila limitada em bytes; produtor bloqueia quando cheia. Sem isso a `GameThread`
  corre quadros à frente e a latência de entrada cresce. Começar com ~1 quadro de folga e medir.
- **Mensagens da janela D3D**: a janela é criada e destruída **pela thread GS** (resolve o problema
  "destruição fora da thread dona" da etapa 1). Quando ociosa, a thread GS espera com
  `MsgWaitForMultipleObjects` (evento da fila + mensagens), para a janela não ficar "Não respondendo".
- **Sem deadlock**: a thread GS nunca toma `m_stateMutex`, `m_backendLifetimeMutex` nem
  `m_presentationMutex`. Quem espera por ela (`GameThread` em `consumeLocalToHostBytes`, janela em
  `Present`, `setRasterBackend`) pode estar segurando esses mutexes sem risco.
- **Encerramento**: o destrutor do `GSGpuBackend` envia `Quit`, junta a thread; device, janela e
  swapchain morrem nela.

### E a janela raylib?

- Etapas E0–E7 (seção 7): a raylib continua a janela principal e recebe o `PresentationFrame`
  produzido por write-back (é o que vira `shot`/PNG e MD5). A janela D3D mostra o alvo da GPU direto,
  opcional (`PS2X_GS_WINDOW=0` a esconde). Duas janelas são aceitáveis enquanto se prova correção.
- Etapa E8: apresentar pela GPU sem readback. Inclinação (já registrada no plano): a janela D3D
  passa a ser a principal e a raylib sai da apresentação. **Pergunta em aberto**: entrada. O pad vem
  da raylib (`Kernel/Stubs/Pad.cpp`); o gamepad via GLFW deve funcionar sem foco, o teclado não
  **[não confirmado]**. Alternativa: XInput/Win32 direto.

---

## 2. Coerência com a VRAM da CPU

### Princípio

A VRAM de 4 MB no host continua sendo o **formato de troca** (o `setRasterBackend` já a trata assim)
e o lugar do swizzle. A GPU guarda **alvos lineares** que são um cache de partes dela. Quem tem o dado
mais novo é decidido **por página de 8 KB** (512 páginas; mesma granularidade do `TexturePageCache`
e da página do GS: 64x32 em 32 bits, 64x64 em 16 bits).

Estruturas (todas na thread GS, fora da RHI):

- `GSTarget`: chave `(base em blocos, FBW, PSM)`, tipo cor ou profundidade, textura RHI, retângulo
  válido, e por página: `gpuValid` (o alvo tem o conteúdo atual) e `gpuDirty` (a VRAM está velha).
- `pageOwner[512]`: `CPU` ou o id do alvo que tem a versão mais nova.
- `pageVersion[512]`: contador incrementado a cada escrita **na VRAM da CPU** (upload, transferência,
  draw de fallback, write-back). Usado pelo cache de texturas para saber se precisa reconverter.

### Invariantes

1. Para cada página, **no máximo um** dono tem a versão mais nova; se o dono é um alvo, a VRAM da
   CPU daquela página está velha (`gpuDirty`).
2. **Nenhuma leitura de VRAM pela CPU** (fallback de draw, `LoadClut`, conversão de textura, local→host,
   origem de local→local, `Present` por CPU, `SnapshotVram`, `ReadVram`) acontece sem antes fazer
   write-back das páginas lidas cujo dono é um alvo.
3. **Nenhuma escrita de VRAM pela CPU** em página com dono GPU acontece sem write-back prévio, **a
   menos que** a escrita cubra a página inteira (upload de imagem grande): aí só se descarta.
   Depois da escrita, dono = CPU e os alvos que cobrem a página ficam `gpuValid = false`.
4. **Nenhum draw na GPU** lê/escreve uma página de alvo com `gpuValid = false` sem upload prévio
   (VRAM → alvo, convertendo o swizzle na CPU).
5. Um alvo que sobrepõe outro com chave diferente (mesma memória vista como outro PSM/largura, ou cor
   sobre Z) é tratado como escrita cruzada: write-back do antigo nas páginas comuns e invalidação.
6. Toda espera da CPU pela GPU é contada no log (`[gs-gpu] sync reason=... pages=... ms=...`).

### Casos

| Evento | O que acontece | Custo |
|---|---|---|
| Draw GPU em alvo cuja região é da CPU (ex.: imagem enviada por upload, tela de loading) | upload das páginas da bbox ∩ alvo | ~0,05 ms por 640x448 (medido na etapa 1) + conversão do swizzle na CPU |
| Draw GPU normal | marca as páginas da bbox: dono = alvo, `gpuDirty` | nenhum extra |
| Draw que cai no **fallback de CPU** com alvo na GPU | write-back das páginas da bbox (cor e Z) → `GSCpuBackend` desenha → dono = CPU; o próximo draw GPU refaz o upload | 1 readback com espera (~0,3–1 ms) + 1 upload; **caro se alternar** |
| Upload host→local | invariante 3; se a página era alvo, alvo fica inválido nela | barato se cobre páginas inteiras |
| Local→local | se origem e destino são alvos compatíveis: cópia na GPU; senão write-back da origem, cópia na CPU, destino = CPU | raro **[não medido]** |
| Local→host (0 no jogo) | write-back das páginas de origem, espera | uma espera |
| Textura lida de um alvo (render-to-texture) | mesmo PSM/largura compatível: amostra o alvo direto (cópia para textura temporária se for o alvo ligado); senão write-back e conversão | 20–29% dos pixels: **precisa** do caminho direto (E6) |
| `LoadClut` com CBP dentro de alvo | write-back das páginas da CLUT (poucos bytes, mas é uma espera) | **[não medido]**: contar |
| `Present` | E0–E7: write-back do(s) alvo(s) de display (e dos `contextFrames` se a heurística de "não preto" precisar) e composição pela CPU atual; E8: composição na GPU | ~1,2 MB por quadro na fase inicial |

### Evitar o pingue-pongue

O pior caso é draw GPU, draw CPU, draw GPU no mesmo alvo: cada troca é uma espera. Regras:

- **Modo por alvo e por quadro**: se um alvo recebeu um draw de fallback, ele fica em modo CPU até o
  próximo `Present` (os draws GPU seguintes nele também vão para a CPU). Troca de modo custa uma
  sincronização por alvo por quadro, no máximo. Contar trocas no log.
- Consequência honesta: **nas primeiras fases a GPU não deixa o jogo mais rápido**; com cobertura
  parcial os alvos passam a maior parte do quadro em modo CPU. Ganho de velocidade só quando a
  cobertura de pixels passar de ~95% por alvo (fase E5/E6). As fases iniciais servem para provar
  correção (seção 5).

### Fallback de GPU durante o desenvolvimento (o "vice-versa")

Se a GPU falhar (device perdido, shader não compila), o backend faz write-back de tudo o que puder,
marca dono = CPU em todas as páginas e passa a delegar 100% ao `GSCpuBackend` (como a etapa 1 já faz
com a apresentação). O jogo continua.

---

## 3. Política de fallback por draw

Uma função pura, `GSGpuDrawPolicy::Classify(const GSDrawState&, fase) -> {Gpu, Cpu}`, decide cada
draw. **Primeiro passo de implementação**: rodar essa mesma função dentro do contador
(`PS2X_GS_FEATURES=1`, no caminho de CPU) para imprimir "% de draws/pixels que iriam para a GPU em
cada fase". Isso prevê o ganho antes de escrever o shader.

Legenda: **G** = GPU; **G+** = GPU numa fase posterior; **C** = CPU (fallback). Fases na seção 7.

| Recurso (do contador) | E2 | E3 | E4 | E5 | E6 | E7 | Observação |
|---|---|---|---|---|---|---|---|
| Sprite sem textura | G | G | G | G | G | G | quad de 2 triângulos, atributos do v1 |
| Triângulo flat/Gouraud | C | G | G | G | G | G | regra de aresta difere (seção 5) |
| Linha / ponto | C | C | C | C | C | G+ | 0% nas medições; deixar na CPU |
| Textura CT32 / CT24 | C | C | G | G | G | G | upload convertido, cache por versão de página |
| Textura CT16/CT16S | C | C | G | G | G | G | `R16_UINT` + expansão com TEXA no shader |
| T8 / T4 (+T8H/T4HL/T4HH) com CLUT recarregada | C | C | C | G | G | G | índices + espelho de CLUT |
| Filtro bilinear | C | C | G | G | G | G | manual no shader (4 `Load`), para casar com a CPU |
| Mipmap | C | C | C | C | C | C | não medido como relevante |
| Wrap REPEAT/CLAMP | C | C | G | G | G | G | no shader (coordenada inteira) |
| REGION_CLAMP/REGION_REPEAT | C | C | G | G | G | G | idem; mesma função da CPU (`wrapTextureCoordinate`) |
| Blend off | G | G | G | G | G | G | |
| Blend simples `(Cs-Cd)*As+Cd`, As ≤ 0x80 | G | G | G | G | G | G | dual-source (fator `As/128` em `SV_Target1`) |
| Blend aditivo `(Cs-0)*C+Cd`, C = As ≤ 0x80 ou FIX | G | G | G | G | G | G | ONE/ONE com `Cs*C/128` premultiplicado |
| Blend com C = Ad, As > 0x80, ou "outro" sem mapeamento fixo | C | C | C | C | C | G+ | E7: tabela de mapeamento + blend no shader com cópia do destino |
| PABE | C | C | C | C | C | G+ | precisa de blend no shader |
| Z16S ALWAYS/GEQUAL/GREATER, ZMSK | G | G | G | G | G | G | **obrigatório desde a E2** (93–94% dos draws) |
| Z16S NEVER | G | G | G | G | G | G | draw descartado inteiro (a CPU retorna antes de escrever) |
| z > 0xFFFF em alvo Z16 | C | C | C | C | C | C | a CPU compara 32 bits e trunca a escrita; não dá em hardware de profundidade. Medir; se for 0, irrelevante |
| Z32/Z24/Z16 | C | C | C | C | C | G+ | 0–7% nas medições |
| Alpha test, AFAIL = KEEP | C | G | G | G | G | G | `discard` no shader |
| AFAIL ≠ KEEP (FB_ONLY, ZB_ONLY, RGB_ONLY) | C | C | C | C | C | G+ | 2 passes (passou / falhou com máscaras diferentes) |
| DATE | C | C | C | C | C | G+ | pré-passe que grava stencil a partir do alpha do destino |
| FBA | G | G | G | G | G | G | alpha OR 0x80 no shader |
| FBMSK em bytes inteiros (0x00/0xFF por canal) | G | G | G | G | G | G | `RenderTargetWriteMask` |
| FBMSK em bits arbitrários | C | C | C | C | C | G+ | blend no shader com cópia do destino |
| Frame CT32 | G | G | G | G | G | G | |
| Frame CT24 | G | G | G | G | G | G | write mask RGB; Ad = 0x80 |
| Frame CT16/CT16S | C | C | C | C | C | G+ | 0% nas medições |
| Fog | C | G | G | G | G | G | fórmula inteira da CPU no shader |
| AA1, dither, COLCLAMP=0, SCANMSK | G (ignorar) | … | … | … | … | … | a referência também ignora; se um dia a CPU passar a tratar, a GPU acompanha |
| Render-to-texture (mesmo PSM/largura) | C | C | C | C | G | G | alvo como SRV; cópia se for o alvo ligado |
| Render-to-texture com reinterpretação (shuffle, PSM diferente) | C | C | C | C | C | C | write-back + conversão; medir |
| `ClearFramebuffer` (stub de DB clear) CT32/CT24 | G | G | G | G | G | G | `ClearView` com retângulo do scissor (D3D11.1) ou quad |

Fase 1 mínima e verificável = **E2**: sprites sem textura, blend off/simples/aditivo com fator ≤
0x80, Z16S, FBA, FBMSK por byte, frame CT32/CT24, scissor. Todo o resto: CPU.

---

## 4. Formatos

### Profundidade (Z16S)

- **Formato**: `DXGI_FORMAT_D32_FLOAT_S8X24_UINT` (recurso `R32G8X24_TYPELESS`, SRV
  `R32_FLOAT_X8X24_TYPELESS` para leitura).
  - Valor gravado = `z / 65536.0`. Todo inteiro de 16 bits dividido por 2^16 é **exato** em float32,
    então ida e volta VRAM ↔ GPU é sem perda e a comparação de dois Z iguais dá igual.
  - Stencil de 8 bits já disponível para DATE (E7) sem trocar de formato depois.
  - Descartado `D16_UNORM`: menor, mas a conversão `z/65535` arredonda no hardware, não tem stencil e
    exigiria troca de formato na E7. Descartado `D24_UNORM_S8_UINT`: conversão também não é exata
    na ida e volta sem cuidado extra; ganho de memória irrelevante.
- **Comparação**: ZTST 0 → descarta o draw; 1 → `ALWAYS`; 2 → `GREATER_EQUAL`; 3 → `GREATER`.
  Bit ZTE ignorado (igual à CPU). `DepthClipEnable = FALSE`, viewport `MinDepth 0 / MaxDepth 1`.
- **Escrita**: ZMSK=1 → `DepthWriteMask ZERO`. AFAIL=FB_ONLY etc. em E7.
- **Arredondamento**: sprite tem Z constante (exato). Triângulo: o rasterizador interpola
  `SV_Position.z` linear em tela (como a CPU), mas sem o `round(z+0.5)` da CPU. Duas variantes do
  shader: padrão sem `SV_Depth` (mantém early-Z) e uma de verificação que escreve
  `SV_Depth = floor(z+0.5)/65536` (desliga early-Z; usada no modo diferencial). Se a diferença for
  relevante, usar `SV_DepthGreaterEqual` (profundidade conservadora, SM 5.0) como meio-termo.
- **Upload/write-back de Z**: não dá para `Map`/`UpdateSubresource` em depth-stencil. Upload =
  textura `R16_UINT` com os valores já des-swizzled na CPU + passe de tela que escreve `SV_Depth`.
  Write-back = passe que lê a SRV de profundidade e grava `R16_UINT` (ou `CopyResource` do recurso
  inteiro para staging) → swizzle Z16S na CPU.
- **Tamanho**: D3D11 exige RTV e DSV com as mesmas dimensões efetivas **[não confirmado no código;
  é a regra do runtime D3D11 que lembro — confirmar com a camada de debug]**. Por isso todos os alvos
  de cor e profundidade usam um tamanho canônico (proposta: largura `max(FBW*64, 640)` arredondada
  para 1024, altura 1024) e o viewport/scissor delimitam a área. `FBW > 16` → CPU até medir.

### Frame

- CT32 → `R8G8B8A8_UNORM` guardando os **bytes crus** (alpha 0x80 = 128/255, não 1.0). Toda a
  aritmética do GS no shader é feita em inteiros (`uint(round(c*255))`), o que permite repetir as
  fórmulas da CPU (`>>7`, truncamentos). `R8G8B8A8_UINT` foi descartado: não tem blend fixo.
- Blend fixo só mexe em RGB; alpha da saída = As (blend de alpha `ONE/ZERO`), FBA aplicado no shader.
- Fatores: As/128 via **dual-source blending** (`SRC1_ALPHA`), FIX/128 via `BlendFactor`. Em alvo
  UNORM os fatores são limitados a [0,1] **[não confirmado; regra da especificação a verificar]**,
  por isso As/FIX > 0x80 vão para a CPU até a E7.
- CT24 → mesma textura, `RenderTargetWriteMask = RGB`; Ad no blend = 0x80; write-back grava só os
  3 bytes (preserva o byte alto da VRAM, como `GSMem::WriteCT24`).
- CT16/CT16S → CPU até haver uso (0% medido). Plano: RGBA8 com quantização no shader; o blend fixo
  em 8 bits seguido de truncamento a 5 bits não é igual à CPU (que expande o destino 5551→8888 e
  converte de volta), então provavelmente blend no shader.

### Endereçamento e swizzle

Ficam **na CPU**, reaproveitando as funções de `GSMem` (`m_readVramFuncs`/`m_writeVramFuncs`, tabelas
de página C32/Z16S). Conversões necessárias, por retângulo: VRAM→linear (upload de alvo e textura) e
linear→VRAM (write-back). Primeiro versão por pixel (simples, ~286 mil chamadas por 640x448); depois
caminho rápido por página/bloco se o perfil pedir. A GPU nunca vê endereço de bloco, só `(x, y)`
dentro de um alvo.

### Texturas e CLUT

- CT32/CT24 → `R8G8B8A8_UNORM` (ou `R32_UINT`) linear com os bytes crus; alpha de CT24 resolvido no
  shader com TEXA (TA0/AEM), então mudar TEXA **não** invalida o cache.
- CT16/CT16S → `R16_UINT` crus, expansão e TEXA no shader.
- T8/T4/T8H/T4HL/T4HH → `R8_UINT` com o índice já extraído na conversão (T4 desempacotado; H/HL/HH
  tiram os bits altos da palavra de 32 bits).
- Cache de texturas: chave `(TBP0, TBW, PSM, TW, TH)`; válido enquanto as `pageVersion` das páginas
  cobertas não mudarem. Sem hash de conteúdo (mais barato; detecta toda escrita).
- **CLUT**: espelho do buffer de CLUT da CPU (`m_clut`, 512 entradas de 16 bits, mesma organização,
  incluindo a metade alta para paletas de 32 bits) num buffer `R16_UINT` de 1 KB na GPU,
  reenviado quando `LoadClut` muda o conteúdo (contador de versão). O shader reproduz `LookupCLUT`
  (CSA, CPSM, CSM1/CSM2) e depois TEXA. Custo por troca de paleta: 1 KB de upload.
  - Descartado: **cache de textura convertida por `(TBP0, CBP, CPSM, CSA, conteúdo da CLUT)`**.
    Com CLUT recarregada em 74–86% dos draws indexados, cada paleta nova reconverte a textura
    inteira na CPU; o espelho custa 1 KB por troca e o índice é convertido uma vez.
  - Descartado: textura de paleta por `(CBP, CPSM, CSA)`: a CLUT do GS é um buffer com estado
    (CLD 2–5 comparam CBP0/CBP1), espelhar o buffer é mais fiel que reconstruir paletas por chave.
- Amostragem: sempre `Load` com coordenada inteira calculada no shader com a **mesma** matemática da
  CPU (UV `FST` quantizado a 1/16, STQ dividido depois da interpolação, wrap por
  `wrapTextureCoordinate`). Bilinear feito à mão com 4 `Load`s. Nenhum sampler de hardware para
  texturas do GS (sampler só no blit de apresentação). Isso evita diferenças de regra de
  arredondamento entre fabricantes e é portátil.

---

## 5. Verificação

Bit a bit deixa de ser o critério do backend GPU (o MD5 continua para o `GSCpuBackend`). O critério
passa a ser **diferença medida e pequena**, em dois níveis.

### Nível 1: modo diferencial por draw (principal)

`PS2X_GS=d3d11 PS2X_GS_VERIFY=1`: a CPU continua a **fonte da verdade** e desenha tudo; a GPU mantém
um espelho. Para cada draw que a política manda para a GPU:

1. garante que o alvo GPU tem a bbox igual à VRAM (upload das páginas da CPU que mudaram);
2. executa o draw na GPU e o mesmo draw na CPU;
3. lê de volta a bbox da GPU (cor e Z) e compara com a VRAM;
4. grava uma linha JSON em `<jogo>/logs/gs_verify_<data>.jsonl`: chave de features (a mesma da
   política), bbox, pixels, `max_abs` e `mean_abs` por canal, `% pixels diferentes`, `% acima do
   limiar`, diferença de Z (máx./contagem).

Por que é o principal: o script de pad conta **quadros do host** (`ps2_virtual_pad::onFrame(hostFrame)`
em `ps2_runtime.cpp`) e o vblank vem do relógio do host; com a GPU o jogo anda em outra velocidade e
a cena animada da captura N não é a mesma. O diferencial isola cada draw e independe do tempo. É
lento (uma espera por draw), só para depuração. Resumo a cada 600 presents, agrupado por chave de
features (`[gs-verify] key=sprite|notex|blend=simple|z16s.ge  draws=… max=… mean=… pct=…`).

Opcional: `PS2X_GS_VERIFY_DUMP=N` salva as N primeiras bboxes reprovadas (antes/CPU/GPU) como PNG
em `logs/` (são imagens do jogo: ficam fora do git, regra 2).

### Nível 2: imagem do quadro inteiro

- Telas **estáticas**, onde o resultado não depende do tempo: menu de idioma
  (`pad_scripts/bench_menu.txt`, já com MD5 estável 3/3), título parado, telas de `explore2.txt`
  em que a imagem não anima (a verificar uma a uma comparando duas execuções do próprio backend de
  CPU: se diferem entre si, a tela não serve).
- A abertura hexagonal (animada) só serve para desempenho e para inspeção visual, não para limiar.
- Sugestão (fora deste design, mudança pequena no runtime): uma variante do script de pad indexada
  por `vsyncTick` do guest, que tornaria mais telas comparáveis. Mesmo assim o jogo pode não ser
  determinístico por vblank quando roda abaixo da velocidade real **[não confirmado]**.

### Ferramenta de diff de PNG

`tools/imgdiff.py` (genérica, na raiz; Python 3, `python -I`):

```
python -I ../tools/imgdiff.py A.png B.png [--crop x,y,w,h] [--threshold 8]
       [--max-mean 0.5] [--max-pct 0.5] [--search-offset 4] [--diff-out logs/diff.png]
       [--json logs/imgdiff_<data>.jsonl]
```

- Lê PNG RGBA (Pillow se existir; senão decodificador mínimo com `zlib`, só PNG 8 bits sem
  entrelaçamento, que é o que a raylib exporta **[não confirmado para todos os casos]**).
- Métricas por canal (R, G, B; alpha opcional): erro máximo, erro médio, % de pixels com qualquer
  diferença, % acima de `--threshold`, PSNR.
- `--diff-out`: mapa de calor (preto = igual) para ver onde está o erro (bordas de triângulo vs
  áreas inteiras).
- `--search-offset`: procura o deslocamento de até ±N px com menor erro (para comparar com o PCSX2,
  cujo recorte da tela pode diferir).
- Saída: uma linha legível no stdout + uma linha JSON; código de saída 1 se passar dos limites (para
  usar em script de regressão). Log com timestamp em `logs/` (regra 6).

### Limiares de aceitação (propostos; calibrar com a primeira rodada e registrar aqui)

| Etapa | Nível 1 (por draw, por chave) | Nível 2 (menu de idioma) |
|---|---|---|
| E0 thread GS com CPU | — | **MD5 idêntico** |
| E1 ida e volta VRAM→alvo→VRAM | **bit a bit** (CT32, CT24, Z16S) | MD5 idêntico (GPU não desenha) |
| E2 sprites sem textura | blend off: **0 diferença**; blend: máx. 1 por canal, ≤1% pixels; Z: 0 | média ≤ 0,1; ≤ 0,5% px acima de 8 |
| E3 triângulos | interior: máx. 1–2; pixels de aresta ≤ 2% da bbox (regra top-left vs. tolerância da CPU) | média ≤ 0,3; ≤ 1% px acima de 8 |
| E4 textura CT32 | ponto: 0 no interior; bilinear: máx. 2 | idem |
| E5 CLUT | ponto: **0** (busca inteira) | idem |
| E6 render-to-texture | máx. 2 | média ≤ 0,5; ≤ 1% px acima de 8 |

Toda diferença maior vira entrada na tabela de casos (seção 8) com a explicação, ou bug.

### PCSX2 como oráculo (quando o usuário tiver a BIOS)

O `GSCpuBackend` é a referência de **regressão**, não de **hardware**: ele tem desvios conhecidos
(sem regra top-left, Z truncado, COLCLAMP ignorado). O PCSX2 com renderer **por software**,
resolução nativa, sem filtro/desentrelaçamento, é o mais próximo do hardware que temos:

- capturar as mesmas telas estáticas no PCSX2 e no port (CPU e GPU) e rodar `imgdiff` com
  `--search-offset`;
- se GPU ≈ PCSX2 e CPU ≠ PCSX2 numa região, o "erro" da GPU é na verdade correção da referência;
  registrar e decidir caso a caso se a CPU deve mudar (o que quebra o MD5 de referência, decisão do
  usuário);
- só usar o PCSX2 como programa externo (screenshots); nada do código dele entra no projeto.

---

## 6. RHI (`gs_rhi.h`)

Acréscimos, todos expressos em conceitos que existem em D3D11, D3D12 e Vulkan:

| Item | Proposta |
|---|---|
| Formatos | `RGBA8`, `R8_UINT`, `R16_UINT`, `R32_UINT`, `R32_FLOAT`, `D32_FLOAT_S8` |
| `TextureDesc` | flags de uso: `ShaderResource`, `RenderTarget`, `DepthStencil`, `CpuUpload` (dinâmica); 1 mip, 1 camada |
| Buffers | `BufferDesc{size, usage: Vertex/Constant/Typed(format), cpuWritable}`; anel de vértices dinâmico (`Map` DISCARD/NO_OVERWRITE no D3D11; anel por quadro em Vulkan/D3D12) |
| Upload parcial | `UpdateTextureRegion(tex, rect, data, pitch)` |
| Readback | `CopyToReadback(tex, rect) -> ReadbackTicket` e `ReadReadback(ticket, dst, pitch, wait)` com anel de texturas de staging; a versão síncrona atual vira um atalho |
| Cópia | `CopyTextureRegion(dst, dstXY, src, srcRect)` (GPU→GPU, para render-to-texture e local→local) |
| Pipeline | `PipelineDesc{shaderKey, blend, depthStencil, colorWriteMask, dualSource}` criado por chave e guardado em cache; no D3D11 vira VS/PS + objetos de estado; em Vulkan/D3D12 vira PSO. Estado dinâmico só: viewport, scissor, blend factor, stencil ref |
| Blend | `BlendDesc{enable, srcColor, dstColor, op, srcAlpha, dstAlpha, opAlpha}` com fatores incluindo `Src1Alpha`, `BlendFactor` |
| Depth/stencil | `DepthStencilDesc{test, compare, write, stencil...}` |
| Passe | `BeginPass(color, depth, loadOps)` / `EndPass()`; em D3D11 é só `OMSetRenderTargets`; deixa o caminho aberto para render passes do Vulkan |
| Draw | `Draw(vertexCount, firstVertex)`; vértices em formato fixo do backend GS (posição, Z, cor, UV/STQ, fog) |
| Binding | `SetTexture(slot, tex)`, `SetBuffer(slot, buf)`, `SetConstants(slot, data, size)` (cbuffer por draw) |
| Limpeza | `ClearColor(tex, rect, rgba)`, `ClearDepth(tex, rect, value)` |
| Sincronização | `Fence` (query de evento no D3D11): saber se um readback está pronto sem bloquear; consultas de timestamp para medir GPU |
| Shaders | HLSL único com `#define` por permutação; D3D11 compila com `D3DCompile` e guarda cache em disco (pasta ignorada pelo git); Vulkan recebe SPIR-V via DXC depois |
| Debug | `PushMarker/PopMarker` (anotações para RenderDoc/PIX) |
| Janela | separar `Surface` (janela + swapchain) do `Device`, para no E8 apresentar numa janela vinda de fora e no Vulkan criar a surface da mesma forma |
| Erros | igual hoje: retorno + uma linha de log, sem exceções; `IsDeviceLost()` |

**Fica fora da RHI** (camada do backend GS, `gs_gpu_*.cpp`): VRAM, swizzle e conversões; cache de
alvos e de texturas; dono por página; espelho de CLUT; tradução de `GSDrawState` em chave de shader
e de pipeline; tabela de mapeamento do blend do GS; política de fallback; composição PMODE/DISPFB;
fila e thread GS; modo diferencial; contadores. A RHI não sabe o que é PS2.

---

## 7. Roteiro em etapas

Cada etapa compila, roda o menu (`bench_menu.txt`) e o `explore2.txt`, e fecha com os limiares da
seção 5 e uma linha no `roteiro-video.md`. Esforço: **P** ≈ até 1 dia, **M** ≈ 2–4 dias, **G** ≈ 1–2
semanas (relativo, para uma pessoa com agente).

| # | Etapa | Entrega | Verificação | Esforço | Risco |
|---|---|---|---|---|---|
| E-1 | Política + contador estendido | `GSGpuDrawPolicy::Classify`; contador imprime % que iria à GPU por fase; histograma de `ALPHA` (A,B,C,D) dos "outros"; contagem de z > 0xFFFF em Z16; textura lida de alvo com PSM diferente; local→local de alvo; `LoadClut` de alvo; taxas de DATE/fog/ATST/AFAIL/FBMSK registradas no `gs-gpu-plan.md` | MD5 do menu inalterado | P | baixo |
| E0 | Thread GS com CPU | `GSGpuBackend` com fila e thread; todo comando executa no `GSCpuBackend` na thread GS; Present por pedido; janela D3D criada/destruída na thread GS | **MD5 idêntico**; log de tamanho da fila e esperas | M | médio (sincronização, deadlock); ganho de velocidade quase nulo (98,7% era raster, só muda de thread) |
| E1 | Alvos e coerência sem desenhar | RHI: formatos, RT/DS, upload/readback por retângulo; `GSTargetCache` + dono por página; conversões VRAM↔linear CT32/CT24/Z16S; modo `PS2X_GS_GPU_ROUNDTRIP=1` que sobe e lê de volta todo alvo a cada present | ida e volta **bit a bit**; MD5 idêntico | M | médio (mapeamento de páginas, tamanhos) |
| E2 | Sprites sem textura | pipeline mínimo; blend off/simples/aditivo; Z16S; FBA; FBMSK por byte; scissor; modo por alvo/quadro; modo diferencial | limiares E2; contagem de syncs por quadro | M | médio (dual-source, Z) |
| E3 | Triângulos | flat/Gouraud, fog, alpha test KEEP; agrupamento de primitivas consecutivas com o mesmo estado num só `Draw` | limiares E3 | M | médio (regra de aresta) |
| E4 | Textura direta | CT32/CT24/CT16; cache por versão de página; TFX (modulate/decal/highlight/highlight2); wrap e region; ponto e bilinear; FST e STQ | limiares E4 | G | alto (muitos detalhes de UV) |
| E5 | CLUT | índices `R8_UINT`; espelho de CLUT; T8/T4/T8H/T4HL/T4HH | limiares E5; **medir FPS** (primeira etapa com chance de ganho real) | M | médio |
| E6 | Render-to-texture | alvo como SRV; cópia quando é o alvo ligado; local→local GPU→GPU | limiares E6; FPS na abertura | G | alto (sobreposição de alvos, formatos) |
| E7 | Casos difíceis | tabela de blend completa + blend no shader com cópia do destino; PABE; AFAIL em 2 passes; DATE com stencil; FBMSK por bit | modo diferencial por chave | G | alto |
| E8 | Apresentação na GPU | composição PMODE/DISPFB/BGCOLOR/campo em shader; write-back só para `shot`/debug; decisão da janela principal | imagem da janela D3D = PNG do `shot`; FPS | M | médio (janela/entrada) |
| E9 | Medição e limpeza | FPS em menu/abertura/gameplay; tabela de casos não suportados; preparar Vulkan | relatório | P | baixo |

Notas:
- A ordem pedida (sprites → Gouraud → CT32 → CLUT) é E2 → E3 → E4 → E5; a diferença é que **Z16S
  entra já na E2**, porque sem ele quase nenhum draw do jogo sai da CPU.
- E0 pode ser entregue e mantida sozinha (`PS2X_GS=cpu-mt`) mesmo que o resto atrase: separa o
  risco de threads do risco de desenho.

---

## 8. Riscos e perguntas em aberto

### Riscos

1. **Pingue-pongue CPU↔GPU** nas fases de cobertura parcial: pode deixar o modo GPU **mais lento**
   que o de CPU até a E5/E6. Mitigação: modo por alvo/quadro, contadores de sync, e não prometer
   desempenho antes da E5.
2. **Render-to-texture com reinterpretação** (alvo CT32 lido como T8/CT16 etc.) força write-back
   com espera. O contador não distingue hoje; a E-1 mede.
3. **Diferenças da referência que não são do hardware**: falta de regra top-left, Z truncado,
   `TexturePageCache` (texturas "congeladas" por página até `TEXFLUSH`; na GPU, ler um alvo direto
   vê sempre o dado mais novo). Cenas que dependem disso vão diferir; o PCSX2 desempata.
4. **Fatores de blend > 1** (As/FIX > 0x80) e **C = Ad** não cabem no blend fixo; em até 38% dos
   draws do menu o blend é "outro". Sem o histograma da E-1 não dá para saber quanto disso o
   mapeamento fixo cobre.
5. **Ordem dentro do quadro**: a GPU executa os draws na ordem da fila, mas o modo diferencial e os
   write-backs mudam o tempo; bugs de coerência podem aparecer só sem o modo diferencial. Manter o
   modo `ROUNDTRIP` (E1) como teste de regressão permanente.
6. **Fila e latência**: com backpressure frouxa a entrada fica atrasada; apertada demais, a
   `GameThread` espera a GPU. Medir.
7. **Volume de upload**: ~360 transferências host→local por present na abertura; bytes por quadro
   não estão no documento de medição. Se for alto, a cópia para a fila pesa (mitigação: buffer
   circular com cópia única).
8. **Precisão de UV/STQ** na E4 é onde PCSX2 e outros acumulam hacks; reproduzir a fórmula da CPU
   no shader em inteiros reduz, mas não elimina (float da GPU vs `float` da CPU com outra ordem de
   operações).
9. **Portabilidade**: dual-source blending existe em Vulkan (`dualSrcBlend`) e D3D12; OK. Cópia de
   depth por retângulo tem restrições diferentes em cada API; mantida atrás de `CopyToReadback`.

### Perguntas para o usuário

1. Aceita que as fases E1–E4 **não melhorem o FPS** (servem para provar correção), com o primeiro
   ganho esperado na E5/E6?
2. Aceita a mudança de critério de regressão do backend GPU de MD5 para os limiares da seção 5?
   Quem decide quando a GPU estiver "mais certa" que a referência de CPU (com base no PCSX2)?
3. Na E8, a janela D3D vira a principal? Pode a entrada sair da raylib (XInput/Win32) se for preciso?
4. Tamanho canônico dos alvos (1024x1024) e limite `FBW ≤ 16`: ok como ponto de partida?
5. Mudar o pad virtual para contar quadros do guest (`vsyncTick`) em vez de quadros do host, para
   ter capturas comparáveis entre backends? (É mudança no runtime, fora do GS.)
6. Quando tiver a BIOS, quais telas capturar no PCSX2 como oráculo (proposta: menu de idioma, título,
   primeiro quadro estável da abertura)?

---

## 9. O que não foi confirmado

- Se **todo** `Submit` vem de uma única thread do host: o `EeScheduler` executa os threads do guest
  e o caminho de DMA/GIF passa por `processPendingTransfers`, mas não segui todos os caminhos (VU1
  `XGKICK`, PATH3). O desenho funciona com vários produtores porque o push é feito sob `m_stateMutex`.
- Taxas de DATE, fog, ATST, AFAIL, FBMSK, PABE, filtro, wrap, local→local e bytes de upload: o
  contador as imprime, mas o log com esses números não está no repositório; só os da tabela do
  `gs-gpu-plan.md` foram usados.
- Regras do D3D11 citadas de memória e a confirmar com a camada de debug: RTV e DSV precisam ter as
  mesmas dimensões; fatores de blend limitados a [0,1] em alvos UNORM; cópia de recurso de
  profundidade só do subrecurso inteiro.
- Comportamento do gamepad da raylib/GLFW sem foco de janela.
- Se a raylib exporta sempre PNG RGBA 8 bits não entrelaçado (para o decodificador mínimo do
  `imgdiff.py`).
- Comportamento real do GS para z maior que o formato (clamp ou truncamento): a referência trunca na
  escrita; só o PCSX2/hardware dirá qual está certo.
- Nada aqui foi compilado nem executado: é um documento de design.
