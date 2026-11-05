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

**Segunda rodada: o que o recompilador+runtime já fazem, e onde o jogo trava.**
1. *Estouro de pilha (`0xC00000FD`):* ao incluir o código dos vãos, o jogo caiu em 4 segundos. **Por quê?** No código recompilado, **cada chamada do jogo é uma chamada C++ aninhada**; uma cadeia de chamadas profunda gasta a pilha nativa, que no Windows tem 1 MB por padrão. **Correção:** reservar 256 MB no link (`/STACK`) — só reserva endereços, as páginas são alocadas sob demanda — e vale para todas as threads criadas com tamanho padrão. Só pedia **relink**, não recompilar. *Lição: depois de "achar o crash", entenda se é bug ou limite de ambiente.*
2. *O boot do IOP funcionou:* o log mostra os 9 módulos carregados **na ordem exata que descobrimos por engenharia reversa** (sio2man → … → ezsound), com as mensagens originais dos módulos ("SDR driver version 4.0.1", "cavia EzSound driver version 2.0"). É a melhor confirmação de que o trabalho anterior estava certo.
3. *Batimento (`PS2X_HEARTBEAT`):* uma linha por segundo com o **PC** do jogo, DMA/GIF/GS. **Por quê?** O trace de funções gigante deixaria tudo lento e ilegível; uma amostra por segundo basta para responder "o jogo está preso, desenhando ou ocioso?". Resultado: **PC fixo em `0x2d2d38` por 60 s, zero DMA/GIF** → preso num laço de espera.
4. *Causa:* o PC está dentro do `fioInit`, que fica em loop chamando `sceSifBindRpc` para o serviço de arquivos do IOP (RPC `0x80000001`) — que **nunca responde**.
5. **Como o runtime foi desenhado (a lição de arquitetura):** ele **não emula o IOP inteiro**; ele **liga as funções do SDK por nome** (`fioOpen`, `scePadInit`, `sceCdRead`…) a implementações no host. Se a função do jogo não tem o nome do SDK, ela é recompilada como código real e fica esperando um hardware que não existe. Ou seja: **dar nomes às funções do SDK não é cosmético — é o que decide se uma função é emulada ou "executada de verdade".** Por isso o trabalho agora é um mapeamento sistemático: lista de ~700 nomes do runtime ↔ funções do jogo, feita por agentes em paralelo (um para a API de arquivos, outro para pad/memory card/DMA/GS/etc.).

**Terceira rodada: o jogo "acorda".**
1. *Pesquisar antes de reinventar:* um agente buscou na internet como o hardware do PS2 sinaliza o fim de um DMA e como outros projetos de recompilação lidam com isso. Resultado: **existe um PR aberto no PS2Recomp descrevendo exatamente o nosso travamento** (outro jogo, mesma sequência). Isso transformou "não sei por que trava" em "já sei o padrão". *Lição de método: pesquisar o sintoma em issues e PRs é barato e muitas vezes mostra que o problema já foi diagnosticado.*
2. *O bug, em português simples:* a biblioteca gráfica do jogo manda o DMA, **depois** registra quem quer ser avisado quando ele terminar, e depois fica esperando esse aviso. Se o DMA termina antes do registro, o aviso se perde para sempre. **Correção:** em vez de descartar um aviso que ninguém quis ainda, **guardá-lo** (um "latch") e entregá-lo quando alguém se registrar — o comportamento de uma linha de interrupção por *nível* no hardware real. Mais: entregar avisos pendentes a cada VBlank.
3. *Medir para confirmar:* contadores baratos no batimento (`registered`, `dispatch`, `latched`, `redelivered`, `queued`) mostraram a correção funcionando: o aviso foi guardado e entregue. **Sem esses números, eu só teria "parece que melhorou".**
4. *O primeiro sinal real:* o jogo configurou o **display** (`dispfb1`, `display1`) e a tela deixou de ser magenta para **preta** — o framebuffer do próprio jogo, ainda limpo. Parece pouco, mas é a troca de "o emulador não mostra nada" para "o jogo está mostrando o que desenhou".
5. *A métrica de desempenho aparece:* com o jogo executando de verdade, o relógio emulado do jogo ficou em **38 Hz em vez de 50 (76% do tempo real)** — o código recompilado é mais lento que o PS2 real. É exatamente o número a otimizar para chegar a 100% (e depois 60 FPS no host).
6. *Um ciclo de build que não perde tempo:* descobrimos que recopiar todos os 41 mil arquivos gerados renovava os carimbos de data e forçava uma recompilação de 14 minutos **a cada** ciclo. Agora a ferramenta copia só o que mudou — o ninja vira incremental de verdade. *Lição: ferramentas de build só são rápidas se você não as engana com timestamps.*

**Quarta rodada: o primeiro quadro, e onde o tempo realmente vai.**
1. **A primeira tela real** — o menu de seleção de idioma — apareceu quando paramos de mexer no jogo e demos os arquivos que ele pedia: ele procurava os **vídeos** no disco (que eu tinha deixado de fora para economizar espaço), falhava e ficava relendo o setor 0 em laço (1.702 avisos no log). *Lição: um laço de "tentar de novo" pode parecer um travamento misterioso; o log com contagem de mensagens repetidas revelou na hora.* `[TELA]` mostrar o PNG do menu.
2. **O controle virtual funciona, mas o menu não reage.** Provamos com um diagnóstico (`[hb-pad]`): o botão chega ao jogo, e a máquina de estados do próprio jogo para configurar o controle terminou (lemos a RAM do jogo com `PS2X_PEEK`). Então o problema é do outro lado. *Lição: separar "o dado chega?" de "o jogo reage?" corta o espaço de busca pela metade.* (Ainda em investigação.)
3. **O gargalo de desempenho não era o que eu supunha.** Eu esperava que o código recompilado fosse lento. Medimos: **uma única thread a 100% de um núcleo (de 16)** — a que executa o jogo. Sem profiler instalado, **escrevemos um amostrador de 80 linhas**: ele pausa a thread a cada milissegundo, anota onde ela está, e um script converte os endereços em nomes usando o arquivo de mapa do linker (sem PDB). Resultado: **98,7% do tempo está no desenhista de pixels por software (o emulador da placa gráfica), só 0,4% no código do jogo.** *Lição central do vídeo: "medir antes de otimizar" não é frase de efeito — eu teria gasto horas otimizando o lugar errado.*
4. **Por que tão lento?** O PS2 tinha um chip gráfico dedicado (GS) que desenha milhões de pixels por segundo; aqui, a CPU faz isso pixel a pixel, na **mesma thread** que executa o jogo. Cada função aparece no perfil por um motivo (escrever o pixel, amostrar a textura, interpolar a cor). A otimização é trabalho de engenharia clássica: tirar do laço o que não muda por pixel, criar caminhos rápidos para os casos comuns, e — se for seguro — repartir o desenho entre várias threads. **Regra de ouro dessa fase: a imagem de saída tem de continuar bit a bit igual** (comparamos o MD5 da captura do menu antes e depois de cada mudança).

