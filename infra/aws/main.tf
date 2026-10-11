# AWS configuration for the gomesh MQTT broker host (Lightsail, us-west-2).
#
# Credentials come from the 1Password-mounted .env at the repo root; run via
# `just tofu aws plan` / `just tofu aws apply`.

terraform {
  required_version = ">= 1.5"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.0"
    }
  }
}

provider "aws" {
  region = "us-west-2"
}

# Supplied as TF_VAR_instance_name from the 1Password-mounted .env.
variable "instance_name" {
  description = "Lightsail instance name of the broker host."
  type        = string
}

# Authoritative: this is the instance's ENTIRE public firewall. Any port not
# listed here is closed on apply, so never drop the SSH rule.
# Not importable; the first apply rewrites the rules wholesale, so this must
# match the live rules exactly (verified via GetInstancePortStates 2026-10-11).
resource "aws_lightsail_instance_public_ports" "broker" {
  instance_name = var.instance_name

  # SSH. Open to everything, which also covers Lightsail's browser-based SSH
  # (the API reports no separate lightsail-connect alias).
  port_info {
    protocol   = "tcp"
    from_port  = 22
    to_port    = 22
    cidrs      = ["0.0.0.0/0"]
    ipv6_cidrs = ["::/0"]
  }

  port_info {
    protocol   = "tcp"
    from_port  = 80
    to_port    = 80
    cidrs      = ["0.0.0.0/0"]
    ipv6_cidrs = ["::/0"]
  }

  # Production broker (Cloudflare connects to the origin over IPv4 only).
  port_info {
    protocol  = "tcp"
    from_port = 443
    to_port   = 443
    cidrs     = ["0.0.0.0/0"]
  }

  # Staging broker via HAProxy (mqtt-stg.gomesh.dev).
  port_info {
    protocol  = "tcp"
    from_port = 8443
    to_port   = 8443
    cidrs     = ["0.0.0.0/0"]
  }
}
