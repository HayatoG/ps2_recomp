# Roteiro de vídeo — "Como portar um jogo de PS2 para PC (do zero, passo a passo)"

> **Documento vivo.** Escrito na primeira pessoa, no tom do canal, para ser lido em voz alta. Edite à vontade.
> **Formato:** vídeo de **aprendizado**. Em cada capítulo: (1) o conceito, explicado com analogia; (2) o que fizemos; (3) **por que** decidimos assim e o que descartamos; (4) o que aprendemos.
> **Regra de manutenção (para quem edita o projeto):** sempre que houver um aprendizado novo ou uma decisão relevante, acrescentar aqui, **com o porquê**. Marque pendências com `[PREENCHER]` e nunca invente resultado: se ainda não funcionou, o vídeo diz que não funcionou.
> **Aviso de conteúdo:** nada do jogo aparece no vídeo além do que for permitido; mostrar código/ferramentas e dados nossos (endereços, nomes, números), nunca texturas, áudio ou vídeos do jogo sem avaliar direitos. Dump sempre do próprio dono do jogo.
> Marcações: `[TELA]` = o que mostrar; `[B-ROLL]` = imagem de apoio; `[ANIMAÇÃO]` = gráfico explicativo; `[SFX]` = efeito.

---

## Títulos e miniaturas (ideias)
1. "Eu tentei portar um jogo de PS2 para PC — veja como funciona de verdade"
2. "Engenharia reversa na prática: do disco do PS2 ao PC"
3. "Como funciona um recompilador de PS2? (e por que decompilar na mão é uma loucura)"

## O que você vai aprender (mostrar na tela logo no começo)
- O que é um **dump**, uma **ISO** e como o PS2 organiza um disco.
- O que é um **ELF**, um **disassembler** e um **decompilador**.
- Por que o processador do PS2 (**R5900 / Emotion Engine**) engana ferramentas comuns.
- A diferença entre **decompilar** e **recompilar estaticamente** — e por que escolhemos a segunda.
- Como descobrir um **formato de arquivo desconhecido** e uma **compressão** sem documentação.
- Como organizar um projeto de RE para ser **legal, reprodutível e reaproveitável** em outros jogos.

---

## 0. Gancho (0:00–0:45)
**Fala:**
"Um jogo de 2004, de PS2, rodando no meu PC — mas sem emulador. Não é 'abrir o jogo no PCSX2'. A ideia é pegar o programa do PS2, entender o que ele faz e transformá-lo em um programa nativo de PC. Eu não sabia se ia funcionar. Neste vídeo eu mostro o processo inteiro: o que deu certo, o que travou e, principalmente, **por que tomei cada decisão** — porque o aprendizado está nas escolhas, não só no resultado."

`[TELA]` Pasta do projeto, logs rolando, número de funções. `[B-ROLL]` Caixa do PS2, tela de título do jogo (apenas se liberado).

**Promessa honesta:** "Vou ser sincero em cada etapa: vou mostrar o que ainda **não** funciona."

---

## 1. Começando do jeito certo: regras antes de código (0:45–3:00)

**Conceito — o que é um *dump*:**
"Dump é a cópia exata do disco do jogo em um arquivo (aqui, uma `.iso` de uns 4,6 GB). Pense numa foto perfeita do disco. O projeto **exige o seu dump**: ele não vem com nada do jogo."

**O que fizemos:** criamos um repositório com regras claras (privado, sem commitar nada com direitos autorais, sempre exigir o dump original, logs exportáveis, launcher separado).

**Por quê?**
- *Direitos autorais:* o código que escrevemos é nosso; o jogo não. Se o dump não está no repositório, o projeto continua legítimo e compartilhável **depois** (e o repo fica privado "até segunda ordem" para eu decidir isso com calma).
- *Validar o dump:* o launcher confere o **serial** (`SLES_530.20`) e o **hash** do executável. Assim, se alguém usar outra versão do jogo, o projeto avisa em vez de quebrar de forma misteriosa. Isso economiza horas de depuração.
- *Logs sempre:* quando algo falha às 3 da manhã, um log é a única testemunha.
- *Base genérica + uma pasta por jogo:* eu quero rodar **outros jogos** depois. Se misturasse tudo, ia ter que refazer. Então a raiz guarda o que serve para qualquer jogo, e cada jogo (`ghost_pc`) guarda só o que é dele.

