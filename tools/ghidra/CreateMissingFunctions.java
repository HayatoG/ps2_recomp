// Cria funcoes nos vaos do .text que nenhuma funcao conhecida cobre.
//
// Motivo: o Ghidra so cria funcoes alcancaveis por chamadas diretas, prologos reconhecidos ou simbolos.
// Codigo alcancavel apenas por ponteiro (callbacks, vtables, tabelas de dispatch) fica de fora, e um
// recompilador estatico entao nao gera o alvo de um JALR/JR indireto ("missing-target" em tempo de execucao).
//
// Uso (headless): -postScript CreateMissingFunctions.java [max_gap_bytes] [dry-run]
//   max_gap_bytes  ignora vaos maiores que isso (padrao 0 = sem limite); vaos enormes costumam ser dados
//   dry-run        so conta e lista, nao cria
// @category ps2decomp

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressRange;
import ghidra.program.model.address.AddressSet;
import ghidra.program.model.address.AddressSetView;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionIterator;
import ghidra.program.model.listing.Listing;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.mem.MemoryBlock;

public class CreateMissingFunctions extends GhidraScript {
    private static final int INSTRUCTION_SIZE = 4;

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        long maxGap = args.length > 0 ? Long.parseLong(args[0]) : 0L;
        boolean dryRun = args.length > 1 && args[1].equalsIgnoreCase("dry-run");

        Memory memory = currentProgram.getMemory();
        Listing listing = currentProgram.getListing();

        AddressSet code = new AddressSet();
        for (MemoryBlock block : memory.getBlocks()) {
            // Microcodigo VU (.vutext) e overlays nao sao codigo do EE.
            if (block.isExecute() && block.isInitialized() && !block.isOverlay()
                    && !block.getName().startsWith(".vu")) {
                code.add(block.getStart(), block.getEnd());
            }
        }

        AddressSet covered = new AddressSet();
        FunctionIterator functions = currentProgram.getFunctionManager().getFunctions(true);
        while (functions.hasNext()) {
            covered.add(functions.next().getBody());
        }

        AddressSetView gaps = code.subtract(covered);
        int created = 0;
        int skippedHuge = 0;
        long gapBytes = gaps.getNumAddresses();

        for (AddressRange range : gaps) {
            if (monitor.isCancelled()) {
                break;
            }
            if (maxGap > 0 && range.getLength() > maxGap) {
                skippedHuge++;
                continue;
            }

            Address cursor = alignUp(range.getMinAddress());
            // Uma instrucao inteira (4 bytes) precisa caber no vao; o ultimo bloco pode terminar desalinhado.
            while (cursor != null && cursor.add(INSTRUCTION_SIZE - 1).compareTo(range.getMaxAddress()) <= 0) {
                if (memory.getInt(cursor) == 0) { // nop de alinhamento
                    cursor = next(cursor);
                    continue;
                }
                if (dryRun) {
                    created++;
                    cursor = next(cursor);
                    continue;
                }

                Function function = null;
                if (listing.getInstructionAt(cursor) != null || disassemble(cursor)) {
                    function = createFunction(cursor, null);
                }
                if (function != null) {
                    created++;
                    Address end = function.getBody().getMaxAddress();
                    cursor = end.compareTo(cursor) >= 0 ? next(end) : next(cursor);
                } else {
                    cursor = next(cursor);
                }
            }
        }

        println("CreateMissingFunctions: vaos=" + gaps.getNumAddressRanges()
                + " bytes_em_vaos=" + gapBytes
                + " funcoes_criadas=" + created
                + " vaos_ignorados_por_tamanho=" + skippedHuge
                + (dryRun ? " (dry-run: contagem de candidatos, nada criado)" : ""));
    }

    private static Address alignUp(Address address) {
        long offset = address.getOffset();
        long rest = offset % INSTRUCTION_SIZE;
        return rest == 0 ? address : address.add(INSTRUCTION_SIZE - rest);
    }

    private static Address next(Address address) {
        try {
            return address.add(INSTRUCTION_SIZE);
        } catch (Exception e) {
            return null;
        }
    }
}
