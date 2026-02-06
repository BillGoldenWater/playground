#include <assert.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

int main() {
    printf("executing\n");

    int ret = 0;
    // rx, tx
    int filedes[2] = {0, 0};
    ret = pipe(filedes);
    if (ret == -1) {
        printf("unable to create pipe: %s\n", strerror(errno));
        return -1;
    }

    ret = fork();
    if (ret == -1) {
        printf("unable to fork: %s\n", strerror(errno));
        return -1;
    }
    if (ret == 0) {
        int ret = dup2(filedes[1], STDOUT_FILENO);
        if (ret == -1) {
            printf("unable to replace stdout: %s\n", strerror(errno));
            return -1;
        }
        ret = close(filedes[0]);
        if (ret == -1) {
            printf("unable to close pipe rx at child side: %s\n", strerror(errno));
            return -1;
        }
        ret = close(filedes[1]);
        if (ret == -1) {
            printf("unable to close orignal pipe tx at child side: %s\n", strerror(errno));
            return -1;
        }

        char *const argv[] = {"child", NULL};
        char *const envp[] = {NULL};
        ret = execve("./child", argv, envp);
        if (ret == -1) {
            printf("unable to exec: %s\n", strerror(errno));
            return -1;
        }
    }
    int child_pid = ret;

    ret = close(filedes[1]);
    if (ret == -1) {
        printf("unable to close pipe tx at parent side: %s\n", strerror(errno));
        return -1;
    }

    size_t buf_size = 4096;
    char *buf = malloc(buf_size);
    while (1) {
        int n = read(filedes[0], buf, buf_size);
        if (n == -1) {
            printf("unable to read child stdout: %s\n", strerror(errno));
            return -1;
        }
        if (n == 0) break;

        write(STDOUT_FILENO, buf, n);
    }
    free(buf);

    int stat_loc = 0;
    int pid = waitpid(child_pid, &stat_loc, 0);
    assert(pid == child_pid);

    return 0;
}