**Aprendizado:** "decidir a organização no começo custa 1 hora; consertar depois custa uma semana."

---

## 2. Abrindo o disco (3:00–6:00)

**Conceito — ISO9660 e o disco do PS2:**
"Um disco de PS2 tem um sistema de arquivos como o de um CD comum (ISO9660). No começo há um arquivo de texto, o `SYSTEM.CNF`, que diz **qual programa o console deve executar**. No nosso disco: `BOOT2 = cdrom0:\SLES_530.20`. O 'SLES' diz que é um jogo **europeu** (PAL)."
`[ANIMAÇÃO]` Disco → setores de 2048 bytes → diretório raiz → SYSTEM.CNF → ELF.

**O que fizemos:** escrevemos um leitor de ISO (em vez de usar um programa pronto) e extraímos o executável (`SLES_530.20`, 3,7 MB), a pasta `MODULES` e os arquivos grandes `.BIN`.

**Por quê escrever o leitor?** São ~100 linhas de Python, ficam no projeto, geram logs e o launcher reaproveita. Um programa pronto seria uma dependência a mais e não validaria o hash.

**Aprendizado:** o jogo inteiro tem **um programa pequeno (3,7 MB)** e **gigabytes de dados** (vídeos, áudio, gráficos). Decompilar o programa é a parte "difícil e finita"; os dados são a parte "grande e repetitiva".

---

## 3. O programa: ELF, disassembler e decompilador (6:00–10:00)

**Conceito:**
- *ELF:* o "formato de empacotamento" do programa (seções de código, dados, memória).
- *Disassembler:* mostra as instruções da CPU em texto (`addiu`, `lw`…).
- *Decompilador:* tenta reconstruir algo parecido com C a partir dessas instruções. **Não recupera nomes originais** — o jogo vem "stripped" (sem nomes de funções).
`[TELA]` Ghidra mostrando uma função sem nome (`FUN_001943e0`).

**O que fizemos:** importamos o ELF no Ghidra pelo **ReVa** (uma ponte que deixa o assistente de IA conversar com o Ghidra).

**Por quê Ghidra + ReVa?** O Ghidra é gratuito, tem decompilador forte e é programável. A ponte ReVa permite automatizar: contar funções, ler strings, renomear em lote. Outras ferramentas da lista (radare2, IDA) ficam como apoio.

### O momento "eita": a análise saiu errada
Rodamos a análise e o ponto de entrada virou lixo (`special2(0,0,0x10,0x28)` repetido 29 vezes). **O que aconteceu?**
- O PS2 usa o **R5900 (Emotion Engine)**: um MIPS com **instruções extras de 128 bits (MMI)**.
- O Ghidra só trazia o MIPS genérico, que **não conhece** essas instruções (`28 0C 00 70` = uma delas, usada para zerar registradores).
- Resultado: o início do programa, que é só "limpar a memória", virou ruído.
`[ANIMAÇÃO]` Mesmos bytes: "MIPS genérico = erro" vs "R5900 = clear de registrador de 128 bits".

**Decisão:** instalar a extensão **ghidra-emotionengine-reloaded** (Apache-2.0) em vez de contornar.
**Por quê?** Contornar significaria interpretar milhares de instruções MMI na mão. A extensão ensina o Ghidra a CPU inteira. **Verificamos antes de instalar**: lemos o conteúdo do pacote, conferimos a licença e compilamos a especificação.
**Aprendizado:** *"a ferramenta certa vale mais que a esperteza"* — e **sempre** valide que a análise faz sentido olhando o começo do programa (o 'crt0').

