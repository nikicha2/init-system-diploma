CC = gcc
CFLAGS = -Wall -Wextra -pedantic -std=c11

SRC_DIR = src
BIN_DIR = bin

SRCS = $(wildcard $(SRC_DIR)/*.c)
TARGETS = $(patsubst $(SRC_DIR)/%.c, $(BIN_DIR)/%, $(SRCS))

all: $(TARGETS)

# Rule: How to build 'bin/%' from 'src/%.c'
# $< represents the input file (src/file.c)
# $@ represents the output file (bin/file)
$(BIN_DIR)/%: $(SRC_DIR)/%.c
	$(CC) $(CFLAGS) $< -o $@

clean:
	rm -f $(TARGETS)

.PHONY: all clean