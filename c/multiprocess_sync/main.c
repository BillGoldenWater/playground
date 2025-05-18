#include <assert.h>
#include <stdio.h>
#include <stdatomic.h>
#include <sys/fcntl.h>
#include <sys/mman.h>
#include <unistd.h>

enum State {
    Uninitialized = 0,
    Initializing = 1,
    Initialized = 2,
};

struct Mem {
    atomic_int state;
    atomic_int ready;
    atomic_int finished;

    atomic_int count;
};

char *SHM_NAME = "/playground/shared";
int SHM_SIZE = 1024;

int main() {
    int fd = shm_open(SHM_NAME, O_CREAT | O_RDWR, 0640);
    ftruncate(fd, SHM_SIZE);

    void *ptr = mmap(0, SHM_SIZE, PROT_WRITE, MAP_SHARED, fd, 0);
    struct Mem *mem = ptr;

    int expected = Uninitialized;
    if (atomic_compare_exchange_strong(&mem->state, &expected, Initializing)) {
        printf("initializing\n");
        mem->ready = 0;
        mem->finished = 0;

        mem->count = 0;
        atomic_store(&mem->state, Initialized);
    } else {
        printf("wait initialized\n");
        while (atomic_load(&mem->state) == Initializing);
    }
    assert(atomic_load(&mem->state) == Initialized);

    printf("wait ready\n");
    atomic_fetch_add(&mem->ready, 1);
    while (atomic_load(&mem->ready) < 2);
    for (int i = 0; i < 100; i++) {
        atomic_fetch_add(&mem->count, 1);
    }
    printf("wait finished\n");
    atomic_fetch_add(&mem->finished, 1);
    while (atomic_load(&mem->finished) < 2);

    int result = atomic_load(&mem->count);
    printf("result: %d\n", result);
    atomic_store(&mem->state, 0);

    munmap(ptr, SHM_SIZE);
    close(fd);
    shm_unlink(SHM_NAME);
    return 0;
}