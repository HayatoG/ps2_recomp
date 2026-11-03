# Projetos similares (regra 8)

Levantamento feito em 2026-11-03 via `gh` (metadados: licença, último push, estrelas) e busca web. Só links e resumos; nenhum código foi copiado. Onde a licença consta como "não detectada", o GitHub não identificou arquivo de licença: tratar como **todos os direitos reservados** até verificar manualmente. Itens marcados (?) não foram confirmados em fonte primária.

> Informações específicas do Ghost in the Shell (desenvolvedor, protótipos, formatos) estão em `ghost_pc/docs/game-research.md`.

## 1. Recompilação estática PS2 (R5900 -> C++/nativo)

| Projeto | URL | Licença | Última atividade | O que reaproveitar | Riscos |
|---|---|---|---|---|---|
| PS2Recomp (ran-j) | https://github.com/ran-j/PS2Recomp | GPL-3.0 | push 2026-10-23; v0.4 (2026-04-05); ~3,8k estrelas | Recompilador ELF -> C++ via TOML (`ps2xRecomp`), runtime (`ps2xRuntime`: memória, tabela de funções, syscalls parcial, stubs GS/VU), suporte a MMI e VU0 macro, `ps2xAnalyzer` (descoberta de funções), `ps2xIOP` (roda IRX originais com HLE genérico), patches C++ por endereço, guia de fluxo com Ghidra na wiki | Experimental; README admite desempenho de VU/GS "muito ruim" e emulação parcial; VU1 não citado; GPL-3.0 contamina o port se linkarmos (repo privado, mas distribuição futura exige GPL ou reescrever); sem jogos comerciais confirmados no README |
| ps2-port-engine (usado por burnout3-pc) | https://github.com/danielgomesvieira2000/burnout3-pc (submódulo `engine/`) | não detectada no repo (README: sem conteúdo do jogo) | push 2026-10-31; 206 estrelas | Pipeline em estágios: `isotool` -> `analyze` -> `derive` -> `lift` -> `port` -> `package`; C++ gerado localmente a partir do disco do usuário e nunca commitado (alinha com regras 2/4); renderer GS nativo em thread própria (Vulkan/D3D11); VU1 gravada e compilada AOT, senão interpretada | Alpha; código gerado com apoio de IA, qualidade a auditar; licença do engine a confirmar |
| UwUCammie/Burn3, RgeditV1/ROR-Recomp, omegakatana92/Samurai-Western-Recomp, saltyboosack-blip/Spider-Man-2-PS2-Recomp, m7kor/Project-4, Pedrohs1771/ps2-native | `gh search repos "ps2 recompilation"` | não detectada / GPLv3 (ps2-native) | todos em 2026-08 a 2026-11 | Exemplos de como adaptar o PS2Recomp por jogo (perfis, adaptadores, overrides); `ps2-native` promete recuperação offline de funções EE | Quase sem estrelas; qualidade e legalidade (assets) desconhecidas; usar só como leitura |
| gmh5225/PS2Recomp | https://github.com/gmh5225/PS2Recomp | herdada | fork "Not ready" | nada | Fork não oficial, não usar |

Como lidam com os pontos difíceis (resumo):
- **MMI**: tradução literal instrução a instrução para C++ com tipos de 128 bits (SSE/AVX no host).
- **VU0 macro (COP2)**: traduzido inline; **VU1/micro**: interpretar ou compilar AOT a partir de microprogramas gravados (burnout3-pc); desempenho é o gargalo.
- **Syscalls**: despacho pelo código da instrução e fallback em `$v1` (PS2Recomp); tabela de syscalls do kernel EE a ser implementada em HLE.
- **Overlays/relocs**: PS2Recomp trata relocações do ELF e permite `handler@ENDEREÇO` para binários stripped; overlays em runtime não estão descritos, exigem trabalho nosso (nosso jogo tem `MODULES/*.IRX` para o IOP, não necessariamente overlays EE).

## 2. Decomps e ports de jogos PS2

