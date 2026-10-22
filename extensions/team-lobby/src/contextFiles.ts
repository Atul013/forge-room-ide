const safeEnvironmentExamples = new Set(['.env.example', '.env.sample', '.env.template', '.env.dist']);
const sensitiveNames = new Set([
	'.npmrc', '.pypirc', '.netrc', '.envrc', '.git-credentials', 'auth.json',
	'credentials', 'credentials.json', 'secrets.json', 'service-account.json',
	'id_rsa', 'id_ed25519', 'id_ecdsa', 'id_dsa'
]);

export function isSensitiveContextPath(value: string): boolean {
	const normalized = value.replace(/\\/g, '/').toLowerCase();
	const parts = normalized.split('/').filter(Boolean);
	const name = parts[parts.length - 1] || '';
	if (/^\.env(?:\..+)?$/.test(name) && !safeEnvironmentExamples.has(name)) { return true; }
	if (sensitiveNames.has(name)) { return true; }
	if (/\.(?:pem|key|p12|pfx|keystore|jks)$/.test(name)) { return true; }
	if (/(?:^|\/)(?:\.aws\/credentials|\.docker\/config\.json|\.kube\/config|\.ssh\/(?:id_(?![^/]+\.pub$)[^/]+|config))$/.test(normalized)) { return true; }
	if (parts.some(part => /^(?:secrets?|credentials?|private[-_]?keys?)$/.test(part))) { return true; }
	return /(?:secret|credential|private[-_]?key|access[-_]?token|refresh[-_]?token|api[-_]?key|service[-_]?account)/.test(name);
}

export function isSensitiveContextTarget(displayPath: string, resolvedWorkspacePath: string): boolean {
	return isSensitiveContextPath(displayPath) || isSensitiveContextPath(resolvedWorkspacePath);
}
