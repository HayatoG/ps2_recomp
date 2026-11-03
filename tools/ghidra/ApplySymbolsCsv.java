// Aplica um CSV (address,name,confidence,evidence) ao programa atual, renomeando funcoes.
// Uso (headless): -postScript ApplySymbolsCsv.java <symbols.csv> [min_confidence: high|medium|low]
// @category ps2decomp

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.SourceType;

import java.io.BufferedReader;
import java.io.File;
import java.nio.file.Files;

public class ApplySymbolsCsv extends GhidraScript {
    private static int rank(String c) {
        switch (c.trim().toLowerCase()) {
            case "high": return 3;
            case "medium": return 2;
            default: return 1;
        }
    }

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        File csv = new File(args.length > 0 ? args[0] : askFile("symbols csv", "Open").getPath());
        int min = rank(args.length > 1 ? args[1] : "low");
        int applied = 0, skipped = 0, missing = 0, failed = 0;
        try (BufferedReader r = Files.newBufferedReader(csv.toPath())) {
            String line = r.readLine(); // cabecalho
            while ((line = r.readLine()) != null) {
                if (line.isBlank()) continue;
                String[] f = line.split(",", 4);
                if (f.length < 3 || rank(f[2]) < min) { skipped++; continue; }
                Address a = toAddr(f[0].trim());
                Function fn = getFunctionAt(a);
                if (fn == null) fn = createFunction(a, null);
                if (fn == null) { missing++; continue; }
                String name = f[1].trim();
                try {
                    if (!fn.getName().equals(name)) fn.setName(name, SourceType.USER_DEFINED);
                    applied++;
                } catch (Exception e) {
                    println("falha em " + f[0] + " -> " + name + ": " + e.getMessage());
                    failed++;
                }
            }
        }
        println("ApplySymbolsCsv: aplicados=" + applied + " ignorados=" + skipped + " sem_funcao=" + missing + " falhas=" + failed);
    }
}
