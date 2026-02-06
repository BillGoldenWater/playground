#include <stdio.h>
#include <unistd.h>

int main(int argc, char **argv) {
    if (argc != 1) return -1;
    sleep(1);
    printf("Hello, from %s\n", argv[0]);
    return 0;
}
