// Cria funcoes nos enderecos listados em um arquivo de texto (um por linha, "0xENDERECO ..." ou so o endereco).
//
// Fecha o ciclo "alvo ausente em tempo de execucao -> funcao no Ghidra -> recompilar": o runtime grava os
// alvos de desvio sem funcao gerada em PS2X_MISSING_LOG, e este script os transforma em funcoes.
//
// Uso (headless): -postScript CreateFunctionsAt.java <enderecos.txt>
// @category ps2decomp

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Listing;

import java.io.BufferedReader;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.LinkedHashSet;
import java.util.Set;

public class CreateFunctionsAt extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 1) {
            printerr("uso: CreateFunctionsAt.java <enderecos.txt>");
            return;
        }

        Set<Long> targets = new LinkedHashSet<>();
        try (BufferedReader reader = Files.newBufferedReader(Paths.get(args[0]))) {
            String line;
            while ((line = reader.readLine()) != null) {
                line = line.trim();
                if (line.isEmpty() || line.startsWith("#")) {
                    continue;
                }
                String token = line.split("\\s+")[0];
                try {
                    targets.add(Long.decode(token));
                } catch (NumberFormatException e) {
                    println("ignorando linha invalida: " + line);
                }
            }
        }

        Listing listing = currentProgram.getListing();
        int created = 0, already = 0, failed = 0;
        for (long target : targets) {
            Address address = toAddr(target);
            if (listing.getFunctionAt(address) != null) {
                already++;
                continue;
            }
            // Um alvo no meio de uma funcao existente nao vira funcao nova: seria um rotulo, nao uma entrada.
            Function containing = listing.getFunctionContaining(address);
            if (containing != null) {
                println("0x" + Long.toHexString(target) + " cai dentro de " + containing.getName()
                        + " (entrada em " + containing.getEntryPoint() + "); nao criado");
                failed++;
                continue;
            }
            boolean ok = listing.getInstructionAt(address) != null || disassemble(address);
            Function function = ok ? createFunction(address, null) : null;
            if (function != null) {
                created++;
            } else {
                println("falha ao criar funcao em 0x" + Long.toHexString(target));
                failed++;
            }
        }
        println("CreateFunctionsAt: alvos=" + targets.size() + " criadas=" + created
                + " ja_existiam=" + already + " falhas=" + failed);
    }
}