**Resultado:** o `entry` virou um código legível: limpa o `.bss`, chama `SetupThread`, `FlushCache`, habilita interrupções e chama o `main`. 7.462 funções detectadas.

---

## 4. Dando nomes às funções (10:00–13:00)
**Conceito:** sem nomes, ler código é impossível. Mas o PS2 tem pistas: as **chamadas de sistema (syscalls)** têm números fixos e públicos, e as **bibliotecas Sony** deixam strings (`"sceSifRebootIop"`, `"cdrom0:\MODULES\..."`).
**O que fizemos:** agentes paralelos nomearam ~190 funções (syscalls do kernel, libc, bibliotecas gráficas, de CD, de pad…) **só com evidência**, marcando o que era palpite.
**Por quê "só com evidência"?** Um nome errado confunde para sempre. Preferimos poucos nomes confiáveis (e um CSV com a confiança de cada um) a muitos duvidosos.
**Aprendizado:** o código do jogo é C++ com a engine própria da desenvolvedora, com *namespaces* (`abes`, `Stacom`) visíveis nas strings; a **Cavia** (não a Exact, como se cogitava) desenvolveu o jogo — um exemplo de **checar fonte** antes de afirmar. (Uma afirmação de que a engine seria "Sega/Production I.G." ficou como **não confirmada**.)

---

## 5. Um segundo computador dentro do PS2: o IOP (13:00–16:00)
**Conceito:** o PS2 tem **duas CPUs**: a principal (EE) e o **IOP** (um chip do PS1 usado para controle, disco, som). O IOP roda **módulos `.IRX`** carregados pelo jogo.
**O que descobrimos (confirmado por referência cruzada no ELF):**
- Ordem de carga: `sio2man → mtapman → padman → cdvdstm → mcman → mcserv → libsd → sdrdrv → ezsound`.
- O jogo reinicia o IOP com uma imagem própria (`IOPRP280.IMG`, SDK Sony **2.8.0**) antes de carregar os módulos.
- O `EZSOUND` é um módulo de som **da própria desenvolvedora**, não do SDK — significa trabalho extra para o port.
**Por quê importa?** Num port, tudo que o IOP fazia (ler pad, cartão de memória, disco, som) precisa ser **reimplementado no PC**. Saber a lista e a ordem diz exatamente o que emular.

---

## 6. Os arquivos de dados: descobrindo formatos sem manual (16:00–22:00)
**Conceito — engenharia reversa de formato:** quando não há documentação, você procura **padrões**: assinaturas (magic numbers), tabelas de offsets/tamanhos, alinhamentos.

### 6.1 O contêiner `fpk`
Todos os `.BIN` grandes começam com `fpk\0`. Mostramos como ler: cabeçalho, contagem de entradas, tabela de 16 bytes por entrada (offset + tamanho), alinhado a 0x800 (o tamanho de um setor de disco — **não é coincidência**).
**Erro e correção:** minha hipótese era "offsets absolutos"; estava errada — são relativos ao início dos dados. *Mostrar o erro ensina mais que mostrar o acerto.*
**Por quê validar?** O parser confere que nenhuma entrada passa do fim do arquivo nem se sobrepõe a outra. Se bate nos 7 arquivos, a interpretação está certa.

### 6.2 A compressão `V3a`
"No IMAGE.BIN há blocos que começam com `\0V3a`. Descobrir o algoritmo foi o ponto alto."
- **Em vez de adivinhar**, procuramos a **rotina de descompressão no próprio executável** (classe `CHlzDecode`, achada pela RTTI do C++).
- Resultado: é o algoritmo **ar002 do Okumura — a família LHA `-lh5-`** (LZSS + Huffman estático).
- **Validação:** 74 de 74 entradas descomprimem com tamanho **exato**; 180 MB viram 531 MB.
**Por quê ler o código do jogo em vez de testar algoritmos?** O código é a **fonte da verdade**. Testar "qual LZ será?" pode levar dias; ler a rotina leva uma tarde.
`[ANIMAÇÃO]` LZSS: "copie 5 bytes de 20 posições atrás" + Huffman: "letras comuns usam códigos curtos".

