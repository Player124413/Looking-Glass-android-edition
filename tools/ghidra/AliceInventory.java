// Local analysis only. No decompiled implementation is copied into the Rust project.
// @category Alice.Research
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;

public class AliceInventory extends GhidraScript {
    private String clean(Object value) { return String.valueOf(value).replace('\t',' ').replace('\n',' ').replace('\r',' '); }
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length != 1) throw new IllegalArgumentException("Expected private output directory");
        Path output = Paths.get(args[0]); Files.createDirectories(output);
        String name = currentProgram.getName();
        int functions = 0, strings = 0, links = 0;
        try (PrintWriter writer = new PrintWriter(Files.newBufferedWriter(output.resolve(name + ".functions.tsv"), StandardCharsets.UTF_8))) {
            writer.println("address\tname\tbody_bytes\texternal");
            FunctionIterator it = currentProgram.getFunctionManager().getFunctions(true);
            while(it.hasNext() && !monitor.isCancelled()) {
                Function f = it.next(); functions++;
                writer.println(f.getEntryPoint()+"\t"+clean(f.getName())+"\t"+f.getBody().getNumAddresses()+"\t"+f.isExternal());
            }
        }
        try (PrintWriter writer = new PrintWriter(Files.newBufferedWriter(output.resolve(name + ".format-xrefs.tsv"), StandardCharsets.UTF_8))) {
            writer.println("string_address\treference_address\tfunction\ttext");
            DataIterator it = currentProgram.getListing().getDefinedData(true);
            while(it.hasNext() && !monitor.isCancelled()) {
                Data d = it.next(); Object value = d.getValue();
                if (!(value instanceof String)) continue;
                String s = (String)value; String lower = s.toLowerCase();
                if (!(lower.contains(".bsp") || lower.contains(".ftx") || lower.contains(".tik") || lower.contains(".ska") || lower.contains(".skb") || lower.contains(".pk3") || lower.contains("wrong version") || lower.contains("lightmap") || lower.contains("loadsurfaces") || lower.contains("getgameapi") || lower.contains("clientthink") || lower.contains("spawnentities"))) continue;
                strings++;
                ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(d.getAddress());
                boolean found = false;
                while(refs.hasNext()) {
                    Reference ref = refs.next(); Function f = currentProgram.getFunctionManager().getFunctionContaining(ref.getFromAddress());
                    writer.println(d.getAddress()+"\t"+ref.getFromAddress()+"\t"+clean(f == null ? "" : f.getName())+"\t"+clean(s)); found = true; links++;
                }
                if(!found) writer.println(d.getAddress()+"\t\t\t"+clean(s));
            }
        }
        try (PrintWriter writer = new PrintWriter(Files.newBufferedWriter(output.resolve(name + ".summary.txt"), StandardCharsets.UTF_8))) {
            writer.println("Program: " + name);
            writer.println("SHA-256: " + currentProgram.getExecutableSHA256());
            writer.println("Language: " + currentProgram.getLanguageID());
            writer.println("Image base: " + currentProgram.getImageBase());
            writer.println("Functions: " + functions);
            writer.println("Format strings: " + strings);
            writer.println("Cross-references: " + links);
        }
        println("ALICE_INVENTORY " + name + " functions=" + functions + " strings=" + strings + " xrefs=" + links);
    }
}
