const test = require('node:test');
const assert = require('node:assert/strict');
const { maxAgentProfiles, maxAgentProfileInstructions, parseAgentProfiles } = require('../dist/agentProfiles');

const document = profiles => JSON.stringify({ format: 'forge-room-agent-profiles', version: 1, profiles });

test('parses and trims reusable profiles while preserving instruction content', () => {
	const parsed = parseAgentProfiles(document([{ name: '  Reviewer  ', description: '  Focus on correctness  ', instructions: '  Check edge cases.\nDo not edit files.  ' }]));
	assert.deepEqual(parsed, { profiles: [{ name: 'Reviewer', description: 'Focus on correctness', instructions: 'Check edge cases.\nDo not edit files.' }] });
});

test('rejects malformed profile files, duplicate names, and unsupported fields', () => {
	assert.equal(parseAgentProfiles('{').error, 'Profile file must contain valid JSON.');
	assert.ok(parseAgentProfiles(document([{ name: 'a', description: '', instructions: 'x' }, { name: 'A', description: '', instructions: 'y' }])).error);
	assert.ok(parseAgentProfiles(document([{ name: 'a', description: '', instructions: 'x', tools: ['shell'] }])).error);
	assert.ok(parseAgentProfiles(JSON.stringify({ format: 'other', version: 1, profiles: [] })).error);
});

test('bounds profile count, file size, and instruction size', () => {
	const profile = { name: 'p', description: '', instructions: 'x' };
	assert.ok(parseAgentProfiles(document(Array.from({ length: maxAgentProfiles + 1 }, (_, index) => ({ ...profile, name: 'p' + index })))).error);
	assert.ok(parseAgentProfiles(' '.repeat(64_001)).error);
	assert.ok(parseAgentProfiles(document([{ ...profile, instructions: 'x'.repeat(maxAgentProfileInstructions + 1) }])).error);
});
