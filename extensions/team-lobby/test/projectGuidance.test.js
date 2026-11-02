const test = require('node:test');
const assert = require('node:assert/strict');
const { boundProjectGuidance, isProjectGuidancePath, maxProjectGuidanceCharacters, maxProjectGuidanceFileCharacters, maxProjectGuidanceFiles } = require('../dist/projectGuidance');

test('accepts root and nested AGENTS.md guidance but rejects unrelated or unsafe paths', () => {
	for (const path of ['AGENTS.md', 'src/AGENTS.md', 'packages/app/src/AGENTS.md', '.forge-room/instructions.md']) { assert.equal(isProjectGuidancePath(path), true, path); }
	for (const path of ['', '../AGENTS.md', 'src/../AGENTS.md', '/tmp/AGENTS.md', 'C:\\private\\AGENTS.md', 'node_modules/pkg/AGENTS.md', 'src/rules.md', '.forge-room/AGENTS.md/extra']) { assert.equal(isProjectGuidancePath(path), false, path); }
});

test('bounds guidance file count, per-file text, aggregate text, and duplicate paths', () => {
	const sources = [
		{ path: 'AGENTS.md', content: 'a'.repeat(maxProjectGuidanceFileCharacters + 1) },
		{ path: 'src/AGENTS.md', content: 'b'.repeat(maxProjectGuidanceCharacters) },
		{ path: 'SRC/agents.md', content: 'duplicate' },
		...Array.from({ length: maxProjectGuidanceFiles }, (_, index) => ({ path: `pkg${index}/AGENTS.md`, content: 'c'.repeat(100) }))
	];
	const bounded = boundProjectGuidance(sources);
	assert.ok(bounded.sources.length <= maxProjectGuidanceFiles);
	assert.ok(bounded.sources.every(source => source.content.length <= maxProjectGuidanceFileCharacters));
	assert.ok(bounded.sources.reduce((total, source) => total + source.content.length, 0) <= maxProjectGuidanceCharacters);
	assert.ok(bounded.omittedCount >= 3);
});
