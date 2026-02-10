# precious 💍

> *My precious...dollar bills* — A cloud infrastructure cost estimator for Terraform, written in Rust.

**precious** tries to do what [Infracost](https://github.com/infracost/infracost) does, just in a more user friendly and up to date way.

## Installation

### Cargo

```bash
cargo install precious-cli
```

### Nix

```bash
# Run directly without installing
nix run github:Polymath-AS/precious

# Install into your profile
nix profile install github:Polymath-AS/precious

# Enter a development shell
nix develop github:Polymath-AS/precious
```

## Quick Start

```bash
# Show cost breakdown for current directory
precious breakdown --path ./terraform
```

## Supported Resources

To be filled

## License

MIT
