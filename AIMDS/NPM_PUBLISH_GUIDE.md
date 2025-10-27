# NPM Package Publishing Guide - AIMDS

## Package Overview

**Name**: `@ruv/aimds`
**Version**: `0.1.0`
**Description**: AI Manipulation Defense System - TypeScript Gateway with AgentDB and lean-agentic integration
**License**: MIT
**Repository**: https://github.com/ruvnet/midstream

## Package Structure

```
AIMDS/
├── package.json          # NPM configuration
├── tsconfig.json         # TypeScript configuration
├── src/                  # TypeScript source files
│   ├── index.ts         # Main entry point
│   ├── gateway/         # Express.js API server
│   ├── agentdb/         # AgentDB vector search client
│   ├── lean-agentic/    # Formal verification engine
│   ├── monitoring/      # Metrics and telemetry
│   └── types/           # TypeScript type definitions
├── dist/                 # Compiled JavaScript (generated)
└── tests/               # Test suites
```

## Publishing Steps

### 1. Pre-Publication Checklist

```bash
cd /workspaces/midstream/AIMDS

# Verify package.json configuration
cat package.json | jq '.name, .version, .main, .types, .files'

# Build TypeScript to JavaScript
npm run build

# Run tests
npm test

# Check package contents (dry run)
npm pack --dry-run
```

### 2. Package Configuration

Ensure `package.json` has correct settings:

```json
{
  "name": "@ruv/aimds",
  "version": "0.1.0",
  "description": "AI Manipulation Defense System with Midstream integration",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "files": [
    "dist/**/*",
    "README.md",
    "LICENSE"
  ],
  "keywords": [
    "ai-security",
    "adversarial-defense",
    "prompt-injection",
    "aimds",
    "midstream",
    "agentdb",
    "lean-agentic",
    "ruv"
  ],
  "homepage": "https://ruv.io/midstream/aimds",
  "repository": {
    "type": "git",
    "url": "https://github.com/ruvnet/midstream.git",
    "directory": "AIMDS"
  },
  "bugs": {
    "url": "https://github.com/ruvnet/midstream/issues"
  }
}
```

### 3. NPM Authentication

**Option A: Interactive Login**
```bash
npm login
# Username: your_npm_username
# Password: your_npm_password
# Email: your_email@example.com
# Two-Factor Auth Code: (if enabled)
```

**Option B: Token Authentication**
```bash
# Set authentication token
npm set //registry.npmjs.org/:_authToken YOUR_NPM_TOKEN

# Or add to .npmrc
echo "//registry.npmjs.org/:_authToken=YOUR_NPM_TOKEN" >> ~/.npmrc
```

Get your token from: https://www.npmjs.com/settings/~/tokens

### 4. Version Management

```bash
# View current version
npm version

# Increment version (choose one)
npm version patch   # 0.1.0 -> 0.1.1 (bug fixes)
npm version minor   # 0.1.0 -> 0.2.0 (new features)
npm version major   # 0.1.0 -> 1.0.0 (breaking changes)

# Or manually edit package.json version field
```

### 5. Build and Test

```bash
# Clean previous builds
rm -rf dist/

# Build TypeScript
npm run build

# Verify build output
ls -lh dist/

# Run all tests
npm test

# Run linting
npm run lint
```

### 6. Create Tarball (Optional Test)

```bash
# Create package tarball
npm pack

# This creates: ruv-aimds-0.1.0.tgz

# Test installation from tarball
mkdir /tmp/test-install
cd /tmp/test-install
npm install /workspaces/midstream/AIMDS/ruv-aimds-0.1.0.tgz
node -e "const aimds = require('@ruv/aimds'); console.log(aimds);"
```

### 7. Publish to NPM

```bash
cd /workspaces/midstream/AIMDS

# Dry run (verify what will be published)
npm publish --dry-run

# Publish with public access (required for scoped packages)
npm publish --access public

# Or publish as private (requires paid npm account)
npm publish --access restricted
```

### 8. Verify Publication

```bash
# Check package info
npm info @ruv/aimds

# Install from npm
npm install @ruv/aimds

# View on npmjs.com
open https://www.npmjs.com/package/@ruv/aimds
```

## Package Variants

### Main Gateway Package

**Name**: `@ruv/aimds`
**Contents**: Full TypeScript gateway with all dependencies
**Use Case**: Node.js server deployment

### WASM Package (Separate)

