# ==========================================
# STAGE 1: Build the Rust Extension
# ==========================================
FROM python:3.11-slim AS builder

# Install system dependencies required for compiling Rust and Python extensions
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    git \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install maturin for building PyO3 wheels
RUN pip install --no-cache-dir maturin

WORKDIR /build

# Copy the Rust core codebase
COPY rust_core/ ./rust_core/

# Build the production wheel using maturin
# This creates a compiled .whl file inside /build/rust_core/target/wheels/
WORKDIR /build/rust_core
RUN maturin build --release --strip

# ==========================================
# STAGE 2: Final Python Runtime Environment
# ==========================================
FROM python:3.11-slim AS runner

WORKDIR /app

# Install runtime-only system dependencies if needed (e.g., for fastembed/onnx)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libgomp1 \
    && rm -rf /var/lib/apt/lists/*

# 1. Copy and install the compiled Rust wheel from the builder stage
COPY --from=builder /build/rust_core/target/wheels/*.whl /tmp/
RUN pip install --no-cache-dir /tmp/*.whl && rm -rf /tmp/*.whl

# 2. Copy and install Python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# 3. Copy application source code
COPY python_agent/ ./python_agent/

# Set the default entrypoint to kick off the LangGraph loop
CMD ["python", "-m", "python_agent.main"]
