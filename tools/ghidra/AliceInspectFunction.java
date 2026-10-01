// Export only specifically selected functions to the local private research folder.
// @category Alice.Research
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.listing.Function;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;

public class AliceInspectFunction extends GhidraScript {
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if(args.length < 2) throw new IllegalArgumentException("Expected private directory and function addresses");
        Path directory = Paths.get(args[0]); Files.createDirectories(directory);
        DecompInterface decompiler = new DecompInterface();
        try {
            decompiler.openProgram(currentProgram);
            for(int i=1;i<args.length;i++) {
                Function function = getFunctionAt(toAddr(args[i]));
                if(function == null) throw new IllegalArgumentException("No function at " + args[i]);
                DecompileResults result = decompiler.decompileFunction(function,30,monitor);
                if(!result.decompileCompleted()) throw new IllegalStateException(result.getErrorMessage());
                Path file = directory.resolve(currentProgram.getName()+"-"+function.getEntryPoint()+".c");
                Files.writeString(file,"// LOCAL RESEARCH OUTPUT: NOT RELEASE SOURCE\n"+result.getDecompiledFunction().getC(),StandardCharsets.UTF_8);
                println("ALICE_FUNCTION " + file);
            }
        } finally { decompiler.dispose(); }
    }
}
