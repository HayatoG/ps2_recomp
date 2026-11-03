# Plano do launcher (genérico)

Status: **planejamento**. Hoje existe `launcher/ps2launcher.py` (CLI em Python: `info`, `verify`,
`extract`, `run`). Este documento define para onde ele vai. Regras que valem sempre: 4 (exigir o
dump original), 5 (launcher simples e funcional, fora do jogo), 6 (logs exportáveis), 2 (nunca
versionar nem baixar assets do jogo).

Fontes do estudo: `akitaonrails/rea-launcher-sdk` (SDK, MIT OR Apache-2.0), `rea-colin-mcrae` e
`rea-screamer-trilogy` (lidos por um agente, só leitura), mais a pesquisa de Steam/GOG/Deck em
`distribution.md`. Itens "não confirmado" não foram verificados.

## 1. O que aprendemos dos launchers do AkitaOnRails
- **Stack:** Rust, workspace com crates (núcleo portátil, parte Win32, UI egui/eframe, hook de
  fechamento). GUI **e** CLI completa: toda opção tem flag, qualquer flag pula a janela, existem
  `--check` (valida sem iniciar), `--no-window` e `--settings`.
- **Distribuição:** zip portátil no Windows (sem instalador nem admin), AppImage no Linux, o mesmo
  `.exe` sob Proton no Steam Deck. Jogo fixa uma tag do SDK e um script confere divergências.
