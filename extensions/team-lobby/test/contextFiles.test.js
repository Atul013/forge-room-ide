const test = require('node:test');
const assert = require('node:assert/strict');
const { isSensitiveContextPath, isSensitiveContextTarget } = require('../dist/contextFiles');

test('identifies common credentials, secret files, and private keys independent of path casing', () => {
	for (const file of ['.env', '.env.production', '.envrc', '.git-credentials', '.npmrc', '.aws/credentials', '.docker/config.json', '.kube/config', '.ssh/id_ed25519', 'certs/dev.PEM', 'secrets/firebase.json', 'src/api-secret.ts', 'config/service-account-key.json', 'src/access-token.ts']) {
		assert.equal(isSensitiveContextPath(file), true, file);
	}
});

test('does not classify example environment files or ordinary source as sensitive', () => {
	for (const file of ['.env.example', '.env.sample', '.env.template', '.env.dist', '.ssh/id_rsa.pub', 'src/authentication.ts', 'config.json', 'package.json', 'docs/security.md']) {
		assert.equal(isSensitiveContextPath(file), false, file);
	}
});

test('sensitive-path checks include the resolved target behind a workspace alias', () => {
	assert.equal(isSensitiveContextTarget('docs/public-config.json', '.env.production'), true);
	assert.equal(isSensitiveContextTarget('guides/key.txt', 'private-keys/id_ed25519'), true);
	assert.equal(isSensitiveContextTarget('src/config.ts', 'src/config.ts'), false);
});
