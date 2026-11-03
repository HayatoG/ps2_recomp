# Ferramentas instaladas (Windows 11)

## Ghidra 12.1.4
- Instalação: `C:\Users\guilh\Downloads\ghidra_12.1.4_PUBLIC` (JDK 21: `C:\Program Files\Microsoft\jdk-21.0.13.21-hotspot`)
- ReVa (MCP): `mcp-reva`, roda Ghidra em processo (pyghidra) — **precisa reiniciar o MCP após instalar extensões**.

## ghidra-emotionengine-reloaded v2.1.39 (2026-10-29)
- Fonte: https://github.com/chaoticgd/ghidra-emotionengine-reloaded (Apache-2.0)
- Asset: `ghidra_12.1.4_PUBLIC_20261029_ghidra-emotionengine-reloaded.zip`, sha256 `088369C4D7DE43CC95DBF254936DFED45EAE5EDDFCAF8B928189344BCC6A4DAC`
- Instalada em `<ghidra>\Ghidra\Extensions\ghidra-emotionengine-reloaded` (cópia direta da pasta).
- Linguagem: `r5900:LE:32:default` (EE R5900 com MMI/VU0). Spec compilada com `support\sleigh.bat` sem erros (`r5900.sla` gerado).
- Motivo: o MIPS padrão do Ghidra não decodifica MMI (ex.: `28 0C 00 70` no crt0 de `SLES_530.20`).
- Scripts úteis na extensão: `ExportDvpOverlays.py`, `ImportSymFile.py`, `PCSX2SaveStateImporter.java`, `ImportMMIORegisterLabels.java`.

## REA (`mcp__rea__*`)
- Abre o ELF MIPS, mas a análise profunda é recusada no Windows (Ghidra do REA aceita só PE; Hopper/IDA/Kuna sem MIPS). Usar o ReVa para ELF/IRX.

## Ghidra headless (sem ReVa)
- `support\analyzeHeadless.bat <dir_projeto> <nome> -import <elf> -processor r5900:LE:32:default` (import+análise) e `-process <programa> -noanalysis` para reutilizar o projeto.
- Chamar via `Start-Process -FilePath analyzeHeadless.bat -ArgumentList ... -RedirectStandardOutput` (aspas aninhadas com `cmd /c` quebram).
- `askFile`/`askXxx` em scripts: passar os valores como **argumentos do `-postScript`**; o arquivo `.properties` não casou com as chaves.
- Scripts próprios: `tools/ghidra/ApplySymbolsCsv.java <symbols.csv> [high|medium|low]` aplica nomes de um CSV `address,name,confidence,evidence`.
- `-scriptPath` aceita vários diretórios separados por `;`.

## Toolchain C++ (winget): VS Build Tools 2022, CMake 4.4.4, Ninja
- Fora do PATH das sessões abertas; ver `ghost_pc/docs/ps2recomp-spike.md` para o script de build.

## Pipeline genérico (rodar de dentro do diretório do jogo, ex. `cd ghost_pc`)
1. `python -I ../tools/extract_iso.py <iso> work MODULES SYSTEM.CNF <ELF de BOOT2>`
2. Importar `work/<ELF>` no ReVa/Ghidra com a linguagem `r5900:LE:32:default`, analisar, exportar anotações em texto para `<jogo>/docs/`.
3. `python -I ../tools/iop_modules.py work/MODULES --json logs/iop_modules.json` para catalogar os módulos IOP.
