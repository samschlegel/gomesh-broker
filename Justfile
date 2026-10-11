# Deploy target is read from .env (gitignored):
#   SSH_USER=ubuntu
#   SSH_HOST=52.10.113.104
set dotenv-load

target := env_var('SSH_USER') + "@" + env_var('SSH_HOST')

build:
    cargo build

test:
    cargo fmt
    cargo check
    cargo clippy
    cargo test

build-release:
    cargo build --release

# Build, upload as a new timestamped release, and restart the service.
# The previous releases are kept on the host for rollback.
deploy: build-release
    scp target/release/gomesh-broker {{target}}:/tmp/gomesh-broker.next
    scp config.toml {{target}}:/tmp/gomesh-broker-config.toml
    ssh {{target}} 'sudo bash -s' < scripts/remote-install.sh

# Roll back to the previous release, or a specific one: `just rollback 20260413-031514`.
rollback version="":
    ssh {{target}} 'sudo bash -s -- {{version}}' < scripts/remote-rollback.sh

# List releases retained on the host ((current) is the active one).
releases:
    ssh {{target}} 'cur=$(readlink /opt/gomesh/releases/current); for d in $(ls -1 /opt/gomesh/releases | grep "^20" | sort); do [ "$d" = "$cur" ] && echo "$d (current)" || echo "$d"; done'

# Install or refresh the systemd unit on the host.
install-service:
    scp scripts/gomesh-broker.service {{target}}:/tmp/gomesh-broker.service
    ssh {{target}} 'sudo install -o root -g root -m 0644 /tmp/gomesh-broker.service /etc/systemd/system/gomesh-broker.service && sudo systemctl daemon-reload && rm -f /tmp/gomesh-broker.service && echo "unit installed"'

# Service control / observability.
restart:
    ssh {{target}} 'sudo systemctl restart gomesh-broker && echo restarted'

status:
    ssh {{target}} 'systemctl --no-pager status gomesh-broker'

logs:
    ssh {{target}} 'journalctl -u gomesh-broker -n 100 -f'
