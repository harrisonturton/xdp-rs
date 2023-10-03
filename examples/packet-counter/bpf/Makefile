CC = clang
CFLAGS = -Wall -Wextra -O3 -g
BPF_TARGET = -g -target bpf

bpf.o: src/bpf.c
	$(CC) $(CFLAGS) $(BPF_TARGET) -c $< -o $@

.PHONY: clean
clean:
	rm bpf.o
