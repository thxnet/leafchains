variable "TAG" {
  default = "develop"
}

variable "REPOSITORY" {
  default = "ghcr.io"
}

variable "DEBUG" {
  default = "0"
}

group "default" {
  targets = [
    "leafchain",
    "leafchain-genesis",
  ]
}

target "base" {
  dockerfile = "dev-support/containers/debian/Containerfile"
  args = {
    DEBUG = "${DEBUG}"
  }
  platforms = ["linux/amd64"]
}

target "leafchain" {
  inherits = ["base"]
  target   = "leafchain"
  tags     = ["${REPOSITORY}/thxnet/leafchain:${TAG}"]
}

target "leafchain-genesis" {
  inherits = ["base"]
  target   = "leafchain-genesis"
  tags     = ["${REPOSITORY}/thxnet/leafchain-genesis:${TAG}"]
  contexts = {
    alpine = "docker-image://docker.io/alpine:3.18"
  }
}