### 6.3 Texturas `zim`
10.445 texturas (4 e 8 bits, com paleta). Os pixels estão **lineares** e vêm junto de pacotes prontos para o hardware gráfico do PS2 (GS). Convertemos algumas para PNG para **ver se eram imagens de verdade** (um HUD, um fundo, uma folha de sprites) — o teste mais honesto de um decodificador.
**Aprendizado:** "um decodificador só está certo quando a imagem aparece certa."

---

## 7. A grande decisão: decompilar na mão ou recompilar? (22:00–27:00)
**Conceito:**
- *Decompilar manualmente (matching decomp):* reescrever o jogo em C/C++ até gerar **o mesmo binário**. Resultado lindo, mas **meses/anos** por jogo (esse tem ~39 mil funções).
- *Recompilação estática:* um programa lê cada instrução do PS2 e **gera código C++ equivalente**, que você compila para PC. É "tradução automática" do programa inteiro.
`[ANIMAÇÃO]` Duas rotas: manual (escada) vs automática (esteira).

**Decisão:** testar a **recompilação estática** primeiro (PS2Recomp).
**Por quê?**
1. É **rápida de testar** (resposta em dias, não meses).
2. O trabalho de RE que já fizemos (funções, módulos) **alimenta** o recompilador — nada é perdido.
3. Se não servir, a decomp manual continua possível, **só para as partes críticas**.
**Riscos que registramos antes de começar:** a parte gráfica (GS) e os processadores vetoriais (VU) do runtime são limitados; a licença é **GPL-3.0**, o que afeta o que eu puder distribuir.
**Por quê tratamos o teste como *spike*?** Spike = experimento curto com critério de decisão. Se o resultado for ruim, descartamos sem culpa.

### O resultado do recompilador
- `ps2_recomp` rodou em **67 s**, sem erros: **39.045 funções**, **39 mil arquivos .cpp**, **248 MB** de C++.
- 4.520 avisos — **todos** de saltos indiretos (`JR/JALR`) que ele não sabe resolver sozinho (ex.: `switch` compilados em tabelas).
- Nenhum aviso de instrução não suportada (MMI/VU0 passaram!).
**O que isso prova e o que não prova:** prova que a **tradução** é possível; **não** prova que o jogo roda. Falta compilar, ligar ao runtime e ver o boot. *(Dizer isso no vídeo é parte da honestidade.)*

---

## 8. Compilando 248 MB de C++ (27:00–31:00)
**Conceito — por que compilar demora tanto:** milhares de arquivos enormes; o compilador junta vários em "unity builds" para ir mais rápido.
**O que fizemos:** build em segundo plano com 14 jobs, logs em arquivo, monitor que avisa só de erros e do fim.
**Por quê em segundo plano?** (regra 9 do projeto) **enquanto compila, a gente programa** — escrevemos o launcher, o pad virtual, os docs. Tempo de máquina esperando é tempo humano jogado fora.
**Por quê uma cópia da árvore do PS2Recomp para o jogo?** Para não "sujar" o código de terceiros (GPL) com 248 MB de um jogo específico, e para poder rodar outro jogo depois.
**Por quê `low_memory_mode` e poucas threads no recompilador?** Havia outras sessões usando o PC (um teste de FPS de outro jogo). Pedimos a vez antes de rodar algo pesado, para não distorcer a medição do colega. *Disciplina de recurso compartilhado.*
`[TELA]` Contador [N/1332] avançando.

