build:
    cargo build

release:
    cargo build --release

deploy:
    cargo build --release
    scp target/release/gomesh-broker ${SSH_USER}@${SSH_HOST}:/opt/gomesh-broker/bin/gomesh-broker
