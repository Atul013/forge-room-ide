const test = require('node:test');
const assert = require('node:assert/strict');
const { routeSharedDocumentUpdate, SharedTextDocument, decodeSharedDocumentUpdate, maxSharedDocumentBytes, maxSharedDocumentUpdateBytes, validSharedDocumentPath } = require('../dist/sharedDocument');
const { sharedWorkspacePath, resolveSharedWorkspacePath } = require('../dist/sharedWorkspacePath');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');

test('shared document paths map safely across uniquely named workspace roots', async () => {
	const root = await fs.mkdtemp(path.join(os.tmpdir(), 'forge-shared-roots-'));
	const source = path.join(root, 'source');
	const tests = path.join(root, 'tests');
	try {
		await fs.mkdir(path.join(source, 'src'), { recursive: true });
		await fs.mkdir(path.join(tests, 'spec'), { recursive: true });
		await fs.writeFile(path.join(tests, 'spec', 'app.test.ts'), 'test');
		const folders = [
			{ name: 'source', uri: { fsPath: source } },
			{ name: 'tests', uri: { fsPath: tests } }
		];
		const roomPath = sharedWorkspacePath(folders, folders[1], path.join(tests, 'spec', 'app.test.ts'));
		assert.equal(roomPath, 'tests/spec/app.test.ts');
		assert.equal(await resolveSharedWorkspacePath(folders, roomPath), path.join(tests, 'spec', 'app.test.ts'));
		assert.equal(await resolveSharedWorkspacePath(folders, 'unknown/file.ts'), undefined);
		assert.equal(await resolveSharedWorkspacePath(folders, 'tests/../source/private.ts'), undefined);
		const ambiguous = [{ ...folders[0], name: 'repo' }, { ...folders[1], name: 'repo' }];
		assert.equal(sharedWorkspacePath(ambiguous, ambiguous[0], path.join(source, 'src', 'main.ts')), undefined);
	} finally { await fs.rm(root, { recursive: true, force: true }); }
});

test('shared paths reject absolute and traversal paths', () => {
	assert.equal(validSharedDocumentPath('src/main.ts'), true);
	for (const value of ['../secret', 'src/../secret', '/etc/passwd', 'C:\\secret', 'src//main.ts', 'src/']) { assert.equal(validSharedDocumentPath(value), false); }
});

test('Yjs updates converge when peers make concurrent edits', () => {
	const first = new SharedTextDocument('src/main.ts', 'abcd');
	const second = new SharedTextDocument('src/main.ts', 'abcd');
	try {
		assert.equal(first.edit(1, 0, 'X'), true);
		assert.equal(second.edit(3, 0, 'Y'), true);
		const firstState = first.encode();
		const secondState = second.encode();
		assert.equal(first.apply(secondState), true);
		assert.equal(second.apply(firstState), true);
		assert.equal(first.value, second.value);
		assert.match(first.value, /X/);
		assert.match(first.value, /Y/);
	} finally { first.destroy(); second.destroy(); }
});

test('room relay rejects non-subscribers and forwards an accepted update only to other subscribers', () => {
	const host = new SharedTextDocument('src/main.ts', 'start');
	const peerA = new SharedTextDocument('src/main.ts');
	const peerB = new SharedTextDocument('src/main.ts');
	try {
		peerA.applyState(host.encode());
		peerB.applyState(host.encode());
		peerA.edit(5, 0, '-A');
		const update = peerA.encode();
		assert.equal(routeSharedDocumentUpdate(host, ['peer-a', 'peer-b'], 'intruder', update), undefined);
		assert.equal(host.value, 'start');
		const routed = routeSharedDocumentUpdate(host, ['peer-a', 'peer-b', 'peer-b'], 'peer-a', update);
		assert.deepEqual(routed.recipients, ['peer-b']);
		peerB.apply(routed.update);
		assert.equal(host.value, peerB.value);
	} finally { host.destroy(); peerA.destroy(); peerB.destroy(); }
});

test('editor change batches apply atomically and emit one local update', () => {
	const doc = new SharedTextDocument('src/app.ts', 'abcdef');
	const updates = [];
	const subscription = doc.onUpdate((update, remote) => updates.push({ update, remote }));
	try {
		assert.equal(doc.editMany([{ offset: 1, deleteLength: 1, insertText: 'B' }, { offset: 4, deleteLength: 1, insertText: 'E' }]), true);
		assert.equal(doc.value, 'aBcdEf');
		assert.equal(updates.length, 1);
		assert.equal(updates[0].remote, false);
		const before = doc.value;
		assert.equal(doc.editMany([{ offset: 0, deleteLength: 4, insertText: '' }, { offset: 2, deleteLength: 2, insertText: '' }]), false);
		assert.equal(doc.value, before);
		assert.equal(updates.length, 1);
	} finally { subscription.dispose(); doc.destroy(); }
});

test('update and document limits reject oversized or malformed data', () => {
	assert.equal(decodeSharedDocumentUpdate('%%%'), undefined);
	assert.equal(decodeSharedDocumentUpdate(Buffer.alloc(maxSharedDocumentUpdateBytes + 1).toString('base64')), undefined);
	assert.throws(() => new SharedTextDocument('large.txt', 'x'.repeat(maxSharedDocumentBytes + 1)), /512 KB/);
	const doc = new SharedTextDocument('file.txt', 'safe');
	try { assert.equal(doc.apply('AA=='), false); assert.equal(doc.value, 'safe'); }
	finally { doc.destroy(); }
});

test('initial state transfer supports empty documents and text larger than an incremental update', () => {
	const emptySource = new SharedTextDocument('empty.txt');
	const emptyTarget = new SharedTextDocument('empty.txt');
	const largeSource = new SharedTextDocument('large.txt', 'x'.repeat(maxSharedDocumentBytes));
	const largeTarget = new SharedTextDocument('large.txt');
	try {
		assert.equal(emptyTarget.applyState(emptySource.encode()), true);
		assert.equal(largeSource.encode().length > Math.ceil(maxSharedDocumentUpdateBytes * 4 / 3), true);
		assert.equal(largeTarget.applyState(largeSource.encode()), true);
		assert.equal(largeTarget.value.length, maxSharedDocumentBytes);
	} finally { emptySource.destroy(); emptyTarget.destroy(); largeSource.destroy(); largeTarget.destroy(); }
});
