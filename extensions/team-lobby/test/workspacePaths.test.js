const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { findWorkspaceFolderByUri, relativeWorkspacePath, resolveWorkspaceFile, resolveWorkspaceFileDetails, resolveWorkspaceFileDestination } = require('../dist/workspacePaths');

test('resolves a saved session to its exact project root in a multi-root workspace', () => {
	const folders = [
		{ name: 'frontend', uri: { toString: () => 'file:///projects/frontend' } },
		{ name: 'service', uri: { toString: () => 'file:///projects/service' } }
	];
	assert.equal(findWorkspaceFolderByUri(folders, 'file:///projects/service'), folders[1]);
	assert.equal(findWorkspaceFolderByUri(folders, 'file:///projects/removed'), undefined);
	assert.equal(findWorkspaceFolderByUri(undefined, 'file:///projects/service'), undefined);
});

test('computes paths relative to the selected workspace root and rejects outside files', () => {
	const root = path.resolve(path.sep, 'projects', 'app');
	assert.equal(relativeWorkspacePath(root, path.join(root, 'src', 'main.ts')), 'src/main.ts');
	assert.equal(relativeWorkspacePath(root, root), undefined);
	assert.equal(relativeWorkspacePath(root, path.resolve(path.sep, 'projects', 'other', 'main.ts')), undefined);
});

test('resolves files inside the workspace and rejects traversal or absolute paths', async t => {
	const root = fs.mkdtempSync(path.join(os.tmpdir(), 'forge-room-workspace-'));
	t.after(() => fs.rmSync(root, { recursive: true, force: true }));
	const file = path.join(root, 'src', 'main.ts');
	fs.mkdirSync(path.dirname(file));
	fs.writeFileSync(file, 'export {}');
	assert.equal(await resolveWorkspaceFile(root, 'src/main.ts'), await fs.promises.realpath(file));
	assert.equal(await resolveWorkspaceFile(root, '../outside.txt'), undefined);
	assert.equal(await resolveWorkspaceFile(root, path.resolve(root, 'src/main.ts')), undefined);
	assert.equal(await resolveWorkspaceFile(root, 'missing.ts'), undefined);
	assert.deepEqual(await resolveWorkspaceFileDetails(root, 'src/main.ts'), {
		absolutePath: await fs.promises.realpath(file),
		workspaceRelativePath: 'src/main.ts'
	});
});

test('rejects a workspace symlink that resolves outside the workspace', async t => {
	const parent = fs.mkdtempSync(path.join(os.tmpdir(), 'forge-room-symlink-'));
	t.after(() => fs.rmSync(parent, { recursive: true, force: true }));
	const root = path.join(parent, 'workspace');
	const outside = path.join(parent, 'private');
	fs.mkdirSync(root);
	fs.mkdirSync(outside);
	fs.writeFileSync(path.join(outside, 'secret.txt'), 'private');
	fs.symlinkSync(outside, path.join(root, 'linked'), 'junction');
	assert.equal(await resolveWorkspaceFile(root, 'linked/secret.txt'), undefined);
});

test('returns the canonical workspace-relative path for an in-workspace link', async t => {
	const root = fs.mkdtempSync(path.join(os.tmpdir(), 'forge-room-alias-'));
	t.after(() => fs.rmSync(root, { recursive: true, force: true }));
	const privateDir = path.join(root, 'private-keys');
	const aliasDir = path.join(root, 'docs');
	fs.mkdirSync(privateDir);
	fs.mkdirSync(aliasDir);
	fs.writeFileSync(path.join(privateDir, 'config.txt'), 'private');
	fs.symlinkSync(privateDir, path.join(aliasDir, 'shared-config'), 'junction');
	const resolved = await resolveWorkspaceFileDetails(root, 'docs/shared-config/config.txt');
	assert.equal(resolved.workspaceRelativePath, 'private-keys/config.txt');
	assert.equal(resolved.absolutePath, await fs.promises.realpath(path.join(privateDir, 'config.txt')));
});

test('resolves safe file destinations and rejects escaping parents or symlink targets', async t => {
	const parent = fs.mkdtempSync(path.join(os.tmpdir(), 'forge-room-destination-'));
	t.after(() => fs.rmSync(parent, { recursive: true, force: true }));
	const root = path.join(parent, 'workspace');
	const outside = path.join(parent, 'outside');
	fs.mkdirSync(root);
	fs.mkdirSync(outside);
	fs.mkdirSync(path.join(root, '.forge-room'));
	assert.equal(await resolveWorkspaceFileDestination(root, '.forge-room/agent-profiles.json'), path.join(root, '.forge-room', 'agent-profiles.json'));
	fs.writeFileSync(path.join(outside, 'secret.json'), '{}');
	fs.symlinkSync(outside, path.join(root, 'linked'), 'junction');
	assert.equal(await resolveWorkspaceFileDestination(root, 'linked/new.json'), undefined);
	try {
		fs.symlinkSync(path.join(outside, 'secret.json'), path.join(root, '.forge-room', 'agent-profiles.json'), 'file');
		assert.equal(await resolveWorkspaceFileDestination(root, '.forge-room/agent-profiles.json'), undefined);
	} catch (error) {
		if (error.code !== 'EPERM') { throw error; }
	}
});
