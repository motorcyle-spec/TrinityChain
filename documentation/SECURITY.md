# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in TrinityChain, please report it responsibly by emailing the maintainers directly rather than using public issue tracking. This allows us to fix the issue and release a patch before public disclosure.

**Email:** security@trinitychain.example (replace with actual security contact email)

When reporting, please include:

- A clear description of the vulnerability
- The affected component(s) and version(s)
- Steps to reproduce (if applicable)
- Potential impact or severity assessment
- Suggested fix (if you have one)

## Security Considerations

### What We Secure

- **Cryptographic operations**: ECDSA signatures, SHA-256 hashing
- **Network communication**: P2P message integrity
- **Consensus mechanism**: Proof-of-Work validation, difficulty adjustment
- **State management**: UTXO set consistency, blockchain integrity
- **User data**: Private key handling (in wallets)

### Known Limitations

TrinityChain is a **research/educational blockchain**. It is not yet suitable for handling real financial value without additional hardening:

1. **Peer authentication**: Currently lacks HMAC-based peer verification
2. **Rate limiting**: P2P rate limits are not fully implemented
3. **DoS resistance**: Network is vulnerable to simple DoS attacks
4. **Wallet security**: Private keys are stored locally without encryption
5. **API security**: REST API lacks authentication on sensitive endpoints

Contributors are encouraged to address these limitations. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Security Updates

We aim to release security patches promptly after disclosure is managed. Updates will be announced via:

- GitHub Releases (security advisories)
- Repository notifications
- Email to reporters

## Scope

### In Scope

- Cryptographic weaknesses
- Consensus mechanism exploits
- Network protocol vulnerabilities
- Blockchain state corruption
- Private key or wallet compromises
- Authentication/authorization bypass

### Out of Scope

- Social engineering attacks
- Physical security
- Third-party library vulnerabilities (report to upstream maintainers)
- Theoretical attacks requiring unrealistic computational resources

## Security Best Practices for Users

1. **Never share your private keys**
2. **Use secure environments** for wallet operations
3. **Verify checksums** when downloading the code
4. **Keep Rust and dependencies updated**: `rustup update && cargo update`
5. **Review code changes** before running untrusted binaries
6. **Run nodes on secure networks** behind firewalls
7. **Back up wallet seeds** using BIP-39 mnemonic recovery phrases (when available)

## Development Security

- All pull requests are reviewed before merge
- Use `cargo audit` to check dependencies: `cargo audit`
- Run tests and linting before submitting: `cargo test && cargo clippy && cargo fmt`
- Sign commits with GPG when possible: `git commit -S`

## Attribution

Security researchers who responsibly report vulnerabilities will be credited (with permission) in release notes and security advisories.

## Additional Resources

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CWE: Common Weakness Enumeration](https://cwe.mitre.org/)
- [Rust Security Advisories](https://rustsec.org/)
