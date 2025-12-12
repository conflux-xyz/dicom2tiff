clippy:
	cargo clippy --all-targets --all-features -- -D warnings

format:
	cargo fmt

build-cli:
	cargo build -p dicom2tiff-cli --release