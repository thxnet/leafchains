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
    target = "leafchain"
    tags = ["${REPOSITORY}/thxnet/leafchain:${TAG}"]
}
