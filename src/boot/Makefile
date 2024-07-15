ifeq ($(PLATFORM),windows32)
EXESUFFIX:=.exe
MKDIR := mkdir
else
EXESUFFIX:=
MKDIR := mkdir
endif

BIN := bin
UTIL := util
LOGO := logo

NATIVE_CC := gcc
PYTHON := python3
PB12_COMPRESS_C := pb12$(EXESUFFIX)
PB12_COMPRESS_PYTHON := $(PYTHON) $(UTIL)/pb12.py compress
PB12_COMPRESS := $(PB12_COMPRESS_C)
LRE_COMPRESS := $(PYTHON) $(UTIL)/lre.py compress
RGBASM  := $(RGBDS)rgbasm
RGBLINK := $(RGBDS)rgblink
RGBGFX := $(RGBDS)rgbgfx

RGBASM_FLAGS := --include .
RGBGFX_FLAGS := -Z -c embedded

all: $(BIN)/cgb_boot.bin $(BIN)/cgb_boot_fast.bin

$(BIN)/%.bin: %.asm logo.lre
	-@$(MKDIR) -p $(dir $@)
	$(RGBASM) $(RGBASM_FLAGS) -o $@.tmp $<
	$(RGBLINK) -x -o $@ $@.tmp
	@rm $@.tmp

$(PB12_COMPRESS_C): $(UTIL)/pb12.c
	$(NATIVE_CC) -std=c99 -Wall -Werror $< -o $@

%.lre: %.2bpp
	$(LRE_COMPRESS) $< $@

%.pb12: %.2bpp
	$(PB12_COMPRESS) $< $@

%.2bpp: %.png
	$(RGBGFX) $(RGBGFX_FLAGS) -o $@ $<

clean:
	rm -rf *.lre *.pb12 *.2bpp $(BIN) $(PB12_COMPRESS_C)
