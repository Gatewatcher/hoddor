# Hoddor Browser Vault

A cutting-edge browser-based vault built with WebAssembly (Rust) that ensures your data remains private and secure by encrypting it directly on the client side. The backend has no access to the decryption keys, offering a truly zero-knowledge implementation for modern web applications.

This solution provides encrypted storage capabilities with robust support for multiple data types, including JSON and binary data, while ensuring that all sensitive operations occur exclusively on the user’s device.

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

### Key Features

- 🔒 **End-to-End Encryption**: Data is encrypted using **ChaCha20Poly1305** entirely on the client side. The backend never has access to raw data or decryption keys.
  
- 🔑 **Password-Based Key Derivation**: Utilizes **Argon2id** to generate cryptographically secure keys from user-provided passwords.

- 📦 **Multi-Vault and Namespace Support**: Allows segregation of data into multiple vaults for organized and secure data storage.

- 📄 **Flexible Data Formats**: Handles structured data like JSON and arbitrary binary data to meet diverse storage needs.

- ⚡ **Encrypted Browser Cache**: Serves as a high-performance encrypted cache, securely storing temporary data on the client side to optimize application performance without compromising privacy.

- 🔗 **Bridge Between Segmented Appliances**: Enables secure and private data exchange between isolated systems by acting as a zero-knowledge intermediary, keeping data encrypted in transit and at rest.

- 🎥 **Chunked Video Storage & Streaming**: Efficiently stores and streams large video files without sacrificing security.

- 🖼️ **Secure Image Storage**: Uses **Base64 encoding** for convenient storage of image assets within the encrypted vault.

- 🔄 **Import/Export Functionality**: Allows users to securely transfer encrypted data between devices or applications.

- 👷 **Web Worker Integration**: Offloads heavy encryption and decryption tasks to web workers for smooth, non-blocking performance.

- 🔒 **Concurrency Protection**: Ensures safe and consistent data access using the **Web Locks API**.

- ⏳ **Data Expiration**: Configurable automatic data deletion ensures efficient storage management and enhanced privacy.

- 🗂️ **Origin-Scoped Storage**: Leverages the **Origin Private File System (OPFS)** to store encrypted data locally on the user’s device, isolating data by web origin to prevent cross-site leakage.

### Built for Privacy-First Applications
This solution is ideal for building **zero-knowledge systems** where user privacy is paramount. The backend serves solely as a storage and synchronization medium, with all sensitive encryption and decryption logic confined to the client side. This ensures that sensitive information never leaves the user's control, empowering developers to build **compliant, privacy-focused, and performant web applications.**


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
