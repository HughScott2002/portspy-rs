# portspy — command cheat-sheet.
#
# Run `make` (or `make help`) to see everything. Each target is one obvious
# command so you don't have to remember the long cargo invocations.

# The wasm target and output paths, named once so they stay in sync.
WASM_TARGET := wasm32-unknown-unknown
WASM_ARTIFACT := target/$(WASM_TARGET)/release/portspy_wasm.wasm
WASM_DEST := web/portspy.wasm

# `make` with no target prints help.
.DEFAULT_GOAL := help

.PHONY: help setup build print wasm serve clean fmt

help: ## show this help
	@echo "portspy — make targets:"
	@echo ""
	@echo "  make setup   install the wasm compile target (one time)"
	@echo "  make build   compile the native portspy binary (release)"
	@echo "  make print   build, then list listening ports in the terminal"
	@echo "  make wasm    compile the browser wasm module + copy it into web/"
	@echo "  make serve   build everything, then run the web dashboard"
	@echo "  make fmt     format all the Rust code"
	@echo "  make clean   remove build artifacts"
	@echo ""

setup: ## install the wasm32 compile target (needed once before `make wasm`)
	rustup target add $(WASM_TARGET)

build: ## compile the native binary in release mode
	cargo build --release -p portspy-cli

print: build ## list listening ports in the terminal
	./target/release/portspy print

wasm: ## compile the wasm module and copy it into web/
	cargo build --release -p portspy-wasm --target $(WASM_TARGET)
	cp $(WASM_ARTIFACT) $(WASM_DEST)
	@echo "wrote $(WASM_DEST)"

serve: build wasm ## build native + wasm, then start the web dashboard
	./target/release/portspy serve

fmt: ## format every crate
	cargo fmt

clean: ## remove build artifacts and the generated wasm
	cargo clean
	rm -f $(WASM_DEST)
