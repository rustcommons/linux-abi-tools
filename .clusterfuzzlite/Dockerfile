# The digest fixes the compiler, preinstalled build tools and their filesystem.
FROM gcr.io/oss-fuzz-base/base-builder-rust@sha256:3428d0a90b1e7efbe2ba947bd1d0c9a70594986b4b6553804547f1fb24cfc92f
ENV RUSTUP_TOOLCHAIN=nightly-2025-09-05 CARGO_BUILD_JOBS=2
RUN cargo install cargo-fuzz --locked --version 0.13.2
COPY . /src/linux-abi-tools
WORKDIR /src/linux-abi-tools
COPY .clusterfuzzlite/build.sh /src/build.sh