| Projeto | URL | Licença | Última atividade | Metodologia / o que reaproveitar | Riscos |
|---|---|---|---|---|---|
| Sly Cooper 1 (TheOnlyZac) | https://github.com/TheOnlyZac/sly1 | não detectada | 2026-10-26 | Matching decomp com splat + ee-gcc; modelo de organização por função; um dos primeiros PS2 com matching | Só útil como modelo de processo |
| God Hand (LucasPicoli) | https://github.com/LucasPicoli/god-hand-decomp/wiki | não detectada | 2026-11-02 | Melhor documentado: splat (sobre `.rom` via objcopy), `INCLUDE_ASM` por função, objdiff, m2c, asm-differ, decomp-permuter, ee-gcc 2.96 + dvp-as + SN linker; script de setup baixa o SDK de arquivos comunitários sem redistribuir; `config/versions.json` com serial/hash; gate sha256 do ELF; scripts de extração de ISO | Exige toolchain original (checar o compilador do nosso jogo) |
| Ratchet & Clank 3 (vetusmagnus/rac3-uya-decomp, antes OpenRAC) | https://github.com/vetusmagnus/rac3-uya-decomp | não detectada | 2026-10-22 | Identificação de toolchain (SN ee-gcc 2.95.3 v1.36, assembler por função); splat | idem |
| Klonoa 2 (entriphy) | https://github.com/entriphy/kl2_lv_decomp | não detectada | 2025-03-07 | ee-gcc 2.9-991111-01 (disponível no decomp.me) bate "na maioria"; referência de metodologia | Parado desde 2025 |
| PaRappa the Rapper 2 | https://github.com/parappadev/parappa2 | não detectada | 2026-09-17 | Matching decomp ativo, 207 estrelas | idem |
| ICO + port PC (nathanialf) | https://github.com/nathanialf/ico , https://github.com/nathanialf/ico-pc | ico-pc: MIT (README) | 2026-10-30 / 2026-11-01 | Exemplo completo decomp -> port PC multiplataforma (Vulkan/D3D12/OpenGL), extrai assets do disco do usuário na primeira execução, `docs/LEGAL.md` e `docs/port/ARCHITECTURE.md` como modelo de documentação | Port roda "o código do próprio jogo" sobre camada de compatibilidade; GS/VU não detalhados no README |
| Haunting Ground / OpenHuey (ArnautDaniel) | https://github.com/ArnautDaniel/OpenHuey | não detectada | 2026-11-02 | Decomp + port PC + reescrita por atores | Recente, pouco maduro |
| Kazarov (TheMoonPeople) | https://github.com/TheMoonPeople/Kazarov | não detectada | 2026-11-02 | Build system compartilhado entre várias decomps PS2 | Específico do grupo |
| ps2-decomp-toolkit (ps2devnoob) | https://github.com/ps2devnoob/ps2-decomp-toolkit | não detectada | 2026-06-26 | Toolkit: análise ELF, split incremental, asm editável, rebuild byte a byte | 2 estrelas, imaturo |
| Outros: Burnout 3 port (PC), Spyro AHT / Sly 2 / Sly 3 / Herdy Gerdy (neofodere), Crash Tag Team Racing | via `gh search repos "ps2 decompilation"` | variam | 2026-10 | Comparar abordagens de GS/VU | Qualidade variável |

Observação: o recurso comum é **splat + ee-gcc 2.9x + objdiff + decomp.me**. Para um jogo de ação 3D da era 2004 (SDK Sony tardio, provavelmente ee-gcc 2.95/3.2 com libs Sony), matching exige descobrir o compilador exato, o que é caro; ver recomendações.

## 3. Ferramentas