**Quinta rodada: o controle funciona e o desenho fica 2× mais rápido.**
1. **Por que o menu ignorava o controle (a história de detetive):** (a) provamos que o botão chegava ao jogo (diagnóstico `[hb-pad]`); (b) decompilamos a rotina de entrada do jogo e achamos uma linha decisiva — ela só aceita os dados do controle se o byte de identificação for **`'s'`**, o código do **DualShock em modo analógico**; com qualquer outro valor, zera todos os botões; (c) o jogo tenta colocar o controle nesse modo procurando o modo "7" numa tabela que o runtime preenchia com **um único modo (digital)**; (d) conserto: anunciar dois modos (digital e DualShock), como o controle real. **Uma linha de decisão, três camadas de diagnóstico para encontrá-la.** Resultado: a captura depois do `CROSS` mudou — o jogo agora reage ao gamepad virtual. *Lição: quando "não funciona", cada hipótese precisa de um jeito barato de ser confirmada ou descartada; sem as ferramentas de diagnóstico, isso seriam dias.*
2. **O que o agente de otimização fez (e por que cada coisa ajuda):** o rasterizador recalculava, **para cada pixel**, coisas que só mudam por figura (formato do framebuffer, modo de teste, de mistura, de textura). Foi tudo movido para fora do laço. O filtro bilinear passou a calcular os 4 canais de cor de uma vez com instruções vetoriais (SIMD), na **mesma ordem de operações** do código escalar para dar resultados idênticos bit a bit. O cache de textura deixou de copiar 8 KB a cada falha. **Resultado: o menu foi de 21 Hz para ~40 Hz (42% → 81% do tempo real).**
3. **O que ele NÃO fez, e por quê:** dividir o desenho entre várias threads. O cache de textura é sequencial e algumas figuras leem o próprio framebuffer enquanto escrevem; paralelizar sem provar que o resultado é idêntico trocaria correção por velocidade. **Regra do projeto: nunca trocar fidelidade por FPS sem prova.**
4. **O medidor mostrou o outro lado da moeda:** na cena seguinte (mais leve) o jogo chega a **50,00 Hz (99,99% do tempo real) e 52 FPS no host**. Ou seja: o desempenho depende da cena — o menu é pesado porque desenha várias imagens grandes com mistura; as cenas simples já estão no alvo.

**Sexta rodada: antecipar uma classe inteira de erros.**
1. *O mesmo erro, pela terceira vez:* o jogo parou de novo por um "alvo ausente" (uma função que ninguém chama diretamente, só por ponteiro). Cada correção individual exigia um rebuild de ~14 minutos. **Em vez de consertar o 3º caso, escrevemos um varredor para a classe inteira:** ele lê todas as palavras de 32 bits das áreas de dados e todas as constantes de endereço do código e acha tudo que aponta para o meio do código fora de qualquer função. Resultado: **349 funções novas de uma vez**, incluindo a que travou. *Lição: depois de ver o mesmo padrão três vezes, pare de remendar e resolva o padrão.*
2. *O jogo passou do ponto e mostrou a abertura* — um fundo hexagonal animado, com ruído ciano e laranja. `[TELA]` captura da cena.
3. **Desempenho medido cena por cena (e por que é importante):** boot e menu ~53% do tempo real; telas intermediárias 61–68%; **a cena hexagonal, 46%** (22,9 Hz de 50). Mesmo o jogo "rodando", a velocidade depende do que está na tela — preencher a tela inteira com uma textura misturada em software é a operação mais cara do rasterizador. *Lição: um benchmark só no menu esconderia o pior caso.*

**Fechamento da primeira sessão de trabalho (o final feliz parcial).**
1. **A tela de título apareceu**: "GHOST IN THE SHELL — STAND ALONE COMPLEX", com os créditos, desenhada pelo port a partir do disco do próprio dono do jogo; depois a abertura hexagonal. `[TELA]` as duas capturas. Tudo isso **guiado por um controle virtual programado**, sem ninguém apertar botão.
2. **Um experimento que deu errado, e por que mostrar isso no vídeo:** ligamos a otimização de programa inteiro (LTCG) só na biblioteca do runtime, esperando uns 10%. Compilou, a imagem saiu idêntica, mas o ganho foi **zero** (34,9 e 34,3 Hz contra 34,6). Revertemos. *Lição: um resultado negativo medido é conhecimento; "deve ajudar" não é.*
3. **Balanço honesto:** o jogo **roda e é jogável até a abertura**; o menu está em ~70–80% do tempo real (era 38%), e a cena mais pesada em 46%. Falta: o rasterizador ainda é o gargalo; o som não foi tratado; e o jogo de verdade (depois da abertura) não foi medido. **O que mais ajudou nas 6 horas** não foi uma ideia genial, e sim **um ciclo curto de "mudar → rodar → olhar a imagem → medir"** e ferramentas de diagnóstico baratas escritas na hora.

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
| GS GPU etapa 1 | Runtime apresenta via raylib/OpenGL; RHI D3D11 isolada: 640x448 em ~0,08 ms (CPU) / ~0,23 ms (GPU), readback idêntico | Apresentar na GPU é barato; o custo está em rasterizar |
| Janela do launcher | `ps2launch-gui` (egui, feature `gui`), `launcher.cfg` com flag > cfg > padrão, arte local opcional | Launcher utilizável por mouse, teclado e controle sem pesar o CLI |
| Imagem torta no D3D11 | Buffer 640x512 com stride fixo; a largura lógica era tratada como pitch | Olhar a imagem pegou o que nenhuma métrica pegava |
| Faixas pretas | Faixas de 63,5 px truncadas para 63; regra `ceil` confirmada no PCSX2 | Um pixel faltando ensina a regra de cobertura do GS |
| Tela preta pós-abertura | Heap do IOP fixo em `0x120000`: alocação de 162 KB retornava 0 e deixava o carregador pausado | Primeira causa provada com Ghidra + log |
| Vídeo aparece | `GetPicture` chama o callback Background antes de esperar: quadros 640x336 na tela | Primeira imagem depois da abertura; ainda sem som |
| Um pixel derruba o jogo | Pools do runtime em `0x1F00000`, dentro da memória de vídeo do jogo; o pixel `0x80363e3b` virou ponteiro | Corrupção de memória provada com vigia + PCSX2 |
| Dentro da história | Título, cutscenes e legendas no port | Primeira vez além da abertura |
| Um arredondamento travou o jogo | `cvt.w.s` arredondava; o R5900 trunca. Comb sort com intervalo preso em 2 | Bug silencioso herdado do recompilador |
| Flags que ninguém lia | MAC do VU0 nunca atualizada; culling não descartava nada | Da tela travada para cena 3D rodando, lenta |
| O registrador que não pode mudar | VF0 do VU0 é constante; o `vsub` para flags o sobrescrevia | Câmera bate com o PCSX2; o mundo ainda sai vermelho |
| A cópia que nunca aconteceu | DMA do scratchpad recusava a memória do VU0 como origem; vértices vinham de dado velho | O prédio aparece, igual à referência |
| O vermelho era uma soma | `PMULTH` somava oito produtos em vez de mantê-los separados; o filtro de cor perdia verde e azul | Céu e prédio no azul da referência |
| A cor que virava zero | `MAX` do VU comparava como float e zerava cores guardadas como inteiro; o hardware compara bits | Cenário inteiro na tela |
| Um ponto só | VIF0 aceitava só UNPACK de 32 bits; vértices da personagem nunca chegavam ao VU0 | A Motoko aparece na cena |
| Gameplay renderiza (04/11/2026) | Caixa de diálogo funciona ("Aramaki here", X avança); cena 3D a ~3 vsync/s | O jogo roda, mas 17x abaixo do alvo de 50 Hz |
| Perfil da cena 3D | Rasterizador GS em software ~52%, interpretador VU1 ~32%, IOP ~5%, guest recompilado ~2,5% | O código da CPU do jogo não é o problema; os dois chips emulados literalmente são |
| 5 agentes em paralelo | VU1 3,0-3,3x (saída idêntica bit a bit); rasterizador CPU 2,04x (818.934 comandos GS idênticos); fila cpu-thread de ~400 mil esperas/10 s para ~280; GPU com VRAM inteira e ROV, PSNR 51-55 dB, ~10% mais rápido | Ganhos reais e verificados, mas pequenos frente à lacuna |
| Medição final, máquina parada (05/11) | Mesma janela (vsync 7530-7640): baseline 2,96 Hz; build novo CPU 5,39 Hz; d3d11 6,87 Hz; alvo 50 Hz | Dobrou, mas fica a 7x+ do alvo: ajuste fino não fecha a conta |
| Decisão de rota | Abandonar "byte-idêntico" no caminho rápido: VU1 nativo (estilo microVU), GS por hardware em D3D11, threads EE/VU1/GS, e antes reduzir o custo do rebuild | Troca de referência exata e lenta por implementação rápida e aproximada |
| Rebuild barato (05/11) | Prelúdio leve isola o runner; editar `ps2_vu1.h` + `gs_frontend` de ~12 min para 34,6 s (42 passos, 0 unidades do runner) | Cada experimento seguinte deixou de custar 12 min |
| VU1 nativo | ~28 ns/ciclo contra ~113 ns (~4x) no harness; VU de 33,7% para ~6% das amostras; bit a bit idêntico em ~15 mil chamadas reais + ~90 mil sintéticas | Hazards resolvidos na tradução, não por ciclo; JIT x86-64 não compensa |
| GS por hardware (`hw`) | Cena 3D 12,2 vsync/s (escala 1) e 11,0 (escala 4) contra baseline 3,0; PSNR > 30 dB; só o brilho do holofote difere | O GS deixou de ser o gargalo; o quadro agora depende da thread do jogo |
| Filtro CRT | Shader só na apresentação; `shot` idêntico com o filtro ligado | Gosto do jogador sem contaminar o oráculo |
| Cache de arquivos em RAM | 180,9 s contra 179,4 s (~0,8%), espera de leitura 67 ms | Resultado nulo honesto: o SSD nunca foi o gargalo |
| Threads, etapas 0-2 | Quase nenhuma sincronização na cena 3D (1 leitura de CSR por quadro); determinismo provado por hash do stream do GS | Viável, mas com VU1 em ~6% o plano foi reordenado: GS primeiro |

