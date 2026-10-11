# Cloudflare configuration for the gomesh MQTT broker.
#
# Manages only the broker's own resources in the gomesh.dev zone; the website
# and email records are left to the dashboard.
#
# The API token comes from the 1Password-mounted .env at the repo root and
# needs Zone:DNS:Edit, Zone:SSL and Certificates:Edit, and Zone:Zone Settings:Edit.
# Run via `just tofu cloudflare plan` / `just tofu cloudflare apply`.

terraform {
  required_version = ">= 1.5"
  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 5.27"
    }
  }
}

provider "cloudflare" {}

# Supplied as TF_VAR_* from the 1Password-mounted .env, to keep them out of the repo.
variable "zone_id" {
  description = "Cloudflare zone ID of gomesh.dev."
  type        = string
}

variable "origin_ip" {
  description = "Public IPv4 address of the broker host."
  type        = string
}

# Production broker. Created by hand, then imported.
resource "cloudflare_dns_record" "mqtt" {
  zone_id = var.zone_id
  name    = "mqtt.gomesh.dev"
  type    = "A"
  content = var.origin_ip
  proxied = true
  ttl     = 1
}

# Staging broker: same host, reached on port 8443 (HAProxy → gomesh-broker-stg).
resource "cloudflare_dns_record" "mqtt_stg" {
  zone_id = var.zone_id
  name    = "mqtt-stg.gomesh.dev"
  type    = "A"
  content = var.origin_ip
  proxied = true
  ttl     = 1
  comment = "gomesh-broker staging; clients connect on :8443"
}

# Global Authenticated Origin Pulls: Cloudflare presents its shared origin-pull
# client certificate (scripts/haproxy/cloudflare-origin-pull-ca.pem) on every
# connection to the origin. HAProxy requires it, so only Cloudflare can reach
# the broker and CF-Connecting-IP can be trusted. Harmless for origins that
# don't request a client certificate.
resource "cloudflare_zone_setting" "tls_client_auth" {
  zone_id    = var.zone_id
  setting_id = "tls_client_auth"
  value      = "on"
}

# Zone-level AOP presents a certificate we upload ourselves, and sends nothing
# if none is uploaded. Kept off: global AOP is what HAProxy is configured for.
resource "cloudflare_authenticated_origin_pulls_settings" "zone" {
  zone_id = var.zone_id
  enabled = false
}
