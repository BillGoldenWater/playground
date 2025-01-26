#include <_abort.h>
#include <errno.h>
#include <signal.h>
#include <setjmp.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>

int DIV_BY_ZERO = 1;
typedef struct DivByZeroException {
    int oprand_a;
} DivByZeroException;

int OTHER = 2;
typedef struct OtherException {
    char* message;
} OtherException;

typedef union Exception {
    DivByZeroException div_by_zero;
    OtherException other;
} Exception;

typedef struct ExceptionCtx {
    jmp_buf jmp;
    int tag;
    Exception payload;
} ExceptionCtx;

ExceptionCtx* cur_exception_ctx;

#define UNHANDLED_EXCEPTION(exception_code) \
    fprintf(stderr, "Unhandled exception, tag: %d\n", exception_code); \
    abort();

#define TRY \
    { \
        ExceptionCtx* prev_ctx = cur_exception_ctx; \
        ExceptionCtx cur_ctx = {0}; \
        cur_exception_ctx = &cur_ctx; \
        if (setjmp(cur_ctx.jmp) == 0)

#define CATCH(exception_code) \
        else if (cur_ctx.tag == exception_code && !(cur_ctx.tag = 0))

#define FINALLY \
        cur_exception_ctx = prev_ctx; \
        if (cur_ctx.tag) { \
            if (cur_exception_ctx) { \
                cur_exception_ctx->tag = cur_ctx.tag; \
                cur_exception_ctx->payload = cur_ctx.payload; \
            } else { \
                UNHANDLED_EXCEPTION(cur_ctx.tag) \
            } \
        }

#define END_TRY \
        if (cur_exception_ctx && cur_exception_ctx->tag) { \
            longjmp(cur_exception_ctx->jmp, -1); \
        } \
    }

#define RAISE(exception_code, exception) \
    if (cur_exception_ctx) { \
        if (cur_exception_ctx->tag) { \
            fprintf(stderr, "Double exception, \
previous exception code: %d, \
current exception code: %d\n", \
                    cur_exception_ctx->tag, \
                    exception_code); \
            abort(); \
        } else { \
            cur_exception_ctx->tag = exception_code; \
            cur_exception_ctx->payload = exception; \
            longjmp(cur_exception_ctx->jmp, -1); \
        } \
    } else { \
        UNHANDLED_EXCEPTION(exception_code) \
    }

int divide(int a, int b) {
    if (b == 0) {
        Exception exception = {.div_by_zero = {.oprand_a = a}};
        RAISE(DIV_BY_ZERO, exception);
    }
    return a / b;
}

int in_catch = 0;
jmp_buf abort_catch_jmp;

void handle_abort(int sig) {
    if (sig != SIGABRT) {
        fprintf(stderr, "unexpected signal: %d\n", sig);
        exit(1);
    }
    if (in_catch) {
        longjmp(abort_catch_jmp, 1);
    } else {
        fprintf(stderr, "unexpected abort: %d\n", sig);
        exit(1);
    }
}

#define CATCH_ABORT \
    in_catch = 1; \
    if (setjmp(abort_catch_jmp) == 0)

#define END_ABORT \
    else { \
        in_catch = 0; \
        cur_exception_ctx = 0; \
        printf("recover from abort\n"); \
    }


int main() {
    if (signal(SIGABRT, handle_abort) == SIG_ERR) {
        fprintf(stderr, "failed to register abort signal handler, errno: %d\n", errno);
        return 1;
    }

    printf("========== case 1: exception and catch and finally\n");
    
    TRY {
        printf("try\n");
        printf("123456 / 5: %d\n", divide(123456, 5));
        printf("654321 / 0: %d\n", divide(654321, 0));
    } CATCH(DIV_BY_ZERO) {
        printf("div by zero: oprand_a: %d\n", cur_ctx.payload.div_by_zero.oprand_a);
    } FINALLY {
        printf("finally\n");
    } END_TRY

    printf("========== case 2: catched exception not leak\n");

    TRY {
        TRY {
            divide(123456, 0);
        } CATCH(DIV_BY_ZERO) {
            printf("catch 2: div by zero: oprand_a: %d\n", cur_ctx.payload.div_by_zero.oprand_a);
        } FINALLY {
            printf("finally 2\n");
        } END_TRY
    } CATCH(DIV_BY_ZERO) {
        printf("catch 1: div by zero: oprand_a: %d\n", cur_ctx.payload.div_by_zero.oprand_a);
    } FINALLY {
        printf("finally 1\n");
    } END_TRY

    printf("========== case 3: uncatched exception propagate to upper level\n");

    TRY {
        TRY {
            Exception exception = {.other = {.message = "custom exception"}};
            RAISE(OTHER, exception);
        } CATCH(DIV_BY_ZERO) {
            printf("catch 2: div by zero: oprand_a: %d\n", cur_ctx.payload.div_by_zero.oprand_a);
        } FINALLY {
            printf("finally 2\n");
        } END_TRY
    } CATCH(OTHER) {
        printf("catch 1: exception: %s\n", cur_ctx.payload.other.message);
    } FINALLY {
        printf("finally 1\n");
    } END_TRY

    printf("========== case 4: unhandled exception\n");

    CATCH_ABORT {
        printf("654321 / 0: %d\n", divide(654321, 0));
    } END_ABORT

    printf("========== case 5: short circuit of unhandled exception \n");

    CATCH_ABORT {
        TRY {
            Exception exception = {.other = {.message = "first exception"}};
            RAISE(OTHER, exception);
        } FINALLY {
            printf("finally\n");
        } END_TRY
    } END_ABORT

    printf("========== case 6: double exception\n");

    CATCH_ABORT {
        TRY {
            TRY {
                Exception exception = {.other = {.message = "first exception"}};
                RAISE(OTHER, exception);
            } FINALLY {
                Exception exception = {.other = {.message = "second exception"}};
                RAISE(OTHER, exception);
            } END_TRY
        } FINALLY {} END_TRY
    } END_ABORT
}