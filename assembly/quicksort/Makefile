# main.o: main.s
# 	as -o main.o main.s 
#
# main: main.o
# 	ld -o main main.o
main: main.s
	cc -L/opt/homebrew/opt/llvm/lib main.s -o main

run: main
	./main

leaks: main
	leaks -atExit -- ./main
