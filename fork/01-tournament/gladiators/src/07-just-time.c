#include <stdlib.h>
#include <time.h>

int main(void)
{
    return time(NULL) % 128;
}

