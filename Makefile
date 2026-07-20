PLUGINS := hdspace-models stats-cmd telemetry
OUT_DIR := build/plugins
TARGET  := wasm32-unknown-unknown
CARGO   := cargo

.PHONY: all clean $(PLUGINS)

all: $(PLUGINS)

$(PLUGINS):
	@mkdir -p $(OUT_DIR)
	$(CARGO) build --release --target $(TARGET) -p $@
	cp target/$(TARGET)/release/$(subst -,_,$@).wasm $(OUT_DIR)/$@.wasm

clean:
	rm -rf $(OUT_DIR)
	$(CARGO) clean
