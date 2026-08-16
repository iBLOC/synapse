# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in Synapse, please report it
privately to the maintainers.

**Please do NOT open a public issue for security vulnerabilities.**

### How to Report

Email: security@synapse-rs.dev (placeholder)

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response

We will:
- Acknowledge receipt within 48 hours
- Investigate and provide a timeline
- Coordinate disclosure timing with you
- Credit you in the fix release (unless you prefer anonymity)

## Security Considerations

Synapse is an agent development framework. When using it in production:

- **API keys** — never commit LLM provider API keys to source control
- **Tool execution** — review tools before exposing to LLMs
  - `McpToolAdapter` invokes external MCP servers
  - Custom tools execute arbitrary code
- **A2A communication** — validate incoming messages from untrusted agents
- **Document content** — chunkers preserve source content; ensure sensitive
  data is not leaked through retrieval

## Dependency Security

We use `cargo audit` to track known vulnerabilities in dependencies.
Run locally:

```bash
cargo install cargo-audit
cargo audit
```