## Apêndice B — Glossário rápido (cards na tela)
**ELF** · **R5900/EE** · **MMI** · **IOP / IRX** · **ISO9660** · **crt0** · **syscall** · **stripped** · **RTTI** · **LZSS/Huffman** · **GS (Graphics Synthesizer)** · **VU (Vector Unit)** · **Recompilação estática** · **Spike** · **Matching decomp**

## Apêndice C — Erros e acertos para citar (a parte mais didática)
- ❌ Offsets do `fpk` absolutos → ✅ relativos ao `data_start` (validar com todos os arquivos).
- ❌ "`pth` começa com `pBAX`" (premissa de um agente) → ✅ `pBAX` é banco de sons; `pth` é outra coisa.
- ❌ Engine "Sega/Production I.G." (não confirmado) → ✅ Cavia como desenvolvedora (fonte checada).
- ❌ Mojibake ao reescrever um doc com a codificação errada → ✅ regra: UTF-8 explícito.
- ❌ Timeout de 10 min matando o build longo → ✅ rodar desacoplado, com log e monitor.
- ❌ Pensar que "recompilou" = "roda" → ✅ separar tradução de execução.
- ❌ "Corrigi o alvo ausente, agora vai" → ✅ cada correção revela a próxima barreira (alvo ausente → estouro de pilha → laço no `fioInit`). **Ter ferramentas de diagnóstico baratas (batimento, registro de alvos, capturas) vale mais que adivinhar.**
- ❌ "O Defender está bloqueando" → ✅ era o **Smart App Control**: recurso à parte do antivírus (exclusões do Defender não valem), que bloqueia executáveis sem assinatura/reputação. Diagnóstico por **evidência** (log do Code Integrity, ids 3077/3033; registro `VerifiedAndReputablePolicyState`) antes de mexer em qualquer coisa — e a decisão de desligar ficou com o dono da máquina.
- ❌ Achar que "sessão ociosa" = "máquina livre" → ✅ outro agente esperava o usuário fechar um jogo (teste de FPS) com o lock ainda dele; rodei um job de 68 s no meio. Consertar não foi só pedir desculpa: virou **regra escrita** (esperar a liberação explícita) e o relato do que rodou para o outro tratar a medição como suspeita. *Recurso compartilhado exige protocolo, não suposição.*
- ❌ Considerar "59,8 FPS e 99,97% em tempo real" um sucesso → ✅ a imagem era magenta: o jogo nem tinha começado. Métrica sem olhar o resultado engana.
- ❌ Confiar na barra "1330/1332" → ✅ o último passo (link com LTCG) era 99% do trabalho e falhou com `Stack overflow`.
- ❌ Medir com os logs padrão do runtime ligados → ✅ perfil de medição separado (logs e LTCG desligados).
- ❌ Começar o backend de GPU desenhando triângulos → ✅ primeiro um backend que só **apresenta** o quadro da CPU pela GPU: prova a camada nova sem poder mudar a imagem, e testado num programa isolado em vez do build de 14 min.
- ❌ Na janela, a cor do selo "experimental" saía cinza → ✅ no egui a cor gravada no texto já formatado (galley) vence a cor passada na hora de desenhar; montar o texto já com a cor final. Só a captura de tela mostrou isso.
- ❌ Esperar o build "acabar" parado → ✅ escrever pad virtual, medidor de FPS, patches, launcher e roteiro enquanto compila (regra 9).
- ❌ "O travamento é intermitente: rodei 9 vezes e não reproduziu" → ✅ as rodadas nem chegavam ao ponto do problema. Teste que não alcança o cenário não prova nada.
- ❌ "Está preso esperando um semáforo" → ✅ era a espera normal de vsync; o `pc` do heartbeat só mostrava quem foi dormir. Faltava ver todas as threads e quem sinalizava.
- ❌ Rodar o jogo a cada hipótese → ✅ o usuário cobrou, com razão. Ler o código no Ghidra respondeu em minutos o que horas de tentativa não responderam. Executar é para confirmar, não para descobrir.
- ❌ Compilar para medir e esquecer que esse build descarta os logs → ✅ traces ligados em tempo de execução, sem mudar o timing do jogo inteiro.
- ❌ Tirar os botões do script para "ver melhor" a cena → ✅ a cena só existia com botões, e gastei 10 minutos numa rodada que nem chegou lá. Mudar uma coisa só de cada vez.
- ❌ "`0x364680` não é uma task, a lista está corrompida" → ✅ no PCSX2 era task legítima. Eu afirmei sem referência; bastou ler a memória do jogo real para derrubar a hipótese.
- ❌ Confiar no watchpoint de hardware porque "armou" → ✅ não disparou nem em escrita que eu sabia existir. Ferramenta de diagnóstico também precisa ser testada; um vigia simples em laço pegou o culpado.
- ❌ Empolgar com a causa e dizer "resolvido" → ✅ a correção passou de um muro e a tela continuou preta. Dizer o que mudou de verdade para quem está olhando.
- ❌ Dois ninjas diferentes no mesmo build (um agente usou o do WinGet 1.13.2, os outros o do Visual Studio 1.12.1) → ✅ cada versão descarta o `.ninja_log` da outra e forçou recompilar ~41,8 mil arquivos. Ferramenta de build é parte do ambiente: fixar uma versão e um lock de build para todos.
- ❌ Culpar o rasterizador ou o VU1 por uma regressão de imagem na cena 3D → ✅ dois agentes provaram (A/B com o rasterizador original no mesmo exe, mais replay) que era uma correção de GIFtags incompletas do frontend, que muda o que é desenhado; ficou atrás de `PS2X_GS_GIF_STATE=1`. Lição: bisseção por troca de arquivos, replay de streams e medir antes de culpar.
- ❌ Confiar na medição parcial com a máquina carregada por 5 agentes → ✅ os números do ninja vieram inflados de 2,5x a 10x. Medição final só com a máquina parada e na mesma janela do jogo.
- ❌ Achar que "fiel" implica "rápido" (backend de GPU que emula cada pixel no shader) → ✅ visualmente equivalente (PSNR 51-55 dB), mas só ~10% mais rápido: o gargalo passou para a thread do guest.
- ❌ Fazer um plano de ganhos de 2-4x quando a lacuna era de ~15-20x → ✅ eu deveria ter dito no início que ajuste fino não fecharia a conta. Dizer a ordem de grandeza antes de planejar.
- ❌ "Hardware 30-50x melhor e roda a 5 FPS? O emulador roda bem" → ✅ comparação enganosa; ver Rodada 9. Implementação de referência não é implementação rápida.
- ❌ Editar um header e esperar 12 minutos como se fosse natural → ✅ o PCH incluía `ps2_runtime.h` e tornava 41,8 mil arquivos dependentes; um prelúdio leve levou o mesmo teste a 34,6 s. Baratear o ciclo de feedback vem antes de otimizar.
- ❌ Rodar o cmake com FetchContent online e interromper → ✅ apagou o raylib e gerou um `build.ninja` sem include; desde então `FETCHCONTENT_FULLY_DISCONNECTED=ON`.
- ❌ Achar que um VU1 "aproximado" seria o preço da velocidade → ✅ resolver hazards na tradução deu ~4x e ainda saiu bit a bit idêntico; só valeu porque o interpretador antigo ficou como oráculo de replay.
- ❌ Header do backend alterado e dependente não recompilado → ✅ `make_unique` com o tamanho antigo corrompeu o heap (`0xC0000374`); ao mudar o header, tocar os `.cpp` dependentes antes de buildar.
- ❌ "O cache de arquivos vai acelerar o carregamento" → ✅ 180,9 s contra 179,4 s: nenhum ganho nesta máquina com SSD. Contar o resultado nulo.
- ❌ Dizer qual dos dois brilhos de holofote está certo → ✅ não sei qual é o comportamento do GS real; fica como pendência em vez de chute.
- ❌ Primeira prova de determinismo falhou → ✅ a escolha do próximo prazo comparava instantes do host; comparar em ciclos deu hashes idênticos.

