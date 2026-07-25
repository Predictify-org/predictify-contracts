#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <process.h>

int main(int argc, char* argv[]) {
    char** new_argv = malloc((argc + 5) * sizeof(char*));
    if (!new_argv) return 1;
    int new_argc = 0;
    
    new_argv[new_argc++] = "C:\\Users\\NEW USER\\.rustup\\toolchains\\stable-x86_64-pc-windows-gnu\\lib\\rustlib\\x86_64-pc-windows-gnu\\bin\\self-contained\\x86_64-w64-mingw32-gcc.exe";
    new_argv[new_argc++] = "-c";
    
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--64") == 0 || strcmp(argv[i], "--32") == 0 || strcmp(argv[i], "--no-leading-underscore") == 0) {
            continue;
        }
        new_argv[new_argc++] = argv[i];
    }
    new_argv[new_argc] = NULL;
    
    intptr_t ret = _spawnv(_P_WAIT, new_argv[0], (const char* const*)new_argv);
    free(new_argv);
    return (int)ret;
}
