#include <stdlib.h>
#include <time.h>

int main(void)
{
    srand(time(NULL));
    char *gosholosho = "gOsHoLoShO";
    return gosholosho[rand() % 10];
}
