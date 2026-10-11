# Settings and credentials are read from .env, which is mounted by 1Password
# from the "gomesh-broker" Environment (the desktop app must be unlocked):
#   SSH_USER, SSH_HOST                       - deploy target
#   CLOUDFLARE_API_TOKEN                     - `just tofu cloudflare ...`
#   AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY - `just tofu aws ...`
set dotenv-load

target := env_var('SSH_USER') + "@" + env_var('SSH_HOST')

# Deploy environment: prod (default) or stg. Override with `just env=stg <recipe>`.
env := "prod"
base := if env == "stg" { "/opt/gomesh-stg" } else { "/opt/gomesh" }
service := if env == "stg" { "gomesh-broker-stg" } else { "gomesh-broker" }

build:
    cargo build

test:
    cargo fmt
    cargo check
    cargo clippy
    cargo test

build-release:
    cargo build --release

# Config file deployed for each environment (both gitignored).
config_file := if env == "stg" { "config.stg.toml" } else { "config.toml" }

# Build, upload as a new timestamped release, and restart the service.
# The previous releases are kept on the host for rollback.
# `just deploy` targets prod; `just env=stg deploy` targets staging.
deploy: build-release
    scp target/release/gomesh-broker {{target}}:/tmp/gomesh-broker-{{env}}.next
    scp {{config_file}} {{target}}:/tmp/gomesh-broker-{{env}}-config.toml
    ssh {{target}} 'sudo bash -s -- {{env}}' < scripts/remote-install.sh

# Roll back to the previous release, or a specific one: `just rollback 20260413-031514`.
rollback version="":
    ssh {{target}} 'sudo bash -s -- {{env}} {{version}}' < scripts/remote-rollback.sh

# List releases retained on the host ((current) is the active one).
releases:
    ssh {{target}} 'rel={{base}}/releases; cur=$(readlink $rel/current); for d in $(ls -1 $rel | grep "^20" | sort); do [ "$d" = "$cur" ] && echo "$d (current)" || echo "$d"; done'

# Install or refresh the systemd unit on the host.
install-service:
    scp scripts/{{service}}.service {{target}}:/tmp/{{service}}.service
    ssh {{target}} 'sudo install -o root -g root -m 0644 /tmp/{{service}}.service /etc/systemd/system/{{service}}.service && sudo systemctl daemon-reload && sudo systemctl enable --quiet {{service}} && rm -f /tmp/{{service}}.service && echo "unit installed"'

# Install HAProxy (if needed) and push its config + Cloudflare origin-pull CA.
install-haproxy:
    scp scripts/haproxy/haproxy.cfg {{target}}:/tmp/haproxy.cfg
    scp scripts/haproxy/cloudflare-origin-pull-ca.pem {{target}}:/tmp/cloudflare-origin-pull-ca.pem
    ssh {{target}} 'sudo bash -s' < scripts/remote-install-haproxy.sh

# Run OpenTofu against infra/<stack> (cloudflare | aws), with credentials from
# the 1Password-mounted .env: `just tofu cloudflare plan`.
tofu stack *args:
    cd infra/{{stack}} && tofu {{args}}

# Service control / observability.
restart:
    ssh {{target}} 'sudo systemctl restart {{service}} && echo restarted'

status:
    ssh {{target}} 'systemctl --no-pager status {{service}}'

logs:
    ssh {{target}} 'journalctl -u {{service}} -n 100 -f'
