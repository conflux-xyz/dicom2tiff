clippy:
	cargo clippy --all-targets --all-features -- -D warnings

format:
	cargo fmt

build-wasm:
	wasm-pack build crates/wasm --target web --scope conflux-xyz
	# We want the final package to be `@conflux-xyz/dicom2tiff` instead of `@conflux-xyz/dicom2tiff-wasm`, but
	# changing the name in Cargo.toml conflicts with also wanting the core create to be called `dicom2tiff`.
	# So we just change the package name after building.
	sed -i ".bak" -e 's/"name": "@conflux-xyz\/dicom2tiff-wasm"/"name": "@conflux-xyz\/dicom2tiff"/g' crates/wasm/pkg/package.json
	sed -i ".bak" -e 's/dicom2tiff_wasm/dicom2tiff/g' crates/wasm/pkg/package.json
	rm crates/wasm/pkg/package.json.bak
	mv crates/wasm/pkg/dicom2tiff_wasm.d.ts crates/wasm/pkg/dicom2tiff.d.ts
	mv crates/wasm/pkg/dicom2tiff_wasm.js crates/wasm/pkg/dicom2tiff.js
	mv crates/wasm/pkg/dicom2tiff_wasm_bg.wasm crates/wasm/pkg/dicom2tiff_bg.wasm
	mv crates/wasm/pkg/dicom2tiff_wasm_bg.wasm.d.ts crates/wasm/pkg/dicom2tiff_bg.wasm.d.ts
	sed -i ".bak" -e 's/dicom2tiff_wasm_bg/dicom2tiff_bg/g' crates/wasm/pkg/dicom2tiff.js
	rm crates/wasm/pkg/dicom2tiff.js.bak

build-cli:
	cargo build -p dicom2tiff-cli --release