## Rodada 7: licença, arquitetura e o plano para GPU, launcher e lojas

**Por que falar de GPL agora.** Nosso runtime é derivado do PS2Recomp (GPL-3.0). A licença só pesa quando distribuímos, mas decidir cedo evita surpresa: `LICENSE` nos dois repos e um `LEGAL.md` com checklist de publicação (varrer *todo* o histórico, não só o último commit). [TELA: o checklist] Honestidade: não sou advogado; o status jurídico do C++ gerado a partir do ELF fica como pergunta aberta.

**Por que GPU, e por que D3D11 primeiro.** O perfil mostrou 98,7% do tempo no rasterizador por software; otimizar a CPU rende poucos por cento por passo. Li o renderer por hardware do PCSX2 só como referência de comportamento (GPL: não copio código). Lições: a VRAM de 4 MB e o swizzle ficam na CPU, a GPU tem alvos lineares com cache, o blend do PS2 muitas vezes precisa de shader, e quando a GPU não dá conta cai-se para software. Nosso encaixe é a interface `GSRasterBackend`. Ordem escolhida: D3D11 (menos código, Windows), depois Vulkan (Linux/Deck) e D3D12. [PREENCHER] resultados quando o backend desenhar na GPU.

**Primeiro passo na GPU: provar o cano antes de mandar água.** Antes de desenhar um triângulo sequer, a pergunta era "por onde a imagem sai hoje?". Resposta lendo o código: pela **raylib** (OpenGL), numa janela da thread principal; o jogo desenha em outra thread. Isso decidiu duas coisas: (1) o D3D11 ganhou **janela própria**, porque misturar OpenGL e DXGI na mesma janela é pedir problema; (2) o novo `GSGpuBackend` não desenha nada: ele **delega tudo** ao rasterizador da CPU e só manda o quadro pronto para a GPU como textura. Parece bobo, mas prova a camada nova (device, swapchain, shaders, upload, readback) com risco zero de mudar a imagem — e sem a variável `PS2X_GS=d3d11`, nada muda. [TELA: as duas janelas lado a lado, quando integrado]. Testei num programa à parte, com um gradiente gerado por código (nada do jogo): RTX 2060, feature level 11_1, upload + apresentação de 640x448 em ~0,08 ms de CPU e ~0,23 ms esperando a GPU; ida e volta da textura idêntica byte a byte. Ou seja: **apresentar na GPU custa quase nada; o caro é desenhar**, e isso é a próxima etapa. O risco anotado para ela: o D3D11 não aceita duas threads no mesmo contexto, e o nosso runtime desenha numa thread e apresenta em outra. [PREENCHER] integração no runtime e MD5 igual entre `cpu` e `d3d11`.

**Launcher e assets, o que aprendi com os launchers do AkitaOnRails.** Identificar a edição por hash e *recusar* em vez de adivinhar; nunca mexer no jogo original; extrair com manifesto, escrita atômica e retomada; rede só se o jogador pedir. O que *não* copio: o pacote HD derivado de arte do jogo (fere a minha regra de não distribuir assets). Nosso launcher hoje só valida o ELF e copia 2,9 GB crus; o plano lista as lacunas e as opções (extrair com manifesto agora, ler direto do ISO depois).

**A janela do launcher (egui).** Mesma ideia dos launchers do Akita, código nosso: arte do jogo à esquerda fundindo num painel azul-petróleo, abas Jogar / Imagem / Controles e um botão JOGAR inclinado. Três decisões com porquê: (1) **a GUI é um segundo binário atrás de uma feature do Cargo**, então o CLI continua com 3 dependências e ~0,5 MB, e quem não quer janela não paga por ela; (2) **a arte não vem no binário nem no repositório**: ela fica em `<jogo>/artwork/` (local, ignorada pelo git) e, se faltar, a janela desenha um degradê com o nome do jogo — a regra de não distribuir assets vale também para a capa; (3) **a janela só mostra opções que o runtime realmente tem** (`PS2X_GS`, `PS2X_TARGET_FPS`, `PS2X_GS_VSYNC`, `PS2X_HEARTBEAT`); Vulkan, D3D12, resolução e tela cheia aparecem desabilitados com "em breve", em vez de botões que não fazem nada. A verificação do dump e a extração rodam em threads, com barra de progresso, para a janela nunca congelar; o controle (gilrs) vira teclas Tab/Espaço para o foco do próprio egui fazer o trabalho. As preferências vão para um `launcher.cfg` de texto, que o CLI também lê, com flag > arquivo > padrão. Tamanho do exe e os testes: `[PREENCHER]` com o que for medido; ainda não testei com um controle de verdade nem num Steam Deck. [TELA: as três abas, capturas do teste]

**Steam, GOG e Deck.** Confirmado: o launcher precisa ser navegável por controle, texto >= 12 px a 1280x800, 30 fps a 800p, sem DRM na GOG, e o Smart App Control exige assinar o executável (já nos mordeu). Não confirmado: a política das lojas para ports que exigem os arquivos do usuário; isso fica registrado como pesquisa pendente.


## Rodada 8: a tela preta e como parar de chutar

