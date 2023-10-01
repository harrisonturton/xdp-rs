CC = clang
CFLAGS = -Wall -Wextra -O3 -g
BPF_TARGET = -g -target bpf

kernel_prog.o: src/kernel_prog.c
	$(CC) $(CFLAGS) $(BPF_TARGET) -c $< -o $@

.PHONY: clean
clean:
	rm kernel_prog
