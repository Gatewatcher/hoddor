# Hoddor Browser Vault

A secure browser-based vault implementation using WebAssembly (Rust) that provides encrypted storage capabilities with support for multiple data types, including JSON & Binary Data.

## Table of Contents

- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Usage](#usage)
- [Testing](#testing)
- [Contributing](#contributing)
- [License](#license)
- [Built With](#built-with)
- [Getting Help](#getting-help)
- [Maintainers](#maintainers)
- [Code of Conduct](#code-of-conduct)

## Features

- 🔒 Secure encryption using ChaCha20Poly1305
- 🔑 Password-based key derivation using Argon2id
- 📦 Support for multiple vaults and namespaces
- 📄 JSON data storage
- 🎥 Chunked video storage and streaming 
- 🖼️ Image storage with Base64 encoding
- 🔄 Import/Export vault functionality
- 👷 Web Worker support for better performance
- 🔒 Concurrency protection using Web Locks API

## Prerequisites

- Rust and Cargo
- wasm-pack
- watchexec
- Node.js and npm
- A modern web browser with File System Access API support

## Installation

1. Clone the repository:
```bash
git clone git@github.com:Gatewatcher/hoddor.git
cd hoddor
```

2. Install JavaScript dependencies:

```bash
cd playground
npm install
```

3. Install Rust dependencies:
```bash
cargo install wasm-pack watchexec
```

4. Start the playground server:
```bash
cd playground
npm run dev
```

## Usage

1. Open your web browser and navigate to `http://localhost:5173`.

## Testing

To run the tests, use the following command:
```bash
cd hoddor
wasm-pack test --headless --chrome
```

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## License

This project is licensed under the MIT License.

## Built With

- Rust
- WebAssembly
- Node.js
- npm
- wasm-pack

## Getting Help

If you need help, you can refer to the following resources:
- [Discord](https://discord.gg/wu3Fr6nE)
- [Support](https://github.com/Gatewatcher/hoddor/issues)

## Maintainers

- [Benoit CHASSIGNOL](benoit.chassignol@gatewatcher.com)
- [David LOIRET](david.loiret@gatewatcher.com)

## Code of Conduct

Please refer to our [Code of Conduct](https://github.com/Gatewatcher/hoddor/blob/main/CODE_OF_CONDUCT.md).