**O sintoma enganava.** Depois da abertura, tudo preto, e o batimento mostrava o jogo sempre em `WaitSema`. Parecia travado. Não estava: o semáforo seguia sendo sinalizado, era só o laço esperando o próximo quadro. [TELA: as linhas `[hb-thr]`] O conceito: um contador de programa amostrado mostra onde o programa *dorme*, não o que ele está *fazendo*.

**Por que construir um profiler próprio.** O profiler que já tínhamos amostra o host, e 98% aparece como "rasterizador". Eu precisava da pergunta no nível do jogo: quais funções do guest rodam agora? Daí o histograma de chamadas por janela de tempo. Alternativa descartada: ligar os logs agressivos do runtime, que mudam o timing justamente do que eu queria observar. O risco aceito: uma janela de 10 s mistura fases, e isso me enganou uma vez, quando li uma descompressão de assets como se fosse a tela preta.

**A prova, em cadeia.** O profiler mostrou que a função do vídeo rodava uma vez por quadro. O Ghidra mostrou a máquina de estados dela. O `PS2X_PEEK` mostrou um flag `+0xD10` preso em 1: o carregador de arquivos pausado com um pedido na fila. E o log fechou: `sceSifAllocIopHeap(0x28800) -> 0x0`. O IOP tem 2 MB, e o heap do runtime começava fixo em 1,125 MB. [TELA: o log com o `0x0`]

**A correção é uma linha; o aprendizado é o caminho.** Baixei o início do heap para `0xA0000`. O valor é uma escolha nossa, não um número medido no hardware. Resultado honesto: o stream do vídeo inicia, mas a tela continua preta, porque o próximo muro é o `GetPicture` esperando dados que ninguém entrega. `[PREENCHER]` quando o vídeo aparecer.

**Trocar para Rust?** Pergunta boa, resposta não, por enquanto. Os bugs são de lógica e comportamento do PS2, não de memória; o código gerado continuaria sendo acesso bruto à RAM emulada; e perderíamos o runtime e as correções do PS2Recomp. Rust fica no launcher, onde já funciona.

**Pedir uma segunda opinião.** Quando empaquei, o usuário mandou chamar o Codex. Escrevi um briefing separando fato de hipótese e pedi que ele tentasse derrubar as minhas hipóteses. Ele confirmou com evidência no binário que a biblioteca real chama um callback quando falta dado, e corrigiu erros meus: eu dizia que faltavam memory card, limitador e controle real, e os três já existem no runtime.

**E a segunda opinião também precisa ser conferida.** O diff dele chamava o callback `Nodata`. Lendo a função no Ghidra, vi que ela programa DMA para o IPU real, que aqui não existe, e que um demux sem dados vira "fim do vídeo". Nosso HLE devolve zero quando a fila de 8 quadros enche, então o filme acabaria sozinho. Troquei pelo callback `Background`, que chama o mesmo demux sem esses efeitos. Alternativas descartadas: devolver quadro vazio (esconde o bug) e criar uma thread nossa de demux (mexe na ordem que o jogo espera). Risco aceito: o laço pode girar sem dados, então, se a chamada não trouxe nada, espera um vsync.

**O resultado.** Na mesma rodada, o FFmpeg decodificou quadros 640x336 e as cenas da abertura apareceram. [TELA: a mão com a munição, sem som, com faixa preta] Ainda sem áudio, e o que vem depois do vídeo: `[PREENCHER]`.

**Um pixel derrubou o jogo.** A tela sumia sempre no mesmo quadro do filme. Com a BIOS que o usuário forneceu, rodei o jogo no PCSX2 e li a memória dele pela interface PINE: o objeto que eu chamava de "lixo" era uma task legítima. Minha hipótese estava errada. Um vigia lendo a memória em laço pegou o momento exato, e o resto veio em cadeia: o `SifCallRpc` copiava 8 KB para `0x80363e3b`, valor lido de um descritor que o runtime guardava em `0x1F10000`. E esse valor é um pixel: R=3b, G=3e, B=36, alfa=80. O jogo usa os 32 MB inteiros, e o filme escreve seus quadros justamente ali. [TELA: o pixel ao lado do endereço] O conceito: um emulador pode esconder memória dele fora do console; um port que mora dentro da RAM do guest precisa provar que aquela área está livre. Alternativa descartada: aumentar a RAM do guest, que mexeria no mascaramento de endereço em todo o código gerado. Escolha: mover pools e pilhas para `0xA0000..0x100000`, abaixo do ELF, depois de ler no jogo real que essa área estava zerada. Risco aceito: foi amostragem na tela-título, não prova para o jogo inteiro.

**E então, a história.** Com esse conserto, o script de botões atravessou título e menus, e os vídeos da história apareceram com legenda. [TELA: "Shouldn't we put a leash on him..."] Gameplay: `[PREENCHER]`.

**Quando o bug é o próprio processador simulado.** A tela seguinte ficava parada, mas o profiler mostrava uma rotina sem chamar nada: um comb sort. Esse algoritmo divide o intervalo por 1,3 até chegar a 1. Só que `2 / 1,3 = 1,54`, e o recompilador arredondava para 2 em vez de truncar para 1. Nunca terminava. [TELA: a macro `nearbyintf` ao lado de `(s32)` no PCSX2] É o tipo de erro que nenhum compilador acusa: o programa está certo, a tradução da instrução é que mente.

**Por isso pedi uma auditoria, não mais uma tentativa.** O Codex comparou nossas instruções com o PCSX2 e achou uma lista: `VRSQRT` ignorando o numerador, `SQRT.S` lendo o registrador errado, `LQ/SQ` sem alinhar. Conferi no código antes de aplicar. Alternativa descartada: copiar o interpretador do PCSX2 inteiro, porque o gerador emite C++ por instrução e a estrutura é outra. Limite honesto: corrigir tudo isso não destravou o jogo sozinho.

**A flag que ninguém atualizava.** O que destravou foi ler o assembly da rotina presa: `cfc2 $t8, $vi17`, logo depois de um `vsub`. O jogo descarta objetos fora da tela olhando as flags do VU0, e o runtime nunca as escrevia. Sem descarte, percorre a cena inteira, e aquilo parecia um travamento. O conceito: num console, flags não são detalhe, são o jeito rápido de tomar decisão. Depois da correção o jogo seguiu para uma cena 3D, ainda muito lenta. `[PREENCHER]` o que é essa cena e o que o profiler mostrou.

**O registrador que não pode mudar.** Com as flags certas, a cena existia, mas a câmera vinha torta. Comparei a matriz com a do PCSX2 pela PINE, número a número, e a resposta estava na mesma instrução da descoberta anterior: o jogo faz `vsub` tendo VF0 como destino só para obter as flags. No hardware, VF0 é fixo em (0,0,0,1) e a escrita some; no nosso código, ela ficava. Por que corrigir no gerador, e não em cada instrução? Porque a regra vale para qualquer instrução que escreva em VF0, e o gerador é o único ponto que vê todas. O custo é regenerar e recompilar tudo. Depois disso, a translação da câmera ficou igual à do emulador: -152, 112,35 e 80,35.

**Um palpite que caiu em cinco minutos.** Sobrou o mundo vermelho. No dump de draws havia um sprite de tela inteira com essa cor, e parecia o culpado. Lendo o registrador TEST, ele deveria escrever só profundidade; lendo o rasterizador, ele respeita isso. Hipótese descartada sem rodar o jogo. Em vez de outro palpite, criei um rastreio que registra qual draw alterou um pixel escolhido. [TELA: log "after_draw=..."]