### Quando o build "chega a 99%" e quebra
Chegamos a 1331/1332 e o linker caiu com **`Stack overflow`**, depois de ~1 hora de CPU. **O que aconteceu?**
- **Conceito — LTCG (Link-Time Code Generation):** com a flag `/GL`, o compilador não gera código de máquina na fase "compilar"; ele guarda uma forma intermediária e só gera o código **no link**, enxergando o programa inteiro (para inlinear e otimizar entre arquivos). Por isso a barra de progresso engana: os 1330 passos "rápidos" só preparam; o trabalho pesado é o último passo, **quase serial** e com ~9 GB de RAM.
- Com **248 MB de C++ gerado** e funções gigantes (algumas com milhares de linhas), o otimizador do compilador estourou a pilha.
- **Decisão:** não brigar com o LTCG. Criamos uma opção de CMake (`PS2X_ENABLE_LTCG`) e fizemos um build de **desempenho sem LTCG**.
- **Por quê?** (1) o ganho do LTCG não compensa a fragilidade; (2) sem ele, o compilador gera código **durante a fase paralela** (usa todos os núcleos) e o link fica rápido — **ciclos de otimização de minutos, não de horas**. Otimização é um loop de "mudar → medir"; se o loop é lento, você otimiza pouco.
- **Aprendizado:** *"o tempo do build faz parte do desempenho do projeto"*. Quando o ciclo é lento, mude o ciclo antes de mudar o código.

### Outro susto: logs ligados por padrão
O CMake do runtime vem com `PS2X_ENABLE_AGRESSIVE_LOGS=ON`, que insere em **cada função gerada** uma linha de log "entrei/saí" **com flush em disco**. Com 39 mil funções isso é um freio de mão puxado.
- **Por quê importa?** Medir FPS com isso ligado é medir o disco, não o jogo. Para **medir**, desligamos (`AGRESSIVE_LOGS` e `RUNTIME_LOGS` = OFF). Para **diagnosticar** (ver até onde o boot vai), um build com logs vira um segundo perfil, separado.
- **Aprendizado:** *defaults de projetos de terceiros são pensados para depurar, não para medir.* Leia o CMake antes de medir qualquer coisa.

### Mexer em código de terceiros sem bagunçar: patches
Precisamos alterar o runtime (GPL) — pad virtual, FPS configurável. **Decisão:** guardar as mudanças como **arquivos novos + um `.patch`** em `ghost_pc/patches/`, e manter o clone original intocado em `third_party/`.
- **Por quê?** A árvore de build é descartável (248 MB de código gerado); os patches são pequenos, versionáveis, **reaplicáveis em outra versão** do projeto e deixam claro o que é nosso (importante por causa da licença GPL). Testamos o patch com `git apply --check` no original e na árvore de trabalho.

---

## 9. Controle e desempenho: pad virtual e metas de FPS (31:00–36:00)
**Conceito — pad virtual:** em vez de depender de mim apertando botões, um **script de entrada** alimenta o "controle" do jogo (o runtime já tem um *override* de pad). Assim o teste é **repetível**: o mesmo caminho de menus, o mesmo ponto de medição.
**Por quê?** Otimização sem medição é chute. Medição sem repetição é ruído.
**O que implementamos:** um script de entrada **por quadro** (`quadro  BOTÕES  [analógicos]`, ex.: `300 START`, `3600 quit`). Usamos **quadros** e não segundos de propósito: com tempo, uma máquina mais lenta "pressiona o botão" em outro ponto do jogo; com quadros, o teste é o mesmo em qualquer velocidade. Escrevemos também um **teste do parser** (entradas válidas e inválidas) antes de usar.
**Por quê reaproveitar o *override* de pad que já existia no runtime?** Menos código nosso, menos bugs, e o caminho de entrada do jogo continua o real (o jogo "acha" que é um controle de verdade).

**FPS tem dois significados — esta é a parte didática:**
1. *FPS do host:* quantas vezes por segundo a janela do PC apresenta uma imagem (o runtime limitava em 60).
2. *Ritmo do jogo:* o jogo de PS2 é guiado pelo **vblank** (50 Hz em PAL). Se o jogo roda a 30 FPS "de verdade", ele espera 2 vblanks por quadro.
Medir só o primeiro engana: você pode apresentar 200 imagens por segundo e o jogo lógico estar travado. Por isso medimos o host (CSV por segundo, **1% low** e percentis de tempo de quadro — a média esconde os engasgos) e, na sequência, o ritmo do jogo.

