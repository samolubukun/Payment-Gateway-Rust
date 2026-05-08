FROM rust:slim as rust-builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build --release

# --- Build Stage: Node/Vite ---
FROM node:20-slim as node-builder
WORKDIR /app/dashboard
COPY dashboard/package*.json ./
RUN npm install
COPY dashboard/ ./
# Inject relative paths for the production build
RUN sed -i 's|const GATEWAY_URL = .*|const GATEWAY_URL = "/api/gateway";|' src/App.jsx
RUN sed -i 's|const BANK_URL = .*|const BANK_URL = "/api/bank";|' src/App.jsx
RUN npm run build

# --- Final Stage ---
FROM debian:bookworm-slim

# Install dependencies: Postgres, Nginx, and SSL libs
RUN apt-get update && apt-get install -y \
    postgresql \
    postgresql-contrib \
    nginx \
    libssl3 \
    ca-certificates \
    curl \
    procps \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user (Hugging Face uses UID 1000)
RUN useradd -m -u 1000 user
WORKDIR /app

# Copy Rust binaries
COPY --from=rust-builder /app/target/release/gateway /usr/local/bin/gateway
COPY --from=rust-builder /app/target/release/mock-bank /usr/local/bin/mock-bank

# Copy frontend static files
COPY --from=node-builder /app/dashboard/dist /var/www/html

# Copy migrations
COPY migrations /app/migrations

# Nginx config
COPY nginx.conf /etc/nginx/sites-available/default
# Fix Nginx permissions for non-root and remove conflicting directives
RUN sed -i 's/^user/#user/' /etc/nginx/nginx.conf && \
    sed -i 's|^pid /run/nginx.pid;|pid /tmp/nginx.pid;|' /etc/nginx/nginx.conf && \
    mkdir -p /var/lib/nginx/body /var/lib/nginx/proxy /var/lib/nginx/fastcgi && \
    chown -R user:user /var/lib/nginx /var/log/nginx /var/www/html /etc/nginx

# Set up Postgres for non-root
# We will initialize the DB in /app/data at runtime
RUN mkdir -p /app/data/postgres && chown -R user:user /app/data

# Add postgres bins to path
ENV PATH="/usr/lib/postgresql/15/bin:$PATH"

# Entrypoint script
COPY entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh && chown user:user /app/entrypoint.sh

USER user

# Environment variables for the apps
ENV DATABASE_HOST=localhost \
    DATABASE_PORT=5432 \
    DATABASE_NAME=mockbank \
    DATABASE_USER=user \
    DATABASE_PASSWORD=password \
    BANK_BASE_URL=http://localhost:8787 \
    GATEWAY_PORT=3000 \
    RUST_LOG=info

# HF Spaces expects port 7860
EXPOSE 7860

CMD ["/app/entrypoint.sh"]
