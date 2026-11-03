#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <time.h>

int main(int argc, char const *argv[]) {
    if (argc != 2) {
        char *err = ": Invalid number of arguments.\n";
        write(STDERR_FILENO, argv[0], strlen(argv[0]));
        write(STDERR_FILENO, err, strlen(err));
        return 1;
    }

    srand(time(NULL));
    return argv[1][rand() % strlen(argv[1])];
}