**Como a IA "vê" o jogo: capturas pelo próprio runtime.** Em vez de fotografar a tela do desktop, o script do pad virtual ganhou o comando `quadro shot nome`, que salva o **quadro nativo do jogo** (640×448) como PNG. **Por quê?** É repetível (mesmo quadro, mesma resolução, independente do tamanho da janela ou do monitor) e permite que o assistente **leia a imagem** e decida o próximo passo sozinho — o loop "rodar → olhar → corrigir" fica automático. Cuidado de segurança: o nome do arquivo só aceita letras, números, `_` e `-` (sem `../`), porque um script de teste nunca deve escrever fora da pasta de capturas.

**Primeira execução de verdade (e o primeiro "tela magenta"):** o runtime abriu, rodou a ~60 FPS de host, os quadros saíram **magenta puro** e o log mostrou `guest-branch:missing-target`. Lição didática: *um FPS alto não significa nada se a imagem é um quadro vazio* — por isso medimos **os dois** relógios e **sempre** olhamos a imagem. O "100% de tempo real" do vsync era enganoso: o vsync é cadenciado pelo relógio do **host**, não pelo trabalho do jogo.
**Causa raiz:** uma chamada indireta (`JALR`) para `0x3067c8`, uma função vazia (`jr ra; nop`) que o Ghidra nunca criou porque ninguém a **chama diretamente** (só aparece como ponteiro de função em dados). Sem função gerada, o recompilador não tem o que chamar.
**Correção geral (não um remendo por endereço):** um script que varre os **vãos do `.text` não cobertos por nenhuma função** (114 KB, ~5% do código) e cria funções ali — de 7.462 para 8.890 funções. **Por quê geral?** Corrigir um endereço por vez custaria um rebuild de ~14 minutos por tentativa; cobrir a classe inteira de código alcançável só por ponteiro evita dezenas de ciclos.

**Metas:** 30 FPS (o mínimo jogável), 60 FPS (o ideal) e "ilimitado" (para medir o custo bruto). O limite virou configurável (`--fps 30|60|0`).
**Como vamos otimizar (ordem):** 1) medir (baseline); 2) achar o gargalo com perfilador; 3) corrigir o maior; 4) medir de novo. *Nunca otimizar o que não foi medido.*
**Resultado:** `[PREENCHER — FPS medido, gargalos, o que ajudou]`.

---

## 10. Trabalhando com IA e agentes (36:00–39:00) *[opcional, ajuste ao seu estilo]*
- **Divisão de trabalho:** tarefas mecânicas → modelo pequeno (Haiku); engenharia padrão → Sonnet; raciocínio difícil (compressão, arquitetura) → Opus. **Por quê?** Custo e qualidade: não gastar o modelo mais caro em tarefa de grep.
- **Verificar sempre:** um agente afirmou algo sobre a engine; **não confirmei, e registrei como não confirmado**. Outro corrompeu a codificação de um arquivo (eu mesmo!) — a lição virou regra no projeto (sempre UTF-8 explícito).
- **Regras escritas valem mais que boa memória:** tudo vive no `AGENTS.md`.

---

## 11. Recap e o que aprendemos (39:00–41:00)
1. **Organize antes de codar** (regras, legalidade, validação do dump).
2. **A ferramenta precisa entender a CPU** (R5900/MMI) — senão tudo que vem depois é ruído.
3. **Leia o código do próprio programa** para descobrir formatos e compressões (V3a = LHA lh5).
4. **Valide sempre** (74/74 entradas; imagens que aparecem certas).
5. **Escolha a estratégia pelo risco e pelo tempo** (recompilar para testar rápido; decompilar só o necessário).
6. **Seja honesto sobre o que ainda não funciona.**

## 12. Próximos passos / chamada (41:00–42:00)
"No próximo vídeo: `[PREENCHER]` (o boot do jogo, o primeiro quadro na tela, ou onde travou). Se você quer ver outro jogo passando por esse pipeline, comenta qual."