**A cópia que nunca aconteceu.** O rastreio mostrou uma pista curiosa: nos triângulos ruins, a coordenada de textura estava perfeita, mas posição e cor eram lixo. A rotina que monta o pacote lê as três coisas de lugares diferentes: o UV vem de uma tabela do modelo, e posição e cor vêm de uma tabela já transformada. Seguindo quem preenche essa tabela, cheguei a um truque elegante da Cavia: o microprograma do VU0 calcula os vértices na própria memória dele, um DMA copia o resultado para o scratchpad, e outro DMA leva para a RAM. O primeiro DMA parte de `0x110049E0`, a memória do VU0. O nosso runtime só aceitava RAM como origem: recusava a cópia em silêncio, e o segundo DMA levava para a RAM o que já estava no scratchpad. Por que a correção é pequena e a busca foi longa? Porque não havia erro nenhum: era dado velho com cara de dado válido. O código do PCSX2 confirmou que esse endereço é legítimo. [TELA: disassembly com `0x110049E0`] Resultado: na mesma execução o prédio da referência apareceu na tela, inteiro e no lugar certo, ainda todo vermelho e sem a Motoko. [TELA: lado a lado com o PCSX2]

**Por que tudo era vermelho.** O rastreio de pixels contou a história: o mundo era desenhado com cores normais, e só no fim um filtro de tela deixava apenas o canal vermelho. Esse filtro é uma matriz de cor em três passadas, uma para cada canal de origem. Os números entregaram a pista: 0x7f, 0xf9 e 0x32 estão na proporção 0,299 : 0,587 : 0,114, os pesos clássicos de luminância. A conta era certa, mas o resultado caía todo no vermelho. No Ghidra, a função é curta: multiplica cada canal pelo tom com a instrução `PMULTH`. Ela faz oito multiplicações independentes; nosso gerador somava tudo em um número só. Vermelho = soma dos tons; verde e azul = zero. Por que corrigir a família inteira, se o jogo só usa `PMULTH`, `PHMSBH` e `PMFHL.UW`? Porque o gerador é genérico e as vizinhas tinham o mesmo erro. Comparei cada uma com o PCSX2, em vez de confiar no nome das intrínsecas.

**Até a auditoria precisa de auditoria.** Pedi a um agente que comparasse as outras instruções MMI com o PCSX2. Ele achou `PEXEW`, `PEXEH` e `PROT3W` embaralhando as partes na ordem errada, e conferi cada uma antes de aceitar. Mas ele classificou `PSLLVW` como inofensiva, porque o disassembly mostra `$zero` no último operando. Ao decodificar os bits da instrução, vi o contrário: `$zero` é a contagem do deslocamento, e o valor é `$t2`. O certo é manter `$t2`; o nosso zerava o registrador, em funções de desenho do mundo. A lição: saída de agente é pista, não evidência. Resultado: o vermelho sumiu e a tela ficou no azul da referência, mas boa parte do cenário continuou preta.

**A cor que virava zero.** Primeiro achei que o problema fosse textura. O dump mostrou outra coisa: em quase todos os vértices, a cor saía do VU1 como `00000080`, embora os dados de cor chegassem certos. Segui o rastreio do microprograma, instrução por instrução. A cor ainda é um inteiro cru dentro de um registrador de ponto flutuante, e passa por um `MAX` com zero. Como float, 73 vira um número minúsculo, um denormal; o nosso processador zera esses números. O PS2 real não faz essa conta como float: o `MAX` compara os bits, e o PCSX2 implementa assim. O microprograma depende disso. Por que corrigir comparando bits, em vez de só desligar a zeragem de denormais? Porque essa regra é do `MAX` e do `MINI`, e o resto do VU tem outras regras para denormais.

**Corrigi e nada mudou.** Foi instrutivo: rodei e vieram exatamente os mesmos 10.437 vértices pretos. A hipótese estava certa, mas a correção não fazia efeito: antes de cada instrução, o interpretador já normalizava os operandos e zerava os denormais, então o meu `MAX` novo só recebia zeros. Lendo os registradores crus, os vértices sem cor nesse grupo foram a zero e o cenário apareceu inteiro: janelas, contêineres e refletor. [TELA: antes e depois lado a lado com o PCSX2]

**Onde a Motoko estava.** Ela ainda não aparecia. O dump mostrou que todos os 3.269 triângulos do outro grupo eram degenerados, com quase todos os vértices no mesmo ponto. Quando tudo cai em um ponto, a transformação pode até estar certa, mas as entradas estão iguais. O código de esqueleto envia os vértices ao VU0 por DMA em formatos compactos, como V3-16 e V4-8, e o nosso VIF0 só aceitava 32 bits. O resto era ignorado em silêncio. Escolhi fazer o VIF0 usar o mesmo código do VIF1, em vez de escrever uma segunda versão: é uma regra só, e um bug corrigido vale para os dois. O risco é quebrar o VIF1, que já funcionava, então a mesma execução testa os dois. Resultado: a Motoko apareceu, de costas no terraço, com o cenário atrás, muito parecida com a referência do PCSX2. Dos 5.101 triângulos desse grupo no dump, 5.036 já têm três pontos distintos. [TELA: captura do port ao lado da referência] Ainda faltam texto e retrato na caixa de diálogo, a velocidade (cerca de 0,3 vsync/s) e o som: `[PREENCHER]`.

## Rodada 9: o jogo roda, a 3 quadros por segundo, e por que isso mudou o rumo

**Onde estamos.** Em 04/11/2026 o jogo abre e a gameplay renderiza. A caixa de diálogo funciona: aparece "Aramaki here" e o X avança. [TELA: a caixa de diálogo, o jogador apertando X] Mas na cena 3D são cerca de 3 vsync por segundo. O alvo é 50. Não dá para jogar. Para o vídeo, é a hora de parar de consertar bugs de correção e perguntar: onde o tempo vai?

**Primeiro medir, depois opinar.** Perfilei a cena 3D no host, onde uma única thread faz tudo. O resultado: rasterizador de GS em software ~52%, interpretador do VU1 ~32%, IOP ~5% e o código do jogo recompilado só ~2,5%. [TELA: o gráfico de pizza do perfil] O conceito: o código da CPU do jogo, que era o que eu mais temia, não é o problema. Os dois problemas são os dois chips que emulamos de forma literal. Um VU1 interpretado gasta dezenas de ciclos do host para cada ciclo do original, e o GS, em software, tem que pintar pixel por pixel o que o hardware dedicado pintava de graça.

**Cinco agentes em paralelo, cada um com seus arquivos.** Rodei cinco agentes (opus e sonnet) ao mesmo tempo, com arquivos por dono e um lock de build, para que ninguém pisasse no trabalho do outro. Por que lock? Porque o build é o recurso mais caro e compartilhado; sem ele, dois agentes recompilam e se atrapalham. O que saiu, tudo verificado:
- **VU1**: 3,0 a 3,3x mais rápido. O teste foi um harness de replay com 15 mil chamadas reais mais 50 mil sintéticas, e a saída é idêntica bit a bit. Por que replay? Porque posso gravar a entrada uma vez e comparar a saída exata a cada otimização, sem rodar o jogo.
- **Rasterizador de CPU**: 2,04x, com replay de 818.934 comandos GS e resultado idêntico.
- **Fila do modo cpu-thread**: o produtor acordava a cada comando, ~400 mil esperas em 10 s. Passou para ~280, com a fila de 1 MiB para 32 MiB. O conceito é o custo de acordar uma thread: é caro, então se acorda em lote.
- **Backend de GPU "VRAM inteira na GPU com ROV"**: visualmente equivalente (PSNR de 51 a 55 dB na gameplay), mas só ~10% mais rápido. O gargalo mudou para a thread do guest. Fiel não é rápido.

