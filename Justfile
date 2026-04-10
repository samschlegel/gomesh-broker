build:
    cargo build

build-release:
    cargo build --release

deploy:
    chmod +x target/release/gomesh-broker
    # has to be .exe because WSL + 1Password memes
    scp.exe target/release/gomesh-broker ${SSH_USER}@${SSH_HOST}:/opt/gomesh/bin/gomesh-broker
    scp.exe config.toml ${SSH_USER}@${SSH_HOST}:/opt/gomesh/etc/gomesh-broker/config.toml