| Ferramenta | URL | Licença | Última atividade | Uso para nós | Riscos |
|---|---|---|---|---|---|
| ghidra-emotionengine-reloaded (já instalada) | https://github.com/chaoticgd/ghidra-emotionengine-reloaded | Apache-2.0 (segundo agregador) | 2026-10-29 | Processador R5900 (MMI, COP2/VU0 macro, loaders ELF/IRX) para Ghidra | Semântica de algumas MMI/VU0 pode ser incompleta no decompilador |
| ghidra-ps2sdk (?) | citado por agregadores como extensão que exige a anterior; URL não confirmada, buscar na página de releases do emotionengine-reloaded | (?) | (?) | Analisador + bancos de tipos para libs do SDK Sony (detectar/rotular funções e structs) | Zona cinzenta legal declarada pelos próprios autores (declarações da API proprietária); verificar origem antes de adotar |
| ps2dev/ps2sdk | https://github.com/ps2dev/ps2sdk | AFL-2.0 | 2026-11-01 | Nomes de syscalls do kernel (`lkernel`), headers EE/IOP (libpad, libdma, libgraph equivalentes, SIF/RPC), nomes e layout de módulos IRX; base para nossos stubs/HLE | API e nomes não são idênticos aos da libs Sony; cuidado ao assumir que símbolos batem |
| ps2dev/ps2toolchain | https://github.com/ps2dev/ps2toolchain | BSD-2-Clause | 2026-10-03 | Compilador `mips64r5900el-ps2-elf-gcc` moderno para testar recompilação e homebrew de teste | GCC moderno **não** gera bytes idênticos aos de ee-gcc 2.9x; inútil para matching |
| ps2dev/ps2sdk-ports | https://github.com/ps2dev/ps2sdk-ports | não detectada | 2026-10-07 | Referência de libs portadas | baixo |
| ps2-packer | https://github.com/ps2dev/ps2-packer | não detectada | 2026-10-08 | Identificar se o ELF/overlays estão empacotados | baixo |
| splat | https://github.com/ethteck/splat | MIT (?) | 2026-10-10 | Split do ELF em asm por função/segmento; suporte a PS2 (R5900) | Exige config manual de segmentos |
| spimdisasm | https://github.com/Decompollaborate/spimdisasm | MIT (?) | 2026-08-06 | Disassembler MIPS usado pelo splat | idem |
| m2c (mips_to_c) | https://github.com/matt-kempster/m2c | GPL-3.0 (?) | 2026-10-22 | Pseudo-C inicial a partir de asm | Não cobre bem MMI/VU |
| objdiff | https://github.com/encounter/objdiff | Apache-2.0/MIT (?) | 2026-10-19 | Comparação asm compilado vs original | Só útil se fizermos matching |
| decomp-permuter | https://github.com/simonlindholm/decomp-permuter | GPL-3.0 (?) | 2026-10-06 | Busca de variações de C para bater bytes | idem |
| decomp.me + compilers | https://github.com/decompme/decomp.me , https://github.com/decompme/compilers | AGPL-3.0 (?) | 2026-11-01 / 2026-10-31 | Binários de ee-gcc 2.9x para testes de matching | Sem SDK Sony redistribuível |
| maspsx | https://github.com/mkst/maspsx | (?) | n/c | Ajuste de saída do assembler GNU para o estilo do assembler SN/PSX (mais usado em PS1; avaliar para PS2) | Foco PS1 |
| ps2tek | https://github.com/PSI-Rockin/ps2tek | (?) | n/c | Documentação de hardware: EE, VU, GS, GIF, DMA, SIF, IOP | Documento, não código |
| PCSX2 | https://github.com/PCSX2/pcsx2 | GPL-3.0 | 2026-11-02 | Referência de semântica de VU0/VU1 (interpretador e recompilador), GS (software renderer, Vulkan/D3D), DMA, VIF/GIF, SIF; **também o melhor depurador do jogo** (breakpoints, dump de memória, trace de GS/DMA) | GPL: copiar código obriga GPL; usar como referência de comportamento e para validar nossa saída |
| IDA FLIRT PS2 (SDK sigs) | https://dogcxf037x385.cloudfront.net/assemblergames.com/threads/reverse-engineering-ps2-signature-files-for-ida-pro.68479/index.html (arquivo do fórum) | n/c | 2018 | Assinaturas FLIRT por versão de SDK (o post recomenda 3.0.3 se a versão for desconhecida); aplicáveis no Ghidra via ApplySig | Origem em libs Sony: risco de direitos autorais, não commitar `.sig`; se usar, manter fora do git |
| ApplySig | https://github.com/NWMonster/ApplySig | não detectada | 2020-05-15 | Aplica assinaturas FLIRT no Ghidra | Parado, requer testar com a versão do Ghidra |
| IDAPy-PS2 | https://github.com/Ziemas/IDAPy-PS2 | não detectada | 2023-10-30 | Scripts para EE/IOP (nomear syscalls, SIF/RPC); lógica portável para Ghidra | Escrito para IDA 7.0 |
| chaoticgd/wrench | https://github.com/chaoticgd/wrench | não detectada | 2026-10-16 | Exemplo de ferramentas de formato de arquivo PS2 (Ratchet & Clank); mesmo autor do plugin Ghidra | Específico de outro jogo |