**Três tropeços que valem o vídeo.**
1. *O ninja errado.* Um agente usou o ninja do WinGet (1.13.2) enquanto os outros usavam o do Visual Studio (1.12.1). Cada versão descarta o `.ninja_log` da outra, e isso forçou recompilar ~41,8 mil arquivos. Ninguém errou no código; o erro foi no ambiente. [TELA: o log do ninja pedindo tudo de novo]
2. *A regressão que não era de quem eu suspeitava.* Apareceu uma regressão de imagem na cena 3D. Dois agentes provaram, com A/B usando o rasterizador original no mesmo exe e replay, que não era o rasterizador nem o VU1: era uma correção de GIFtags incompletas do frontend, que muda o que é desenhado. Ficou atrás de `PS2X_GS_GIF_STATE=1`. A lição: bisseção por troca de arquivos e replay de streams, e medir antes de culpar.
3. *A medição que enganou.* Com a máquina carregada por cinco agentes, os números do ninja vieram inflados de 2,5x a 10x. Por isso a medição final foi feita com a máquina parada.

**O número que importa.** Mesma janela do jogo (vsync 7530 a 7640), máquina parada: baseline **2,96 Hz**; build novo no modo CPU **5,39 Hz**; d3d11 **6,87 Hz**; alvo **50 Hz**. Ou seja, dobrou, mas fica a 7x ou mais do alvo. [TELA: a tabela de quatro linhas]

**O erro de planejamento que eu assumo.** Eu tinha feito um plano de ganhos de 2 a 4x. A lacuna real era de ~15 a 20x. Eu deveria ter dito, no começo, que ajuste fino não fecharia a conta. Ordem de grandeza primeiro, plano depois.

**A pergunta do usuário, e a resposta honesta.** O usuário questionou: "hardware 30-50x melhor e roda a 5 FPS? O emulador roda bem." É uma ótima pergunta, e a comparação é enganosa por quatro razões. (1) A CPU só é ~15x mais rápida em UMA thread. (2) Emular cada ciclo do VU1 custa dezenas de ciclos do host. (3) O GS do PS2 preenche cerca de 7 milhões de pixels por quadro em hardware dedicado, e nós fazemos isso em software. (4) O PCSX2 roda bem porque compila o VU1 para código nativo (o microVU), desenha o GS na GPU com render targets e o blend do hardware, aceitando pequenas diferenças, e separa EE, VU e GS em threads. O que eu tinha construído era a implementação de referência: exata e verificável, não rápida. Isso não foi trabalho perdido: ela é o oráculo contra o qual vou comparar a versão rápida. [TELA: o diagrama das três threads do PCSX2]

**A decisão de rota (do usuário).** Abandonar o "byte-idêntico" para o caminho rápido. Alternativa descartada: continuar espremendo a referência (rende 2x aqui e ali e não fecha 15x). Risco aceito: a versão rápida pode diferir em pixels e pequenos detalhes da referência, e precisaremos medir essa diferença com replay e PSNR, em vez de assumir. O plano:
- (a) **VU1 para código nativo no estilo microVU**: traduzir o microprograma uma vez para código do host, em vez de interpretar instrução a instrução.
- (b) **GS por hardware de verdade em D3D11**: render targets, blend e Z do hardware, resolução interna escalável, com a interface já preparada para D3D12 e Vulkan depois.
- (c) **Threads separadas para EE, VU1 e GS.**
- (d) **Antes de tudo, reduzir o custo do rebuild.** O PCH do runner inclui `ps2_runtime.h`, com 22 headers e 46,7 mil linhas; editar qualquer um refaz tudo. O plano: um prelúdio leve com 11 headers, um `build_dev` em `/Od` e um script único de build. Por que primeiro? Porque cada experimento das etapas (a) a (c) paga esse custo; baratear o rebuild multiplica tudo que vem depois (e evita repetir o episódio do ninja).

**Ideias futuras (só ideias, nada feito).** O usuário levantou: resolução alta/4K e widescreen. A resolução interna escalável do renderer por hardware viabiliza a primeira; o widescreen exige patch da projeção do jogo. Também texturas refeitas (substituição por hash no upload; na RAM do host, não na RAM do guest) e modelos novos. Nenhuma foi implementada. Resultado dessas etapas: `[PREENCHER]`.

**Referências (regra 8).** Para esse caminho, leio o PCSX2 (GPL-3: microVU e GSdx/GSHw) e o paraLLEl-GS só como referência de design, sem copiar código. Ver `docs/similar-projects.md`.

## Rodada 10: a madrugada em que mudei de rota (rebuild barato, VU1 nativo, GS na GPU)

**O plano da rodada 9, executado de madrugada (05/11/2026).** Depois da decisão do usuário (abandonar o "byte-idêntico" no caminho rápido), o trabalho foi feito de noite, com até 7 agentes em paralelo (opus e sonnet), cada um dono dos seus arquivos e todos obedecendo um lock de build. Duas regras do usuário valeram: nenhuma pergunta enquanto ele dormia, e internet só para leitura. No meio, bati no limite de uso de 5 horas; o trabalho parou e foi retomado depois, e é por isso que os docs têm horários quebrados. [TELA: a lista de agentes e o lock de build] Aviso ao espectador: tudo abaixo é medição desta máquina, com outros agentes rodando ao mesmo tempo; os absolutos têm ruído.

**1. Primeiro baratear o rebuild (e por que isso vem antes de tudo).** O problema: editar um header comum refazia ~41,8 mil arquivos do runner, mais de 12 minutos. A causa, lendo o ninja: o PCH incluía `ps2_runtime.h`, que puxa 22 headers, e o ninja trata o PCH como dependência implícita das 1307 unidades unity. Mexer em qualquer header daquele fechamento refazia tudo. A solução foi um "prelúdio leve" (`ps2_runner_prelude.h`) que só declara o que o código gerado usa, isolando o runner do resto. Um script confere que as 29 declarações batem com as reais (0 divergências), e os 41.808 `.cpp` gerados foram retargetados com um script, sem rodar o gerador de novo (o gerador já foi ajustado, então uma regeneração futura produz o mesmo). Resultado: editar `ps2_vu1.h` + `gs_frontend.h/.cpp` passou de ~12 min para **34,6 s** (42 passos, 0 unidades do runner). Prova de que nada mudou: `regress -Scenario menu` byte-idêntico. [TELA: o ninja com "42 passos"]
- Alternativa: um `build_dev` em `/Od` (rebuild completo ~5,3 min, mas não serve para medir FPS e pode divergir em bits de ponto flutuante); ficou como árvore de iteração. O link dela ainda falhou naquela noite por fontes novas de outros agentes fora do `CMakeLists.txt`: `[PREENCHER]` quando for refeito.
- Armadilhas que valem o vídeo: (a) o reconfigure do cmake custa ~145 s (41 mil fontes), então não toque o `CMakeLists.txt` à toa, e fonte nova em `src/lib` exige reconfigure; (b) um cmake interrompido com FetchContent online apagou o raylib uma vez e gerou um `build.ninja` sem o include dele: agora sempre `FETCHCONTENT_FULLY_DISCONNECTED=ON`; (c) o ninja do WinGet contra o do Visual Studio, o mesmo erro da rodada 9, virou um `tools/build.ps1` que fixa o ninja do VS, pega o lock, grava log e avisa se o `.ninja_log` encolheu. O conceito: o build é um recurso compartilhado, e a ferramenta dele faz parte do ambiente.
- Limite honesto: o rebuild completo continua ~12,5 min; o ganho é só para quem edita headers do runtime.

