#include <stdlib.h>
#include <time.h>

int main(void)
{
    srand(time(NULL));
    int r = rand();
    return r % 128;
}
