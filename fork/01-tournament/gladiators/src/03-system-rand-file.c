#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>

int main(void) {
    unsigned char num;
    int fd = open("/dev/urandom", O_RDONLY);
    read(fd, &num, 1);
    close(fd);
    return (num % 128) + 1;
}
