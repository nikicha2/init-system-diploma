#include <unistd.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char *argv[])
{
    if (argc != 2) {
        char *err = ": Invalid number of arguments.\n";
        write(STDERR_FILENO, argv[0], strlen(argv[0]));
        write(STDERR_FILENO, err, strlen(err));
        return 1;
    }

    srand(atoi(argv[1]));
    int r = rand();
    return r % 128;
}