---

## Apêndice A — Linha do tempo das descobertas (para a edição)
| Quando | Descoberta | Por que importou |
|---|---|---|
| Início | `BOOT2 = SLES_530.20`, PAL | Define o alvo e o hash do dump |
| Análise 1 | `entry` virou lixo | MIPS genérico não decodifica MMI |
| Após extensão | crt0 legível, 7.462 funções | Base confiável para tudo |
| IOP | 9 módulos, ordem de carga, imagem 2.8.0 | Define o que emular |
| `fpk` | contêiner comum a todos os `.BIN` | Destrava os dados |
| `V3a` | LHA lh5, 74/74 | Descompressão dos gráficos |
| `zim` | 10.445 texturas legíveis | Prova do decodificador |
| Recompilador | 67 s, 248 MB, 4.520 avisos (só saltos) | Estratégia viável (tradução) |
| Build 1 (LTCG + logs) | Link falhou: `Stack overflow` após ~1 h de CPU | O link é onde o compilador trabalha de verdade |
| Config do runtime | `AGRESSIVE_LOGS=ON` por padrão | Logs por função inviabilizam medir FPS |
| Build 2 (sem LTCG, sem logs) | `[PREENCHER]` | Base para medir e otimizar |

## Apêndice B — Glossário rápido (cards na tela)
**ELF** · **R5900/EE** · **MMI** · **IOP / IRX** · **ISO9660** · **crt0** · **syscall** · **stripped** · **RTTI** · **LZSS/Huffman** · **GS (Graphics Synthesizer)** · **VU (Vector Unit)** · **Recompilação estática** · **Spike** · **Matching decomp**

## Apêndice C — Erros e acertos para citar (a parte mais didática)
- ❌ Offsets do `fpk` absolutos → ✅ relativos ao `data_start` (validar com todos os arquivos).
- ❌ "`pth` começa com `pBAX`" (premissa de um agente) → ✅ `pBAX` é banco de sons; `pth` é outra coisa.
- ❌ Engine "Sega/Production I.G." (não confirmado) → ✅ Cavia como desenvolvedora (fonte checada).
- ❌ Mojibake ao reescrever um doc com a codificação errada → ✅ regra: UTF-8 explícito.
- ❌ Timeout de 10 min matando o build longo → ✅ rodar desacoplado, com log e monitor.
- ❌ Pensar que "recompilou" = "roda" → ✅ separar tradução de execução.
- ❌ "O Defender está bloqueando" → ✅ era o **Smart App Control**: recurso à parte do antivírus (exclusões do Defender não valem), que bloqueia executáveis sem assinatura/reputação. Diagnóstico por **evidência** (log do Code Integrity, ids 3077/3033; registro `VerifiedAndReputablePolicyState`) antes de mexer em qualquer coisa — e a decisão de desligar ficou com o dono da máquina.
- ❌ Achar que "sessão ociosa" = "máquina livre" → ✅ outro agente esperava o usuário fechar um jogo (teste de FPS) com o lock ainda dele; rodei um job de 68 s no meio. Consertar não foi só pedir desculpa: virou **regra escrita** (esperar a liberação explícita) e o relato do que rodou para o outro tratar a medição como suspeita. *Recurso compartilhado exige protocolo, não suposição.*
- ❌ Considerar "59,8 FPS e 99,97% em tempo real" um sucesso → ✅ a imagem era magenta: o jogo nem tinha começado. Métrica sem olhar o resultado engana.
- ❌ Confiar na barra "1330/1332" → ✅ o último passo (link com LTCG) era 99% do trabalho e falhou com `Stack overflow`.
- ❌ Medir com os logs padrão do runtime ligados → ✅ perfil de medição separado (logs e LTCG desligados).
- ❌ Esperar o build "acabar" parado → ✅ escrever pad virtual, medidor de FPS, patches, launcher e roteiro enquanto compila (regra 9).
