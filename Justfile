build:
    cargo build

test:
    cargo fmt
    cargo check
    cargo clippy
    cargo test

build-release:
    cargo build --release

deploy:
    chmod +x target/release/gomesh-broker
    # has to be .exe because WSL + 1Password memes
    scp.exe target/release/gomesh-broker ${SSH_USER}@${SSH_HOST}:/opt/gomesh/bin/gomesh-broker.next
    scp.exe config.toml ${SSH_USER}@${SSH_HOST}:/opt/gomesh/etc/gomesh-broker/config.toml
    ssh.exe ${SSH_USER}@${SSH_HOST} chmod +x /opt/gomesh/bin/gomesh-broker.next
    ssh.exe ${SSH_USER}@${SSH_HOST} mv /opt/gomesh/bin/gomesh-broker.next /opt/gomesh/bin/gomesh-broker