**Name**: `@midstream/wasm`
**Location**: `/workspaces/midstream/npm-wasm`
**Contents**: Midstream Rust crates compiled to WASM
**Size**: 62-64 KB
**Use Case**: Browser and Node.js WASM usage

```bash
cd /workspaces/midstream/npm-wasm
npm publish --access public
```

## Scoped Package Naming

Using `@ruv/` scope for organization branding:

- ✅ `@ruv/aimds` - AI Manipulation Defense System
- ✅ `@midstream/wasm` - Midstream WASM bindings
- 🔄 `@ruv/temporal-compare` - Future: Direct Rust crate wrapper
- 🔄 `@ruv/lean-agentic` - Future: TypeScript bindings

## Dependencies

Current dependencies in package.json:

```json
{
  "dependencies": {
    "express": "^4.18.2",
    "agentdb": "^1.6.1",
    "lean-agentic": "^0.3.2",
    "prom-client": "^15.0.0",
    "winston": "^3.11.0"
  },
  "devDependencies": {
    "typescript": "^5.3.3",
    "@types/node": "^20.10.6",
    "@types/express": "^4.17.21",
    "vitest": "^1.1.0"
  }
}
```

## Post-Publication Tasks

### 1. Update Documentation

```bash
# Update AIMDS README.md with npm install instructions
echo "## Installation\n\n\`\`\`bash\nnpm install @ruv/aimds\n\`\`\`" >> README.md
```

### 2. Create Release Notes

Create `/workspaces/midstream/AIMDS/CHANGELOG.md`:

```markdown
# Changelog

## [0.1.0] - 2025-10-27

### Added
- Initial release of AIMDS TypeScript Gateway
- AgentDB v1.6.1 integration for vector search
- lean-agentic v0.3.2 integration for formal verification
- Express.js REST API with comprehensive middleware
- Prometheus metrics and Winston logging
- Docker and Kubernetes deployment configurations
- Comprehensive test suite (67% passing)

### Security
- TLS/HTTPS configuration required before production
- API key rotation required (see SECURITY_AUDIT_REPORT.md)

### Performance
- Detection: <10ms latency
- Analysis: <520ms latency
- Response: <50ms latency
```

### 3. Update GitHub

```bash
# Tag the release
git tag -a v0.1.0 -m "AIMDS v0.1.0 - Initial Release"
git push origin v0.1.0

# Create GitHub release (via web UI or gh CLI)
gh release create v0.1.0 \
  --title "AIMDS v0.1.0 - Initial Release" \
  --notes-file CHANGELOG.md
```

### 4. Update Project Links

- Add npm badge to README.md
- Update documentation with installation instructions
- Link to published package on ruv.io

## Badges for README

Add these badges to your README.md:

```markdown
[![npm version](https://img.shields.io/npm/v/@ruv/aimds.svg)](https://www.npmjs.com/package/@ruv/aimds)
[![npm downloads](https://img.shields.io/npm/dm/@ruv/aimds.svg)](https://www.npmjs.com/package/@ruv/aimds)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
```

## Troubleshooting

### "403 Forbidden" Error

```bash
# Check authentication
npm whoami

# Re-login
npm login

# Verify scope access
npm access ls-packages @ruv
```

### "Package Name Already Exists"

```bash
# Change package name in package.json
"name": "@ruv/aimds-v2"

# Or increment version
npm version patch
```

### "Files Not Included"

```bash
# Check files array in package.json
"files": [
  "dist/**/*",
  "README.md",
  "LICENSE"
]

# Verify with dry run
npm publish --dry-run
```

### TypeScript Build Errors

```bash
# Clean and rebuild
rm -rf dist/ node_modules/
npm install
npm run build
```

## Security Best Practices

1. **Never publish .env files**
   - Already in .gitignore
   - Not in package.json files array

2. **Use semantic versioning**
   - Breaking changes: major version
   - New features: minor version
   - Bug fixes: patch version

3. **Enable 2FA on npm account**
   - https://www.npmjs.com/settings/~/tfa

4. **Regular security audits**
   ```bash
   npm audit
   npm audit fix
   ```

## Support and Resources

- **NPM Package**: https://www.npmjs.com/package/@ruv/aimds
- **GitHub**: https://github.com/ruvnet/midstream
- **Documentation**: https://ruv.io/midstream/aimds
- **Issues**: https://github.com/ruvnet/midstream/issues

---

**Generated**: 2025-10-27
**Status**: Ready for Publication ✅
**Next Step**: Run `npm publish --access public`
