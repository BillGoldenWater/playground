TARGET_DIR := $(strip $(shell cargo metadata --format-version 1 | jq -r .target_directory))
OUT := $(TARGET_DIR)/release/onebrc

SOURCES := $(shell find src -type f) ./Cargo.toml
# .PHONY: $(OUT)

samply: $(OUT)
	samply record $(OUT)

hyperfine: $(OUT)
	hyperfine $(OUT)

perf_stat: $(OUT)
	sudo perf stat $(OUT)

perf_report: $(OUT)
	sudo perf record --call-graph dwarf $(OUT)
	sudo perf report

$(OUT): $(SOURCES)
	RUSTFLAGS="-C target-cpu=native" cargo build --release
