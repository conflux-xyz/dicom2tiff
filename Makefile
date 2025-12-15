clippy:
	cargo clippy --all-targets --all-features -- -D warnings

format:
	cargo fmt

build-wasm:
	wasm-pack build crates/wasm --target web --scope conflux-xyz
	# We want the final package to be `@conflux-xyz/dicom2tiff` instead of `@conflux-xyz/dicom2tiff-wasm`, but
	# changing the name in Cargo.toml conflicts with also wanting the core crate to be called `dicom2tiff`.
	# So we just change the package name after building.
	sed 's/"name": "@conflux-xyz\/dicom2tiff-wasm"/"name": "@conflux-xyz\/dicom2tiff"/g' crates/wasm/pkg/package.json > crates/wasm/pkg/package.json.tmp && mv crates/wasm/pkg/package.json.tmp crates/wasm/pkg/package.json
	sed 's/dicom2tiff_wasm/dicom2tiff/g' crates/wasm/pkg/package.json > crates/wasm/pkg/package.json.tmp && mv crates/wasm/pkg/package.json.tmp crates/wasm/pkg/package.json
	mv crates/wasm/pkg/dicom2tiff_wasm.d.ts crates/wasm/pkg/dicom2tiff.d.ts
	mv crates/wasm/pkg/dicom2tiff_wasm.js crates/wasm/pkg/dicom2tiff.js
	mv crates/wasm/pkg/dicom2tiff_wasm_bg.wasm crates/wasm/pkg/dicom2tiff_bg.wasm
	mv crates/wasm/pkg/dicom2tiff_wasm_bg.wasm.d.ts crates/wasm/pkg/dicom2tiff_bg.wasm.d.ts
	sed 's/dicom2tiff_wasm_bg/dicom2tiff_bg/g' crates/wasm/pkg/dicom2tiff.js > crates/wasm/pkg/dicom2tiff.js.tmp && mv crates/wasm/pkg/dicom2tiff.js.tmp crates/wasm/pkg/dicom2tiff.js

build-cli:
	cargo build -p dicom2tiff-cli --release

build-gh-pages: build-wasm
	rm -rf docs
	mkdir docs/
	touch docs/.nojekyll
	cp examples/web/index.html docs/
	cp examples/web/worker.js docs/
	cp examples/web/logo* docs/
	sed 's|../../crates/wasm/pkg/dicom2tiff.js|./wasm/dicom2tiff.js|g' docs/worker.js > docs/worker.js.tmp && mv docs/worker.js.tmp docs/worker.js
	mkdir docs/wasm
	rsync -av --exclude='.gitignore' crates/wasm/pkg/ docs/wasm/