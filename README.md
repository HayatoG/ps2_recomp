# ps2decomp

Base genérica de ferramentas, regras e método para **decompilar e portar jogos de PS2 para PC** (recompilação estática do MIPS R5900 + runtime). Cada jogo vive em **seu próprio repositório** dentro de uma pasta com o mesmo nome (ex.: `ghost_pc/`, que não é versionada aqui).

> **Repositório privado.** Não contém jogo algum: nenhum ISO, ELF, texturas, áudio ou vídeo. O projeto exige o **dump original do próprio usuário** e valida serial e hash antes de rodar.

## Conteúdo
| Pasta/arquivo | O quê |
|---|---|
| `AGENTS.md` | Regras do projeto, delegação por modelo, coordenação entre sessões, convenções |
| `tools/` | Ferramentas genéricas: leitor/extrator de ISO (`extract_iso.py`), contêiner `fpk`, compressão `V3a`/LHA, texturas `zim`, módulos IOP, scripts do Ghidra (`tools/ghidra/`), pós-processamento do TOML, sincronização do C++ gerado |
| `launcher/` | `ps2launcher.py`: valida o dump, extrai o necessário e inicia o port (`--fps`, `--pad-script`, `--shot-dir`, `--missing-log`, `--timeout`) |
| `docs/` | Ferramentas instaladas e projetos similares |
| `roteiro-video.md` | Roteiro de um vídeo de aprendizado sobre o processo, com o porquê de cada decisão |

## Fluxo (resumo)
1. Colocar o dump no diretório do jogo e rodar `python -I launcher/ps2launcher.py verify --game <jogo>`.
2. Extrair o ELF e os módulos, analisar no Ghidra (extensão R5900) e exportar para o PS2Recomp.
3. Recompilar, sincronizar o C++ gerado, compilar o runtime (sem LTCG) e rodar com pad virtual e capturas de tela.

## Licenças e terceiros
- O código gerado e o runtime derivam do **PS2Recomp** (GPL-3.0), clonado em `third_party/` (não versionado). Os patches ficam no repositório de cada jogo.
- Ferramentas usadas: Ghidra, `ghidra-emotionengine-reloaded` (Apache-2.0), radare2, entre outras listadas em `docs/tools.md`.