- **Identidade por hash:** SHA-256 do executável do jogo contra uma tabela de edições conhecidas
  (`Known<E>`). Edição desconhecida: **recusa**, mostra o hash e o link para reportar ("o código,
  nunca o executável"); nunca faz patch por adivinhação.
- **Jogo do usuário somente leitura:** o launcher monta uma cópia privada ("runtime"), com manifesto
  `tamanho/mtime/caminho` para copiar só o que mudou, escrita por troca atômica de arquivo e
  verificação de que runtime e pasta do jogo não se sobrepõem.
- **Ledger por SHA-256** do que o launcher gerou: só remove/sobrescreve o que ainda tem os bytes
  que ele escreveu.
- **Config:** `launcher.cfg` (`chave = valor`), gerado na primeira execução todo comentado; flags
  sobrescrevem só aquela execução; `launcher.cfg` ao lado do exe = modo portátil.
- **Rede:** nenhuma sem o jogador pedir. Download opcional é retomável, com SHA-256 conferido
  durante o stream, checagem de espaço livre, extração segura (ignora nomes que escapam da pasta)
  e marcador `.complete`.
- **Logs:** arquivo append-only com níveis, com limite de tamanho (reinicia em 1 MiB); logar nunca
  derruba o launch.
- **Deck:** detecta a tela 1280x800 e escolhe um perfil; janela navegável por gamepad; gamescope
  opcional. O atalho non-Steam é manual (não confirmado que haja automação). Não foi testado num
  Deck real (a própria README admite).
- **Processo:** regras checáveis (`scripts/launcher-check`), `prepush-scan`, allowlist do pacote,
  testes de snapshot (`--help`, cfg), fixtures sintéticas, caminhos hostis (espaços, acentos).
- **Não copiar:** Wine/Proton/DOSBox/dgVoodoo, patch de bytes x86, registro do Windows, e o pacote
  HD derivado de arte do jogo distribuído em release (fere a nossa regra 2).

## 2. Análise: assets no nosso launcher

### Hoje
- Valida serial (BOOT2 do `SYSTEM.CNF`) + sha256 do ELF de boot; só isso.
- Extrai os arquivos listados no `game.toml` do ISO para `<jogo>/work/` (cópia crua, sem
  conversão). No `ghost_pc`: ~2,9 GB, dos quais ~2,2 GB são os `MOV*.BIN`.
- O runtime usa o diretório do ELF como raiz de `cdrom0:`, então os dados precisam estar ao lado
  do ELF em `work/`.
- Entrada aceita: ISO9660 com setores de 2048 bytes. Idempotência só por "o arquivo existe".

### Lacunas
1. **Dados sem verificação.** Arquivos extraídos não têm tamanho/hash conferidos: ISO corrompido,
   extração interrompida ou edição diferente com o mesmo ELF passam despercebidos.
2. **Extração não atômica.** Falha no meio deixa arquivo truncado que conta como "existe".
3. **Sem checagem de espaço livre**, sem retomada, sem progresso.
4. **Só uma edição conhecida** (Europa); sem tabela de edições nem recusa amigável com o hash.
5. **Só ISO 2048.** Falta BIN/CUE (setores de 2352, modo 2), CHD, e talvez MDS/MDF (não
   confirmado que o usuário precise).
6. **Cópia de 2,9 GB** que pode ser evitada (ver opções).
7. **Local de `work/`** fixo ao lado do jogo; para Steam/GOG/Deck precisa de caminhos por SO
   (ver `distribution.md`).
8. **Sem ledger/manifesto**: não sabe o que gerou, então não limpa nem refaz só o necessário.

### Opções para os dados do disco
| Opção | Prós | Contras |
|---|---|---|
| A. Extrair tudo para `work/` (atual) | Simples, o runtime já funciona | 2,9 GB duplicados, extração lenta, sem retomada |
| B. A + manifesto, hash por arquivo, escrita atômica, retomada | Robusto, barato de fazer, sem mexer no runtime | Ainda duplica espaço |
| C. VFS do runtime lendo direto do ISO/CHD | Sem cópia, aceita CHD/BIN, mais próximo de "o disco" | Exige mudar o VFS do runtime e tratar leitura aleatória/seek (**não verificado** que o VFS atual permita); desempenho a medir |
| D. Extrair só o que o jogo precisa em disco rápido e deixar o vídeo no ISO | Meio-termo | Dois caminhos, complexidade |

**Recomendação:** B agora (compatível com tudo); C como etapa posterior após medir. Os vídeos já
são o grosso; como o FMV nem foi tratado ainda, decidir C junto com o trabalho de vídeo.

### Validação proposta
- **Tabela de edições** (serial → ELF sha256 → lista de arquivos com tamanho e sha256), no
  `game.toml` ou num `editions.toml` do jogo. Hashes e tamanhos são conhecimento nosso (não são
  bytes do jogo) e podem ser versionados.
- Recusa com mensagem clara: mostra serial/hash lidos e como reportar; nunca tenta rodar uma
  edição desconhecida.
- Hash do próprio ISO como referência cruzada com bases da comunidade (**não confirmado** que
  exista base utilizável; não depender).
- Modo `verify --deep` (hash de todos os arquivos extraídos) e `verify` rápido (tamanho + ELF).

### Extração proposta
- Escrever em `*.part` e renomear ao concluir; marcador `.complete` por arquivo (ou manifesto).
- Checar espaço livre antes; mostrar progresso; retomar a partir do manifesto.
- Verificar o hash durante a cópia (stream), não depois.
- Caminhos hostis: espaços, acentos, ISO em unidade de rede; testar.
- Ledger/manifesto em `<dados>/manifest.json`: o que foi gerado, de qual dump, com qual hash.
- Remover/refazer só o que o launcher gerou.

### Fora de questão
- Embutir ou baixar assets do jogo. Pacotes "HD"/convertidos de arte do jogo não serão
  distribuídos (regra 2).
- Rede por padrão. Downloads só de ferramentas de terceiros, opcionais, com URL + SHA-256 + tamanho
  fixos e ação explícita do usuário.

## 3. Arquitetura do launcher
- **Núcleo sem UI** (biblioteca): identificar o dump, verificar, extrair, montar o ambiente, iniciar
  o runtime, logs. A CLI e a GUI chamam o mesmo núcleo.
- **CLI** com paridade total (cada opção é flag e chave do `launcher.cfg`): `info`, `verify`,
  `extract`, `run`, `--check`, `--no-window`, `--settings`.
- **GUI** simples: escolher/lembrar a pasta ou ISO, estado da verificação, botão Jogar, opções
  (renderer D3D11/D3D12/Vulkan, resolução, limite de FPS, tela cheia, entrada), exportar logs.
  Toda a navegação por gamepad (requisito Deck Verified); fonte mínima 12 px a 1280x800.
- **Linguagem (decisão em aberto):**
  - *Rust + egui*, como o Akita: binário único, bom para GUI + CLI, SDK MIT/Apache que podemos
    estudar e reaproveitar peças (identificação por hash, download retomável, log, escrita
    atômica). Contra: mais uma linguagem no projeto.
  - *Python (atual)*: rápido de evoluir; mas distribuir exige empacotar o interpretador e o
    Smart App Control complica exe gerado por PyInstaller.
  - *C++ com SDL3 + Dear ImGui no mesmo toolchain do runtime*: uma só linguagem, entrada por SDL3
    igual ao jogo. Contra: reimplementar as utilidades.
  - **Decisão do usuário (2026-11-03): launcher em Rust, leve.** Implementação inicial em `launcher-rs/`.
  - ~~Recomendação provisória:~~ manter o Python como protótipo/ferramenta de desenvolvedor e
    escrever o launcher de distribuição em Rust (egui), reaproveitando ideias do SDK. Confirmar
    com o usuário antes de começar.
- **Config e dados** por SO (ver `distribution.md`): portátil (pasta ao lado) ou perfil do usuário.
- **Logs:** `launcher.log` com nível e timestamp, limite de tamanho, botão/flag para exportar
  (regra 6). Também recolher o log do runtime.

## 4. Etapas
1. ~~Tabela de edições + `verify` rápido/profundo + recusa amigável~~ **feito em `launcher-rs/`** (`[[dump.file]]`, `verify`, `verify --deep`, `hashes`).
2. ~~Extração atômica com manifesto, espaço livre e retomada~~ **feito** (`.part`, manifesto, checagem de espaço; retomada por arquivo).
3. Camada de caminhos por SO e `launcher.cfg` (gerado e comentado). **`launcher.cfg` feito** (`launcher-rs/src/settings.rs`, lido pelo CLI e pela janela; flag > cfg > padrao); caminhos por SO pendentes.
4. Entradas BIN/CUE e CHD.
5. Decidir a linguagem final e portar o núcleo; GUI com gamepad. **Rust decidido; janela `ps2launch-gui` feita** (egui/glow, feature `gui`, abas Jogar/Imagem/Controles, gilrs, `--check`/`--no-window`/`--settings`). Falta testar com controle real e no Deck.
6. Integração com a Steam local (`steam-add`, arte, perfil Deck; ver `steam-library.md`), empacotamento e assinatura (`distribution.md`).
7. Avaliar VFS direto do ISO/CHD no runtime.
