FROM cgr.dev/chainguard/rust:latest AS wasm-builder
WORKDIR /workspace
COPY rust/pdftool_core ./rust/pdftool_core
COPY scripts/build-wasm.sh ./scripts/build-wasm.sh
RUN cargo install wasm-pack && ./scripts/build-wasm.sh

FROM cgr.dev/chainguard/node:20 AS builder
WORKDIR /workspace
ENV DENO_INSTALL=/opt/deno
ENV PATH=${DENO_INSTALL}/bin:${PATH}
RUN curl -fsSL https://deno.land/install.sh | DENO_INSTALL=${DENO_INSTALL} sh -s v2.6.9
COPY --from=wasm-builder /workspace/rust/pdftool_core/pkg ./rust/pdftool_core/pkg
COPY . .
RUN deno task build

FROM cgr.dev/chainguard/static:latest
WORKDIR /app
COPY --from=builder /workspace/dist ./dist
COPY --from=builder /workspace/rust/pdftool_core/pkg ./rust/pdftool_core/pkg
CMD ["/bin/sh", "-c", "ls -R dist"]
