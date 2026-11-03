// Cria funcoes nos enderecos de codigo que sao APONTADOS por dados ou carregados como constante, e que ainda
// nao pertencem a nenhuma funcao.
//
// Motivo: funcoes alcancaveis so por ponteiro (callbacks, vtables, tabelas de despacho) nao tem chamadas diretas,
// entao o Ghidra nao as cria; um recompilador estatico nao gera o alvo de um JALR/JR indireto e o jogo para com
// "missing-target" em tempo de execucao. Cada correcao individual custa um rebuild completo do codigo gerado, entao
// este script antecipa a classe inteira de uma vez.
//
// Fontes de candidatos:
//   1. palavras de 32 bits alinhadas em blocos de dados (nao executaveis) cujo valor cai em .text;
//   2. referencias de dados (constantes lui/addiu resolvidas pelo analisador do MIPS) que apontam para .text.
// Um candidato vira funcao somente se: esta alinhado, nao e entrada de funcao, nao cai DENTRO de funcao existente
// (rotulos de jump table), e a instrucao decodifica.
//
// Uso (headless): -postScript CreateFunctionsFromPointers.java [dry-run]
// @category ps2decomp

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressIterator;
import ghidra.program.model.address.AddressSet;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Listing;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceManager;

import java.util.TreeSet;

public class CreateFunctionsFromPointers extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        boolean dryRun = args.length > 0 && args[0].equalsIgnoreCase("dry-run");

        Memory memory = currentProgram.getMemory();
        Listing listing = currentProgram.getListing();
        ReferenceManager references = currentProgram.getReferenceManager();

        AddressSet code = new AddressSet();
        for (MemoryBlock block : memory.getBlocks()) {
            // Microcodigo VU (.vutext) e overlays nao sao codigo do EE.
            if (block.isExecute() && block.isInitialized() && !block.isOverlay()
                    && !block.getName().startsWith(".vu")) {
                code.add(block.getStart(), block.getEnd());
            }
        }

        TreeSet<Long> candidates = new TreeSet<>();
        long fromData = 0;
        long fromCode = 0;

        // 1. Palavras em blocos de dados que apontam para .text.
        for (MemoryBlock block : memory.getBlocks()) {
            if (block.isExecute() || !block.isInitialized() || block.isOverlay() || block.getName().startsWith(".vu")) {
                continue;
            }
            Address cursor = block.getStart();
            Address last = block.getEnd().subtract(3);
            if (last.compareTo(cursor) < 0) {
                continue;
            }
            // Alinha em 4 bytes: ponteiros de funcao em dados sao alinhados.
            long misalignment = cursor.getOffset() % 4;
            if (misalignment != 0) {
                cursor = cursor.add(4 - misalignment);
            }
            while (cursor.compareTo(last) <= 0 && !monitor.isCancelled()) {
                long value = memory.getInt(cursor) & 0xFFFFFFFFL;
                if ((value & 3) == 0 && code.contains(toAddr(value))) {
                    if (candidates.add(value)) {
                        fromData++;
                    }
                }
                cursor = cursor.add(4);
            }
        }

        // 2. Referencias de dados (constantes de enderecos) cujo destino esta em .text.
        AddressIterator destinations = references.getReferenceDestinationIterator(code, true);
        while (destinations.hasNext() && !monitor.isCancelled()) {
            Address destination = destinations.next();
            if ((destination.getOffset() & 3) != 0) {
                continue;
            }
            boolean dataReference = false;
            for (Reference ref : references.getReferencesTo(destination)) {
                if (ref.getReferenceType().isData() && !ref.getReferenceType().isFlow()) {
                    dataReference = true;
                    break;
                }
            }
            if (dataReference && candidates.add(destination.getOffset())) {
                fromCode++;
            }
        }

        int created = 0, alreadyFunction = 0, insideFunction = 0, undecodable = 0;
        for (long target : candidates) {
            Address address = toAddr(target);
            if (listing.getFunctionAt(address) != null) {
                alreadyFunction++;
                continue;
            }
            if (listing.getFunctionContaining(address) != null) {
                insideFunction++;
                continue;
            }
            if (dryRun) {
                created++;
                continue;
            }
            boolean decodes = listing.getInstructionAt(address) != null || disassemble(address);
            Function function = decodes ? createFunction(address, null) : null;
            if (function != null) {
                created++;
            } else {
                undecodable++;
            }
        }

        println("CreateFunctionsFromPointers: candidatos=" + candidates.size()
                + " (dados=" + fromData + ", constantes=" + fromCode + ")"
                + " criadas=" + created + " ja_eram_funcao=" + alreadyFunction
                + " dentro_de_funcao=" + insideFunction + " nao_decodifica=" + undecodable
                + (dryRun ? " (dry-run)" : ""));
    }
}
