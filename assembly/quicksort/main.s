.global _main
.align 4

; allocate stack memory, size must be multiple of 16
.macro stack_alloc size
    sub sp, sp, \size
.endm

; de-allocate stack memory, size must be multiple of 16
.macro stack_dealloc size
    add sp, sp, \size
.endm

; allocate 16 bytes on the stack and store a register at SP
.macro push reg
    str \reg, [sp, -16]!
.endm

; allocate 16 bytes on the stack and store 2 register at SP
.macro push2 reg, reg2
    stp \reg, \reg2, [sp, -16]!
.endm

; load data at SP into a register, and de-allocate 16 bytes of stack space
.macro pop reg
    ldr \reg, [sp], 16
.endm

; load data at SP into 2 register, and de-allocate 16 bytes of stack space
.macro pop2 reg, reg2
    ldp \reg, \reg2, [sp], 16
.endm

;; x19, ptr; new stack
;; x20, usize; size of new stack
;; x21, ptr; old stack
;; x22, usize; exit code
;_main:
;    ldr x20, =10000000
;
;    mov x0, x20
;    bl malloc
;    mov x19, x0
;
;    mov x21, sp
;
;    add x0, x19, x20
;    mov sp, x0
;    bl actual_main
;    mov x22, x0
;
;    mov sp, x21
;
;    mov x0, x19
;    mov x1, x20
;    bl munmap
;
;    mov x0, x22
;    b exit_with

;debug_fmt:
;    .asciz "debug: %zd, %zd\n"
;    .align 4
;debug_idx_len:
;    .asciz "idx: %zd, len: %zd\n"
;    .align 4
; x19, u64; number count
; x20, ptr; number arr, size by x19
_main:
    push2 lr, x19
    push x20

    ; read input size
    ; exit if empty
    ; x19 = read_a_number
    ; if is_empty: exit
    bl read_a_number
    cmp x1, xzr
    b.ne exit_with_unexpected_end
    mov x19, x0

    ; x0 = x19 * 8
    ; with overflow check
    mov x0, 8
    umulh x1, x19, x0
    mul x0, x19, x0
    cmp x1, xzr
    b.ne exit_with_arr_size_too_large

    ; x20 = malloc(x0)
    bl malloc
    mov x20, x0

    ; read numbers
    mov x0, x20
    mov x1, x19
    bl read_numbers_to_buffer

    ; sort
    mov x0, x20
    mov x1, x19
    bl sort
    
    ; print numbers
    mov x0, x20
    mov x1, x19
    bl print_numbers

    ; free memory
    mov x0, 8
    mul x1, x19, x0
    mov x0, x20
    bl munmap

    pop x20
    pop2 lr, x19
    b exit

;params:
;    x0, *u64; arr addr
;    x1, usize; arr len
;
; x19, *u64; arr addr
; x20, usize; arr len
; x21, usize; index
; x22, u64; cur number
; x23, u64; next number
; x24, u64; cur number char index
print_numbers:
    push2 lr, x19
    push2 x20, x21
    push2 x22, x23
    push x24
    ; for 64 bit unsigned integer, 
    ; in ascii, max len is 20 bytes,
    ; with space and null, is 22 bytes, 32 bytes is enough

    ; number space start at SP + 29, decrease
    stack_alloc 32

    ; addr = arg_addr
    mov x19, x0
    ; len = arg_len
    mov x20, x1

    ; sp[30] = ' '
    mov w0, 32
    strb w0, [sp, 30]
    ; sp[31] = '\0'
    mov w0, 0
    strb w0, [sp, 31]

    ; index = 0
    mov x21, xzr
    ; cur_ch = 0
    mov x24, xzr

    ; cur = arr[index]
    mov x0, 8
    mul x0, x21, x0
    ldr x22, [x19, x21]
    ; while index != len
