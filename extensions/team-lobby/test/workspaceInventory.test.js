const test = require('node:test');
const assert = require('node:assert/strict');
const { formatWorkspaceInventory } = require('../dist/workspaceInventory');

test('workspace inventory returns only unique safe relative file paths', () => {
	const result = formatWorkspaceInventory(['src\\main.ts', 'src/main.ts', '../outside', '.env', 'C:\\secret', '/absolute', 'safe\nforged']);
	assert.equal(result, 'Workspace files (1, additional files omitted; paths only):\n- src/main.ts');
	assert.equal(formatWorkspaceInventory([]), 'Workspace files (0; paths only):');
});

test('workspace inventory enforces file and character bounds', () => {
	assert.match(formatWorkspaceInventory(['a.ts', 'b.ts', 'c.ts'], 2), /a\.ts\n- b\.ts/);
	assert.match(formatWorkspaceInventory(['a.ts', 'b.ts', 'c.ts'], 2), /additional files omitted/);
	assert.equal(formatWorkspaceInventory(['a.ts'], 0), 'Workspace files (0, additional files omitted; paths only):');
	assert.equal(formatWorkspaceInventory(['src/very-long-file.ts'], 10, 22).length, 22);
	assert.match(formatWorkspaceInventory(['src/very-long-file.ts'], 10, 22), /truncated/);
});
