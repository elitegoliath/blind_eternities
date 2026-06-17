FROM python:3.11 AS builder

# Install system dependencies required for compiling Rust and Python extensions
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    git \
    libgomp1 \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install maturin for building PyO3 wheels
RUN pip install --no-cache-dir maturin

WORKDIR /app

# Copy the Rust core codebase
COPY rust_core/ ./rust_core/

# Build the production wheel using maturin
# This creates a compiled .whl file inside /app/rust_core/target/wheels/
WORKDIR /app/rust_core
RUN maturin build --release --strip

# Install the compiled wheel
RUN pip install --no-cache-dir target/wheels/*.whl

WORKDIR /app

# Copy and install Python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application source code and scripts
COPY python_agent/ ./python_agent/
COPY entrypoint.sh .

# Ensure the entrypoint script is executable
RUN chmod +x entrypoint.sh

# Define the entrypoint script
ENTRYPOINT ["./entrypoint.sh"]

# Set the default entrypoint to kick off the LangGraph loop
CMD ["python", "-m", "python_agent.main"]