Lprint_numbers_while_start:
    cmp x21, x20
    b.eq Lprint_numbers_while_end

    ; next = cur / 10
    mov x0, 10
    udiv x23, x22, x0
    ; x0 = cur - (next * 10)
    msub x0, x23, x0, x22

    ; x0 = '0' + x0
    add x0, x0, 48
    ; x1 = 29 - cur_ch
    mov x1, 29
    sub x1, x1, x24
    ; sp[x1] = w0
    strb w0, [sp, x1]

    ; x1 = 29 - cur_ch
    mov x1, 29
    sub x1, x1, x24

    ; next digit

    ; cur = next
    mov x22, x23
    ; cur_ch += 1
    add x24, x24, 1

    ; if cur != 0
    cmp x22, xzr
    b.ne Lprint_numbers_while_start
    ; next number

    ; print(sp + x1)
    add x0, sp, x1
    bl print
    ; index += 1
    add x21, x21, 1
    ; cur = arr[index]
    mov x0, 8
    mul x0, x21, x0
    ldr x22, [x19, x0]
    ; cur_ch = 0
    mov x24, 0
    b Lprint_numbers_while_start
Lprint_numbers_while_end:

    mov w0, 10
    strb w0, [sp, 30]
    add x0, sp, 30
    bl print

    stack_dealloc 32
    pop x24
    pop2 x22, x23
    pop2 x20, x21
    pop2 lr, x19
    ret

;params:
;    x0, ptr; u64 array to sort
;    x1, usize; array len
;
; x19, ptr; arr ptr
; x20, usize; arr len
sort:
    b quicksort

;params:
;    x0, ptr; u64 array
;    x1, usize; arr len
;
; x19, ptr; arr ptr
; x20, usize; arr len
; x21, usize; left 
; x22, usize; right
; x23, u64; pivot
quicksort: 
    push2 lr, x19
    push2 x20, x21
    push2 x22, x23

    ; arr = arg_arr
    mov x19, x0
    ; len = arg_len
    mov x20, x1

    ; terminate check
    ; len <= 1
    cmp x20, 1
    b.ls Lquicksort_ret

    ; l = 0
    mov x21, 0
    ; r = len - 1
    sub x22, x20, 1
    ; pivot = arr[l]
    mov x0, 8
    mul x0, x21, x0
    ldr x23, [x19, x0]

    ; while l != r
Lquicksort_while_start:
    cmp x21, x22
    b.eq Lquicksort_while_end 

    ;; dbg print arr
    ;mov x0, x19
    ;mov x1, x20
    ;bl print_numbers
    ;; dbg print l and r
    ;push2 x21, x22
    ;adr x0, debug_fmt
    ;bl _printf
    ;pop2 x21, x22

    ; if arr[l] > pivot
    mov x0, 8
    mul x0, x21, x0
    ldr x0, [x19, x0]
    cmp x0, x23
    b.hi Lquicksort_while_l_gt_pivot
    ; if arr[r] < pivot
    mov x0, 8
    mul x0, x22, x0
    ldr x0, [x19, x0]
    cmp x0, x23
    b.lo Lquicksort_while_r_lt_pivot
    ; else
    ; r--
    sub x22, x22, 1
    b Lquicksort_while_if_end
Lquicksort_while_l_gt_pivot:
    ; swap
    mov x0, 8
    mul x2, x21, x0
    mul x3, x22, x0

    ldr x0, [x19, x2]
    ldr x1, [x19, x3]
    str x0, [x19, x3]
    str x1, [x19, x2]
    ; r--
    sub x22, x22, 1
    b Lquicksort_while_if_end
Lquicksort_while_r_lt_pivot:
    ; swap
    mov x0, 8
    mul x2, x21, x0
    mul x3, x22, x0

    ldr x0, [x19, x2]
    ldr x1, [x19, x3]
    str x0, [x19, x3]
    str x1, [x19, x2]
    ; l++
    add x21, x21, 1
Lquicksort_while_if_end:
    b Lquicksort_while_start
