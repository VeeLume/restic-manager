FROM ubuntu:24.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    docker.io \
    cron \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy the built binary (will be mounted)
# The binary will be provided at runtime via volume mount

# Create directories
RUN mkdir -p /root/.local/bin \
    && mkdir -p /var/log/restic-manager \
    && mkdir -p /backup-data \
    && mkdir -p /backup-repo

# Create restic password file
RUN echo "test-password-123" > /root/restic_password && chmod 600 /root/restic_password

# Entry point script
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
CMD ["bash"]
