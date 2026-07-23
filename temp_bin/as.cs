using System;
using System.Diagnostics;
using System.IO;
using System.Linq;

class Program {
    static int Main(string[] args) {
        string outputFile = null;
        string inputFile = null;
        
        for (int i = 0; i < args.Length; i++) {
            if (args[i] == "-o" && i + 1 < args.Length) {
                outputFile = args[i + 1];
                i++;
            } else if (args[i].EndsWith(".s", StringComparison.OrdinalIgnoreCase)) {
                inputFile = args[i];
            }
        }
        
        if (outputFile == null || inputFile == null) {
            if (args.Length > 0) {
                inputFile = args[args.Length - 1];
            }
            if (outputFile == null) {
                Console.Error.WriteLine("as wrapper error: Output file not specified (-o)");
                return 1;
            }
        }
        
        inputFile = Path.GetFullPath(inputFile);
        outputFile = Path.GetFullPath(outputFile);
        
        string inputDir = Path.GetDirectoryName(inputFile);
        string wrapperPath = Path.Combine(inputDir, "dlltool_asm_wrapper_" + Guid.NewGuid().ToString("N") + ".rs");
        string inputBasename = Path.GetFileName(inputFile);
        
        string rustCode = "#![no_std]\ncore::arch::global_asm!(include_str!(\"" + inputBasename + "\"));\n";
        File.WriteAllText(wrapperPath, rustCode);
        
        var startInfo = new ProcessStartInfo {
            FileName = "rustc",
            Arguments = "--target=x86_64-pc-windows-gnu --crate-type=lib --emit=obj -o \"" + outputFile + "\" \"" + wrapperPath + "\"",
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardError = true,
            RedirectStandardOutput = true
        };
        
        int exitCode = 1;
        try {
            using (var process = Process.Start(startInfo)) {
                string stdout = process.StandardOutput.ReadToEnd();
                string stderr = process.StandardError.ReadToEnd();
                process.WaitForExit();
                exitCode = process.ExitCode;
                
                if (exitCode != 0) {
                    Console.Error.WriteLine("rustc assembler wrapper failed:");
                    Console.Error.WriteLine(stderr);
                    Console.WriteLine(stdout);
                }
            }
        } catch (Exception ex) {
            Console.Error.WriteLine("Error launching rustc: " + ex.Message);
        } finally {
            try {
                if (File.Exists(wrapperPath)) {
                    File.Delete(wrapperPath);
                }
            } catch {}
        }
        
        return exitCode;
    }
}
