# LEGAL — licenças e direitos autorais (base ps2_recomp)

> **Estudo de caso / Case study.** PT-BR: projeto de estudo pessoal; o desenvolvimento só continua no meu tempo disponível, quando possível (sem prazos nem garantia de conclusão). EN: personal case study; development only continues in my available time, when possible (no deadlines, no guarantee of completion). Ver/see `README.md`.

> Orientação técnica do projeto, **não é aconselhamento jurídico**. Antes de qualquer publicação, o usuário decide (e, se for distribuir um port pronto, consulta um advogado). `ps2_recomp` é **público** desde 24/11/2026 por decisão do usuário; os repositórios dos jogos continuam **privados** (regra 3).

## 1. Licença do código

- Este repositório é distribuído sob **GPL-3.0** (`LICENSE`, texto oficial do GNU).
- Motivo: o runtime e os patches são derivados do **PS2Recomp** (ran-j, GPL-3.0). Obra derivada distribuída precisa sair sob GPL-3.0 com o código-fonte disponível.
- A GPL só obriga ao **distribuir**. Uso próprio e repositório privado não têm obrigação, mas dar acesso a terceiros pode contar como distribuição.

## 2. O que é derivado e o que é nosso

| Parte | Origem | Licença prevista |
|---|---|---|
| Patches no `ps2xRuntime` e arquivos novos ligados a ele (`ps2_virtual_pad`, `ps2_frame_stats`, `ps2_video_clock`, `ps2_irq_stats`, `ps2_sampling_profiler`, `ghost_overrides.cpp`) | Derivado do PS2Recomp | GPL-3.0 |
| Código C++ gerado pelo `ps2_recomp` a partir do ELF de um jogo | Gerado a partir de obra protegida do jogo | **Nunca versionado** (regra 2); status jurídico incerto |
| `launcher/`, `tools/` (Python e scripts Ghidra) | Código nosso, sem código do PS2Recomp | GPL-3.0 por padrão neste repo (pode ser relicenciado por quem é autor, se não incorporar código GPL de terceiros) |
| `docs/`, `roteiro-video.md` | Texto nosso | Sem código do jogo; mesmo `LICENSE` do repo |

## 3. O que nunca entra em nenhum repositório (regra 2)

ISO, ELF/IRX extraídos, `.BIN`, vídeo, áudio, texturas, modelos, dumps de memória, saves, capturas de tela do jogo, binários originais, qualquer asset convertido. A licença do código **não** cobre nada disso: são direitos autorais do titular do jogo.

## 4. Antes de tornar qualquer repositório público (checklist)

1. Confirmar a autorização explícita do usuário (regra 3).
2. Varrer **todo o histórico** do git (não só o último commit) por bytes do jogo, `refs.txt` e arquivos ignorados que tenham entrado por engano.
3. Cabeçalho de licença nos arquivos novos de código; manter os avisos de copyright do PS2Recomp nos arquivos modificados e indicar as alterações.
4. Garantir que o build/launcher continua exigindo o dump original do usuário (regra 4) e que nada baixa ou embute assets.
5. Revisar nomes, marcas e logotipos do jogo em README, ícones e capturas.
6. Se o objetivo for distribuir um port pronto (e não só código), consultar um advogado sobre o código gerado e a marca.

## 5. Outras licenças em uso

Ver `docs/similar-projects.md`: PCSX2 (GPL-3.0), m2c (GPL-3.0), decomp-permuter (GPL-3.0), ps2dev/ps2sdk (AFL-2.0). Copiar código GPL de terceiros obriga GPL; usar como referência de comportamento não. Repositórios sem licença declarada: só leitura e estudo.
