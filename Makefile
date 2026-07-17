SDK_DIR := ../../openagent-go/cmd/cli/sdk/rust
PLUGINS := extended-settings stats-cmd telemetry hdspace-models
OUT_DIR := build/plugins
TARGET  := wasm32-unknown-unknown
CARGO   := cargo

.PHONY: all clean build-sdk $(PLUGINS)

all: build-sdk $(PLUGINS)

build-sdk:
	cd $(SDK_DIR) && $(CARGO) build --release --target $(TARGET)

$(PLUGINS):
	@mkdir -p $(OUT_DIR)
	$(CARGO) build --release --target $(TARGET) -p $@
	cp target/$(TARGET)/release/$(subst -,_,$@).wasm $(OUT_DIR)/$@.wasm

clean:
	rm -rf $(OUT_DIR)
	$(CARGO) clean