Lquicksort_while_end:

    ; arg_ptr = ptr
    mov x0, x19
    ; arg_len = l + 1
    add x1, x21, 1
    ; dbg print idx, len
    ;mov x0, 0
    ;push2 x0, x1
    ;adr x0, debug_idx_len
    ;bl _printf
    ;pop2 x0, x1
    ;mov x0, x19
    ; debug end
    bl quicksort

    ; arg_ptr = ptr + ((l + 1) * 8)
    ; x1 = l + 1
    add x1, x21, 1
    ; x1 = x1 * 8
    mov x0, 8
    mul x1, x1, x0
    ; arg_ptr = ptr + x1
    add x0, x19, x1
    ; arg_len = len - (l + 1)
    add x1, x21, 1
    sub x1, x20, x1
    ; dbg print idx, len
    ;push2 x0, x1
    ;sub x0, x0, x19
    ;mov x2, 8
    ;udiv x0, x0, x2
    ;push2 x0, x1
    ;adr x0, debug_idx_len
    ;bl _printf
    ;pop2 xzr, x1
    ;pop2 x0, x1
    ; debug end
    bl quicksort

Lquicksort_ret:
    pop2 x22, x23
    pop2 x20, x21
    pop2 lr, x19
    ret

;params:
;    x0, ptr; buffer ptr
;    x1, usize; target size
;
; x19, usize; target size
; x20, ptr; buffer ptr
; x21, usize; loop count
read_numbers_to_buffer:
    push2 lr, x19
    push2 x20, x21
    ;stack_alloc 16

    mov x19, x1
    mov x20, x0
    mov x21, 0

    ; read numbers, size by x19
    ; x21: loop count
    Lread_loop:
    cmp x21, x19
    b.eq Lend_read_loop

    ;; read a number, exit if empty
    bl read_a_number
    cmp x1, xzr
    b.ne exit_with_unexpected_end

    ;add x0, sp, 8
    ;str x0, [sp]
    ;adr x0, scanf_fmt
    ;bl _scanf
    ;ldr x0, [sp, 8]

    ; x1 = x21 * 8
    mov x2, 8
    mul x1, x21, x2
    str x0, [x20, x1]

    add x21, x21, 1

    b Lread_loop
    Lend_read_loop:

    ;stack_dealloc 16
    pop2 x20, x21
    pop2 lr, x19
    ret
;scanf_fmt:
;    .asciz "%zu"
;    .align 4


;return:
;    x0, u64; number
;    x1, bool; is_empty
;
; x19, bool; is inside number
; x20, byte; current byte
; x21, u64; result
read_a_number:
    push2 lr, x19
    push2 x20, x21

    mov x19, 0
    mov x20, 0
    mov x21, 0

    stack_alloc 16
    Lread_a_number__read:
    mov x0, 0
    mov x1, sp
    mov x2, 1
    bl read

    ; check read error
    cmp x0, 0
    b.lt exit_syscall_err

    ldrb w0, [sp] ; read buf
    mov x20, x0
    bl valid_input

    cmp x20, 10
    b.eq Lread_a_number__done_read ; break if LF

    cmp x19, 0
    b.eq Lread_a_number__is_in_number_else
    ; inside number

    cmp x20, 32
    b.eq Lread_a_number__done_read ; break if space
    ; ascii to digit
    sub x20, x20, 48

    ; clear flags
    mov x0, 0
    msr nzcv, x0
    ; x21 * 10 + x20
    ; with overflow check
    mov x0, 10
    umulh x1, x21, x0
    mul x21, x21, x0
    cmp x1, xzr
    b.ne exit_with_number_overflow
    add x21, x21, x20

    b Lread_a_number__is_in_number_endif
    Lread_a_number__is_in_number_else: 
    ; outside number

    cmp x20, 32
    b.eq Lread_a_number__read ; continue if is space
    mov x19, 1
    sub x21, x20, 48

    Lread_a_number__is_in_number_endif:
    b Lread_a_number__read ; loop read

    Lread_a_number__done_read:
    stack_dealloc 16

    mov x0, x21
    mov x1, 0

    cmp x19, xzr
    b.ne Lread_a_number__is_empty_endif
    ; outside number

    mov x1, 1

    Lread_a_number__is_empty_endif:

    pop2 x20, x21
    pop2 lr, x19
    ret

