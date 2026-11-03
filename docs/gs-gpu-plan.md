# Plano: backend GS por GPU

Status: **planejamento** (nada implementado). Fonte do estudo: leitura dos headers e de partes do
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
1. `GSGpuBackend : GSRasterBackend`, escolhido por variável de ambiente (ex.: `PS2X_GS=gpu|cpu`).
   `GSCpuBackend` continua como **referência** e fallback.
2. **RHI fina** (camada de dispositivo): textura, buffer, pipeline, draw, cópia, readback,
   apresentar. Shaders em **HLSL** uma vez só; D3D usa direto, Vulkan recebe SPIR-V (DXC).
3. Ordem das APIs (decidida pelo usuário): **D3D11 no Windows primeiro**, depois **Vulkan** e **D3D12**.
   D3D11 dá menos código e basta para este GS; Vulkan traz Linux/Steam Deck; D3D12 entra quando a RHI
   estiver estável. ("D3D13" não existe; a versão atual é D3D12.)

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
- Qual biblioteca de janela/entrada o runtime já usa para apresentar? `[PREENCHER]` (verificar
  antes da etapa 1).
- Quanto do que o jogo usa cai nos "casos difíceis"? medir com um contador de features por draw
  no `GSCpuBackend` antes de investir (barato e dá os dados para priorizar).