## Recomendações (priorizadas)

1. **Caminho principal: recompilação estática (PS2Recomp ou pipeline próprio no estilo ps2-port-engine) com Ghidra como apoio de análise, e não matching decomp.** Motivos: o objetivo é um port jogável em PC; matching exige descobrir o compilador/SDK Sony exato de uma build de 2004 e é muito lento para 7462 funções; a recompilação já foi demonstrada em jogos reais (burnout3-pc alpha jogável) e com IRX via `ps2xIOP`. Reservar decomp manual (C legível) apenas para rotinas selecionadas e que precisem ser alteradas (câmera, resolução, widescreen), usando os passos 4 e 5.
2. **Spike de viabilidade em 1 a 2 semanas**: rodar o PS2Recomp (v0.4) sobre `work/SLES_530.20` (já extraído) com `.toml` de teste, listar instruções/syscalls não suportados, contar quantas funções caem em MMI/COP2. Decisão de adotar o PS2Recomp vs. escrever o nosso depende desse resultado; registrar em `docs/decisions.md`. Atenção à GPL-3.0: o repo é privado, mas qualquer distribuição futura do port deve usar GPL ou evitar código dele; confirmar com o usuário antes.
3. **Nomeação de funções do SDK** (item pendente no AGENTS.md): (a) importar nomes/protótipos do ps2dev/ps2sdk (AFL-2.0) como base de headers; (b) avaliar a extensão ghidra-ps2sdk e assinaturas FLIRT com ApplySig, mantendo qualquer `.sig`/banco de tipos derivado de libs Sony **fora do git** e registrando só os nomes/endereços resultantes (conhecimento nosso); (c) usar os padrões de IDAPy-PS2 para rotular syscalls e RPCs SIF.
4. **Renderização/hardware**: estudar ps2tek e o código do PCSX2 (GS, VIF, VU) como referência de comportamento; para o port, usar um renderer GS dedicado em Vulkan/D3D (modelo burnout3-pc / ico-pc), com VU1 interpretada primeiro e compilação AOT de microprogramas depois, medindo o desempenho.
5. **Usar o PCSX2 como oráculo**: tracing de DMA/GIF/VU1, dumps de memória e breakpoints no jogo real para validar a saída recompilada e descobrir formatos de `IMAGE.BIN`/`BGM.BIN`.
6. **Formatos de arquivo**: sem trabalho prévio; engenharia reversa própria. Começar pelo código (xrefs de leitura de arquivo no ELF via ReVa, estrutura da tabela de `IMAGE.BIN`), depois ferramentas em `tools/` com logs. Vídeos `MOV*.BIN` provavelmente PSS/MPEG-2 (verificar magic `00 00 01 BA`); áudio em VAG/ADPCM ou similar (hipóteses, sem verificação).
7. **Documentação e processo**: adotar o modelo de ico-pc (`docs/LEGAL.md`, `ARCHITECTURE.md`, extração de assets do disco do usuário na primeira execução) para atender às regras 2, 4 e 5. Se decidirmos fazer matching em alguns módulos, usar a pilha splat + objdiff + decomp.me, tomando o setup do God Hand como referência.
8. **Riscos gerais**: licenças GPL (PS2Recomp, PCSX2, m2c); material derivado do SDK Sony (assinaturas, tipos) é zona cinzenta; vários repositórios sem licença declarada, usar só como leitura/estudo; projetos muito novos e com código gerado por IA exigem auditoria.
