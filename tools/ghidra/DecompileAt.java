// Decompila as funcoes que contem os enderecos dados e grava o C em um arquivo de texto.
// Util quando o ReVa (MCP) esta indisponivel e para alimentar relatorios em lote.
//
// Uso (headless): -postScript DecompileAt.java <saida.txt> <0xENDERECO> [<0xENDERECO> ...]
// Cada funcao sai com cabecalho: endereco de entrada, nome, chamadores diretos (ate 8).
// @category ps2decomp

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

import java.io.PrintWriter;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.LinkedHashSet;
import java.util.Set;

public class DecompileAt extends GhidraScript {
    private static final int TIMEOUT_SECONDS = 60;
    private static final int MAX_CALLERS = 8;

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 2) {
            printerr("uso: DecompileAt.java <saida.txt> <0xENDERECO> [...]");
            return;
        }

        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        Set<Address> done = new LinkedHashSet<>();

        try (PrintWriter out = new PrintWriter(Files.newBufferedWriter(Paths.get(args[0])))) {
            for (int i = 1; i < args.length; i++) {
                Address address = toAddr(Long.decode(args[i]));
                Function function = getFunctionContaining(address);
                if (function == null) {
                    out.println("// " + args[i] + ": nenhuma funcao contem este endereco\n");
                    continue;
                }
                if (!done.add(function.getEntryPoint())) {
                    continue;
                }

                out.println("// ===== " + function.getName() + " @ " + function.getEntryPoint()
                        + " (consultado: " + args[i] + ") =====");
                out.println("// chamadores: " + callers(function));
                DecompileResults result = decompiler.decompileFunction(function, TIMEOUT_SECONDS, monitor);
                if (result.decompileCompleted()) {
                    out.println(result.getDecompiledFunction().getC());
                } else {
                    out.println("// falha na decompilacao: " + result.getErrorMessage());
                }
                out.println();
            }
        }
        decompiler.dispose();
        println("DecompileAt: " + done.size() + " funcoes gravadas em " + args[0]);
    }

    private String callers(Function function) {
        StringBuilder text = new StringBuilder();
        int count = 0;
        ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(function.getEntryPoint());
        while (refs.hasNext() && count < MAX_CALLERS) {
            Reference ref = refs.next();
            if (!ref.getReferenceType().isCall()) {
                continue;
            }
            Function caller = getFunctionContaining(ref.getFromAddress());
            text.append(ref.getFromAddress()).append(caller != null ? " (" + caller.getName() + ")" : "").append("; ");
            count++;
        }
        return count == 0 ? "(nenhum chamador direto)" : text.toString();
    }
}