;params:
;    x0 input byte
valid_input:
    cmp x0, 10
    b.eq Lvalid_input_ret
    cmp x0, 32
    b.eq Lvalid_input_ret
    cmp x0, 48
    b.lo exit_with_invalid_input
    cmp x0, 57
    b.hi exit_with_invalid_input
Lvalid_input_ret:
    ret

LCmsg_input_unexpected_character:
    .asciz "unexpected character\n"
    .align 4
exit_with_invalid_input:
    adr x0, LCmsg_input_unexpected_character
    b exit_with_msg

LCmsg_input_number_overflow:
    .asciz "number overflow\n"
    .align 4
exit_with_number_overflow:
    adr x0, LCmsg_input_number_overflow
    b exit_with_msg

LCmsg_input_array_size_too_large:
    .asciz "array size too large\n"
    .align 4
exit_with_arr_size_too_large:
    adr x0, LCmsg_input_array_size_too_large
    b exit_with_msg

LCmsg_input_unexpected_end:
    .asciz "unexpected end of input\n"
    .align 4
exit_with_unexpected_end:
    adr x0, LCmsg_input_unexpected_end
    b exit_with_msg

;params:
;    x0 adderss of null terminated message string
exit_with_msg:
    bl print

    mov x0, 1
    b exit_with

;count the length of a null terminated string
;(excluding the terminating null character)
;(https:;cplusplus.com/reference/cstring/strlen/)
;
;params:
;    x0 string start address
;return:
;    length by x0
str_len:
    mov x1, 0
Lstr_len_count_start:
    ldrb w2, [x0], 1
    cmp w2, 0
    cinc x1, x1, ne
    b.ne Lstr_len_count_start

    mov x0, x1
    ret

;params:
;    x0 address
;    x1 length
write_stdout: 
    push lr

    mov x2, x1
    mov x1, x0
    mov x0, 1
    bl write

    pop lr
    ret

;params:
;    x0, ptr; address of null terminated string
print:
    push2 x19, lr

    mov x19, x0
    bl str_len

    mov x1, x0
    mov x0, x19
    bl write_stdout

    pop2 x19, lr
    ret

;params: 
;    x0 fd
;    x1 address
;    x2 length
write:
    mov x16, 4
    svc 0x80
    ret

;params: 
;    x0 fd
;    x1 out buf address
;    x2 length limit
read:
    mov x16, 3
    svc 0x80
    ret

;params:
;    x0 size
;return:
;    x0 addr
malloc:
    push lr

    mov x1, x0 ; size

    mov x0, 0 ; addr
    mov x2, 3 ; prot PROT_READ | PROT_WRITE
    mov x3, 4098 ; flag MAP_PRIVATE | MAP_ANONYMOUS
    mov x4, -1 ; fd
    mov x5, 0 ; offset
    bl mmap

    cmp x0, 0
    b.lt exit_syscall_err

    pop lr
    ret

mmap:
    mov x16, 197
    svc 0x80
    ret

;params:
;    x0 addr
;    x1 size
munmap:
    mov x16, 73
    svc 0x80
    ret

LCmsg_error_syscall:
    .asciz "syscall error\n"
    .align 4
exit_syscall_err:
    adr x0, LCmsg_error_syscall
    bl print

    ldr x0, =0x0000000180ca17d4
    blr x0

    b exit_with

exit: 
    mov x0, 0
exit_with:
    mov x16, 1
    svc 0x80

; vim:filetype=asm
