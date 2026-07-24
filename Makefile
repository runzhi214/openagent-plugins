PLUGINS := hdspace-models hdspace-renew
OUT_DIR := build/plugins
TARGET  := wasm32-unknown-unknown
CARGO   := cargo

.PHONY: all clean docker $(PLUGINS)

all: $(PLUGINS)

$(PLUGINS):
	@mkdir -p $(OUT_DIR)
	$(CARGO) build --release --target $(TARGET) -p $@
	cp target/$(TARGET)/release/$(subst -,_,$@).wasm $(OUT_DIR)/$@.wasm

docker:
	@mkdir -p $(OUT_DIR)
	docker build --output type=local,dest=$(OUT_DIR) .

clean:
	rm -rf $(OUT_DIR)
	$(CARGO) clean