**2. VU1 nativo, estilo microVU.** Em vez de interpretar instrução por instrução com filas de escrita atrasada, traduzo cada par de instruções uma vez (cache de 2048 pares) e executo com SSE/FMA nas 4 lanes. A ideia central do microVU: os hazards (esperar o resultado de uma instrução anterior) são resolvidos na tradução, não ciclo a ciclo na execução. Descobri que dava para pular direto ao ciclo de prontidão de cada par, porque o interpretador já parava todo leitor de registrador pendente e o tempo é determinístico. Só três casos precisam de tratamento explícito (shadow da lower, branch lendo o VI antigo, truncamento a 16 bits). Medição no harness: **~28 ns por ciclo contra ~113 ns (~4x)**. No jogo, o VU foi de **33,7% para ~6% das amostras**, com a ressalva de que a janela não é idêntica à da medição antiga e a máquina estava carregada; a medida final com a máquina parada: `[PREENCHER]`. [TELA: a pizza do perfil, antes e depois]
- Surpresa: mesmo sem exigir byte-idêntico, o resultado saiu **bit a bit idêntico, inclusive no contador de ciclos**, em replay de ~15 mil chamadas reais e ~90 mil sintéticas (várias sementes), e o `scene3d` com 6 capturas idênticas em md5. Por que isso importa: o interpretador antigo virou oráculo; qualquer divergência é bug, não "aproximação".
- Por que o PATH1 é recuperado sob demanda: o XGKICK lê a memória aos poucos; copiar tudo instantaneamente (atalho comum nos emuladores) muda o dado se o programa escreve no buffer durante a transferência.
- JIT x86-64 **não feito**: com o VU em ~6%, o ganho restante é pequeno e o risco, alto. Também descartei "closure compilation" do interpretador mantendo as filas, porque o custo era justamente das filas. Risco aceito: o caminho rápido de flags depende de arredondamento para zero e de FTZ/DAZ desligados.
- Regra 8: li o `microVU_Analyze.inl` do PCSX2 (GPL-3, só leitura); nenhum trecho copiado.

**3. O GS por hardware, em D3D11 (`PS2X_GS=hw`).** Por que o backend ROV da rodada 9 só ganhou ~10%? Porque emular cada pixel no shader é **fiel, não rápido**: ele reproduz as regras do GS no pixel shader, e o gargalo continuou no resto. O novo backend usa o hardware de verdade: render targets reais por (base, largura, formato), com escala interna; blend e Z do hardware (o blend do GS vira dual-source blend, `DEST_ALPHA` etc.; só alguns casos raros são aproximados e contados); alpha test em dois passes como no PCSX2. [TELA: o diagrama das três cópias da memória]
- O truque que decidiu o desenho: este jogo lê o buffer principal de cor como **textura de paleta de 8 bits com largura 20** (~9 mil sprites de pós-processo por quadro). O PCSX2 trata esse tipo de coisa com caminhos especiais de conversão de alvo. Eu mantive uma **cópia da memória local do GS na GPU** (4 MiB) com coerência por página de 8 KiB: quando uma textura tem outro layout, o alvo sujo é escrito de volta nessa cópia por compute e a textura lê dela com o swizzle do GS. Alternativa descartada: "alvo como verdade única" do PCSX2, por exigir os caminhos especiais acima. Risco: complexidade de coerência; os bugs achados no caminho (ordem lote x atualização da memória GPU, Z inteiro contra Z interpolado, header sem recompilar o dependente e corrupção de heap) estão no doc para não repetir.
- Números (cena 3D, janela de vsync 7530-7640, sem outro runner rodando): **12,2 vsync/s na escala 1** e **11,0 na escala 4** (2560x2048 interno), contra baseline 3,0, CPU 6,1 e ROV 6,9. Dois sinais: ~4x sobre o baseline; e a escala 4 custa só ~10%, ou seja, o GS deixou de ser o gargalo e o quadro agora é limitado pela thread do jogo. A resolução da medida é de ~±10% (o CSV tem uma linha por segundo).
- Qualidade: PSNR acima de 30 dB em todas as capturas do jogo (36,9 a 73,6 dB no 1x); no harness, 30,9 a 32 dB nas escalas 2 e 4. Diferença visível restante: o **brilho do holofote**, sprites que se realimentam. Não sei qual dos dois (CPU de referência ou hardware) é o comportamento do GS real; fica como pergunta aberta, e eu não vou escolher no chute.
- A RHI foi escrita para mapear 1:1 para D3D12 e Vulkan (barreiras, anéis com fence e dual-source blend são as diferenças a cuidar), mas só o D3D11 existe. Ainda faltam DATE, PABE, wrap sem clamp e dither, e rodar as outras regressões com `hw` antes de torná-lo padrão: `[PREENCHER]`.

**4. Filtro CRT, só na apresentação.** Um shader HLSL (D3D11/RHI) e um porte GLSL (raylib), com presets suave/médio/forte, que mexem só no que vai para a janela. Nunca nas capturas e regressões: confirmei com `shot` idêntico (md5) com o filtro ligado. Por quê: o filtro é gosto do jogador, e um oráculo precisa continuar medindo o jogo, não o filtro. Os valores são ponto de partida visual, não calibrados contra um CRT real. [TELA: o jogo com e sem o filtro]

**5. Cache de arquivos em RAM: resultado honesto, sem ganho.** Escrevi um cache em blocos de 1 MiB com LRU, read-ahead e testes de correção byte a byte (400 leituras aleatórias, 4 threads). Integrado nos três caminhos de leitura do disco emulado; as capturas saem idênticas. Medição: **180,9 s sem cache contra 179,4 s com cache** (~0,8%, dentro do ruído), com espera total de leitura de 67 ms. Nesta máquina, com SSD, o disco nunca foi o gargalo. O benefício esperado fica para HDD ou rede; o custo é amplificação de leitura (~6x em leituras pequenas). Lição: medir antes de comemorar, e contar o resultado nulo. [TELA: a tabela com os dois tempos]

**6. Threads: viável, mas a ordem mudou.** O desenho tem 4 threads e 3 anéis SPSC (EE para VU, EE para GS, VU1 para GS). Implementei só as etapas 0 a 2, todas atrás de variáveis de ambiente: contadores de sincronização, relógio só do guest e um hash do stream do GS como oráculo de determinismo. Os contadores mostraram quase nenhum ponto de sincronização na cena 3D, só uma leitura de CSR por quadro do jogo (e dá para não esperar nela se não houver SIGNAL/FINISH em voo). A primeira tentativa de determinismo falhou (a escolha do próximo prazo comparava instantes do host) e foi corrigida comparando em ciclos; depois, corridas simultâneas deram hashes idênticos. Nada mudou de thread ainda. Com o VU1 em ~6%, uma thread dedicada a ele ganharia pouco, então reordenei: **acelerar o GS primeiro**, e a thread do VU1 depois. Honestidade: os 50 Hz ainda não foram atingidos; próximas medições: `[PREENCHER]`.

**7. Ideias futuras (nada feito).** Widescreen (exige patch da projeção do jogo) e 4K com hacks pontuais; texturas HD por substituição por hash no upload, na RAM do host; D3D12 e Vulkan sobre a RHI. O launcher já tem as opções (`--renderer hw`, `--internal-scale`, `--crt`, `--vfs-cache-mb`), e `--widescreen` existe como planejado, sem efeito.

**O que eu levo desta rodada.** Primeiro, baratear o ciclo de feedback multiplica todo o resto. Segundo, um oráculo lento e exato (o interpretador antigo, o rasterizador de CPU) é o que permite ousar na versão rápida com replay e PSNR. Terceiro, fiel não é rápido, e o resultado nulo do cache vale o mesmo espaço no vídeo que o 4x do VU1.
